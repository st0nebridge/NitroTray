//! @module ui::flyout
//! @description Content of the hover flyout shown over the CPU or GPU tray graph icon: that processor's
//! temperature and fan-speed history (the icon's graph, large), current values and temperature stats.
//!
//! @input  Which icon (CPU/GPU), the monitoring history, settings, and the page preferences.
//! @output A serialisable `FlyoutState` in the user's temperature unit, pushed to `flyout.html`.
//! @dependencies serde, config::settings, telemetry::{history, thresholds}, tray::graph_icon, ui::bridge
//!
//! Only embedded-controller readings (temperature, fan RPM) are shown: they are already polled for the
//! graph icons, so hovering never starts NVML queries that could wake the discrete GPU (D-20260930-010).
use serde::Serialize;

use crate::config::settings::{GraphIconSettings, Settings};
use crate::telemetry::history::{History, MetricsHistory};
use crate::telemetry::thresholds::TempLevel;
use crate::tray::graph_icon;
use crate::ui::bridge::UiPrefs;

/// Flyout edge in DIP.
pub const FLYOUT_SIZE: (f64, f64) = (340.0, 214.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlyoutKind {
    Cpu,
    Gpu,
}

impl FlyoutKind {
    /// The tray icon id that owns this flyout (`app::tray_host` builds icons "cpu" and "gpu").
    pub fn from_icon_id(id: &str) -> Option<Self> {
        match id {
            "cpu" => Some(Self::Cpu),
            "gpu" => Some(Self::Gpu),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Gpu => "GPU",
        }
    }
}

/// Minimum, mean and maximum of the temperatures in the graph window (user unit).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TempStats {
    pub min: f32,
    pub avg: f32,
    pub max: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlyoutState {
    pub kind: FlyoutKind,
    pub title: &'static str,
    /// Newest temperature in the user's unit.
    pub temp: Option<f32>,
    pub level: TempLevel,
    pub fan_rpm: Option<u32>,
    /// Oldest-first series; gaps are `null`. Temperatures are in the user's unit.
    pub temps: Vec<Option<f32>>,
    pub fans: Vec<Option<f32>>,
    /// Slots in the graph; the newest sample sits at the right edge.
    pub capacity: usize,
    /// Y range of the temperature trace in the user's unit (the icon's configured range).
    pub temp_range: [f32; 2],
    /// Full-scale RPM of the fan trace (the icon's rule: configured, or the peak plus headroom).
    pub fan_max: f32,
    pub temp_color: String,
    pub fan_color: String,
    pub fill: bool,
    pub show_fan: bool,
    pub stats: Option<TempStats>,
    /// Seconds between the oldest and newest sample.
    pub span_s: u64,
    pub prefs: UiPrefs,
}

pub fn stats(values: &[Option<f32>]) -> Option<TempStats> {
    let present: Vec<f32> = values.iter().flatten().copied().filter(|v| v.is_finite()).collect();
    if present.is_empty() {
        return None;
    }
    let min = present.iter().copied().fold(f32::INFINITY, f32::min);
    let max = present.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let avg = present.iter().sum::<f32>() / present.len() as f32;
    Some(TempStats { min, avg, max })
}

fn series(kind: FlyoutKind, h: &MetricsHistory) -> (&History, &History) {
    match kind {
        FlyoutKind::Cpu => (&h.cpu_temp, &h.cpu_fan),
        FlyoutKind::Gpu => (&h.gpu_temp, &h.gpu_fan),
    }
}

fn icon_settings(kind: FlyoutKind, settings: &Settings) -> &GraphIconSettings {
    match kind {
        FlyoutKind::Cpu => &settings.tray_icons.cpu,
        FlyoutKind::Gpu => &settings.tray_icons.gpu,
    }
}

pub fn state(kind: FlyoutKind, history: &MetricsHistory, settings: &Settings, prefs: UiPrefs) -> FlyoutState {
    let unit = settings.telemetry.temperature_unit;
    let icon = icon_settings(kind, settings);
    let thresholds = match kind {
        FlyoutKind::Cpu => settings.telemetry.cpu_thresholds,
        FlyoutKind::Gpu => settings.telemetry.gpu_thresholds,
    };
    let (temp_h, fan_h) = series(kind, history);
    let latest_c = temp_h.latest();
    let fan_on = settings.telemetry.fan_telemetry;
    let temps: Vec<Option<f32>> = temp_h.values().into_iter().map(|v| v.map(|c| unit.convert(c))).collect();
    let fans = if fan_on { fan_h.values() } else { Vec::new() };
    let span_s = match (history.timestamps.front(), history.timestamps.back()) {
        (Some(first), Some(last)) => last.saturating_sub(*first) / 1000,
        _ => 0,
    };
    FlyoutState {
        kind,
        title: kind.title(),
        temp: latest_c.map(|c| unit.convert(c)),
        level: thresholds.classify(latest_c),
        fan_rpm: if fan_on { fan_h.latest().map(|r| r.round() as u32) } else { None },
        stats: stats(&temps),
        temps,
        fans,
        capacity: history.cpu_temp.capacity(),
        temp_range: [unit.convert(icon.temperature_min_c), unit.convert(icon.temperature_max_c)],
        fan_max: graph_icon::fan_scale(icon.fan_max_rpm, fan_h.peak()),
        temp_color: icon.temperature_color.clone(),
        fan_color: icon.fan_color.clone(),
        fill: icon.fill,
        show_fan: fan_on,
        span_s,
        prefs,
    }
}
