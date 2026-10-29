//! @module clicks_test
//! @description Tray icon gestures: a single left click opens a graph icon's own popup (the app icon's
//! opens the main popup), a double click opens the main popup, and the release that ends a double click
//! does nothing.
use nitrotray::tray::clicks::{ClickAction, Clicks, Target};
use nitrotray::tray::positioning::Rect;
use nitrotray::ui::flyout::FlyoutKind::{Cpu, Gpu};

const CPU_ICON: Rect = Rect { left: 1500, top: 1040, right: 1524, bottom: 1064 };
const GPU_ICON: Rect = Rect { left: 1476, top: 1040, right: 1500, bottom: 1064 };
const APP_ICON: Rect = Rect { left: 1524, top: 1040, right: 1548, bottom: 1064 };

#[test]
fn a_single_click_on_a_graph_icon_toggles_that_icons_popup() {
    let mut c = Clicks::new();
    c.down();
    assert_eq!(c.up(Target::Graph(Cpu), CPU_ICON), Some(ClickAction::ToggleDetails(Cpu, CPU_ICON)));
    c.down();
    assert_eq!(c.up(Target::Graph(Gpu), GPU_ICON), Some(ClickAction::ToggleDetails(Gpu, GPU_ICON)));
}

#[test]
fn a_single_click_on_the_app_icon_toggles_the_main_popup() {
    let mut c = Clicks::new();
    c.down();
    assert_eq!(c.up(Target::App, APP_ICON), Some(ClickAction::ToggleMain(APP_ICON)));
}

#[test]
fn a_double_click_opens_the_main_popup_from_any_icon() {
    let mut c = Clicks::new();
    assert_eq!(c.double(CPU_ICON), ClickAction::OpenMain(CPU_ICON));
    assert_eq!(c.double(APP_ICON), ClickAction::OpenMain(APP_ICON));
}

#[test]
fn the_release_that_ends_a_double_click_does_nothing() {
    // Windows reports a double click as press, release, double-click, release.
    let mut c = Clicks::new();
    c.down();
    assert_eq!(c.up(Target::Graph(Cpu), CPU_ICON), Some(ClickAction::ToggleDetails(Cpu, CPU_ICON)));
    assert_eq!(c.double(CPU_ICON), ClickAction::OpenMain(CPU_ICON));
    assert_eq!(c.up(Target::Graph(Cpu), CPU_ICON), None, "the second release must not toggle anything");
}

#[test]
fn the_click_after_a_double_click_works_again() {
    let mut c = Clicks::new();
    c.double(CPU_ICON);
    c.up(Target::Graph(Cpu), CPU_ICON);
    c.down();
    assert_eq!(c.up(Target::Graph(Cpu), CPU_ICON), Some(ClickAction::ToggleDetails(Cpu, CPU_ICON)));
}

#[test]
fn a_new_press_clears_a_release_that_never_arrived() {
    // The second release goes elsewhere when the button is let go away from the icon.
    let mut c = Clicks::new();
    c.double(CPU_ICON);
    c.down();
    assert_eq!(c.up(Target::Graph(Cpu), CPU_ICON), Some(ClickAction::ToggleDetails(Cpu, CPU_ICON)));
}

#[test]
fn a_release_is_swallowed_once() {
    let mut c = Clicks::new();
    c.double(CPU_ICON);
    assert_eq!(c.up(Target::Graph(Cpu), CPU_ICON), None);
    assert_eq!(c.up(Target::Graph(Cpu), CPU_ICON), Some(ClickAction::ToggleDetails(Cpu, CPU_ICON)));
}

#[test]
fn icon_ids_map_to_targets() {
    assert_eq!(Target::from_icon_id("app"), Some(Target::App));
    assert_eq!(Target::from_icon_id("cpu"), Some(Target::Graph(Cpu)));
    assert_eq!(Target::from_icon_id("gpu"), Some(Target::Graph(Gpu)));
    assert_eq!(Target::from_icon_id("fan"), None);
    assert_eq!(Target::from_icon_id(""), None);
}
