//! @module hotkeys_test
//! @description HotkeySpec → global_hotkey mapping.
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use nitrotray::app::hotkeys::{code, to_hotkey};
use nitrotray::config::hotkey::{HotkeySpec, Key};

#[test]
fn maps_keys() {
    assert_eq!(code(Key::Letter('A')), Some(Code::KeyA));
    assert_eq!(code(Key::Letter('Z')), Some(Code::KeyZ));
    assert_eq!(code(Key::Digit(0)), Some(Code::Digit0));
    assert_eq!(code(Key::Digit(9)), Some(Code::Digit9));
    assert_eq!(code(Key::Function(1)), Some(Code::F1));
    assert_eq!(code(Key::Function(24)), Some(Code::F24));
    assert_eq!(code(Key::Letter('a')), None);
    assert_eq!(code(Key::Digit(10)), None);
    assert_eq!(code(Key::Function(0)), None);
    assert_eq!(code(Key::Function(25)), None);
}

#[test]
fn maps_modifiers() {
    let hk = to_hotkey(&HotkeySpec::parse("Win+Alt+N").unwrap()).unwrap();
    assert_eq!(hk, HotKey::new(Some(Modifiers::SUPER | Modifiers::ALT), Code::KeyN));
    let hk = to_hotkey(&HotkeySpec::parse("Ctrl+Shift+F5").unwrap()).unwrap();
    assert_eq!(hk, HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::F5));
}
