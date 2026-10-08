//! @module store_test
//! @description config.json load/save: defaults when missing, atomic save, corrupt files set aside.
use nitrotray::config::settings::Settings;
use nitrotray::config::store::{default_dir, ConfigStore, FILE_NAME};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nitrotray-store-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn missing_file_gives_defaults_without_warning() {
    let store = ConfigStore::new(temp_dir("missing"));
    assert_eq!(store.load(), (Settings::default(), None));
    assert!(store.path().ends_with(FILE_NAME));
}

#[test]
fn save_then_load_round_trips() {
    let dir = temp_dir("roundtrip");
    let store = ConfigStore::new(&dir);
    let mut s = Settings::default();
    s.general.hotkey = "Ctrl+Alt+J".into();
    s.tray_icons.gpu.enabled = false;
    store.save(&s).unwrap();
    assert!(!dir.join(format!("{FILE_NAME}.tmp")).exists(), "temp file renamed away");
    let (loaded, warning) = store.load();
    assert_eq!(loaded, s);
    assert_eq!(warning, None);
    assert_eq!(store.dir(), dir.as_path());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn corrupt_file_is_set_aside_with_a_warning() {
    let dir = temp_dir("corrupt");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(FILE_NAME), "{ not json").unwrap();
    let store = ConfigStore::new(&dir);
    let (s, warning) = store.load();
    assert_eq!(s, Settings::default());
    assert!(warning.unwrap().contains("defaults restored"));
    assert!(dir.join(format!("{FILE_NAME}.bad")).exists());
    assert!(!dir.join(FILE_NAME).exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn loaded_values_are_sanitised() {
    let dir = temp_dir("sanitise");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(FILE_NAME), r#"{"telemetry":{"background_interval_ms":1}}"#).unwrap();
    let (s, _) = ConfigStore::new(&dir).load();
    assert_eq!(s.telemetry.background_interval_ms, 1000);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn save_reports_unwritable_locations() {
    let file = std::env::temp_dir().join(format!("nitrotray-store-file-{}", std::process::id()));
    std::fs::write(&file, "x").unwrap();
    let store = ConfigStore::new(file.join("sub"));
    assert!(store.save(&Settings::default()).is_err());
    let _ = std::fs::remove_file(file);
}

#[test]
fn default_dir_honours_override() {
    std::env::set_var("NITROTRAY_CONFIG_DIR", r"C:\override");
    assert_eq!(default_dir(), std::path::PathBuf::from(r"C:\override"));
    std::env::remove_var("NITROTRAY_CONFIG_DIR");
    assert!(default_dir().ends_with("NitroTray"));
}
