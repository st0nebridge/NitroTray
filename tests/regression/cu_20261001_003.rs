//! @module cu_20261001_003
//! @description Regression test (tray app hung at start-up with a blank popup): creating a WebView2 controller
//! while another of our webviews is on screen never completed, leaving the event loop inside wry's wait
//! (no tray menu, no hover, `--exit` ignored). The runtime must create the flyout during start-up before
//! the popup is first shown, and later (re)create webviews only while none is on screen.
//! The live check is `tests/live/webviews.ps1`.
const RUNTIME: &str = include_str!("../../src/app/runtime.rs");

fn body_of<'a>(src: &'a str, signature: &str) -> &'a str {
    let start = src.find(signature).unwrap_or_else(|| panic!("missing {signature}"));
    let rest = &src[start..];
    let end = rest[1..].find("\n    fn ").or_else(|| rest[1..].find("\n    pub(crate) fn ")).unwrap_or(rest.len() - 1);
    &rest[..end + 1]
}

#[test]
fn startup_creates_the_flyout_before_showing_the_popup() {
    let init = body_of(RUNTIME, "fn init(");
    let popup = init.find("self.build_popup(target)").expect("init builds the popup");
    let flyout = init.find("self.sync_flyout_surface(target)").expect("init builds the flyout");
    let show = init.find("self.show_popup(None)").expect("init may show the popup");
    assert!(popup < flyout && flyout < show, "create both webviews, then show the popup");
}

#[test]
fn later_webview_creation_waits_until_none_is_on_screen() {
    let handle = body_of(RUNTIME, "pub(crate) fn handle(");
    let gate = handle.find("if self.surfaces_hidden()").expect("creation is gated");
    for call in ["self.build_popup(target)", "self.sync_flyout_surface(target)"] {
        let at = handle.find(call).unwrap_or_else(|| panic!("{call} runs in the loop"));
        assert!(at > gate, "{call} must sit inside the surfaces_hidden() gate");
    }
    let hidden = body_of(RUNTIME, "pub(crate) fn surfaces_hidden(");
    for flag in ["!self.popup_visible", "!self.full_visible", "self.flyout_kind.is_none()"] {
        assert!(hidden.contains(flag), "surfaces_hidden checks {flag}");
    }
}
