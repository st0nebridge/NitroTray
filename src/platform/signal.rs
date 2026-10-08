//! @module platform::signal
//! @description Cross-process signals: "show the popup" (second launch) and "exit" (`--exit`, installers).
//!
//! @input  An event name (session-local).
//! @output `ShowSignal` for the owner (wait loop) and `notify_existing()` for a second instance.
//! @dependencies windows (Threading)
use windows::core::HSTRING;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    CreateEventW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE, SYNCHRONIZATION_ACCESS_RIGHTS,
};

pub const SHOW_EVENT: &str = "Local\\NitroTray.ShowPopup";
/// Asks the running instance to exit cleanly (used by `NitroTray.exe --exit` and installers).
pub const EXIT_EVENT: &str = "Local\\NitroTray.Exit";

/// Auto-reset event owned by the running instance.
pub struct ShowSignal(HANDLE);

// SAFETY: event handles may be waited on from any thread.
unsafe impl Send for ShowSignal {}

impl ShowSignal {
    pub fn create(name: &str) -> Option<Self> {
        // SAFETY: creates (or opens) a named auto-reset event.
        unsafe { CreateEventW(None, false, false, &HSTRING::from(name)).ok().map(Self) }
    }

    /// Blocks up to `timeout_ms`; true when the event was signalled.
    pub fn wait(&self, timeout_ms: u32) -> bool {
        // SAFETY: waits on our own handle.
        unsafe { WaitForSingleObject(self.0, timeout_ms) == WAIT_OBJECT_0 }
    }
}

impl Drop for ShowSignal {
    fn drop(&mut self) {
        // SAFETY: closes the handle created in `create`.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Signals the running instance; false when none is listening.
pub fn notify_existing(name: &str) -> bool {
    // SAFETY: opens the named event with modify rights only, signals it, closes it.
    unsafe {
        let Ok(h) = OpenEventW(SYNCHRONIZATION_ACCESS_RIGHTS(EVENT_MODIFY_STATE.0), false, &HSTRING::from(name)) else {
            return false;
        };
        let ok = SetEvent(h).is_ok();
        let _ = CloseHandle(h);
        ok
    }
}
