//! @module model_test
//! @description App state machine: control gating, pending/results, notices, calibration,
//! opt-in reapply, settings merge, menu actions and derived UI state.
use nitrotray::acer::Capabilities;
use nitrotray::app::model::*;
use nitrotray::config::settings::{Settings, TemperatureUnit, ThemeChoice};
use nitrotray::controls::{FanCurveProfile, FanMode, PerformanceMode};
use nitrotray::ipc::protocol::ProviderStatus;
use nitrotray::telemetry::fan::CALIBRATION_SETTLE_MS;
use nitrotray::telemetry::snapshot::{CpuTelemetry, FanTelemetry, GpuTelemetry, SystemSnapshot};
use nitrotray::telemetry::thresholds::TempLevel;
use nitrotray::tray::menu::MenuAction;
use nitrotray::ui::bridge::{FanRowState, NoticeKind, Page, PendingAction, UiCommand};

use crate::support::all_caps;

fn snapshot() -> SystemSnapshot {
    SystemSnapshot {
        timestamp_ms: 0,
        cpu: CpuTelemetry { temperature_c: Some(85.0), utilisation_pct: Some(21.0), frequency_mhz: Some(3947) },
        gpu: GpuTelemetry {
            temperature_c: Some(82.0),
            utilisation_pct: Some(93.0),
            frequency_mhz: Some(2565),
            power_w: None,
        },
        cpu_fan: Some(FanTelemetry { rpm: Some(7317), duty_pct: None }),
        gpu_fan: Some(FanTelemetry { rpm: Some(7692), duty_pct: None }),
        fan_mode: Some(FanMode::Auto),
        performance_mode: Some(PerformanceMode::Default),
        performance_raw: None,
    }
}

fn update(status: ProviderStatus, caps: Capabilities) -> TelemetryUpdate {
    TelemetryUpdate {
        snapshot: snapshot(),
        status,
        capabilities: caps,
        provider: "acer-wmi".into(),
        model: "Nitro AN515-58".into(),
    }
}

fn connected() -> AppModel {
    let mut m = AppModel::new(Settings::default(), false, false);
    m.on_telemetry(update(ProviderStatus::Connected, all_caps()), 0);
    m
}

#[test]
fn ui_state_reflects_the_reference_snapshot() {
    let mut m = connected();
    let s = m.ui_state(0);
    assert_eq!(
        (s.cpu.temp, s.cpu.level, s.cpu.util, s.cpu.mhz),
        (Some(85.0), Some(TempLevel::Hot), Some(21.0), Some(3947))
    );
    assert_eq!(s.gpu.level, Some(TempLevel::Hot), "default GPU bands: 80–89 red");
    assert_eq!(s.cpu_fan.state, FanRowState::Ok);
    assert_eq!(s.cpu_fan.ring_pct, None, "no duty and no calibration → no arc");
    assert_eq!((s.fan_mode, s.performance_mode), (Some(FanMode::Auto), Some(PerformanceMode::Default)));
    assert_eq!(s.connection, ProviderStatus::Connected);
    assert_eq!((s.provider.as_str(), s.model.as_str()), ("acer-wmi", "Nitro AN515-58"));
    assert_eq!(s.prefs.theme, "dark");
    assert_eq!(s.prefs.unit_symbol, "°C");
    assert!(!s.has_custom_profile);
    assert!(
        m.snapshot().cpu_fan.is_some() && m.capabilities().custom_fan_mode && m.status() == ProviderStatus::Connected
    );
}

#[test]
fn fahrenheit_and_disabled_fan_rows() {
    let mut settings = Settings::default();
    settings.telemetry.temperature_unit = TemperatureUnit::Fahrenheit;
    settings.telemetry.fan_telemetry = false;
    let mut m = AppModel::new(settings, false, false);
    m.on_telemetry(update(ProviderStatus::Connected, all_caps()), 0);
    let s = m.ui_state(0);
    assert_eq!(s.cpu.temp, Some(185.0));
    assert_eq!(s.cpu.level, Some(TempLevel::Hot), "classified in °C");
    assert_eq!(s.cpu_fan.state, FanRowState::Disabled);
    assert_eq!(s.prefs.unit_symbol, "°F");
}

#[test]
fn unsupported_fan_rows_are_unavailable() {
    let mut caps = all_caps();
    caps.gpu_fan = false;
    let mut m = AppModel::new(Settings::default(), false, false);
    m.on_telemetry(update(ProviderStatus::Connected, caps), 0);
    assert_eq!(m.ui_state(0).gpu_fan.state, FanRowState::Unavailable);
}

#[test]
fn control_requires_a_connected_helper() {
    let mut m = AppModel::new(Settings::default(), false, false);
    m.on_telemetry(update(ProviderStatus::HelperUnavailable, Capabilities::none()), 0);
    assert!(m.request_fan(FanMode::Max, 10).is_empty());
    let n = m.ui_state(10).notice.unwrap();
    assert_eq!(n.kind, NoticeKind::Error);
    assert_eq!(n.title, "Fan control unavailable.");
    assert!(n.detail.contains("helper service is not running"));
    assert!(m.request_performance(PerformanceMode::Quiet, 11).is_empty());
    assert_eq!(m.ui_state(11).notice.unwrap().title, "Performance control unavailable.");
}

#[test]
fn unsupported_modes_explain_themselves() {
    let mut caps = all_caps();
    caps.max_fan_mode = false;
    caps.quiet_mode = false;
    let mut m = AppModel::new(Settings::default(), false, false);
    m.on_telemetry(update(ProviderStatus::Connected, caps), 0);
    assert!(m.request_fan(FanMode::Max, 0).is_empty());
    assert_eq!(m.ui_state(0).notice.unwrap().title, "Could not switch to Max fan mode.");
    assert!(m.request_performance(PerformanceMode::Quiet, 0).is_empty());
    assert_eq!(m.ui_state(0).notice.unwrap().detail, "Quiet mode is not supported on this model.");
}

#[test]
fn fan_request_sets_pending_and_blocks_duplicates() {
    let mut m = connected();
    assert_eq!(m.request_fan(FanMode::Max, 0), vec![Effect::Control(ControlRequest::Fan(FanMode::Max, None))]);
    assert_eq!(m.ui_state(0).pending, Some(PendingAction::Fan(FanMode::Max)));
    assert!(m.request_performance(PerformanceMode::Quiet, 0).is_empty(), "one action at a time");
    assert!(m.request_fan(FanMode::Auto, 0).is_empty());
}

#[test]
fn selecting_the_current_mode_is_a_no_op() {
    let mut m = connected();
    assert!(m.request_fan(FanMode::Auto, 0).is_empty());
    assert!(m.request_performance(PerformanceMode::Default, 0).is_empty());
    assert!(m.ui_state(0).notice.is_none());
}

#[test]
fn custom_without_a_profile_opens_the_editor() {
    let mut m = connected();
    assert_eq!(m.request_fan(FanMode::Custom, 0), vec![Effect::OpenWindow(Page::FanCurve)]);
    assert_eq!(m.ui_state(0).pending, None);
    m.settings.fan.custom_profile = Some(FanCurveProfile::default_profile());
    assert_eq!(
        m.request_fan(FanMode::Custom, 0),
        vec![Effect::Control(ControlRequest::Fan(FanMode::Custom, Some(FanCurveProfile::default_profile())))]
    );
}

#[test]
fn successful_results_update_state_and_remember_modes() {
    let mut m = connected();
    m.request_performance(PerformanceMode::Performance, 0);
    let req = ControlRequest::Performance(PerformanceMode::Performance);
    assert_eq!(
        m.on_control_result(&req, Ok("performance".into()), 1),
        vec![Effect::PersistSettings, Effect::RefreshModes]
    );
    let s = m.ui_state(1);
    assert_eq!((s.pending, s.performance_mode), (None, Some(PerformanceMode::Performance)));
    assert_eq!(m.settings.startup.last_performance_mode, Some(PerformanceMode::Performance));
    m.on_control_result(&ControlRequest::Fan(FanMode::Max, None), Ok("max".into()), 2);
    assert_eq!(m.settings.startup.last_fan_mode, Some(FanMode::Max));
}

#[test]
fn failures_use_the_specified_wording_and_expire() {
    let mut m = connected();
    m.request_performance(PerformanceMode::Performance, 0);
    let req = ControlRequest::Performance(PerformanceMode::Performance);
    let effects = m.on_control_result(&req, Err("NitroSense service did not accept the request.".into()), 100);
    assert_eq!(effects, vec![Effect::RefreshModes]);
    let n = m.ui_state(100).notice.unwrap();
    assert_eq!(n.title, "Could not switch to Performance mode.");
    assert_eq!(n.detail, "NitroSense service did not accept the request.");
    assert!(m.ui_state(100 + ERROR_NOTICE_MS - 1).notice.is_some());
    assert!(m.ui_state(100 + ERROR_NOTICE_MS).notice.is_none(), "transient notice");
    m.on_control_result(&ControlRequest::Fan(FanMode::Max, None), Err("x".into()), 200);
    assert_eq!(m.ui_state(200).notice.unwrap().title, "Could not switch to Max fan mode.");
    assert!(m.on_command(UiCommand::DismissNotice, 201).is_empty());
    assert!(m.ui_state(201).notice.is_none());
}

#[test]
fn calibration_is_learned_in_max_mode_and_persisted() {
    let mut m = connected();
    let mut u = update(ProviderStatus::Connected, all_caps());
    u.snapshot.fan_mode = Some(FanMode::Max);
    assert!(m.on_telemetry(u.clone(), 1000).is_empty());
    let effects = m.on_telemetry(u.clone(), 1000 + CALIBRATION_SETTLE_MS);
    assert_eq!(effects, vec![Effect::PersistSettings]);
    assert_eq!(m.settings.fan.calibration.cpu_max_rpm, Some(7317));
    let mut auto = u;
    auto.snapshot.fan_mode = Some(FanMode::Auto);
    auto.snapshot.cpu_fan = Some(FanTelemetry { rpm: Some(3658), duty_pct: None });
    m.on_telemetry(auto, 20_000);
    let ring = m.ui_state(20_000).cpu_fan.ring_pct.unwrap();
    assert!((ring - 50.0).abs() < 0.1, "measured maximum drives the ring: {ring}");
}

#[test]
fn reapply_happens_once_and_only_when_opted_in() {
    let mut settings = Settings::default();
    settings.startup.last_performance_mode = Some(PerformanceMode::Quiet);
    settings.startup.last_fan_mode = Some(FanMode::Max);
    let mut off = AppModel::new(settings.clone(), false, false);
    assert!(off.on_telemetry(update(ProviderStatus::Connected, all_caps()), 0).is_empty());

    settings.startup.reapply_last_modes = true;
    let mut on = AppModel::new(settings.clone(), false, false);
    assert!(on.on_telemetry(update(ProviderStatus::HelperUnavailable, Capabilities::none()), 0).is_empty());
    let effects = on.on_telemetry(update(ProviderStatus::Connected, all_caps()), 1);
    assert_eq!(
        effects,
        vec![
            Effect::Control(ControlRequest::Performance(PerformanceMode::Quiet)),
            Effect::Control(ControlRequest::Fan(FanMode::Max, None))
        ]
    );
    assert!(on.on_telemetry(update(ProviderStatus::Connected, all_caps()), 2).is_empty(), "only once");

    settings.startup.last_fan_mode = Some(FanMode::Custom);
    settings.startup.last_performance_mode = None;
    let mut custom = AppModel::new(settings.clone(), false, false);
    assert!(
        custom.on_telemetry(update(ProviderStatus::Connected, all_caps()), 0).is_empty(),
        "custom needs a stored profile"
    );

    let profile = FanCurveProfile::default_profile();
    settings.fan.custom_profile = Some(profile.clone());
    let mut with_curve = AppModel::new(settings.clone(), false, false);
    assert_eq!(
        with_curve.on_telemetry(update(ProviderStatus::Connected, all_caps()), 0),
        vec![Effect::Control(ControlRequest::Fan(FanMode::Custom, Some(profile)))]
    );
    settings.startup.last_fan_mode = Some(FanMode::Max);
    let mut max = AppModel::new(settings, false, false);
    assert_eq!(
        max.on_telemetry(update(ProviderStatus::Connected, all_caps()), 0),
        vec![Effect::Control(ControlRequest::Fan(FanMode::Max, None))],
        "only Custom carries the curve"
    );
}

#[test]
fn settings_save_keeps_runtime_owned_fields() {
    let mut m = connected();
    m.settings.fan.custom_profile = Some(FanCurveProfile::default_profile());
    m.settings.startup.last_fan_mode = Some(FanMode::Max);
    let mut incoming = Settings::default();
    incoming.general.hotkey = "Ctrl+Alt+J".into();
    incoming.telemetry.background_interval_ms = 1;
    let effects = m.on_command(UiCommand::SaveSettings { settings: Box::new(incoming) }, 5);
    assert_eq!(effects, vec![Effect::PersistSettings, Effect::ApplySettings, Effect::SendWindowData]);
    assert_eq!(m.settings.general.hotkey, "Ctrl+Alt+J");
    assert_eq!(m.settings.telemetry.background_interval_ms, 1000, "sanitised");
    assert!(m.settings.fan.custom_profile.is_some());
    assert_eq!(m.settings.startup.last_fan_mode, Some(FanMode::Max));
    assert_eq!(m.ui_state(5).notice.unwrap().kind, NoticeKind::Info);
}

#[test]
fn fan_curve_save_validates_and_optionally_applies() {
    let mut m = connected();
    let mut bad = FanCurveProfile::default_profile();
    bad.cpu.points.truncate(1);
    assert_eq!(m.on_command(UiCommand::SaveFanCurve { profile: bad, apply: true }, 0), vec![Effect::SendWindowData]);
    assert_eq!(m.ui_state(0).notice.unwrap().title, "Fan curve not saved.");
    let good = FanCurveProfile::default_profile();
    assert_eq!(
        m.on_command(UiCommand::SaveFanCurve { profile: good.clone(), apply: false }, 1),
        vec![Effect::PersistSettings, Effect::SendWindowData]
    );
    let effects = m.on_command(UiCommand::SaveFanCurve { profile: good.clone(), apply: true }, 2);
    assert_eq!(effects[2], Effect::Control(ControlRequest::Fan(FanMode::Custom, Some(good))));
}

#[test]
fn simple_commands_map_to_effects() {
    let mut m = connected();
    assert_eq!(m.on_command(UiCommand::Ready, 0), vec![Effect::SendWindowData]);
    assert_eq!(m.on_command(UiCommand::ClosePopup, 0), vec![Effect::HidePopup]);
    assert_eq!(m.on_command(UiCommand::DragWindow, 0), vec![Effect::DragPopup]);
    assert_eq!(
        m.on_command(UiCommand::OpenMonitoring, 0),
        vec![Effect::HidePopup, Effect::OpenWindow(Page::Monitoring)]
    );
    assert_eq!(m.on_command(UiCommand::OpenSettings, 0), vec![Effect::HidePopup, Effect::OpenWindow(Page::Settings)]);
    assert_eq!(m.on_command(UiCommand::OpenFanCurve, 0), vec![Effect::HidePopup, Effect::OpenWindow(Page::FanCurve)]);
    assert_eq!(m.on_command(UiCommand::Navigate { page: Page::Settings }, 0), vec![Effect::OpenWindow(Page::Settings)]);
    let aumid = m.settings.integration.nitrosense_app_id.clone();
    assert_eq!(m.on_command(UiCommand::OpenNitroSense, 0), vec![Effect::HidePopup, Effect::LaunchNitroSense(aumid)]);
    assert_eq!(m.on_command(UiCommand::SetAutostart { enabled: true }, 0), vec![Effect::SetAutostart(true)]);
    assert_eq!(m.on_command(UiCommand::ExportDiagnostics, 0), vec![Effect::ExportDiagnostics]);
    assert_eq!(m.on_command(UiCommand::InstallHelper, 0), vec![Effect::InstallHelper]);
    assert_eq!(m.on_command(UiCommand::PinTrayIcons, 0), vec![Effect::PinTrayIcons]);
    assert_eq!(m.on_command(UiCommand::SetFanMode { mode: FanMode::Max }, 0).len(), 1);
    let mut n = connected();
    assert_eq!(n.on_command(UiCommand::SetPerformanceMode { mode: PerformanceMode::Quiet }, 0).len(), 1);
}

#[test]
fn menu_actions_map_to_effects() {
    let mut m = connected();
    assert_eq!(m.on_menu(MenuAction::OpenPopup, 0), vec![Effect::ShowPopup]);
    assert_eq!(m.on_menu(MenuAction::OpenMonitoring, 0), vec![Effect::OpenWindow(Page::Monitoring)]);
    assert_eq!(m.on_menu(MenuAction::OpenSettings, 0), vec![Effect::OpenWindow(Page::Settings)]);
    assert_eq!(m.on_menu(MenuAction::Exit, 0), vec![Effect::Exit]);
    assert!(matches!(m.on_menu(MenuAction::OpenNitroSense, 0)[0], Effect::LaunchNitroSense(_)));
    assert!(!m.autostart(), "constructed with autostart off");
    assert_eq!(m.on_menu(MenuAction::ToggleAutostart, 0), vec![Effect::SetAutostart(true)]);
    m.set_autostart(true);
    assert!(m.autostart());
    assert_eq!(m.on_menu(MenuAction::ToggleAutostart, 0), vec![Effect::SetAutostart(false)]);
    assert_eq!(m.on_menu(MenuAction::SetPerformance(PerformanceMode::Quiet), 0).len(), 1);
    let mut n = connected();
    assert_eq!(n.on_menu(MenuAction::SetFan(FanMode::Max), 0).len(), 1);
}

#[test]
fn theme_resolution() {
    let mut m = AppModel::new(Settings::default(), false, true);
    assert_eq!(m.resolved_theme(), "dark");
    m.settings.appearance.theme = ThemeChoice::Light;
    assert_eq!(m.resolved_theme(), "light");
    m.settings.appearance.theme = ThemeChoice::MatchWindows;
    assert_eq!(m.resolved_theme(), "light");
    m.set_light_system_theme(false);
    assert_eq!(m.resolved_theme(), "dark");
}

#[test]
fn unexposed_firmware_profile_is_named() {
    let mut m = AppModel::new(Settings::default(), false, false);
    let mut u = update(ProviderStatus::Connected, all_caps());
    u.snapshot.performance_mode = None;
    u.snapshot.performance_raw = Some(5);
    m.on_telemetry(u, 0);
    assert_eq!(m.ui_state(0).performance_other.as_deref(), Some("Turbo"));
}

#[test]
fn connection_texts_cover_every_status() {
    for s in [
        ProviderStatus::Connected,
        ProviderStatus::Simulated,
        ProviderStatus::AccessDenied,
        ProviderStatus::NotPresent,
        ProviderStatus::HelperUnavailable,
    ] {
        assert!(connection_text(s).ends_with('.'));
    }
    assert!(connection_text(ProviderStatus::Simulated).contains("Simulated"));
}

#[test]
fn info_notices_are_shorter_lived() {
    let mut m = connected();
    m.notify(NoticeKind::Info, "t", "d", 0);
    assert!(m.ui_state(INFO_NOTICE_MS - 1).notice.is_some());
    assert!(m.ui_state(INFO_NOTICE_MS).notice.is_none());
}
