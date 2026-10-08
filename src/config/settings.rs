//! @module config::settings
//! @description User preferences with defaults and sanitisation of untrusted input.
//!
//! @input  JSON from config.json or from the settings page (both untrusted).
//! @output A `Settings` value whose every field is inside its allowed range.
//! @dependencies serde, controls, telemetry::{fan, scheduler, thresholds}, config::hotkey, acer::nitrosense
use serde::{Deserialize, Serialize};

use crate::acer::nitrosense;
use crate::config::hotkey::HotkeySpec;
use crate::controls::{FanCurveProfile, FanMode, PerformanceMode};
use crate::telemetry::fan::FanCalibration;
use crate::telemetry::scheduler::{RefreshRate, MAX_BACKGROUND_MS, MIN_BACKGROUND_MS};
use crate::telemetry::thresholds::TempThresholds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

impl TemperatureUnit {
    pub fn convert(self, celsius: f32) -> f32 {
        match self {
            Self::Celsius => celsius,
            Self::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    MatchWindows,
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccentIntensity {
    Subtle,
    #[default]
    Normal,
    Vivid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    /// When started with Windows, stay in the tray instead of opening the popup.
    pub start_minimised: bool,
    pub open_popup_on_tray_click: bool,
    pub close_on_focus_loss: bool,
    pub hotkey_enabled: bool,
    pub hotkey: String,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            start_minimised: true,
            open_popup_on_tray_click: true,
            close_on_focus_loss: true,
            hotkey_enabled: true,
            hotkey: "Win+Alt+N".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetrySettings {
    pub temperature_unit: TemperatureUnit,
    pub refresh_rate: RefreshRate,
    /// Poll interval while no UI is visible (graph icons only).
    pub background_interval_ms: u64,
    /// Query the GPU vendor driver (NVML) while a UI surface is visible.
    pub gpu_driver_queries: bool,
    pub fan_telemetry: bool,
    pub cpu_thresholds: TempThresholds,
    pub gpu_thresholds: TempThresholds,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            temperature_unit: TemperatureUnit::Celsius,
            refresh_rate: RefreshRate::Normal,
            background_interval_ms: 2000,
            gpu_driver_queries: true,
            fan_telemetry: true,
            cpu_thresholds: TempThresholds::CPU_DEFAULT,
            gpu_thresholds: TempThresholds::GPU_DEFAULT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: ThemeChoice,
    pub transparency: bool,
    pub accent_intensity: AccentIntensity,
    pub reduced_motion: bool,
}

/// One System Informer–style graph icon (temperature + fan history).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphIconSettings {
    pub enabled: bool,
    pub show_temperature: bool,
    pub show_fan: bool,
    pub temperature_color: String,
    pub fan_color: String,
    pub background_color: String,
    pub fill: bool,
    pub temperature_min_c: f32,
    pub temperature_max_c: f32,
    /// Full-scale RPM of the fan trace; 0 = scale to the highest RPM seen this session.
    pub fan_max_rpm: u32,
}

impl GraphIconSettings {
    pub fn cpu_default() -> Self {
        Self {
            enabled: true,
            show_temperature: true,
            show_fan: true,
            temperature_color: "#FF2A2A".into(),
            fan_color: "#22D3EE".into(),
            background_color: "#000000".into(),
            fill: true,
            temperature_min_c: 30.0,
            temperature_max_c: 100.0,
            fan_max_rpm: 0,
        }
    }

    pub fn gpu_default() -> Self {
        Self { temperature_color: "#FF9A2E".into(), fan_color: "#3DDC84".into(), ..Self::cpu_default() }
    }
}

impl Default for GraphIconSettings {
    fn default() -> Self {
        Self::cpu_default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrayIconSettings {
    pub show_app_icon: bool,
    /// Hovering the CPU or GPU icon shows its history flyout (in place of the plain tooltip).
    pub hover_details: bool,
    pub cpu: GraphIconSettings,
    pub gpu: GraphIconSettings,
}

impl Default for TrayIconSettings {
    fn default() -> Self {
        Self {
            show_app_icon: true,
            hover_details: true,
            cpu: GraphIconSettings::cpu_default(),
            gpu: GraphIconSettings::gpu_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IntegrationSettings {
    pub nitrosense_app_id: String,
}

impl Default for IntegrationSettings {
    fn default() -> Self {
        Self { nitrosense_app_id: nitrosense::DEFAULT_AUMID.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FanSettings {
    pub custom_profile: Option<FanCurveProfile>,
    pub calibration: FanCalibration,
}

/// Modes are only re-applied at startup when the user opts in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StartupSettings {
    pub reapply_last_modes: bool,
    pub last_fan_mode: Option<FanMode>,
    pub last_performance_mode: Option<PerformanceMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub telemetry: TelemetrySettings,
    pub appearance: AppearanceSettings,
    pub tray_icons: TrayIconSettings,
    pub integration: IntegrationSettings,
    pub fan: FanSettings,
    pub startup: StartupSettings,
}

/// `#RRGGBB` only.
pub fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn color_or(value: &str, fallback: &str) -> String {
    if is_hex_color(value) {
        value.to_ascii_uppercase()
    } else {
        fallback.to_string()
    }
}

fn sanitize_icon(icon: GraphIconSettings, fallback: &GraphIconSettings) -> GraphIconSettings {
    let mut i = icon;
    i.temperature_color = color_or(&i.temperature_color, &fallback.temperature_color);
    i.fan_color = color_or(&i.fan_color, &fallback.fan_color);
    i.background_color = color_or(&i.background_color, &fallback.background_color);
    let range_ok = i.temperature_min_c.is_finite()
        && i.temperature_max_c.is_finite()
        && (0.0..=110.0).contains(&i.temperature_min_c)
        && (10.0..=120.0).contains(&i.temperature_max_c)
        && i.temperature_max_c - i.temperature_min_c >= 10.0;
    if !range_ok {
        i.temperature_min_c = fallback.temperature_min_c;
        i.temperature_max_c = fallback.temperature_max_c;
    }
    i.fan_max_rpm = i.fan_max_rpm.min(20_000);
    if !i.show_temperature && !i.show_fan {
        i.show_temperature = true;
    }
    i
}

impl Settings {
    /// Clamps or resets every out-of-range field; never fails.
    pub fn sanitized(self) -> Self {
        let mut s = self;
        let t = &mut s.telemetry;
        t.background_interval_ms = t.background_interval_ms.clamp(MIN_BACKGROUND_MS, MAX_BACKGROUND_MS);
        t.cpu_thresholds = t.cpu_thresholds.or_default(TempThresholds::CPU_DEFAULT);
        t.gpu_thresholds = t.gpu_thresholds.or_default(TempThresholds::GPU_DEFAULT);

        let icons = &mut s.tray_icons;
        icons.cpu = sanitize_icon(icons.cpu.clone(), &GraphIconSettings::cpu_default());
        icons.gpu = sanitize_icon(icons.gpu.clone(), &GraphIconSettings::gpu_default());
        if !icons.show_app_icon && !icons.cpu.enabled && !icons.gpu.enabled {
            icons.show_app_icon = true;
        }

        if HotkeySpec::parse(&s.general.hotkey).is_none() {
            s.general.hotkey = GeneralSettings::default().hotkey;
        }
        if !nitrosense::is_valid_aumid(&s.integration.nitrosense_app_id) {
            s.integration.nitrosense_app_id = nitrosense::DEFAULT_AUMID.into();
        }
        if s.fan.custom_profile.as_ref().is_some_and(|p| p.validate().is_err()) {
            s.fan.custom_profile = None;
        }
        s
    }

    /// True when any graph icon needs background telemetry.
    pub fn graph_icons_enabled(&self) -> bool {
        self.tray_icons.cpu.enabled || self.tray_icons.gpu.enabled
    }
}
