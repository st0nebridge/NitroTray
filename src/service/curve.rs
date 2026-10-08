//! @module service::curve
//! @description Custom fan-curve controller: maps temperatures to duty each tick, fail-closed.
//!
//! @input  A validated `FanCurveProfile` and a `HardwareBackend` to read temperatures / write duty.
//! @output `TickOutcome` telling the service core whether duty changed or it must restore Auto.
//! @dependencies acer::backend, controls::fan_curve
//!
//! Rules: rising temperatures take effect immediately; falling duty is limited to
//! `RAMP_DOWN_PER_TICK` per tick to avoid oscillation; a missing temperature is tolerated for
//! fewer than `MAX_READ_FAILURES` consecutive ticks, after which the core must fail safe.
use crate::acer::backend::HardwareBackend;
use crate::controls::FanCurveProfile;

pub const TICK_MS: u64 = 1000;
pub const RAMP_DOWN_PER_TICK: u8 = 5;
pub const MAX_READ_FAILURES: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickOutcome {
    Applied {
        cpu: u8,
        gpu: u8,
    },
    Unchanged,
    /// Temperatures unavailable this tick; still within the tolerance.
    Skipped,
    /// The core must restore firmware Auto mode.
    FailSafe(String),
}

#[derive(Debug, Clone)]
pub struct CurveController {
    profile: FanCurveProfile,
    applied: Option<(u8, u8)>,
    failures: u32,
}

/// Next duty given the previous one: up immediately, down at most `RAMP_DOWN_PER_TICK`.
pub fn ramp(previous: Option<u8>, wanted: u8) -> u8 {
    match previous {
        Some(p) if wanted < p => wanted.max(p.saturating_sub(RAMP_DOWN_PER_TICK)),
        _ => wanted,
    }
}

impl CurveController {
    pub fn new(profile: FanCurveProfile) -> Self {
        Self { profile, applied: None, failures: 0 }
    }

    pub fn profile(&self) -> &FanCurveProfile {
        &self.profile
    }

    /// Duty last written to the fans, if any.
    pub fn applied(&self) -> Option<(u8, u8)> {
        self.applied
    }

    pub fn target(&self, cpu_temp_c: f32, gpu_temp_c: f32) -> (u8, u8) {
        let cpu = ramp(self.applied.map(|a| a.0), self.profile.cpu.duty_at(cpu_temp_c));
        let gpu = ramp(self.applied.map(|a| a.1), self.profile.gpu.duty_at(gpu_temp_c));
        (cpu, gpu)
    }

    pub fn tick(&mut self, backend: &mut dyn HardwareBackend) -> TickOutcome {
        let temps = backend.sensors().ok().and_then(|r| Some((r.cpu_temp_c?, r.gpu_temp_c?)));
        let Some((cpu_t, gpu_t)) = temps else {
            self.failures += 1;
            return if self.failures >= MAX_READ_FAILURES {
                TickOutcome::FailSafe(format!("temperatures unavailable for {} ticks", self.failures))
            } else {
                TickOutcome::Skipped
            };
        };
        self.failures = 0;
        let target = self.target(cpu_t, gpu_t);
        if self.applied == Some(target) {
            return TickOutcome::Unchanged;
        }
        match backend.set_fan_duty(target.0, target.1) {
            Ok(()) => {
                self.applied = Some(target);
                TickOutcome::Applied { cpu: target.0, gpu: target.1 }
            }
            Err(e) => TickOutcome::FailSafe(format!("could not set fan duty: {e}")),
        }
    }
}
