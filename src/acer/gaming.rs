//! @module acer::gaming
//! @description `HardwareBackend` over the AcerGamingFunction WMI methods, via an injectable executor.
//!
//! @input  A `WmiExecutor` (real COM/WMI on Windows, a scripted fake in tests) and the machine model.
//! @output Decoded sensors, fan and performance state; firmware writes with status checking.
//! @dependencies acer::acpi, acer::backend, acer::discovery, controls
use crate::acer::acpi::{self, FanId, Sensor};
use crate::acer::backend::{AcerError, HardwareBackend, PerformanceState, SensorReadings};
use crate::acer::discovery::{self, Capabilities, ProbeResults};
use crate::controls::{FanMode, PerformanceMode};

/// Transport for the two WMI method shapes the gaming class uses.
pub trait WmiExecutor {
    /// `Get*` methods: `gmInput` uint32 → `gmOutput` uint64.
    fn get(&mut self, method: &str, input: u32) -> Result<u64, AcerError>;
    /// `Set*` methods: `gmInput` uint64 → `gmOutput` uint32 status.
    fn set(&mut self, method: &str, input: u64) -> Result<u32, AcerError>;
}

pub struct AcerGaming<E: WmiExecutor> {
    exec: E,
    model: String,
    caps: Option<Capabilities>,
}

impl<E: WmiExecutor> AcerGaming<E> {
    pub fn new(exec: E, model: impl Into<String>) -> Self {
        Self { exec, model: model.into(), caps: None }
    }

    fn get(&mut self, method: &str, input: u32) -> Result<u64, AcerError> {
        self.exec.get(method, input)
    }

    fn set(&mut self, method: &str, input: u64) -> Result<(), AcerError> {
        let out = self.exec.set(method, input)?;
        acpi::decode_set_status(out).map_err(|s| AcerError::Firmware(s.0))
    }

    pub fn read_sensor(&mut self, sensor: Sensor) -> Result<u16, AcerError> {
        let out = self.get(acpi::METHOD_GET_SYS_INFO, acpi::sensor_reading_input(sensor))?;
        acpi::decode_sensor_reading(out).map_err(|s| AcerError::Firmware(s.0))
    }

    fn caps(&mut self) -> Capabilities {
        if self.caps.is_none() {
            let caps = self.discover();
            self.caps = Some(caps);
        }
        self.caps.unwrap_or_default()
    }

    fn probe(&mut self) -> ProbeResults {
        let sensors = self
            .get(acpi::METHOD_GET_SYS_INFO, acpi::supported_sensors_input())
            .and_then(|o| acpi::decode_supported_sensors(o).map_err(|s| AcerError::Firmware(s.0)));
        let fan_behavior = self
            .get(acpi::METHOD_GET_FAN_BEHAVIOR, acpi::fan_behavior_get_input(acpi::FAN_BITS_BOTH))
            .and_then(|o| acpi::decode_fan_behavior(o, acpi::FAN_BITS_BOTH).map_err(|s| AcerError::Firmware(s.0)));
        let fan_speed = self
            .get(acpi::METHOD_GET_FAN_SPEED, acpi::fan_speed_get_input(FanId::Cpu))
            .and_then(|o| acpi::decode_fan_speed(o).map_err(|s| AcerError::Firmware(s.0)));
        let profiles = self
            .get(acpi::METHOD_GET_MISC_SETTING, acpi::misc_get_input(acpi::MISC_SUPPORTED_PROFILES))
            .and_then(|o| acpi::decode_misc(o).map_err(|s| AcerError::Firmware(s.0)));
        let current_profile = self
            .get(acpi::METHOD_GET_MISC_SETTING, acpi::misc_get_input(acpi::MISC_PLATFORM_PROFILE))
            .and_then(|o| acpi::decode_misc(o).map_err(|s| AcerError::Firmware(s.0)));
        ProbeResults { model: self.model.clone(), sensors, fan_behavior, fan_speed, profiles, current_profile }
    }
}

impl<E: WmiExecutor> HardwareBackend for AcerGaming<E> {
    fn discover(&mut self) -> Capabilities {
        let caps = discovery::derive(&self.probe());
        self.caps = Some(caps);
        caps
    }

    fn sensors(&mut self) -> Result<SensorReadings, AcerError> {
        let caps = self.caps();
        let mut r = SensorReadings::default();
        let mut last_err = None;
        let mut read = |me: &mut Self, s: Sensor| match me.read_sensor(s) {
            Ok(v) => Some(v),
            Err(e) => {
                last_err = Some(e);
                None
            }
        };
        if caps.cpu_temperature {
            r.cpu_temp_c = read(self, Sensor::CpuTemperature).map(f32::from);
        }
        if caps.gpu_temperature {
            r.gpu_temp_c = read(self, Sensor::GpuTemperature).map(f32::from);
        }
        if caps.cpu_fan {
            r.cpu_fan_rpm = read(self, Sensor::CpuFanSpeed).map(u32::from);
        }
        if caps.gpu_fan {
            r.gpu_fan_rpm = read(self, Sensor::GpuFanSpeed).map(u32::from);
        }
        match (r == SensorReadings::default(), last_err) {
            (true, Some(e)) => Err(e),
            _ => Ok(r),
        }
    }

    fn fan_mode(&mut self) -> Result<Option<FanMode>, AcerError> {
        let out = self.get(acpi::METHOD_GET_FAN_BEHAVIOR, acpi::fan_behavior_get_input(acpi::FAN_BITS_BOTH))?;
        let b = acpi::decode_fan_behavior(out, acpi::FAN_BITS_BOTH).map_err(|s| AcerError::Firmware(s.0))?;
        Ok(match (b.cpu, b.gpu) {
            (Some(c), Some(g)) if c == g => FanMode::from_firmware_behavior(c),
            _ => None,
        })
    }

    fn set_fan_mode(&mut self, mode: FanMode) -> Result<(), AcerError> {
        let caps = self.caps();
        if !caps.supports_fan(mode) {
            return Err(AcerError::Unsupported(match mode {
                FanMode::Auto => "Auto fan mode",
                FanMode::Max => "Max fan mode",
                FanMode::Custom => "Custom fan mode",
            }));
        }
        let input = acpi::fan_behavior_set_input(acpi::FAN_BITS_BOTH, mode.firmware_behavior());
        self.set(acpi::METHOD_SET_FAN_BEHAVIOR, input)
    }

    fn set_fan_duty(&mut self, cpu_pct: u8, gpu_pct: u8) -> Result<(), AcerError> {
        if !self.caps().custom_fan_mode {
            return Err(AcerError::Unsupported("Custom fan duty"));
        }
        for (fan, pct) in [(FanId::Cpu, cpu_pct), (FanId::Gpu, gpu_pct)] {
            let input = acpi::fan_speed_set_input(fan, pct)
                .ok_or_else(|| AcerError::InvalidArgument(format!("duty {pct} > 100")))?;
            self.set(acpi::METHOD_SET_FAN_SPEED, input)?;
        }
        Ok(())
    }

    fn performance(&mut self) -> Result<PerformanceState, AcerError> {
        let out = self.get(acpi::METHOD_GET_MISC_SETTING, acpi::misc_get_input(acpi::MISC_PLATFORM_PROFILE))?;
        let raw = acpi::decode_misc(out).map_err(|s| AcerError::Firmware(s.0))?;
        Ok(PerformanceState { mode: PerformanceMode::from_firmware_profile(raw), raw: Some(raw) })
    }

    fn set_performance(&mut self, mode: PerformanceMode) -> Result<(), AcerError> {
        if !self.caps().supports_performance(mode) {
            return Err(AcerError::Unsupported(match mode {
                PerformanceMode::Quiet => "Quiet mode",
                PerformanceMode::Default => "Default mode",
                PerformanceMode::Performance => "Performance mode",
            }));
        }
        let input = acpi::misc_set_input(acpi::MISC_PLATFORM_PROFILE, mode.firmware_profile());
        self.set(acpi::METHOD_SET_MISC_SETTING, input)
    }

    fn provider_name(&self) -> &'static str {
        "acer-wmi"
    }
}
