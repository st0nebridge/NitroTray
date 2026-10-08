//! @module app::tray_host
//! @description Owns the notification-area icons (app icon + CPU/GPU graph icons) and the context menu.
//!
//! @input  Settings (which icons, colours, ranges), snapshots, menu model.
//! @output Live tray icons with graph images and tooltips (none on the graph icons while their hover
//! flyout replaces it); menu check/enabled states.
//! @dependencies tray-icon (muda), tray::{app_icon, graph_icon, menu}, telemetry::history, config
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use windows::Win32::UI::WindowsAndMessaging::SM_CXSMICON;

use crate::config::settings::{GraphIconSettings, Settings, TemperatureUnit};
use crate::controls::{FanMode, PerformanceMode};
use crate::telemetry::history::History;
use crate::telemetry::snapshot::SystemSnapshot;
use crate::tray::menu::MenuModel;
use crate::tray::{app_icon, graph_icon};

const HISTORY: usize = 64;

struct Series {
    temp: History,
    fan: History,
    fan_peak: Option<f32>,
}

impl Series {
    fn new() -> Self {
        Self { temp: History::new(HISTORY), fan: History::new(HISTORY), fan_peak: None }
    }
}

pub struct TrayHost {
    menu: Menu,
    perf_items: Vec<CheckMenuItem>,
    fan_items: Vec<CheckMenuItem>,
    autostart_item: CheckMenuItem,
    app: Option<TrayIcon>,
    cpu: Option<TrayIcon>,
    gpu: Option<TrayIcon>,
    size: u32,
    cpu_series: Series,
    gpu_series: Series,
    left_click_menu: bool,
}

/// Tray icon edge in pixels for the system DPI (16 at 100 %, 20 at 125 %, 24 at 150 %).
pub fn tray_icon_px() -> u32 {
    // SAFETY: plain metric queries.
    unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()).clamp(16, 64) as u32 }
}

fn icon(rgba: Vec<u8>, size: u32) -> Option<Icon> {
    Icon::from_rgba(rgba, size, size).ok()
}

impl TrayHost {
    pub fn new() -> Result<Self, String> {
        let perf_items: Vec<CheckMenuItem> = PerformanceMode::ALL
            .iter()
            .map(|m| CheckMenuItem::with_id(format!("perf:{}", m.as_str()), m.label(), false, false, None))
            .collect();
        let fan_items: Vec<CheckMenuItem> = FanMode::ALL
            .iter()
            .map(|m| CheckMenuItem::with_id(format!("fan:{}", m.as_str()), m.label(), false, false, None))
            .collect();
        let perf_refs: Vec<&dyn tray_icon::menu::IsMenuItem> = perf_items.iter().map(|i| i as _).collect();
        let fan_refs: Vec<&dyn tray_icon::menu::IsMenuItem> = fan_items.iter().map(|i| i as _).collect();
        let perf = Submenu::with_items("Performance Mode", true, &perf_refs).map_err(|e| e.to_string())?;
        let fan = Submenu::with_items("Fan Mode", true, &fan_refs).map_err(|e| e.to_string())?;
        let autostart_item = CheckMenuItem::with_id("autostart", "Start with Windows", true, false, None);
        let menu = Menu::with_items(&[
            &MenuItem::with_id("open", "Open Nitro Tray", true, None),
            &PredefinedMenuItem::separator(),
            &perf,
            &fan,
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id("monitor", "Full Monitoring…", true, None),
            &MenuItem::with_id("settings", "Settings…", true, None),
            &MenuItem::with_id("nitrosense", "Open NitroSense", true, None),
            &autostart_item,
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id("exit", "Exit", true, None),
        ])
        .map_err(|e| e.to_string())?;
        Ok(Self {
            menu,
            perf_items,
            fan_items,
            autostart_item,
            app: None,
            cpu: None,
            gpu: None,
            size: tray_icon_px(),
            cpu_series: Series::new(),
            gpu_series: Series::new(),
            left_click_menu: false,
        })
    }

    fn build(&self, id: &str, rgba: Vec<u8>, tooltip: &str) -> Option<TrayIcon> {
        TrayIconBuilder::new()
            .with_id(id)
            .with_icon(icon(rgba, self.size)?)
            .with_tooltip(tooltip)
            .with_menu(Box::new(self.menu.clone()))
            .with_menu_on_left_click(self.left_click_menu)
            .build()
            .ok()
    }

    /// Creates or removes icons to match settings (order: app, CPU, GPU).
    pub fn sync(&mut self, settings: &Settings) {
        let left_menu = !settings.general.open_popup_on_tray_click;
        if left_menu != self.left_click_menu {
            self.left_click_menu = left_menu;
            self.app = None;
            self.cpu = None;
            self.gpu = None;
        }
        // The shell inserts newer icons to the left: create app, GPU, CPU so they read CPU | GPU | app.
        let t = &settings.tray_icons;
        if !t.show_app_icon {
            self.app = None;
        }
        if !t.gpu.enabled {
            self.gpu = None;
        }
        if !t.cpu.enabled {
            self.cpu = None;
        }
        if t.show_app_icon && self.app.is_none() {
            self.app = self.build("app", app_icon::render(self.size), crate::meta::APP_NAME);
        }
        if t.gpu.enabled && self.gpu.is_none() {
            self.gpu = self.build("gpu", self.graph(&t.gpu, &self.gpu_series), "GPU");
        }
        if t.cpu.enabled && self.cpu.is_none() {
            self.cpu = self.build("cpu", self.graph(&t.cpu, &self.cpu_series), "CPU");
        }
        self.refresh(settings, None);
    }

    fn graph(&self, s: &GraphIconSettings, series: &Series) -> Vec<u8> {
        let style = graph_icon::style_from(s, series.fan_peak);
        graph_icon::render(self.size, &style, &series.temp.values(), &series.fan.values())
    }

    /// Records one temperature/fan sample for the graphs.
    pub fn record(&mut self, snap: &SystemSnapshot) {
        let rpm = |f: Option<crate::telemetry::snapshot::FanTelemetry>| f.and_then(|f| f.rpm).map(|r| r as f32);
        for (series, temp, fan) in [
            (&mut self.cpu_series, snap.cpu.temperature_c, rpm(snap.cpu_fan)),
            (&mut self.gpu_series, snap.gpu.temperature_c, rpm(snap.gpu_fan)),
        ] {
            series.temp.push(temp);
            series.fan.push(fan);
            if let Some(f) = fan {
                series.fan_peak = Some(series.fan_peak.map_or(f, |p| p.max(f)));
            }
        }
    }

    /// Redraws graph icons and tooltips from the recorded history.
    pub fn refresh(&self, settings: &Settings, snap: Option<&SystemSnapshot>) {
        let unit: TemperatureUnit = settings.telemetry.temperature_unit;
        let rpm = |f: Option<crate::telemetry::snapshot::FanTelemetry>| f.and_then(|f| f.rpm);
        let plain_tips = !settings.tray_icons.hover_details;
        if let Some(tray) = &self.cpu {
            let _ = tray.set_icon(icon(self.graph(&settings.tray_icons.cpu, &self.cpu_series), self.size));
            let tip = graph_icon::tooltip(
                "CPU",
                snap.and_then(|s| s.cpu.temperature_c),
                unit,
                snap.and_then(|s| rpm(s.cpu_fan)),
            );
            let _ = tray.set_tooltip(plain_tips.then_some(tip));
        }
        if let Some(tray) = &self.gpu {
            let _ = tray.set_icon(icon(self.graph(&settings.tray_icons.gpu, &self.gpu_series), self.size));
            let tip = graph_icon::tooltip(
                "GPU",
                snap.and_then(|s| s.gpu.temperature_c),
                unit,
                snap.and_then(|s| rpm(s.gpu_fan)),
            );
            let _ = tray.set_tooltip(plain_tips.then_some(tip));
        }
        if let (Some(tray), Some(s)) = (&self.app, snap) {
            let t =
                |c: Option<f32>| c.map_or_else(|| "--".into(), |c| format!("{:.0}{}", unit.convert(c), unit.symbol()));
            let _ = tray.set_tooltip(Some(format!(
                "{} — CPU {} · GPU {}",
                crate::meta::APP_NAME,
                t(s.cpu.temperature_c),
                t(s.gpu.temperature_c)
            )));
        }
    }

    pub fn update_menu(&self, model: &MenuModel) {
        for (item, m) in self.perf_items.iter().zip(&model.performance) {
            item.set_checked(m.checked);
            item.set_enabled(m.enabled);
        }
        for (item, m) in self.fan_items.iter().zip(&model.fan) {
            item.set_checked(m.checked);
            item.set_enabled(m.enabled);
        }
        self.autostart_item.set_checked(model.autostart);
    }

    /// Removes all icons (on exit) so none linger in the notification area.
    pub fn clear(&mut self) {
        self.app = None;
        self.cpu = None;
        self.gpu = None;
    }
}
