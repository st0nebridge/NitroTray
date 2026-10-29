//! @module service::unlock_log
//! @description The helper's one file read: the newest boot log of the UEFI tensor unlocker, reduced to a
//! typed `BootReport`. The location is fixed (`\GPU4090-UNLOCK\RUNnnnn.TXT` on the EFI system partition),
//! nothing is written, the request names no path, and the log text never leaves this module.
//!
//! @input  A directory of run logs (the real one is found by `service::esp`).
//! @output `Option<BootReport>`; no log at all is an answer, an unreadable one an error.
//! @dependencies unlock::boot_log
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::unlock::boot_log::{is_run_log_name, parse, BootReport};

/// The unlocker's directory on the EFI system partition.
pub const LOG_DIR: &str = "GPU4090-UNLOCK";
/// A log is about 16 KiB and its facts sit throughout; nothing past this is read.
pub const MAX_LOG_BYTES: u64 = 256 * 1024;

/// Where the service gets the unlocker's newest boot report from.
pub trait UnlockLogSource: Send {
    fn latest(&mut self) -> Result<Option<BootReport>, String>;
}

/// A fixed answer, for the simulated helper and tests.
pub struct StaticUnlockLog(pub Option<BootReport>);

impl UnlockLogSource for StaticUnlockLog {
    fn latest(&mut self) -> Result<Option<BootReport>, String> {
        Ok(self.0.clone())
    }
}

/// The report of the highest-numbered `RUN<digits>.TXT` in `dir`; a missing directory or no logs is `None`.
pub fn latest_in_dir(dir: &Path) -> Result<Option<BootReport>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("could not list the unlocker's logs: {e}")),
    };
    let newest = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let number: u64 = name.get(3..name.len().checked_sub(4)?)?.parse().ok()?;
            is_run_log_name(&name).then_some((number, name))
        })
        .max();
    let Some((_, name)) = newest else { return Ok(None) };
    let path = dir.join(&name);
    let mut text = Vec::new();
    std::fs::File::open(&path)
        .and_then(|f| f.take(MAX_LOG_BYTES).read_to_end(&mut text))
        .map_err(|e| format!("could not read the unlocker's log {name}: {e}"))?;
    let written = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    Ok(Some(parse(&name, &String::from_utf8_lossy(&text), written)))
}
