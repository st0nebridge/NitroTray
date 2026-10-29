//! @module unlock::boot_log
//! @description Reads the UEFI unlocker's boot log (`RUNnnnn.TXT` on the EFI system partition) down to the
//! facts that say whether that boot lifted the throttle: the build, the final selector readback, the
//! transaction result, and whether the run stopped or skipped.
//!
//! @input  A log's file name and text, and when it was written.
//! @output `BootReport`: run name, status, build and write time (the log text itself is never passed on).
//! @dependencies serde
use serde::{Deserialize, Serialize};

/// Rate selectors the unlocker reads back after a successful unlock.
const UNLOCKED_SS0: u64 = 0x8888_8888;
const UNLOCKED_SS1: u64 = 0x8;
/// Builds that run every check but never write to the GPU.
const DRY_RUN_BUILDS: [&str; 2] = ["dryrun", "reltest"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootStatus {
    Unlocked,
    DryRun,
    Skipped,
    Stopped,
    NotUnlocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootReport {
    /// The log's name without its extension, e.g. "RUN0028".
    pub run: String,
    pub status: BootStatus,
    /// The unlocker build that ran ("auto", "dryrun", ...); empty when the log does not say.
    pub build: String,
    /// When the log was written, seconds since the Unix epoch.
    pub written_unix: Option<u64>,
}

/// True for `RUN<digits>.TXT` in any case, and nothing else.
pub fn is_run_log_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper
        .strip_prefix("RUN")
        .and_then(|rest| rest.strip_suffix(".TXT"))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn run_name(file_name: &str) -> &str {
    let stem = file_name.len().checked_sub(4).filter(|&at| file_name.is_char_boundary(at));
    match stem {
        Some(at) if file_name[at..].eq_ignore_ascii_case(".txt") => &file_name[..at],
        _ => file_name,
    }
}

/// The build named by the first `build=<word>` that starts a word.
fn build_of(text: &str) -> &str {
    text.match_indices("build=")
        .find(|(at, _)| !text[..*at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_'))
        .map(|(at, key)| {
            let rest = &text[at + key.len()..];
            let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
            &rest[..end]
        })
        .unwrap_or("")
}

/// `SS0`/`SS1` of the first line that starts `POST-GOP SS0=0x<hex> SS1=0x<hex>`.
fn post_gop(text: &str) -> Option<(u64, u64)> {
    let line = text.lines().find_map(|l| l.strip_prefix("POST-GOP SS0=0x"))?;
    let (ss0, ss1) = line.split_once(" SS1=0x")?;
    let ss1: String = ss1.chars().take_while(char::is_ascii_hexdigit).collect();
    Some((u64::from_str_radix(ss0, 16).ok()?, u64::from_str_radix(&ss1, 16).ok()?))
}

pub fn parse(file_name: &str, text: &str, written_unix: Option<u64>) -> BootReport {
    let build = build_of(text);
    let lines = || text.lines().map(str::trim).filter(|l| !l.is_empty());
    let stopped = lines().any(|l| l.starts_with("STOP:") || l.contains("STOPPED"));
    let transaction_ok = lines().any(|l| l.starts_with("TRANSACTION OK"));
    let skipped = text.lines().any(|l| l.starts_with("SKIP: stage="));
    let status = if stopped {
        BootStatus::Stopped
    } else if transaction_ok && post_gop(text) == Some((UNLOCKED_SS0, UNLOCKED_SS1)) {
        BootStatus::Unlocked
    } else if skipped {
        BootStatus::Skipped
    } else if DRY_RUN_BUILDS.contains(&build) {
        BootStatus::DryRun
    } else {
        BootStatus::NotUnlocked
    };
    BootReport { run: run_name(file_name).to_string(), status, build: build.to_string(), written_unix }
}
