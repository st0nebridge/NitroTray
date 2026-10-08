//! @module support
//! @description Shared test doubles: a scripted WMI executor and a programmable hardware backend.
#![allow(dead_code)]
use std::collections::{HashMap, VecDeque};

use nitrotray::acer::backend::{AcerError, HardwareBackend, PerformanceState, SensorReadings};
use nitrotray::acer::gaming::WmiExecutor;
use nitrotray::acer::Capabilities;
use nitrotray::controls::{FanMode, PerformanceMode};

/// Replays canned WMI outputs keyed by (method, input) and records every call.
#[derive(Default)]
pub struct ScriptedExecutor {
    pub gets: HashMap<(String, u32), Result<u64, AcerError>>,
    pub sets: HashMap<String, Result<u32, AcerError>>,
    pub calls: Vec<(String, u64)>,
}

impl ScriptedExecutor {
    /// Outputs captured from the AN515-58 read-only probe on 2026-09-30.
    pub fn an515_58() -> Self {
        let mut e = Self::default();
        let g = |m: &str, i: u32, o: u64| ((m.to_string(), i), Ok(o));
        e.gets.extend([
            g("GetGamingSysInfo", 0x0000, 0x2_2700_0000),
            g("GetGamingSysInfo", 0x0101, 0x4100),
            g("GetGamingSysInfo", 0x0201, 0x16_8900),
            g("GetGamingSysInfo", 0x0601, 0x17_EA00),
            g("GetGamingSysInfo", 0x0A01, 0x3100),
            g("GetGamingFanBehavior", 0x09, 0x4100),
            g("GetGamingFanSpeed", 0x01, 0x3200),
            g("GetGamingFanSpeed", 0x04, 0x3200),
            g("GetGamingMiscSetting", 0x0A, 0x3300),
            g("GetGamingMiscSetting", 0x0B, 0x0400),
        ]);
        for m in ["SetGamingFanBehavior", "SetGamingFanSpeed", "SetGamingMiscSetting"] {
            e.sets.insert(m.to_string(), Ok(0));
        }
        e
    }
}

impl WmiExecutor for ScriptedExecutor {
    fn get(&mut self, method: &str, input: u32) -> Result<u64, AcerError> {
        self.calls.push((method.to_string(), u64::from(input)));
        self.gets.get(&(method.to_string(), input)).cloned().unwrap_or(Err(AcerError::Firmware(0xFF)))
    }

    fn set(&mut self, method: &str, input: u64) -> Result<u32, AcerError> {
        self.calls.push((method.to_string(), input));
        // Emulate firmware state for read-back of set operations.
        match method {
            "SetGamingFanBehavior" => {
                let cpu = (input >> 16) & 0x3;
                let gpu = (input >> 22) & 0x3;
                self.gets.insert(("GetGamingFanBehavior".into(), 0x09), Ok((cpu << 8) | (gpu << 14)));
            }
            "SetGamingMiscSetting" if input & 0xFF == 0x0B => {
                self.gets.insert(("GetGamingMiscSetting".into(), 0x0B), Ok(input & 0xFF00));
            }
            _ => {}
        }
        self.sets.get(method).cloned().unwrap_or(Ok(0))
    }
}

/// Backend whose every answer the test controls.
pub struct FakeBackend {
    pub caps: Capabilities,
    pub sensors: VecDeque<Result<SensorReadings, AcerError>>,
    pub default_sensors: Result<SensorReadings, AcerError>,
    pub fan_mode: Option<FanMode>,
    pub fan_mode_error: Option<AcerError>,
    pub ignore_fan_writes: bool,
    pub duty_error: Option<AcerError>,
    pub duties: Vec<(u8, u8)>,
    pub fan_writes: Vec<FanMode>,
    pub performance: PerformanceState,
    pub perf_error: Option<AcerError>,
    pub ignore_perf_writes: bool,
}

pub fn all_caps() -> Capabilities {
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

pub fn readings(cpu: f32, gpu: f32) -> SensorReadings {
    SensorReadings { cpu_temp_c: Some(cpu), gpu_temp_c: Some(gpu), cpu_fan_rpm: Some(4000), gpu_fan_rpm: Some(4200) }
}

impl Default for FakeBackend {
    fn default() -> Self {
        Self {
            caps: all_caps(),
            sensors: VecDeque::new(),
            default_sensors: Ok(readings(60.0, 55.0)),
            fan_mode: Some(FanMode::Auto),
            fan_mode_error: None,
            ignore_fan_writes: false,
            duty_error: None,
            duties: vec![],
            fan_writes: vec![],
            performance: PerformanceState { mode: Some(PerformanceMode::Default), raw: Some(1) },
            perf_error: None,
            ignore_perf_writes: false,
        }
    }
}

impl HardwareBackend for FakeBackend {
    fn discover(&mut self) -> Capabilities {
        self.caps
    }
    fn sensors(&mut self) -> Result<SensorReadings, AcerError> {
        self.sensors.pop_front().unwrap_or_else(|| self.default_sensors.clone())
    }
    fn fan_mode(&mut self) -> Result<Option<FanMode>, AcerError> {
        match &self.fan_mode_error {
            Some(e) => Err(e.clone()),
            None => Ok(self.fan_mode),
        }
    }
    fn set_fan_mode(&mut self, mode: FanMode) -> Result<(), AcerError> {
        self.fan_writes.push(mode);
        if !self.ignore_fan_writes {
            self.fan_mode = Some(mode);
        }
        Ok(())
    }
    fn set_fan_duty(&mut self, cpu_pct: u8, gpu_pct: u8) -> Result<(), AcerError> {
        if let Some(e) = &self.duty_error {
            return Err(e.clone());
        }
        self.duties.push((cpu_pct, gpu_pct));
        Ok(())
    }
    fn performance(&mut self) -> Result<PerformanceState, AcerError> {
        match &self.perf_error {
            Some(e) => Err(e.clone()),
            None => Ok(self.performance),
        }
    }
    fn set_performance(&mut self, mode: PerformanceMode) -> Result<(), AcerError> {
        if !self.ignore_perf_writes {
            self.performance = PerformanceState { mode: Some(mode), raw: Some(mode.firmware_profile()) };
        }
        Ok(())
    }
    fn provider_name(&self) -> &'static str {
        "fake"
    }
}

/// Unique pipe name per test so parallel tests never collide.
pub fn unique_pipe(tag: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    format!(r"\\.\pipe\NitroTrayTest-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))
}
