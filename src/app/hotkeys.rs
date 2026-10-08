//! @module app::hotkeys
//! @description Maps a parsed `HotkeySpec` to a `global_hotkey::HotKey` for registration.
//!
//! @input  `config::hotkey::HotkeySpec`.
//! @output `HotKey` (modifiers + physical key code).
//! @dependencies global-hotkey, config::hotkey
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

use crate::config::hotkey::{HotkeySpec, Key};

const LETTERS: [Code; 26] = [
    Code::KeyA,
    Code::KeyB,
    Code::KeyC,
    Code::KeyD,
    Code::KeyE,
    Code::KeyF,
    Code::KeyG,
    Code::KeyH,
    Code::KeyI,
    Code::KeyJ,
    Code::KeyK,
    Code::KeyL,
    Code::KeyM,
    Code::KeyN,
    Code::KeyO,
    Code::KeyP,
    Code::KeyQ,
    Code::KeyR,
    Code::KeyS,
    Code::KeyT,
    Code::KeyU,
    Code::KeyV,
    Code::KeyW,
    Code::KeyX,
    Code::KeyY,
    Code::KeyZ,
];
const DIGITS: [Code; 10] = [
    Code::Digit0,
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
];
const FKEYS: [Code; 24] = [
    Code::F1,
    Code::F2,
    Code::F3,
    Code::F4,
    Code::F5,
    Code::F6,
    Code::F7,
    Code::F8,
    Code::F9,
    Code::F10,
    Code::F11,
    Code::F12,
    Code::F13,
    Code::F14,
    Code::F15,
    Code::F16,
    Code::F17,
    Code::F18,
    Code::F19,
    Code::F20,
    Code::F21,
    Code::F22,
    Code::F23,
    Code::F24,
];

pub fn code(key: Key) -> Option<Code> {
    match key {
        Key::Letter(c) if c.is_ascii_uppercase() => Some(LETTERS[(c as u8 - b'A') as usize]),
        Key::Digit(d) if d <= 9 => Some(DIGITS[d as usize]),
        Key::Function(n) if (1..=24).contains(&n) => Some(FKEYS[(n - 1) as usize]),
        _ => None,
    }
}

pub fn to_hotkey(spec: &HotkeySpec) -> Option<HotKey> {
    let mut m = Modifiers::empty();
    if spec.ctrl {
        m |= Modifiers::CONTROL;
    }
    if spec.alt {
        m |= Modifiers::ALT;
    }
    if spec.shift {
        m |= Modifiers::SHIFT;
    }
    if spec.win {
        m |= Modifiers::SUPER;
    }
    Some(HotKey::new(Some(m), code(spec.key)?))
}
