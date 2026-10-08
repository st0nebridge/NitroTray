//! @module ui::bridge
//! @description Contract between Rust and the web UI: the state pushed to the page, and commands it sends.
//!
//! @input  JSON command strings posted by the page via `window.ipc.postMessage`.
//! @output Strictly parsed `UiCommand`s; `UiState` values; JS call snippets for `evaluate_script`.
//! @dependencies serde, serde_json, acer, controls, config, ipc::protocol, telemetry::thresholds
use serde::{Deserialize, Serialize};

use crate::acer::Capabilities;
use crate::config::settings::{AccentIntensity, Settings, TemperatureUnit};
use crate::controls::{FanCurveProfile, FanMode, PerformanceMode};
use crate::ipc::protocol::ProviderStatus;
use crate::telemetry::thresholds::TempLevel;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UiProcessor {
    /// Temperature in the user's unit.
    pub temp: Option<f32>,
    pub level: Option<TempLevel>,
    pub util: Option<f32>,
    pub mhz: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FanRowState {
    Ok,
    Unavailable,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UiFan {
    pub state: FanRowState,
    pub rpm: Option<u32>,
    /// Ring fill 0–100, only when duty is known or calibrated; `None` draws no arc.
    pub ring_pct: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PendingAction {
    Fan(FanMode),
    Performance(PerformanceMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    Error,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UiPrefs {
    /// Resolved theme: "dark" or "light".
    pub theme: &'static str,
    pub accent: AccentIntensity,
    pub reduced_motion: bool,
    pub transparency: bool,
    pub unit_symbol: &'static str,
    pub unit: TemperatureUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UiState {
    pub cpu: UiProcessor,
    pub gpu: UiProcessor,
    pub cpu_fan: UiFan,
    pub gpu_fan: UiFan,
    pub fan_mode: Option<FanMode>,
    pub performance_mode: Option<PerformanceMode>,
    /// Name of a firmware profile the popup does not expose (e.g. "Turbo").
    pub performance_other: Option<String>,
    pub capabilities: Capabilities,
    pub connection: ProviderStatus,
    pub connection_text: String,
    pub provider: String,
    pub model: String,
    pub has_custom_profile: bool,
    pub pending: Option<PendingAction>,
    pub notice: Option<Notice>,
    pub prefs: UiPrefs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Page {
    Monitoring,
    FanCurve,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiCommand {
    Ready,
    ClosePopup,
    DragWindow,
    OpenMonitoring,
    OpenSettings,
    OpenFanCurve,
    OpenNitroSense,
    DismissNotice,
    SetFanMode { mode: FanMode },
    SetPerformanceMode { mode: PerformanceMode },
    SaveSettings { settings: Box<Settings> },
    SaveFanCurve { profile: FanCurveProfile, apply: bool },
    SetAutostart { enabled: bool },
    ExportDiagnostics,
    InstallHelper,
    PinTrayIcons,
    Navigate { page: Page },
}

pub const MAX_COMMAND_BYTES: usize = 64 * 1024;

/// Parses one page message; unknown commands or fields are rejected.
pub fn parse_command(json: &str) -> Result<UiCommand, String> {
    if json.len() > MAX_COMMAND_BYTES {
        return Err("command too large".into());
    }
    let raw: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("malformed command: {e}"))?;
    let cmd: UiCommand = serde_json::from_value(raw.clone()).map_err(|e| format!("invalid command: {e}"))?;
    let canonical = serde_json::to_value(&cmd).unwrap_or_default();
    match (raw.as_object(), canonical.as_object()) {
        (Some(input), Some(known)) => match input.keys().find(|k| !known.contains_key(*k)) {
            Some(k) => Err(format!("unknown field '{k}'")),
            None => Ok(cmd),
        },
        _ => Err("command must be a JSON object".into()),
    }
}

/// `window.nitro.<function>(<json>)`, guarded so it is a no-op before the page script loads.
pub fn script_call<T: Serialize>(function: &str, payload: &T) -> String {
    let json = serde_json::to_string(payload).unwrap_or_else(|_| "null".into());
    format!("window.nitro&&window.nitro.{function}&&window.nitro.{function}({json});")
}

/// Integration panel data for the settings page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IntegrationInfo {
    pub status: ProviderStatus,
    pub status_text: String,
    pub provider: String,
    pub model: String,
    pub helper_service: crate::acer::services::ServiceState,
    pub acer_service: crate::acer::services::ServiceState,
    pub config_dir: String,
}

/// Everything the full window needs when it (re)loads.
#[derive(Debug, Clone, Serialize)]
pub struct WindowData<'a> {
    pub settings: &'a Settings,
    pub autostart: bool,
    pub integration: IntegrationInfo,
    pub history: crate::telemetry::history::HistoryView,
}

/// One monitoring sample (°C, %, RPM, MHz, W); pushed each telemetry tick while the window is open.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Sample {
    pub timestamp: u64,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub cpu_fan: Option<f32>,
    pub gpu_fan: Option<f32>,
    pub cpu_util: Option<f32>,
    pub gpu_util: Option<f32>,
    pub cpu_clock: Option<f32>,
    pub gpu_clock: Option<f32>,
    pub gpu_power: Option<f32>,
}

impl Sample {
    pub fn from_snapshot(s: &crate::telemetry::snapshot::SystemSnapshot) -> Self {
        let rpm = |f: Option<crate::telemetry::snapshot::FanTelemetry>| f.and_then(|f| f.rpm).map(|r| r as f32);
        Self {
            timestamp: s.timestamp_ms,
            cpu_temp: s.cpu.temperature_c,
            gpu_temp: s.gpu.temperature_c,
            cpu_fan: rpm(s.cpu_fan),
            gpu_fan: rpm(s.gpu_fan),
            cpu_util: s.cpu.utilisation_pct,
            gpu_util: s.gpu.utilisation_pct,
            cpu_clock: s.cpu.frequency_mhz.map(|v| v as f32),
            gpu_clock: s.gpu.frequency_mhz.map(|v| v as f32),
            gpu_power: s.gpu.power_w,
        }
    }
}
