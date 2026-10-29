//! @module unlock::tracker
//! @description The app's memory of the tensor check: the last reading and the last failure, the newest
//! boot report, whether a check is running and when the next is due. Pure: the clock is passed in.
//!
//! @input  Finished checks (a measurement and the helper's boot report), the session start, a ms clock.
//! @output `UnlockView` for the pages, and whether another check should start.
//! @dependencies unlock::{state, boot_log, view}
use super::boot_log::BootReport;
use super::state::{classify, TensorRates, TensorState};
use super::view::{view, LiveCheck, UnlockInputs, UnlockView};

/// A reading is trusted this long; opening a window later than this starts a new check.
pub const STALE_MS: u64 = 60_000;

#[derive(Debug, Clone, Default)]
pub struct UnlockTracker {
    live: Option<(TensorRates, TensorState, u64)>,
    probe_error: Option<String>,
    boot: Option<BootReport>,
    checking: bool,
    /// When the last check finished, whether it worked or not.
    attempted_ms: Option<u64>,
    session_start_unix: Option<u64>,
}

impl UnlockTracker {
    pub fn set_session_start(&mut self, unix: Option<u64>) {
        self.session_start_unix = unix;
    }

    /// Whether a check should start now: never while one runs, otherwise when asked or when the last is stale.
    pub fn due(&self, now_ms: u64, force: bool) -> bool {
        !self.checking && (force || self.attempted_ms.is_none_or(|at| now_ms.saturating_sub(at) >= STALE_MS))
    }

    pub fn begin(&mut self) {
        self.checking = true;
    }

    /// Records a finished check. A failed measurement keeps the last good one; an unreachable helper keeps
    /// the last boot report, while a helper that found no log clears it.
    pub fn finish(
        &mut self,
        rates: Result<TensorRates, String>,
        boot: Result<Option<BootReport>, String>,
        now_ms: u64,
    ) {
        self.checking = false;
        self.attempted_ms = Some(now_ms);
        match rates
            .and_then(|r| classify(r).map(|state| (r, state)).ok_or_else(|| "the GPU reported no throughput".into()))
        {
            Ok((rates, state)) => {
                self.live = Some((rates, state, now_ms));
                self.probe_error = None;
            }
            Err(why) => self.probe_error = Some(why),
        }
        if let Ok(report) = boot {
            self.boot = report;
        }
    }

    pub fn view(&self, enabled: bool, now_ms: u64) -> UnlockView {
        view(&UnlockInputs {
            enabled,
            live: self.live.map(|(rates, state, at)| LiveCheck {
                rates,
                state,
                age_s: now_ms.saturating_sub(at) / 1000,
            }),
            probe_error: self.probe_error.clone(),
            checking: self.checking,
            boot: self.boot.clone(),
            session_start_unix: self.session_start_unix,
        })
    }
}
