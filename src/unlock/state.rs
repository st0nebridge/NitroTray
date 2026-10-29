//! @module unlock::state
//! @description Verdict on the tensor-core rate from two timings. FP16 multiply-accumulate with FP32
//! accumulate runs at half rate on GeForce parts unless the throttle is lifted; with FP16 accumulate it is
//! never throttled, so the ratio of the two says which state the GPU is in.
//!
//! @input  `TensorRates`: throughput of the two accumulate modes, in TFLOPS.
//! @output `TensorState`, or no verdict when the reference is unusable.
//! @dependencies serde
use serde::{Deserialize, Serialize};

/// FP32-accumulate over FP16-accumulate above which the throttle counts as lifted. Measured on the
/// AN515-58: 0.50 throttled; an unlocked part reads about 1.0.
pub const UNLOCKED_RATIO: f32 = 0.75;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TensorRates {
    pub acc32_tflops: f32,
    pub acc16_tflops: f32,
}

impl TensorRates {
    /// FP32-accumulate rate over the FP16-accumulate reference; `None` unless both are usable numbers.
    pub fn ratio(&self) -> Option<f32> {
        let ratio = self.acc32_tflops / self.acc16_tflops;
        (self.acc16_tflops > 0.0 && self.acc32_tflops >= 0.0 && ratio.is_finite()).then_some(ratio)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TensorState {
    Unlocked,
    Locked,
}

pub fn classify(rates: TensorRates) -> Option<TensorState> {
    rates.ratio().map(|r| if r > UNLOCKED_RATIO { TensorState::Unlocked } else { TensorState::Locked })
}
