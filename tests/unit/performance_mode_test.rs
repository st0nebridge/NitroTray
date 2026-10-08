//! @module performance_mode_test
//! @description PerformanceMode ↔ firmware platform profiles (verified mapping, D-20260930-005).
use nitrotray::controls::performance_mode::{firmware_profile_name, PROFILE_ECO, PROFILE_TURBO};
use nitrotray::controls::PerformanceMode;

#[test]
fn maps_to_verified_firmware_profiles() {
    assert_eq!(PerformanceMode::Quiet.firmware_profile(), 0x00);
    assert_eq!(PerformanceMode::Default.firmware_profile(), 0x01);
    assert_eq!(PerformanceMode::Performance.firmware_profile(), 0x04);
}

#[test]
fn unexposed_profiles_are_not_invented() {
    assert_eq!(PerformanceMode::from_firmware_profile(PROFILE_TURBO), None);
    assert_eq!(PerformanceMode::from_firmware_profile(PROFILE_ECO), None);
    assert_eq!(PerformanceMode::from_firmware_profile(0x02), None);
    for m in PerformanceMode::ALL {
        assert_eq!(PerformanceMode::from_firmware_profile(m.firmware_profile()), Some(m));
    }
}

#[test]
fn names_labels_and_wire_format() {
    for m in PerformanceMode::ALL {
        assert_eq!(PerformanceMode::parse(m.as_str()), Some(m));
        assert_eq!(serde_json::to_string(&m).unwrap(), format!("\"{}\"", m.as_str()));
    }
    assert_eq!(PerformanceMode::parse("turbo"), None);
    assert_eq!(PerformanceMode::Performance.label(), "Performance");
    assert_eq!(PerformanceMode::Quiet.label(), "Quiet");
    assert_eq!(PerformanceMode::Default.label(), "Default");
    assert_eq!(firmware_profile_name(0x05), "Turbo");
    assert_eq!(firmware_profile_name(0x06), "Eco");
    assert_eq!(firmware_profile_name(0x04), "Performance");
    assert_eq!(firmware_profile_name(0x00), "Quiet");
    assert_eq!(firmware_profile_name(0x01), "Default");
    assert_eq!(firmware_profile_name(0x09), "Unknown");
}
