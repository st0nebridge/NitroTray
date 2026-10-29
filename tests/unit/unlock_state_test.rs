//! @module unlock_state_test
//! @description Tensor-core rate verdict: the FP32-accumulate to FP16-accumulate ratio and its threshold.
use nitrotray::unlock::state::{classify, TensorRates, TensorState, UNLOCKED_RATIO};

fn rates(acc32: f32, acc16: f32) -> TensorRates {
    TensorRates { acc32_tflops: acc32, acc16_tflops: acc16 }
}

#[test]
fn the_locked_rate_measured_on_the_an515_58_is_locked() {
    // 27.6 and 55.1 TFLOPS: the half-rate throttle, measured 2026-10-29.
    let r = rates(27.6, 55.1);
    assert!((r.ratio().unwrap() - 0.50).abs() < 0.01);
    assert_eq!(classify(r), Some(TensorState::Locked));
}

#[test]
fn a_full_rate_is_unlocked() {
    assert_eq!(classify(rates(45.5, 45.7)), Some(TensorState::Unlocked));
    assert_eq!(classify(rates(55.0, 55.0)), Some(TensorState::Unlocked));
}

#[test]
fn the_threshold_is_exclusive_and_sits_between_the_two_rates() {
    let threshold = std::hint::black_box(UNLOCKED_RATIO);
    assert!(threshold > 0.5 && threshold < 1.0, "between the throttled 0.50 and the lifted 1.0");
    assert_eq!(classify(rates(UNLOCKED_RATIO * 100.0, 100.0)), Some(TensorState::Locked), "exactly at the threshold");
    assert_eq!(classify(rates(UNLOCKED_RATIO * 100.0 + 0.5, 100.0)), Some(TensorState::Unlocked));
    assert_eq!(classify(rates(UNLOCKED_RATIO * 100.0 - 0.5, 100.0)), Some(TensorState::Locked));
}

#[test]
fn no_verdict_without_a_usable_reference() {
    assert_eq!(rates(30.0, 0.0).ratio(), None);
    assert_eq!(rates(30.0, -1.0).ratio(), None);
    assert_eq!(rates(f32::NAN, 50.0).ratio(), None);
    assert_eq!(rates(30.0, f32::INFINITY).ratio(), Some(0.0), "a finite ratio, however odd");
    assert_eq!(classify(rates(30.0, 0.0)), None);
}

#[test]
fn negative_throughput_is_no_measurement() {
    assert_eq!(rates(-1.0, 50.0).ratio(), None);
}
