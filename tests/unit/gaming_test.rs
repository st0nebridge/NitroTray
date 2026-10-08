//! @module gaming_test
//! @description AcerGaming backend over a scripted executor replaying the AN515-58 probe.
use nitrotray::acer::backend::{AcerError, HardwareBackend};
use nitrotray::acer::gaming::AcerGaming;
use nitrotray::controls::{FanMode, PerformanceMode};

use crate::support::ScriptedExecutor;

fn an515() -> AcerGaming<ScriptedExecutor> {
    AcerGaming::new(ScriptedExecutor::an515_58(), "Nitro AN515-58")
}

#[test]
fn discovers_full_capabilities_on_an515_58() {
    let caps = an515().discover();
    assert!(caps.cpu_temperature && caps.gpu_temperature && caps.cpu_fan && caps.gpu_fan);
    assert!(caps.auto_fan_mode && caps.max_fan_mode && caps.custom_fan_mode);
    assert!(caps.quiet_mode && caps.default_mode && caps.performance_mode);
}

#[test]
fn reads_probe_sensor_values() {
    let r = an515().sensors().unwrap();
    assert_eq!(r.cpu_temp_c, Some(65.0));
    assert_eq!(r.gpu_temp_c, Some(49.0));
    assert_eq!(r.cpu_fan_rpm, Some(5769));
    assert_eq!(r.gpu_fan_rpm, Some(6122));
}

#[test]
fn reads_modes() {
    let mut g = an515();
    assert_eq!(g.fan_mode(), Ok(Some(FanMode::Auto)));
    let p = g.performance().unwrap();
    assert_eq!(p.mode, Some(PerformanceMode::Performance));
    assert_eq!(p.raw, Some(4));
    assert_eq!(g.provider_name(), "acer-wmi");
}

#[test]
fn writes_encoded_fan_behaviour_and_reads_it_back() {
    let mut g = an515();
    g.set_fan_mode(FanMode::Max).unwrap();
    assert_eq!(g.fan_mode(), Ok(Some(FanMode::Max)));
    g.set_fan_mode(FanMode::Auto).unwrap();
    assert_eq!(g.fan_mode(), Ok(Some(FanMode::Auto)));
}

#[test]
fn writes_performance_profile() {
    let mut g = an515();
    g.set_performance(PerformanceMode::Quiet).unwrap();
    assert_eq!(g.performance().unwrap().mode, Some(PerformanceMode::Quiet));
}

#[test]
fn writes_fan_duty_for_both_fans() {
    let mut g = an515();
    g.set_fan_duty(60, 70).unwrap();
    assert!(g.set_fan_duty(101, 50).is_err());
}

#[test]
fn firmware_rejection_surfaces_as_error() {
    let mut exec = ScriptedExecutor::an515_58();
    exec.sets.insert("SetGamingMiscSetting".into(), Ok(0x01));
    let mut g = AcerGaming::new(exec, "Nitro AN515-58");
    assert_eq!(g.set_performance(PerformanceMode::Quiet), Err(AcerError::Firmware(1)));
}

#[test]
fn unknown_model_gets_no_custom_mode() {
    let mut g = AcerGaming::new(ScriptedExecutor::an515_58(), "Predator PH999");
    let caps = g.discover();
    assert!(caps.max_fan_mode);
    assert!(!caps.custom_fan_mode);
    assert_eq!(g.set_fan_mode(FanMode::Custom), Err(AcerError::Unsupported("Custom fan mode")));
    assert_eq!(g.set_fan_duty(50, 50), Err(AcerError::Unsupported("Custom fan duty")));
}

#[test]
fn access_denied_everywhere_yields_no_capabilities_and_errors() {
    let mut exec = ScriptedExecutor::default();
    for (k, v) in ScriptedExecutor::an515_58().gets {
        let _ = v;
        exec.gets.insert(k, Err(AcerError::AccessDenied));
    }
    let mut g = AcerGaming::new(exec, "Nitro AN515-58");
    assert_eq!(g.discover(), nitrotray::acer::Capabilities::none());
    assert_eq!(g.fan_mode(), Err(AcerError::AccessDenied));
    assert!(g.set_fan_mode(FanMode::Max).is_err());
    assert!(g.set_performance(PerformanceMode::Quiet).is_err());
}

#[test]
fn sensor_failure_after_discovery_is_reported() {
    let mut exec = ScriptedExecutor::an515_58();
    let mut g_ok = AcerGaming::new(ScriptedExecutor::an515_58(), "Nitro AN515-58");
    let _ = g_ok.discover();
    for i in [0x0101u32, 0x0201, 0x0601, 0x0A01] {
        exec.gets.insert(("GetGamingSysInfo".into(), i), Err(AcerError::Transport("boom".into())));
    }
    let mut g = AcerGaming::new(exec, "Nitro AN515-58");
    assert_eq!(g.sensors(), Err(AcerError::Transport("boom".into())));
}

#[test]
fn partial_sensor_failure_keeps_the_good_readings() {
    let mut exec = ScriptedExecutor::an515_58();
    exec.gets.insert(("GetGamingSysInfo".into(), 0x0601), Err(AcerError::Firmware(1)));
    let mut g = AcerGaming::new(exec, "Nitro AN515-58");
    let r = g.sensors().unwrap();
    assert_eq!(r.cpu_temp_c, Some(65.0));
    assert_eq!(r.gpu_fan_rpm, None);
}

#[test]
fn mismatched_fan_modes_read_as_unknown() {
    let mut exec = ScriptedExecutor::an515_58();
    exec.gets.insert(("GetGamingFanBehavior".into(), 0x09), Ok((1 << 8) | (2 << 14)));
    let mut g = AcerGaming::new(exec, "Nitro AN515-58");
    assert_eq!(g.fan_mode(), Ok(None));
}

#[test]
fn read_sensor_reports_firmware_status() {
    let mut exec = ScriptedExecutor::an515_58();
    exec.gets.insert(("GetGamingSysInfo".into(), 0x0301), Ok(0x3001));
    let mut g = AcerGaming::new(exec, "Nitro AN515-58");
    assert_eq!(g.read_sensor(nitrotray::acer::acpi::Sensor::ExternalTemperature2), Err(AcerError::Firmware(1)));
}
