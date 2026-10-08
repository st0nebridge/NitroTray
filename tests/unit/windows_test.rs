//! @module windows_test
//! @description Registry helpers and autostart against a throwaway HKCU test key; machine info on this host.
use nitrotray::platform::windows::{
    autostart_command, delete_value, machine_info, read_dword, read_string, write_string, Autostart, Hive,
    SingleInstance,
};

fn test_key() -> String {
    format!(r"Software\NitroTray-Test\{}", std::process::id())
}

#[test]
fn string_values_round_trip_and_delete() {
    let key = test_key();
    write_string(Hive::CurrentUser, &key, "probe", "hello ünïcode").unwrap();
    assert_eq!(read_string(Hive::CurrentUser, &key, "probe").as_deref(), Some("hello ünïcode"));
    delete_value(Hive::CurrentUser, &key, "probe").unwrap();
    assert_eq!(read_string(Hive::CurrentUser, &key, "probe"), None);
    delete_value(Hive::CurrentUser, &key, "probe").unwrap();
}

#[test]
fn autostart_uses_the_configured_key_only() {
    let a = Autostart { subkey: test_key(), value: "NitroTrayAutostart".into() };
    assert!(!a.is_enabled());
    a.set(true, r"C:\Apps\NitroTray.exe").unwrap();
    assert_eq!(a.command().as_deref(), Some(r#""C:\Apps\NitroTray.exe" --autostart"#));
    assert!(a.is_enabled());
    a.set(false, "").unwrap();
    assert!(!a.is_enabled());
    assert_eq!(autostart_command("x.exe"), "\"x.exe\" --autostart");
    let d = Autostart::default();
    assert_eq!(d.value, "NitroTray");
    assert!(d.subkey.ends_with(r"CurrentVersion\Run"));
}

#[test]
fn reads_machine_identity() {
    let m = machine_info();
    assert!(m.os_build.as_deref().is_some_and(|b| b.contains('.')));
    assert!(m.model.is_some());
    assert!(read_dword(Hive::LocalMachine, r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "UBR").is_some());
    assert_eq!(read_dword(Hive::CurrentUser, &test_key(), "absent"), None);
    let _ = nitrotray::platform::windows::apps_use_light_theme();
}

#[test]
fn single_instance_is_exclusive() {
    let name = format!("Local\\NitroTrayTest.{}", std::process::id());
    let first = SingleInstance::acquire(&name).expect("first acquire");
    assert!(SingleInstance::acquire(&name).is_none());
    drop(first);
    assert!(SingleInstance::acquire(&name).is_some());
}

#[test]
fn shell_open_rejects_nonsense_targets() {
    assert!(nitrotray::platform::windows::shell_open(r"C:\definitely\not\here.nitro").is_err());
}
