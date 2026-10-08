//! @module discovery_test
//! @description Capability derivation is fail-closed and model-gated.
use nitrotray::acer::acpi::{FanBehaviors, SensorSet};
use nitrotray::acer::backend::AcerError;
use nitrotray::acer::discovery::{blocking_error, derive, model_allows_custom_fan, Capabilities, ProbeResults};
use nitrotray::controls::{FanMode, PerformanceMode};

fn probe() -> ProbeResults {
    ProbeResults {
        model: "Nitro AN515-58".into(),
        sensors: Ok(SensorSet(0x227)),
        fan_behavior: Ok(FanBehaviors { cpu: Some(1), gpu: Some(1) }),
        fan_speed: Ok(50),
        profiles: Ok(0x33),
        current_profile: Ok(4),
    }
}

#[test]
fn full_probe_enables_everything() {
    let c = derive(&probe());
    for m in FanMode::ALL {
        assert!(c.supports_fan(m));
    }
    for m in PerformanceMode::ALL {
        assert!(c.supports_performance(m));
    }
}

#[test]
fn sensor_bits_map_individually() {
    let mut p = probe();
    p.sensors = Ok(SensorSet(0b10_0000_0000));
    let c = derive(&p);
    assert!(c.gpu_temperature);
    assert!(!c.cpu_temperature && !c.cpu_fan && !c.gpu_fan);
}

#[test]
fn unreadable_fan_behaviour_disables_all_fan_modes() {
    let mut p = probe();
    p.fan_behavior = Err(AcerError::Firmware(1));
    let c = derive(&p);
    assert!(!c.auto_fan_mode && !c.max_fan_mode && !c.custom_fan_mode);
    let mut p = probe();
    p.fan_behavior = Ok(FanBehaviors { cpu: Some(0), gpu: Some(1) });
    assert!(!derive(&p).auto_fan_mode, "out-of-range behaviour value");
    let mut p = probe();
    p.fan_behavior = Ok(FanBehaviors { cpu: Some(1), gpu: None });
    assert!(!derive(&p).max_fan_mode);
}

#[test]
fn custom_mode_needs_fan_speed_and_known_model() {
    let mut p = probe();
    p.fan_speed = Err(AcerError::Firmware(1));
    assert!(!derive(&p).custom_fan_mode);
    let mut p = probe();
    p.model = "Nitro AN517-99".into();
    let c = derive(&p);
    assert!(c.max_fan_mode && !c.custom_fan_mode);
    assert!(model_allows_custom_fan("  nitro an515-58 "));
    assert!(!model_allows_custom_fan(""));
}

#[test]
fn profiles_come_from_the_supported_bitmap() {
    let mut p = probe();
    p.profiles = Ok(0b0000_0011);
    let c = derive(&p);
    assert!(c.quiet_mode && c.default_mode && !c.performance_mode);
    let none = |c: Capabilities| PerformanceMode::ALL.iter().all(|m| !c.supports_performance(*m));
    let mut p = probe();
    p.current_profile = Err(AcerError::Firmware(1));
    assert!(none(derive(&p)), "profile must also be readable");
    let mut p = probe();
    p.profiles = Err(AcerError::AccessDenied);
    assert!(none(derive(&p)));
}

#[test]
fn all_errors_mean_no_capabilities_and_a_blocking_reason() {
    let p = ProbeResults {
        model: String::new(),
        sensors: Err(AcerError::AccessDenied),
        fan_behavior: Err(AcerError::AccessDenied),
        fan_speed: Err(AcerError::AccessDenied),
        profiles: Err(AcerError::AccessDenied),
        current_profile: Err(AcerError::AccessDenied),
    };
    assert_eq!(derive(&p), Capabilities::none());
    assert_eq!(blocking_error(&p), Some(AcerError::AccessDenied));
    assert_eq!(blocking_error(&probe()), None);
    let mut partial = probe();
    partial.current_profile = Err(AcerError::NotPresent);
    assert_eq!(blocking_error(&partial), Some(AcerError::NotPresent));
    let mut fans = probe();
    fans.fan_behavior = Err(AcerError::Firmware(9));
    assert_eq!(blocking_error(&fans), Some(AcerError::Firmware(9)));
}

#[test]
fn capabilities_round_trip_json() {
    let c = derive(&probe());
    let back: Capabilities = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(back, c);
    let none = Capabilities::none();
    assert!(FanMode::ALL.iter().all(|m| !none.supports_fan(*m)));
}
