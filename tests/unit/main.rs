//! @module unit
//! @description Unit/integration test crate: one submodule per library module (`<module>_test.rs`).
#![allow(clippy::field_reassign_with_default)]
mod support;

mod acpi_test;
mod app_icon_test;
mod app_worker_test;
mod assets_test;
mod backend_test;
mod bridge_test;
mod cli_test;
mod client_test;
mod collector_test;
mod core_test;
mod curve_test;
mod diagnostics_test;
mod discovery_test;
mod docs_images_test;
mod fan_curve_test;
mod fan_mode_test;
mod fan_test;
mod flyout_test;
mod gaming_test;
mod graph_icon_test;
mod history_test;
mod hotkey_test;
mod install_path_test;
mod installer_test;
mod menu_test;
mod meta_test;
mod model_test;
mod performance_mode_test;
mod positioning_test;
mod power_test;
mod protocol_test;
mod raster_test;
mod scheduler_test;
mod settings_test;
mod simulated_test;
mod snapshot_test;
mod store_test;
mod thresholds_test;
mod worker_test;

#[cfg(windows)]
mod binaries_test;
#[cfg(windows)]
mod cpu_test;
#[cfg(windows)]
mod gpu_test;
#[cfg(windows)]
mod hardware_test;
#[cfg(windows)]
mod host_test;
#[cfg(windows)]
mod hotkeys_test;
mod hover_test;
#[cfg(windows)]
mod live_control_test;
#[cfg(windows)]
mod nitrosense_test;
#[cfg(windows)]
mod pipe_test;
#[cfg(windows)]
mod services_test;
#[cfg(windows)]
mod signal_test;
#[cfg(windows)]
mod tray_prefs_test;
#[cfg(windows)]
mod windows_test;
#[cfg(windows)]
mod wmi_test;
