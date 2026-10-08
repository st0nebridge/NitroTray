//! @module flyout_test
//! @description Hover flyout content: series per icon, unit conversion, stats, fan scale, span and gaps.
use nitrotray::config::settings::{AccentIntensity, Settings, TemperatureUnit};
use nitrotray::telemetry::history::MetricsHistory;
use nitrotray::telemetry::snapshot::{CpuTelemetry, FanTelemetry, GpuTelemetry, SystemSnapshot};
use nitrotray::telemetry::thresholds::TempLevel;
use nitrotray::ui::bridge::UiPrefs;
use nitrotray::ui::flyout::{state, stats, FlyoutKind, TempStats, FLYOUT_SIZE};

fn prefs() -> UiPrefs {
    UiPrefs {
        theme: "dark",
        accent: AccentIntensity::Normal,
        reduced_motion: false,
        transparency: false,
        unit_symbol: "°C",
        unit: TemperatureUnit::Celsius,
    }
}

fn snap(t: u64, cpu: Option<f32>, cpu_rpm: Option<u32>, gpu: Option<f32>, gpu_rpm: Option<u32>) -> SystemSnapshot {
    SystemSnapshot {
        timestamp_ms: t,
        cpu: CpuTelemetry { temperature_c: cpu, utilisation_pct: Some(10.0), frequency_mhz: Some(3000) },
        gpu: GpuTelemetry { temperature_c: gpu, utilisation_pct: None, frequency_mhz: None, power_w: None },
        cpu_fan: Some(FanTelemetry { rpm: cpu_rpm, duty_pct: None }),
        gpu_fan: Some(FanTelemetry { rpm: gpu_rpm, duty_pct: None }),
        ..SystemSnapshot::default()
    }
}

fn history() -> MetricsHistory {
    let mut h = MetricsHistory::new(300);
    h.push(&snap(10_000, Some(60.0), Some(3000), Some(50.0), Some(3500)));
    h.push(&snap(12_000, None, None, Some(52.0), Some(3600)));
    h.push(&snap(70_000, Some(90.0), Some(4400), Some(56.0), Some(3700)));
    h
}

#[test]
fn icon_ids_map_to_kinds() {
    assert_eq!(FlyoutKind::from_icon_id("cpu"), Some(FlyoutKind::Cpu));
    assert_eq!(FlyoutKind::from_icon_id("gpu"), Some(FlyoutKind::Gpu));
    assert_eq!(FlyoutKind::from_icon_id("app"), None);
    assert_eq!((FlyoutKind::Cpu.title(), FlyoutKind::Gpu.title()), ("CPU", "GPU"));
    assert!(FLYOUT_SIZE.0 > FLYOUT_SIZE.1 && FLYOUT_SIZE.1 > 150.0);
}

#[test]
fn cpu_flyout_carries_its_own_series_stats_and_icon_style() {
    let s = Settings::default();
    let f = state(FlyoutKind::Cpu, &history(), &s, prefs());
    assert_eq!(f.title, "CPU");
    assert_eq!(f.temps, vec![Some(60.0), None, Some(90.0)]);
    assert_eq!(f.fans, vec![Some(3000.0), None, Some(4400.0)]);
    assert_eq!((f.temp, f.fan_rpm), (Some(90.0), Some(4400)));
    assert_eq!(f.level, TempLevel::Hot, "90 °C is in the CPU red band (85–94)");
    assert_eq!(f.stats, Some(TempStats { min: 60.0, avg: 75.0, max: 90.0 }), "gaps are ignored");
    assert_eq!(f.span_s, 60);
    assert_eq!(f.capacity, 300);
    assert_eq!(f.temp_range, [30.0, 100.0]);
    assert_eq!(f.temp_color, s.tray_icons.cpu.temperature_color);
    assert_eq!(f.fan_color, s.tray_icons.cpu.fan_color);
    assert!(f.fill && f.show_fan);
    assert_eq!(f.fan_max, 3000.0f32.max(4400.0 * 1.1), "auto scale: peak plus headroom");
}

#[test]
fn gpu_flyout_uses_gpu_series_thresholds_and_colours() {
    let mut s = Settings::default();
    s.tray_icons.gpu.fan_max_rpm = 6000;
    let f = state(FlyoutKind::Gpu, &history(), &s, prefs());
    assert_eq!(f.temps, vec![Some(50.0), Some(52.0), Some(56.0)]);
    assert_eq!(f.fan_rpm, Some(3700));
    assert_eq!(f.level, TempLevel::Normal);
    assert_eq!(f.fan_max, 6000.0, "a configured full scale wins");
    assert_eq!(f.temp_color, s.tray_icons.gpu.temperature_color);
    assert_ne!(f.temp_color, s.tray_icons.cpu.temperature_color);
}

#[test]
fn fahrenheit_converts_values_range_and_stats() {
    let mut s = Settings::default();
    s.telemetry.temperature_unit = TemperatureUnit::Fahrenheit;
    let f = state(FlyoutKind::Cpu, &history(), &s, prefs());
    assert_eq!(f.temps, vec![Some(140.0), None, Some(194.0)]);
    assert_eq!(f.temp, Some(194.0));
    assert_eq!(f.temp_range, [86.0, 212.0]);
    assert_eq!(f.stats.map(|t| (t.min, t.max)), Some((140.0, 194.0)));
    assert_eq!(f.level, TempLevel::Hot, "bands are classified in °C");
}

#[test]
fn disabled_fan_telemetry_hides_the_fan_trace() {
    let mut s = Settings::default();
    s.telemetry.fan_telemetry = false;
    let f = state(FlyoutKind::Cpu, &history(), &s, prefs());
    assert!(f.fans.is_empty());
    assert_eq!(f.fan_rpm, None);
    assert!(!f.show_fan);
}

#[test]
fn empty_history_is_honest() {
    let f = state(FlyoutKind::Gpu, &MetricsHistory::new(300), &Settings::default(), prefs());
    assert!(f.temps.is_empty());
    assert_eq!((f.temp, f.fan_rpm, f.stats, f.span_s), (None, None, None, 0));
    assert_eq!(f.level, TempLevel::Unknown);
    assert_eq!(f.fan_max, 3000.0, "minimum automatic fan scale");
}

#[test]
fn stats_skip_gaps_and_non_finite_values() {
    assert_eq!(stats(&[]), None);
    assert_eq!(stats(&[None, None]), None);
    assert_eq!(stats(&[Some(f32::NAN), Some(4.0), None, Some(2.0)]), Some(TempStats { min: 2.0, avg: 3.0, max: 4.0 }));
}

#[test]
fn serialises_for_the_page() {
    let f = state(FlyoutKind::Cpu, &history(), &Settings::default(), prefs());
    let v = serde_json::to_value(&f).unwrap();
    assert_eq!(v["kind"], "cpu");
    assert_eq!(v["level"], "hot");
    assert_eq!(v["temps"][1], serde_json::Value::Null);
    assert_eq!(v["prefs"]["theme"], "dark");
}
