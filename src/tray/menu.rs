//! @module tray::menu
//! @description Tray context menu: stable item ids, check states, and id → action mapping.
//!
//! @input  Current fan/performance mode, capabilities, autostart state.
//! @output A declarative `MenuModel` the tray host renders, and `MenuAction`s for clicked ids.
//! @dependencies controls, acer::Capabilities

use crate::acer::Capabilities;
use crate::controls::{FanMode, PerformanceMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    OpenPopup,
    SetPerformance(PerformanceMode),
    SetFan(FanMode),
    OpenNitroSense,
    OpenMonitoring,
    OpenSettings,
    ToggleAutostart,
    Exit,
}

pub fn action_for(id: &str) -> Option<MenuAction> {
    Some(match id {
        "open" => MenuAction::OpenPopup,
        "nitrosense" => MenuAction::OpenNitroSense,
        "monitor" => MenuAction::OpenMonitoring,
        "settings" => MenuAction::OpenSettings,
        "autostart" => MenuAction::ToggleAutostart,
        "exit" => MenuAction::Exit,
        _ => {
            if let Some(m) = id.strip_prefix("perf:") {
                MenuAction::SetPerformance(PerformanceMode::parse(m)?)
            } else {
                MenuAction::SetFan(FanMode::parse(id.strip_prefix("fan:")?)?)
            }
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckItem {
    pub id: String,
    pub label: String,
    pub checked: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuModel {
    pub performance: Vec<CheckItem>,
    pub fan: Vec<CheckItem>,
    pub autostart: bool,
}

pub fn model(fan: Option<FanMode>, perf: Option<PerformanceMode>, caps: &Capabilities, autostart: bool) -> MenuModel {
    MenuModel {
        performance: PerformanceMode::ALL
            .iter()
            .map(|m| CheckItem {
                id: format!("perf:{}", m.as_str()),
                label: m.label().into(),
                checked: perf == Some(*m),
                enabled: caps.supports_performance(*m),
            })
            .collect(),
        fan: FanMode::ALL
            .iter()
            .map(|m| CheckItem {
                id: format!("fan:{}", m.as_str()),
                label: m.label().into(),
                checked: fan == Some(*m),
                enabled: caps.supports_fan(*m),
            })
            .collect(),
        autostart,
    }
}
