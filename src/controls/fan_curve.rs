//! @module controls::fan_curve
//! @description Custom fan-curve model: validation and temperature→duty interpolation.
//!
//! @input  Curve points (°C, duty %) from the curve editor or an IPC request.
//! @output Validated curves and duty percentages for the helper's curve controller.
//! @dependencies serde
//!
//! Safety rules enforced here, because a bad curve can defeat thermal management:
//! temperatures strictly increasing, duty never decreasing as temperature rises, duty inside
//! [`MIN_DUTY_PCT`, 100], and a hard override to 100 % at or above [`SAFETY_FULL_SPEED_C`].
use serde::{Deserialize, Serialize};

pub const MIN_POINTS: usize = 2;
pub const MAX_POINTS: usize = 8;
/// Firmware-safe duty floor; the curve can never ask for less airflow than this.
pub const MIN_DUTY_PCT: u8 = 20;
pub const MAX_DUTY_PCT: u8 = 100;
pub const MIN_TEMP_C: u8 = 20;
pub const MAX_TEMP_C: u8 = 100;
/// At or above this temperature the controller demands full speed regardless of the curve.
pub const SAFETY_FULL_SPEED_C: f32 = 90.0;
pub const MAX_NAME_LEN: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurvePoint {
    pub temp_c: u8,
    pub duty_pct: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanCurve {
    pub points: Vec<CurvePoint>,
}

/// A named pair of curves, one per fan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanCurveProfile {
    pub name: String,
    pub cpu: FanCurve,
    pub gpu: FanCurve,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurveError {
    TooFewPoints,
    TooManyPoints,
    TempOutOfRange { index: usize },
    DutyOutOfRange { index: usize },
    NonMonotonicTemp { index: usize },
    DecreasingDuty { index: usize },
    InvalidName,
}

impl std::fmt::Display for CurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewPoints => write!(f, "a curve needs at least {MIN_POINTS} points"),
            Self::TooManyPoints => write!(f, "a curve allows at most {MAX_POINTS} points"),
            Self::TempOutOfRange { index } => {
                write!(f, "point {} temperature must be {MIN_TEMP_C}-{MAX_TEMP_C} °C", index + 1)
            }
            Self::DutyOutOfRange { index } => {
                write!(f, "point {} duty must be {MIN_DUTY_PCT}-{MAX_DUTY_PCT} %", index + 1)
            }
            Self::NonMonotonicTemp { index } => {
                write!(f, "point {} must be hotter than the point before it", index + 1)
            }
            Self::DecreasingDuty { index } => {
                write!(f, "point {} must not lower the fan speed", index + 1)
            }
            Self::InvalidName => write!(f, "profile name must be 1-{MAX_NAME_LEN} printable characters"),
        }
    }
}

impl FanCurve {
    /// The example profile used as the default.
    pub fn default_curve() -> Self {
        let pts = [(40, 25), (50, 35), (60, 50), (70, 70), (80, 90), (90, 100)];
        Self { points: pts.iter().map(|&(temp_c, duty_pct)| CurvePoint { temp_c, duty_pct }).collect() }
    }

    pub fn validate(&self) -> Result<(), CurveError> {
        if self.points.len() < MIN_POINTS {
            return Err(CurveError::TooFewPoints);
        }
        if self.points.len() > MAX_POINTS {
            return Err(CurveError::TooManyPoints);
        }
        for (index, p) in self.points.iter().enumerate() {
            if !(MIN_TEMP_C..=MAX_TEMP_C).contains(&p.temp_c) {
                return Err(CurveError::TempOutOfRange { index });
            }
            if !(MIN_DUTY_PCT..=MAX_DUTY_PCT).contains(&p.duty_pct) {
                return Err(CurveError::DutyOutOfRange { index });
            }
            if index > 0 {
                let prev = self.points[index - 1];
                if p.temp_c <= prev.temp_c {
                    return Err(CurveError::NonMonotonicTemp { index });
                }
                if p.duty_pct < prev.duty_pct {
                    return Err(CurveError::DecreasingDuty { index });
                }
            }
        }
        Ok(())
    }

    /// Duty for a temperature: linear between points, clamped to the end points, full speed at
    /// or above the safety threshold. Assumes a validated curve.
    pub fn duty_at(&self, temp_c: f32) -> u8 {
        if !temp_c.is_finite() || temp_c >= SAFETY_FULL_SPEED_C {
            return MAX_DUTY_PCT;
        }
        let (first, last) = match (self.points.first(), self.points.last()) {
            (Some(f), Some(l)) => (*f, *l),
            _ => return MAX_DUTY_PCT,
        };
        if temp_c <= f32::from(first.temp_c) {
            return first.duty_pct.max(MIN_DUTY_PCT);
        }
        if temp_c >= f32::from(last.temp_c) {
            return last.duty_pct.max(MIN_DUTY_PCT);
        }
        for pair in self.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (ta, tb) = (f32::from(a.temp_c), f32::from(b.temp_c));
            if temp_c <= tb {
                let t = (temp_c - ta) / (tb - ta);
                let duty = f32::from(a.duty_pct) + t * (f32::from(b.duty_pct) - f32::from(a.duty_pct));
                return (duty.round() as u8).clamp(MIN_DUTY_PCT, MAX_DUTY_PCT);
            }
        }
        MAX_DUTY_PCT
    }
}

impl FanCurveProfile {
    pub fn default_profile() -> Self {
        Self { name: "Balanced custom".into(), cpu: FanCurve::default_curve(), gpu: FanCurve::default_curve() }
    }

    pub fn validate(&self) -> Result<(), CurveError> {
        let name_ok = !self.name.trim().is_empty()
            && self.name.chars().count() <= MAX_NAME_LEN
            && !self.name.chars().any(char::is_control);
        if !name_ok {
            return Err(CurveError::InvalidName);
        }
        self.cpu.validate()?;
        self.gpu.validate()
    }
}
