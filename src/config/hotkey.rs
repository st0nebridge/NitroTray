//! @module config::hotkey
//! @description Parses and formats the popup shortcut (default Win+Alt+N).
//!
//! @input  Human strings such as "Win+Alt+N", "Ctrl+Shift+F9".
//! @output `HotkeySpec` (modifiers + key); `None` for anything without a modifier or with an unknown key.
//! @dependencies none

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Letter(char),
    Digit(u8),
    Function(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeySpec {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: Key,
}

fn parse_key(token: &str) -> Option<Key> {
    let t = token.to_ascii_uppercase();
    let mut chars = t.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_uppercase() => return Some(Key::Letter(c)),
        (Some(c), None) if c.is_ascii_digit() => return Some(Key::Digit(c as u8 - b'0')),
        _ => {}
    }
    let n: u8 = t.strip_prefix('F')?.parse().ok()?;
    (1..=24).contains(&n).then_some(Key::Function(n))
}

impl HotkeySpec {
    pub fn parse(text: &str) -> Option<Self> {
        let (mut ctrl, mut alt, mut shift, mut win, mut key) = (false, false, false, false, None);
        for raw in text.split('+') {
            let token = raw.trim();
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                "win" | "super" | "meta" => win = true,
                _ => {
                    if key.is_some() {
                        return None;
                    }
                    key = Some(parse_key(token)?);
                }
            }
        }
        let spec = Self { ctrl, alt, shift, win, key: key? };
        // A bare key would steal ordinary typing; require at least one non-Shift modifier.
        (ctrl || alt || win).then_some(spec)
    }

    pub fn display(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (on, name) in [(self.ctrl, "Ctrl"), (self.win, "Win"), (self.alt, "Alt"), (self.shift, "Shift")] {
            if on {
                parts.push(name.into());
            }
        }
        parts.push(match self.key {
            Key::Letter(c) => c.to_string(),
            Key::Digit(d) => d.to_string(),
            Key::Function(n) => format!("F{n}"),
        });
        parts.join("+")
    }
}
