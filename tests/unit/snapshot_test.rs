//! @module snapshot_test
//! @description Snapshot emptiness and wire format.
use nitrotray::controls::{FanMode, PerformanceMode};
use nitrotray::telemetry::snapshot::{CpuTelemetry, FanTelemetry, GpuTelemetry, SystemSnapshot};

#[test]
fn default_snapshot_is_empty() {
    assert!(SystemSnapshot::default().is_empty());
}

#[test]
fn any_reading_makes_it_non_empty() {
    let cases = [
        SystemSnapshot { cpu: CpuTelemetry { temperature_c: Some(50.0), ..Default::default() }, ..Default::default() },
        SystemSnapshot { gpu: GpuTelemetry { utilisation_pct: Some(1.0), ..Default::default() }, ..Default::default() },
        SystemSnapshot { cpu_fan: Some(FanTelemetry::default()), ..Default::default() },
        SystemSnapshot { gpu_fan: Some(FanTelemetry::default()), ..Default::default() },
        SystemSnapshot { fan_mode: Some(FanMode::Auto), ..Default::default() },
        SystemSnapshot { performance_mode: Some(PerformanceMode::Quiet), ..Default::default() },
    ];
    for s in cases {
        assert!(!s.is_empty(), "{s:?}");
    }
}

#[test]
fn serialises_modes_snake_case_and_missing_as_null() {
    let s = SystemSnapshot { fan_mode: Some(FanMode::Max), ..Default::default() };
    let v: serde_json::Value = serde_json::to_value(&s).unwrap();
    assert_eq!(v["fan_mode"], "max");
    assert!(v["cpu"]["temperature_c"].is_null());
    assert!(v["performance_mode"].is_null());
}
