//! @module power_test
//! @description AC-line decoding and a live power-status read.
use nitrotray::platform::power::{decode_ac_line, on_ac_power};

#[test]
fn decodes_ac_line_status() {
    assert_eq!(decode_ac_line(0), Some(false));
    assert_eq!(decode_ac_line(1), Some(true));
    assert_eq!(decode_ac_line(255), None);
}

#[test]
fn live_read_is_a_known_state_on_a_laptop() {
    assert!(on_ac_power().is_some());
}
