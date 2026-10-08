//! @module fan_curve_test
//! @description Fan-curve validation and interpolation — the safety rules.
use nitrotray::controls::fan_curve::{MAX_POINTS, MIN_DUTY_PCT, SAFETY_FULL_SPEED_C};
use nitrotray::controls::{CurveError, CurvePoint, FanCurve, FanCurveProfile};

fn curve(points: &[(u8, u8)]) -> FanCurve {
    FanCurve { points: points.iter().map(|&(temp_c, duty_pct)| CurvePoint { temp_c, duty_pct }).collect() }
}

#[test]
fn default_curve_is_the_example_profile_and_valid() {
    let c = FanCurve::default_curve();
    assert_eq!(c.points.len(), 6);
    assert_eq!(c.points[0], CurvePoint { temp_c: 40, duty_pct: 25 });
    assert_eq!(c.points[5], CurvePoint { temp_c: 90, duty_pct: 100 });
    assert_eq!(c.validate(), Ok(()));
    assert_eq!(FanCurveProfile::default_profile().validate(), Ok(()));
}

#[test]
fn rejects_point_count_violations() {
    assert_eq!(curve(&[(50, 50)]).validate(), Err(CurveError::TooFewPoints));
    let many: Vec<(u8, u8)> = (0..=MAX_POINTS as u8).map(|i| (30 + i * 5, 30 + i)).collect();
    assert_eq!(curve(&many).validate(), Err(CurveError::TooManyPoints));
    let max: Vec<(u8, u8)> = (0..MAX_POINTS as u8).map(|i| (30 + i * 5, 30 + i)).collect();
    assert_eq!(curve(&max).validate(), Ok(()));
}

#[test]
fn rejects_out_of_range_values() {
    assert_eq!(curve(&[(19, 30), (60, 50)]).validate(), Err(CurveError::TempOutOfRange { index: 0 }));
    assert_eq!(curve(&[(20, 30), (101, 50)]).validate(), Err(CurveError::TempOutOfRange { index: 1 }));
    assert_eq!(curve(&[(20, 30), (100, 50)]).validate(), Ok(()));
    assert_eq!(curve(&[(40, MIN_DUTY_PCT - 1), (60, 50)]).validate(), Err(CurveError::DutyOutOfRange { index: 0 }));
    assert_eq!(curve(&[(40, 30), (60, 101)]).validate(), Err(CurveError::DutyOutOfRange { index: 1 }));
    assert_eq!(curve(&[(40, MIN_DUTY_PCT), (60, 100)]).validate(), Ok(()));
}

#[test]
fn rejects_non_monotonic_curves() {
    assert_eq!(curve(&[(50, 30), (50, 40)]).validate(), Err(CurveError::NonMonotonicTemp { index: 1 }));
    assert_eq!(curve(&[(50, 30), (45, 40)]).validate(), Err(CurveError::NonMonotonicTemp { index: 1 }));
    assert_eq!(curve(&[(40, 60), (60, 50)]).validate(), Err(CurveError::DecreasingDuty { index: 1 }));
    assert_eq!(curve(&[(40, 50), (60, 50)]).validate(), Ok(()));
}

#[test]
fn interpolates_linearly_and_clamps_to_end_points() {
    let c = curve(&[(40, 30), (60, 50), (80, 90)]);
    assert_eq!(c.duty_at(20.0), 30);
    assert_eq!(c.duty_at(40.0), 30);
    assert_eq!(c.duty_at(50.0), 40);
    assert_eq!(c.duty_at(60.0), 50);
    assert_eq!(c.duty_at(70.0), 70);
    assert_eq!(c.duty_at(75.0), 80);
    assert_eq!(c.duty_at(80.0), 90);
    assert_eq!(c.duty_at(85.0), 90);
}

#[test]
fn safety_override_forces_full_speed() {
    let c = curve(&[(40, 30), (60, 40)]);
    assert_eq!(c.duty_at(SAFETY_FULL_SPEED_C - 0.5), 40);
    assert_eq!(c.duty_at(SAFETY_FULL_SPEED_C), 100);
    assert_eq!(c.duty_at(99.0), 100);
    assert_eq!(c.duty_at(f32::NAN), 100);
    assert_eq!(FanCurve { points: vec![] }.duty_at(50.0), 100);
}

#[test]
fn duty_floor_applies_even_to_unvalidated_curves() {
    let c = curve(&[(40, 5), (60, 10)]);
    assert_eq!(c.duty_at(30.0), MIN_DUTY_PCT);
    assert_eq!(c.duty_at(50.0), MIN_DUTY_PCT);
    assert_eq!(c.duty_at(70.0), MIN_DUTY_PCT);
}

#[test]
fn profile_name_rules() {
    let mut p = FanCurveProfile::default_profile();
    p.name = "   ".into();
    assert_eq!(p.validate(), Err(CurveError::InvalidName));
    p.name = "x".repeat(41);
    assert_eq!(p.validate(), Err(CurveError::InvalidName));
    p.name = "x".repeat(40);
    assert_eq!(p.validate(), Ok(()));
    p.name = "bad\nname".into();
    assert_eq!(p.validate(), Err(CurveError::InvalidName));
    p.name = "ok".into();
    p.gpu = curve(&[(50, 50)]);
    assert_eq!(p.validate(), Err(CurveError::TooFewPoints));
}

#[test]
fn errors_render_readable_messages() {
    let msgs = [
        CurveError::TooFewPoints.to_string(),
        CurveError::TooManyPoints.to_string(),
        CurveError::TempOutOfRange { index: 0 }.to_string(),
        CurveError::DutyOutOfRange { index: 1 }.to_string(),
        CurveError::NonMonotonicTemp { index: 2 }.to_string(),
        CurveError::DecreasingDuty { index: 3 }.to_string(),
        CurveError::InvalidName.to_string(),
    ];
    assert!(msgs[0].contains("at least 2"));
    assert!(msgs[1].contains("at most 8"));
    assert!(msgs[2].starts_with("point 1"));
    assert!(msgs[3].starts_with("point 2") && msgs[3].contains("20-100"));
    assert!(msgs[4].starts_with("point 3") && msgs[4].contains("hotter"));
    assert!(msgs[5].starts_with("point 4") && msgs[5].contains("lower"));
    assert!(msgs[6].contains("1-40"));
}

#[test]
fn deserialisation_rejects_unknown_fields() {
    let bad = r#"{"temp_c":40,"duty_pct":30,"extra":1}"#;
    assert!(serde_json::from_str::<CurvePoint>(bad).is_err());
}
