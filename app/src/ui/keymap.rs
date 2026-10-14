//! Key events to commands: the terminal frontend's default keys for each command and the lookup
//! that turns pressed keys into one, alike under the kitty keyboard protocol and legacy key
//! reporting (N-PORT-03), single keys and sequences such as `gg` alike.

use std::{collections::HashMap, time::Duration};

use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::{
    app::{Command, Direction, Focus, Go, Side, Whose, view::View},
    domain::dates::DateKind,
};

/// How long a sequence waits for its next key, as in taskpaper-terminal (its keybindings.md §1).
const TIMEOUT: Duration = Duration::from_secs(1);

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
    // taskpaper-terminal's selection keys: `Shift` with a motion extends (D-24).
    (Command::Extend(Direction::Down), &["J", "shift+down"]),
    (Command::Extend(Direction::Up), &["K", "shift+up"]),
    (Command::TogglePicked, &["v"]),
    (Command::SelectAll, &["ctrl+a"]),
    (Command::ClearSelection, &["esc"]),
    // taskpaper-terminal's keys; a legacy terminal sends `Ctrl+Shift+Z` as `Ctrl+Z`, which
    // undoes (D-35).
    (Command::ToggleCompleted, &["x", "ctrl+enter"]),
    (Command::ShowCompleted, &["ctrl+o", "g c"]),
    (Command::Undo, &["z", "ctrl+z"]),
    (Command::Redo, &["Z", "ctrl+shift+z", "ctrl+y"]),
    // taskpaper-terminal's zoom, home and history keys; home is the Inbox, so its Go to Inbox key
    // goes home too (D-36).
    (Command::Go(Go::Into), &[".", "alt+l", "alt+right", "g z"]),
    (Command::Go(Go::Up), &[">", "alt+h", "alt+left", "g u"]),
    (Command::Go(Go::Home), &["ctrl+'", "g h", "g i"]),
    (Command::Go(Go::Back), &["H", "alt+shift+left"]),
    (Command::Go(Go::Forward), &["L", "alt+shift+right"]),
    // FR-KEY-06's go-to keys for the views (D-23).
    (Command::Go(Go::View(View::Today)), &["g t"]),
    (Command::Go(Go::View(View::Upcoming)), &["g U"]),
    (Command::Go(Go::View(View::All)), &["g a"]),
    (Command::Navigator, &["ctrl+g"]),
    // taskpaper-terminal's keys to edit, add below and above, and delete (D-37), with its type
    // letters after the creation keys, and `a` for an area (D-40).
    (Command::EditTitle, &["enter"]),
    (Command::NewTask(Side::Below), &["o", "o t"]),
    (Command::NewTask(Side::Above), &["O", "O t"]),
    (Command::NewSection(Side::Below), &["o s"]),
    (Command::NewSection(Side::Above), &["O s"]),
    (Command::NewProject(Side::Below), &["o p"]),
    (Command::NewProject(Side::Above), &["O p"]),
    (Command::NewArea(Side::Below), &["o a"]),
    (Command::NewArea(Side::Above), &["O a"]),
    (Command::Delete, &["d d", "delete"]),
    // taskpaper-terminal's Move to key.
    (Command::MoveTo, &["g m"]),
    // taskpaper-terminal's clipboard keys (its D-53).
    (Command::Duplicate, &["D"]),
    (Command::Copy, &["y", "ctrl+c"]),
    (Command::Cut, &["ctrl+x"]),
    (Command::Paste(Side::Below), &["p", "ctrl+v"]),
    (Command::Paste(Side::Above), &["P", "alt+v"]),
    // The container shown, which no row of its own shows (D-40).
    (Command::RenameContainer, &["R"]),
    (Command::DeleteContainer, &["d c"]),
    // `Alt` with a motion moves, and `]`/`[` indent and outdent beside taskpaper-terminal's
    // `Tab`/`Shift+Tab` (D-24, D-42).
    (Command::Move(Direction::Up), &["alt+k", "alt+up"]),
    (Command::Move(Direction::Down), &["alt+j", "alt+down"]),
    (Command::Indent, &["]", "tab"]),
    (Command::Outdent, &["[", "shift+tab"]),
    // taskpaper-terminal's convert type and group keys; `S` for a section beside its `G`, which
    // groups in a new parent as a new project does here (D-47).
    (Command::Convert, &["m y"]),
    (Command::GroupIntoSection, &["g S"]),
    (Command::GroupIntoProject, &["g G"]),
    // `m` modifies the focused item (FR-KEY-06); `n` for the notes, as TaskPaper calls the lines a
    // description is stored as, and shifted for the container shown, whose title stands above
    // the rows (D-43).
    (Command::EditDescription(Whose::Focused), &["m n"]),
    (Command::EditDescription(Whose::Shown), &["m N"]),
    // taskpaper-terminal's date prompt keys, and `t` to start today (D-23).
    (Command::SetDate(DateKind::Start), &["m s"]),
    (Command::SetDate(DateKind::Due), &["m d"]),
    (Command::StartToday, &["m t"]),
    // `m` and ta**g**, since `mt` sets the start date today; `g` and **f**ilter goes to a narrower
    // view of the place shown, `F` taking the filter off again (D-50).
    (Command::Tags, &["m g"]),
    (Command::FilterTags, &["g f"]),
    (Command::ClearTagFilter, &["g F"]),
    // Todoist's sidebar key, and taskpaper-terminal's keys to focus the pane left and right
    // (D-45).
    (Command::Sidebar, &["M"]),
    (Command::FocusSidebar, &["ctrl+w h", "ctrl+w left"]),
    (Command::FocusPane, &["ctrl+w l", "ctrl+w right"]),
    // taskpaper-terminal's conflict prompt keys, which no plain letter is since the prompt can open
    // while the user types (its D-115, D-38).
    (Command::KeepMine, &["ctrl+k"]),
    (Command::TakeTheirs, &["ctrl+r"]),
    (Command::ShowDiff, &["ctrl+l"]),
];

/// The first default key of `command`, as [`DEFAULT_KEYS`] writes it, to show beside it.
pub(super) fn written(command: Command) -> Option<&'static str> {
    DEFAULT_KEYS
        .iter()
        .find(|&&(bound, _)| bound == command)
        .and_then(|(_, keys)| keys.first().copied())
}

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

/// The commands the keys give, the keys of a sequence pressed so far, when the first of them was
/// pressed and the clock's latest reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    commands: HashMap<Vec<Key>, Command>,
    /// Every sequence that is the start of a longer binding, with its keys as written.
    prefixes: HashMap<Vec<Key>, String>,
    pending: Vec<Key>,
    since: Duration,
    now: Duration,
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
            since: Duration::ZERO,
            now: Duration::ZERO,
        }
    }
}

impl Keymap {
    /// The command `event` completes, if it is a key press. A press that starts or continues a
    /// sequence waits for the next key; one that continues none cancels the keys pressed so far
    /// and does nothing else, unless [`cut_short`](Self::cut_short) took it first. Releases,
    /// repeats and other events change nothing.
    pub fn press(&mut self, event: &Event) -> Option<Command> {
        let key = key_press(event)?;
        let mut keys = std::mem::take(&mut self.pending);
        if keys.is_empty() {
            self.since = self.now;
        }
        keys.push(key);
        if self.prefixes.contains_key(&keys) {
            self.pending = keys;
            return None;
        }
        self.commands.get(&keys).copied()
    }

    /// The command of the keys pressed so far, when they are a binding of their own and `event`
    /// presses a key that continues no binding from them; the keys are then dropped, so that
    /// `event` acts after the command as a key pressed alone, as taskpaper-terminal's `o` then
    /// `B` starts a new item with `B` (its keybindings.md §1). `None`, changing nothing,
    /// otherwise.
    pub fn cut_short(&mut self, event: &Event) -> Option<Command> {
        let command = *self.commands.get(&self.pending)?;
        let mut keys = self.pending.clone();
        keys.push(key_press(event)?);
        if self.prefixes.contains_key(&keys) || self.commands.contains_key(&keys) {
            return None;
        }
        self.pending.clear();
        Some(command)
    }

    /// Takes the clock's reading `now`, counted from any fixed moment. A sequence pressed
    /// [`TIMEOUT`] or more before ends: its keys are dropped, and their command, when they are
    /// a binding of their own, is returned.
    pub fn tick(&mut self, now: Duration) -> Option<Command> {
        self.now = now;
        if self.pending.is_empty() || now < self.since + TIMEOUT {
            return None;
        }
        let keys = std::mem::take(&mut self.pending);
        self.commands.get(&keys).copied()
    }

    /// When the sequence pressed so far ends without another key, if one is pending.
    #[must_use]
    pub fn deadline(&self) -> Option<Duration> {
        (!self.pending.is_empty()).then(|| self.since + TIMEOUT)
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
    fn only_a_single_key_binding_starts_another() {
        let bound: Vec<Vec<Key>> = DEFAULT_KEYS
            .iter()
            .flat_map(|(_, keys)| keys.iter().filter_map(|key| parse_sequence(key)))
            .collect();
        for keys in &bound {
            for other in &bound {
                assert!(
                    keys == other || keys.len() == 1 || !other.starts_with(keys),
                    "{keys:?} starts {other:?}"
                );
            }
        }
    }

    // FR-NAV-02, FR-NAV-04, FR-VIEW-07, FR-KEY-02
    #[test]
    fn going_and_the_navigator_have_taskpaper_terminals_keys() {
        let mut keymap = Keymap::default();
        let none = KeyModifiers::NONE;
        let alt = KeyModifiers::ALT;
        let alt_shift = KeyModifiers::ALT | KeyModifiers::SHIFT;
        let go = |to| Some(Command::Go(to));
        for (events, command) in [
            (vec![char('.')], go(Go::Into)),
            (vec![press(KeyCode::Char('l'), alt)], go(Go::Into)),
            (vec![press(KeyCode::Right, alt)], go(Go::Into)),
            (vec![char('g'), char('z')], go(Go::Into)),
            (vec![char('>')], go(Go::Up)),
            (vec![press(KeyCode::Char('h'), alt)], go(Go::Up)),
            (vec![press(KeyCode::Left, alt)], go(Go::Up)),
            (vec![char('g'), char('u')], go(Go::Up)),
            (
                vec![press(KeyCode::Char('\''), KeyModifiers::CONTROL)],
                go(Go::Home),
            ),
            (vec![char('g'), char('h')], go(Go::Home)),
            (vec![char('g'), char('i')], go(Go::Home)),
            (vec![char('H')], go(Go::Back)),
            (vec![press(KeyCode::Left, alt_shift)], go(Go::Back)),
            (vec![char('L')], go(Go::Forward)),
            (vec![press(KeyCode::Right, alt_shift)], go(Go::Forward)),
            (vec![char('g'), char('t')], go(Go::View(View::Today))),
            (vec![char('g'), char('U')], go(Go::View(View::Upcoming))),
            (vec![char('g'), char('a')], go(Go::View(View::All))),
            (
                vec![press(KeyCode::Char('g'), KeyModifiers::CONTROL)],
                Some(Command::Navigator),
            ),
            (vec![press(KeyCode::Right, none)], None),
        ] {
            let mut given = None;
            for event in &events {
                given = keymap.press(event);
            }
            assert_eq!(given, command, "{events:?}");
        }
    }

    // FR-TAG-02, FR-TAG-04, FR-KEY-06
    #[test]
    fn m_g_tags_and_g_f_filters_by_tags() {
        let mut keymap = Keymap::default();
        for (keys, command) in [
            (['m', 'g'], Command::Tags),
            (['g', 'f'], Command::FilterTags),
            (['g', 'F'], Command::ClearTagFilter),
        ] {
            assert_eq!(keymap.press(&char(keys[0])), None);
            assert_eq!(keymap.press(&char(keys[1])), Some(command), "{keys:?}");
        }
    }

    // FR-OPS-02, FR-OPS-03, FR-KEY-02
    #[test]
    fn alt_with_a_motion_moves_and_brackets_and_tab_indent() {
        let mut keymap = Keymap::default();
        let alt = KeyModifiers::ALT;
        let moved = |direction| Some(Command::Move(direction));
        for (event, command) in [
            (press(KeyCode::Char('k'), alt), moved(Direction::Up)),
            (press(KeyCode::Up, alt), moved(Direction::Up)),
            (press(KeyCode::Char('j'), alt), moved(Direction::Down)),
            (press(KeyCode::Down, alt), moved(Direction::Down)),
            (char(']'), Some(Command::Indent)),
            (
                press(KeyCode::Tab, KeyModifiers::NONE),
                Some(Command::Indent),
            ),
            (char('['), Some(Command::Outdent)),
            (
                press(KeyCode::BackTab, KeyModifiers::SHIFT),
                Some(Command::Outdent),
            ),
        ] {
            assert_eq!(keymap.press(&event), command, "{event:?}");
        }
    }

    #[test]
    fn ctrl_q_quits() {
        let mut keymap = Keymap::default();
        let ctrl_q = press(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(keymap.press(&ctrl_q), Some(Command::Quit));
    }

    // FR-TASK-06, FR-OPS-07, FR-OPS-08, FR-KEY-02
    #[test]
    fn m_y_converts_and_g_shift_s_and_g_shift_g_group() {
        let mut keymap = Keymap::default();
        let shifted = |c| press(KeyCode::Char(c), KeyModifiers::SHIFT);
        for (second, command) in [
            (char('y'), Command::Convert),
            (shifted('s'), Command::GroupIntoSection),
            (shifted('g'), Command::GroupIntoProject),
        ] {
            let first = if command == Command::Convert {
                'm'
            } else {
                'g'
            };
            assert_eq!(keymap.press(&char(first)), None);
            assert_eq!(keymap.press(&second), Some(command), "{second:?}");
        }
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

    // FR-TASK-02, FR-VIEW-11, FR-EDIT-06, FR-KEY-02
    #[test]
    fn completing_showing_completed_undo_and_redo_have_taskpaper_terminal_keys() {
        let mut keymap = Keymap::default();
        let ctrl = KeyModifiers::CONTROL;
        for (event, command) in [
            (char('x'), Command::ToggleCompleted),
            (press(KeyCode::Enter, ctrl), Command::ToggleCompleted),
            (press(KeyCode::Char('o'), ctrl), Command::ShowCompleted),
            (char('z'), Command::Undo),
            (press(KeyCode::Char('z'), ctrl), Command::Undo),
            (char('Z'), Command::Redo),
            (
                press(KeyCode::Char('z'), ctrl | KeyModifiers::SHIFT),
                Command::Redo,
            ),
            (press(KeyCode::Char('y'), ctrl), Command::Redo),
        ] {
            assert_eq!(keymap.press(&event), Some(command), "{event:?}");
        }
        assert_eq!(keymap.press(&char('g')), None);
        assert_eq!(keymap.press(&char('c')), Some(Command::ShowCompleted));
    }

    // FR-EDIT-03, FR-KEY-06
    #[test]
    fn m_n_edits_the_focused_description_and_m_shift_n_the_containers() {
        let mut keymap = Keymap::default();
        assert_eq!(keymap.press(&char('m')), None);
        assert_eq!(
            keymap.press(&char('n')),
            Some(Command::EditDescription(Whose::Focused))
        );
        assert_eq!(keymap.press(&char('m')), None);
        assert_eq!(
            keymap.press(&press(KeyCode::Char('n'), KeyModifiers::SHIFT)),
            Some(Command::EditDescription(Whose::Shown))
        );
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

    // FR-EDIT-05, FR-ORG-06, FR-KEY-02
    #[test]
    fn creation_keys_take_taskpaper_terminals_type_letters() {
        let mut keymap = Keymap::default();
        for (first, side) in [('o', Side::Below), ('O', Side::Above)] {
            for (second, command) in [
                ('t', Command::NewTask(side)),
                ('s', Command::NewSection(side)),
                ('p', Command::NewProject(side)),
                ('a', Command::NewArea(side)),
            ] {
                assert_eq!(keymap.press(&char(first)), None);
                assert_eq!(keymap.pending(), first.to_string());
                assert_eq!(keymap.cut_short(&char(second)), None);
                assert_eq!(
                    keymap.press(&char(second)),
                    Some(command),
                    "{first}{second}"
                );
            }
        }
        assert_eq!(keymap.press(&char('R')), Some(Command::RenameContainer));
        keymap.press(&char('d'));
        assert_eq!(keymap.press(&char('c')), Some(Command::DeleteContainer));
    }

    // FR-EDIT-05, FR-KEY-02
    #[test]
    fn a_key_that_continues_no_sequence_cuts_a_binding_short_and_then_acts_alone() {
        let mut keymap = Keymap::default();
        assert_eq!(keymap.cut_short(&char('o')), None, "nothing pending");
        keymap.press(&char('o'));
        assert_eq!(keymap.cut_short(&Event::Resize(80, 24)), None);
        assert_eq!(
            keymap.cut_short(&char('j')),
            Some(Command::NewTask(Side::Below))
        );
        assert_eq!(keymap.pending(), "");
        assert_eq!(keymap.press(&char('j')), Some(Command::Focus(Focus::Next)));
        keymap.press(&char('g'));
        assert_eq!(keymap.cut_short(&char('#')), None, "g alone is no binding");
        assert_eq!(keymap.pending(), "g");
    }

    // FR-KEY-02
    #[test]
    fn a_sequence_ends_a_second_after_its_first_key() {
        let mut keymap = Keymap::default();
        assert_eq!(keymap.deadline(), None);
        assert_eq!(keymap.tick(Duration::from_secs(5)), None);
        keymap.press(&char('O'));
        assert_eq!(keymap.deadline(), Some(Duration::from_secs(6)));
        assert_eq!(keymap.tick(Duration::from_millis(5999)), None);
        assert_eq!(keymap.pending(), "O");
        assert_eq!(
            keymap.tick(Duration::from_secs(6)),
            Some(Command::NewTask(Side::Above))
        );
        assert_eq!((keymap.pending(), keymap.deadline()), ("", None));
        keymap.press(&char('g'));
        assert_eq!(keymap.tick(Duration::from_secs(7)), None);
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
