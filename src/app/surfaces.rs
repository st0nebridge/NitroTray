//! @module app::surfaces
//! @description Popup, hover flyout and full-window management: creation, placement beside the tray,
//! show/hide/toggle with focus-loss dismissal, and Windows 11 chrome.
//!
//! @input  The runtime's surfaces and settings; tray anchor rectangles.
//! @output Window visibility/placement side effects; visibility reported to the telemetry worker.
//! @dependencies tao, windows (WindowsAndMessaging), app::{events, runtime}, tray::positioning, ui::{bridge, flyout, webview}
use std::time::{Duration, Instant};

use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event_loop::EventLoopWindowTarget;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};

use crate::app::events::{SurfaceKind, UserEvent};
use crate::app::runtime::Runtime;
use crate::app::worker::WorkerMsg;
use crate::tray::positioning::{self, Rect, MARGIN_DIP};
use crate::ui::bridge::{self, NoticeKind};
use crate::ui::flyout::{self, FlyoutKind, FLYOUT_SIZE};
use crate::ui::webview::{self, Surface, POPUP_SIZE};

/// How long after a focus-loss hide a tray click counts as "close" rather than "reopen".
pub const TOGGLE_GRACE: Duration = Duration::from_millis(350);

fn cursor() -> (i32, i32) {
    let mut p = POINT::default();
    // SAFETY: writes the cursor position into a local.
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

/// Sizes a surface to `size_dip` on the monitor under the anchor and places it beside the tray.
fn place(surface: &Surface, size_dip: (f64, f64), anchor: Option<Rect>) {
    let (x, y) = anchor.map_or_else(cursor, |a| a.center());
    let Some(geo) = positioning::monitor_at(x, y) else { return };
    let edge = positioning::taskbar_edge(&geo);
    let size = ((size_dip.0 * geo.scale).round() as i32, (size_dip.1 * geo.scale).round() as i32);
    let margin = (MARGIN_DIP * geo.scale).round() as i32;
    let (px, py) = positioning::popup_position(size, anchor, geo.work, edge, margin);
    surface.window.set_outer_position(PhysicalPosition::new(px, py));
    surface.window.set_inner_size(PhysicalSize::new(size.0 as u32, size.1 as u32));
}

impl Runtime {
    fn page_sink(&self, kind: SurfaceKind) -> impl Fn(String) + 'static {
        let proxy = self.proxy.clone();
        move |message| {
            let _ = proxy.send_event(UserEvent::Ui(kind, message));
        }
    }

    pub(crate) fn build_popup(&mut self, target: &EventLoopWindowTarget<UserEvent>) {
        let transparent = self.model.settings.appearance.transparency;
        self.popup = None;
        let sink = self.page_sink(SurfaceKind::Popup);
        match webview::build_popup(target, &mut self.web_context, sink, transparent) {
            Ok(surface) => {
                surface.set_hidden(true);
                self.popup_transparent = transparent;
                self.popup = Some(surface);
                self.apply_chrome();
            }
            Err(e) => self.model.notify(NoticeKind::Error, "Popup could not be created.", e, self.now()),
        }
        self.popup_visible = false;
    }

    pub(crate) fn ensure_full(&mut self, target: Option<&EventLoopWindowTarget<UserEvent>>) -> bool {
        if self.full.is_some() {
            return true;
        }
        let Some(target) = target else { return false };
        let sink = self.page_sink(SurfaceKind::Full);
        match webview::build_full(target, &mut self.web_context, sink) {
            Ok(s) => {
                self.full = Some(s);
                self.apply_chrome();
                true
            }
            Err(e) => {
                self.model.notify(NoticeKind::Error, "Window could not be created.", e, self.now());
                false
            }
        }
    }

    pub(crate) fn apply_chrome(&self) {
        let dark = self.model.resolved_theme() == "dark";
        if let Some(p) = &self.popup {
            webview::apply_chrome(p.hwnd(), dark, true, self.popup_transparent);
        }
        if let Some(f) = &self.flyout {
            webview::apply_chrome(f.hwnd(), dark, true, false);
        }
        if let Some(f) = &self.full {
            webview::apply_chrome(f.hwnd(), dark, false, false);
        }
    }

    pub(crate) fn update_visibility(&self) {
        let _ = self.worker_tx.send(WorkerMsg::Visibility(self.popup_visible || self.full_visible));
    }

    pub(crate) fn show_popup(&mut self, anchor: Option<Rect>) {
        self.hover.dismissed();
        self.hide_flyout();
        if let Some(popup) = &self.popup {
            place(popup, POPUP_SIZE, anchor);
        }
        self.popup_visible = true;
        self.push_state();
        if let Some(popup) = &self.popup {
            popup.set_hidden(false);
            popup.window.set_visible(true);
            popup.window.set_focus();
            let _ = popup.webview.focus();
        }
        self.update_visibility();
    }

    pub(crate) fn hide_popup(&mut self) {
        if let Some(popup) = &self.popup {
            popup.window.set_visible(false);
            // Lets WebView2 stop rendering and trim memory while hidden.
            popup.set_hidden(true);
        }
        if self.popup_visible {
            self.popup_hidden_at = Some(Instant::now());
        }
        self.popup_visible = false;
        self.deactivate_at = None;
        self.update_visibility();
    }

    pub(crate) fn toggle_popup(&mut self, anchor: Option<Rect>) {
        if self.popup_visible {
            self.hide_popup();
        } else if !self.popup_hidden_at.is_some_and(|t| t.elapsed() < TOGGLE_GRACE) {
            self.show_popup(anchor);
        }
    }

    pub(crate) fn show_full(&mut self, page: bridge::Page, target: Option<&EventLoopWindowTarget<UserEvent>>) {
        if !self.ensure_full(target) {
            self.pending_full_page = Some(page);
            return;
        }
        if let Some(full) = &self.full {
            full.eval(&bridge::script_call("navigate", &page));
            full.set_hidden(false);
            full.window.set_visible(true);
            full.window.set_minimized(false);
            full.window.set_focus();
        }
        self.full_visible = true;
        self.update_visibility();
        self.send_window_data();
        self.push_state();
    }

    pub(crate) fn hide_full(&mut self) {
        if let Some(f) = &self.full {
            f.window.set_visible(false);
            f.set_hidden(true);
        }
        self.full_visible = false;
        self.update_visibility();
    }

    /// The flyout exists only while a graph icon is shown and hover details are on.
    pub(crate) fn sync_flyout_surface(&mut self, target: &EventLoopWindowTarget<UserEvent>) {
        let t = &self.model.settings.tray_icons;
        let wanted = t.hover_details && (t.cpu.enabled || t.gpu.enabled);
        if !wanted {
            self.hover.dismissed();
            self.hide_flyout();
            self.flyout = None;
            return;
        }
        if self.flyout.is_some() {
            return;
        }
        let sink = self.page_sink(SurfaceKind::Flyout);
        match webview::build_flyout(target, &mut self.web_context, sink) {
            Ok(surface) => {
                surface.set_hidden(true);
                self.flyout = Some(surface);
                self.apply_chrome();
            }
            Err(e) => self.model.notify(NoticeKind::Error, "Hover details could not be created.", e, self.now()),
        }
    }

    /// Shows the flyout for `kind` above its icon without taking focus.
    pub(crate) fn show_flyout(&mut self, kind: FlyoutKind, anchor: Rect) {
        let Some(f) = &self.flyout else { return };
        place(f, FLYOUT_SIZE, Some(anchor));
        self.flyout_kind = Some(kind);
        f.set_hidden(false);
        self.push_flyout();
        if let Some(f) = &self.flyout {
            webview::show_inactive(f.hwnd());
        }
    }

    pub(crate) fn hide_flyout(&mut self) {
        if let (Some(f), Some(_)) = (&self.flyout, self.flyout_kind) {
            webview::hide_window(f.hwnd());
            f.set_hidden(true);
        }
        self.flyout_kind = None;
    }

    /// Sends the visible flyout its processor's history and current values.
    pub(crate) fn push_flyout(&self) {
        if let (Some(f), Some(kind)) = (&self.flyout, self.flyout_kind) {
            let state = flyout::state(kind, &self.history, &self.model.settings, self.model.prefs());
            f.eval(&bridge::script_call("onFlyout", &state));
        }
    }

    /// Hides the popup when another window took the foreground.
    pub(crate) fn check_deactivation(&mut self) {
        let Some(popup) = &self.popup else { return };
        // SAFETY: plain query of the foreground window.
        let fg = unsafe { GetForegroundWindow() };
        if self.popup_visible && self.model.settings.general.close_on_focus_loss && fg != popup.hwnd() {
            self.hide_popup();
        }
    }
}
