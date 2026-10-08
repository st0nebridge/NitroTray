//! @module cu_20260930_002
//! @description Regression test: IPC requests with unknown fields are rejected even for field-less ops
//! (serde ignores extra keys on unit variants of internally tagged enums).
use nitrotray::ipc::protocol::{parse_request, ErrorCode};

#[test]
fn unit_ops_reject_extra_fields() {
    for op in ["get_capabilities", "get_sensors", "get_fan_state", "get_performance_mode"] {
        let line = format!(r#"{{"op":"{op}","offset":45}}"#);
        let err = parse_request(&line).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidRequest, "{op}");
        assert_eq!(err.message, "unknown field 'offset'");
    }
}
