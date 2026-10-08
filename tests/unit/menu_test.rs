//! @module menu_test
//! @description Tray menu ids ↔ actions, and check/enabled state.
use nitrotray::acer::Capabilities;
use nitrotray::controls::{FanMode, PerformanceMode};
use nitrotray::tray::menu::{action_for, model, MenuAction};

use crate::support::all_caps;

#[test]
fn maps_every_id() {
    assert_eq!(action_for("open"), Some(MenuAction::OpenPopup));
    assert_eq!(action_for("nitrosense"), Some(MenuAction::OpenNitroSense));
    assert_eq!(action_for("monitor"), Some(MenuAction::OpenMonitoring));
    assert_eq!(action_for("settings"), Some(MenuAction::OpenSettings));
    assert_eq!(action_for("autostart"), Some(MenuAction::ToggleAutostart));
    assert_eq!(action_for("exit"), Some(MenuAction::Exit));
    assert_eq!(action_for("perf:quiet"), Some(MenuAction::SetPerformance(PerformanceMode::Quiet)));
    assert_eq!(action_for("fan:max"), Some(MenuAction::SetFan(FanMode::Max)));
    for bad in ["", "perf:turbo", "fan:", "delete", "fan:auto:x"] {
        assert_eq!(action_for(bad), None, "{bad}");
    }
}

#[test]
fn model_checks_current_modes_and_gates_by_capability() {
    let mut caps = all_caps();
    caps.custom_fan_mode = false;
    let m = model(Some(FanMode::Auto), Some(PerformanceMode::Performance), &caps, true);
    assert!(m.autostart);
    assert_eq!(m.fan.iter().map(|i| i.checked).collect::<Vec<_>>(), vec![true, false, false]);
    assert_eq!(m.fan.iter().map(|i| i.enabled).collect::<Vec<_>>(), vec![true, true, false]);
    assert_eq!(m.performance.iter().map(|i| i.checked).collect::<Vec<_>>(), vec![false, false, true]);
    assert_eq!(m.performance[2].id, "perf:performance");
    assert_eq!(m.fan[1].label, "Max");
    let none = model(None, None, &Capabilities::none(), false);
    assert!(none.fan.iter().chain(&none.performance).all(|i| !i.checked && !i.enabled));
}
