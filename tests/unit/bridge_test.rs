//! @module bridge_test
//! @description Page command parsing is strict; script calls are guarded; samples mirror snapshots.
use nitrotray::config::settings::Settings;
use nitrotray::controls::{FanCurveProfile, FanMode, PerformanceMode};
use nitrotray::telemetry::snapshot::{CpuTelemetry, FanTelemetry, GpuTelemetry, SystemSnapshot};
use nitrotray::ui::bridge::*;

#[test]
fn parses_every_command_shape() {
    let cases = [
        (r#"{"cmd":"ready"}"#, UiCommand::Ready),
        (r#"{"cmd":"close_popup"}"#, UiCommand::ClosePopup),
        (r#"{"cmd":"drag_window"}"#, UiCommand::DragWindow),
        (r#"{"cmd":"open_monitoring"}"#, UiCommand::OpenMonitoring),
        (r#"{"cmd":"open_settings"}"#, UiCommand::OpenSettings),
        (r#"{"cmd":"open_fan_curve"}"#, UiCommand::OpenFanCurve),
        (r#"{"cmd":"open_nitro_sense"}"#, UiCommand::OpenNitroSense),
        (r#"{"cmd":"dismiss_notice"}"#, UiCommand::DismissNotice),
        (r#"{"cmd":"set_fan_mode","mode":"max"}"#, UiCommand::SetFanMode { mode: FanMode::Max }),
        (
            r#"{"cmd":"set_performance_mode","mode":"quiet"}"#,
            UiCommand::SetPerformanceMode { mode: PerformanceMode::Quiet },
        ),
        (r#"{"cmd":"set_autostart","enabled":true}"#, UiCommand::SetAutostart { enabled: true }),
        (r#"{"cmd":"export_diagnostics"}"#, UiCommand::ExportDiagnostics),
        (r#"{"cmd":"install_helper"}"#, UiCommand::InstallHelper),
        (r#"{"cmd":"pin_tray_icons"}"#, UiCommand::PinTrayIcons),
        (r#"{"cmd":"navigate","page":"fan_curve"}"#, UiCommand::Navigate { page: Page::FanCurve }),
    ];
    for (json, expected) in cases {
        assert_eq!(parse_command(json), Ok(expected), "{json}");
    }
}

#[test]
fn nested_payloads_round_trip() {
    let save = UiCommand::SaveSettings { settings: Box::new(Settings::default()) };
    assert_eq!(parse_command(&serde_json::to_string(&save).unwrap()), Ok(save));
    let curve = UiCommand::SaveFanCurve { profile: FanCurveProfile::default_profile(), apply: true };
    assert_eq!(parse_command(&serde_json::to_string(&curve).unwrap()), Ok(curve));
    let partial = r#"{"cmd":"save_settings","settings":{"general":{"hotkey":"Ctrl+Alt+K"}}}"#;
    match parse_command(partial).unwrap() {
        UiCommand::SaveSettings { settings } => assert_eq!(settings.general.hotkey, "Ctrl+Alt+K"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn rejects_unknown_or_malformed_commands() {
    for bad in [
        r#"{"cmd":"run_shell","line":"calc"}"#,
        r#"{"cmd":"ready","extra":1}"#,
        r#"{"cmd":"set_fan_mode","mode":"turbo"}"#,
        r#"{"cmd":"set_fan_mode"}"#,
        r#"["ready"]"#,
        "not json",
    ] {
        assert!(parse_command(bad).is_err(), "{bad}");
    }
    assert_eq!(parse_command(r#"{"cmd":"ready","x":1}"#).unwrap_err(), "unknown field 'x'");
    assert_eq!(parse_command(&" ".repeat(MAX_COMMAND_BYTES + 1)).unwrap_err(), "command too large");
}

#[test]
fn script_calls_are_guarded_and_json_escaped() {
    let js = script_call("onState", &serde_json::json!({"t":"</script>\"x"}));
    assert_eq!(js, r#"window.nitro&&window.nitro.onState&&window.nitro.onState({"t":"</script>\"x"});"#);
    assert_eq!(
        script_call("navigate", &Page::Settings),
        r#"window.nitro&&window.nitro.navigate&&window.nitro.navigate("settings");"#
    );
}

#[test]
fn sample_mirrors_snapshot() {
    let s = SystemSnapshot {
        timestamp_ms: 9,
        cpu: CpuTelemetry { temperature_c: Some(85.0), utilisation_pct: Some(21.0), frequency_mhz: Some(3947) },
        gpu: GpuTelemetry {
            temperature_c: Some(82.0),
            utilisation_pct: Some(93.0),
            frequency_mhz: Some(2565),
            power_w: Some(90.0),
        },
        cpu_fan: Some(FanTelemetry { rpm: Some(7317), duty_pct: None }),
        gpu_fan: None,
        ..SystemSnapshot::default()
    };
    let x = Sample::from_snapshot(&s);
    assert_eq!(x.timestamp, 9);
    assert_eq!((x.cpu_temp, x.gpu_temp, x.cpu_fan, x.gpu_fan), (Some(85.0), Some(82.0), Some(7317.0), None));
    assert_eq!(
        (x.cpu_util, x.gpu_util, x.cpu_clock, x.gpu_clock, x.gpu_power),
        (Some(21.0), Some(93.0), Some(3947.0), Some(2565.0), Some(90.0))
    );
}

#[test]
fn pending_action_serialises_kind_and_value() {
    let v = serde_json::to_value(PendingAction::Fan(FanMode::Custom)).unwrap();
    assert_eq!(v, serde_json::json!({"kind":"fan","value":"custom"}));
    let v = serde_json::to_value(PendingAction::Performance(PerformanceMode::Quiet)).unwrap();
    assert_eq!(v, serde_json::json!({"kind":"performance","value":"quiet"}));
    assert_eq!(serde_json::to_value(FanRowState::Unavailable).unwrap(), "unavailable");
    assert_eq!(serde_json::to_value(NoticeKind::Info).unwrap(), "info");
}
