//! @module ui
//! @description Web UI plumbing: the Rust↔page contract, embedded assets, and webview/window hosting.
//!
//! @dependencies ui::{bridge, assets, flyout, webview}
pub mod assets;
pub mod bridge;
pub mod flyout;
#[cfg(windows)]
pub mod webview;
