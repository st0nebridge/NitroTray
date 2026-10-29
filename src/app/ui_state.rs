//! @module app::ui_state
//! @description Derives the pages' `UiState` from the app model: processor and fan rows, the resolved theme
//! and preferences, and the tensor-core lock view. Part of `app::model`'s `AppModel`, split out to keep each
//! file to one purpose; it only reads the model.
//!
//! @input  `AppModel` (settings, snapshot, capabilities, notices, tensor tracker) and a millisecond clock.
//! @output `UiState`, `UiPrefs`, the resolved theme name.
//! @dependencies app::model, config, telemetry::{snapshot, fan}, ui::bridge
use crate::app::model::{connection_text, AppModel};
use crate::config::settings::ThemeChoice;
use crate::telemetry::fan::ring_pct;
use crate::telemetry::snapshot::FanTelemetry;
use crate::ui::bridge::{FanRowState, UiFan, UiPrefs, UiProcessor, UiState};

impl AppModel {
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
            unlock: self.unlock_view(now),
            prefs: self.prefs(),
        }
    }
}
