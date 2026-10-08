//! @module collector_test
//! @description Telemetry composition: provider isolation, fallbacks, gating by options and status.
use std::sync::{Arc, Mutex};

use nitrotray::acer::{Capabilities, FanState, PerformanceState, SensorReadings};
use nitrotray::controls::{FanMode, PerformanceMode};
use nitrotray::ipc::client::{ClientError, TransportError};
use nitrotray::ipc::protocol::{CapabilitiesReport, ErrorCode, ErrorInfo, ProviderStatus};
use nitrotray::telemetry::scheduler::DueSet;
use nitrotray::telemetry::{CpuSample, CpuSource, GpuSample, GpuSource, HardwareSource, PollOptions, TelemetryService};

use crate::support::all_caps;

struct Cpu(Result<CpuSample, String>);
impl CpuSource for Cpu {
    fn sample(&mut self) -> Result<CpuSample, String> {
        self.0.clone()
    }
}

struct PanickyCpu;
impl CpuSource for PanickyCpu {
    fn sample(&mut self) -> Result<CpuSample, String> {
        panic!("driver exploded")
    }
}

struct Gpu(Result<GpuSample, String>, Arc<Mutex<u32>>);
impl GpuSource for Gpu {
    fn sample(&mut self) -> Result<GpuSample, String> {
        *self.1.lock().unwrap() += 1;
        self.0.clone()
    }
}

#[derive(Clone)]
struct Hw {
    status: ProviderStatus,
    caps: Capabilities,
    sensors: Result<SensorReadings, ClientError>,
    fan: Result<FanState, ClientError>,
    perf: Result<PerformanceState, ClientError>,
    caps_error: Option<ClientError>,
    calls: Arc<Mutex<u32>>,
}

impl Default for Hw {
    fn default() -> Self {
        Self {
            status: ProviderStatus::Connected,
            caps: all_caps(),
            sensors: Ok(SensorReadings {
                cpu_temp_c: Some(85.0),
                gpu_temp_c: Some(82.0),
                cpu_fan_rpm: Some(7317),
                gpu_fan_rpm: Some(7692),
            }),
            fan: Ok(FanState {
                mode: Some(FanMode::Max),
                cpu_duty_pct: Some(100),
                gpu_duty_pct: Some(100),
                curve_active: false,
            }),
            perf: Ok(PerformanceState { mode: Some(PerformanceMode::Default), raw: Some(1) }),
            caps_error: None,
            calls: Arc::new(Mutex::new(0)),
        }
    }
}

impl HardwareSource for Hw {
    fn capabilities(&mut self) -> Result<CapabilitiesReport, ClientError> {
        *self.calls.lock().unwrap() += 1;
        if let Some(e) = &self.caps_error {
            return Err(e.clone());
        }
        Ok(CapabilitiesReport {
            protocol_version: 1,
            provider: "x".into(),
            model: "m".into(),
            status: self.status,
            capabilities: self.caps,
        })
    }
    fn sensors(&mut self) -> Result<SensorReadings, ClientError> {
        self.sensors.clone()
    }
    fn fan_state(&mut self) -> Result<FanState, ClientError> {
        self.fan.clone()
    }
    fn performance(&mut self) -> Result<PerformanceState, ClientError> {
        self.perf.clone()
    }
}

fn opts() -> PollOptions {
    PollOptions { gpu_driver: true, fans: true }
}

fn cpu_ok() -> Option<Box<dyn CpuSource>> {
    Some(Box::new(Cpu(Ok(CpuSample { utilisation_pct: Some(21.0), frequency_mhz: Some(3947) }))))
}

fn gpu_ok(counter: Arc<Mutex<u32>>) -> Option<Box<dyn GpuSource>> {
    Some(Box::new(Gpu(
        Ok(GpuSample {
            temperature_c: Some(81.0),
            utilisation_pct: Some(93.0),
            frequency_mhz: Some(2565),
            power_w: Some(90.0),
        }),
        counter,
    )))
}

#[test]
fn composes_the_reference_snapshot() {
    let mut t = TelemetryService::new(cpu_ok(), gpu_ok(Default::default()), Box::new(Hw::default()));
    let s = t.poll(DueSet::all(), opts(), 1000);
    assert_eq!(s.timestamp_ms, 1000);
    assert_eq!((s.cpu.temperature_c, s.cpu.utilisation_pct, s.cpu.frequency_mhz), (Some(85.0), Some(21.0), Some(3947)));
    assert_eq!((s.gpu.temperature_c, s.gpu.utilisation_pct, s.gpu.frequency_mhz), (Some(82.0), Some(93.0), Some(2565)));
    assert_eq!(s.gpu.power_w, Some(90.0));
    assert_eq!(s.cpu_fan.unwrap().rpm, Some(7317));
    assert_eq!(s.gpu_fan.unwrap().duty_pct, Some(100.0));
    assert_eq!(s.fan_mode, Some(FanMode::Max));
    assert_eq!(s.performance_mode, Some(PerformanceMode::Default));
    assert_eq!(t.status(), ProviderStatus::Connected);
    assert!(t.capabilities().custom_fan_mode);
    assert!(t.report().is_some());
}

#[test]
fn helper_absence_blanks_hardware_readings_but_keeps_os_telemetry() {
    let hw = Hw { caps_error: Some(ClientError::Transport(TransportError::NotRunning)), ..Hw::default() };
    let mut t = TelemetryService::new(cpu_ok(), gpu_ok(Default::default()), Box::new(hw));
    let s = t.poll(DueSet::all(), opts(), 0);
    assert_eq!(t.status(), ProviderStatus::HelperUnavailable);
    assert_eq!(s.cpu.temperature_c, None, "never synthesised");
    assert_eq!(s.gpu.temperature_c, Some(81.0), "driver fallback");
    assert_eq!(s.cpu.utilisation_pct, Some(21.0));
    assert!(s.cpu_fan.is_none() && s.fan_mode.is_none() && s.performance_mode.is_none());
    assert!(t.recent_errors().iter().any(|e| e.source == "helper"));
}

#[test]
fn capabilities_are_retried_only_after_the_interval() {
    let hw = Hw { caps_error: Some(ClientError::Transport(TransportError::NotRunning)), ..Hw::default() };
    let calls = Arc::clone(&hw.calls);
    let mut t = TelemetryService::new(None, None, Box::new(hw));
    t.poll(DueSet::all(), opts(), 0);
    t.poll(DueSet::all(), opts(), 5000);
    assert_eq!(*calls.lock().unwrap(), 1);
    t.poll(DueSet::all(), opts(), 10_000);
    assert_eq!(*calls.lock().unwrap(), 2);
    t.refresh_capabilities(10_001);
    assert_eq!(*calls.lock().unwrap(), 3);
}

#[test]
fn provider_failures_and_panics_are_isolated() {
    let mut t = TelemetryService::new(
        Some(Box::new(PanickyCpu)),
        Some(Box::new(Gpu(Err("nvml".into()), Default::default()))),
        Box::new(Hw::default()),
    );
    let s = t.poll(DueSet::all(), opts(), 0);
    assert_eq!(s.cpu.utilisation_pct, None);
    assert_eq!(s.gpu.utilisation_pct, None);
    assert_eq!(s.cpu.temperature_c, Some(85.0), "helper still works");
    let errs = t.recent_errors();
    assert!(errs.iter().any(|e| e.source == "cpu" && e.message.contains("panicked")));
    assert!(errs.iter().any(|e| e.source == "gpu" && e.message == "nvml"));
    t.poll(DueSet::all(), opts(), 1);
    assert_eq!(t.recent_errors().len(), errs.len(), "recurring errors are not duplicated");
    assert!(t.recent_errors().iter().all(|e| e.timestamp_ms == 1), "refreshed to the latest occurrence");
}

#[test]
fn cpu_error_is_logged_and_blanks_cpu_readings() {
    let mut t = TelemetryService::new(Some(Box::new(Cpu(Err("pdh".into())))), None, Box::new(Hw::default()));
    let s = t.poll(DueSet::all(), opts(), 0);
    assert_eq!((s.cpu.utilisation_pct, s.cpu.frequency_mhz), (None, None));
}

#[test]
fn gpu_driver_is_not_touched_when_disabled_or_not_due() {
    let counter = Arc::new(Mutex::new(0));
    let mut t = TelemetryService::new(None, gpu_ok(Arc::clone(&counter)), Box::new(Hw::default()));
    let s = t.poll(DueSet::all(), PollOptions { gpu_driver: false, fans: true }, 0);
    assert_eq!(*counter.lock().unwrap(), 0);
    assert_eq!(s.gpu.utilisation_pct, None);
    let modes_only = DueSet { modes: true, ..DueSet::default() };
    t.poll(modes_only, opts(), 1);
    assert_eq!(*counter.lock().unwrap(), 0);
    let fans_only = DueSet { fans: true, ..DueSet::default() };
    t.poll(fans_only, opts(), 2);
    assert_eq!(*counter.lock().unwrap(), 0, "fan-only ticks must not wake the GPU");
}

#[test]
fn fans_disabled_or_unsupported_hide_fan_rows() {
    let mut t = TelemetryService::new(None, None, Box::new(Hw::default()));
    let s = t.poll(DueSet::all(), PollOptions { gpu_driver: false, fans: false }, 0);
    assert!(s.cpu_fan.is_none() && s.gpu_fan.is_none());
    let mut caps = all_caps();
    caps.gpu_fan = false;
    let mut t = TelemetryService::new(None, None, Box::new(Hw { caps, ..Hw::default() }));
    let s = t.poll(DueSet::all(), opts(), 0);
    assert!(s.cpu_fan.is_some() && s.gpu_fan.is_none());
}

#[test]
fn transport_loss_mid_session_downgrades_status() {
    let hw = Hw { sensors: Err(ClientError::Transport(TransportError::NotRunning)), ..Hw::default() };
    let mut t = TelemetryService::new(None, None, Box::new(hw));
    let s = t.poll(DueSet::all(), opts(), 0);
    assert_eq!(t.status(), ProviderStatus::HelperUnavailable);
    assert_eq!(t.capabilities(), Capabilities::none());
    assert_eq!(s.cpu.temperature_c, None);
}

#[test]
fn remote_errors_keep_status_but_blank_values() {
    let denied = ClientError::Remote(ErrorInfo::new(ErrorCode::FirmwareRejected, "x"));
    let hw = Hw { fan: Err(denied.clone()), perf: Err(denied), ..Hw::default() };
    let mut t = TelemetryService::new(None, None, Box::new(hw));
    let s = t.poll(DueSet::all(), opts(), 0);
    assert_eq!(t.status(), ProviderStatus::Connected);
    assert_eq!((s.fan_mode, s.performance_mode), (None, None));
}

#[test]
fn unexposed_profile_is_reported_raw() {
    let hw = Hw { perf: Ok(PerformanceState { mode: None, raw: Some(5) }), ..Hw::default() };
    let mut t = TelemetryService::new(None, None, Box::new(hw));
    let s = t.poll(DueSet::all(), opts(), 0);
    assert_eq!((s.performance_mode, s.performance_raw), (None, Some(5)));
}

#[test]
fn access_denied_helper_yields_no_capabilities() {
    let hw = Hw { status: ProviderStatus::AccessDenied, ..Hw::default() };
    let mut t = TelemetryService::new(None, None, Box::new(hw));
    let s = t.poll(DueSet::all(), opts(), 0);
    assert_eq!(t.status(), ProviderStatus::AccessDenied);
    assert_eq!(t.capabilities(), Capabilities::none());
    assert!(s.cpu.temperature_c.is_none() && s.fan_mode.is_none());
}

#[test]
fn stale_values_persist_between_due_ticks() {
    let mut t = TelemetryService::new(cpu_ok(), None, Box::new(Hw::default()));
    t.poll(DueSet::all(), opts(), 0);
    let s = t.poll(DueSet { utilisation: true, ..DueSet::default() }, opts(), 500);
    assert_eq!(s.cpu.temperature_c, Some(85.0));
    assert_eq!(s.fan_mode, Some(FanMode::Max));
}

#[test]
fn driver_temperature_fills_a_missing_helper_gpu_temperature() {
    let hw = Hw { sensors: Ok(SensorReadings { gpu_temp_c: None, ..SensorReadings::default() }), ..Hw::default() };
    let mut t = TelemetryService::new(None, gpu_ok(Default::default()), Box::new(hw));
    let s = t.poll(DueSet::all(), opts(), 0);
    assert_eq!(s.gpu.temperature_c, Some(81.0));
    let s = t.poll(DueSet { utilisation: true, ..DueSet::default() }, opts(), 500);
    assert_eq!(s.gpu.temperature_c, Some(81.0));
}

#[test]
fn distinct_messages_from_one_source_are_all_kept() {
    // Same source, different message: both must survive in one log.
    let hw = Hw {
        sensors: Err(ClientError::Protocol("a".into())),
        fan: Err(ClientError::Protocol("b".into())),
        ..Hw::default()
    };
    let mut t3 = TelemetryService::new(None, None, Box::new(hw));
    t3.poll(DueSet::all(), opts(), 0);
    let helper: Vec<_> = t3.recent_errors().into_iter().filter(|e| e.source == "helper").map(|e| e.message).collect();
    assert_eq!(helper.len(), 2, "{helper:?}");
}

/// CPU source whose utilisation increases on every sample, so staleness is detectable.
struct CountingCpu(u32);
impl CpuSource for CountingCpu {
    fn sample(&mut self) -> Result<CpuSample, String> {
        self.0 += 1;
        Ok(CpuSample { utilisation_pct: Some(self.0 as f32), frequency_mhz: Some(1000 + self.0) })
    }
}

/// Helper source counting sensor reads.
struct CountingHw(Arc<Mutex<u32>>);
impl HardwareSource for CountingHw {
    fn capabilities(&mut self) -> Result<CapabilitiesReport, ClientError> {
        Hw::default().capabilities()
    }
    fn sensors(&mut self) -> Result<SensorReadings, ClientError> {
        *self.0.lock().unwrap() += 1;
        Hw::default().sensors
    }
    fn fan_state(&mut self) -> Result<FanState, ClientError> {
        Hw::default().fan
    }
    fn performance(&mut self) -> Result<PerformanceState, ClientError> {
        Hw::default().perf
    }
}

#[test]
fn each_due_flag_triggers_its_provider_on_its_own() {
    let mut t = TelemetryService::new(Some(Box::new(CountingCpu(0))), None, Box::new(Hw::default()));
    t.poll(DueSet::default(), opts(), 0);
    assert_eq!(t.poll(DueSet { utilisation: true, ..DueSet::default() }, opts(), 1).cpu.utilisation_pct, Some(1.0));
    assert_eq!(t.poll(DueSet { clocks: true, ..DueSet::default() }, opts(), 2).cpu.frequency_mhz, Some(1002));

    let gpu_calls = Arc::new(Mutex::new(0));
    let mut g = TelemetryService::new(None, gpu_ok(Arc::clone(&gpu_calls)), Box::new(Hw::default()));
    g.poll(DueSet { utilisation: true, ..DueSet::default() }, opts(), 0);
    g.poll(DueSet { clocks: true, ..DueSet::default() }, opts(), 1);
    g.poll(DueSet { temperatures: true, ..DueSet::default() }, opts(), 2);
    assert_eq!(*gpu_calls.lock().unwrap(), 3, "any single visible metric group samples the GPU driver");

    let sensor_calls = Arc::new(Mutex::new(0));
    let mut h = TelemetryService::new(None, None, Box::new(CountingHw(Arc::clone(&sensor_calls))));
    h.poll(DueSet { fans: true, ..DueSet::default() }, opts(), 0);
    h.poll(DueSet { temperatures: true, ..DueSet::default() }, opts(), 1);
    assert_eq!(*sensor_calls.lock().unwrap(), 2, "fans alone and temperatures alone both read sensors");
}

#[test]
fn disabling_the_gpu_driver_blanks_previous_driver_values() {
    let mut t = TelemetryService::new(None, gpu_ok(Default::default()), Box::new(Hw::default()));
    let on = t.poll(DueSet::all(), opts(), 0);
    assert_eq!(on.gpu.utilisation_pct, Some(93.0));
    let off = t.poll(DueSet::default(), PollOptions { gpu_driver: false, fans: true }, 1);
    assert_eq!((off.gpu.utilisation_pct, off.gpu.frequency_mhz, off.gpu.power_w), (None, None, None));
}
