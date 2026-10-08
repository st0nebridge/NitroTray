//! @module acer::backend
//! @description Hardware-backend contract used by the helper service, plus the state types it returns.
//!
//! @input  Implemented by the WMI-backed `AcerGaming` and by the development `SimulatedBackend`.
//! @output Sensor readings, fan state, performance state and control results.
//! @dependencies serde, controls, acer::discovery (Capabilities), acer::error
use serde::{Deserialize, Serialize};

use crate::acer::discovery::Capabilities;
pub use crate::acer::error::AcerError;
use crate::controls::{FanMode, PerformanceMode};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct SensorReadings {
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub cpu_fan_rpm: Option<u32>,
    pub gpu_fan_rpm: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct FanState {
    /// `None` when the two fans disagree or the value is outside the known modes.
    pub mode: Option<FanMode>,
    /// Commanded duty when known (Max = 100, Custom = last applied).
    pub cpu_duty_pct: Option<u8>,
    pub gpu_duty_pct: Option<u8>,
    /// True while the helper's custom-curve controller is driving the fans.
    pub curve_active: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PerformanceState {
    pub mode: Option<PerformanceMode>,
    /// Raw firmware profile byte, reported even when it has no popup mode (e.g. Turbo 0x05).
    pub raw: Option<u8>,
}

/// Everything the helper needs from the hardware. Implementations must never panic on
/// firmware errors; they return `AcerError` instead.
pub trait HardwareBackend {
    /// Probe the machine and report what can be read and controlled.
    fn discover(&mut self) -> Capabilities;
    fn sensors(&mut self) -> Result<SensorReadings, AcerError>;
    fn fan_mode(&mut self) -> Result<Option<FanMode>, AcerError>;
    fn set_fan_mode(&mut self, mode: FanMode) -> Result<(), AcerError>;
    /// Set both fans' custom duty; the backend must already be in custom mode.
    fn set_fan_duty(&mut self, cpu_pct: u8, gpu_pct: u8) -> Result<(), AcerError>;
    fn performance(&mut self) -> Result<PerformanceState, AcerError>;
    fn set_performance(&mut self, mode: PerformanceMode) -> Result<(), AcerError>;
    /// Short provider name for diagnostics ("acer-wmi", "simulated").
    fn provider_name(&self) -> &'static str;
}

impl<T: HardwareBackend + ?Sized> HardwareBackend for Box<T> {
    fn discover(&mut self) -> Capabilities {
        (**self).discover()
    }
    fn sensors(&mut self) -> Result<SensorReadings, AcerError> {
        (**self).sensors()
    }
    fn fan_mode(&mut self) -> Result<Option<FanMode>, AcerError> {
        (**self).fan_mode()
    }
    fn set_fan_mode(&mut self, mode: FanMode) -> Result<(), AcerError> {
        (**self).set_fan_mode(mode)
    }
    fn set_fan_duty(&mut self, cpu_pct: u8, gpu_pct: u8) -> Result<(), AcerError> {
        (**self).set_fan_duty(cpu_pct, gpu_pct)
    }
    fn performance(&mut self) -> Result<PerformanceState, AcerError> {
        (**self).performance()
    }
    fn set_performance(&mut self, mode: PerformanceMode) -> Result<(), AcerError> {
        (**self).set_performance(mode)
    }
    fn provider_name(&self) -> &'static str {
        (**self).provider_name()
    }
}

/// Backend used when the firmware interface could not be opened: every call reports why.
#[derive(Debug, Clone)]
pub struct UnavailableBackend(pub AcerError);

impl HardwareBackend for UnavailableBackend {
    fn discover(&mut self) -> Capabilities {
        Capabilities::none()
    }
    fn sensors(&mut self) -> Result<SensorReadings, AcerError> {
        Err(self.0.clone())
    }
    fn fan_mode(&mut self) -> Result<Option<FanMode>, AcerError> {
        Err(self.0.clone())
    }
    fn set_fan_mode(&mut self, _mode: FanMode) -> Result<(), AcerError> {
        Err(self.0.clone())
    }
    fn set_fan_duty(&mut self, _cpu_pct: u8, _gpu_pct: u8) -> Result<(), AcerError> {
        Err(self.0.clone())
    }
    fn performance(&mut self) -> Result<PerformanceState, AcerError> {
        Err(self.0.clone())
    }
    fn set_performance(&mut self, _mode: PerformanceMode) -> Result<(), AcerError> {
        Err(self.0.clone())
    }
    fn provider_name(&self) -> &'static str {
        "unavailable"
    }
}
