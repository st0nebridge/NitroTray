//! @module app::effects
//! @description Executes the side effects the app model requests: control writes, windows, persistence,
//! autostart, NitroSense launch, helper install, diagnostics, tray-icon promotion.
//!
//! @input  `Vec<Effect>` from `AppModel`, the runtime's handles.
//! @output OS side effects; notices in the model when an effect fails.
//! @dependencies app::{model, runtime, events, worker}, acer::{nitrosense, services}, platform, diagnostics, ui::bridge
use tao::event_loop::EventLoopWindowTarget;

use crate::acer::services::{self, ServiceState};
use crate::app::events::UserEvent;
use crate::app::model::{connection_text, Effect};
use crate::app::runtime::Runtime;
use crate::app::worker::{TelemetryConfig, WorkerMsg};
use crate::platform::windows::{apps_use_light_theme, Autostart};
use crate::ui::bridge::{self, IntegrationInfo, NoticeKind, WindowData};

const HELPER_EXE: &str = "NitroTrayService.exe";

fn current_exe() -> String {
    std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default()
}

impl Runtime {
    pub(crate) fn perform(&mut self, effects: Vec<Effect>, target: Option<&EventLoopWindowTarget<UserEvent>>) {
        let changed = !effects.is_empty();
        for effect in effects {
            self.perform_one(effect, target);
        }
        if changed {
            self.push_state();
        }
    }

    fn perform_one(&mut self, effect: Effect, target: Option<&EventLoopWindowTarget<UserEvent>>) {
        let now = self.now();
        match effect {
            Effect::Control(req) => {
                let _ = self.control_tx.send(req);
            }
            Effect::ShowPopup => self.show_popup(None),
            Effect::HidePopup => self.hide_popup(),
            Effect::DragPopup => {
                if let Some(p) = &self.popup {
                    let _ = p.window.drag_window();
                }
            }
            Effect::OpenWindow(page) => self.show_full(page, target),
            Effect::LaunchNitroSense(aumid) => {
                if let Err(e) = crate::acer::nitrosense::launch(&aumid) {
                    self.model.notify(NoticeKind::Error, "Could not open NitroSense.", e, now);
                }
            }
            Effect::PersistSettings => {
                if let Err(e) = self.store.save(&self.model.settings) {
                    self.model.notify(NoticeKind::Error, "Settings could not be saved.", e, now);
                }
            }
            Effect::ApplySettings => self.apply_settings(),
            Effect::SetAutostart(enabled) => match Autostart::default().set(enabled, &current_exe()) {
                Ok(()) => {
                    self.model.set_autostart(enabled);
                    self.send_window_data();
                }
                Err(e) => self.model.notify(NoticeKind::Error, "Start with Windows could not be changed.", e, now),
            },
            Effect::ExportDiagnostics => self.export_diagnostics(),
            Effect::InstallHelper => self.install_helper(),
            Effect::PinTrayIcons => self.pin_tray_icons(),
            Effect::SendWindowData => self.send_window_data(),
            Effect::RefreshModes => {
                let _ = self.worker_tx.send(WorkerMsg::RefreshModes);
            }
            Effect::Exit => {
                let _ = self.worker_tx.send(WorkerMsg::Stop);
                if let Some(t) = &mut self.tray {
                    t.clear();
                }
                self.exit = true;
            }
        }
    }

    fn apply_settings(&mut self) {
        let settings = self.model.settings.clone();
        if let Some(t) = &mut self.tray {
            t.sync(&settings);
        }
        self.register_hotkey();
        let _ = self.worker_tx.send(WorkerMsg::Config(TelemetryConfig::from_settings(&settings)));
        self.model.set_light_system_theme(apps_use_light_theme());
        if settings.appearance.transparency != self.popup_transparent {
            self.rebuild_popup = true;
        }
        self.sync_flyout = true;
        self.apply_chrome();
        self.push_flyout();
    }

    pub(crate) fn send_window_data(&mut self) {
        let Some(full) = &self.full else { return };
        let r = self.report.as_ref();
        let status = self.model.status();
        let data = WindowData {
            settings: &self.model.settings,
            autostart: self.model.autostart(),
            integration: IntegrationInfo {
                status,
                status_text: connection_text(status).into(),
                provider: r.map(|r| r.provider.clone()).unwrap_or_default(),
                model: r
                    .map(|r| r.model.clone())
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| crate::platform::windows::machine_info().model.unwrap_or_default()),
                helper_service: services::query(crate::meta::SERVICE_NAME),
                acer_service: services::query("PSSvc"),
                config_dir: self.store.dir().display().to_string(),
            },
            history: self.history.view(),
        };
        full.eval(&bridge::script_call("onWindowData", &data));
    }

    fn export_diagnostics(&mut self) {
        let now = self.now();
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let report = crate::diagnostics::build(crate::diagnostics::ReportInputs {
            unix_secs: secs,
            machine: crate::diagnostics::machine(),
            services: services::report(),
            provider: self.report.as_ref().map_or("none", |r| r.provider.as_str()),
            status: self.model.status(),
            capabilities: self.model.capabilities(),
            errors: &self.errors,
        });
        match crate::diagnostics::write(self.store.dir(), &report) {
            Ok(path) => {
                let _ = crate::platform::windows::shell_open(&self.store.dir().display().to_string());
                self.model.notify(NoticeKind::Info, "Diagnostics exported.", path.display().to_string(), now);
            }
            Err(e) => self.model.notify(NoticeKind::Error, "Diagnostics export failed.", e, now),
        }
    }

    fn install_helper(&mut self) {
        let now = self.now();
        let exe = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join(HELPER_EXE)));
        let Some(helper) = exe.filter(|p| p.exists()) else {
            self.model.notify(
                NoticeKind::Error,
                "Helper not found.",
                format!("{HELPER_EXE} must sit next to NitroTray.exe."),
                now,
            );
            return;
        };
        if services::query(crate::meta::SERVICE_NAME) != ServiceState::NotInstalled {
            self.model.notify(
                NoticeKind::Info,
                "Helper already installed.",
                "Use Services to start it if it is stopped.",
                now,
            );
            return;
        }
        match crate::platform::windows::run_elevated(&helper.display().to_string(), "install") {
            Ok(()) => {
                self.model.notify(NoticeKind::Info, "Installing helper service…", "Approve the Windows prompt.", now);
                let proxy = self.proxy.clone();
                std::thread::spawn(move || {
                    for _ in 0..6 {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        let _ = proxy.send_event(UserEvent::CapabilitiesRecheck);
                    }
                });
            }
            Err(e) => self.model.notify(NoticeKind::Error, "Helper install was not started.", e, now),
        }
    }

    fn pin_tray_icons(&mut self) {
        let now = self.now();
        match crate::platform::tray_prefs::promote(&current_exe(), crate::platform::tray_prefs::NOTIFY_ROOT) {
            Ok(0) => self.model.notify(
                NoticeKind::Info,
                "Nothing to pin yet.",
                "Windows has not registered the icons yet; try again shortly.",
                now,
            ),
            Ok(n) => self.model.notify(
                NoticeKind::Info,
                "Tray icons pinned.",
                format!("{n} icon(s) set to show on the taskbar."),
                now,
            ),
            Err(e) => self.model.notify(NoticeKind::Error, "Could not pin tray icons.", e, now),
        }
    }
}
