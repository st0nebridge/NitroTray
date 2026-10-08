//! @module meta
//! @description Application identity constants shared by the tray host, the helper service and tests.
//!
//! @output Names, pipe path and version strings; no side effects.
//! @dependencies none

/// Display name used for windows, tray tooltips and the autostart entry.
pub const APP_NAME: &str = "NitroTray";

/// Windows service name of the privileged helper.
pub const SERVICE_NAME: &str = "NitroTrayHelper";

/// Local named pipe served by the helper.
pub const PIPE_NAME: &str = r"\\.\pipe\NitroTray";

/// IPC protocol revision; bumped on any incompatible request/response change.
pub const PROTOCOL_VERSION: u32 = 1;

/// Session-wide single-instance mutex of the tray.
pub const INSTANCE_MUTEX: &str = "Local\\NitroTray.SingleInstance";

/// Accepts an instance tag only if it is 1–32 of `[A-Za-z0-9_-]`.
pub fn tag_from(value: Option<String>) -> Option<String> {
    value
        .filter(|t| (1..=32).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

/// Optional `NITROTRAY_INSTANCE` tag scoping session-wide names (mutex, show/exit events), so a
/// development or test instance can never signal or block the installed one.
pub fn instance_tag() -> Option<String> {
    tag_from(std::env::var("NITROTRAY_INSTANCE").ok())
}

/// `name` for the default instance, `name.tag` for a tagged one.
pub fn scoped(name: &str, tag: Option<&str>) -> String {
    match tag {
        Some(t) => format!("{name}.{t}"),
        None => name.to_string(),
    }
}

/// Crate version baked in at compile time.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
