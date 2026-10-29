//! @module service
//! @description Privileged helper: typed request core, curve controller, hosting.
//!
//! @dependencies service::{cli, core, curve, simulated, worker, host, install_path, unlock_log, esp}
pub mod cli;
pub mod core;
pub mod curve;
#[cfg(windows)]
pub mod esp;
#[cfg(windows)]
pub mod host;
pub mod install_path;
pub mod simulated;
pub mod unlock_log;
pub mod worker;
