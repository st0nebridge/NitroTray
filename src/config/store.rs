//! @module config::store
//! @description Loads and atomically saves config.json (%LOCALAPPDATA%\NitroTray).
//!
//! @input  A directory (default, or `NITROTRAY_CONFIG_DIR` for development/tests).
//! @output Sanitised `Settings`; a warning when a corrupt file was set aside as config.json.bad.
//! @dependencies serde_json, config::settings, meta
use std::path::{Path, PathBuf};

use super::settings::Settings;

pub const FILE_NAME: &str = "config.json";

pub struct ConfigStore {
    dir: PathBuf,
}

/// `%LOCALAPPDATA%\NitroTray`, overridable with `NITROTRAY_CONFIG_DIR`.
pub fn default_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("NITROTRAY_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join(crate::meta::APP_NAME)
}

impl ConfigStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    /// Missing file → defaults. Corrupt file → defaults, the file is renamed to `.bad`, and a
    /// warning is returned so the UI can say so.
    pub fn load(&self) -> (Settings, Option<String>) {
        let path = self.path();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return (Settings::default(), None),
        };
        match serde_json::from_str::<Settings>(&text) {
            Ok(s) => (s.sanitized(), None),
            Err(e) => {
                let bad = self.dir.join(format!("{FILE_NAME}.bad"));
                let _ = std::fs::rename(&path, &bad);
                (Settings::default(), Some(format!("Settings file was unreadable ({e}); defaults restored.")))
            }
        }
    }

    /// Write to a temp file then rename over the old one, so a crash never leaves half a file.
    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("cannot create {}: {e}", self.dir.display()))?;
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        let tmp = self.dir.join(format!("{FILE_NAME}.tmp"));
        std::fs::write(&tmp, json).map_err(|e| format!("cannot write settings: {e}"))?;
        std::fs::rename(&tmp, self.path()).map_err(|e| format!("cannot replace settings: {e}"))
    }
}
