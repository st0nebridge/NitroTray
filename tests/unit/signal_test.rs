//! @module signal_test
//! @description Cross-process show/exit signals via named events.
use nitrotray::platform::signal::{notify_existing, ShowSignal, EXIT_EVENT, SHOW_EVENT};

#[test]
fn signal_round_trip() {
    let name = format!("Local\\NitroTrayTest.Signal.{}", std::process::id());
    assert!(!notify_existing(&name), "nobody listening yet");
    let s = ShowSignal::create(&name).unwrap();
    assert!(!s.wait(0));
    assert!(notify_existing(&name));
    assert!(s.wait(1000));
    assert!(!s.wait(0), "auto-reset");
}

#[test]
fn event_names_are_session_local_and_distinct() {
    assert!(SHOW_EVENT.starts_with("Local\\") && EXIT_EVENT.starts_with("Local\\"));
    assert_ne!(SHOW_EVENT, EXIT_EVENT);
}
