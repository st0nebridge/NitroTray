//! @module service::core
//! @description Helper service core: dispatches typed requests to a hardware backend, owns the curve controller.
//!
//! @input  Validated `Request`s (or raw lines), periodic `tick()` calls, `shutdown()`.
//! @output `Response`s; firmware changes verified by read-back; fail-safe restoration of Auto.
//! @dependencies acer, controls, ipc::protocol, service::curve, meta
//!
//! Crash safety: while a custom curve is active a marker file exists. If the service starts
//! and finds the marker, the previous instance died mid-curve, so Auto is restored.
use std::path::PathBuf;

use crate::acer::backend::{AcerError, FanState, HardwareBackend};
use crate::acer::Capabilities;
use crate::controls::{FanCurveProfile, FanMode, PerformanceMode};
use crate::ipc::protocol::{self, CapabilitiesReport, ErrorCode, ProviderStatus, Request, Response};
use crate::service::curve::{CurveController, TickOutcome};

pub fn error_code(e: &AcerError) -> ErrorCode {
    match e {
        AcerError::AccessDenied => ErrorCode::AccessDenied,
        AcerError::NotPresent => ErrorCode::Unavailable,
        AcerError::Firmware(_) => ErrorCode::FirmwareRejected,
        AcerError::Unsupported(_) => ErrorCode::Unsupported,
        AcerError::InvalidArgument(_) => ErrorCode::InvalidRequest,
        AcerError::Transport(_) => ErrorCode::Internal,
    }
}

pub fn status_for(e: &AcerError) -> ProviderStatus {
    match e {
        AcerError::AccessDenied => ProviderStatus::AccessDenied,
        _ => ProviderStatus::NotPresent,
    }
}

fn fail(e: &AcerError) -> Response {
    Response::fail(error_code(e), e.to_string())
}

pub struct ServiceCore<B: HardwareBackend> {
    backend: B,
    caps: Capabilities,
    status: ProviderStatus,
    model: String,
    curve: Option<CurveController>,
    marker: Option<PathBuf>,
    last_event: Option<String>,
}

impl<B: HardwareBackend> ServiceCore<B> {
    pub fn new(mut backend: B, model: impl Into<String>, status: ProviderStatus, marker: Option<PathBuf>) -> Self {
        let caps = if matches!(status, ProviderStatus::Connected | ProviderStatus::Simulated) {
            backend.discover()
        } else {
            Capabilities::none()
        };
        let mut core = Self { backend, caps, status, model: model.into(), curve: None, marker, last_event: None };
        core.recover_after_crash();
        core
    }

    fn recover_after_crash(&mut self) {
        let Some(marker) = self.marker.clone() else {
            return;
        };
        if marker.exists() {
            let restored = self.caps.auto_fan_mode && self.backend.set_fan_mode(FanMode::Auto).is_ok();
            let _ = std::fs::remove_file(&marker);
            self.last_event = Some(if restored {
                "restored Auto fan mode after an unclean shutdown during a custom curve".into()
            } else {
                "found a stale custom-curve marker but could not restore Auto".into()
            });
        }
    }

    fn set_marker(&self, active: bool) {
        if let Some(m) = &self.marker {
            if active {
                if let Some(dir) = m.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(m, b"custom curve active\n");
            } else {
                let _ = std::fs::remove_file(m);
            }
        }
    }

    pub fn capabilities(&self) -> Capabilities {
        self.caps
    }

    pub fn curve_active(&self) -> bool {
        self.curve.is_some()
    }

    pub fn last_event(&self) -> Option<&str> {
        self.last_event.as_deref()
    }

    /// Why hardware requests cannot be served, if they cannot.
    fn unavailable(&self) -> Option<Response> {
        match self.status {
            ProviderStatus::Connected | ProviderStatus::Simulated => None,
            ProviderStatus::AccessDenied => {
                Some(Response::fail(ErrorCode::AccessDenied, "helper is not running elevated"))
            }
            _ => Some(Response::fail(ErrorCode::Unavailable, "Acer gaming firmware interface not available")),
        }
    }

    pub fn handle_line(&mut self, line: &str) -> String {
        let resp = match protocol::parse_request(line) {
            Ok(req) => self.handle(req),
            Err(e) => Response::from_error(e),
        };
        protocol::encode_line(&resp)
    }

    pub fn handle(&mut self, req: Request) -> Response {
        if let Request::GetCapabilities = req {
            return Response {
                capabilities: Some(CapabilitiesReport {
                    protocol_version: crate::meta::PROTOCOL_VERSION,
                    provider: self.backend.provider_name().into(),
                    model: self.model.clone(),
                    status: self.status,
                    capabilities: self.caps,
                }),
                ..Response::ok()
            };
        }
        if let Some(r) = self.unavailable() {
            return r;
        }
        match req {
            Request::GetCapabilities => unreachable!("handled above"),
            Request::GetSensors => match self.backend.sensors() {
                Ok(s) => Response { sensors: Some(s), ..Response::ok() },
                Err(e) => fail(&e),
            },
            Request::GetFanState => match self.fan_state() {
                Ok(s) => Response { fan_state: Some(s), ..Response::ok() },
                Err(e) => fail(&e),
            },
            Request::GetPerformanceMode => match self.backend.performance() {
                Ok(p) => Response { performance: Some(p), ..Response::ok() },
                Err(e) => fail(&e),
            },
            Request::SetFanMode { mode, curve } => self.set_fan_mode(mode, curve),
            Request::SetPerformanceMode { mode } => self.set_performance(mode),
        }
    }

    fn fan_state(&mut self) -> Result<FanState, AcerError> {
        let mode = self.backend.fan_mode()?;
        let (cpu, gpu) = match mode {
            Some(FanMode::Max) => (Some(100), Some(100)),
            Some(FanMode::Custom) => match self.curve.as_ref().and_then(CurveController::applied) {
                Some((c, g)) => (Some(c), Some(g)),
                None => (None, None),
            },
            _ => (None, None),
        };
        Ok(FanState { mode, cpu_duty_pct: cpu, gpu_duty_pct: gpu, curve_active: self.curve.is_some() })
    }

    fn stop_curve(&mut self) {
        if self.curve.take().is_some() {
            self.set_marker(false);
        }
    }

    fn set_fan_mode(&mut self, mode: FanMode, curve: Option<FanCurveProfile>) -> Response {
        if !self.caps.supports_fan(mode) {
            return Response::fail(
                ErrorCode::Unsupported,
                format!("{} fan mode is not supported on this model", mode.label()),
            );
        }
        if mode != FanMode::Custom {
            self.stop_curve();
        }
        if let Err(e) = self.backend.set_fan_mode(mode) {
            return fail(&e);
        }
        if mode == FanMode::Custom {
            let Some(profile) = curve else {
                return Response::fail(ErrorCode::InvalidRequest, "custom mode requires a curve");
            };
            self.set_marker(true);
            let mut controller = CurveController::new(profile);
            if let TickOutcome::FailSafe(reason) = controller.tick(&mut self.backend) {
                self.stop_curve();
                let _ = self.backend.set_fan_mode(FanMode::Auto);
                return Response::fail(ErrorCode::FirmwareRejected, format!("custom curve could not start: {reason}"));
            }
            self.curve = Some(controller);
        }
        match self.backend.fan_mode() {
            Ok(Some(actual)) if actual == mode => {
                Response { effective_mode: Some(actual.as_str().into()), ..Response::ok() }
            }
            Ok(other) => Response::fail(
                ErrorCode::FirmwareRejected,
                format!("firmware reports {} after the change", other.map_or("an unknown mode", FanMode::label)),
            ),
            Err(e) => fail(&e),
        }
    }

    fn set_performance(&mut self, mode: PerformanceMode) -> Response {
        if !self.caps.supports_performance(mode) {
            return Response::fail(
                ErrorCode::Unsupported,
                format!("{} mode is not supported on this model", mode.label()),
            );
        }
        if let Err(e) = self.backend.set_performance(mode) {
            return fail(&e);
        }
        match self.backend.performance() {
            Ok(p) if p.mode == Some(mode) => Response { effective_mode: Some(mode.as_str().into()), ..Response::ok() },
            Ok(p) => Response::fail(
                ErrorCode::FirmwareRejected,
                format!(
                    "firmware kept profile {}",
                    p.raw.map_or("unknown", crate::controls::performance_mode::firmware_profile_name)
                ),
            ),
            Err(e) => fail(&e),
        }
    }

    /// Advances the custom curve; restores Auto if the controller demands it.
    pub fn tick(&mut self) -> Option<TickOutcome> {
        let controller = self.curve.as_mut()?;
        let outcome = controller.tick(&mut self.backend);
        if let TickOutcome::FailSafe(reason) = &outcome {
            self.last_event = Some(format!("custom curve stopped: {reason}"));
            self.stop_curve();
            let _ = self.backend.set_fan_mode(FanMode::Auto);
        }
        Some(outcome)
    }

    /// Clean stop: never leave the fans under a curve nobody is driving.
    pub fn shutdown(&mut self) {
        if self.curve.is_some() {
            self.stop_curve();
            let _ = self.backend.set_fan_mode(FanMode::Auto);
        }
    }
}
