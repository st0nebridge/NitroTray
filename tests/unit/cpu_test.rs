//! @module cpu_test
//! @description CPU clock/utility maths and a live PDH sample on this machine.
use nitrotray::telemetry::cpu::{clamp_pct, effective_mhz, PdhCpu};
use nitrotray::telemetry::CpuSource;

#[test]
fn effective_clock_scales_base_by_performance() {
    assert_eq!(effective_mhz(2300.0, 171.6), Some(3947));
    assert_eq!(effective_mhz(2300.0, 100.0), Some(2300));
    assert_eq!(effective_mhz(0.0, 100.0), None);
    assert_eq!(effective_mhz(2300.0, 0.0), None);
    assert_eq!(effective_mhz(f64::NAN, 100.0), None);
    assert_eq!(effective_mhz(2300.0, f64::INFINITY), None);
}

#[test]
fn utility_is_clamped_to_percent() {
    assert_eq!(clamp_pct(-3.0), Some(0.0));
    assert_eq!(clamp_pct(42.5), Some(42.5));
    assert_eq!(clamp_pct(130.0), Some(100.0));
    assert_eq!(clamp_pct(f64::NAN), None);
}

#[test]
fn live_pdh_sample_is_plausible() {
    let mut cpu = PdhCpu::open().expect("PDH processor counters");
    std::thread::sleep(std::time::Duration::from_millis(250));
    let s = cpu.sample().unwrap();
    let util = s.utilisation_pct.expect("utility");
    assert!((0.0..=100.0).contains(&util));
    let mhz = s.frequency_mhz.expect("clock");
    assert!((400..=6000).contains(&mhz), "{mhz} MHz");
}
