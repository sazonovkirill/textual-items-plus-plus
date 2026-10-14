//! The date prompt in the terminal (FR-DATE-02): the keys that drive it and its panel, across the
//! screen just above the status line, as taskpaper-terminal draws it
//! ([ui.md §1](../../../specs/reference/taskpaper-terminal/specs/ui.md#1-layout)): its border and
//! title, the date it sets, in the accent color, the text typed and, below it, what `Enter` does.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Clear, Widget},
};

use super::{
    Theme, field,
    keymap::{self, Key},
    text_keys,
};
use crate::{
    app::{DateInput, DatePromptView},
    domain::dates::{DateError, DateKind},
};

/// The keys of the open prompt besides the text keys.
pub(super) const PROMPT: &[(Key, DateInput)] = &[
    ((KeyModifiers::NONE, KeyCode::Enter), DateInput::Apply),
    ((KeyModifiers::NONE, KeyCode::Esc), DateInput::Cancel),
];

/// What `event` does in the open prompt: `Enter` applies, `Esc` cancels, and the text keys edit
/// the text. Only key presses count.
pub(super) fn date_prompt_input(event: &Event) -> Option<DateInput> {
    let key = keymap::key_press(event)?;
    PROMPT
        .iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
        .or_else(|| text_keys::text_edit(key).map(DateInput::Edit))
}

/// The open prompt's panel: titled with the date it sets, the text typed after a space with its
/// selection in the selection colors, and below it the date `Enter` sets, that it removes the
/// date, or why the text is not a date.
pub(super) struct DatePromptPanel<'a> {
    view: &'a DatePromptView<'a>,
    theme: &'a Theme,
}

impl<'a> DatePromptPanel<'a> {
    /// The panel's height, its border included.
    pub(super) const HEIGHT: u16 = 4;

    /// The panel showing `view` in `theme`.
    pub(super) fn new(view: &'a DatePromptView<'a>, theme: &'a Theme) -> Self {
        Self { view, theme }
    }

    /// Where the terminal's cursor goes when the panel is drawn over `area`: at the caret, or in
    /// the last column inside the border when the text is wider; nowhere when the panel has no
    /// room inside its border.
    pub(super) fn caret(&self, area: Rect) -> Option<Position> {
        field::prompt_caret(self.view.field, area)
    }

    /// What `Enter` does with the text: set the date, remove it, or nothing, quoting where the
    /// text stops being a date.
    fn preview(&self) -> Line<'static> {
        match self.view.date {
            Ok(Some(date)) => Line::from(format!(" {}", date.strftime("%a %-d %b %Y"))),
            Ok(None) => Line::styled(
                " No date: Enter removes it",
                Style::new().fg(self.theme.dim_text),
            ),
            Err(error) => Line::styled(
                format!(" {}", not_a_date(self.view.field.text(), error)),
                Style::new().fg(self.theme.error),
            ),
        }
    }
}

/// Why `text` is not a date, quoting where it stops being one.
fn not_a_date(text: &str, error: DateError) -> String {
    let from = |offset: usize| text.get(offset..).unwrap_or_default();
    match error {
        DateError::Unexpected { offset } if !from(offset).is_empty() => {
            format!("Not a date from “{}”", from(offset))
        }
        DateError::Empty | DateError::Unexpected { .. } => {
            "Not a date: something is missing".to_owned()
        }
        DateError::NoSuchDay { offset } => format!("No such day: “{}”", from(offset)),
        DateError::OutOfRange { .. } => "Out of range".to_owned(),
    }
}

impl Widget for DatePromptPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let theme = self.theme;
        let title = match self.view.kind {
            DateKind::Start => " Start date ",
            DateKind::Due => " Due date ",
        };
        let accent = Style::new().fg(theme.accent);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(title);
        let inner = frame.inner(area);
        frame.render(area, buf);
        let mut rows = inner.rows();
        if let Some(row) = rows.next() {
            field::prompt_line(self.view.field, theme).render(row, buf);
        }
        if let Some(row) = rows.next() {
            self.preview().render(row, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyEventKind};
    use jiff::civil::{Date, date};
    use ratatui::buffer::Cell;

    use super::*;
    use crate::{
        app::{TextEdit, TextField},
        ui::BuiltIn,
    };

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    // FR-DATE-02
    #[test]
    fn enter_applies_escape_cancels_and_text_keys_edit() {
        let none = KeyModifiers::NONE;
        let input = |code| date_prompt_input(&press(code, none));
        assert_eq!(input(KeyCode::Enter), Some(DateInput::Apply));
        assert_eq!(input(KeyCode::Esc), Some(DateInput::Cancel));
        assert_eq!(
            input(KeyCode::Char('f')),
            Some(DateInput::Edit(TextEdit::Insert('f')))
        );
        assert_eq!(
            input(KeyCode::Backspace),
            Some(DateInput::Edit(TextEdit::DeleteBackward))
        );
        let mut release = KeyEvent::new(KeyCode::Enter, none);
        release.kind = KeyEventKind::Release;
        assert_eq!(date_prompt_input(&Event::Key(release)), None);
    }

    /// The panel 30 columns wide for `kind` with `field` and `date`, drawn as text lines.
    fn drawn(kind: DateKind, field: &TextField, date: Result<Option<Date>, DateError>) -> Buffer {
        let theme = BuiltIn::default().colors();
        let view = DatePromptView { kind, field, date };
        let area = Rect::new(0, 0, 30, DatePromptPanel::HEIGHT);
        let mut buf = Buffer::empty(area);
        DatePromptPanel::new(&view, &theme).render(area, &mut buf);
        buf
    }

    fn lines(buf: &Buffer) -> Vec<String> {
        buf.content()
            .chunks(usize::from(buf.area.width))
            .map(|line| line.iter().map(Cell::symbol).collect())
            .collect()
    }

    // FR-DATE-02
    #[test]
    fn the_panel_shows_the_text_and_the_date_it_resolves_to() {
        let field = TextField::new("fri".to_owned());
        let buf = drawn(DateKind::Due, &field, Ok(Some(date(2026, 10, 16))));
        assert_eq!(
            lines(&buf),
            [
                "┌ Due date ──────────────────┐",
                "│ fri                        │",
                "│ Fri 16 Oct 2026            │",
                "└────────────────────────────┘",
            ]
        );
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(0, 0)].fg, theme.accent);
        assert_eq!(buf[(2, 0)].fg, theme.accent, "the title");
    }

    // FR-DATE-02
    #[test]
    fn the_panel_shows_the_selection_and_says_a_blank_text_removes_the_date() {
        let field = TextField::new("fri".to_owned()).edited(TextEdit::SelectAll);
        let buf = drawn(DateKind::Start, &field, Ok(None));
        let shown = lines(&buf);
        assert_eq!(shown[0], "┌ Start date ────────────────┐");
        assert_eq!(shown[2], "│ No date: Enter removes it  │");
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(2, 1)].bg, theme.selection);
        assert_eq!(buf[(2, 2)].fg, theme.dim_text);
    }

    // FR-DATE-02
    #[test]
    fn the_panel_says_why_a_text_is_not_a_date() {
        let theme = BuiltIn::default().colors();
        for (text, error, said) in [
            (
                "fri!",
                DateError::Unexpected { offset: 3 },
                "│ Not a date from “!”        │",
            ),
            (
                "+3",
                DateError::Unexpected { offset: 2 },
                "│ Not a date: something is mi│",
            ),
            (
                "02-30",
                DateError::NoSuchDay { offset: 0 },
                "│ No such day: “02-30”       │",
            ),
            (
                "+20000y",
                DateError::OutOfRange { offset: 0 },
                "│ Out of range               │",
            ),
            ("", DateError::Empty, "│ Not a date: something is mi│"),
        ] {
            let field = TextField::new(text.to_owned());
            let buf = drawn(DateKind::Due, &field, Err(error));
            assert_eq!(lines(&buf)[2], said, "{text}");
            assert_eq!(buf[(2, 2)].fg, theme.error);
        }
    }

    // FR-DATE-02
    #[test]
    fn the_caret_follows_the_text_and_stops_at_the_border() {
        let theme = BuiltIn::default().colors();
        let area = Rect::new(0, 10, 12, DatePromptPanel::HEIGHT);
        let caret = |text: &str| {
            let field = TextField::new(text.to_owned());
            let view = DatePromptView {
                kind: DateKind::Start,
                field: &field,
                date: Ok(None),
            };
            DatePromptPanel::new(&view, &theme).caret(area)
        };
        assert_eq!(caret(""), Some(Position::new(2, 11)));
        assert_eq!(caret("fri"), Some(Position::new(5, 11)));
        assert_eq!(caret("next friday +1w"), Some(Position::new(10, 11)));
        let field = TextField::new(String::new());
        let view = DatePromptView {
            kind: DateKind::Start,
            field: &field,
            date: Ok(None),
        };
        assert_eq!(
            DatePromptPanel::new(&view, &theme).caret(Rect::new(0, 0, 1, 1)),
            None,
            "a prompt with no room inside its border has no caret (N-PORT-02)"
        );
    }
}
