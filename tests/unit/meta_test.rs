//! @module meta_test
//! @description Verifies the application identity constants the tray, helper and installer rely on.
use nitrotray::meta;

#[test]
fn version_matches_manifest() {
    assert_eq!(meta::version(), env!("CARGO_PKG_VERSION"));
    assert!(!meta::version().is_empty());
}

#[test]
fn pipe_name_is_local_named_pipe() {
    // Byte-exact: "\\.\pipe\NitroTray" — 2 leading backslashes, built without escapes to be heredoc-proof.
    let expected: String = ['\\', '\\', '.', '\\'].iter().collect::<String>() + "pipe" + "\\" + "NitroTray";
    assert_eq!(meta::PIPE_NAME, expected);
    assert!(meta::PIPE_NAME.starts_with(r"\\.\pipe\"));
}

#[test]
fn identity_constants_are_distinct() {
    assert_ne!(meta::APP_NAME, meta::SERVICE_NAME);
    assert_eq!(meta::PROTOCOL_VERSION, 1);
}

#[test]
fn instance_tags_are_validated_and_scope_names() {
    assert_eq!(meta::tag_from(Some("dev".into())), Some("dev".into()));
    assert_eq!(meta::tag_from(Some("test-123_A".into())), Some("test-123_A".into()));
    for bad in ["", "has space", "slash/x", "x".repeat(33).as_str(), "dot.x"] {
        assert_eq!(meta::tag_from(Some(bad.to_string())), None, "{bad}");
    }
    assert_eq!(meta::tag_from(None), None);
    assert_eq!(meta::scoped(r"Local\NitroTray.Exit", None), r"Local\NitroTray.Exit");
    assert_eq!(meta::scoped(r"Local\NitroTray.Exit", Some("dev")), r"Local\NitroTray.Exit.dev");
    assert!(meta::INSTANCE_MUTEX.starts_with("Local\\"));
}
