//! @module fan_mode_test
//! @description FanMode ↔ firmware behaviour values and wire names.
use nitrotray::controls::fan_mode::{BEHAVIOR_AUTO, BEHAVIOR_CUSTOM, BEHAVIOR_TURBO};
use nitrotray::controls::FanMode;

#[test]
fn firmware_values_match_acer_wmi() {
    assert_eq!(FanMode::Auto.firmware_behavior(), 1);
    assert_eq!(FanMode::Max.firmware_behavior(), 2);
    assert_eq!(FanMode::Custom.firmware_behavior(), 3);
    assert_eq!((BEHAVIOR_AUTO, BEHAVIOR_TURBO, BEHAVIOR_CUSTOM), (1, 2, 3));
}

#[test]
fn round_trips_through_firmware_values() {
    for m in FanMode::ALL {
        assert_eq!(FanMode::from_firmware_behavior(m.firmware_behavior()), Some(m));
    }
    assert_eq!(FanMode::from_firmware_behavior(0), None);
    assert_eq!(FanMode::from_firmware_behavior(4), None);
}

#[test]
fn wire_names_and_labels() {
    for m in FanMode::ALL {
        assert_eq!(FanMode::parse(m.as_str()), Some(m));
        assert_eq!(serde_json::to_string(&m).unwrap(), format!("\"{}\"", m.as_str()));
    }
    assert_eq!(FanMode::parse("turbo"), None);
    assert_eq!(FanMode::Auto.label(), "Auto");
    assert_eq!(FanMode::Max.label(), "Max");
    assert_eq!(FanMode::Custom.label(), "Custom");
}
