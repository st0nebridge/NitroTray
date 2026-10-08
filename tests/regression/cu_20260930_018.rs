//! @module cu_20260930_018
//! @description Regression test (NSIS installer): the elevated installer may only place the SYSTEM helper where
//! administrators alone can write, and per-user cleanup must target the signed-in user. The script is
//! not unit-testable, so this test guards its source; tests/installer/roundtrip.ps1 runs it for real.

const SCRIPT: &str = include_str!("../../src/installer/nitrotray.nsi");

fn section<'a>(start: &str, end: &str) -> &'a str {
    let from = SCRIPT.find(start).unwrap_or_else(|| panic!("missing {start}"));
    let rest = &SCRIPT[from..];
    &rest[..rest.find(end).unwrap_or(rest.len())]
}

#[test]
fn install_folder_is_fixed_under_program_files() {
    assert!(SCRIPT.contains("RequestExecutionLevel admin"));
    assert!(SCRIPT.contains("InstallDir \"$PROGRAMFILES64\\${APP_NAME}\""));
    assert!(
        !SCRIPT.contains("MUI_PAGE_DIRECTORY"),
        "no directory choice: a user-writable folder would let the helper be swapped"
    );
    let on_init = section("Function .onInit", "FunctionEnd");
    assert!(on_init.contains("StrCpy $INSTDIR \"$PROGRAMFILES64\\${APP_NAME}\""), "/D= must not move the install");
}

#[test]
fn helper_is_registered_from_the_installed_copy() {
    let helper = section("Section \"Helper service\"", "SectionEnd");
    assert!(helper.contains("\"$INSTDIR\\${HELPER_EXE}\" install"));
    assert!(!SCRIPT.contains("$TEMP\\${HELPER_EXE}") && !SCRIPT.contains("$PLUGINSDIR\\${HELPER_EXE}"));
}

#[test]
fn per_user_cleanup_runs_in_the_users_context() {
    let uninstall = section("Section \"Uninstall\"", "SectionEnd");
    let switch = uninstall.find("SetShellVarContext current").expect("uninstall switches to the user context");
    let first_local = uninstall.find("$LOCALAPPDATA").expect("settings are offered for removal");
    assert!(switch < first_local, "$LOCALAPPDATA in the all-users context is %ProgramData%");
    assert!(uninstall.contains("/SD IDNO"), "a silent uninstall keeps the user's settings");
    let autostart = section("Function ${prefix}RemoveAutostart", "FunctionEnd");
    assert!(
        autostart.contains("${If} $0 == '\"$INSTDIR\\${TRAY_EXE}\" --autostart'"),
        "only this install's entry is removed"
    );
}

#[test]
fn binaries_are_released_before_they_are_replaced() {
    let app = section("Section \"${APP_NAME}\" SEC_APP", "SectionEnd");
    let stop = app.find("Call StopNitroTray").expect("the tray and helper are stopped first");
    assert!(stop < app.find("File \"${SRCDIR}\\${HELPER_EXE}\"").unwrap());
    let stopper = section("Function ${prefix}StopNitroTray", "FunctionEnd");
    assert!(stopper.contains("sc.exe\" stop ${SERVICE_NAME}") && stopper.contains("WaitUnlocked"));
}
