//! @module backend_test
//! @description Backend error messages, boxed forwarding, and the unavailable backend.
use nitrotray::acer::backend::{AcerError, HardwareBackend, UnavailableBackend};
use nitrotray::acer::Capabilities;
use nitrotray::controls::{FanMode, PerformanceMode};

use crate::support::FakeBackend;

#[test]
fn error_messages_are_user_readable() {
    assert!(AcerError::AccessDenied.to_string().contains("denied"));
    assert!(AcerError::NotPresent.to_string().contains("not present"));
    assert_eq!(AcerError::Firmware(0x2A).to_string(), "the firmware rejected the request (status 0x2A)");
    assert_eq!(AcerError::Unsupported("Max fan mode").to_string(), "Max fan mode is not supported on this model");
    assert!(AcerError::InvalidArgument("x".into()).to_string().ends_with("x"));
    assert!(AcerError::Transport("y".into()).to_string().ends_with("y"));
}

#[test]
fn unavailable_backend_explains_every_call() {
    let mut b = UnavailableBackend(AcerError::AccessDenied);
    assert_eq!(b.discover(), Capabilities::none());
    assert_eq!(b.sensors(), Err(AcerError::AccessDenied));
    assert_eq!(b.fan_mode(), Err(AcerError::AccessDenied));
    assert_eq!(b.set_fan_mode(FanMode::Auto), Err(AcerError::AccessDenied));
    assert_eq!(b.set_fan_duty(50, 50), Err(AcerError::AccessDenied));
    assert_eq!(b.performance(), Err(AcerError::AccessDenied));
    assert_eq!(b.set_performance(PerformanceMode::Quiet), Err(AcerError::AccessDenied));
    assert_eq!(b.provider_name(), "unavailable");
}

#[test]
fn boxed_backend_forwards_every_method() {
    let mut b: Box<dyn HardwareBackend> = Box::new(FakeBackend::default());
    assert!(b.discover().custom_fan_mode);
    assert!(b.sensors().is_ok());
    assert_eq!(b.fan_mode(), Ok(Some(FanMode::Auto)));
    b.set_fan_mode(FanMode::Max).unwrap();
    assert_eq!(b.fan_mode(), Ok(Some(FanMode::Max)));
    b.set_fan_duty(40, 45).unwrap();
    b.set_performance(PerformanceMode::Quiet).unwrap();
    assert_eq!(b.performance().unwrap().mode, Some(PerformanceMode::Quiet));
    assert_eq!(b.provider_name(), "fake");
}
