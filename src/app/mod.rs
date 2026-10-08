//! @module app
//! @description The tray host application: pure state model, background workers, and the event-loop runtime.
//!
//! @dependencies app::{model, worker, events, bootstrap, runtime, surfaces, effects, tray_host, hotkeys}
#[cfg(windows)]
pub mod bootstrap;
#[cfg(windows)]
pub mod effects;
#[cfg(windows)]
pub mod events;
#[cfg(windows)]
pub mod hotkeys;
pub mod model;
#[cfg(windows)]
pub mod runtime;
#[cfg(windows)]
pub mod surfaces;
#[cfg(windows)]
pub mod tray_host;
pub mod worker;
