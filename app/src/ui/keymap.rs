//! Key events to commands: the terminal frontend's default keys for each command and the lookup
//! that turns pressed keys into one, alike under the kitty keyboard protocol and legacy key
//! reporting (N-PORT-03), single keys and sequences such as `gg` alike.

use std::{collections::HashMap, time::Duration};

use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::{
    app::{Command, Direction, End, Focus, Go, Side, Whose, view::View},
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
    // The motions of taskpaper-terminal (FR-KEY-02), without its parent and sibling motions
    // (D-82).
    (Command::Focus(Focus::Next), &["j", "down"]),
    (Command::Focus(Focus::Previous), &["k", "up"]),
    (Command::Focus(Focus::First), &["g g", "home"]),
    (Command::Focus(Focus::Last), &["G", "end"]),
    // taskpaper-terminal's selection keys: `Shift` with a motion extends (D-24).
    (Command::Extend(Direction::Down), &["J", "shift+down"]),
    (Command::Extend(Direction::Up), &["K", "shift+up"]),
    (Command::TogglePicked, &["v"]),
    // vim's visual line mode (D-82).
    (Command::VisualLines, &["V"]),
    (Command::SelectAll, &["ctrl+a"]),
    (Command::ClearSelection, &["esc"]),
    // taskpaper-terminal's keys, and the owner's `.` (D-80); a legacy terminal sends
    // `Ctrl+Shift+Z` as `Ctrl+Z`, which undoes (D-35).
    (Command::ToggleCompleted, &["x", ".", "ctrl+enter"]),
    (Command::ShowCompleted, &["ctrl+o"]),
    (Command::Undo, &["z", "ctrl+z"]),
    (Command::Redo, &["Z", "ctrl+shift+z", "ctrl+y"]),
    // taskpaper-terminal's Go to Inbox and history keys, without its zoom and home (D-36, D-80).
    (Command::Go(Go::Inbox), &["g i"]),
    (Command::Go(Go::Back), &["H", "alt+shift+left"]),
    (Command::Go(Go::Forward), &["L", "alt+shift+right"]),
    // FR-KEY-06's go-to keys for the views (D-23), and the owner's lower-case ones for Upcoming
    // and the Logbook beside taskpaper-terminal's (D-82).
    (Command::Go(Go::View(View::Today)), &["g t"]),
    (Command::Go(Go::View(View::Upcoming)), &["g u", "g U"]),
    (Command::Go(Go::View(View::All)), &["g a"]),
    (Command::Go(Go::View(View::Logbook)), &["g l", "g L"]),
    // `d` as `dd` deletes into it (D-55).
    (Command::Go(Go::View(View::Trash)), &["g d"]),
    // taskpaper-terminal's Navigator key, with the owner's `Ctrl+K` (D-36).
    (Command::Navigator, &["ctrl+g", "ctrl+k"]),
    // taskpaper-terminal's palette key, and **g**o to a **c**ommand, which the owner moved from Show
    // Completed (D-63).
    (Command::Palette, &["ctrl+p", "g c"]),
    // The owner's key, which taskpaper-terminal gives Go to Symbol until search exists (D-69).
    (Command::GoToSymbol, &["/"]),
    // Find, as `Ctrl+F` finds in most apps, and **g**o **s**earch (D-70).
    (Command::Search, &["ctrl+f", "g s"]),
    // taskpaper-terminal's keys to edit, add below and above, and delete (D-37); `o` and `O` add
    // the focused row's kind, and the lower and upper case of a kind's letter add one at the
    // bottom and the top of the container shown (owner, D-40). In the sidebar `a` adds an area,
    // `p` a top-level project and `i`, for **i**nside, a project in the focused area; in the
    // pane `i` adds a project as `p` does (owner, D-75).
    (Command::EditTitle, &["enter"]),
    (Command::NewLike(Side::Below), &["o"]),
    (Command::NewLike(Side::Above), &["O"]),
    (Command::AddTask(End::Bottom), &["a"]),
    (Command::AddTask(End::Top), &["A"]),
    (Command::AddSection(End::Bottom), &["s"]),
    (Command::AddSection(End::Top), &["S"]),
    (Command::AddProject(End::Bottom), &["p"]),
    (Command::AddProject(End::Top), &["P"]),
    (Command::AddProjectInArea(End::Bottom), &["i"]),
    (Command::AddProjectInArea(End::Top), &["I"]),
    (Command::Delete, &["d d", "delete"]),
    // In the Trash, `u` undeletes, as vim's undo would the deletion, and `de` empties it (D-55).
    (Command::Restore, &["u"]),
    (Command::EmptyTrash, &["d e"]),
    // The owner's Move to key, beside taskpaper-terminal's (D-39).
    (Command::MoveTo, &["M", "g m"]),
    // taskpaper-terminal's clipboard keys (its D-53), without `p` and `P`, which add projects
    // (D-44).
    (Command::Duplicate, &["D"]),
    (Command::Copy, &["y", "ctrl+c"]),
    (Command::Cut, &["ctrl+x"]),
    (Command::Paste(Side::Below), &["ctrl+v"]),
    (Command::Paste(Side::Above), &["alt+v"]),
    // The container shown, which no row of its own shows (D-40); deleting it has no key (D-83).
    (Command::RenameContainer, &["R"]),
    // `Alt` with a motion moves, leaving taskpaper-terminal's `Tab`/`Shift+Tab` to move between
    // panes; the attachments panel's own keys nest checklist items (D-24, D-42, D-79, D-82).
    (Command::Move(Direction::Up), &["alt+k", "alt+up"]),
    (Command::Move(Direction::Down), &["alt+j", "alt+down"]),
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
    // taskpaper-terminal's Open Note and Open Comments open the attachments panel; with `Alt`,
    // on the container shown, since the shifted key is taken (FR-ATT-02).
    (Command::OpenNote(Whose::Focused), &["n"]),
    (Command::OpenNote(Whose::Shown), &["alt+n"]),
    (Command::OpenComments(Whose::Focused), &["N"]),
    (Command::OpenComments(Whose::Shown), &["alt+N"]),
    // `c` for the checklist, which only a task has, so it has no key for the container shown.
    (Command::OpenChecklist, &["c"]),
    // taskpaper-terminal's date prompt keys, and `t` to start today (D-23).
    (Command::SetDate(DateKind::Start), &["m s"]),
    (Command::SetDate(DateKind::Due), &["m d"]),
    (Command::StartToday, &["m t"]),
    // `m` and **l**ater, as Things 3's Someday, whose `Cmd+O` no terminal sends (D-89).
    (Command::ToggleLater, &["m l"]),
    // The owner's brackets nudge a date a day, as taskpaper-terminal's `[`/`]` nudge the start
    // date: the plain ones the start date, the curly ones the due date (D-82).
    (Command::DayEarlier(DateKind::Start), &["["]),
    (Command::DayLater(DateKind::Start), &["]"]),
    (Command::DayEarlier(DateKind::Due), &["{"]),
    (Command::DayLater(DateKind::Due), &["}"]),
    // `m` and ta**g**, since `mt` sets the start date today; `g` and **f**ilter goes to a narrower
    // view of the place shown, `F` taking the filter off again (D-50).
    (Command::Tags, &["m g"]),
    (Command::FilterTags, &["g f"]),
    (Command::ClearTagFilter, &["g F"]),
    // The owner's key to fold a project in the sidebar (D-67).
    (Command::Collapse, &["'"]),
    // The owner's sidebar key (D-45), and taskpaper-terminal's pane keys: a legacy terminal sends
    // `Ctrl+\` as `Ctrl+4` (D-60). The owner's keys cycle through the panes, the sidebar first,
    // `g` with a digit reaches one, `g0` the sidebar, and `gj`/`gk`, taskpaper-terminal's next and
    // previous sibling, browse the sidebar's entries in the first pane (D-79).
    (Command::Sidebar, &["ctrl+b"]),
    (Command::NextPane, &["tab", "g n"]),
    (Command::PreviousPane, &["shift+tab", "g p"]),
    (Command::Browse(Direction::Down), &["g j", "ctrl+down"]),
    (Command::Browse(Direction::Up), &["g k", "ctrl+up"]),
    (
        Command::FocusLeft,
        &["ctrl+w h", "ctrl+w left", "ctrl+shift+left"],
    ),
    (
        Command::FocusRight,
        &["ctrl+w l", "ctrl+w right", "ctrl+shift+right"],
    ),
    (Command::FocusPane(0), &["g 0"]),
    (Command::FocusPane(1), &["g 1"]),
    (Command::FocusPane(2), &["g 2"]),
    (Command::FocusPane(3), &["g 3"]),
    (Command::FocusPane(4), &["g 4"]),
    (Command::FocusPane(5), &["g 5"]),
    (Command::FocusPane(6), &["g 6"]),
    (Command::FocusPane(7), &["g 7"]),
    (Command::FocusPane(8), &["g 8"]),
    (Command::FocusPane(9), &["g 9"]),
    (Command::OpenInNewPane, &["ctrl+\\", "ctrl+4", "ctrl+w v"]),
    (Command::ClosePane, &["ctrl+w c"]),
    (Command::Narrow, &["ctrl+w <", "alt+shift+h"]),
    (Command::Widen, &["ctrl+w >", "alt+shift+l"]),
    // taskpaper-terminal's keys for its shortcuts panel and for scrolling it a page (its D-55).
    (Command::Shortcuts, &["?"]),
    (Command::ScrollShortcuts(Direction::Down), &[")"]),
    (Command::ScrollShortcuts(Direction::Up), &["("]),
    // taskpaper-terminal's key: **g**o to the settings, `,` as macOS apps open theirs.
    (Command::Settings, &["g ,"]),
    // Save, as `Ctrl+S` saves in most apps; this one saves every change already, so it saves
    // to another file (D-90).
    (Command::SaveAs, &["ctrl+s"]),
    // taskpaper-terminal's conflict prompt keys, which no plain letter is since the prompt can open
    // while the user types (its D-115, D-38); k**e**ep mine, since `Ctrl+K` opens the Navigator.
    (Command::KeepMine, &["ctrl+e"]),
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

/// How `keys` read on screen: plain characters run together, as `gg`, and otherwise each key as
/// [`key_label`] writes it with a space between them, as `Ctrl+W h`.
fn sequence_label(keys: &[Key]) -> String {
    let plain = keys.iter().all(|&(modifiers, code)| {
        modifiers.is_empty() && matches!(code, KeyCode::Char(c) if c != ' ')
    });
    let presses: Vec<String> = keys.iter().copied().map(key_label).collect();
    presses.join(if plain { "" } else { " " })
}

/// How one key reads on screen: its modifiers, then the key, a letter with `Ctrl` in capitals,
/// a capital letter with a modifier marked with `Shift`, and the arrow keys as arrows.
pub(super) fn key_label((modifiers, code): Key) -> String {
    let mut label = String::new();
    let shifted = matches!(code, KeyCode::Char(c) if c.is_uppercase()) && !modifiers.is_empty();
    for (modifier, name) in [
        (KeyModifiers::CONTROL, "Ctrl+"),
        (KeyModifiers::ALT, "Alt+"),
    ] {
        if modifiers.contains(modifier) {
            label.push_str(name);
        }
    }
    if shifted || modifiers.contains(KeyModifiers::SHIFT) {
        label.push_str("Shift+");
    }
    let name = match code {
        KeyCode::Char(c) if c != ' ' && modifiers.contains(KeyModifiers::CONTROL) => {
            c.to_uppercase().to_string()
        }
        KeyCode::Up => "↑".to_owned(),
        KeyCode::Down => "↓".to_owned(),
        KeyCode::Left => "←".to_owned(),
        KeyCode::Right => "→".to_owned(),
        KeyCode::PageUp => "PgUp".to_owned(),
        KeyCode::PageDown => "PgDn".to_owned(),
        KeyCode::BackTab => "Tab".to_owned(),
        KeyCode::Backspace => "Backspace".to_owned(),
        KeyCode::Delete => "Delete".to_owned(),
        KeyCode::Enter => "Enter".to_owned(),
        code => code.to_string(),
    };
    label.push_str(&name);
    label
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
    /// Every binding, in the order its keys are written.
    commands: Vec<(Vec<Key>, Command)>,
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
    /// and does nothing else. Releases, repeats and other events change nothing.
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
        self.command(&keys)
    }

    /// The command `keys` are bound to, if any.
    fn command(&self, keys: &[Key]) -> Option<Command> {
        self.commands
            .iter()
            .find(|(bound, _)| bound == keys)
            .map(|&(_, command)| command)
    }

    /// How the keys bound to `command` read on screen, in the order they are written, separated by
    /// commas, as taskpaper-terminal's shortcuts panel writes them: a sequence of plain
    /// characters run together, such as `gg`, and others with a space between their keys, such
    /// as `Ctrl+W h`; empty when no key gives it.
    #[must_use]
    pub fn labels(&self, command: Command) -> String {
        self.commands
            .iter()
            .filter(|&&(_, bound)| bound == command)
            .map(|(keys, _)| sequence_label(keys))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Takes the clock's reading `now`, counted from any fixed moment. A sequence pressed
    /// [`TIMEOUT`] or more before ends, its keys dropped.
    pub fn tick(&mut self, now: Duration) {
        self.now = now;
        if now >= self.since + TIMEOUT {
            self.pending.clear();
        }
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

    // FR-NAV-02, FR-NAV-04, FR-VIEW-07, FR-KEY-02
    #[test]
    fn going_and_the_navigator_have_taskpaper_terminals_keys() {
        let mut keymap = Keymap::default();
        let none = KeyModifiers::NONE;
        let alt = KeyModifiers::ALT;
        let alt_shift = KeyModifiers::ALT | KeyModifiers::SHIFT;
        let go = |to| Some(Command::Go(to));
        for (events, command) in [
            (vec![char('g'), char('i')], go(Go::Inbox)),
            (vec![char('H')], go(Go::Back)),
            (vec![press(KeyCode::Left, alt_shift)], go(Go::Back)),
            (vec![char('L')], go(Go::Forward)),
            (vec![press(KeyCode::Right, alt_shift)], go(Go::Forward)),
            (vec![char('g'), char('t')], go(Go::View(View::Today))),
            (vec![char('g'), char('u')], go(Go::View(View::Upcoming))),
            (vec![char('g'), char('U')], go(Go::View(View::Upcoming))),
            (vec![char('g'), char('a')], go(Go::View(View::All))),
            (vec![char('g'), char('l')], go(Go::View(View::Logbook))),
            (vec![char('g'), char('L')], go(Go::View(View::Logbook))),
            (vec![char('g'), char('d')], go(Go::View(View::Trash))),
            (
                vec![press(KeyCode::Char('g'), KeyModifiers::CONTROL)],
                Some(Command::Navigator),
            ),
            (vec![press(KeyCode::Right, none)], None),
            (vec![char('>')], None),
            (vec![press(KeyCode::Char('l'), alt)], None),
            (vec![press(KeyCode::Left, alt)], None),
            (
                vec![press(KeyCode::Char('\''), KeyModifiers::CONTROL)],
                None,
            ),
            (vec![char('g'), char('h')], None),
            (vec![char('g'), char('z')], None),
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

    // FR-DATE-06, FR-KEY-02
    #[test]
    fn brackets_nudge_the_start_date_and_braces_the_due_date() {
        let mut keymap = Keymap::default();
        for (key, command) in [
            ('[', Command::DayEarlier(DateKind::Start)),
            (']', Command::DayLater(DateKind::Start)),
            ('{', Command::DayEarlier(DateKind::Due)),
            ('}', Command::DayLater(DateKind::Due)),
        ] {
            assert_eq!(keymap.press(&char(key)), Some(command), "{key}");
        }
    }

    // FR-OPS-02, FR-KEY-02
    #[test]
    fn alt_with_a_motion_moves() {
        let mut keymap = Keymap::default();
        let alt = KeyModifiers::ALT;
        let moved = |direction| Some(Command::Move(direction));
        for (event, command) in [
            (press(KeyCode::Char('k'), alt), moved(Direction::Up)),
            (press(KeyCode::Up, alt), moved(Direction::Up)),
            (press(KeyCode::Char('j'), alt), moved(Direction::Down)),
            (press(KeyCode::Down, alt), moved(Direction::Down)),
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
        ] {
            assert_eq!(keymap.press(&event), command, "{event:?}");
        }
    }

    // FR-PANE-02, FR-VIEW-10, FR-KEY-02
    #[test]
    fn tab_and_g_cycle_through_the_panes_and_browse_the_sidebar() {
        let mut keymap = Keymap::default();
        let ctrl = KeyModifiers::CONTROL;
        let ctrl_shift = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
        for (events, command) in [
            (
                vec![press(KeyCode::Tab, KeyModifiers::NONE)],
                Some(Command::NextPane),
            ),
            (vec![char('g'), char('n')], Some(Command::NextPane)),
            (
                vec![press(KeyCode::BackTab, KeyModifiers::SHIFT)],
                Some(Command::PreviousPane),
            ),
            (vec![char('g'), char('p')], Some(Command::PreviousPane)),
            (vec![char('g'), char('0')], Some(Command::FocusPane(0))),
            (vec![char('g'), char('9')], Some(Command::FocusPane(9))),
            (vec![press(KeyCode::Char('1'), KeyModifiers::ALT)], None),
            (
                vec![press(KeyCode::Down, ctrl)],
                Some(Command::Browse(Direction::Down)),
            ),
            (
                vec![press(KeyCode::Up, ctrl)],
                Some(Command::Browse(Direction::Up)),
            ),
            (
                vec![press(KeyCode::Left, ctrl_shift)],
                Some(Command::FocusLeft),
            ),
            (
                vec![press(KeyCode::Right, ctrl_shift)],
                Some(Command::FocusRight),
            ),
            (vec![char('g'), char('r')], None),
            (
                vec![press(KeyCode::Char('k'), ctrl)],
                Some(Command::Navigator),
            ),
        ] {
            let mut given = None;
            for event in &events {
                given = keymap.press(event);
            }
            assert_eq!(given, command, "{events:?}");
        }
    }

    // FR-TASK-02, FR-VIEW-11, FR-EDIT-06, FR-KEY-02
    #[test]
    fn completing_showing_completed_undo_and_redo_have_taskpaper_terminal_keys() {
        let mut keymap = Keymap::default();
        let ctrl = KeyModifiers::CONTROL;
        for (event, command) in [
            (char('x'), Command::ToggleCompleted),
            (char('.'), Command::ToggleCompleted),
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
    }

    // FR-NAV-06
    #[test]
    fn ctrl_p_and_g_c_open_the_command_palette() {
        let mut keymap = Keymap::default();
        let ctrl_p = press(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert_eq!(keymap.press(&ctrl_p), Some(Command::Palette));
        assert_eq!(keymap.press(&char('g')), None);
        assert_eq!(keymap.press(&char('c')), Some(Command::Palette));
    }

    // FR-NAV-05
    #[test]
    fn slash_opens_go_to_symbol() {
        let mut keymap = Keymap::default();
        assert_eq!(keymap.press(&char('/')), Some(Command::GoToSymbol));
    }

    // FR-ATT-02
    #[test]
    fn n_opens_the_note_and_shift_n_the_comments_with_alt_for_the_container_shown() {
        let mut keymap = Keymap::default();
        let shift = KeyModifiers::SHIFT;
        let alt = KeyModifiers::ALT;
        for (event, command) in [
            (char('n'), Command::OpenNote(Whose::Focused)),
            (
                press(KeyCode::Char('n'), alt),
                Command::OpenNote(Whose::Shown),
            ),
            (
                press(KeyCode::Char('n'), shift),
                Command::OpenComments(Whose::Focused),
            ),
            (
                press(KeyCode::Char('N'), alt | shift),
                Command::OpenComments(Whose::Shown),
            ),
        ] {
            assert_eq!(keymap.press(&event), Some(command));
        }
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

    // FR-ATT-02
    #[test]
    fn c_opens_the_checklist() {
        let mut keymap = Keymap::default();
        assert_eq!(keymap.press(&char('c')), Some(Command::OpenChecklist));
    }

    // FR-NAV-01, FR-KEY-02
    #[test]
    fn a_sequence_waits_for_its_next_key_showing_what_is_pending() {
        let mut keymap = Keymap::default();
        assert_eq!(keymap.pending(), "");
        for (second, command) in [
            ('g', Command::Focus(Focus::First)),
            ('j', Command::Browse(Direction::Down)),
            ('k', Command::Browse(Direction::Up)),
        ] {
            assert_eq!(keymap.press(&char('g')), None);
            assert_eq!(keymap.pending(), "g");
            assert_eq!(keymap.press(&char(second)), Some(command), "g {second}");
            assert_eq!(keymap.pending(), "");
        }
    }

    #[test]
    fn a_key_that_continues_no_sequence_cancels_it_and_does_nothing_else() {
        let mut keymap = Keymap::default();
        keymap.press(&char('g'));
        assert_eq!(keymap.press(&char('x')), None);
        assert_eq!(keymap.pending(), "");
        assert_eq!(keymap.press(&char('x')), Some(Command::ToggleCompleted));
    }

    // FR-EDIT-05, FR-ORG-06, FR-KEY-02, D-40
    #[test]
    fn adding_keys_act_at_once() {
        let mut keymap = Keymap::default();
        for (key, command) in [
            ('o', Command::NewLike(Side::Below)),
            ('O', Command::NewLike(Side::Above)),
            ('a', Command::AddTask(End::Bottom)),
            ('A', Command::AddTask(End::Top)),
            ('s', Command::AddSection(End::Bottom)),
            ('S', Command::AddSection(End::Top)),
            ('p', Command::AddProject(End::Bottom)),
            ('P', Command::AddProject(End::Top)),
            ('i', Command::AddProjectInArea(End::Bottom)),
            ('I', Command::AddProjectInArea(End::Top)),
        ] {
            assert_eq!(keymap.press(&char(key)), Some(command), "{key}");
        }
        assert_eq!(keymap.press(&char('R')), Some(Command::RenameContainer));
    }

    // FR-KEY-02
    #[test]
    fn a_sequence_ends_a_second_after_its_first_key() {
        let mut keymap = Keymap::default();
        assert_eq!(keymap.deadline(), None);
        keymap.tick(Duration::from_secs(5));
        keymap.press(&char('g'));
        assert_eq!(keymap.deadline(), Some(Duration::from_secs(6)));
        keymap.tick(Duration::from_millis(5999));
        assert_eq!(keymap.pending(), "g");
        keymap.tick(Duration::from_secs(6));
        assert_eq!((keymap.pending(), keymap.deadline()), ("", None));
        assert_eq!(keymap.press(&char('x')), Some(Command::ToggleCompleted));
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

    // FR-KEY-03
    #[test]
    fn question_mark_shows_the_shortcuts_and_parentheses_scroll_them() {
        let mut keymap = Keymap::default();
        for (c, command) in [
            ('?', Command::Shortcuts),
            (')', Command::ScrollShortcuts(Direction::Down)),
            ('(', Command::ScrollShortcuts(Direction::Up)),
        ] {
            assert_eq!(keymap.press(&char(c)), Some(command), "{c}");
        }
    }

    // FR-KEY-03
    #[test]
    fn labels_write_keys_as_the_shortcuts_panel_shows_them() {
        let label = |keys| parse_sequence(keys).map(|keys| sequence_label(&keys));
        for (keys, expected) in [
            ("j", "j"),
            ("G", "G"),
            ("g g", "gg"),
            ("o s", "os"),
            ("space", "Space"),
            ("g space", "g Space"),
            ("ctrl+q", "Ctrl+Q"),
            ("ctrl+'", "Ctrl+'"),
            ("ctrl+space", "Ctrl+Space"),
            ("alt+k", "Alt+k"),
            ("alt+K", "Alt+Shift+K"),
            ("ctrl+shift+z", "Ctrl+Shift+Z"),
            ("alt+shift+left", "Alt+Shift+←"),
            ("shift+down", "Shift+↓"),
            ("shift+tab", "Shift+Tab"),
            ("up", "↑"),
            ("right", "→"),
            ("ctrl+down", "Ctrl+↓"),
            ("pagedown", "PgDn"),
            ("pageup", "PgUp"),
            ("delete", "Delete"),
            ("backspace", "Backspace"),
            ("enter", "Enter"),
            ("ctrl+enter", "Ctrl+Enter"),
            ("esc", "Esc"),
            ("home", "Home"),
            ("tab", "Tab"),
            ("ctrl+w h", "Ctrl+W h"),
            ("f1", "F1"),
        ] {
            assert_eq!(label(keys).as_deref(), Some(expected), "{keys}");
        }
        let keymap = Keymap::default();
        assert_eq!(keymap.labels(Command::Go(Go::Inbox)), "gi");
        assert_eq!(keymap.labels(Command::ToggleCompleted), "x, ., Ctrl+Enter");
        assert_eq!(keymap.labels(Command::Redo), "Z, Ctrl+Shift+Z, Ctrl+Y");
        assert_eq!(keymap.labels(Command::PreviousPane), "Shift+Tab, gp");
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
