//! @module unlock
//! @description The GeForce tensor-core throttle and whether it is lifted: the rate verdict, the UEFI
//! unlocker's boot log, the combined view the pages show, the probe that measures the rate, and (on
//! Windows) the CUDA driver binding that runs it.
//!
//! @dependencies unlock::{state, boot_log, view, tracker, ptx, cuda}
pub mod boot_log;
#[cfg(windows)]
pub mod cuda;
pub mod ptx;
pub mod state;
pub mod tracker;
pub mod view;
