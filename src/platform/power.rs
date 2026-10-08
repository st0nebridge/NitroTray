//! @module platform::power
//! @description Power-source queries used to explain firmware refusals (e.g. Performance on battery).
//!
//! @output `Some(true)` on AC, `Some(false)` on battery, `None` when unknown.
//! @dependencies windows (System::Power) on Windows

/// Decodes `SYSTEM_POWER_STATUS.ACLineStatus` (0 offline, 1 online, 255 unknown).
pub fn decode_ac_line(status: u8) -> Option<bool> {
    match status {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

#[cfg(windows)]
pub fn on_ac_power() -> Option<bool> {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS::default();
    // SAFETY: writes into a local struct.
    unsafe { GetSystemPowerStatus(&mut s).ok()? };
    decode_ac_line(s.ACLineStatus)
}

#[cfg(not(windows))]
pub fn on_ac_power() -> Option<bool> {
    None
}
