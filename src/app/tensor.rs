//! @module app::tensor
//! @description Background worker for the tensor-core check: on request it measures the GPU's tensor rates
//! and asks the helper for the unlocker's newest boot report, then delivers both answers together. One
//! request at a time, on its own thread, so a first CUDA context (about half a second on a sleeping GPU)
//! never stalls the telemetry loop or the UI.
//!
//! @input  `TensorMsg`s, a measurement function, a boot-report function, a delivery callback.
//! @output `TensorResult`s through the callback; a panicking measurement becomes an error.
//! @dependencies unlock::{state, boot_log}
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::Receiver;

use crate::unlock::boot_log::BootReport;
use crate::unlock::state::TensorRates;

pub enum TensorMsg {
    Check,
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TensorResult {
    pub rates: Result<TensorRates, String>,
    /// The helper's answer: `Ok(None)` means it looked and found no log.
    pub boot: Result<Option<BootReport>, String>,
}

/// Serves `Check` requests until `Stop` or the channel closes.
pub fn run_tensor(
    rx: &Receiver<TensorMsg>,
    mut measure: impl FnMut() -> Result<TensorRates, String>,
    mut boot: impl FnMut() -> Result<Option<BootReport>, String>,
    mut deliver: impl FnMut(TensorResult),
) {
    while let Ok(TensorMsg::Check) = rx.recv() {
        let boot = boot();
        let rates = catch_unwind(AssertUnwindSafe(&mut measure))
            .unwrap_or_else(|_| Err("the tensor probe panicked".to_string()));
        deliver(TensorResult { rates, boot });
    }
}
