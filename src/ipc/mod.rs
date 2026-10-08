//! @module ipc
//! @description Tray ↔ helper IPC: typed protocol, named-pipe transport and client.
//!
//! @dependencies ipc::protocol, ipc::pipe, ipc::client
pub mod client;
#[cfg(windows)]
pub mod pipe;
pub mod protocol;
