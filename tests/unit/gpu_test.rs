//! @module gpu_test
//! @description NVML unit conversion and a live sample from the RTX 4060 on this machine.
use nitrotray::telemetry::gpu::{milliwatts_to_watts, Nvml};
use nitrotray::telemetry::GpuSource;

#[test]
fn converts_milliwatts() {
    assert_eq!(milliwatts_to_watts(95_450), 95.5);
    assert_eq!(milliwatts_to_watts(0), 0.0);
    assert_eq!(milliwatts_to_watts(1_049), 1.0);
}

#[test]
fn live_nvml_sample_when_driver_present() {
    let Ok(mut nvml) = Nvml::load() else {
        eprintln!("NVML unavailable; skipping live sample");
        return;
    };
    let s = nvml.sample().unwrap();
    if let Some(t) = s.temperature_c {
        assert!((10.0..=110.0).contains(&t), "{t}");
    }
    if let Some(u) = s.utilisation_pct {
        assert!((0.0..=100.0).contains(&u));
    }
    assert!(s.frequency_mhz.is_some() || s.temperature_c.is_some());
}

#[test]
fn lazy_nvml_loads_once_and_caches_failure_or_device() {
    let mut lazy = nitrotray::telemetry::gpu::LazyNvml::default();
    let first = lazy.sample();
    let second = lazy.sample();
    assert_eq!(first.is_ok(), second.is_ok(), "stable outcome across calls");
    if let Err(e) = second {
        assert!(!e.is_empty());
    }
}
