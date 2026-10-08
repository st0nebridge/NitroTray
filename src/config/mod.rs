//! @module config
//! @description User settings: model + sanitisation, hotkey parsing, persistence.
//!
//! @dependencies config::{settings, hotkey, store}
pub mod hotkey;
pub mod settings;
pub mod store;

pub use settings::Settings;
pub use store::ConfigStore;
