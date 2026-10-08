//! @module curve_test
//! @description Curve controller: ramping, unchanged detection, and fail-closed behaviour (D-20260930-007).
use nitrotray::acer::backend::AcerError;
use nitrotray::controls::{CurvePoint, FanCurve, FanCurveProfile};
use nitrotray::service::curve::{ramp, CurveController, TickOutcome, MAX_READ_FAILURES, RAMP_DOWN_PER_TICK};

use crate::support::{readings, FakeBackend};

fn flat_profile(duty_low: u8, duty_high: u8) -> FanCurveProfile {
    let c = FanCurve {
        points: vec![CurvePoint { temp_c: 40, duty_pct: duty_low }, CurvePoint { temp_c: 80, duty_pct: duty_high }],
    };
    FanCurveProfile { name: "t".into(), cpu: c.clone(), gpu: c }
}

#[test]
fn ramp_rises_immediately_and_falls_gradually() {
    assert_eq!(ramp(None, 30), 30);
    assert_eq!(ramp(Some(30), 80), 80);
    assert_eq!(ramp(Some(80), 30), 80 - RAMP_DOWN_PER_TICK);
    assert_eq!(ramp(Some(33), 30), 30);
    assert_eq!(ramp(Some(3), 0), 0);
}

#[test]
fn applies_duty_from_temperatures_then_reports_unchanged() {
    let mut b = FakeBackend::default();
    b.default_sensors = Ok(readings(60.0, 40.0));
    let mut c = CurveController::new(flat_profile(20, 100));
    assert_eq!(c.tick(&mut b), TickOutcome::Applied { cpu: 60, gpu: 20 });
    assert_eq!(c.applied(), Some((60, 20)));
    assert_eq!(c.tick(&mut b), TickOutcome::Unchanged);
    assert_eq!(b.duties, vec![(60, 20)]);
    assert_eq!(c.profile().name, "t");
}

#[test]
fn cooling_down_steps_the_duty_down() {
    let mut b = FakeBackend::default();
    b.sensors.push_back(Ok(readings(80.0, 80.0)));
    b.default_sensors = Ok(readings(40.0, 40.0));
    let mut c = CurveController::new(flat_profile(20, 100));
    assert_eq!(c.tick(&mut b), TickOutcome::Applied { cpu: 100, gpu: 100 });
    assert_eq!(c.tick(&mut b), TickOutcome::Applied { cpu: 95, gpu: 95 });
    assert_eq!(c.target(40.0, 40.0), (90, 90));
}

#[test]
fn tolerates_brief_sensor_gaps_then_fails_safe() {
    let mut b = FakeBackend::default();
    b.default_sensors = Err(AcerError::Firmware(1));
    let mut c = CurveController::new(flat_profile(20, 100));
    for _ in 1..MAX_READ_FAILURES {
        assert_eq!(c.tick(&mut b), TickOutcome::Skipped);
    }
    assert!(matches!(c.tick(&mut b), TickOutcome::FailSafe(m) if m.contains("unavailable")));
}

#[test]
fn a_good_reading_resets_the_failure_count() {
    let mut b = FakeBackend::default();
    b.sensors.push_back(Err(AcerError::Firmware(1)));
    b.sensors.push_back(Err(AcerError::Firmware(1)));
    b.sensors.push_back(Ok(readings(50.0, 50.0)));
    b.sensors.push_back(Err(AcerError::Firmware(1)));
    b.sensors.push_back(Err(AcerError::Firmware(1)));
    let mut c = CurveController::new(flat_profile(20, 100));
    assert_eq!(c.tick(&mut b), TickOutcome::Skipped);
    assert_eq!(c.tick(&mut b), TickOutcome::Skipped);
    assert!(matches!(c.tick(&mut b), TickOutcome::Applied { .. }));
    assert_eq!(c.tick(&mut b), TickOutcome::Skipped);
    assert_eq!(c.tick(&mut b), TickOutcome::Skipped);
}

#[test]
fn a_missing_single_temperature_counts_as_failure() {
    let mut b = FakeBackend::default();
    let mut r = readings(50.0, 50.0);
    r.gpu_temp_c = None;
    b.default_sensors = Ok(r);
    let mut c = CurveController::new(flat_profile(20, 100));
    assert_eq!(c.tick(&mut b), TickOutcome::Skipped);
}

#[test]
fn duty_write_failure_fails_safe() {
    let mut b = FakeBackend::default();
    b.duty_error = Some(AcerError::Firmware(2));
    let mut c = CurveController::new(flat_profile(20, 100));
    assert!(matches!(c.tick(&mut b), TickOutcome::FailSafe(m) if m.contains("could not set fan duty")));
    assert_eq!(c.applied(), None);
}
