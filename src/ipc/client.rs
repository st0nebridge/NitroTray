//! @module ipc::client
//! @description Typed helper client: one request per connection over a pluggable transport.
//!
//! @input  Typed calls (capabilities, sensors, fan/performance state, mode changes).
//! @output Decoded payloads, or `ClientError` distinguishing "helper absent" from "helper said no".
//! @dependencies ipc::protocol, ipc::pipe (Windows transport), acer, controls
use crate::acer::{FanState, PerformanceState, SensorReadings};
use crate::controls::{FanCurveProfile, FanMode, PerformanceMode};

use super::protocol::{self, CapabilitiesReport, ErrorCode, ErrorInfo, ProviderStatus, Request, Response};

/// Transport-level failure kinds the client cares about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    NotRunning,
    Untrusted,
    Other(String),
}

/// Moves one request line to the helper and returns its response line.
pub trait Transport: Send + Sync {
    fn roundtrip(&self, line: &str) -> Result<String, TransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    Transport(TransportError),
    Remote(ErrorInfo),
    Protocol(String),
}

impl ClientError {
    /// Provider status implied by this failure.
    pub fn status(&self) -> ProviderStatus {
        match self {
            Self::Transport(TransportError::NotRunning | TransportError::Untrusted) => {
                ProviderStatus::HelperUnavailable
            }
            Self::Remote(ErrorInfo { code: ErrorCode::AccessDenied, .. }) => ProviderStatus::AccessDenied,
            Self::Remote(ErrorInfo { code: ErrorCode::Unavailable, .. }) => ProviderStatus::NotPresent,
            _ => ProviderStatus::HelperUnavailable,
        }
    }

    /// Sentence suitable for the popup's inline notice.
    pub fn user_message(&self) -> String {
        match self {
            Self::Transport(TransportError::NotRunning) => "The NitroTray helper service is not running.".into(),
            Self::Transport(TransportError::Untrusted) => "Refused an untrusted helper pipe.".into(),
            Self::Transport(TransportError::Other(m)) => format!("Helper connection failed: {m}"),
            Self::Remote(e) => format!("{}.", e.message.trim_end_matches('.')),
            Self::Protocol(m) => format!("Unexpected helper reply: {m}"),
        }
    }
}

pub struct HelperClient<T: Transport> {
    transport: T,
}

impl<T: Transport> HelperClient<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn call(&self, req: &Request) -> Result<Response, ClientError> {
        let line = protocol::encode_line(req);
        let reply = self.transport.roundtrip(&line).map_err(ClientError::Transport)?;
        let resp = protocol::parse_response(&reply).map_err(|e| ClientError::Protocol(e.message))?;
        if resp.ok {
            Ok(resp)
        } else {
            Err(ClientError::Remote(
                resp.error.unwrap_or_else(|| ErrorInfo::new(ErrorCode::Internal, "helper failed without detail")),
            ))
        }
    }

    fn field<V>(resp: Response, pick: impl FnOnce(Response) -> Option<V>, what: &str) -> Result<V, ClientError> {
        pick(resp).ok_or_else(|| ClientError::Protocol(format!("missing {what}")))
    }

    pub fn capabilities(&self) -> Result<CapabilitiesReport, ClientError> {
        Self::field(self.call(&Request::GetCapabilities)?, |r| r.capabilities, "capabilities")
    }

    pub fn sensors(&self) -> Result<SensorReadings, ClientError> {
        Self::field(self.call(&Request::GetSensors)?, |r| r.sensors, "sensors")
    }

    pub fn fan_state(&self) -> Result<FanState, ClientError> {
        Self::field(self.call(&Request::GetFanState)?, |r| r.fan_state, "fan_state")
    }

    pub fn performance(&self) -> Result<PerformanceState, ClientError> {
        Self::field(self.call(&Request::GetPerformanceMode)?, |r| r.performance, "performance")
    }

    pub fn set_fan_mode(&self, mode: FanMode, curve: Option<FanCurveProfile>) -> Result<String, ClientError> {
        let req = Request::SetFanMode { mode, curve };
        Self::field(self.call(&req)?, |r| r.effective_mode, "effective_mode")
    }

    pub fn set_performance_mode(&self, mode: PerformanceMode) -> Result<String, ClientError> {
        Self::field(self.call(&Request::SetPerformanceMode { mode })?, |r| r.effective_mode, "effective_mode")
    }
}

/// Named-pipe transport (Windows).
#[cfg(windows)]
pub struct PipeTransport {
    pub name: String,
    pub policy: super::pipe::ServerPolicy,
}

#[cfg(windows)]
impl PipeTransport {
    /// Production defaults, overridable for development: `NITROTRAY_PIPE` renames the pipe and
    /// `NITROTRAY_DEV_HELPER=1` accepts a console helper outside session 0.
    pub fn from_env() -> Self {
        let name = std::env::var("NITROTRAY_PIPE").unwrap_or_else(|_| crate::meta::PIPE_NAME.to_string());
        let dev = std::env::var("NITROTRAY_DEV_HELPER").is_ok_and(|v| v == "1");
        let policy = if dev { super::pipe::ServerPolicy::AnySession } else { super::pipe::ServerPolicy::ServiceOnly };
        Self { name, policy }
    }
}

#[cfg(windows)]
impl Transport for PipeTransport {
    fn roundtrip(&self, line: &str) -> Result<String, TransportError> {
        use super::pipe::IpcError;
        super::pipe::request(&self.name, line, self.policy).map_err(|e| match e {
            IpcError::NotRunning => TransportError::NotRunning,
            IpcError::UntrustedServer => TransportError::Untrusted,
            other => TransportError::Other(other.to_string()),
        })
    }
}
