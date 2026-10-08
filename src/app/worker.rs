//! @module app::worker
//! @description Background loops: the telemetry poller (scheduler-driven) and the control executor.
//!
//! @input  A `TelemetryService`, a control sink, message channels, a clock, delivery callbacks.
//! @output `TelemetryDelivery` values and control results, delivered through the callbacks.
//! @dependencies telemetry, ipc::client, controls, config, app::model
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use crate::app::model::{ControlRequest, TelemetryUpdate};
use crate::controls::{FanCurveProfile, FanMode, PerformanceMode};
use crate::ipc::client::{HelperClient, Transport};
use crate::ipc::protocol::CapabilitiesReport;
use crate::telemetry::scheduler::{cadence, DueSet, PollScheduler, RefreshRate};
use crate::telemetry::{PollOptions, ProviderErrorEntry, TelemetryService};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetryConfig {
    pub rate: RefreshRate,
    pub background_ms: u64,
    pub graph_icons: bool,
    pub gpu_driver: bool,
    pub fans: bool,
}

impl TelemetryConfig {
    /// Worker configuration derived from user settings.
    pub fn from_settings(s: &crate::config::Settings) -> Self {
        Self {
            rate: s.telemetry.refresh_rate,
            background_ms: s.telemetry.background_interval_ms,
            graph_icons: s.graph_icons_enabled(),
            gpu_driver: s.telemetry.gpu_driver_queries,
            fans: s.telemetry.fan_telemetry,
        }
    }
}

pub enum WorkerMsg {
    Visibility(bool),
    Config(TelemetryConfig),
    /// Re-read fan/performance state now (after a control change).
    RefreshModes,
    /// Re-read capabilities now (e.g. after installing the helper).
    RefreshCapabilities,
    Stop,
}

#[derive(Debug, Clone)]
pub struct TelemetryDelivery {
    pub update: TelemetryUpdate,
    pub due: DueSet,
    pub errors: Vec<ProviderErrorEntry>,
    pub report: Option<CapabilitiesReport>,
}

fn deliver_poll(
    svc: &mut TelemetryService,
    due: DueSet,
    opts: PollOptions,
    now: u64,
    deliver: &impl Fn(TelemetryDelivery),
) {
    let snapshot = svc.poll(due, opts, now);
    let report = svc.report().cloned();
    let (provider, model) =
        report.as_ref().map_or((String::new(), String::new()), |r| (r.provider.clone(), r.model.clone()));
    deliver(TelemetryDelivery {
        update: TelemetryUpdate { snapshot, status: svc.status(), capabilities: svc.capabilities(), provider, model },
        due,
        errors: svc.recent_errors(),
        report,
    });
}

/// Polls on the scheduler's cadence until `Stop` or the channel closes.
pub fn run_telemetry(
    mut svc: TelemetryService,
    rx: &Receiver<WorkerMsg>,
    mut cfg: TelemetryConfig,
    clock: impl Fn() -> u64,
    deliver: impl Fn(TelemetryDelivery),
) {
    let mut sched = PollScheduler::new();
    let mut visible = false;
    loop {
        let now = clock();
        let cad = cadence(cfg.rate, cfg.background_ms, visible, cfg.graph_icons);
        let opts = PollOptions { gpu_driver: cfg.gpu_driver && visible, fans: cfg.fans };
        let due = sched.due(now, &cad);
        if due.any() {
            deliver_poll(&mut svc, due, opts, now, &deliver);
        }
        let wait = sched.next_wake_ms(clock(), &cad).unwrap_or(5000).clamp(20, 5000);
        match rx.recv_timeout(Duration::from_millis(wait)) {
            Ok(WorkerMsg::Visibility(v)) => {
                if v && !visible {
                    sched.reset();
                }
                visible = v;
            }
            Ok(WorkerMsg::Config(c)) => {
                cfg = c;
                sched.reset();
            }
            Ok(WorkerMsg::RefreshModes) => {
                let due = DueSet { modes: true, ..DueSet::default() };
                deliver_poll(&mut svc, due, opts, clock(), &deliver);
            }
            Ok(WorkerMsg::RefreshCapabilities) => {
                svc.refresh_capabilities(clock());
                sched.reset();
            }
            Ok(WorkerMsg::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

/// Hardware writes, as the control thread sees them.
pub trait ControlSink {
    fn set_fan(&self, mode: FanMode, curve: Option<FanCurveProfile>) -> Result<String, String>;
    fn set_performance(&self, mode: PerformanceMode) -> Result<String, String>;
}

impl<T: Transport> ControlSink for HelperClient<T> {
    fn set_fan(&self, mode: FanMode, curve: Option<FanCurveProfile>) -> Result<String, String> {
        self.set_fan_mode(mode, curve).map_err(|e| e.user_message())
    }
    fn set_performance(&self, mode: PerformanceMode) -> Result<String, String> {
        self.set_performance_mode(mode).map_err(|e| e.user_message())
    }
}

/// Executes control requests one at a time (never concurrently) and reports each result.
pub fn run_control(
    sink: &impl ControlSink,
    rx: &Receiver<ControlRequest>,
    done: impl Fn(ControlRequest, Result<String, String>),
) {
    while let Ok(req) = rx.recv() {
        let result = match &req {
            ControlRequest::Fan(mode, curve) => sink.set_fan(*mode, curve.clone()),
            ControlRequest::Performance(mode) => sink.set_performance(*mode),
        };
        done(req, result);
    }
}
