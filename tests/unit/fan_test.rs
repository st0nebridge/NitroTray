//! @module fan_test
//! @description Honest fan rings: duty when known, measured maximum otherwise, else nothing.
use nitrotray::controls::FanMode;
use nitrotray::telemetry::fan::{fan_telemetry, ring_pct, Calibrator, FanCalibration, CALIBRATION_SETTLE_MS};
use nitrotray::telemetry::snapshot::FanTelemetry;

#[test]
fn builds_telemetry_and_caps_duty() {
    assert_eq!(fan_telemetry(Some(5000), Some(150)), FanTelemetry { rpm: Some(5000), duty_pct: Some(100.0) });
    assert_eq!(fan_telemetry(None, None), FanTelemetry::default());
}

#[test]
fn ring_prefers_known_duty() {
    let f = FanTelemetry { rpm: Some(3000), duty_pct: Some(40.0) };
    assert_eq!(ring_pct(&f, Some(6000)), Some(40.0));
}

#[test]
fn ring_uses_measured_maximum_only() {
    let f = FanTelemetry { rpm: Some(3000), duty_pct: None };
    assert_eq!(ring_pct(&f, Some(6000)), Some(50.0));
    assert_eq!(ring_pct(&f, None), None, "no guessed maximum");
    assert_eq!(ring_pct(&f, Some(0)), None);
    let over = FanTelemetry { rpm: Some(7000), duty_pct: None };
    assert_eq!(ring_pct(&over, Some(6000)), Some(100.0));
    assert_eq!(ring_pct(&FanTelemetry::default(), Some(6000)), None);
}

#[test]
fn calibrator_learns_only_after_max_mode_settles() {
    let mut c = Calibrator::default();
    let known = FanCalibration::default();
    assert_eq!(c.observe(Some(FanMode::Auto), (Some(7000), Some(7000)), 0, known), None);
    assert_eq!(c.observe(Some(FanMode::Max), (Some(3000), Some(3000)), 1000, known), None);
    assert_eq!(c.observe(Some(FanMode::Max), (Some(6000), Some(6200)), 1000 + CALIBRATION_SETTLE_MS - 1, known), None);
    let learned = c.observe(Some(FanMode::Max), (Some(7300), Some(7650)), 1000 + CALIBRATION_SETTLE_MS, known).unwrap();
    assert_eq!(learned, FanCalibration { cpu_max_rpm: Some(7300), gpu_max_rpm: Some(7650) });
    // No news when the peak does not improve on what is already stored.
    assert_eq!(c.observe(Some(FanMode::Max), (Some(7200), Some(7600)), 9000, learned), None);
    // A higher peak on one fan updates only that fan.
    let more = c.observe(Some(FanMode::Max), (Some(7400), None), 10_000, learned).unwrap();
    assert_eq!(more, FanCalibration { cpu_max_rpm: Some(7400), gpu_max_rpm: Some(7650) });
}

#[test]
fn leaving_max_mode_resets_the_settle_timer() {
    let mut c = Calibrator::default();
    let known = FanCalibration::default();
    c.observe(Some(FanMode::Max), (Some(7000), Some(7000)), 0, known);
    c.observe(Some(FanMode::Custom), (Some(7000), Some(7000)), 3000, known);
    assert_eq!(c.observe(Some(FanMode::Max), (Some(7000), Some(7000)), CALIBRATION_SETTLE_MS, known), None);
    assert!(c.observe(Some(FanMode::Max), (Some(7000), Some(7000)), 2 * CALIBRATION_SETTLE_MS, known).is_some());
    assert_eq!(c.observe(None, (Some(9000), Some(9000)), 3 * CALIBRATION_SETTLE_MS, known), None);
}

#[test]
fn zero_rpm_never_calibrates() {
    let mut c = Calibrator::default();
    let known = FanCalibration::default();
    c.observe(Some(FanMode::Max), (Some(0), None), 0, known);
    assert_eq!(c.observe(Some(FanMode::Max), (Some(0), None), CALIBRATION_SETTLE_MS, known), None);
}
