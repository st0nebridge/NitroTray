//! @module controls::performance_mode
//! @description Performance mode enum (Quiet / Default / Performance) and its mapping to Acer firmware profiles.
//!
//! @input  Firmware platform-profile bytes (misc setting 0x0B) or UI/IPC mode names.
//! @output `PerformanceMode` values; unknown firmware profiles map to `None` (never invented).
//! @dependencies serde
//!
//! Firmware values verified on a Nitro AN515-58 (NitroSense `CurrentOperationMode` = firmware 0x04)
//! and against upstream Linux `acer-wmi` (`ACER_PREDATOR_V4_THERMAL_PROFILE_*`).
use serde::{Deserialize, Serialize};

/// Firmware profile values (`acer_predator_v4_thermal_profile`).
pub const PROFILE_QUIET: u8 = 0x00;
pub const PROFILE_BALANCED: u8 = 0x01;
pub const PROFILE_PERFORMANCE: u8 = 0x04;
pub const PROFILE_TURBO: u8 = 0x05;
pub const PROFILE_ECO: u8 = 0x06;

/// The three user-facing performance modes of the tray popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceMode {
    Quiet,
    Default,
    Performance,
}

impl PerformanceMode {
    pub const ALL: [PerformanceMode; 3] = [Self::Quiet, Self::Default, Self::Performance];

    /// Firmware platform-profile byte written for this mode.
    pub fn firmware_profile(self) -> u8 {
        match self {
            Self::Quiet => PROFILE_QUIET,
            Self::Default => PROFILE_BALANCED,
            Self::Performance => PROFILE_PERFORMANCE,
        }
    }

    /// Maps a firmware profile back to a popup mode; profiles the popup does not expose return `None`.
    pub fn from_firmware_profile(profile: u8) -> Option<Self> {
        match profile {
            PROFILE_QUIET => Some(Self::Quiet),
            PROFILE_BALANCED => Some(Self::Default),
            PROFILE_PERFORMANCE => Some(Self::Performance),
            _ => None,
        }
    }

    /// Human label as shown on the segmented control.
    pub fn label(self) -> &'static str {
        match self {
            Self::Quiet => "Quiet",
            Self::Default => "Default",
            Self::Performance => "Performance",
        }
    }

    /// Parses the snake_case wire name.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "quiet" => Some(Self::Quiet),
            "default" => Some(Self::Default),
            "performance" => Some(Self::Performance),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Default => "default",
            Self::Performance => "performance",
        }
    }
}

/// Descriptive name for any firmware profile, including ones the popup does not expose.
pub fn firmware_profile_name(profile: u8) -> &'static str {
    match profile {
        PROFILE_QUIET => "Quiet",
        PROFILE_BALANCED => "Default",
        PROFILE_PERFORMANCE => "Performance",
        PROFILE_TURBO => "Turbo",
        PROFILE_ECO => "Eco",
        _ => "Unknown",
    }
}
