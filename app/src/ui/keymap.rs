//! Key events to commands: the terminal frontend's default keys for each command and the lookup
//! that turns pressed keys into one, alike under the kitty keyboard protocol and legacy key
//! reporting (N-PORT-03), single keys and sequences such as `gg` alike.

use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::app::{Command, Focus};

/// One key press: its modifiers and code, with `Shift` dropped from characters because the
/// character already says whether it is shifted.
pub(super) type Key = (KeyModifiers, KeyCode);

/// Each command's default keys, written `mod+mod+key` in lower case for a key with `ctrl`, `alt`
/// or `shift`, and the character or key name alone for a key without them; the keys of a
/// sequence are separated by spaces. No binding starts another, so a sequence waits for its next
/// key only while no binding is complete. A feature binds its commands here.
const DEFAULT_KEYS: &[(Command, &[&str])] = &[
    (Command::Quit, &["ctrl+q"]),
    // The motions of taskpaper-terminal (FR-KEY-02); a section heading is the nearest thing this
    // app has to its next and previous sibling (D-34).
    (Command::Focus(Focus::Next), &["j", "down"]),
    (Command::Focus(Focus::Previous), &["k", "up"]),
    (Command::Focus(Focus::First), &["g g", "home"]),
    (Command::Focus(Focus::Last), &["G", "end"]),
    (Command::Focus(Focus::Parent), &["#"]),
    (Command::Focus(Focus::NextSection), &["g j", "ctrl+down"]),
    (Command::Focus(Focus::PreviousSection), &["g k", "ctrl+up"]),
];

/// The keys `text` names, as [`DEFAULT_KEYS`] writes a sequence. `None` unless each one is a
/// key.
fn parse_sequence(text: &str) -> Option<Vec<Key>> {
    text.split(' ').map(parse_key).collect()
}

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

/// The commands the keys give, and the keys of a sequence pressed so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    commands: HashMap<Vec<Key>, Command>,
    /// Every sequence that is the start of a longer binding, with its keys as written.
    prefixes: HashMap<Vec<Key>, String>,
    pending: Vec<Key>,
}

impl Default for Keymap {
    /// The default keys of every command.
    fn default() -> Self {
        let written = || {
            DEFAULT_KEYS
                .iter()
                .flat_map(|&(command, keys)| keys.iter().map(move |&keys| (keys, command)))
        };
        let commands = written()
            .map(|(keys, command)| {
                let keys = parse_sequence(keys)
                    .expect("every default key is written as parse_sequence reads");
                (keys, command)
            })
            .collect();
        let prefixes = written()
            .flat_map(|(keys, _)| {
                keys.match_indices(' ').map(move |(end, _)| {
                    let prefix = keys.get(..end).unwrap_or_default();
                    let parsed = parse_sequence(prefix)
                        .expect("every default key is written as parse_sequence reads");
                    (parsed, prefix.to_owned())
                })
            })
            .collect();
        Self {
            commands,
            prefixes,
            pending: Vec::new(),
        }
    }
}

impl Keymap {
    /// The command `event` completes, if it is a key press. A press that starts or continues a
    /// sequence waits for the next key; one that continues none cancels the keys pressed so far
    /// and does nothing else. Releases, repeats and other events change nothing.
    pub fn press(&mut self, event: &Event) -> Option<Command> {
        let key = key_press(event)?;
        let mut keys = std::mem::take(&mut self.pending);
        keys.push(key);
        if self.prefixes.contains_key(&keys) {
            self.pending = keys;
            return None;
        }
        self.commands.get(&keys).copied()
    }

    /// The keys of an unfinished sequence, as the status line shows them, such as `g`; empty
    /// when none is pending.
    #[must_use]
    pub fn pending(&self) -> &str {
        self.prefixes.get(&self.pending).map_or("", String::as_str)
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

    fn char(c: char) -> Event {
        press(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn every_default_key_parses_and_belongs_to_one_command() {
        let mut seen = HashSet::new();
        for (_, keys) in DEFAULT_KEYS {
            for key in *keys {
                assert!(
                    parse_sequence(key).is_some_and(|parsed| seen.insert(parsed)),
                    "{key} does not parse or is bound twice"
                );
            }
        }
    }

    #[test]
    fn no_binding_starts_another() {
        let bound: Vec<Vec<Key>> = DEFAULT_KEYS
            .iter()
            .flat_map(|(_, keys)| keys.iter().filter_map(|key| parse_sequence(key)))
            .collect();
        for keys in &bound {
            for other in &bound {
                assert!(
                    keys == other || !other.starts_with(keys),
                    "{keys:?} starts {other:?}"
                );
            }
        }
    }

    #[test]
    fn ctrl_q_quits() {
        let mut keymap = Keymap::default();
        let ctrl_q = press(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(keymap.press(&ctrl_q), Some(Command::Quit));
    }

    // FR-NAV-01, FR-KEY-02
    #[test]
    fn the_motions_have_vi_keys_and_arrow_keys() {
        let mut keymap = Keymap::default();
        let none = KeyModifiers::NONE;
        let focus = |motion| Some(Command::Focus(motion));
        for (event, command) in [
            (char('j'), focus(Focus::Next)),
            (press(KeyCode::Down, none), focus(Focus::Next)),
            (char('k'), focus(Focus::Previous)),
            (press(KeyCode::Up, none), focus(Focus::Previous)),
            (press(KeyCode::Home, none), focus(Focus::First)),
            (char('G'), focus(Focus::Last)),
            (
                press(KeyCode::Char('g'), KeyModifiers::SHIFT),
                focus(Focus::Last),
            ),
            (press(KeyCode::End, none), focus(Focus::Last)),
            (char('#'), focus(Focus::Parent)),
            (
                press(KeyCode::Down, KeyModifiers::CONTROL),
                focus(Focus::NextSection),
            ),
            (
                press(KeyCode::Up, KeyModifiers::CONTROL),
                focus(Focus::PreviousSection),
            ),
        ] {
            assert_eq!(keymap.press(&event), command, "{event:?}");
        }
    }

    // FR-NAV-01, FR-KEY-02
    #[test]
    fn a_sequence_waits_for_its_next_key_showing_what_is_pending() {
        let mut keymap = Keymap::default();
        assert_eq!(keymap.pending(), "");
        for (second, motion) in [
            ('g', Focus::First),
            ('j', Focus::NextSection),
            ('k', Focus::PreviousSection),
        ] {
            assert_eq!(keymap.press(&char('g')), None);
            assert_eq!(keymap.pending(), "g");
            assert_eq!(
                keymap.press(&char(second)),
                Some(Command::Focus(motion)),
                "g {second}"
            );
            assert_eq!(keymap.pending(), "");
        }
    }

    #[test]
    fn a_key_that_continues_no_sequence_cancels_it_and_does_nothing_else() {
        let mut keymap = Keymap::default();
        keymap.press(&char('g'));
        assert_eq!(keymap.press(&char('#')), None);
        assert_eq!(keymap.pending(), "");
        assert_eq!(
            keymap.press(&char('#')),
            Some(Command::Focus(Focus::Parent))
        );
    }

    #[test]
    fn releases_and_other_events_leave_a_pending_sequence_alone() {
        let mut keymap = Keymap::default();
        keymap.press(&char('g'));
        let mut release = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(keymap.press(&Event::Key(release)), None);
        assert_eq!(keymap.press(&Event::Resize(80, 24)), None);
        assert_eq!(keymap.pending(), "g");
        assert_eq!(keymap.press(&char('g')), Some(Command::Focus(Focus::First)));
    }

    #[test]
    fn q_without_ctrl_or_with_more_modifiers_does_nothing() {
        let mut keymap = Keymap::default();
        for modifiers in [
            KeyModifiers::NONE,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ] {
            let q = press(KeyCode::Char('q'), modifiers);
            assert_eq!(keymap.press(&q), None, "{modifiers:?}");
        }
    }

    #[test]
    fn only_presses_give_commands() {
        let mut keymap = Keymap::default();
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            let mut key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
            key.kind = kind;
            assert_eq!(keymap.press(&Event::Key(key)), None, "{kind:?}");
        }
        assert_eq!(keymap.press(&Event::FocusGained), None);
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

    #[test]
    fn parses_sequences_of_keys_separated_by_spaces() {
        let g = (KeyModifiers::NONE, KeyCode::Char('g'));
        assert_eq!(parse_sequence("g g"), Some(vec![g, g]));
        assert_eq!(parse_sequence("g"), Some(vec![g]));
        assert_eq!(parse_sequence("g meta+x"), None);
        assert_eq!(parse_sequence("g  g"), None);
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
