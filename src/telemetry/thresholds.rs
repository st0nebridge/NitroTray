//! @module telemetry::thresholds
//! @description Configurable temperature colour bands: normal / warm / hot / critical.
//!
//! @input  A temperature in °C and a threshold set.
//! @output A `TempLevel` used by the UI to colour the reading; sanitised threshold sets.
//! @dependencies serde
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TempLevel {
    Unknown,
    Normal,
    Warm,
    Hot,
    Critical,
}

/// Lower bounds (inclusive) of the warm, hot and critical bands.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TempThresholds {
    pub warm_c: f32,
    pub hot_c: f32,
    pub critical_c: f32,
}

pub const MIN_THRESHOLD_C: f32 = 30.0;
pub const MAX_THRESHOLD_C: f32 = 110.0;

impl TempThresholds {
    /// CPU defaults: < 70 normal, 70–84 warm, 85–94 red, ≥ 95 critical.
    pub const CPU_DEFAULT: Self = Self { warm_c: 70.0, hot_c: 85.0, critical_c: 95.0 };
    /// GPU defaults: < 70 normal, 70–79 warm, 80–89 red, ≥ 90 critical.
    pub const GPU_DEFAULT: Self = Self { warm_c: 70.0, hot_c: 80.0, critical_c: 90.0 };

    pub fn classify(&self, temp_c: Option<f32>) -> TempLevel {
        match temp_c {
            Some(t) if t.is_finite() => {
                if t >= self.critical_c {
                    TempLevel::Critical
                } else if t >= self.hot_c {
                    TempLevel::Hot
                } else if t >= self.warm_c {
                    TempLevel::Warm
                } else {
                    TempLevel::Normal
                }
            }
            _ => TempLevel::Unknown,
        }
    }

    pub fn is_valid(&self) -> bool {
        let in_range = |v: f32| v.is_finite() && (MIN_THRESHOLD_C..=MAX_THRESHOLD_C).contains(&v);
        in_range(self.warm_c)
            && in_range(self.hot_c)
            && in_range(self.critical_c)
            && self.warm_c < self.hot_c
            && self.hot_c < self.critical_c
    }

    /// Returns `self` when valid, otherwise the given fallback.
    pub fn or_default(self, fallback: Self) -> Self {
        if self.is_valid() {
            self
        } else {
            fallback
        }
    }
}
