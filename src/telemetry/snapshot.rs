//! @module telemetry::snapshot
//! @description Immutable telemetry snapshot pushed from the telemetry service to the UI.
//!
//! @output `SystemSnapshot` and its per-device parts; every reading is optional and never synthesised.
//! @dependencies serde, controls (FanMode, PerformanceMode)
use serde::{Deserialize, Serialize};

use crate::controls::{FanMode, PerformanceMode};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CpuTelemetry {
    pub temperature_c: Option<f32>,
    pub utilisation_pct: Option<f32>,
    pub frequency_mhz: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct GpuTelemetry {
    pub temperature_c: Option<f32>,
    pub utilisation_pct: Option<f32>,
    pub frequency_mhz: Option<u32>,
    /// Board power, monitoring view only.
    pub power_w: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct FanTelemetry {
    pub rpm: Option<u32>,
    /// Commanded duty, present only when it is actually known (Max or Custom mode).
    pub duty_pct: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemSnapshot {
    /// Milliseconds since the telemetry service started.
    pub timestamp_ms: u64,
    pub cpu: CpuTelemetry,
    pub gpu: GpuTelemetry,
    pub cpu_fan: Option<FanTelemetry>,
    pub gpu_fan: Option<FanTelemetry>,
    pub fan_mode: Option<FanMode>,
    pub performance_mode: Option<PerformanceMode>,
    /// Raw firmware profile when it is one the popup does not expose (e.g. Turbo).
    pub performance_raw: Option<u8>,
}

impl SystemSnapshot {
    /// True when no reading of any kind is present.
    pub fn is_empty(&self) -> bool {
        self.cpu == CpuTelemetry::default()
            && self.gpu == GpuTelemetry::default()
            && self.cpu_fan.is_none()
            && self.gpu_fan.is_none()
            && self.fan_mode.is_none()
            && self.performance_mode.is_none()
    }
}
