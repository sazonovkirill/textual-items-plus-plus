//! Key events to commands: the terminal frontend's default key for each command and the lookup
//! that turns a pressed key into one, alike under the kitty keyboard protocol and legacy key
//! reporting (N-PORT-03).

use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::app::Command;

/// One key press: its modifiers and code, with `Shift` dropped from characters because the
/// character already says whether it is shifted.
pub(super) type Key = (KeyModifiers, KeyCode);

/// Each command's default keys, written `mod+mod+key` in lower case for a key with `ctrl`, `alt`
/// or `shift`, and the character or key name alone for a key without them. A feature binds its
/// commands here.
const DEFAULT_KEYS: &[(Command, &[&str])] = &[(Command::Quit, &["ctrl+q"])];

/// The key `text` names, as [`DEFAULT_KEYS`] writes keys; `shift+` with a character is that
/// character shifted. `None` for anything else.
fn parse_key(text: &str) -> Option<Key> {
    let (modifiers, name) = match text.rsplit_once('+') {
        _ if text.chars().count() == 1 => (None, text),
        None => (None, text),
        Some((modifiers, "")) => (Some(modifiers.strip_suffix('+')?), "+"),
        Some((modifiers, name)) => (Some(modifiers), name),
    };
    let mut held = KeyModifiers::NONE;
    for modifier in modifiers.into_iter().flat_map(|text| text.split('+')) {
        held |= match modifier {
            "ctrl" => KeyModifiers::CONTROL,
            "alt" => KeyModifiers::ALT,
            "shift" => KeyModifiers::SHIFT,
            _ => return None,
        };
    }
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(pressed(held, KeyCode::Char(c)));
    }
    let code = match name {
        "space" => KeyCode::Char(' '),
        "enter" => KeyCode::Enter,
        "esc" => KeyCode::Esc,
        "tab" if held.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        name => KeyCode::F(name.strip_prefix('f')?.parse().ok()?),
    };
    Some((held, code))
}

/// The key `code` with `modifiers` as the keymap matches it. The kitty protocol sends a shifted
/// character as the plain one with `Shift`, legacy reporting as the shifted one, sometimes with
/// `Shift` too; both become the shifted character without `Shift`.
fn pressed(modifiers: KeyModifiers, code: KeyCode) -> Key {
    match code {
        KeyCode::Char(c) if modifiers.contains(KeyModifiers::SHIFT) => (
            modifiers.difference(KeyModifiers::SHIFT),
            KeyCode::Char(c.to_uppercase().next().unwrap_or(c)),
        ),
        code => (modifiers, code),
    }
}

/// The key `event` presses, as the keymaps match it; releases, repeats and other events press
/// none.
pub(super) fn key_press(event: &Event) -> Option<Key> {
    let key = event.as_key_press_event()?;
    Some(pressed(key.modifiers, key.code))
}

/// The commands the keys give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    #[expect(
        clippy::zero_sized_map_values,
        reason = "Quit is the only command until the container view adds more"
    )]
    commands: HashMap<Key, Command>,
}

impl Default for Keymap {
    /// The default keys of every command.
    fn default() -> Self {
        let commands = DEFAULT_KEYS
            .iter()
            .flat_map(|&(command, keys)| keys.iter().map(move |&key| (key, command)))
            .map(|(key, command)| {
                let key = parse_key(key).expect("every default key is written as parse_key reads");
                (key, command)
            })
            .collect();
        Self { commands }
    }
}

impl Keymap {
    /// The command `event` gives, if it is the press of a bound key; releases and repeats give
    /// none.
    #[must_use]
    pub fn command(&self, event: &Event) -> Option<Command> {
        self.commands.get(&key_press(event)?).copied()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crossterm::event::{KeyEvent, KeyEventKind};

    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn every_default_key_parses_and_belongs_to_one_command() {
        let mut seen = HashSet::new();
        for (_, keys) in DEFAULT_KEYS {
            for key in *keys {
                assert!(
                    parse_key(key).is_some_and(|parsed| seen.insert(parsed)),
                    "{key} does not parse or is bound twice"
                );
            }
        }
    }

    #[test]
    fn ctrl_q_quits() {
        let keymap = Keymap::default();
        let ctrl_q = press(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(keymap.command(&ctrl_q), Some(Command::Quit));
    }

    #[test]
    fn q_without_ctrl_or_with_more_modifiers_does_nothing() {
        let keymap = Keymap::default();
        for modifiers in [
            KeyModifiers::NONE,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ] {
            let q = press(KeyCode::Char('q'), modifiers);
            assert_eq!(keymap.command(&q), None, "{modifiers:?}");
        }
    }

    #[test]
    fn only_presses_give_commands() {
        let keymap = Keymap::default();
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            let mut key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
            key.kind = kind;
            assert_eq!(keymap.command(&Event::Key(key)), None, "{kind:?}");
        }
        assert_eq!(keymap.command(&Event::FocusGained), None);
    }

    #[test]
    fn a_shifted_character_matches_alike_from_either_key_reporting() {
        let kitty = pressed(KeyModifiers::SHIFT, KeyCode::Char('g'));
        let legacy = pressed(KeyModifiers::NONE, KeyCode::Char('G'));
        let legacy_with_shift = pressed(KeyModifiers::SHIFT, KeyCode::Char('G'));
        assert_eq!(kitty, (KeyModifiers::NONE, KeyCode::Char('G')));
        assert_eq!(legacy, kitty);
        assert_eq!(legacy_with_shift, kitty);
        assert_eq!(
            pressed(
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                KeyCode::Char('z')
            ),
            (KeyModifiers::CONTROL, KeyCode::Char('Z'))
        );
        assert_eq!(
            pressed(KeyModifiers::SHIFT, KeyCode::Up),
            (KeyModifiers::SHIFT, KeyCode::Up)
        );
    }

    #[track_caller]
    fn check(text: &str, expected: Option<Key>) {
        assert_eq!(parse_key(text), expected, "{text}");
    }

    #[test]
    fn parses_characters_with_and_without_modifiers() {
        check("q", Some((KeyModifiers::NONE, KeyCode::Char('q'))));
        check("+", Some((KeyModifiers::NONE, KeyCode::Char('+'))));
        check("ctrl+q", Some((KeyModifiers::CONTROL, KeyCode::Char('q'))));
        check(
            "ctrl+alt+x",
            Some((
                KeyModifiers::CONTROL | KeyModifiers::ALT,
                KeyCode::Char('x'),
            )),
        );
        check("shift+g", Some((KeyModifiers::NONE, KeyCode::Char('G'))));
        check("alt++", Some((KeyModifiers::ALT, KeyCode::Char('+'))));
        check("é", Some((KeyModifiers::NONE, KeyCode::Char('é'))));
    }

    #[test]
    fn parses_named_keys() {
        let none = KeyModifiers::NONE;
        for (text, code) in [
            ("space", KeyCode::Char(' ')),
            ("enter", KeyCode::Enter),
            ("esc", KeyCode::Esc),
            ("tab", KeyCode::Tab),
            ("backspace", KeyCode::Backspace),
            ("delete", KeyCode::Delete),
            ("up", KeyCode::Up),
            ("down", KeyCode::Down),
            ("left", KeyCode::Left),
            ("right", KeyCode::Right),
            ("home", KeyCode::Home),
            ("end", KeyCode::End),
            ("pageup", KeyCode::PageUp),
            ("pagedown", KeyCode::PageDown),
            ("f1", KeyCode::F(1)),
            ("f12", KeyCode::F(12)),
        ] {
            check(text, Some((none, code)));
        }
        check("shift+tab", Some((KeyModifiers::SHIFT, KeyCode::BackTab)));
        check("shift+up", Some((KeyModifiers::SHIFT, KeyCode::Up)));
    }

    #[test]
    fn refuses_what_is_not_a_key() {
        for text in ["", "meta+q", "ctrl+", "ctrl+qq", "enterr", "fn", "f999"] {
            check(text, None);
        }
    }
}
