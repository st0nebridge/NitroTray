//! @module unlock_cuda_live_test
//! @description Live check of the tensor probe on the installed NVIDIA GPU. Ignored by default: it needs an
//! Ampere-or-newer GPU and wakes it. Run with
//! `cargo test --test unit unlock_cuda_live -- --ignored --nocapture`; set `NITROTRAY_EXPECT_TENSOR` to
//! `locked` or `unlocked` to assert the verdict as well (the unlock is lost at power-off, so only the
//! person running this knows which state the machine is in).
use nitrotray::unlock::cuda::measure;
use nitrotray::unlock::state::{classify, TensorState};

#[test]
#[ignore = "needs an NVIDIA GPU and wakes it"]
fn the_probe_measures_both_accumulate_rates() {
    let rates = measure().expect("probe");
    let ratio = rates.ratio().expect("a usable ratio");
    println!("acc32 {:.1} TFLOPS, acc16 {:.1} TFLOPS, ratio {ratio:.2}", rates.acc32_tflops, rates.acc16_tflops);
    assert!(rates.acc16_tflops > 1.0, "the reference rate is not plausibly zero: {rates:?}");
    assert!((0.3..1.3).contains(&ratio), "the ratio is a throttle or none: {ratio}");
    match std::env::var("NITROTRAY_EXPECT_TENSOR").as_deref() {
        Ok("locked") => assert_eq!(classify(rates), Some(TensorState::Locked), "{rates:?}"),
        Ok("unlocked") => assert_eq!(classify(rates), Some(TensorState::Unlocked), "{rates:?}"),
        _ => {}
    }
}

#[test]
#[ignore = "needs an NVIDIA GPU and wakes it"]
fn the_probe_leaves_nothing_behind_and_repeats() {
    let first = measure().expect("first probe");
    let second = measure().expect("a second probe in the same process");
    let (a, b) = (first.ratio().unwrap(), second.ratio().unwrap());
    println!("ratios {a:.3} and {b:.3}");
    assert!((a - b).abs() < 0.1, "the verdict is stable between runs: {a} vs {b}");
}
