//! @module acer
//! @description Acer firmware integration: WMI codec, hardware-backend contract, discovery and NitroSense helpers.
//!
//! @dependencies acer::acpi, acer::backend, acer::gaming, acer::discovery, acer::error, acer::nitrosense, acer::services, acer::wmi
pub mod acpi;
pub mod backend;
pub mod discovery;
pub mod error;
pub mod gaming;
pub mod nitrosense;
pub mod services;
#[cfg(windows)]
pub mod wmi;

pub use backend::{AcerError, FanState, HardwareBackend, PerformanceState, SensorReadings};
pub use discovery::Capabilities;
