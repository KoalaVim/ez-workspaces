//! fzf key-string parsing and key-event normalization (design D7).
//!
//! A [`Key`] is the canonical form of a key press. Letters carry shift in the
//! character case (`N`, never `n` + SHIFT), so `alt-N`, `alt-shift-n` and both
//! ways terminals report Alt-Shift-N all compare equal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::error::{EzError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl Key {
    pub const fn new(code: KeyCode, mods: KeyModifiers) -> Self {
        Self { code, mods }
    }

    /// Normalize a terminal key event.
    pub fn from_event(ev: &KeyEvent) -> Self {
        normalize(ev.code, ev.modifiers)
    }

    /// Parse an fzf-style key string (`ctrl-t`, `alt-N`, `alt-shift-n`, `enter`, `f5`, ...).
    pub fn parse(s: &str) -> Result<Self> {
        let invalid = |why: &str| EzError::Config(format!("Invalid key binding '{s}': {why}"));
        let mut mods = KeyModifiers::NONE;
        let mut rest = s;
        loop {
            let lower = rest.to_ascii_lowercase();
            let (flag, len) = if lower.starts_with("ctrl-") {
                (KeyModifiers::CONTROL, 5)
            } else if lower.starts_with("alt-") {
                (KeyModifiers::ALT, 4)
            } else if lower.starts_with("shift-") {
                (KeyModifiers::SHIFT, 6)
            } else {
                break;
            };
            mods |= flag;
            rest = &rest[len..];
        }
        if rest.is_empty() {
            return Err(invalid("missing key"));
        }
        let code = match named_key(&rest.to_ascii_lowercase()) {
            Some(code) => code,
            None => {
                let mut chars = rest.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => KeyCode::Char(c),
                    _ => return Err(invalid("unknown key name")),
                }
            }
        };
        Ok(normalize(code, mods))
    }
}

fn named_key(name: &str) -> Option<KeyCode> {
    Some(match name {
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "btab" => KeyCode::BackTab,
        "space" => KeyCode::Char(' '),
        "bspace" | "backspace" => KeyCode::Backspace,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pgup" | "page-up" => KeyCode::PageUp,
        "pgdn" | "page-down" => KeyCode::PageDown,
        "del" | "delete" => KeyCode::Delete,
        "ins" | "insert" => KeyCode::Insert,
        f if f.starts_with('f') => {
            let n: u8 = f[1..].parse().ok()?;
            if (1..=12).contains(&n) {
                KeyCode::F(n)
            } else {
                return None;
            }
        }
        _ => return None,
    })
}

fn normalize(code: KeyCode, mut mods: KeyModifiers) -> Key {
    let mut code = code;
    if code == KeyCode::Tab && mods.contains(KeyModifiers::SHIFT) {
        code = KeyCode::BackTab;
    }
    match code {
        KeyCode::BackTab => mods.remove(KeyModifiers::SHIFT),
        KeyCode::Char(c) => {
            // crossterm reports Ctrl-\ ] ^ / as Ctrl-4..7.
            if mods.contains(KeyModifiers::CONTROL) {
                code = KeyCode::Char(match c {
                    '4' => '\\',
                    '5' => ']',
                    '6' => '^',
                    '7' => '/',
                    other => other.to_ascii_lowercase(),
                });
                mods.remove(KeyModifiers::SHIFT);
            } else if c.is_ascii_alphabetic() {
                if mods.contains(KeyModifiers::SHIFT) {
                    code = KeyCode::Char(c.to_ascii_uppercase());
                }
                mods.remove(KeyModifiers::SHIFT);
            } else if c != ' ' || !mods.contains(KeyModifiers::SHIFT) {
                // Shift is already encoded in symbols like '?' or '+'.
                mods.remove(KeyModifiers::SHIFT);
            }
        }
        _ => {}
    }
    Key { code, mods }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::KeybindsConfig;

    fn k(s: &str) -> Key {
        Key::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn ctrl_and_alt_letters() {
        assert_eq!(
            k("ctrl-t"),
            Key::new(KeyCode::Char('t'), KeyModifiers::CONTROL)
        );
        assert_eq!(k("alt-n"), Key::new(KeyCode::Char('n'), KeyModifiers::ALT));
    }

    #[test]
    fn uppercase_means_shift() {
        let expect = Key::new(KeyCode::Char('N'), KeyModifiers::ALT);
        assert_eq!(k("alt-N"), expect);
        assert_eq!(k("alt-shift-n"), expect);
        assert_eq!(k("alt-shift-N"), expect);
        assert_ne!(k("alt-N"), k("alt-n"));
    }

    #[test]
    fn terminal_alt_shift_reports_normalize() {
        let upper = KeyEvent::new(KeyCode::Char('N'), KeyModifiers::ALT);
        let lower = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::ALT | KeyModifiers::SHIFT);
        let upper_shift =
            KeyEvent::new(KeyCode::Char('N'), KeyModifiers::ALT | KeyModifiers::SHIFT);
        assert_eq!(Key::from_event(&upper), k("alt-N"));
        assert_eq!(Key::from_event(&lower), k("alt-N"));
        assert_eq!(Key::from_event(&upper_shift), k("alt-N"));
    }

    #[test]
    fn plain_typing_normalizes() {
        let ev = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        assert_eq!(
            Key::from_event(&ev),
            Key::new(KeyCode::Char('A'), KeyModifiers::NONE)
        );
        let q = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
        assert_eq!(Key::from_event(&q), k("?"));
    }

    #[test]
    fn named_keys() {
        let none = KeyModifiers::NONE;
        assert_eq!(k("enter"), Key::new(KeyCode::Enter, none));
        assert_eq!(k("esc"), Key::new(KeyCode::Esc, none));
        assert_eq!(k("tab"), Key::new(KeyCode::Tab, none));
        assert_eq!(k("btab"), Key::new(KeyCode::BackTab, none));
        assert_eq!(k("space"), Key::new(KeyCode::Char(' '), none));
        assert_eq!(k("bspace"), Key::new(KeyCode::Backspace, none));
        assert_eq!(k("up"), Key::new(KeyCode::Up, none));
        assert_eq!(k("down"), Key::new(KeyCode::Down, none));
        assert_eq!(k("left"), Key::new(KeyCode::Left, none));
        assert_eq!(k("right"), Key::new(KeyCode::Right, none));
        assert_eq!(k("home"), Key::new(KeyCode::Home, none));
        assert_eq!(k("end"), Key::new(KeyCode::End, none));
        assert_eq!(k("pgup"), Key::new(KeyCode::PageUp, none));
        assert_eq!(k("pgdn"), Key::new(KeyCode::PageDown, none));
        assert_eq!(k("del"), Key::new(KeyCode::Delete, none));
        assert_eq!(k("alt-enter"), Key::new(KeyCode::Enter, KeyModifiers::ALT));
        for n in 1..=12u8 {
            assert_eq!(k(&format!("f{n}")), Key::new(KeyCode::F(n), none));
        }
    }

    #[test]
    fn btab_event_matches() {
        let ev = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(Key::from_event(&ev), k("btab"));
        assert_eq!(k("shift-tab"), k("btab"));
    }

    #[test]
    fn ctrl_symbols() {
        assert_eq!(
            k("ctrl-/"),
            Key::new(KeyCode::Char('/'), KeyModifiers::CONTROL)
        );
        let ev = KeyEvent::new(KeyCode::Char('7'), KeyModifiers::CONTROL);
        assert_eq!(Key::from_event(&ev), k("ctrl-/"));
        assert_eq!(
            k("ctrl-space"),
            Key::new(KeyCode::Char(' '), KeyModifiers::CONTROL)
        );
    }

    #[test]
    fn invalid_strings_error() {
        for bad in [
            "", "ctrl-", "alt-", "nonsense", "ctrl-foo", "f13", "f0", "alt-ab",
        ] {
            let err = Key::parse(bad).unwrap_err();
            assert!(matches!(err, EzError::Config(_)), "{bad}");
            assert!(err.to_string().contains("Invalid key binding"), "{bad}");
        }
    }

    #[test]
    fn all_default_keybinds_parse() {
        let b = KeybindsConfig::default();
        let all = [
            (&b.new_session, "alt-n"),
            (&b.delete_session, "alt-d"),
            (&b.rename_session, "alt-r"),
            (&b.view_tree, "ctrl-t"),
            (&b.view_workspace, "ctrl-w"),
            (&b.view_repo, "ctrl-e"),
            (&b.view_owner, "ctrl-o"),
            (&b.view_label, "ctrl-g"),
            (&b.edit_labels, "alt-l"),
            (&b.cd_session, "ctrl-d"),
            (&b.new_bare_session, "alt-N"),
            (&b.sort_toggle, "ctrl-s"),
            (&b.session_from_dirty, "alt-s"),
            (&b.clone_repo, "alt-a"),
            (&b.remove_repo, "alt-d"),
            (&b.note_open, "alt-i"),
            (&b.note_cd, "alt-I"),
            (&b.view_all, "ctrl-a"),
        ];
        for (value, expected) in all {
            assert_eq!(value, expected);
            Key::parse(value).unwrap();
        }
        assert_eq!(
            k(&b.new_bare_session),
            Key::new(KeyCode::Char('N'), KeyModifiers::ALT)
        );
        assert_ne!(k(&b.note_open), k(&b.note_cd));
    }
}
