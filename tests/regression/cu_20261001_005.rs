//! @module cu_20261001_005
//! @description Regression test (window chrome): the full window carries the NitroTray icon for its title bar and its
//! taskbar/Alt-Tab button (it showed the generic application icon), and the settings save bar keeps the
//! "Unsaved changes" status level with the button label (the paragraph margin of `.muted` lifted it 8 DIP).
const WEBVIEW: &str = include_str!("../../src/ui/webview.rs");
const WINDOW_CSS: &str = include_str!("../../src/ui/web/css/window.css");

#[test]
fn full_window_sets_both_app_icons() {
    let start = WEBVIEW.find("pub fn build_full").expect("build_full");
    let body = &WEBVIEW[start..];
    let body = &body[..body.find("\n}\n").unwrap()];
    assert!(body.contains(".with_window_icon(app_icon(SM_CXSMICON))"), "title bar icon");
    assert!(body.contains(".with_taskbar_icon(app_icon(SM_CXICON))"), "taskbar and Alt-Tab icon");
    assert!(WEBVIEW.contains("crate::tray::app_icon::render(size)"), "the icon is NitroTray's own");
}

#[test]
fn save_bar_status_sits_on_the_button_baseline() {
    let bar = WINDOW_CSS.find("\n.save-bar {").map(|i| &WINDOW_CSS[i..]).expect(".save-bar rule");
    assert!(bar[..bar.find('}').unwrap()].contains("align-items: baseline"));
    assert!(WINDOW_CSS.contains("\n.save-bar .muted { margin: 0; }"), "no paragraph margin in the bar");
}
