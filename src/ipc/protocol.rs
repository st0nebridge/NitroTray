//! @module ipc::protocol
//! @description Strictly typed helper IPC schema: requests, responses, validation, framing.
//!
//! @input  One newline-terminated JSON request per connection (≤ `MAX_MESSAGE_BYTES`).
//! @output Parsed + semantically validated `Request`, or an `ErrorInfo`; serialised `Response` lines.
//! @dependencies serde, serde_json, acer (Capabilities, state types), controls
//!
//! Unknown ops, unknown fields, oversize messages, and invalid curves are all rejected before
//! any hardware is touched. There is deliberately no generic read/write operation.
use serde::{Deserialize, Serialize};

use crate::acer::{Capabilities, FanState, PerformanceState, SensorReadings};
use crate::controls::{FanCurveProfile, FanMode, PerformanceMode};

pub const MAX_MESSAGE_BYTES: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    GetCapabilities,
    GetSensors,
    GetFanState,
    GetPerformanceMode,
    SetFanMode {
        mode: FanMode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        curve: Option<FanCurveProfile>,
    },
    SetPerformanceMode {
        mode: PerformanceMode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    Unsupported,
    FirmwareRejected,
    AccessDenied,
    Unavailable,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: ErrorCode,
    pub message: String,
}

impl ErrorInfo {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

/// How the hardware provider is reachable, as seen from the tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Connected,
    Simulated,
    AccessDenied,
    NotPresent,
    HelperUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitiesReport {
    pub protocol_version: u32,
    pub provider: String,
    pub model: String,
    pub status: ProviderStatus,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<CapabilitiesReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensors: Option<SensorReadings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fan_state: Option<FanState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance: Option<PerformanceState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_mode: Option<String>,
}

impl Response {
    pub fn ok() -> Self {
        Self { ok: true, ..Self::default() }
    }

    pub fn fail(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { ok: false, error: Some(ErrorInfo::new(code, message)), ..Self::default() }
    }

    pub fn from_error(e: ErrorInfo) -> Self {
        Self { error: Some(e), ..Self::default() }
    }
}

/// Parses and validates one request line.
pub fn parse_request(line: &str) -> Result<Request, ErrorInfo> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(ErrorInfo::new(ErrorCode::InvalidRequest, "message too large"));
    }
    let malformed = |e: serde_json::Error| ErrorInfo::new(ErrorCode::InvalidRequest, format!("malformed request: {e}"));
    let raw: serde_json::Value = serde_json::from_str(line.trim()).map_err(malformed)?;
    let req: Request = serde_json::from_value(raw.clone()).map_err(malformed)?;
    reject_unknown_fields(&raw, &req)?;
    validate(&req)?;
    Ok(req)
}

/// serde ignores extra keys on unit variants of an internally tagged enum, so every key of the
/// input must also appear in the request's canonical serialisation.
fn reject_unknown_fields(raw: &serde_json::Value, req: &Request) -> Result<(), ErrorInfo> {
    let canonical = serde_json::to_value(req).unwrap_or_default();
    let (Some(input), Some(known)) = (raw.as_object(), canonical.as_object()) else {
        return Err(ErrorInfo::new(ErrorCode::InvalidRequest, "request must be a JSON object"));
    };
    match input.keys().find(|k| !known.contains_key(*k)) {
        Some(k) => Err(ErrorInfo::new(ErrorCode::InvalidRequest, format!("unknown field '{k}'"))),
        None => Ok(()),
    }
}

/// Semantic checks beyond the schema.
pub fn validate(req: &Request) -> Result<(), ErrorInfo> {
    if let Request::SetFanMode { mode, curve } = req {
        match (mode, curve) {
            (FanMode::Custom, None) => {
                return Err(ErrorInfo::new(ErrorCode::InvalidRequest, "custom mode requires a curve"));
            }
            (FanMode::Custom, Some(c)) => {
                c.validate().map_err(|e| ErrorInfo::new(ErrorCode::InvalidRequest, format!("invalid curve: {e}")))?;
            }
            (_, Some(_)) => {
                return Err(ErrorInfo::new(ErrorCode::InvalidRequest, "a curve is only accepted with custom mode"));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Serialises a value as one protocol line (JSON + `\n`).
pub fn encode_line<T: Serialize>(value: &T) -> String {
    let mut s = serde_json::to_string(value)
        .unwrap_or_else(|_| r#"{"ok":false,"error":{"code":"internal","message":"serialisation failed"}}"#.to_string());
    s.push('\n');
    s
}

pub fn parse_response(line: &str) -> Result<Response, ErrorInfo> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(ErrorInfo::new(ErrorCode::Internal, "response too large"));
    }
    serde_json::from_str(line.trim())
        .map_err(|e| ErrorInfo::new(ErrorCode::Internal, format!("malformed response: {e}")))
}
