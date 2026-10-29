//! @module unlock::view
//! @description What the popup and the details popup say about the tensor unlock: one verdict and its
//! wording, from the last live measurement and the unlocker's boot log. A reading never outlives its
//! evidence: nothing measured is "unknown", never "locked".
//!
//! @input  `UnlockInputs`: last live check, probe error, whether a check is running, the newest boot
//! report, the session's start time and whether driver queries are allowed.
//! @output `UnlockView`, serialised for the pages.
//! @dependencies serde, unlock::{state, boot_log}
use serde::Serialize;

use super::boot_log::{BootReport, BootStatus};
use super::state::{TensorRates, TensorState};

/// The unlocker runs before Windows starts, so its log predates the session by the firmware and boot time.
pub const SESSION_GRACE_S: u64 = 15 * 60;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveCheck {
    pub rates: TensorRates,
    pub state: TensorState,
    /// Seconds since the measurement.
    pub age_s: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UnlockInputs {
    /// GPU driver queries are allowed (Settings › Telemetry).
    pub enabled: bool,
    /// The last successful measurement.
    pub live: Option<LiveCheck>,
    /// Why the last measurement failed, if it did.
    pub probe_error: Option<String>,
    pub checking: bool,
    pub boot: Option<BootReport>,
    /// When this Windows session started, seconds since the Unix epoch.
    pub session_start_unix: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewState {
    Unlocked,
    Locked,
    /// Locked now, although the unlocker's log for this session says it unlocked at boot.
    Lost,
    Checking,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnlockView {
    pub state: ViewState,
    /// Short label for the popup's badge; empty when there is nothing to claim.
    pub badge: String,
    pub detail: String,
    /// The reading in a few words ("ratio 0.50, checked 12 s ago"); empty without one.
    pub summary: String,
    pub ratio: Option<f32>,
    pub age_s: Option<u64>,
    /// What the unlocker's boot log says, when there is one.
    pub boot: Option<String>,
}

/// Whether the log was written in this session; `None` when either time is missing.
fn this_session(boot: &BootReport, session_start: Option<u64>) -> Option<bool> {
    Some(boot.written_unix? + SESSION_GRACE_S >= session_start?)
}

fn ago(age_s: u64) -> String {
    match age_s {
        0 => "just now".into(),
        1..=59 => format!("{age_s} s ago"),
        60..=3599 => format!("{} min ago", age_s / 60),
        _ => format!("{} h ago", age_s / 3600),
    }
}

fn boot_line(boot: &BootReport, session_start: Option<u64>) -> String {
    let what = match boot.status {
        BootStatus::Unlocked => "unlocked at boot",
        BootStatus::DryRun => "dry run, nothing written",
        BootStatus::Skipped => "skipped, nothing written",
        BootStatus::Stopped => "stopped",
        BootStatus::NotUnlocked => "did not unlock",
    };
    let stale = if this_session(boot, session_start) == Some(false) { " (before this session)" } else { "" };
    format!("Unlocker run {}: {what}{stale}", boot.run)
}

pub fn view(i: &UnlockInputs) -> UnlockView {
    let boot = i.boot.as_ref().map(|b| boot_line(b, i.session_start_unix));
    let unknown = |state: ViewState, badge: &str, detail: String| UnlockView {
        state,
        badge: badge.into(),
        detail,
        summary: String::new(),
        ratio: None,
        age_s: None,
        boot: boot.clone(),
    };
    if !i.enabled {
        return unknown(
            ViewState::Unknown,
            "",
            "Tensor rate checks are off (Settings › Telemetry › GPU driver telemetry).".into(),
        );
    }
    let Some(live) = i.live else {
        return match (&i.probe_error, i.checking) {
            (_, true) => unknown(ViewState::Checking, "Checking", "Checking the tensor-core rate…".into()),
            (Some(why), false) => {
                unknown(ViewState::Unknown, "", format!("The tensor-core rate could not be checked: {why}."))
            }
            (None, false) => unknown(ViewState::Unknown, "", "Not checked yet.".into()),
        };
    };
    let ratio = live.rates.ratio();
    let r = ratio.map_or_else(|| "?".to_string(), |r| format!("{r:.2}"));
    let when = ago(live.age_s);
    let unlocked_at_boot = i
        .boot
        .as_ref()
        .is_some_and(|b| b.status == BootStatus::Unlocked && this_session(b, i.session_start_unix) != Some(false));
    let (state, badge, mut detail) = match live.state {
        TensorState::Unlocked => (
            ViewState::Unlocked,
            "Unlocked",
            format!("FP32-accumulate tensor rate is at full rate (ratio {r}), checked {when}."),
        ),
        TensorState::Locked if unlocked_at_boot => (
            ViewState::Lost,
            "Unlock lost",
            format!(
                "Unlocked at boot ({}) but at half rate now (ratio {r}), checked {when}: the RTX was reset after the unlock.",
                i.boot.as_ref().map_or("", |b| b.run.as_str())
            ),
        ),
        TensorState::Locked => (
            ViewState::Locked,
            "Locked",
            format!("FP32-accumulate tensor rate is at half rate (ratio {r}), checked {when}."),
        ),
    };
    if let Some(why) = &i.probe_error {
        detail.push_str(&format!(" Last check failed: {why}."));
    }
    if i.checking {
        detail.push_str(" Checking again.");
    }
    UnlockView {
        state,
        badge: badge.into(),
        detail,
        summary: format!("ratio {r}, checked {when}"),
        ratio,
        age_s: Some(live.age_s),
        boot,
    }
}
