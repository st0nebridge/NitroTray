//! @module app::events
//! @description Event-loop vocabulary: everything the tray host reacts to, plus launch options.
//!
//! @output `UserEvent` (sent through the tao event-loop proxy), `SurfaceKind`, `Options`.
//! @dependencies tray-icon, global-hotkey, app::{model, worker}
use global_hotkey::GlobalHotKeyEvent;
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;

use crate::app::model::ControlRequest;
use crate::app::worker::TelemetryDelivery;

/// Which webview a page message came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    Popup,
    Flyout,
    Full,
}

pub enum UserEvent {
    Telemetry(Box<TelemetryDelivery>),
    ControlDone(ControlRequest, Result<String, String>),
    Tray(TrayIconEvent),
    Menu(MenuEvent),
    Hotkey(GlobalHotKeyEvent),
    Ui(SurfaceKind, String),
    ShowRequested,
    ExitRequested,
    CapabilitiesRecheck,
}

pub struct Options {
    /// Launched by the HKCU Run entry.
    pub autostart_launch: bool,
    /// `--exit`: ask a running instance to quit, then return.
    pub exit_running: bool,
}
