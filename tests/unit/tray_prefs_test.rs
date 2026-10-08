//! @module tray_prefs_test
//! @description Tray-icon promotion touches only entries for our executable, under a throwaway test root.
use nitrotray::platform::tray_prefs::{promote, same_exe, NOTIFY_ROOT};
use nitrotray::platform::windows::{delete_value, read_dword, write_string, Hive};

#[test]
fn path_comparison_is_case_and_separator_insensitive() {
    assert!(same_exe(r"C:\Apps\NitroTray.exe", "c:/apps/nitrotray.EXE"));
    assert!(!same_exe(r"C:\Apps\NitroTray.exe", r"C:\Apps\Other.exe"));
    assert!(!same_exe("", ""));
    assert_eq!(NOTIFY_ROOT, r"Control Panel\NotifyIconSettings");
}

#[test]
fn promotes_matching_entries_only() {
    let root = format!(r"Software\NitroTray-Test\Notify{}", std::process::id());
    let ours = r"C:\Apps\NitroTray.exe";
    write_string(Hive::CurrentUser, &format!(r"{root}\111"), "ExecutablePath", ours).unwrap();
    write_string(Hive::CurrentUser, &format!(r"{root}\222"), "ExecutablePath", r"C:\Other\app.exe").unwrap();
    write_string(Hive::CurrentUser, &format!(r"{root}\333"), "ExecutablePath", &ours.to_lowercase()).unwrap();
    assert_eq!(promote(ours, &root), Ok(2));
    assert_eq!(read_dword(Hive::CurrentUser, &format!(r"{root}\111"), "IsPromoted"), Some(1));
    assert_eq!(read_dword(Hive::CurrentUser, &format!(r"{root}\222"), "IsPromoted"), None);
    for k in ["111", "222", "333"] {
        let _ = delete_value(Hive::CurrentUser, &format!(r"{root}\{k}"), "ExecutablePath");
        let _ = delete_value(Hive::CurrentUser, &format!(r"{root}\{k}"), "IsPromoted");
    }
    assert_eq!(promote(ours, r"Software\NitroTray-Test\DoesNotExist"), Ok(0));
}
