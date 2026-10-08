//! @module protocol_test
//! @description Strict IPC schema: typed ops only, unknown fields/ops/oversize rejected, curve rules enforced.
use nitrotray::controls::{FanCurveProfile, FanMode, PerformanceMode};
use nitrotray::ipc::protocol::*;

#[test]
fn parses_the_documented_example_request() {
    assert_eq!(
        parse_request(r#"{"op":"set_fan_mode","mode":"auto"}"#),
        Ok(Request::SetFanMode { mode: FanMode::Auto, curve: None })
    );
    assert_eq!(parse_request(r#"{"op":"get_capabilities"}"#), Ok(Request::GetCapabilities));
    assert_eq!(parse_request(r#"{"op":"get_sensors"}"#), Ok(Request::GetSensors));
    assert_eq!(parse_request(r#"{"op":"get_fan_state"}"#), Ok(Request::GetFanState));
    assert_eq!(parse_request(r#" {"op":"get_performance_mode"} "#), Ok(Request::GetPerformanceMode));
    assert_eq!(
        parse_request(r#"{"op":"set_performance_mode","mode":"quiet"}"#),
        Ok(Request::SetPerformanceMode { mode: PerformanceMode::Quiet })
    );
}

#[test]
fn rejects_anything_outside_the_schema() {
    for bad in [
        r#"{"op":"write_ec","offset":45,"value":1}"#,
        r#"{"op":"set_fan_mode","mode":"turbo"}"#,
        r#"{"op":"set_fan_mode","mode":"auto","extra":true}"#,
        r#"{"op":"get_sensors","x":1}"#,
        r#"{"mode":"auto"}"#,
        r#"not json"#,
        r#"{"op":"set_performance_mode","mode":5}"#,
        r#"{"op":"get_capabilities","curve":null}"#,
        r#"["get_sensors"]"#,
    ] {
        let err = parse_request(bad).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidRequest, "{bad}");
    }
}

#[test]
fn names_the_unknown_field() {
    assert_eq!(parse_request(r#"{"op":"get_sensors","x":1}"#).unwrap_err().message, "unknown field 'x'");
}

#[test]
fn rejects_oversize_messages() {
    let huge = format!(r#"{{"op":"get_sensors","pad":"{}"}}"#, "x".repeat(MAX_MESSAGE_BYTES));
    assert_eq!(parse_request(&huge).unwrap_err().message, "message too large");
    assert!(parse_response(&"x".repeat(MAX_MESSAGE_BYTES + 1)).is_err());
    let body = r#"{"op":"get_sensors"}"#;
    let exact = format!("{body}{}", " ".repeat(MAX_MESSAGE_BYTES - body.len()));
    assert_eq!(exact.len(), MAX_MESSAGE_BYTES);
    assert_eq!(parse_request(&exact), Ok(Request::GetSensors), "exactly the limit is accepted");
}

#[test]
fn custom_mode_curve_rules() {
    let profile = FanCurveProfile::default_profile();
    let ok = Request::SetFanMode { mode: FanMode::Custom, curve: Some(profile.clone()) };
    assert_eq!(parse_request(&encode_line(&ok)), Ok(ok));
    let missing = r#"{"op":"set_fan_mode","mode":"custom"}"#;
    assert!(parse_request(missing).unwrap_err().message.contains("requires a curve"));
    let stray = Request::SetFanMode { mode: FanMode::Max, curve: Some(profile.clone()) };
    assert!(validate(&stray).unwrap_err().message.contains("only accepted"));
    let mut bad = profile;
    bad.cpu.points.truncate(1);
    let invalid = Request::SetFanMode { mode: FanMode::Custom, curve: Some(bad) };
    assert!(validate(&invalid).unwrap_err().message.starts_with("invalid curve"));
}

#[test]
fn response_shapes() {
    let ok = Response { effective_mode: Some("auto".into()), ..Response::ok() };
    assert_eq!(encode_line(&ok), "{\"ok\":true,\"effective_mode\":\"auto\"}\n");
    let fail = Response::fail(ErrorCode::FirmwareRejected, "no");
    let line = encode_line(&fail);
    assert!(line.contains("\"code\":\"firmware_rejected\""));
    assert_eq!(parse_response(&line).unwrap(), fail);
    let from = Response::from_error(ErrorInfo::new(ErrorCode::Unsupported, "x"));
    assert!(!from.ok && from.error.unwrap().code == ErrorCode::Unsupported);
    assert!(parse_response("garbage").is_err());
}

#[test]
fn provider_status_wire_names() {
    assert_eq!(serde_json::to_string(&ProviderStatus::HelperUnavailable).unwrap(), "\"helper_unavailable\"");
    assert_eq!(serde_json::to_string(&ProviderStatus::AccessDenied).unwrap(), "\"access_denied\"");
}

#[test]
fn response_size_limit_is_exact() {
    let too_big = format!("{{\"ok\":true,\"effective_mode\":\"{}\"}}", "x".repeat(MAX_MESSAGE_BYTES));
    assert_eq!(parse_response(&too_big).unwrap_err().message, "response too large");
    let pad = MAX_MESSAGE_BYTES - r#"{"ok":true,"effective_mode":""}"#.len();
    let exact = format!("{{\"ok\":true,\"effective_mode\":\"{}\"}}", "x".repeat(pad));
    assert_eq!(exact.len(), MAX_MESSAGE_BYTES);
    assert!(parse_response(&exact).unwrap().ok, "exactly at the limit is accepted");
}
