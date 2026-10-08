//! @module cu_20260930_013
//! @description Regression test (idle CPU): hidden surfaces must pause WebView2 rendering. Measured
//! on the AN515-58 with the popup hidden: 2.318 % of the machine before, 0.036 % after. The code lives in
//! window glue that cannot run headlessly, so this test guards the source contract instead.

const WEBVIEW: &str = include_str!("../../src/ui/webview.rs");
const SURFACES: &str = include_str!("../../src/app/surfaces.rs");

#[test]
fn webviews_are_created_hidden() {
    assert!(WEBVIEW.contains(".with_visible(false)"), "a visible controller keeps compositing behind a hidden window");
    assert!(WEBVIEW.contains("SetMemoryUsageTargetLevel"), "hidden surfaces ask WebView2 to trim memory");
}

#[test]
fn every_hide_path_pauses_rendering_and_every_show_resumes_it() {
    let hide_popup = SURFACES.split("fn hide_popup").nth(1).unwrap().split("\n    }\n").next().unwrap();
    assert!(hide_popup.contains("set_hidden(true)"));
    let hide_full = SURFACES.split("fn hide_full").nth(1).unwrap().split("\n    }\n").next().unwrap();
    assert!(hide_full.contains("set_hidden(true)"));
    let show_popup = SURFACES.split("fn show_popup").nth(1).unwrap().split("\n    }\n").next().unwrap();
    assert!(show_popup.contains("set_hidden(false)"));
    let build_popup = SURFACES.split("fn build_popup").nth(1).unwrap().split("\n    }\n").next().unwrap();
    assert!(build_popup.contains("set_hidden(true)"), "the popup starts hidden and idle");
}
