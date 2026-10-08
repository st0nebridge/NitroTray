//! @module cu_20260930_010
//! @description Regression test (security): the LocalSystem helper is registered from `%ProgramFiles%\NitroTray`,
//! never from the per-user folder next to NitroTray.exe (user-writable → SYSTEM code execution).
use std::path::Path;

use nitrotray::service::install_path::{is_protected_location, service_exe};

#[test]
fn registered_binary_is_admin_only() {
    let pf = Path::new(r"C:\Program Files");
    let tray_dir_helper = Path::new(r"C:\Users\someone\AppData\Local\Programs\NitroTray\NitroTrayService.exe");
    assert!(!is_protected_location(tray_dir_helper, pf));
    assert!(is_protected_location(&service_exe(pf), pf));
    let src = include_str!("../../src/service/host.rs");
    assert!(src.contains("service_info(target)"), "install must register the Program Files copy");
    assert!(!src.contains("executable_path: exe,\n        launch_arguments") || src.contains("fn service_info(exe"));
}
