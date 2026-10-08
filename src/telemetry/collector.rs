//! @module telemetry::collector
//! @description Telemetry service: composes CPU, GPU and helper providers into snapshots.
//!
//! @input  Provider trait objects, a `DueSet` from the scheduler, poll options and a clock.
//! @output `SystemSnapshot`s; provider status/capabilities; a log of recent provider errors.
//! @dependencies telemetry::{snapshot, scheduler}, ipc::{client, protocol}, acer
//!
//! Isolation: every provider call is wrapped so a failure (or panic) blanks that reading and is
//! logged, and can never take the tray host down. Missing data stays missing — never synthesised.
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};

use super::scheduler::DueSet;
use super::snapshot::{FanTelemetry, SystemSnapshot};
use crate::acer::{Capabilities, FanState, PerformanceState, SensorReadings};
use crate::ipc::client::{ClientError, HelperClient, Transport};
use crate::ipc::protocol::{CapabilitiesReport, ProviderStatus};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CpuSample {
    pub utilisation_pct: Option<f32>,
    pub frequency_mhz: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GpuSample {
    pub temperature_c: Option<f32>,
    pub utilisation_pct: Option<f32>,
    pub frequency_mhz: Option<u32>,
    pub power_w: Option<f32>,
}

pub trait CpuSource: Send {
    fn sample(&mut self) -> Result<CpuSample, String>;
}

pub trait GpuSource: Send {
    fn sample(&mut self) -> Result<GpuSample, String>;
}

/// Hardware readings that require the privileged helper.
pub trait HardwareSource: Send {
    fn capabilities(&mut self) -> Result<CapabilitiesReport, ClientError>;
    fn sensors(&mut self) -> Result<SensorReadings, ClientError>;
    fn fan_state(&mut self) -> Result<FanState, ClientError>;
    fn performance(&mut self) -> Result<PerformanceState, ClientError>;
}

impl<T: Transport> HardwareSource for HelperClient<T> {
    fn capabilities(&mut self) -> Result<CapabilitiesReport, ClientError> {
        HelperClient::capabilities(self)
    }
    fn sensors(&mut self) -> Result<SensorReadings, ClientError> {
        HelperClient::sensors(self)
    }
    fn fan_state(&mut self) -> Result<FanState, ClientError> {
        HelperClient::fan_state(self)
    }
    fn performance(&mut self) -> Result<PerformanceState, ClientError> {
        HelperClient::performance(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PollOptions {
    /// Query the GPU vendor driver (NVML).
    pub gpu_driver: bool,
    /// Report fan RPM at all.
    pub fans: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderErrorEntry {
    pub timestamp_ms: u64,
    pub source: &'static str,
    pub message: String,
}

const ERROR_LOG_LEN: usize = 20;
/// While the helper is unreachable, capabilities are re-queried at this interval.
pub const CAPS_RETRY_MS: u64 = 10_000;

fn guarded<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| Err("provider panicked".into()))
}

pub struct TelemetryService {
    cpu: Option<Box<dyn CpuSource>>,
    gpu: Option<Box<dyn GpuSource>>,
    hw: Box<dyn HardwareSource>,
    last: SystemSnapshot,
    status: ProviderStatus,
    caps: Capabilities,
    report: Option<CapabilitiesReport>,
    caps_checked_ms: Option<u64>,
    errors: VecDeque<ProviderErrorEntry>,
}

impl TelemetryService {
    pub fn new(cpu: Option<Box<dyn CpuSource>>, gpu: Option<Box<dyn GpuSource>>, hw: Box<dyn HardwareSource>) -> Self {
        Self {
            cpu,
            gpu,
            hw,
            last: SystemSnapshot::default(),
            status: ProviderStatus::HelperUnavailable,
            caps: Capabilities::none(),
            report: None,
            caps_checked_ms: None,
            errors: VecDeque::new(),
        }
    }

    pub fn status(&self) -> ProviderStatus {
        self.status
    }

    pub fn capabilities(&self) -> Capabilities {
        self.caps
    }

    pub fn report(&self) -> Option<&CapabilitiesReport> {
        self.report.as_ref()
    }

    pub fn recent_errors(&self) -> Vec<ProviderErrorEntry> {
        self.errors.iter().cloned().collect()
    }

    fn log(&mut self, now_ms: u64, source: &'static str, message: String) {
        // Keep one entry per distinct error, refreshed to its latest occurrence.
        self.errors.retain(|e| !(e.source == source && e.message == message));
        if self.errors.len() == ERROR_LOG_LEN {
            self.errors.pop_front();
        }
        self.errors.push_back(ProviderErrorEntry { timestamp_ms: now_ms, source, message });
    }

    fn hw_ok(&self) -> bool {
        matches!(self.status, ProviderStatus::Connected | ProviderStatus::Simulated)
    }

    /// Re-reads capabilities now (e.g. after the user installs the helper).
    pub fn refresh_capabilities(&mut self, now_ms: u64) {
        self.caps_checked_ms = Some(now_ms);
        let result = catch_unwind(AssertUnwindSafe(|| self.hw.capabilities()))
            .unwrap_or_else(|_| Err(ClientError::Protocol("provider panicked".into())));
        match result {
            Ok(r) => {
                self.status = r.status;
                self.caps = if self.hw_ok() { r.capabilities } else { Capabilities::none() };
                self.report = Some(r);
            }
            Err(e) => {
                self.status = e.status();
                self.caps = Capabilities::none();
                self.report = None;
                self.log(now_ms, "helper", e.user_message());
            }
        }
    }

    fn hw_call<T>(
        &mut self,
        now_ms: u64,
        f: impl FnOnce(&mut dyn HardwareSource) -> Result<T, ClientError>,
    ) -> Option<T> {
        let result = catch_unwind(AssertUnwindSafe(|| f(self.hw.as_mut())))
            .unwrap_or_else(|_| Err(ClientError::Protocol("provider panicked".into())));
        match result {
            Ok(v) => Some(v),
            Err(e) => {
                if let ClientError::Transport(_) = e {
                    self.status = e.status();
                    self.caps = Capabilities::none();
                }
                self.log(now_ms, "helper", e.user_message());
                None
            }
        }
    }

    pub fn poll(&mut self, due: DueSet, opts: PollOptions, now_ms: u64) -> SystemSnapshot {
        let retry = !self.hw_ok() && self.caps_checked_ms.is_none_or(|t| now_ms.saturating_sub(t) >= CAPS_RETRY_MS);
        if self.caps_checked_ms.is_none() || retry {
            self.refresh_capabilities(now_ms);
        }
        let mut s = self.last.clone();
        s.timestamp_ms = now_ms;

        if due.utilisation || due.clocks {
            if let Some(cpu) = self.cpu.as_mut() {
                match guarded(|| cpu.sample()) {
                    Ok(c) => {
                        s.cpu.utilisation_pct = c.utilisation_pct;
                        s.cpu.frequency_mhz = c.frequency_mhz;
                    }
                    Err(e) => {
                        s.cpu.utilisation_pct = None;
                        s.cpu.frequency_mhz = None;
                        self.log(now_ms, "cpu", e);
                    }
                }
            }
        }

        let mut driver_temp = None;
        let gpu_due = due.utilisation || due.clocks || due.temperatures;
        if opts.gpu_driver && gpu_due {
            if let Some(gpu) = self.gpu.as_mut() {
                match guarded(|| gpu.sample()) {
                    Ok(g) => {
                        s.gpu.utilisation_pct = g.utilisation_pct;
                        s.gpu.frequency_mhz = g.frequency_mhz;
                        s.gpu.power_w = g.power_w;
                        driver_temp = g.temperature_c;
                    }
                    Err(e) => {
                        s.gpu.utilisation_pct = None;
                        s.gpu.frequency_mhz = None;
                        s.gpu.power_w = None;
                        self.log(now_ms, "gpu", e);
                    }
                }
            }
        } else if !opts.gpu_driver {
            s.gpu.utilisation_pct = None;
            s.gpu.frequency_mhz = None;
            s.gpu.power_w = None;
        }

        if due.temperatures || due.fans {
            let readings = if self.hw_ok() { self.hw_call(now_ms, |h| h.sensors()) } else { None };
            let r = readings.unwrap_or_default();
            s.cpu.temperature_c = r.cpu_temp_c;
            s.gpu.temperature_c = r.gpu_temp_c.or(driver_temp);
            let duty = |f: Option<FanTelemetry>| f.and_then(|f| f.duty_pct);
            s.cpu_fan = (opts.fans && self.caps.cpu_fan)
                .then(|| FanTelemetry { rpm: r.cpu_fan_rpm, duty_pct: duty(s.cpu_fan) });
            s.gpu_fan = (opts.fans && self.caps.gpu_fan)
                .then(|| FanTelemetry { rpm: r.gpu_fan_rpm, duty_pct: duty(s.gpu_fan) });
        } else if driver_temp.is_some() && s.gpu.temperature_c.is_none() {
            s.gpu.temperature_c = driver_temp;
        }

        if due.modes {
            if self.hw_ok() {
                let fan = self.hw_call(now_ms, |h| h.fan_state());
                let perf = self.hw_call(now_ms, |h| h.performance());
                s.fan_mode = fan.and_then(|f| f.mode);
                let duty = |d: Option<u8>| d.map(|v| f32::from(v.min(100)));
                if let (Some(f), Some(cf)) = (fan, s.cpu_fan.as_mut()) {
                    cf.duty_pct = duty(f.cpu_duty_pct);
                }
                if let (Some(f), Some(gf)) = (fan, s.gpu_fan.as_mut()) {
                    gf.duty_pct = duty(f.gpu_duty_pct);
                }
                s.performance_mode = perf.and_then(|p| p.mode);
                s.performance_raw = perf.and_then(|p| if p.mode.is_none() { p.raw } else { None });
            } else {
                s.fan_mode = None;
                s.performance_mode = None;
                s.performance_raw = None;
            }
        }

        self.last = s.clone();
        s
    }
}
