//! @module app::runtime
//! @description Tray host event dispatch: routes tray, menu, hotkey, window, page and worker events to the
//! app model and pushes state back out. The popup and the hover flyout stay instantiated-but-hidden.
//!
//! @input  tao events and `UserEvent`s.
//! @output Model updates, pushed UI state, effects executed via `app::effects`, windows via `app::surfaces`.
//! @dependencies tao, wry, global-hotkey, tray-icon, app::{events, model, worker, tray_host, hotkeys}, ui, tray::{hover, positioning}
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyManager, HotKeyState};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopProxy, EventLoopWindowTarget};
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use wry::WebContext;

use crate::app::events::{Options, SurfaceKind, UserEvent};
use crate::app::model::{AppModel, ControlRequest, Effect};
use crate::app::tray_host::TrayHost;
use crate::app::worker::{TelemetryConfig, TelemetryDelivery, WorkerMsg};
use crate::config::{ConfigStore, Settings};
use crate::ipc::protocol::CapabilitiesReport;
use crate::platform::windows::{apps_use_light_theme, Autostart};
use crate::telemetry::history::MetricsHistory;
use crate::telemetry::ProviderErrorEntry;
use crate::tray::hover::{Hover, HoverAction};
use crate::tray::menu;
use crate::tray::positioning::Rect;
use crate::ui::bridge::{self, NoticeKind};
use crate::ui::flyout::FlyoutKind;
use crate::ui::webview::Surface;

const DEACTIVATE_DELAY: Duration = Duration::from_millis(80);
pub const MONITOR_HISTORY: usize = 300;

pub(crate) struct Runtime {
    pub(crate) model: AppModel,
    pub(crate) store: ConfigStore,
    pub(crate) proxy: EventLoopProxy<UserEvent>,
    pub(crate) tray: Option<TrayHost>,
    pub(crate) popup: Option<Surface>,
    pub(crate) flyout: Option<Surface>,
    /// Which processor the visible flyout shows; `None` while hidden.
    pub(crate) flyout_kind: Option<FlyoutKind>,
    pub(crate) hover: Hover,
    /// Create or drop the flyout to match settings at the next loop turn (needs the event-loop target).
    pub(crate) sync_flyout: bool,
    pub(crate) full: Option<Surface>,
    pub(crate) web_context: WebContext,
    pub(crate) worker_tx: Sender<WorkerMsg>,
    pub(crate) control_tx: Sender<ControlRequest>,
    pub(crate) hotkeys: Option<GlobalHotKeyManager>,
    pub(crate) hotkey: Option<HotKey>,
    pub(crate) history: MetricsHistory,
    pub(crate) errors: Vec<ProviderErrorEntry>,
    pub(crate) report: Option<CapabilitiesReport>,
    pub(crate) started: Instant,
    pub(crate) popup_visible: bool,
    pub(crate) full_visible: bool,
    pub(crate) popup_hidden_at: Option<Instant>,
    pub(crate) deactivate_at: Option<Instant>,
    pub(crate) popup_transparent: bool,
    pub(crate) rebuild_popup: bool,
    pub(crate) pending_full_page: Option<bridge::Page>,
    pub(crate) exit: bool,
    opts: Options,
    startup_warning: Option<String>,
}

impl Runtime {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        settings: Settings,
        store: ConfigStore,
        proxy: EventLoopProxy<UserEvent>,
        worker_tx: Sender<WorkerMsg>,
        control_tx: Sender<ControlRequest>,
        started: Instant,
        opts: Options,
        startup_warning: Option<String>,
    ) -> Self {
        let web_dir = store.dir().join("WebView2");
        Self {
            model: AppModel::new(settings, Autostart::default().is_enabled(), apps_use_light_theme()),
            store,
            proxy,
            tray: None,
            popup: None,
            flyout: None,
            flyout_kind: None,
            hover: Hover::new(),
            sync_flyout: true,
            full: None,
            web_context: WebContext::new(Some(web_dir)),
            worker_tx,
            control_tx,
            hotkeys: GlobalHotKeyManager::new().ok(),
            hotkey: None,
            history: MetricsHistory::new(MONITOR_HISTORY),
            errors: Vec::new(),
            report: None,
            started,
            popup_visible: false,
            full_visible: false,
            popup_hidden_at: None,
            deactivate_at: None,
            popup_transparent: false,
            rebuild_popup: false,
            pending_full_page: None,
            exit: false,
            opts,
            startup_warning,
        }
    }

    pub(crate) fn now(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    pub(crate) fn handle(
        &mut self,
        event: Event<'_, UserEvent>,
        target: &EventLoopWindowTarget<UserEvent>,
        flow: &mut ControlFlow,
    ) {
        match event {
            Event::NewEvents(StartCause::Init) => self.init(target),
            Event::UserEvent(e) => self.on_user_event(e, target),
            Event::WindowEvent { window_id, event, .. } => self.on_window_event(window_id, event),
            Event::MainEventsCleared => {
                if self.deactivate_at.is_some_and(|t| Instant::now() >= t) {
                    self.deactivate_at = None;
                    self.check_deactivation();
                }
                // Creating a WebView2 controller while another of ours is on screen never completed
                // (the loop hung inside wry's wait; CU-20261001-003), so (re)creation waits until none is.
                if self.surfaces_hidden() {
                    if self.rebuild_popup {
                        self.rebuild_popup = false;
                        self.build_popup(target);
                    }
                    if self.sync_flyout {
                        self.sync_flyout = false;
                        self.sync_flyout_surface(target);
                    }
                }
                let action = self.hover.tick(self.now());
                self.apply_hover(action);
            }
            _ => {}
        }
        let hover_at = self.hover.deadline().map(|ms| self.started + Duration::from_millis(ms));
        *flow = if self.exit {
            ControlFlow::Exit
        } else if let Some(t) = [self.deactivate_at, hover_at].into_iter().flatten().min() {
            ControlFlow::WaitUntil(t)
        } else {
            ControlFlow::Wait
        };
    }

    /// No webview of ours is on screen: the only time a new one may be created.
    pub(crate) fn surfaces_hidden(&self) -> bool {
        !self.popup_visible && !self.full_visible && self.flyout_kind.is_none()
    }

    fn apply_hover(&mut self, action: Option<HoverAction>) {
        match action {
            Some(HoverAction::Show(kind, anchor)) if !self.popup_visible && self.flyout.is_some() => {
                self.show_flyout(kind, anchor)
            }
            Some(HoverAction::Show(..)) => self.hover.dismissed(),
            Some(HoverAction::Hide) => self.hide_flyout(),
            None => {}
        }
    }

    fn on_tray(&mut self, event: TrayIconEvent) {
        let now = self.now();
        let flyout = |id: &tray_icon::TrayIconId| FlyoutKind::from_icon_id(&id.0);
        match event {
            TrayIconEvent::Enter { id, rect, .. } => {
                if let Some(kind) = flyout(&id) {
                    let action = self.hover.enter(kind, icon_rect(&rect), now);
                    self.apply_hover(action);
                }
            }
            TrayIconEvent::Leave { id, .. } => {
                if let Some(kind) = flyout(&id) {
                    let action = self.hover.leave(kind, now);
                    self.apply_hover(action);
                }
            }
            TrayIconEvent::Click { rect, button, button_state, .. } => {
                let action = self.hover.click();
                self.apply_hover(action);
                if button == MouseButton::Left
                    && button_state == MouseButtonState::Up
                    && self.model.settings.general.open_popup_on_tray_click
                {
                    self.toggle_popup(Some(icon_rect(&rect)));
                }
            }
            TrayIconEvent::DoubleClick { .. } => {
                let action = self.hover.click();
                self.apply_hover(action);
            }
            _ => {}
        }
    }

    fn init(&mut self, target: &EventLoopWindowTarget<UserEvent>) {
        match TrayHost::new() {
            Ok(mut t) => {
                t.sync(&self.model.settings);
                self.tray = Some(t);
            }
            Err(e) => self.model.notify(NoticeKind::Error, "Tray icon could not be created.", e, self.now()),
        }
        self.build_popup(target);
        // Before the popup is first shown: see `surfaces_hidden`.
        self.sync_flyout = false;
        self.sync_flyout_surface(target);
        self.register_hotkey();
        let _ = self.worker_tx.send(WorkerMsg::Config(TelemetryConfig::from_settings(&self.model.settings)));
        if let Some(w) = self.startup_warning.take() {
            self.model.notify(NoticeKind::Error, "Settings reset.", w, self.now());
        }
        if !self.opts.autostart_launch || !self.model.settings.general.start_minimised {
            self.show_popup(None);
        }
        self.push_state();
    }

    pub(crate) fn register_hotkey(&mut self) {
        let Some(manager) = &self.hotkeys else { return };
        if let Some(old) = self.hotkey.take() {
            let _ = manager.unregister(old);
        }
        let g = &self.model.settings.general;
        if !g.hotkey_enabled {
            return;
        }
        let spec = crate::config::hotkey::HotkeySpec::parse(&g.hotkey);
        match spec.and_then(|s| crate::app::hotkeys::to_hotkey(&s)) {
            Some(hk) if manager.register(hk).is_ok() => self.hotkey = Some(hk),
            _ => {
                let msg = format!("{} is unavailable or already used by another app.", g.hotkey);
                self.model.notify(NoticeKind::Error, "Shortcut not registered.", msg, self.now());
            }
        }
    }

    /// Pushes UI state to visible pages and syncs the tray menu.
    pub(crate) fn push_state(&mut self) {
        let state = self.model.ui_state(self.now());
        let js = bridge::script_call("onState", &state);
        if let (Some(p), true) = (&self.popup, self.popup_visible) {
            p.eval(&js);
        }
        if let (Some(f), true) = (&self.full, self.full_visible) {
            f.eval(&js);
        }
        let m = menu::model(state.fan_mode, state.performance_mode, &state.capabilities, self.model.autostart());
        if let Some(t) = &self.tray {
            t.update_menu(&m);
        }
    }

    fn on_window_event(&mut self, id: tao::window::WindowId, event: WindowEvent<'_>) {
        let is_popup = self.popup.as_ref().is_some_and(|p| p.window.id() == id);
        let is_full = self.full.as_ref().is_some_and(|f| f.window.id() == id);
        match event {
            WindowEvent::Focused(false) if is_popup => self.deactivate_at = Some(Instant::now() + DEACTIVATE_DELAY),
            WindowEvent::Focused(true) if is_popup => self.deactivate_at = None,
            WindowEvent::CloseRequested if is_popup => self.hide_popup(),
            WindowEvent::CloseRequested if is_full => self.hide_full(),
            WindowEvent::ThemeChanged(_) => {
                self.model.set_light_system_theme(apps_use_light_theme());
                self.apply_chrome();
                self.push_state();
                self.push_flyout();
            }
            _ => {}
        }
    }

    fn on_user_event(&mut self, e: UserEvent, target: &EventLoopWindowTarget<UserEvent>) {
        let now = self.now();
        match e {
            UserEvent::Telemetry(d) => self.on_telemetry(*d),
            UserEvent::ControlDone(req, result) => {
                let effects = self.model.on_control_result(&req, result, now);
                self.perform(effects, Some(target));
            }
            UserEvent::Tray(event) => self.on_tray(event),
            UserEvent::Menu(m) => {
                if let Some(action) = menu::action_for(&m.id.0) {
                    let effects = self.model.on_menu(action, now);
                    self.perform(effects, Some(target));
                }
            }
            UserEvent::Hotkey(h) => {
                if h.state == HotKeyState::Pressed && self.hotkey.is_some_and(|k| k.id() == h.id) {
                    self.toggle_popup(None);
                }
            }
            UserEvent::Ui(kind, json) => self.on_page_message(kind, &json, target),
            UserEvent::ShowRequested => self.show_popup(None),
            UserEvent::ExitRequested => self.perform(vec![Effect::Exit], Some(target)),
            UserEvent::CapabilitiesRecheck => {
                let _ = self.worker_tx.send(WorkerMsg::RefreshCapabilities);
            }
        }
    }

    fn on_page_message(&mut self, kind: SurfaceKind, json: &str, target: &EventLoopWindowTarget<UserEvent>) {
        let Ok(cmd) = bridge::parse_command(json) else { return };
        // The flyout is display-only: it may announce itself, never command anything.
        if kind == SurfaceKind::Flyout {
            if matches!(cmd, bridge::UiCommand::Ready) {
                self.push_flyout();
            }
            return;
        }
        if matches!(cmd, bridge::UiCommand::Ready) {
            let js = bridge::script_call("onState", &self.model.ui_state(self.now()));
            match kind {
                SurfaceKind::Popup => self.popup.as_ref().map(|p| p.eval(&js)),
                SurfaceKind::Flyout => None,
                SurfaceKind::Full => {
                    if let Some(page) = self.pending_full_page.take() {
                        self.show_full(page, Some(target));
                    }
                    self.full.as_ref().map(|f| f.eval(&js))
                }
            };
        }
        let effects = self.model.on_command(cmd, self.now());
        self.perform(effects, Some(target));
    }

    fn on_telemetry(&mut self, d: TelemetryDelivery) {
        let now = self.now();
        let sampled = d.due.temperatures || d.due.fans;
        if d.due.temperatures {
            self.history.push(&d.update.snapshot);
        }
        let snapshot = d.update.snapshot.clone();
        self.errors = d.errors;
        self.report = d.report;
        let effects = self.model.on_telemetry(d.update, now);
        if let Some(t) = &mut self.tray {
            if sampled {
                t.record(&snapshot);
            }
            t.refresh(&self.model.settings, Some(&snapshot));
        }
        if sampled {
            self.push_flyout();
        }
        if let (Some(f), true, true) = (&self.full, self.full_visible, d.due.temperatures) {
            f.eval(&bridge::script_call("onSample", &bridge::Sample::from_snapshot(&snapshot)));
        }
        self.perform(effects, None);
        self.push_state();
    }
}

/// Tray icon rectangle in physical pixels.
fn icon_rect(rect: &tray_icon::Rect) -> Rect {
    let (x, y) = (rect.position.x.round() as i32, rect.position.y.round() as i32);
    Rect::new(x, y, x + rect.size.width as i32, y + rect.size.height as i32)
}
