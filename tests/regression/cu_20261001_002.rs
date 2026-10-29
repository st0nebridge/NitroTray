//! @module cu_20261001_002
//! @description Regression test (CPU/GPU tray icon details popup): shows embedded-controller readings only
//! (never utilisation/clocks, which would need NVML and can wake the discrete GPU). Its hover timing and the
//! `hover_details` setting were replaced by click gestures in cu_20261029_001.
use nitrotray::config::settings::Settings;
use nitrotray::telemetry::history::MetricsHistory;
use nitrotray::ui::bridge::UiPrefs;
use nitrotray::ui::flyout::{state, FlyoutKind};

#[test]
fn flyout_carries_no_driver_metrics() {
    let s = Settings::default();
    let prefs = UiPrefs {
        theme: "dark",
        accent: s.appearance.accent_intensity,
        reduced_motion: false,
        transparency: false,
        unit_symbol: "°C",
        unit: s.telemetry.temperature_unit,
    };
    let json = serde_json::to_value(state(FlyoutKind::Gpu, &MetricsHistory::new(300), &s, prefs)).unwrap();
    let keys: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();
    for banned in ["util", "utilisation", "mhz", "clock", "power"] {
        assert!(!keys.iter().any(|k| k.contains(banned)), "flyout must not need NVML data: {keys:?}");
    }
}
