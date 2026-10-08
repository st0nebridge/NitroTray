//! @module cu_20261001_002
//! @description Regression test (hover flyout on the CPU/GPU tray icons): on by default, shows embedded-controller
//! readings only (never utilisation/clocks, which would need NVML and can wake the discrete GPU), appears
//! only after the pointer rests, and goes away on leave or click so single-click keeps opening the popup.
use nitrotray::config::settings::Settings;
use nitrotray::telemetry::history::MetricsHistory;
use nitrotray::tray::hover::{Hover, HoverAction, SHOW_DELAY_MS};
use nitrotray::tray::positioning::Rect;
use nitrotray::ui::bridge::UiPrefs;
use nitrotray::ui::flyout::{state, FlyoutKind};

#[test]
fn hover_details_are_on_by_default() {
    assert!(Settings::default().tray_icons.hover_details);
    let old: Settings = serde_json::from_str(r#"{"tray_icons":{"show_app_icon":true}}"#).unwrap();
    assert!(old.tray_icons.hover_details, "configs written before the flyout get it too");
}

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

#[test]
fn hover_rests_before_showing_and_never_survives_a_click() {
    let icon = Rect::new(1500, 1040, 1524, 1064);
    let mut h = Hover::new();
    h.enter(FlyoutKind::Cpu, icon, 0);
    assert_eq!(h.tick(SHOW_DELAY_MS - 1), None);
    assert_eq!(h.tick(SHOW_DELAY_MS), Some(HoverAction::Show(FlyoutKind::Cpu, icon)));
    assert_eq!(h.click(), Some(HoverAction::Hide));
    h.enter(FlyoutKind::Cpu, icon, 1000);
    h.tick(1000 + SHOW_DELAY_MS);
    assert_eq!(h.leave(FlyoutKind::Cpu, 2000), Some(HoverAction::Hide));
}
