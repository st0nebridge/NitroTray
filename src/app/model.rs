//! @module app::model
//! @description Application state machine: turns telemetry, UI commands, menu clicks and control results
//! into UI state and side-effect requests. Pure — the runtime performs the effects.
//!
//! @input  `TelemetryUpdate`, `UiCommand`, `MenuAction`, control results, a millisecond clock.
//! @output `UiState` for the webviews, `Vec<Effect>` for the runtime.
//! @dependencies acer, config, controls, ipc::protocol, telemetry::{snapshot, fan}, tray::menu, ui::bridge
use crate::acer::Capabilities;
use crate::config::settings::{Settings, ThemeChoice};
use crate::controls::{FanCurveProfile, FanMode, PerformanceMode};
use crate::ipc::protocol::ProviderStatus;
use crate::telemetry::fan::{ring_pct, Calibrator};
use crate::telemetry::snapshot::{FanTelemetry, SystemSnapshot};
use crate::tray::menu::MenuAction;
use crate::ui::bridge::{
    FanRowState, Notice, NoticeKind, Page, PendingAction, UiCommand, UiFan, UiPrefs, UiProcessor, UiState,
};

pub const ERROR_NOTICE_MS: u64 = 6000;
pub const INFO_NOTICE_MS: u64 = 3500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlRequest {
    Fan(FanMode, Option<FanCurveProfile>),
    Performance(PerformanceMode),
}

impl ControlRequest {
    fn label(&self) -> String {
        match self {
            Self::Fan(m, _) => format!("{} fan", m.label()),
            Self::Performance(m) => m.label().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Control(ControlRequest),
    ShowPopup,
    HidePopup,
    DragPopup,
    OpenWindow(Page),
    LaunchNitroSense(String),
    PersistSettings,
    ApplySettings,
    SetAutostart(bool),
    ExportDiagnostics,
    InstallHelper,
    PinTrayIcons,
    SendWindowData,
    RefreshModes,
    Exit,
}

/// One telemetry delivery from the worker.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryUpdate {
    pub snapshot: SystemSnapshot,
    pub status: ProviderStatus,
    pub capabilities: Capabilities,
    pub provider: String,
    pub model: String,
}

pub struct AppModel {
    pub settings: Settings,
    snapshot: SystemSnapshot,
    caps: Capabilities,
    status: ProviderStatus,
    provider: String,
    model: String,
    pending: Option<PendingAction>,
    notice: Option<(Notice, u64)>,
    calibrator: Calibrator,
    autostart: bool,
    light_system_theme: bool,
    reapplied: bool,
}

pub fn connection_text(status: ProviderStatus) -> &'static str {
    match status {
        ProviderStatus::Connected => "Connected to the Acer firmware interface.",
        ProviderStatus::Simulated => "Simulated hardware (development helper).",
        ProviderStatus::AccessDenied => "The NitroTray helper is running without administrator rights.",
        ProviderStatus::NotPresent => "This PC does not expose the Acer gaming firmware interface.",
        ProviderStatus::HelperUnavailable => {
            "The NitroTray helper service is not running. Install it from Settings › Integration."
        }
    }
}

impl AppModel {
    pub fn new(settings: Settings, autostart: bool, light_system_theme: bool) -> Self {
        Self {
            settings,
            snapshot: SystemSnapshot::default(),
            caps: Capabilities::none(),
            status: ProviderStatus::HelperUnavailable,
            provider: String::new(),
            model: String::new(),
            pending: None,
            notice: None,
            calibrator: Calibrator::default(),
            autostart,
            light_system_theme,
            reapplied: false,
        }
    }

    pub fn snapshot(&self) -> &SystemSnapshot {
        &self.snapshot
    }

    pub fn capabilities(&self) -> Capabilities {
        self.caps
    }

    pub fn status(&self) -> ProviderStatus {
        self.status
    }

    pub fn autostart(&self) -> bool {
        self.autostart
    }

    pub fn set_autostart(&mut self, enabled: bool) {
        self.autostart = enabled;
    }

    pub fn set_light_system_theme(&mut self, light: bool) {
        self.light_system_theme = light;
    }

    fn connected(&self) -> bool {
        matches!(self.status, ProviderStatus::Connected | ProviderStatus::Simulated)
    }

    pub fn notify(&mut self, kind: NoticeKind, title: impl Into<String>, detail: impl Into<String>, now: u64) {
        let ttl = if kind == NoticeKind::Error { ERROR_NOTICE_MS } else { INFO_NOTICE_MS };
        self.notice = Some((Notice { kind, title: title.into(), detail: detail.into() }, now + ttl));
    }

    pub fn on_telemetry(&mut self, u: TelemetryUpdate, now: u64) -> Vec<Effect> {
        let mut effects = Vec::new();
        let rpm = |f: Option<FanTelemetry>| f.and_then(|f| f.rpm);
        if let Some(cal) = self.calibrator.observe(
            u.snapshot.fan_mode,
            (rpm(u.snapshot.cpu_fan), rpm(u.snapshot.gpu_fan)),
            now,
            self.settings.fan.calibration,
        ) {
            self.settings.fan.calibration = cal;
            effects.push(Effect::PersistSettings);
        }
        self.snapshot = u.snapshot;
        self.status = u.status;
        self.caps = u.capabilities;
        self.provider = u.provider;
        self.model = u.model;
        if self.connected() && !self.reapplied {
            self.reapplied = true;
            effects.extend(self.startup_reapply());
        }
        effects
    }

    /// Only when the user opted in.
    fn startup_reapply(&mut self) -> Vec<Effect> {
        let s = &self.settings.startup;
        if !s.reapply_last_modes {
            return vec![];
        }
        let mut out = Vec::new();
        if let Some(m) = s.last_performance_mode.filter(|m| self.caps.supports_performance(*m)) {
            out.push(Effect::Control(ControlRequest::Performance(m)));
        }
        if let Some(m) = s.last_fan_mode.filter(|m| self.caps.supports_fan(*m)) {
            let curve = if m == FanMode::Custom { self.settings.fan.custom_profile.clone() } else { None };
            if m != FanMode::Custom || curve.is_some() {
                out.push(Effect::Control(ControlRequest::Fan(m, curve)));
            }
        }
        out
    }

    fn blocked(&mut self, what: &str, now: u64) -> bool {
        if self.pending.is_some() {
            return true;
        }
        if !self.connected() {
            self.notify(NoticeKind::Error, format!("{what} unavailable."), connection_text(self.status), now);
            return true;
        }
        false
    }

    pub fn request_fan(&mut self, mode: FanMode, now: u64) -> Vec<Effect> {
        if self.blocked("Fan control", now) {
            return vec![];
        }
        if !self.caps.supports_fan(mode) {
            let title = format!("Could not switch to {} fan mode.", mode.label());
            self.notify(
                NoticeKind::Error,
                title,
                format!("{} fan mode is not supported on this model.", mode.label()),
                now,
            );
            return vec![];
        }
        let curve = match mode {
            FanMode::Custom => match self.settings.fan.custom_profile.clone() {
                Some(p) => Some(p),
                None => return vec![Effect::OpenWindow(Page::FanCurve)],
            },
            _ if self.snapshot.fan_mode == Some(mode) => return vec![],
            _ => None,
        };
        self.pending = Some(PendingAction::Fan(mode));
        vec![Effect::Control(ControlRequest::Fan(mode, curve))]
    }

    pub fn request_performance(&mut self, mode: PerformanceMode, now: u64) -> Vec<Effect> {
        if self.blocked("Performance control", now) {
            return vec![];
        }
        if !self.caps.supports_performance(mode) {
            let title = format!("Could not switch to {} mode.", mode.label());
            self.notify(
                NoticeKind::Error,
                title,
                format!("{} mode is not supported on this model.", mode.label()),
                now,
            );
            return vec![];
        }
        if self.snapshot.performance_mode == Some(mode) {
            return vec![];
        }
        self.pending = Some(PendingAction::Performance(mode));
        vec![Effect::Control(ControlRequest::Performance(mode))]
    }

    pub fn on_control_result(&mut self, req: &ControlRequest, result: Result<String, String>, now: u64) -> Vec<Effect> {
        self.pending = None;
        match result {
            Ok(_) => {
                match req {
                    ControlRequest::Fan(m, _) => {
                        self.snapshot.fan_mode = Some(*m);
                        self.settings.startup.last_fan_mode = Some(*m);
                    }
                    ControlRequest::Performance(m) => {
                        self.snapshot.performance_mode = Some(*m);
                        self.snapshot.performance_raw = None;
                        self.settings.startup.last_performance_mode = Some(*m);
                    }
                }
                vec![Effect::PersistSettings, Effect::RefreshModes]
            }
            Err(detail) => {
                let title = format!("Could not switch to {} mode.", req.label());
                self.notify(NoticeKind::Error, title, detail, now);
                vec![Effect::RefreshModes]
            }
        }
    }

    /// Accepts edits from the settings page, keeping fields the runtime owns.
    fn save_settings(&mut self, incoming: Settings, now: u64) -> Vec<Effect> {
        let mut next = incoming.sanitized();
        next.fan = self.settings.fan.clone();
        next.startup.last_fan_mode = self.settings.startup.last_fan_mode;
        next.startup.last_performance_mode = self.settings.startup.last_performance_mode;
        self.settings = next;
        self.notify(NoticeKind::Info, "Settings saved.", "", now);
        vec![Effect::PersistSettings, Effect::ApplySettings, Effect::SendWindowData]
    }

    fn save_curve(&mut self, profile: FanCurveProfile, apply: bool, now: u64) -> Vec<Effect> {
        if let Err(e) = profile.validate() {
            self.notify(NoticeKind::Error, "Fan curve not saved.", e.to_string(), now);
            return vec![Effect::SendWindowData];
        }
        self.settings.fan.custom_profile = Some(profile);
        let mut effects = vec![Effect::PersistSettings, Effect::SendWindowData];
        if apply {
            effects.extend(self.request_fan(FanMode::Custom, now));
        } else {
            self.notify(NoticeKind::Info, "Fan curve saved.", "", now);
        }
        effects
    }

    pub fn on_command(&mut self, cmd: UiCommand, now: u64) -> Vec<Effect> {
        match cmd {
            UiCommand::Ready => vec![Effect::SendWindowData],
            UiCommand::ClosePopup => vec![Effect::HidePopup],
            UiCommand::DragWindow => vec![Effect::DragPopup],
            UiCommand::OpenMonitoring => vec![Effect::HidePopup, Effect::OpenWindow(Page::Monitoring)],
            UiCommand::OpenSettings => vec![Effect::HidePopup, Effect::OpenWindow(Page::Settings)],
            UiCommand::OpenFanCurve => vec![Effect::HidePopup, Effect::OpenWindow(Page::FanCurve)],
            UiCommand::Navigate { page } => vec![Effect::OpenWindow(page)],
            UiCommand::OpenNitroSense => {
                vec![Effect::HidePopup, Effect::LaunchNitroSense(self.settings.integration.nitrosense_app_id.clone())]
            }
            UiCommand::DismissNotice => {
                self.notice = None;
                vec![]
            }
            UiCommand::SetFanMode { mode } => self.request_fan(mode, now),
            UiCommand::SetPerformanceMode { mode } => self.request_performance(mode, now),
            UiCommand::SaveSettings { settings } => self.save_settings(*settings, now),
            UiCommand::SaveFanCurve { profile, apply } => self.save_curve(profile, apply, now),
            UiCommand::SetAutostart { enabled } => vec![Effect::SetAutostart(enabled)],
            UiCommand::ExportDiagnostics => vec![Effect::ExportDiagnostics],
            UiCommand::InstallHelper => vec![Effect::InstallHelper],
            UiCommand::PinTrayIcons => vec![Effect::PinTrayIcons],
        }
    }

    pub fn on_menu(&mut self, action: MenuAction, now: u64) -> Vec<Effect> {
        match action {
            MenuAction::OpenPopup => vec![Effect::ShowPopup],
            MenuAction::SetPerformance(m) => self.request_performance(m, now),
            MenuAction::SetFan(m) => self.request_fan(m, now),
            MenuAction::OpenNitroSense => {
                vec![Effect::LaunchNitroSense(self.settings.integration.nitrosense_app_id.clone())]
            }
            MenuAction::OpenMonitoring => vec![Effect::OpenWindow(Page::Monitoring)],
            MenuAction::OpenSettings => vec![Effect::OpenWindow(Page::Settings)],
            MenuAction::ToggleAutostart => vec![Effect::SetAutostart(!self.autostart)],
            MenuAction::Exit => vec![Effect::Exit],
        }
    }

    fn processor(
        &self,
        t: &crate::telemetry::snapshot::CpuTelemetry,
        thresholds: crate::telemetry::thresholds::TempThresholds,
    ) -> UiProcessor {
        let unit = self.settings.telemetry.temperature_unit;
        UiProcessor {
            temp: t.temperature_c.map(|c| unit.convert(c)),
            level: Some(thresholds.classify(t.temperature_c)),
            util: t.utilisation_pct,
            mhz: t.frequency_mhz,
        }
    }

    fn fan_row(&self, fan: Option<FanTelemetry>, present: bool, calibrated: Option<u32>) -> UiFan {
        if !self.settings.telemetry.fan_telemetry {
            return UiFan { state: FanRowState::Disabled, rpm: None, ring_pct: None };
        }
        match fan.filter(|f| present && f.rpm.is_some()) {
            Some(f) => UiFan { state: FanRowState::Ok, rpm: f.rpm, ring_pct: ring_pct(&f, calibrated) },
            None => UiFan { state: FanRowState::Unavailable, rpm: None, ring_pct: None },
        }
    }

    pub fn resolved_theme(&self) -> &'static str {
        match self.settings.appearance.theme {
            ThemeChoice::Dark => "dark",
            ThemeChoice::Light => "light",
            ThemeChoice::MatchWindows if self.light_system_theme => "light",
            ThemeChoice::MatchWindows => "dark",
        }
    }

    /// Theme, accent, motion and unit preferences shared by every page.
    pub fn prefs(&self) -> UiPrefs {
        let a = &self.settings.appearance;
        let unit = self.settings.telemetry.temperature_unit;
        UiPrefs {
            theme: self.resolved_theme(),
            accent: a.accent_intensity,
            reduced_motion: a.reduced_motion,
            transparency: a.transparency,
            unit_symbol: unit.symbol(),
            unit,
        }
    }

    pub fn ui_state(&mut self, now: u64) -> UiState {
        if self.notice.as_ref().is_some_and(|(_, until)| now >= *until) {
            self.notice = None;
        }
        let s = &self.snapshot;
        let t = &self.settings.telemetry;
        let gpu_as_cpu = crate::telemetry::snapshot::CpuTelemetry {
            temperature_c: s.gpu.temperature_c,
            utilisation_pct: s.gpu.utilisation_pct,
            frequency_mhz: s.gpu.frequency_mhz,
        };
        let cal = self.settings.fan.calibration;
        UiState {
            cpu: self.processor(&s.cpu, t.cpu_thresholds),
            gpu: self.processor(&gpu_as_cpu, t.gpu_thresholds),
            cpu_fan: self.fan_row(s.cpu_fan, self.caps.cpu_fan, cal.cpu_max_rpm),
            gpu_fan: self.fan_row(s.gpu_fan, self.caps.gpu_fan, cal.gpu_max_rpm),
            fan_mode: s.fan_mode,
            performance_mode: s.performance_mode,
            performance_other: s
                .performance_raw
                .map(|r| crate::controls::performance_mode::firmware_profile_name(r).to_string()),
            capabilities: self.caps,
            connection: self.status,
            connection_text: connection_text(self.status).into(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            has_custom_profile: self.settings.fan.custom_profile.is_some(),
            pending: self.pending.clone(),
            notice: self.notice.as_ref().map(|(n, _)| n.clone()),
            prefs: self.prefs(),
        }
    }
}
