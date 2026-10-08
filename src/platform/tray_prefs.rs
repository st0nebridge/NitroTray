//! @module platform::tray_prefs
//! @description Windows 11 notification-area preferences: promote this app's tray icons out of the
//! overflow flyout (HKCU\Control Panel\NotifyIconSettings\<id>\IsPromoted), on explicit user request only.
//!
//! @input  The executable path whose icons should be promoted; a registry root (overridable for tests).
//! @output Number of icon entries promoted.
//! @dependencies windows (Registry), platform::windows
use windows::core::{HSTRING, PWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_ENUMERATE_SUB_KEYS, KEY_READ,
};

use super::windows::{read_string, write_dword, Hive};

pub const NOTIFY_ROOT: &str = r"Control Panel\NotifyIconSettings";

/// Case-insensitive path comparison, tolerant of `/` vs `\`.
pub fn same_exe(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.trim().replace('/', "\\").to_ascii_lowercase();
    !a.trim().is_empty() && norm(a) == norm(b)
}

fn subkeys(root: &str) -> Vec<String> {
    let mut out = Vec::new();
    // SAFETY: enumerates subkey names of a key opened for enumeration and closes it.
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(root), None, KEY_READ | KEY_ENUMERATE_SUB_KEYS, &mut key)
            .is_err()
        {
            return out;
        }
        let mut index = 0u32;
        loop {
            let mut buf = [0u16; 256];
            let mut len = buf.len() as u32;
            let rc = RegEnumKeyExW(key, index, Some(PWSTR(buf.as_mut_ptr())), &mut len, None, None, None, None);
            if rc.is_err() {
                break;
            }
            out.push(String::from_utf16_lossy(&buf[..len as usize]));
            index += 1;
        }
        let _ = RegCloseKey(key);
    }
    out
}

/// Sets `IsPromoted = 1` on every notify-icon entry registered for `exe` under `root`.
pub fn promote(exe: &str, root: &str) -> Result<usize, String> {
    let mut count = 0;
    for sub in subkeys(root) {
        let key = format!(r"{root}\{sub}");
        if read_string(Hive::CurrentUser, &key, "ExecutablePath").is_some_and(|p| same_exe(&p, exe)) {
            write_dword(Hive::CurrentUser, &key, "IsPromoted", 1)?;
            count += 1;
        }
    }
    Ok(count)
}
