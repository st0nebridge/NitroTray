//! @module telemetry::fan
//! @description Fan telemetry shaping and honest ring percentages.
//!
//! @input  RPM and commanded duty from the helper; fan mode; a clock.
//! @output `FanTelemetry`; ring percentage only from known duty or an RPM maximum measured in Max mode.
//! @dependencies serde, controls::FanMode, telemetry::snapshot
use serde::{Deserialize, Serialize};

use crate::controls::FanMode;
use crate::telemetry::snapshot::FanTelemetry;

/// Time Max mode must hold before its RPM counts as the fan's maximum.
pub const CALIBRATION_SETTLE_MS: u64 = 5000;

pub fn fan_telemetry(rpm: Option<u32>, duty_pct: Option<u8>) -> FanTelemetry {
    FanTelemetry { rpm, duty_pct: duty_pct.map(|d| f32::from(d.min(100))) }
}

/// RPM maxima observed in Max mode; persisted in settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FanCalibration {
    pub cpu_max_rpm: Option<u32>,
    pub gpu_max_rpm: Option<u32>,
}

/// Ring fill: known duty wins; otherwise RPM against a measured maximum; otherwise unknown.
pub fn ring_pct(fan: &FanTelemetry, measured_max_rpm: Option<u32>) -> Option<f32> {
    if let Some(d) = fan.duty_pct {
        return Some(d.clamp(0.0, 100.0));
    }
    match (fan.rpm, measured_max_rpm) {
        (Some(rpm), Some(max)) if max > 0 => Some((rpm as f32 / max as f32 * 100.0).clamp(0.0, 100.0)),
        _ => None,
    }
}

/// Watches Max mode and records the settled RPM peak of each fan.
#[derive(Debug, Clone, Default)]
pub struct Calibrator {
    max_since_ms: Option<u64>,
    peak: (u32, u32),
}

impl Calibrator {
    /// Feeds one observation; returns an updated calibration when a new maximum is learned.
    pub fn observe(
        &mut self,
        mode: Option<FanMode>,
        rpm: (Option<u32>, Option<u32>),
        now_ms: u64,
        current: FanCalibration,
    ) -> Option<FanCalibration> {
        if mode != Some(FanMode::Max) {
            self.max_since_ms = None;
            self.peak = (0, 0);
            return None;
        }
        let since = *self.max_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(since) < CALIBRATION_SETTLE_MS {
            return None;
        }
        self.peak.0 = self.peak.0.max(rpm.0.unwrap_or(0));
        self.peak.1 = self.peak.1.max(rpm.1.unwrap_or(0));
        let better = |peak: u32, known: Option<u32>| (peak > 0 && known.is_none_or(|k| peak > k)).then_some(peak);
        let (cpu, gpu) = (better(self.peak.0, current.cpu_max_rpm), better(self.peak.1, current.gpu_max_rpm));
        (cpu.is_some() || gpu.is_some()).then(|| FanCalibration {
            cpu_max_rpm: cpu.or(current.cpu_max_rpm),
            gpu_max_rpm: gpu.or(current.gpu_max_rpm),
        })
    }
}
