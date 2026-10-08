//! @module core_test
//! @description Helper service core: gating, read-back verification, curve lifecycle and crash recovery.
use nitrotray::acer::backend::{AcerError, PerformanceState};
use nitrotray::acer::Capabilities;
use nitrotray::controls::{FanCurveProfile, FanMode, PerformanceMode};
use nitrotray::ipc::protocol::{parse_response, ErrorCode, ProviderStatus, Request};
use nitrotray::service::core::{error_code, status_for, ServiceCore};
use nitrotray::service::curve::TickOutcome;

use crate::support::FakeBackend;

fn core(b: FakeBackend) -> ServiceCore<FakeBackend> {
    ServiceCore::new(b, "Nitro AN515-58", ProviderStatus::Connected, None)
}

fn code(r: &nitrotray::ipc::protocol::Response) -> ErrorCode {
    r.error.as_ref().unwrap().code
}

#[test]
fn capabilities_are_always_answerable() {
    let mut c = ServiceCore::new(FakeBackend::default(), "m", ProviderStatus::AccessDenied, None);
    let r = c.handle(Request::GetCapabilities);
    let report = r.capabilities.unwrap();
    assert_eq!(report.status, ProviderStatus::AccessDenied);
    assert_eq!(report.capabilities, Capabilities::none());
    assert_eq!(report.provider, "fake");
    assert_eq!(report.protocol_version, 1);
    assert_eq!(code(&c.handle(Request::GetSensors)), ErrorCode::AccessDenied);
    let mut absent = ServiceCore::new(FakeBackend::default(), "m", ProviderStatus::NotPresent, None);
    assert_eq!(code(&absent.handle(Request::GetFanState)), ErrorCode::Unavailable);
}

#[test]
fn reads_are_passed_through() {
    let mut c = core(FakeBackend::default());
    assert!(c.handle(Request::GetSensors).sensors.is_some());
    let fs = c.handle(Request::GetFanState).fan_state.unwrap();
    assert_eq!(fs.mode, Some(FanMode::Auto));
    assert_eq!((fs.cpu_duty_pct, fs.gpu_duty_pct, fs.curve_active), (None, None, false));
    assert_eq!(c.handle(Request::GetPerformanceMode).performance.unwrap().mode, Some(PerformanceMode::Default));
}

#[test]
fn read_errors_map_to_codes() {
    let mut b = FakeBackend::default();
    b.default_sensors = Err(AcerError::Transport("x".into()));
    b.fan_mode_error = Some(AcerError::Firmware(1));
    b.perf_error = Some(AcerError::AccessDenied);
    let mut c = core(b);
    assert_eq!(code(&c.handle(Request::GetSensors)), ErrorCode::Internal);
    assert_eq!(code(&c.handle(Request::GetFanState)), ErrorCode::FirmwareRejected);
    assert_eq!(code(&c.handle(Request::GetPerformanceMode)), ErrorCode::AccessDenied);
}

#[test]
fn max_mode_reports_full_duty() {
    let mut c = core(FakeBackend::default());
    let r = c.handle(Request::SetFanMode { mode: FanMode::Max, curve: None });
    assert_eq!(r.effective_mode.as_deref(), Some("max"));
    let fs = c.handle(Request::GetFanState).fan_state.unwrap();
    assert_eq!((fs.cpu_duty_pct, fs.gpu_duty_pct), (Some(100), Some(100)));
}

#[test]
fn unsupported_modes_are_refused_before_touching_hardware() {
    let mut b = FakeBackend::default();
    b.caps.max_fan_mode = false;
    b.caps.quiet_mode = false;
    let mut c = core(b);
    let r = c.handle(Request::SetFanMode { mode: FanMode::Max, curve: None });
    assert_eq!(code(&r), ErrorCode::Unsupported);
    let r = c.handle(Request::SetPerformanceMode { mode: PerformanceMode::Quiet });
    assert_eq!(code(&r), ErrorCode::Unsupported);
    assert!(r.error.unwrap().message.contains("Quiet"));
}

#[test]
fn firmware_that_ignores_a_write_is_reported() {
    let mut b = FakeBackend::default();
    b.ignore_fan_writes = true;
    b.ignore_perf_writes = true;
    b.performance = PerformanceState { mode: None, raw: Some(5) };
    let mut c = core(b);
    let r = c.handle(Request::SetFanMode { mode: FanMode::Max, curve: None });
    assert_eq!(code(&r), ErrorCode::FirmwareRejected);
    assert!(r.error.unwrap().message.contains("Auto"));
    let r = c.handle(Request::SetPerformanceMode { mode: PerformanceMode::Quiet });
    assert_eq!(r.error.unwrap().message, "firmware kept profile Turbo");
}

#[test]
fn write_then_read_errors_propagate() {
    let mut b = FakeBackend::default();
    b.perf_error = Some(AcerError::Firmware(3));
    let mut c = core(b);
    assert_eq!(
        code(&c.handle(Request::SetPerformanceMode { mode: PerformanceMode::Quiet })),
        ErrorCode::FirmwareRejected
    );
    let mut b = FakeBackend::default();
    b.fan_mode_error = Some(AcerError::Transport("t".into()));
    let mut c = core(b);
    assert_eq!(code(&c.handle(Request::SetFanMode { mode: FanMode::Auto, curve: None })), ErrorCode::Internal);
}

#[test]
fn custom_curve_lifecycle_with_marker() {
    let dir = std::env::temp_dir().join(format!("nitrotray-core-{}", std::process::id()));
    let marker = dir.join("marker");
    let _ = std::fs::remove_file(&marker);
    let mut c = ServiceCore::new(FakeBackend::default(), "m", ProviderStatus::Connected, Some(marker.clone()));
    let r = c.handle(Request::SetFanMode { mode: FanMode::Custom, curve: Some(FanCurveProfile::default_profile()) });
    assert_eq!(r.effective_mode.as_deref(), Some("custom"), "{r:?}");
    assert!(c.curve_active());
    assert!(marker.exists());
    let fs = c.handle(Request::GetFanState).fan_state.unwrap();
    assert!(fs.curve_active && fs.cpu_duty_pct.is_some());
    assert!(matches!(c.tick(), Some(TickOutcome::Unchanged)));
    c.handle(Request::SetFanMode { mode: FanMode::Auto, curve: None });
    assert!(!c.curve_active());
    assert!(!marker.exists());
    assert_eq!(c.tick(), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn custom_without_curve_or_with_failing_start_is_refused() {
    let mut c = core(FakeBackend::default());
    assert_eq!(code(&c.handle(Request::SetFanMode { mode: FanMode::Custom, curve: None })), ErrorCode::InvalidRequest);
    let mut b = FakeBackend::default();
    b.duty_error = Some(AcerError::Firmware(1));
    let mut c = core(b);
    let r = c.handle(Request::SetFanMode { mode: FanMode::Custom, curve: Some(FanCurveProfile::default_profile()) });
    assert!(r.error.unwrap().message.starts_with("custom curve could not start"));
    assert!(!c.curve_active());
}

#[test]
fn failing_curve_restores_auto_and_records_why() {
    let mut b = FakeBackend::default();
    for _ in 0..1 {
        b.sensors.push_back(Ok(crate::support::readings(60.0, 60.0)));
    }
    b.default_sensors = Err(AcerError::Firmware(1));
    let mut c = core(b);
    c.handle(Request::SetFanMode { mode: FanMode::Custom, curve: Some(FanCurveProfile::default_profile()) });
    assert!(c.curve_active());
    let mut last = None;
    for _ in 0..3 {
        last = c.tick();
    }
    assert!(matches!(last, Some(TickOutcome::FailSafe(_))));
    assert!(!c.curve_active());
    assert!(c.last_event().unwrap().starts_with("custom curve stopped"));
    let fs = c.handle(Request::GetFanState).fan_state.unwrap();
    assert_eq!(fs.mode, Some(FanMode::Auto));
}

#[test]
fn shutdown_restores_auto_only_when_a_curve_runs() {
    let mut c = core(FakeBackend::default());
    c.shutdown();
    c.handle(Request::SetFanMode { mode: FanMode::Custom, curve: Some(FanCurveProfile::default_profile()) });
    c.shutdown();
    assert!(!c.curve_active());
    assert_eq!(c.handle(Request::GetFanState).fan_state.unwrap().mode, Some(FanMode::Auto));
}

#[test]
fn stale_marker_triggers_auto_restore_on_start() {
    let dir = std::env::temp_dir().join(format!("nitrotray-crash-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let marker = dir.join("marker");
    std::fs::write(&marker, b"x").unwrap();
    let mut b = FakeBackend::default();
    b.fan_mode = Some(FanMode::Custom);
    let c = ServiceCore::new(b, "m", ProviderStatus::Connected, Some(marker.clone()));
    assert!(!marker.exists());
    assert!(c.last_event().unwrap().starts_with("restored Auto"));
    std::fs::write(&marker, b"x").unwrap();
    let mut no_auto = FakeBackend::default();
    no_auto.caps.auto_fan_mode = false;
    let c = ServiceCore::new(no_auto, "m", ProviderStatus::Connected, Some(marker.clone()));
    assert!(c.last_event().unwrap().contains("could not restore"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn handle_line_parses_and_encodes() {
    let mut c = core(FakeBackend::default());
    let r = parse_response(&c.handle_line(r#"{"op":"set_performance_mode","mode":"performance"}"#)).unwrap();
    assert_eq!(r.effective_mode.as_deref(), Some("performance"));
    let bad = parse_response(&c.handle_line("{\"op\":\"format_disk\"}")).unwrap();
    assert_eq!(code(&bad), ErrorCode::InvalidRequest);
    assert!(c.capabilities().custom_fan_mode);
}

#[test]
fn error_and_status_mapping() {
    assert_eq!(error_code(&AcerError::AccessDenied), ErrorCode::AccessDenied);
    assert_eq!(error_code(&AcerError::NotPresent), ErrorCode::Unavailable);
    assert_eq!(error_code(&AcerError::Firmware(1)), ErrorCode::FirmwareRejected);
    assert_eq!(error_code(&AcerError::Unsupported("x")), ErrorCode::Unsupported);
    assert_eq!(error_code(&AcerError::InvalidArgument("x".into())), ErrorCode::InvalidRequest);
    assert_eq!(error_code(&AcerError::Transport("x".into())), ErrorCode::Internal);
    assert_eq!(status_for(&AcerError::AccessDenied), ProviderStatus::AccessDenied);
    assert_eq!(status_for(&AcerError::NotPresent), ProviderStatus::NotPresent);
}
