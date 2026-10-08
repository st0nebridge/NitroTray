//! @module platform
//! @description Operating-system services: registry, autostart, shell, machine info, power, signals, tray prefs.
//!
//! @dependencies platform::{windows, power, signal, tray_prefs}
pub mod power;
#[cfg(windows)]
pub mod signal;
#[cfg(windows)]
pub mod tray_prefs;
#[cfg(windows)]
pub mod windows;
