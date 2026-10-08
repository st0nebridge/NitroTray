//! @module simulated_test
//! @description The development backend behaves like firmware and labels itself "simulated".
use nitrotray::acer::backend::HardwareBackend;
use nitrotray::controls::{FanMode, PerformanceMode};
use nitrotray::service::simulated::{SimulatedBackend, SIM_CPU_FAN_MAX_RPM, SIM_GPU_FAN_MAX_RPM};

#[test]
fn identifies_itself_and_supports_everything() {
    let mut b = SimulatedBackend::default();
    assert_eq!(b.provider_name(), "simulated");
    let caps = b.discover();
    assert!(caps.custom_fan_mode && caps.performance_mode);
}

#[test]
fn sensors_are_plausible_and_mode_dependent() {
    let mut b = SimulatedBackend::default();
    let r = b.sensors().unwrap();
    let cpu = r.cpu_temp_c.unwrap();
    assert!((50.0..=95.0).contains(&cpu), "{cpu}");
    b.set_fan_mode(FanMode::Max).unwrap();
    let r = b.sensors().unwrap();
    assert_eq!(r.cpu_fan_rpm, Some(SIM_CPU_FAN_MAX_RPM as u32));
    assert_eq!(r.gpu_fan_rpm, Some(SIM_GPU_FAN_MAX_RPM as u32));
    b.set_fan_mode(FanMode::Custom).unwrap();
    b.set_fan_duty(50, 100).unwrap();
    let r = b.sensors().unwrap();
    assert_eq!(r.cpu_fan_rpm, Some((SIM_CPU_FAN_MAX_RPM / 2.0) as u32));
    assert!(b.set_fan_duty(101, 0).is_err());
    assert_eq!(b.fan_mode(), Ok(Some(FanMode::Custom)));
}

#[test]
fn performance_mode_shifts_temperatures() {
    let mut quiet = SimulatedBackend::default();
    quiet.set_performance(PerformanceMode::Quiet).unwrap();
    let mut perf = SimulatedBackend::default();
    perf.set_performance(PerformanceMode::Performance).unwrap();
    let q = quiet.sensors().unwrap().cpu_temp_c.unwrap();
    let p = perf.sensors().unwrap().cpu_temp_c.unwrap();
    assert!(p > q);
    assert_eq!(perf.performance().unwrap().raw, Some(4));
}

#[test]
fn auto_mode_rpm_tracks_temperature() {
    let mut b = SimulatedBackend::default();
    let r = b.sensors().unwrap();
    let rpm = r.cpu_fan_rpm.unwrap() as f32;
    assert!(rpm > 0.2 * SIM_CPU_FAN_MAX_RPM && rpm < SIM_CPU_FAN_MAX_RPM);
}
