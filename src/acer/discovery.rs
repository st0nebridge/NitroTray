//! @module acer::discovery
//! @description Capability discovery: derives what can be read and controlled from probe results.
//!
//! @input  `ProbeResults` gathered by a backend (model name + read-only firmware query results).
//! @output `Capabilities`, from which the UI derives enabled/disabled/read-only states.
//! @dependencies serde, acer::acpi, acer::error, controls
//!
//! Direct fan-duty control is model-gated: only models on `CUSTOM_FAN_MODELS`
//! (verified against upstream `acer-wmi` pwm quirks) get Custom mode.
use serde::{Deserialize, Serialize};

use crate::acer::acpi::{FanBehaviors, ProfileSet, Sensor, SensorSet};
use crate::acer::error::AcerError;
use crate::controls::fan_mode::{BEHAVIOR_AUTO, BEHAVIOR_CUSTOM};
use crate::controls::{FanMode, PerformanceMode};

/// Models whose firmware accepts direct fan-duty writes (Linux acer-wmi `pwm = 1`, verified).
pub const CUSTOM_FAN_MODELS: &[&str] = &["Nitro AN515-58"];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub cpu_fan: bool,
    pub gpu_fan: bool,
    pub auto_fan_mode: bool,
    pub max_fan_mode: bool,
    pub custom_fan_mode: bool,
    pub quiet_mode: bool,
    pub default_mode: bool,
    pub performance_mode: bool,
    pub cpu_temperature: bool,
    pub gpu_temperature: bool,
}

impl Capabilities {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn supports_fan(&self, mode: FanMode) -> bool {
        match mode {
            FanMode::Auto => self.auto_fan_mode,
            FanMode::Max => self.max_fan_mode,
            FanMode::Custom => self.custom_fan_mode,
        }
    }

    pub fn supports_performance(&self, mode: PerformanceMode) -> bool {
        match mode {
            PerformanceMode::Quiet => self.quiet_mode,
            PerformanceMode::Default => self.default_mode,
            PerformanceMode::Performance => self.performance_mode,
        }
    }
}

/// Raw read-only probe results a backend collects for discovery.
#[derive(Debug, Clone)]
pub struct ProbeResults {
    pub model: String,
    pub sensors: Result<SensorSet, AcerError>,
    pub fan_behavior: Result<FanBehaviors, AcerError>,
    pub fan_speed: Result<u8, AcerError>,
    pub profiles: Result<u8, AcerError>,
    pub current_profile: Result<u8, AcerError>,
}

pub fn model_allows_custom_fan(model: &str) -> bool {
    let m = model.trim();
    CUSTOM_FAN_MODELS.iter().any(|known| known.eq_ignore_ascii_case(m))
}

fn behavior_known(v: Option<u8>) -> bool {
    matches!(v, Some(b) if (BEHAVIOR_AUTO..=BEHAVIOR_CUSTOM).contains(&b))
}

/// Fail-closed derivation: anything that did not read back cleanly is reported unsupported.
pub fn derive(p: &ProbeResults) -> Capabilities {
    let mut c = Capabilities::none();
    if let Ok(set) = p.sensors {
        c.cpu_temperature = set.contains(Sensor::CpuTemperature);
        c.gpu_temperature = set.contains(Sensor::GpuTemperature);
        c.cpu_fan = set.contains(Sensor::CpuFanSpeed);
        c.gpu_fan = set.contains(Sensor::GpuFanSpeed);
    }
    if let Ok(b) = p.fan_behavior {
        let fans_ok = behavior_known(b.cpu) && behavior_known(b.gpu);
        c.auto_fan_mode = fans_ok;
        c.max_fan_mode = fans_ok;
        c.custom_fan_mode = fans_ok && p.fan_speed.is_ok() && model_allows_custom_fan(&p.model);
    }
    if let (Ok(bits), Ok(_)) = (&p.profiles, &p.current_profile) {
        let set = ProfileSet(*bits);
        c.quiet_mode = set.contains(PerformanceMode::Quiet.firmware_profile());
        c.default_mode = set.contains(PerformanceMode::Default.firmware_profile());
        c.performance_mode = set.contains(PerformanceMode::Performance.firmware_profile());
    }
    c
}

/// First error that explains why nothing at all was discovered (for the connection status).
pub fn blocking_error(p: &ProbeResults) -> Option<AcerError> {
    p.sensors.as_ref().err().or(p.fan_behavior.as_ref().err()).or(p.current_profile.as_ref().err()).cloned()
}
