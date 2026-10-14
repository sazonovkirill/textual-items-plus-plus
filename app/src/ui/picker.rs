//! The fuzzy picker in the terminal (FR-UI-09): the keys that drive it and the window it is drawn
//! in, a centered box with the query on its first line and the matching entries below it.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Widget},
};

use super::{
    Icon, IconSet, Theme,
    keymap::{self, Key},
    text_keys,
};
use crate::app::{PickerInput, PickerView};

/// The keys of an open picker besides the text keys of its query.
pub(super) const PICKER: &[(Key, PickerInput)] = &[
    ((KeyModifiers::NONE, KeyCode::Esc), PickerInput::Dismiss),
    ((KeyModifiers::NONE, KeyCode::Enter), PickerInput::Choose),
    // taskpaper-terminal's keys to open the choice in a new pane, `Ctrl+T` as fzf and Telescope
    // open a tab, for terminals that take `Alt+Enter`.
    (
        (KeyModifiers::ALT, KeyCode::Enter),
        PickerInput::ChooseInNewPane,
    ),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('t')),
        PickerInput::ChooseInNewPane,
    ),
    ((KeyModifiers::NONE, KeyCode::Up), PickerInput::Previous),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('p')),
        PickerInput::Previous,
    ),
    ((KeyModifiers::NONE, KeyCode::Down), PickerInput::Next),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('n')),
        PickerInput::Next,
    ),
];

/// What `event` does to an open picker: its picker key, else the change to the query its text
/// key asks for. Only key presses count.
#[must_use]
pub fn picker_input(event: &Event) -> Option<PickerInput> {
    let key = keymap::key_press(event)?;
    PICKER
        .iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
        .or_else(|| text_keys::text_edit(key).map(PickerInput::Edit))
}

/// The largest the window gets.
const SIZE: (u16, u16) = (80, 20);

/// What stands before the query.
const PROMPT: &str = "› ";

/// An open picker titled with what it chooses, such as Navigator: the query, its selection in the
/// selection colors, then each matching entry's icon and text and, dimmed after it, its detail,
/// the focused one in the selection colors and kept in view, or a dimmed line saying nothing
/// matches.
pub struct PickerPopup<'a, T> {
    title: &'a str,
    view: &'a PickerView<'a, T>,
    icon: &'a dyn Fn(&T) -> Icon,
    depth: Option<&'a dyn Fn(&T) -> usize>,
    badge: Option<&'a dyn Fn(&T) -> String>,
    color: Option<fn(&Theme, &T) -> Color>,
    icons: IconSet,
    theme: &'a Theme,
}

impl<'a, T> PickerPopup<'a, T> {
    /// The window of `view` titled `title`, in `theme`, each entry after the icon `icon` gives
    /// its value, drawn from `icons`.
    #[must_use]
    pub fn new(
        title: &'a str,
        view: &'a PickerView<'a, T>,
        icon: &'a dyn Fn(&T) -> Icon,
        icons: IconSet,
        theme: &'a Theme,
    ) -> Self {
        Self {
            title,
            view,
            icon,
            depth: None,
            badge: None,
            color: None,
            icons,
            theme,
        }
    }

    /// The window of a picker whose candidates, as given, are a tree, each `depth` levels deep:
    /// with an empty query it lists them indented two columns a level, without their details,
    /// which the tree shows.
    #[must_use]
    pub fn tree(mut self, depth: &'a dyn Fn(&T) -> usize) -> Self {
        self.depth = Some(depth);
        self
    }

    /// The window with each entry's icon and text in the color of the theme `color` gives it,
    /// such as its kind's.
    #[must_use]
    pub fn color(mut self, color: fn(&Theme, &T) -> Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The window with what `badge` gives each entry, such as a count, dimmed at
    /// the entry's right end; nothing for an empty one.
    #[must_use]
    pub fn badge(mut self, badge: &'a dyn Fn(&T) -> String) -> Self {
        self.badge = Some(badge);
        self
    }

    /// Where the window goes in `area`: centered, as wide as it can be up to 80 columns and as
    /// tall as its entries with the query and the border, up to 20 rows.
    #[must_use]
    pub fn area(&self, area: Rect) -> Rect {
        let entries = u16::try_from(self.view.entries.len().max(1)).unwrap_or(u16::MAX);
        let height = entries.saturating_add(3).min(SIZE.1);
        area.centered(Constraint::Length(SIZE.0), Constraint::Length(height))
    }

    /// Where the terminal's cursor goes when the window is drawn over `area`: at the query's
    /// caret, or at the window's last column when the query is wider than the window.
    #[must_use]
    pub fn caret(&self, area: Rect) -> Position {
        let inner = Block::bordered().inner(self.area(area));
        let query = self.view.query;
        let before = query.text().get(..query.caret()).unwrap_or_default();
        let column = Line::from(format!("{PROMPT}{before}")).width();
        let column = u16::try_from(column).unwrap_or(u16::MAX);
        let last = inner.right().saturating_sub(1);
        Position::new(inner.x.saturating_add(column).min(last), inner.y)
    }

    /// The query line, its selection in the selection colors.
    fn query(&self) -> Line<'a> {
        let query = self.view.query;
        let text = query.text();
        let selection = query.selection().unwrap_or(query.caret()..query.caret());
        let selected = Style::new()
            .bg(self.theme.selection)
            .fg(self.theme.band_text);
        Line::from(vec![
            Span::styled(PROMPT, Style::new().fg(self.theme.primary)),
            Span::raw(text.get(..selection.start).unwrap_or_default()),
            Span::styled(text.get(selection.clone()).unwrap_or_default(), selected),
            Span::raw(text.get(selection.end..).unwrap_or_default()),
        ])
    }
}

impl<T> Widget for PickerPopup<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = self.area(area);
        let theme = self.theme;
        Clear.render(area, buf);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(Style::new().fg(theme.primary))
            .title(self.title);
        let inner = frame.inner(area);
        frame.render(area, buf);
        let mut rows = inner.rows();
        if let Some(row) = rows.next() {
            self.query().render(row, buf);
        }
        let rows: Vec<Rect> = rows.collect();
        let dim = Style::new().fg(theme.dim_text);
        if self.view.entries.is_empty() {
            if let Some(&row) = rows.first() {
                Line::styled(" No matches", dim).render(row, buf);
            }
            return;
        }
        let focused = self.view.focused;
        let offset = (focused + 1).saturating_sub(rows.len());
        let entries = self.view.entries.iter().enumerate().skip(offset);
        let tree = self.depth.filter(|_| self.view.query.text().is_empty());
        for ((index, entry), row) in entries.zip(rows) {
            let indent = tree.map_or(0, |depth| depth(&entry.value) * 2);
            let style = self.color.map_or_else(Style::new, |color| {
                Style::new().fg(color(theme, &entry.value))
            });
            let mut line = Line::from(vec![
                Span::raw(" ".repeat(indent + 1)),
                Span::styled((self.icon)(&entry.value).glyph(self.icons), style),
                Span::raw(" "),
                Span::styled(entry.text.as_str(), style),
            ]);
            if tree.is_none() && !entry.detail.is_empty() {
                line.push_span(Span::styled(format!("  {}", entry.detail), dim));
            }
            if index == focused {
                buf.set_style(row, Style::new().bg(theme.selection).fg(theme.band_text));
            }
            line.render(row, buf);
            let badge = self
                .badge
                .map(|badge| badge(&entry.value))
                .unwrap_or_default();
            if !badge.is_empty() {
                let badge = Span::styled(format!(" {badge} "), Style::new().fg(theme.dim_text));
                let width = u16::try_from(badge.width()).unwrap_or(u16::MAX);
                let at = Rect::new(row.right().saturating_sub(width), row.y, width, 1);
                badge.render(at.intersection(row), buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyEventKind};

    use super::*;
    use crate::{
        app::{Candidate, Motion, Picker, TextEdit},
        ui::BuiltIn,
    };

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    // FR-UI-09
    #[test]
    fn escape_closes_enter_chooses_and_arrows_or_ctrl_p_and_n_move() {
        let none = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;
        for (code, modifiers, expected) in [
            (KeyCode::Esc, none, PickerInput::Dismiss),
            (KeyCode::Enter, none, PickerInput::Choose),
            (
                KeyCode::Enter,
                KeyModifiers::ALT,
                PickerInput::ChooseInNewPane,
            ),
            (KeyCode::Char('t'), ctrl, PickerInput::ChooseInNewPane),
            (KeyCode::Up, none, PickerInput::Previous),
            (KeyCode::Char('p'), ctrl, PickerInput::Previous),
            (KeyCode::Down, none, PickerInput::Next),
            (KeyCode::Char('n'), ctrl, PickerInput::Next),
        ] {
            let event = press(code, modifiers);
            assert_eq!(picker_input(&event), Some(expected), "{code:?}");
        }
    }

    // FR-UI-09
    #[test]
    fn other_keys_edit_the_query_as_in_any_text_field() {
        let none = KeyModifiers::NONE;
        for (event, expected) in [
            (press(KeyCode::Char('p'), none), TextEdit::Insert('p')),
            (
                press(KeyCode::Char('P'), KeyModifiers::SHIFT),
                TextEdit::Insert('P'),
            ),
            (press(KeyCode::Backspace, none), TextEdit::DeleteBackward),
            (
                press(KeyCode::Left, none),
                TextEdit::Move {
                    motion: Motion::Left,
                    select: false,
                },
            ),
            (
                press(KeyCode::Down, KeyModifiers::SHIFT),
                TextEdit::Move {
                    motion: Motion::End,
                    select: true,
                },
            ),
        ] {
            assert_eq!(picker_input(&event), Some(PickerInput::Edit(expected)));
        }
        assert_eq!(picker_input(&press(KeyCode::Tab, none)), None);
    }

    #[test]
    fn only_presses_drive_a_picker() {
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            let mut key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            key.kind = kind;
            assert_eq!(picker_input(&Event::Key(key)), None, "{kind:?}");
        }
        assert_eq!(picker_input(&Event::FocusGained), None);
    }

    /// A theme whose roles all differ, so each style shows which role drew it.
    fn theme() -> Theme {
        BuiltIn::GithubLight.colors()
    }

    fn candidate(value: u8, text: &str, detail: &str) -> Candidate<u8> {
        Candidate {
            value,
            text: text.to_owned(),
            detail: detail.to_owned(),
        }
    }

    /// Projects for odd values, areas for even ones.
    const ICON: fn(&u8) -> Icon = |value| {
        if value % 2 == 1 {
            Icon::Project
        } else {
            Icon::Area
        }
    };

    fn typed(picker: &mut Picker<u8>, edits: impl IntoIterator<Item = TextEdit>) {
        for edit in edits {
            assert_eq!(picker.input(PickerInput::Edit(edit)), None);
        }
    }

    fn rendered(picker: &Picker<u8>, area: Rect) -> Buffer {
        let view = picker.view();
        let mut buf = Buffer::empty(area);
        PickerPopup::new("Pick", &view, &ICON, IconSet::Plain, &theme()).render(area, &mut buf);
        buf
    }

    #[test]
    fn fits_its_entries_in_at_most_80_by_20_centered() {
        let one = Picker::new(vec![candidate(1, "One", "")]);
        let view = one.view();
        let colors = theme();
        let popup = PickerPopup::new("Pick", &view, &ICON, IconSet::Plain, &colors);
        assert_eq!(
            popup.area(Rect::new(0, 0, 100, 30)),
            Rect::new(10, 13, 80, 4)
        );
        assert_eq!(popup.area(Rect::new(0, 0, 40, 3)), Rect::new(0, 0, 40, 3));
        let none = Picker::<u8>::new(vec![]);
        let view = none.view();
        let colors = theme();
        let popup = PickerPopup::new("Pick", &view, &ICON, IconSet::Plain, &colors);
        assert_eq!(
            popup.area(Rect::new(0, 0, 100, 30)).height,
            4,
            "room for No matches"
        );
        let many = Picker::new((0..40).map(|n| candidate(n, "x", "")).collect());
        let view = many.view();
        let colors = theme();
        let popup = PickerPopup::new("Pick", &view, &ICON, IconSet::Plain, &colors);
        assert_eq!(popup.area(Rect::new(0, 0, 100, 30)).height, 20);
    }

    // FR-UI-09
    #[test]
    fn draws_the_query_and_the_entries_after_their_icons_the_focused_one_selected() {
        let mut picker = Picker::new(vec![
            candidate(1, "One", ""),
            candidate(2, "Two", "Work"),
            candidate(3, "Three", ""),
        ]);
        assert_eq!(picker.input(PickerInput::Next), None);
        let buf = rendered(&picker, Rect::new(0, 0, 18, 6));
        let mut expected = Buffer::with_lines([
            "┌Pick────────────┐",
            "│›               │",
            "│ ○ One          │",
            "│ ◆ Two  Work    │",
            "│ ○ Three        │",
            "└────────────────┘",
        ]);
        expected.set_style(
            expected.area,
            Style::new().bg(theme().background).fg(theme().primary),
        );
        expected.set_style(Rect::new(1, 1, 16, 4), Style::new().fg(theme().text));
        expected.set_style(Rect::new(1, 1, 2, 1), Style::new().fg(theme().primary));
        expected.set_style(
            Rect::new(1, 3, 16, 1),
            Style::new().bg(theme().selection).fg(theme().band_text),
        );
        expected.set_style(Rect::new(7, 3, 6, 1), Style::new().fg(theme().dim_text));
        assert_eq!(buf, expected);
    }

    #[test]
    fn says_when_nothing_matches() {
        let mut picker = Picker::new(vec![candidate(1, "One", "")]);
        typed(&mut picker, [TextEdit::Insert('z')]);
        let buf = rendered(&picker, Rect::new(0, 0, 16, 4));
        let row: String = (0..16).map(|x| buf[(x, 2)].symbol()).collect();
        assert_eq!(row, "│ No matches   │");
        assert_eq!(buf[(2, 2)].fg, theme().dim_text);
    }

    // FR-UI-09
    #[test]
    fn keeps_the_focused_entry_in_view() {
        let mut picker = Picker::new((1..=30).map(|n| candidate(n, &n.to_string(), "")).collect());
        for _ in 0..24 {
            picker.input(PickerInput::Next);
        }
        let buf = rendered(&picker, Rect::new(0, 0, 80, 20));
        let row = |y| (0..80).map(|x| buf[(x, y)].symbol()).collect::<String>();
        assert!(row(2).starts_with("│ ○ 9 "), "{}", row(2));
        assert!(row(18).starts_with("│ ○ 25 "), "{}", row(18));
        assert_eq!(buf[(1, 18)].bg, theme().selection);
    }

    #[test]
    fn draws_the_selected_part_of_the_query_in_the_selection_colors() {
        let mut picker = Picker::new(vec![candidate(1, "abc", "")]);
        let select_left = TextEdit::Move {
            motion: Motion::Left,
            select: true,
        };
        typed(
            &mut picker,
            [TextEdit::Insert('a'), TextEdit::Insert('b'), select_left],
        );
        let buf = rendered(&picker, Rect::new(0, 0, 12, 4));
        assert_eq!(buf[(3, 1)].symbol(), "a");
        assert_eq!(buf[(3, 1)].bg, theme().background);
        assert_eq!(buf[(4, 1)].symbol(), "b");
        assert_eq!(buf[(4, 1)].bg, theme().selection);
        assert_eq!(buf[(4, 1)].fg, theme().band_text);
    }

    // FR-UI-09, N-PORT-04
    #[test]
    fn the_caret_follows_the_query_by_display_width_and_stays_inside_the_window() {
        let mut picker = Picker::<u8>::new(vec![]);
        typed(&mut picker, "日本x".chars().map(TextEdit::Insert));
        typed(
            &mut picker,
            [TextEdit::Move {
                motion: Motion::Left,
                select: false,
            }],
        );
        let view = picker.view();
        let colors = theme();
        let popup = PickerPopup::new("Pick", &view, &ICON, IconSet::Plain, &colors);
        assert_eq!(popup.caret(Rect::new(0, 0, 100, 30)), Position::new(17, 14));
        let mut picker = Picker::<u8>::new(vec![]);
        typed(&mut picker, std::iter::repeat_n(TextEdit::Insert('x'), 200));
        let view = picker.view();
        let colors = theme();
        let popup = PickerPopup::new("Pick", &view, &ICON, IconSet::Plain, &colors);
        assert_eq!(popup.caret(Rect::new(0, 0, 100, 30)), Position::new(88, 14));
    }

    // FR-VIEW-07
    #[test]
    fn a_tree_is_indented_while_the_query_is_empty_with_colors_and_badges() {
        let mut picker = Picker::new(vec![
            candidate(2, "Work", ""),
            candidate(1, "Site", "Work"),
            candidate(3, "Blog", "Work › Site"),
        ]);
        let depth = |value: &u8| usize::from(*value) / 2 + usize::from(*value == 3);
        let badge = |value: &u8| match value {
            1 => String::new(),
            value => value.to_string(),
        };
        let draw = |picker: &Picker<u8>| {
            let view = picker.view();
            let area = Rect::new(0, 0, 26, 6);
            let mut buf = Buffer::empty(area);
            PickerPopup::new("Pick", &view, &ICON, IconSet::Plain, &theme())
                .tree(&depth)
                .badge(&badge)
                .color(|_, value| {
                    if *value == 2 {
                        Color::Red
                    } else {
                        Color::Green
                    }
                })
                .render(area, &mut buf);
            buf
        };
        // The entries' rows, below the query.
        let rows = |buf: &Buffer| -> Vec<String> {
            (0..6)
                .map(|y| (0..26).map(|x| buf[(x, y)].symbol()).collect::<String>())
                .skip_while(|row| !row.starts_with("│›"))
                .skip(1)
                .take_while(|row| !row.starts_with('└'))
                .collect()
        };
        let buf = draw(&picker);
        assert_eq!(
            rows(&buf),
            [
                "│   ◆ Work             2 │",
                "│ ○ Site                 │",
                "│     ○ Blog           3 │",
            ]
        );
        assert_eq!(
            (buf[(4, 2)].fg, buf[(6, 2)].fg, buf[(4, 3)].fg),
            (Color::Red, Color::Red, Color::Green),
            "each entry's icon and text in its color"
        );
        assert_eq!(buf[(22, 2)].fg, theme().dim_text);
        assert_eq!(buf[(22, 2)].bg, theme().selection, "on the focused row");
        typed(&mut picker, [TextEdit::Insert('o')]);
        assert_eq!(
            rows(&draw(&picker)),
            ["│ ◆ Work               2 │", "│ ○ Blog  Work › Site  3 │",],
            "matches show their details and no tree"
        );
    }
}
