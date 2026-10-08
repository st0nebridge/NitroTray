//! @module settings_test
//! @description Settings defaults, sanitisation of hostile input, and the Rust↔JS settings contract fixture.
use nitrotray::config::settings::*;
use nitrotray::controls::{CurvePoint, FanCurveProfile};
use nitrotray::telemetry::thresholds::TempThresholds;

const FIXTURE: &str = "tests/ui/fixtures/settings.default.json";

#[test]
fn defaults_follow_the_specification() {
    let s = Settings::default();
    assert!(s.general.start_minimised && s.general.open_popup_on_tray_click && s.general.close_on_focus_loss);
    assert_eq!(s.general.hotkey, "Win+Alt+N");
    assert_eq!(s.telemetry.cpu_thresholds, TempThresholds::CPU_DEFAULT);
    assert_eq!(s.telemetry.background_interval_ms, 2000);
    assert_eq!(s.appearance.theme, ThemeChoice::Dark);
    assert!(!s.startup.reapply_last_modes, "re-applying modes is opt-in only");
    assert!(s.tray_icons.cpu.enabled && s.tray_icons.gpu.enabled && s.tray_icons.show_app_icon);
    assert_ne!(s.tray_icons.cpu.temperature_color, s.tray_icons.gpu.temperature_color);
    assert!(s.graph_icons_enabled());
}

#[test]
fn contract_fixture_matches_rust_defaults() {
    let expected = serde_json::to_value(Settings::default()).unwrap();
    let text = std::fs::read_to_string(FIXTURE).expect("fixture exists; regenerate with NITROTRAY_WRITE_FIXTURES=1");
    if std::env::var("NITROTRAY_WRITE_FIXTURES").is_ok() {
        std::fs::write(FIXTURE, serde_json::to_string_pretty(&expected).unwrap() + "\n").unwrap();
        return;
    }
    let actual: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(actual, expected, "JS fixture out of date — rerun with NITROTRAY_WRITE_FIXTURES=1");
}

#[test]
fn unit_conversion() {
    assert_eq!(TemperatureUnit::Celsius.convert(85.0), 85.0);
    assert_eq!(TemperatureUnit::Fahrenheit.convert(100.0), 212.0);
    assert_eq!(TemperatureUnit::Fahrenheit.symbol(), "°F");
    assert_eq!(TemperatureUnit::Celsius.symbol(), "°C");
}

#[test]
fn hex_colour_rules() {
    assert!(is_hex_color("#A1b2C3"));
    for bad in ["A1B2C3", "#12345", "#1234567", "#GGGGGG", "", "#12 456"] {
        assert!(!is_hex_color(bad), "{bad}");
    }
}

#[test]
fn sanitising_repairs_hostile_values() {
    let mut s = Settings::default();
    s.telemetry.background_interval_ms = 10;
    s.telemetry.gpu_thresholds = TempThresholds { warm_c: 90.0, hot_c: 80.0, critical_c: 70.0 };
    s.tray_icons.cpu.temperature_color = "javascript:alert(1)".into();
    s.tray_icons.cpu.fan_color = "#abcdef".into();
    s.tray_icons.gpu.temperature_min_c = 90.0;
    s.tray_icons.gpu.temperature_max_c = 95.0;
    s.tray_icons.gpu.fan_max_rpm = 1_000_000;
    s.tray_icons.gpu.show_temperature = false;
    s.tray_icons.gpu.show_fan = false;
    s.general.hotkey = "N".into();
    s.integration.nitrosense_app_id = "calc.exe & whoami".into();
    let bad_curve = FanCurveProfile {
        name: "x".into(),
        cpu: nitrotray::controls::FanCurve { points: vec![CurvePoint { temp_c: 50, duty_pct: 50 }] },
        gpu: nitrotray::controls::FanCurve::default_curve(),
    };
    s.fan.custom_profile = Some(bad_curve);
    let s = s.sanitized();
    assert_eq!(s.telemetry.background_interval_ms, 1000);
    assert_eq!(s.telemetry.gpu_thresholds, TempThresholds::GPU_DEFAULT);
    assert_eq!(s.tray_icons.cpu.temperature_color, GraphIconSettings::cpu_default().temperature_color);
    assert_eq!(s.tray_icons.cpu.fan_color, "#ABCDEF", "valid colours are normalised, not replaced");
    assert_eq!((s.tray_icons.gpu.temperature_min_c, s.tray_icons.gpu.temperature_max_c), (30.0, 100.0));
    assert_eq!(s.tray_icons.gpu.fan_max_rpm, 20_000);
    assert!(s.tray_icons.gpu.show_temperature, "an icon always shows at least one series");
    assert_eq!(s.general.hotkey, "Win+Alt+N");
    assert_eq!(s.integration.nitrosense_app_id, nitrotray::acer::nitrosense::DEFAULT_AUMID);
    assert!(s.fan.custom_profile.is_none());
}

#[test]
fn at_least_one_tray_icon_stays_visible() {
    let mut s = Settings::default();
    s.tray_icons.show_app_icon = false;
    s.tray_icons.cpu.enabled = false;
    s.tray_icons.gpu.enabled = false;
    let s = s.sanitized();
    assert!(s.tray_icons.show_app_icon);
    assert!(!s.graph_icons_enabled());
}

#[test]
fn temperature_range_edges() {
    let mut s = Settings::default();
    s.tray_icons.cpu.temperature_min_c = 0.0;
    s.tray_icons.cpu.temperature_max_c = 10.0;
    assert_eq!(s.clone().sanitized().tray_icons.cpu.temperature_max_c, 10.0);
    s.tray_icons.cpu.temperature_min_c = 40.0;
    s.tray_icons.cpu.temperature_max_c = 50.0;
    let narrow = s.clone().sanitized().tray_icons.cpu;
    assert_eq!((narrow.temperature_min_c, narrow.temperature_max_c), (40.0, 50.0), "a 10 °C span is the minimum kept");
    s.tray_icons.cpu.temperature_max_c = 49.5;
    assert_eq!(s.clone().sanitized().tray_icons.cpu.temperature_max_c, 100.0, "narrower spans fall back");
    s.tray_icons.cpu.temperature_min_c = f32::NAN;
    assert_eq!(s.clone().sanitized().tray_icons.cpu.temperature_min_c, 30.0);
    s.tray_icons.cpu.temperature_min_c = 111.0;
    s.tray_icons.cpu.temperature_max_c = 120.0;
    assert_eq!(s.sanitized().tray_icons.cpu.temperature_min_c, 30.0);
}

#[test]
fn partial_json_fills_defaults() {
    let s: Settings =
        serde_json::from_str(r#"{"general":{"hotkey":"Ctrl+Alt+F9"},"telemetry":{"temperature_unit":"fahrenheit"}}"#)
            .unwrap();
    assert_eq!(s.general.hotkey, "Ctrl+Alt+F9");
    assert!(s.general.close_on_focus_loss);
    assert_eq!(s.telemetry.temperature_unit, TemperatureUnit::Fahrenheit);
    assert_eq!(s.tray_icons, TrayIconSettings::default());
    assert_eq!(GraphIconSettings::default(), GraphIconSettings::cpu_default());
    assert_eq!(serde_json::to_value(AccentIntensity::Vivid).unwrap(), "vivid");
}

#[test]
fn app_icon_may_be_hidden_when_a_graph_icon_is_shown() {
    let mut s = Settings::default();
    s.tray_icons.show_app_icon = false;
    s.tray_icons.gpu.enabled = false;
    let s = s.sanitized();
    assert!(!s.tray_icons.show_app_icon, "CPU graph is visible, so the app icon may stay hidden");
    let mut g = Settings::default();
    g.tray_icons.show_app_icon = false;
    g.tray_icons.cpu.enabled = false;
    assert!(!g.sanitized().tray_icons.show_app_icon, "GPU graph alone also suffices");
}
