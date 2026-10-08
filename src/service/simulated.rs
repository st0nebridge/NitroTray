//! @module service::simulated
//! @description Development-only simulated hardware backend (`console --simulate`); reports provider "simulated".
//!
//! @input  Mode changes and duty writes, like real firmware.
//! @output Plausible, deterministic temperatures and RPM so the full tray ↔ helper chain can be exercised
//!         without elevation. The UI labels this provider as simulated; it is never used implicitly.
//! @dependencies acer::backend, acer::discovery, controls
use crate::acer::backend::{AcerError, HardwareBackend, PerformanceState, SensorReadings};
use crate::acer::discovery::Capabilities;
use crate::controls::{FanMode, PerformanceMode};

pub const SIM_CPU_FAN_MAX_RPM: f32 = 7400.0;
pub const SIM_GPU_FAN_MAX_RPM: f32 = 7700.0;

#[derive(Debug, Clone)]
pub struct SimulatedBackend {
    step: u64,
    fan_mode: FanMode,
    performance: PerformanceMode,
    duty: (u8, u8),
}

impl Default for SimulatedBackend {
    fn default() -> Self {
        Self { step: 0, fan_mode: FanMode::Auto, performance: PerformanceMode::Default, duty: (50, 50) }
    }
}

impl SimulatedBackend {
    fn load_offset(&self) -> f32 {
        match self.performance {
            PerformanceMode::Quiet => -8.0,
            PerformanceMode::Default => 0.0,
            PerformanceMode::Performance => 9.0,
        }
    }

    fn temps(&self) -> (f32, f32) {
        let t = self.step as f32;
        let cpu = 72.0 + self.load_offset() + 9.0 * (t / 17.0).sin() + 3.0 * (t / 5.0).sin();
        let gpu = 68.0 + self.load_offset() * 0.7 + 7.0 * (t / 23.0).sin();
        (cpu.round(), gpu.round())
    }

    fn rpm(&self, temp: f32, max: f32, duty: u8) -> u32 {
        let r = match self.fan_mode {
            FanMode::Max => max,
            FanMode::Custom => max * f32::from(duty) / 100.0,
            FanMode::Auto => ((temp - 40.0) / 50.0).clamp(0.25, 1.0) * max * 0.97,
        };
        r.round() as u32
    }
}

impl HardwareBackend for SimulatedBackend {
    fn discover(&mut self) -> Capabilities {
        Capabilities {
            cpu_fan: true,
            gpu_fan: true,
            auto_fan_mode: true,
            max_fan_mode: true,
            custom_fan_mode: true,
            quiet_mode: true,
            default_mode: true,
            performance_mode: true,
            cpu_temperature: true,
            gpu_temperature: true,
        }
    }

    fn sensors(&mut self) -> Result<SensorReadings, AcerError> {
        self.step += 1;
        let (cpu, gpu) = self.temps();
        Ok(SensorReadings {
            cpu_temp_c: Some(cpu),
            gpu_temp_c: Some(gpu),
            cpu_fan_rpm: Some(self.rpm(cpu, SIM_CPU_FAN_MAX_RPM, self.duty.0)),
            gpu_fan_rpm: Some(self.rpm(gpu, SIM_GPU_FAN_MAX_RPM, self.duty.1)),
        })
    }

    fn fan_mode(&mut self) -> Result<Option<FanMode>, AcerError> {
        Ok(Some(self.fan_mode))
    }

    fn set_fan_mode(&mut self, mode: FanMode) -> Result<(), AcerError> {
        self.fan_mode = mode;
        Ok(())
    }

    fn set_fan_duty(&mut self, cpu_pct: u8, gpu_pct: u8) -> Result<(), AcerError> {
        if cpu_pct > 100 || gpu_pct > 100 {
            return Err(AcerError::InvalidArgument("duty > 100".into()));
        }
        self.duty = (cpu_pct, gpu_pct);
        Ok(())
    }

    fn performance(&mut self) -> Result<PerformanceState, AcerError> {
        Ok(PerformanceState { mode: Some(self.performance), raw: Some(self.performance.firmware_profile()) })
    }

    fn set_performance(&mut self, mode: PerformanceMode) -> Result<(), AcerError> {
        self.performance = mode;
        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        "simulated"
    }
}
