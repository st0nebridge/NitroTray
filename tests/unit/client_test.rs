//! @module client_test
//! @description Typed helper client over an in-memory transport wired to a real ServiceCore.
use std::sync::Mutex;

use nitrotray::controls::{FanCurveProfile, FanMode, PerformanceMode};
use nitrotray::ipc::client::{ClientError, HelperClient, Transport, TransportError};
use nitrotray::ipc::protocol::{ErrorCode, ErrorInfo, ProviderStatus};
use nitrotray::service::core::ServiceCore;

use crate::support::FakeBackend;

struct CoreTransport(Mutex<ServiceCore<FakeBackend>>);

impl Transport for CoreTransport {
    fn roundtrip(&self, line: &str) -> Result<String, TransportError> {
        Ok(self.0.lock().unwrap().handle_line(line))
    }
}

struct Fixed(Result<String, TransportError>);

impl Transport for Fixed {
    fn roundtrip(&self, _line: &str) -> Result<String, TransportError> {
        self.0.clone()
    }
}

fn client() -> HelperClient<CoreTransport> {
    let core = ServiceCore::new(FakeBackend::default(), "Nitro AN515-58", ProviderStatus::Connected, None);
    HelperClient::new(CoreTransport(Mutex::new(core)))
}

#[test]
fn typed_calls_round_trip_through_the_core() {
    let c = client();
    let caps = c.capabilities().unwrap();
    assert_eq!(caps.status, ProviderStatus::Connected);
    assert_eq!(caps.model, "Nitro AN515-58");
    assert!(c.sensors().unwrap().cpu_temp_c.is_some());
    assert_eq!(c.fan_state().unwrap().mode, Some(FanMode::Auto));
    assert_eq!(c.performance().unwrap().mode, Some(PerformanceMode::Default));
    assert_eq!(c.set_fan_mode(FanMode::Max, None).unwrap(), "max");
    assert_eq!(c.set_performance_mode(PerformanceMode::Quiet).unwrap(), "quiet");
    assert_eq!(c.set_fan_mode(FanMode::Custom, Some(FanCurveProfile::default_profile())).unwrap(), "custom");
}

#[test]
fn remote_errors_are_distinguished_from_transport_errors() {
    let c = client();
    let err = c.set_fan_mode(FanMode::Custom, None).unwrap_err();
    assert!(matches!(err, ClientError::Remote(ErrorInfo { code: ErrorCode::InvalidRequest, .. })));
    let absent = HelperClient::new(Fixed(Err(TransportError::NotRunning)));
    let e = absent.sensors().unwrap_err();
    assert_eq!(e.status(), ProviderStatus::HelperUnavailable);
    assert_eq!(e.user_message(), "The NitroTray helper service is not running.");
}

#[test]
fn error_status_and_messages() {
    let denied = ClientError::Remote(ErrorInfo::new(ErrorCode::AccessDenied, "helper is not running elevated"));
    assert_eq!(denied.status(), ProviderStatus::AccessDenied);
    assert_eq!(denied.user_message(), "helper is not running elevated.");
    let absent = ClientError::Remote(ErrorInfo::new(ErrorCode::Unavailable, "gone."));
    assert_eq!(absent.status(), ProviderStatus::NotPresent);
    assert_eq!(absent.user_message(), "gone.");
    let firmware = ClientError::Remote(ErrorInfo::new(ErrorCode::FirmwareRejected, "x"));
    assert_eq!(firmware.status(), ProviderStatus::HelperUnavailable);
    assert_eq!(ClientError::Transport(TransportError::Untrusted).status(), ProviderStatus::HelperUnavailable);
    assert!(ClientError::Transport(TransportError::Untrusted).user_message().contains("untrusted"));
    assert!(ClientError::Transport(TransportError::Other("e".into())).user_message().ends_with("e"));
    assert_eq!(ClientError::Protocol("p".into()).status(), ProviderStatus::HelperUnavailable);
    assert!(ClientError::Protocol("p".into()).user_message().ends_with("p"));
}

#[test]
fn malformed_or_incomplete_replies_are_protocol_errors() {
    let junk = HelperClient::new(Fixed(Ok("junk".into())));
    assert!(matches!(junk.sensors(), Err(ClientError::Protocol(_))));
    let empty_ok = HelperClient::new(Fixed(Ok("{\"ok\":true}".into())));
    assert_eq!(empty_ok.sensors(), Err(ClientError::Protocol("missing sensors".into())));
    assert_eq!(empty_ok.capabilities().unwrap_err(), ClientError::Protocol("missing capabilities".into()));
    assert_eq!(empty_ok.fan_state().unwrap_err(), ClientError::Protocol("missing fan_state".into()));
    assert_eq!(empty_ok.performance().unwrap_err(), ClientError::Protocol("missing performance".into()));
    assert_eq!(
        empty_ok.set_fan_mode(FanMode::Auto, None).unwrap_err(),
        ClientError::Protocol("missing effective_mode".into())
    );
    let bare_fail = HelperClient::new(Fixed(Ok("{\"ok\":false}".into())));
    assert!(matches!(bare_fail.sensors(), Err(ClientError::Remote(ErrorInfo { code: ErrorCode::Internal, .. }))));
}

#[test]
fn the_unlock_boot_report_comes_back_typed() {
    use nitrotray::service::unlock_log::StaticUnlockLog;
    use nitrotray::unlock::boot_log::{BootReport, BootStatus};
    let report =
        BootReport { run: "RUN0007".into(), status: BootStatus::DryRun, build: "dryrun".into(), written_unix: None };
    let core = ServiceCore::new(FakeBackend::default(), "m", ProviderStatus::Connected, None)
        .with_unlock_log(Box::new(StaticUnlockLog(Some(report.clone()))));
    let c = HelperClient::new(CoreTransport(Mutex::new(core)));
    assert_eq!(c.unlock_boot().unwrap(), Some(report));
    let none = ServiceCore::new(FakeBackend::default(), "m", ProviderStatus::Connected, None)
        .with_unlock_log(Box::new(StaticUnlockLog(None)));
    assert_eq!(HelperClient::new(CoreTransport(Mutex::new(none))).unlock_boot().unwrap(), None);
}

#[test]
fn an_older_helper_that_does_not_know_the_request_is_a_remote_error() {
    let old = HelperClient::new(Fixed(Ok(
        r#"{"ok":false,"error":{"code":"invalid_request","message":"malformed request: unknown variant"}}"#.to_string()
            + "
",
    )));
    match old.unlock_boot().unwrap_err() {
        ClientError::Remote(e) => assert_eq!(e.code, ErrorCode::InvalidRequest),
        other => panic!("{other:?}"),
    }
}
