//! @module cu_20261029_001
//! @description Regression test (tray icon gestures): the CPU and GPU icons no longer open a flyout on
//! hover, so the settings file loses `hover_details` (an old file that still has it loads unchanged);
//! one real double click, as Windows delivers it, opens the icon's popup and then the main popup and
//! leaves the main popup open.
use nitrotray::config::settings::Settings;
use nitrotray::tray::clicks::{ClickAction, Clicks, Target};
use nitrotray::tray::positioning::Rect;
use nitrotray::ui::flyout::FlyoutKind;

#[test]
fn hover_details_is_no_longer_a_setting() {
    let json = serde_json::to_string(&Settings::default()).unwrap();
    assert!(!json.contains("hover_details"), "the setting has no effect any more: {json}");
}

#[test]
fn a_config_that_still_has_hover_details_loads() {
    let old: Settings =
        serde_json::from_str(r#"{"tray_icons":{"hover_details":false,"show_app_icon":false}}"#).unwrap();
    assert!(!old.tray_icons.show_app_icon, "the other tray icon settings survive");
    assert_eq!(old.tray_icons.cpu, Settings::default().tray_icons.cpu);
}

#[test]
fn a_real_double_click_ends_with_the_main_popup_open() {
    let icon = Rect::new(1500, 1040, 1524, 1064);
    let mut c = Clicks::new();
    let mut actions = Vec::new();
    // press, release, double-click, release: what the notification area sends for one double click.
    c.down();
    actions.extend(c.up(Target::Graph(FlyoutKind::Cpu), icon));
    actions.push(c.double(icon));
    actions.extend(c.up(Target::Graph(FlyoutKind::Cpu), icon));
    assert_eq!(actions, vec![ClickAction::ToggleDetails(FlyoutKind::Cpu, icon), ClickAction::OpenMain(icon)]);
}
