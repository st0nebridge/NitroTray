//! @module telemetry::scheduler
//! @description Poll scheduler: per-metric cadences that depend on UI visibility.
//!
//! @input  Refresh preset, background interval, whether a UI surface or graph icon needs data, and a clock.
//! @output Which metric groups are due now, and how long until the next one is.
//! @dependencies serde
//!
//! Popup open: utilisation 500 ms, clocks/temperatures/fans 1000 ms, modes 3 s (Normal preset).
//! Popup closed: temperatures and fans only for the graph icons at the background interval,
//! modes every 5 s for the tray menu, and utilisation/clocks (and NVML) suspended.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RefreshRate {
    Fast,
    #[default]
    Normal,
    Relaxed,
}

pub const MIN_BACKGROUND_MS: u64 = 1000;
pub const MAX_BACKGROUND_MS: u64 = 5000;
const CLOSED_MODES_MS: u64 = 5000;

/// Interval per metric group; `None` means suspended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cadence {
    pub utilisation_ms: Option<u64>,
    pub clocks_ms: Option<u64>,
    pub temperatures_ms: Option<u64>,
    pub fans_ms: Option<u64>,
    pub modes_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DueSet {
    pub utilisation: bool,
    pub clocks: bool,
    pub temperatures: bool,
    pub fans: bool,
    pub modes: bool,
}

impl DueSet {
    pub fn any(&self) -> bool {
        self.utilisation || self.clocks || self.temperatures || self.fans || self.modes
    }

    pub fn all() -> Self {
        Self { utilisation: true, clocks: true, temperatures: true, fans: true, modes: true }
    }
}

/// Computes cadences for the current visibility state.
pub fn cadence(rate: RefreshRate, background_ms: u64, ui_visible: bool, graph_icons: bool) -> Cadence {
    if ui_visible {
        let (util, clocks, temps, fans, modes) = match rate {
            RefreshRate::Fast => (500, 500, 500, 500, 2000),
            RefreshRate::Normal => (500, 1000, 1000, 1000, 3000),
            RefreshRate::Relaxed => (1000, 2000, 2000, 2000, 5000),
        };
        return Cadence {
            utilisation_ms: Some(util),
            clocks_ms: Some(clocks),
            temperatures_ms: Some(temps),
            fans_ms: Some(fans),
            modes_ms: Some(modes),
        };
    }
    let bg = background_ms.clamp(MIN_BACKGROUND_MS, MAX_BACKGROUND_MS);
    Cadence {
        utilisation_ms: None,
        clocks_ms: None,
        temperatures_ms: graph_icons.then_some(bg),
        fans_ms: graph_icons.then_some(bg),
        modes_ms: Some(CLOSED_MODES_MS),
    }
}

/// Tracks when each metric group last ran.
#[derive(Debug, Clone, Default)]
pub struct PollScheduler {
    last: [Option<u64>; 5],
}

impl PollScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget history so everything is due immediately (e.g. when the popup opens).
    pub fn reset(&mut self) {
        self.last = [None; 5];
    }

    fn intervals(c: &Cadence) -> [Option<u64>; 5] {
        [c.utilisation_ms, c.clocks_ms, c.temperatures_ms, c.fans_ms, c.modes_ms]
    }

    fn is_due(last: Option<u64>, interval: Option<u64>, now: u64) -> bool {
        match (interval, last) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(i), Some(l)) => now.saturating_sub(l) >= i,
        }
    }

    /// Marks and returns the groups due at `now_ms`.
    pub fn due(&mut self, now_ms: u64, c: &Cadence) -> DueSet {
        let intervals = Self::intervals(c);
        let mut flags = [false; 5];
        for (i, interval) in intervals.iter().enumerate() {
            if Self::is_due(self.last[i], *interval, now_ms) {
                flags[i] = true;
                self.last[i] = Some(now_ms);
            }
        }
        DueSet { utilisation: flags[0], clocks: flags[1], temperatures: flags[2], fans: flags[3], modes: flags[4] }
    }

    /// Milliseconds until the next group is due; `None` when everything is suspended.
    pub fn next_wake_ms(&self, now_ms: u64, c: &Cadence) -> Option<u64> {
        Self::intervals(c)
            .iter()
            .zip(self.last.iter())
            .filter_map(|(interval, last)| {
                let i = (*interval)?;
                Some(match last {
                    None => 0,
                    Some(l) => (l + i).saturating_sub(now_ms),
                })
            })
            .min()
    }
}
