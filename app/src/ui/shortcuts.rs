//! The shortcuts panel at the right of the panes, as taskpaper-terminal draws it
//! ([ui.md §4](../../../specs/reference/taskpaper-terminal/specs/ui.md#4-shortcuts-panel-fr-key-04),
//! FR-KEY-03): the commands the app takes now with their keys as the keymap binds them, and while
//! a picker, the date prompt or edit mode has the keys, that window's own keys first, read from
//! the tables that bind them.

use crossterm::event::KeyModifiers;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Padding, Widget},
};
use unicode_width::UnicodeWidthStr;

use super::{
    Theme,
    attachments::{self, PanelKey, Scroll},
    date_prompt, edit_dialog,
    edit_keys::{CHECKLIST, COMMENT, DESCRIPTION, DESCRIPTION_MOTIONS, EDIT, PASTE},
    keymap::{Key, key_label},
    picker, quick_add, save_as, search, settings, text_keys,
};
use crate::app::{
    AttachmentsInput, DateInput, Direction, EditDialogInput, EditInput, Motion, Part, PickerInput,
    QuickAddInput, REGISTRY, SaveAsInput, SearchInput, SettingsInput, TextEdit,
};

/// A group of the panel: its heading, and each command's name with how its keys read.
pub(super) type Listing = (&'static str, Vec<(String, String)>);

/// What the terminal has shown of its keyboard, which the panel notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Keyboard {
    /// Whether the kitty keyboard protocol is active.
    pub(super) kitty: bool,
    /// Whether a key with `Alt` has arrived, which on macOS takes a terminal setting.
    pub(super) alt_seen: bool,
}

/// The window that has the keys, when one does that binds keys of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Window {
    /// The Navigator, Move to or the tag picker.
    Picker,
    /// The date prompt.
    DatePrompt,
    /// The edit dialog, while its tag picker is closed.
    EditDialog,
    /// The Save As prompt.
    SaveAs,
    /// The Quick Add panel, while its picker is closed.
    QuickAdd,
    /// The settings window.
    Settings,
    /// Typing the words to search for.
    Search,
    /// The attachments panel, while nothing in it is edited.
    Attachments,
    /// Edit mode, editing this part of an item.
    Editing(Part),
    /// Edit mode, editing a checklist item's text in the attachments panel.
    EditingChecklistItem,
}

/// The groups of keys `window` binds: its own, then the text keys it leaves as they are.
pub(super) fn window_keys(window: Window) -> Vec<Listing> {
    let (heading, mut own): (_, Vec<(Key, String)>) = match window {
        Window::Picker => ("Picker", named(picker::PICKER, picker_name)),
        Window::DatePrompt => ("Date Prompt", named(date_prompt::PROMPT, date_name)),
        Window::EditDialog => ("Edit Dialog", named(edit_dialog::KEYS, dialog_name)),
        Window::SaveAs => ("Save As", named(save_as::KEYS, save_as_name)),
        Window::QuickAdd => ("Quick Add", named(quick_add::KEYS, quick_add_name)),
        Window::Settings => ("Settings", named(settings::KEYS, settings_name)),
        Window::Search => ("Search", named(search::KEYS, search_name)),
        Window::Attachments => (
            "Attachments Panel",
            named(attachments::KEYS, |key| {
                match key {
                    PanelKey::Input(AttachmentsInput::Close) => "Close",
                    PanelKey::Input(AttachmentsInput::Delete) => "Delete",
                    // Edit is the only other input a key gives as it is.
                    PanelKey::Input(_) => "Edit",
                    PanelKey::Command(command) => {
                        REGISTRY
                            .iter()
                            .find(|entry| entry.command == command)
                            .expect("the panel's commands are registered")
                            .title
                    }
                    PanelKey::Nest(Direction::Down) => "Indent or Next Pane",
                    PanelKey::Nest(Direction::Up) => "Outdent or Previous Pane",
                    PanelKey::AddComment => "Add Comment",
                    PanelKey::Widen => "Full or Half Width",
                    PanelKey::Scroll(Scroll::Lines(lines)) if lines < 0 => "Scroll Up",
                    PanelKey::Scroll(Scroll::Lines(_)) => "Scroll Down",
                    PanelKey::Scroll(Scroll::Pages(pages)) if pages < 0 => "Page Up",
                    PanelKey::Scroll(Scroll::Pages(_)) => "Page Down",
                    PanelKey::Scroll(Scroll::Top) => "Top",
                    PanelKey::Scroll(Scroll::Bottom) => "Bottom",
                }
                .to_owned()
            }),
        ),
        Window::Editing(Part::Title) => ("Editing", named(EDIT, edit_name)),
        Window::EditingChecklistItem => (
            "Editing Checklist Item",
            named(CHECKLIST, edit_name)
                .into_iter()
                .chain(
                    named(EDIT, edit_name)
                        .into_iter()
                        .filter(|&(key, _)| CHECKLIST.iter().all(|&(bound, _)| bound != key)),
                )
                .collect(),
        ),
        Window::Editing(Part::Comment(_)) => ("Editing Comment", named(COMMENT, edit_name)),
        Window::Editing(part @ (Part::Description | Part::Note)) => {
            let motions = DESCRIPTION_MOTIONS
                .iter()
                .flat_map(|&((modifiers, code), motion)| {
                    let name = motion_name(motion(0));
                    [
                        ((modifiers, code), name.clone()),
                        (
                            (modifiers.union(KeyModifiers::SHIFT), code),
                            format!("Select {name}"),
                        ),
                    ]
                });
            let heading = if part == Part::Note {
                "Editing Note"
            } else {
                "Editing Description"
            };
            (
                heading,
                motions.chain(named(DESCRIPTION, edit_name)).collect(),
            )
        }
    };
    if let Window::Editing(_) | Window::EditingChecklistItem = window {
        own.push((PASTE, "Paste".to_owned()));
    }
    if matches!(window, Window::Attachments | Window::Settings) {
        return vec![(heading, merged(own))];
    }
    let text = text_keys::TEXT
        .iter()
        .filter(|&&(key, _)| own.iter().all(|&(bound, _)| bound != key))
        .map(|&(key, edit)| (key, text_name(edit)))
        .collect();
    vec![(heading, merged(own)), ("Text", merged(text))]
}

/// Each key of `table` with the name `name` gives what it does.
fn named<T: Copy>(table: &[(Key, T)], name: impl Fn(T) -> String) -> Vec<(Key, String)> {
    table
        .iter()
        .map(|&(key, input)| (key, name(input)))
        .collect()
}

/// Each name of `keys` once, where it first comes, with how its keys read, separated by commas.
fn merged(keys: Vec<(Key, String)>) -> Vec<(String, String)> {
    let mut listed: Vec<(String, String)> = Vec::new();
    for (key, name) in keys {
        let label = key_label(key);
        match listed.iter_mut().find(|(listed, _)| *listed == name) {
            Some((_, labels)) => {
                labels.push_str(", ");
                labels.push_str(&label);
            }
            None => listed.push((name, label)),
        }
    }
    listed
}

/// What a picker's key does, as the panel names it.
fn picker_name(input: PickerInput) -> String {
    match input {
        PickerInput::Edit(edit) => text_name(edit),
        PickerInput::Previous => "Previous Match".to_owned(),
        PickerInput::Next => "Next Match".to_owned(),
        PickerInput::Choose => "Choose".to_owned(),
        PickerInput::ChooseInNewPane => "Open in New Pane".to_owned(),
        PickerInput::Dismiss => "Close".to_owned(),
    }
}

/// What a key of the Quick Add panel does, as the panel names it.
fn quick_add_name(input: QuickAddInput) -> String {
    match input {
        QuickAddInput::Edit(edit) => text_name(edit),
        QuickAddInput::Add => "Add Task".to_owned(),
        QuickAddInput::Cancel => "Cancel".to_owned(),
    }
}

/// What a key of the date prompt does, as the panel names it.
fn date_name(input: DateInput) -> String {
    match input {
        DateInput::Edit(edit) => text_name(edit),
        DateInput::Apply => "Set Date".to_owned(),
        DateInput::Cancel => "Cancel".to_owned(),
    }
}

/// What a key of the edit dialog does, as the panel names it.
fn dialog_name(input: EditDialogInput) -> String {
    match input {
        EditDialogInput::Next => "Next Field",
        EditDialogInput::Previous => "Previous Field",
        EditDialogInput::Accept => "Save",
        // The dialog's table binds no other input to a key of its own.
        EditDialogInput::Cancel | EditDialogInput::Edit(_) | EditDialogInput::Tags(_) => "Cancel",
    }
    .to_owned()
}

/// What a key of the Save As prompt does, as the panel names it.
fn save_as_name(input: SaveAsInput) -> String {
    match input {
        SaveAsInput::Edit(edit) => text_name(edit),
        SaveAsInput::Apply => "Save".to_owned(),
        SaveAsInput::Cancel => "Cancel".to_owned(),
    }
}

/// What a key does while the words to search for are typed, as the panel names it.
fn search_name(input: SearchInput) -> String {
    match input {
        SearchInput::Edit(edit) => text_name(edit),
        SearchInput::Done => "Go to Results".to_owned(),
    }
}

/// What a key of the settings window does, as the panel names it.
fn settings_name(input: SettingsInput) -> String {
    match input {
        SettingsInput::Focus(Direction::Up) => "Previous Setting",
        SettingsInput::Focus(Direction::Down) => "Next Setting",
        SettingsInput::Turn(Direction::Down) => "Next Value",
        SettingsInput::Turn(Direction::Up) => "Previous Value",
        SettingsInput::Reset => "Reset to Default",
        SettingsInput::Close => "Close",
    }
    .to_owned()
}

/// What an edit-mode key does, as the panel names it.
fn edit_name(input: EditInput) -> String {
    match input {
        EditInput::Text(edit) => text_name(edit),
        EditInput::Next => "Done, Then Next Item".to_owned(),
        EditInput::Done => "Done".to_owned(),
        EditInput::Cancel => "Cancel".to_owned(),
        EditInput::Indent => "Indent".to_owned(),
        EditInput::Outdent => "Outdent".to_owned(),
    }
}

/// What a text key does, as the panel names it.
fn text_name(edit: TextEdit) -> String {
    match edit {
        TextEdit::Insert('\n') => "New Line".to_owned(),
        TextEdit::Insert(_) => "Type".to_owned(),
        TextEdit::Move {
            motion,
            select: false,
        } => motion_name(motion),
        TextEdit::Move {
            motion,
            select: true,
        } => format!("Select {}", motion_name(motion)),
        TextEdit::DeleteBackward => "Delete Back".to_owned(),
        TextEdit::DeleteForward => "Delete Forward".to_owned(),
        TextEdit::DeleteWordBackward => "Delete Word Back".to_owned(),
        TextEdit::DeleteWordForward => "Delete Word Forward".to_owned(),
        TextEdit::DeleteLineBackward => "Delete to Line Start".to_owned(),
        TextEdit::SelectAll => "Select All".to_owned(),
    }
}

/// Where a caret motion goes, as the panel names it.
fn motion_name(motion: Motion) -> String {
    match motion {
        Motion::Left => "Left",
        Motion::Right => "Right",
        Motion::WordLeft => "Word Left",
        Motion::WordRight => "Word Right",
        Motion::LineStart => "Line Start",
        Motion::LineEnd => "Line End",
        Motion::Up { .. } => "Line Up",
        Motion::Down { .. } => "Line Down",
        Motion::Start => "Text Start",
        Motion::End => "Text End",
        Motion::To(_) => "Click",
    }
    .to_owned()
}

/// The columns the panel takes of panes `width` wide: a quarter, wide enough for a command's
/// name beside several keys.
pub(super) fn width(width: u16) -> u16 {
    (width / 4).clamp(36, 48)
}

/// `keys`, separated by commas, broken between keys into lines: the first `first` columns wide,
/// the rest `rest` wide, each line ending in the comma before the key on the next. The first line
/// is empty when not even the first key fits on it; a key wider than a line has one of its own.
fn broken(keys: &str, first: usize, rest: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    let mut room = first;
    let mut keys = keys.split(", ").peekable();
    while let Some(key) = keys.next() {
        let key = if keys.peek().is_some() {
            format!("{key},")
        } else {
            key.to_owned()
        };
        let line = lines.last_mut().expect("lines starts with one");
        let after = if line.is_empty() { 0 } else { line.width() + 1 };
        if after + key.width() <= room {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&key);
        } else {
            lines.push(key);
            room = rest;
        }
    }
    lines
}

/// The shortcuts panel: titled "Shortcuts" beside a rule down its left edge, each group's name
/// in the headers color over its commands, each command's name with its keys at the right in
/// the primary color, scrolled, with a `↑ more` line first while lines are hidden above and a
/// `↓ more` line last while lines are hidden below, each with the keys that scroll that way, and
/// below it what the keyboard reports: how to send `Alt` on macOS until an `Alt` key arrives, and
/// whether the kitty protocol is on.
pub(super) struct ShortcutsPanel<'a> {
    groups: &'a [Listing],
    keyboard: Keyboard,
    theme: &'a Theme,
    scroll: usize,
    keys: [&'a str; 2],
}

/// Which list lines a panel shows: from the first line on, as many as there is room for, and
/// whether lines are hidden above and below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shown {
    first: usize,
    count: usize,
    above: bool,
    below: bool,
}

impl<'a> ShortcutsPanel<'a> {
    /// The panel in `theme` listing `groups` from the top, its `↑ more` and `↓ more` lines
    /// naming `keys`, the keys that scroll up and down, and noting what `keyboard` reports.
    pub(super) fn new(
        groups: &'a [Listing],
        keyboard: Keyboard,
        theme: &'a Theme,
        keys: [&'a str; 2],
    ) -> Self {
        Self {
            groups,
            keyboard,
            theme,
            scroll: 0,
            keys,
        }
    }

    /// The panel with its list scrolled `scroll` lines down, as far as the list goes.
    pub(super) fn scrolled(self, scroll: usize) -> Self {
        Self { scroll, ..self }
    }

    /// How far the list scrolls when the panel is drawn over `area`, and how many lines a page
    /// scrolls: the lines shown between the `↑ more` and `↓ more` lines, at least one.
    pub(super) fn scroll_range(&self, area: Rect) -> (usize, usize) {
        let list = self.list(area);
        let end = self.shown(list, usize::MAX).first;
        (end, usize::from(list.height).saturating_sub(2).max(1))
    }

    /// The list's area when the panel is drawn over `area`: inside the frame, above the notes.
    fn list(&self, area: Rect) -> Rect {
        let height = u16::try_from(self.notes().len()).unwrap_or(u16::MAX);
        let [list, _] = Layout::vertical([Constraint::Fill(1), Constraint::Length(height)])
            .areas(self.frame().inner(area));
        list
    }

    /// The lines the list drawn over `list` shows scrolled `scroll` lines down, as far as the
    /// list goes: all of them when they fit, else as many as fit beside the `more` lines.
    fn shown(&self, list: Rect, scroll: usize) -> Shown {
        let height = usize::from(list.height);
        let lines = self.rows(list.width).len();
        if lines <= height {
            return Shown {
                first: 0,
                count: lines,
                above: false,
                below: false,
            };
        }
        // At the end the `↑ more` line takes a row and the last lines take the rest.
        let first = scroll.min(lines - height.saturating_sub(1));
        let above = first > 0;
        let below = first + height.saturating_sub(1) < lines;
        Shown {
            first,
            count: height.saturating_sub(usize::from(above) + usize::from(below)),
            above,
            below,
        }
    }

    /// The panel's frame: a rule down its left edge, its title and padding.
    fn frame(&self) -> Block<'static> {
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::new().fg(self.theme.rule))
            .padding(Padding::horizontal(1))
            .title(Line::from("Shortcuts").bold())
    }

    /// The rows of a list `width` columns wide: each group's name, then each of its commands'
    /// names with their keys, the keys that do not fit beside the name going on in the rows
    /// below it, broken between keys (D-88).
    fn rows(&self, width: u16) -> Vec<(Line<'static>, Line<'static>)> {
        let width = usize::from(width);
        let mut rows = Vec::new();
        for (heading, entries) in self.groups {
            rows.push((
                Line::styled(*heading, Style::new().fg(self.theme.headers).bold()),
                Line::default(),
            ));
            for (name, keys) in entries {
                let name = format!(" {name}");
                let beside = width.saturating_sub(name.width() + 1);
                let mut name = Some(Line::from(name));
                for keys in broken(keys, beside, width) {
                    rows.push((
                        name.take().unwrap_or_default(),
                        Line::styled(keys, Style::new().fg(self.theme.primary)),
                    ));
                }
            }
        }
        rows
    }

    /// The row saying more lines are hidden, `↑` above or `↓` below, with the keys that show them.
    fn more(&self, arrow: &str, keys: &str) -> (Line<'static>, Line<'static>) {
        (
            Line::from(format!("{arrow} more")).dim(),
            Line::styled(keys.to_owned(), Style::new().fg(self.theme.primary)),
        )
    }

    /// The notes under the list, after a blank line.
    fn notes(&self) -> Vec<Line<'static>> {
        let mut notes = vec![Line::default()];
        if !self.keyboard.alt_seen {
            notes.push(Line::from("No Alt key seen yet: on macOS"));
            notes.push(Line::from("set Option as Meta or Esc+"));
        }
        let kitty = if self.keyboard.kitty { "on" } else { "off" };
        notes.push(Line::from(format!("Kitty keyboard protocol: {kitty}")));
        notes.into_iter().map(Line::dim).collect()
    }
}

impl Widget for ShortcutsPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let frame = self.frame();
        let inner = frame.inner(area);
        frame.render(area, buf);
        let list = self.list(area);
        let shown = self.shown(list, self.scroll);
        let [up, down] = self.keys;
        let rows = shown
            .above
            .then(|| self.more("↑", up))
            .into_iter()
            .chain(
                self.rows(list.width)
                    .into_iter()
                    .skip(shown.first)
                    .take(shown.count),
            )
            .chain(shown.below.then(|| self.more("↓", down)));
        for ((name, keys), row) in rows.zip(list.rows()) {
            let width = u16::try_from(keys.width()).unwrap_or(u16::MAX);
            let [name_area, keys_area] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)])
                    .spacing(1)
                    .areas(row);
            name.render(name_area, buf);
            keys.right_aligned().render(keys_area, buf);
        }
        let footer = inner.rows().skip(usize::from(list.height));
        for (note, row) in self.notes().into_iter().zip(footer) {
            note.render(row, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;
    use ratatui::buffer::Cell;

    use super::*;
    use crate::ui::BuiltIn;

    // FR-KEY-03
    #[test]
    fn every_text_edit_has_a_name() {
        let names: Vec<_> = [
            TextEdit::Insert('a'),
            TextEdit::DeleteWordForward,
            TextEdit::Move {
                motion: Motion::To(0),
                select: false,
            },
        ]
        .into_iter()
        .map(text_name)
        .collect();
        assert_eq!(names, ["Type", "Delete Word Forward", "Click"]);
        let edit = TextEdit::SelectAll;
        assert_eq!(picker_name(PickerInput::Edit(edit)), "Select All");
        assert_eq!(date_name(DateInput::Edit(edit)), "Select All");
        assert_eq!(quick_add_name(QuickAddInput::Edit(edit)), "Select All");
        assert_eq!(search_name(SearchInput::Edit(edit)), "Select All");
        assert_eq!(save_as_name(SaveAsInput::Edit(edit)), "Select All");
    }

    // FR-KEY-03, FR-ATT-04, FR-EDIT-08
    #[test]
    fn each_window_lists_its_own_keys_then_the_text_keys_it_leaves() {
        let headings = |window| -> Vec<_> {
            window_keys(window)
                .into_iter()
                .map(|(heading, keys)| (heading, keys.len()))
                .collect()
        };
        assert_eq!(headings(Window::Picker), [("Picker", 5), ("Text", 19)]);
        assert_eq!(
            headings(Window::DatePrompt),
            [("Date Prompt", 2), ("Text", 19)]
        );
        assert_eq!(
            window_keys(Window::SaveAs),
            [
                (
                    "Save As",
                    vec![
                        ("Save".to_owned(), "Enter".to_owned()),
                        ("Cancel".to_owned(), "Esc".to_owned())
                    ]
                ),
                window_keys(Window::DatePrompt)[1].clone(),
            ]
        );
        assert_eq!(
            window_keys(Window::QuickAdd),
            [
                (
                    "Quick Add",
                    vec![
                        ("Add Task".to_owned(), "Enter".to_owned()),
                        ("Cancel".to_owned(), "Esc".to_owned())
                    ]
                ),
                window_keys(Window::DatePrompt)[1].clone(),
            ]
        );
        assert_eq!(
            headings(Window::Editing(Part::Title)),
            [("Editing", 2), ("Text", 19)]
        );
        assert_eq!(
            headings(Window::Editing(Part::Description)),
            [("Editing Description", 15), ("Text", 15)]
        );
        assert_eq!(
            headings(Window::Editing(Part::Note)),
            [("Editing Note", 15), ("Text", 15)]
        );
        assert_eq!(
            headings(Window::Editing(Part::Comment(0))),
            [("Editing Comment", 3), ("Text", 19)]
        );
        assert_eq!(
            window_keys(Window::EditingChecklistItem)[0],
            (
                "Editing Checklist Item",
                [
                    ("Done, Then Next Item", "Enter"),
                    ("Indent", "Tab"),
                    ("Outdent", "Shift+Tab"),
                    ("Done", "Esc"),
                    ("Paste", "Ctrl+V"),
                ]
                .map(|(name, keys)| (name.to_owned(), keys.to_owned()))
                .to_vec()
            )
        );
        let (_, editing) = &window_keys(Window::Editing(Part::Title))[0];
        assert!(editing.contains(&("Paste".to_owned(), "Ctrl+V".to_owned())));
        assert_eq!(headings(Window::Attachments), [("Attachments Panel", 15)]);
        assert_eq!(headings(Window::Settings), [("Settings", 6)]);
        assert_eq!(headings(Window::Search), [("Search", 1), ("Text", 19)]);
        assert_eq!(
            headings(Window::EditDialog),
            [("Edit Dialog", 4), ("Text", 19)]
        );
    }

    // FR-KEY-03
    #[test]
    fn keys_with_one_name_are_listed_together_where_the_first_one_is() {
        let none = KeyModifiers::NONE;
        let keys = vec![
            ((none, KeyCode::Up), "Up".to_owned()),
            ((none, KeyCode::Char('k')), "Down".to_owned()),
            ((KeyModifiers::CONTROL, KeyCode::Char('p')), "Up".to_owned()),
        ];
        assert_eq!(
            merged(keys),
            [
                ("Up".to_owned(), "↑, Ctrl+P".to_owned()),
                ("Down".to_owned(), "k".to_owned())
            ]
        );
    }

    // FR-KEY-03
    #[test]
    fn a_name_is_never_cut_and_keys_that_do_not_fit_beside_it_go_on_below_broken_between_keys() {
        let groups: [Listing; 1] = [(
            "View and Panels",
            [
                ("Focus Pane Left", "Ctrl+W h, Ctrl+W ←, Ctrl+Shift+←"),
                ("Narrow Pane", "Ctrl+W <"),
                ("Previous Sidebar Entry", "gk, Ctrl+↑"),
            ]
            .map(|(name, keys)| (name.to_owned(), keys.to_owned()))
            .to_vec(),
        )];
        let theme = BuiltIn::default().colors();
        let keyboard = Keyboard {
            kitty: true,
            alt_seen: true,
        };
        let panel = ShortcutsPanel::new(&groups, keyboard, &theme, ["(", ")"]);
        assert_eq!(
            panel.scroll_range(Rect::new(0, 0, 36, 6)),
            (4, 1),
            "scrolling counts the lines the keys go on to"
        );
        let area = Rect::new(0, 0, 36, 10);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        let lines: Vec<String> = buf
            .content()
            .chunks(usize::from(area.width))
            .map(|line| line.iter().map(Cell::symbol).collect())
            .collect();
        assert_eq!(
            lines,
            [
                "│Shortcuts                          ",
                "│ View and Panels                   ",
                "│  Focus Pane Left        Ctrl+W h, ",
                "│            Ctrl+W ←, Ctrl+Shift+← ",
                "│  Narrow Pane             Ctrl+W < ",
                "│  Previous Sidebar Entry       gk, ",
                "│                            Ctrl+↑ ",
                "│                                   ",
                "│                                   ",
                "│ Kitty keyboard protocol: on       ",
            ]
        );
    }
}
