//! @module hotkey_test
//! @description Shortcut parsing/formatting (default Win+Alt+N).
use nitrotray::config::hotkey::{HotkeySpec, Key};

#[test]
fn parses_the_default() {
    let h = HotkeySpec::parse("Win+Alt+N").unwrap();
    assert!(h.win && h.alt && !h.ctrl && !h.shift);
    assert_eq!(h.key, Key::Letter('N'));
    assert_eq!(h.display(), "Win+Alt+N");
}

#[test]
fn accepts_aliases_case_and_spaces() {
    let h = HotkeySpec::parse(" ctrl + SHIFT + f12 ").unwrap();
    assert!(h.ctrl && h.shift);
    assert_eq!(h.key, Key::Function(12));
    assert_eq!(h.display(), "Ctrl+Shift+F12");
    assert_eq!(HotkeySpec::parse("Control+7").unwrap().key, Key::Digit(7));
    assert!(HotkeySpec::parse("Super+Meta+x").unwrap().win);
    assert_eq!(HotkeySpec::parse("Ctrl+Win+Alt+Shift+Q").unwrap().display(), "Ctrl+Win+Alt+Shift+Q");
}

#[test]
fn rejects_unsafe_or_malformed_shortcuts() {
    for bad in ["N", "Shift+N", "Win+Alt", "Win+Alt+N+M", "Ctrl+F25", "Ctrl+F0", "Ctrl+Enter", "", "Alt+Fx"] {
        assert!(HotkeySpec::parse(bad).is_none(), "{bad}");
    }
    assert_eq!(HotkeySpec::parse("Alt+F1").unwrap().key, Key::Function(1));
    assert_eq!(HotkeySpec::parse("Alt+F24").unwrap().key, Key::Function(24));
}
