//! @module controls::fan_mode
//! @description Fan mode enum (Auto / Max / Custom) and its mapping to Acer firmware fan-behaviour values.
//!
//! @input  Firmware fan-behaviour 2-bit fields or UI/IPC mode names.
//! @output `FanMode` values; out-of-range firmware values map to `None`.
//! @dependencies serde
//!
//! Firmware values from upstream Linux `acer-wmi` (`acer_wmi_gaming_fan_mode`), verified on an
//! AN515-58 where both fans read back `0x01` (auto) in `GetGamingFanBehavior`.
use serde::{Deserialize, Serialize};

pub const BEHAVIOR_AUTO: u8 = 0x01;
pub const BEHAVIOR_TURBO: u8 = 0x02;
pub const BEHAVIOR_CUSTOM: u8 = 0x03;

/// The three user-facing fan modes of the tray popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FanMode {
    Auto,
    Max,
    Custom,
}

impl FanMode {
    pub const ALL: [FanMode; 3] = [Self::Auto, Self::Max, Self::Custom];

    /// Firmware fan-behaviour value written for this mode.
    pub fn firmware_behavior(self) -> u8 {
        match self {
            Self::Auto => BEHAVIOR_AUTO,
            Self::Max => BEHAVIOR_TURBO,
            Self::Custom => BEHAVIOR_CUSTOM,
        }
    }

    pub fn from_firmware_behavior(value: u8) -> Option<Self> {
        match value {
            BEHAVIOR_AUTO => Some(Self::Auto),
            BEHAVIOR_TURBO => Some(Self::Max),
            BEHAVIOR_CUSTOM => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Max => "Max",
            Self::Custom => "Custom",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "auto" => Some(Self::Auto),
            "max" => Some(Self::Max),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Max => "max",
            Self::Custom => "custom",
        }
    }
}
