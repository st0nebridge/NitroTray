//! @module thresholds_test
//! @description Temperature colour bands at their exact boundaries.
use nitrotray::telemetry::thresholds::{TempLevel, TempThresholds};

#[test]
fn cpu_bands_match_the_defaults() {
    let t = TempThresholds::CPU_DEFAULT;
    assert_eq!(t.classify(Some(69.9)), TempLevel::Normal);
    assert_eq!(t.classify(Some(70.0)), TempLevel::Warm);
    assert_eq!(t.classify(Some(84.9)), TempLevel::Warm);
    assert_eq!(t.classify(Some(85.0)), TempLevel::Hot);
    assert_eq!(t.classify(Some(94.9)), TempLevel::Hot);
    assert_eq!(t.classify(Some(95.0)), TempLevel::Critical);
}

#[test]
fn gpu_bands_match_the_defaults() {
    let t = TempThresholds::GPU_DEFAULT;
    assert_eq!(t.classify(Some(69.0)), TempLevel::Normal);
    assert_eq!(t.classify(Some(70.0)), TempLevel::Warm);
    assert_eq!(t.classify(Some(79.9)), TempLevel::Warm);
    assert_eq!(t.classify(Some(80.0)), TempLevel::Hot);
    assert_eq!(t.classify(Some(89.9)), TempLevel::Hot);
    assert_eq!(t.classify(Some(90.0)), TempLevel::Critical);
}

#[test]
fn missing_or_invalid_readings_are_unknown() {
    let t = TempThresholds::CPU_DEFAULT;
    assert_eq!(t.classify(None), TempLevel::Unknown);
    assert_eq!(t.classify(Some(f32::NAN)), TempLevel::Unknown);
    assert_eq!(t.classify(Some(f32::INFINITY)), TempLevel::Unknown);
}

#[test]
fn validation_requires_ordered_in_range_values() {
    assert!(TempThresholds::CPU_DEFAULT.is_valid());
    assert!(TempThresholds::GPU_DEFAULT.is_valid());
    let unordered = TempThresholds { warm_c: 80.0, hot_c: 70.0, critical_c: 90.0 };
    assert!(!unordered.is_valid());
    let equal = TempThresholds { warm_c: 70.0, hot_c: 80.0, critical_c: 80.0 };
    assert!(!equal.is_valid());
    let low = TempThresholds { warm_c: 29.0, hot_c: 80.0, critical_c: 90.0 };
    assert!(!low.is_valid());
    let high = TempThresholds { warm_c: 70.0, hot_c: 80.0, critical_c: 111.0 };
    assert!(!high.is_valid());
    let edges = TempThresholds { warm_c: 30.0, hot_c: 80.0, critical_c: 110.0 };
    assert!(edges.is_valid());
    assert_eq!(unordered.or_default(TempThresholds::CPU_DEFAULT), TempThresholds::CPU_DEFAULT);
    assert_eq!(edges.or_default(TempThresholds::CPU_DEFAULT), edges);
}

#[test]
fn levels_serialise_snake_case() {
    assert_eq!(serde_json::to_string(&TempLevel::Critical).unwrap(), "\"critical\"");
    assert_eq!(serde_json::to_string(&TempLevel::Unknown).unwrap(), "\"unknown\"");
}
