//! @module main
//! @description NitroTray.exe entry point: starts the tray host (no console window in release builds).
//!
//! @dependencies nitrotray::app::{bootstrap, events}
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = nitrotray::app::events::Options {
        autostart_launch: args.iter().any(|a| a == "--autostart"),
        exit_running: args.iter().any(|a| a == "--exit"),
    };
    #[cfg(windows)]
    if let Err(e) = nitrotray::app::bootstrap::run(opts) {
        report_fatal(&e);
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn report_fatal(message: &str) {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    eprintln!("{message}");
    // SAFETY: modal message box with owned strings.
    unsafe {
        MessageBoxW(None, &HSTRING::from(message), &HSTRING::from(nitrotray::meta::APP_NAME), MB_OK | MB_ICONERROR);
    }
}
