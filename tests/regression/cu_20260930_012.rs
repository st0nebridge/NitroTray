//! @module cu_20260930_012
//! @description Regression test: session-wide names are scoped by NITROTRAY_INSTANCE, so `NitroTray.exe --exit`
//! from a test or dev build can never shut down the user's running tray (it did, via the mutation runs).
use nitrotray::meta::{scoped, tag_from, INSTANCE_MUTEX};
use nitrotray::platform::signal::{EXIT_EVENT, SHOW_EVENT};

#[test]
fn tagged_names_never_equal_default_names() {
    let tag = tag_from(Some("test".into()));
    for name in [INSTANCE_MUTEX, SHOW_EVENT, EXIT_EVENT] {
        assert_ne!(scoped(name, tag.as_deref()), scoped(name, None), "{name}");
    }
}
