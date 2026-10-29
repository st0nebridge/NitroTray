//! @module service::esp
//! @description Finds the unlocker's log directory on the EFI system partition without mounting anything:
//! the partition has no drive letter, but Windows gives every volume a GUID path the service (LocalSystem)
//! can read. Only FAT volumes are looked at, and only for the one directory.
//!
//! @input  None; the volumes are enumerated on each call (the partition is found by its content, not its
//! position or type GUID, so a moved or re-created partition still works).
//! @output `UnlockLogSource` over the first FAT volume that holds `\GPU4090-UNLOCK\`.
//! @dependencies windows (Storage::FileSystem), service::unlock_log
use std::path::PathBuf;

use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetVolumeInformationW};

use crate::service::unlock_log::{latest_in_dir, UnlockLogSource, LOG_DIR};
use crate::unlock::boot_log::BootReport;

/// `\\?\Volume{GUID}\` is 49 characters; this leaves room for the terminator.
const VOLUME_NAME_CHARS: usize = 64;

fn text_of(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

fn is_fat(volume: &str) -> bool {
    let wide: Vec<u16> = volume.encode_utf16().chain(Some(0)).collect();
    let mut fs = [0u16; 32];
    // SAFETY: a read-only query of a volume root, with a NUL-terminated name and a local output buffer.
    let ok = unsafe { GetVolumeInformationW(PCWSTR(wide.as_ptr()), None, None, None, None, Some(&mut fs)) };
    ok.is_ok() && text_of(&fs).to_ascii_uppercase().starts_with("FAT")
}

/// The log directory on the first FAT volume that has one.
pub fn find_log_dir() -> Result<Option<PathBuf>, String> {
    let mut name = [0u16; VOLUME_NAME_CHARS];
    // SAFETY: the volume search handle is used only here and closed on every path out.
    unsafe {
        let search = FindFirstVolumeW(&mut name).map_err(|e| format!("could not list the volumes: {e}"))?;
        let mut found = None;
        loop {
            let volume = text_of(&name);
            if found.is_none() && is_fat(&volume) {
                let dir = PathBuf::from(&volume).join(LOG_DIR);
                if dir.is_dir() {
                    found = Some(dir);
                }
            }
            if found.is_some() || FindNextVolumeW(search, &mut name).is_err() {
                break;
            }
        }
        let _ = FindVolumeClose(search);
        Ok(found)
    }
}

/// The newest boot report of the unlocker, read from the EFI system partition.
pub struct EspUnlockLog;

impl UnlockLogSource for EspUnlockLog {
    fn latest(&mut self) -> Result<Option<BootReport>, String> {
        match find_log_dir()? {
            Some(dir) => latest_in_dir(&dir),
            None => Ok(None),
        }
    }
}
