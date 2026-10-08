//! @module install_path_test
//! @description The SYSTEM helper binary is only ever registered from the admin-only Program Files folder.
use std::path::Path;

use nitrotray::service::install_path::*;

#[test]
fn target_lives_under_program_files() {
    let pf = Path::new(r"C:\Program Files");
    assert_eq!(service_dir(pf), Path::new(r"C:\Program Files\NitroTray"));
    assert_eq!(service_exe(pf), Path::new(r"C:\Program Files\NitroTray\NitroTrayService.exe"));
    assert!(is_protected_location(&service_exe(pf), pf));
}

#[test]
fn user_writable_locations_are_not_protected() {
    let pf = Path::new(r"C:\Program Files");
    for p in [
        r"C:\Users\me\AppData\Local\Programs\NitroTray\NitroTrayService.exe",
        r"D:\src\nitrotray\target\release\NitroTrayService.exe",
        r"C:\Program Files Evil\NitroTray\NitroTrayService.exe",
        r"C:\Program Files",
    ] {
        assert!(!is_protected_location(Path::new(p), pf), "{p}");
    }
    assert!(is_protected_location(Path::new("c:/program files/x.exe"), Path::new(r"C:\Program Files\")));
    assert!(!is_protected_location(Path::new(r"C:\x.exe"), Path::new("")));
}

#[test]
fn program_files_comes_from_the_environment() {
    let pf = program_files().expect("set on Windows");
    assert!(pf.is_absolute());
    assert_eq!(SERVICE_EXE, "NitroTrayService.exe");
}
