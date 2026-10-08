//! @module cu_20260930_019
//! @description Regression test (SYSTEM write to a user-creatable path): the installed helper's custom-curve marker
//! lives in its administrators-only Program Files folder, never under %ProgramData%, where a standard user
//! can create `NitroTray` first and redirect the SYSTEM write with a junction or link.
use nitrotray::meta::PIPE_NAME;
use nitrotray::service::host::marker_path;
use nitrotray::service::install_path::{is_protected_location, program_files, service_dir};

#[test]
fn production_marker_is_in_the_protected_folder() {
    let pf = program_files().expect("ProgramFiles is set on Windows");
    let marker = marker_path(PIPE_NAME, false).expect("the service keeps a crash marker");
    assert!(is_protected_location(&marker, &pf), "{}", marker.display());
    assert_eq!(marker, service_dir(&pf).join("curve-active"));
    let data = std::env::var_os("ProgramData").expect("ProgramData is set on Windows");
    assert!(!marker.starts_with(data));
}

#[test]
fn development_markers_never_share_the_production_file() {
    let production = marker_path(PIPE_NAME, false).unwrap();
    let dev = marker_path(r"\\.\pipe\NitroTrayDev", false).unwrap();
    assert_ne!(dev, production);
    assert!(dev.file_name().unwrap().to_string_lossy().starts_with("curve-active-dev-"));
    let simulated = marker_path(PIPE_NAME, true).unwrap();
    assert_ne!(simulated, production, "a simulated helper on the production pipe name is still a dev helper");
}
