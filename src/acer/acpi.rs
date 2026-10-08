//! @module acer::acpi
//! @description Bit-level codec for the AcerGamingFunction WMI methods (inputs to build, outputs to decode).
//!
//! @input  Sensor ids, fan ids/bitmaps, modes, misc-setting indices; raw `gmOutput` integers.
//! @output WMI `gmInput` integers and decoded readings, or the firmware status byte on failure.
//! @dependencies none (pure)
//!
//! Layouts from upstream Linux `drivers/platform/x86/acer-wmi.c` and verified on a Nitro AN515-58
//! with an elevated read-only probe (2026-09-30): supported sensors 0x227 (CPU temp, CPU fan,
//! ext temp, GPU fan, GPU temp), CPU 65 °C = 0x4100, CPU fan 5769 RPM = 0x168900,
//! fan behaviour 0x4100 = both Auto, supported profiles 0x33, current profile 0x04.

pub const METHOD_GET_SYS_INFO: &str = "GetGamingSysInfo";
pub const METHOD_GET_FAN_BEHAVIOR: &str = "GetGamingFanBehavior";
pub const METHOD_SET_FAN_BEHAVIOR: &str = "SetGamingFanBehavior";
pub const METHOD_GET_FAN_SPEED: &str = "GetGamingFanSpeed";
pub const METHOD_SET_FAN_SPEED: &str = "SetGamingFanSpeed";
pub const METHOD_GET_MISC_SETTING: &str = "GetGamingMiscSetting";
pub const METHOD_SET_MISC_SETTING: &str = "SetGamingMiscSetting";

const STATUS_MASK: u64 = 0xFF;
const CMD_SUPPORTED_SENSORS: u32 = 0x0000;
const CMD_SENSOR_READING: u32 = 0x0001;

/// Predator-v4 sensor ids (`acer_wmi_predator_v4_sensor_id`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sensor {
    CpuTemperature = 0x01,
    CpuFanSpeed = 0x02,
    ExternalTemperature2 = 0x03,
    GpuFanSpeed = 0x06,
    GpuTemperature = 0x0A,
}

impl Sensor {
    pub const ALL: [Sensor; 5] =
        [Self::CpuTemperature, Self::CpuFanSpeed, Self::ExternalTemperature2, Self::GpuFanSpeed, Self::GpuTemperature];

    pub fn id(self) -> u8 {
        self as u8
    }
}

/// Fan-behaviour bitmap bits (`ACER_GAMING_FAN_BEHAVIOR_CPU/GPU`).
pub const FAN_BIT_CPU: u16 = 0x0001;
pub const FAN_BIT_GPU: u16 = 0x0008;
pub const FAN_BITS_BOTH: u16 = FAN_BIT_CPU | FAN_BIT_GPU;

/// Fan ids for the fan-speed methods (`acer_wmi_gaming_fan_id`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanId {
    Cpu = 0x01,
    Gpu = 0x04,
}

/// Misc-setting indices (`acer_wmi_gaming_misc_setting`).
pub const MISC_SUPPORTED_PROFILES: u8 = 0x0A;
pub const MISC_PLATFORM_PROFILE: u8 = 0x0B;

/// Firmware rejected the call with this non-zero status byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareStatus(pub u8);

fn status(out: u64) -> Result<u64, FirmwareStatus> {
    match (out & STATUS_MASK) as u8 {
        0 => Ok(out),
        s => Err(FirmwareStatus(s)),
    }
}

/// Set of supported sensors (bit `id - 1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SensorSet(pub u16);

impl SensorSet {
    pub fn contains(self, sensor: Sensor) -> bool {
        self.0 & (1 << (sensor.id() - 1)) != 0
    }
}

pub fn supported_sensors_input() -> u32 {
    CMD_SUPPORTED_SENSORS
}

pub fn decode_supported_sensors(out: u64) -> Result<SensorSet, FirmwareStatus> {
    let v = status(out)?;
    Ok(SensorSet(((v >> 24) & 0xFFFF) as u16))
}

pub fn sensor_reading_input(sensor: Sensor) -> u32 {
    CMD_SENSOR_READING | (u32::from(sensor.id()) << 8)
}

/// Reading in bits 23:8 — °C for temperatures, RPM for fans.
pub fn decode_sensor_reading(out: u64) -> Result<u16, FirmwareStatus> {
    let v = status(out)?;
    Ok(((v >> 8) & 0xFFFF) as u16)
}

/// Per-fan behaviour values as reported by the firmware (1 auto, 2 turbo, 3 custom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FanBehaviors {
    pub cpu: Option<u8>,
    pub gpu: Option<u8>,
}

pub fn fan_behavior_get_input(bitmap: u16) -> u32 {
    u32::from(bitmap)
}

pub fn decode_fan_behavior(out: u64, bitmap: u16) -> Result<FanBehaviors, FirmwareStatus> {
    let v = status(out)?;
    let cpu = (bitmap & FAN_BIT_CPU != 0).then_some(((v >> 8) & 0x3) as u8);
    let gpu = (bitmap & FAN_BIT_GPU != 0).then_some(((v >> 14) & 0x3) as u8);
    Ok(FanBehaviors { cpu, gpu })
}

/// Input for `SetGamingFanBehavior`: bitmap in bits 15:0, CPU mode 17:16, GPU mode 23:22.
pub fn fan_behavior_set_input(bitmap: u16, mode: u8) -> u64 {
    let mode = u64::from(mode & 0x3);
    let mut input = u64::from(bitmap);
    if bitmap & FAN_BIT_CPU != 0 {
        input |= mode << 16;
    }
    if bitmap & FAN_BIT_GPU != 0 {
        input |= mode << 22;
    }
    input
}

pub fn fan_speed_get_input(fan: FanId) -> u32 {
    fan as u32
}

/// Stored speed percentage in bits 15:8.
pub fn decode_fan_speed(out: u64) -> Result<u8, FirmwareStatus> {
    let v = status(out)?;
    Ok(((v >> 8) & 0xFF) as u8)
}

/// Input for `SetGamingFanSpeed`; `None` when the percentage exceeds 100.
pub fn fan_speed_set_input(fan: FanId, pct: u8) -> Option<u64> {
    (pct <= 100).then(|| (fan as u64) | (u64::from(pct) << 8))
}

pub fn misc_get_input(index: u8) -> u32 {
    u32::from(index)
}

/// Setting value in bits 15:8.
pub fn decode_misc(out: u64) -> Result<u8, FirmwareStatus> {
    let v = status(out)?;
    Ok(((v >> 8) & 0xFF) as u8)
}

pub fn misc_set_input(index: u8, value: u8) -> u64 {
    u64::from(index) | (u64::from(value) << 8)
}

/// Status of a `Set*` call (`gmOutput` low byte).
pub fn decode_set_status(out: u32) -> Result<(), FirmwareStatus> {
    status(u64::from(out)).map(|_| ())
}

/// Supported platform profiles: bit N set means profile value N is accepted
/// (AN515-58 reports 0x33 = {Quiet 0, Balanced 1, Performance 4, Turbo 5}).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProfileSet(pub u8);

impl ProfileSet {
    pub fn contains(self, profile: u8) -> bool {
        profile < 8 && self.0 & (1 << profile) != 0
    }
}
