//! The Save As prompt in the terminal (FR-STORE-11): the keys that drive it and its panel, across
//! the screen just above the status line where the date prompt goes: its border and title in the
//! accent color, the path typed and, below it, what `Enter` does or why writing failed.

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
use crate::app::{SaveAsInput, SaveAsStatus, SaveAsView};

/// The keys of the open prompt besides the text keys.
pub(super) const KEYS: &[(Key, SaveAsInput)] = &[
    ((KeyModifiers::NONE, KeyCode::Enter), SaveAsInput::Apply),
    ((KeyModifiers::NONE, KeyCode::Esc), SaveAsInput::Cancel),
];

/// What `event` does in the open prompt: `Enter` saves, `Esc` cancels, and the text keys edit
/// the path. Only key presses count.
pub(super) fn save_as_input(event: &Event) -> Option<SaveAsInput> {
    let key = keymap::key_press(event)?;
    KEYS.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
        .or_else(|| text_keys::text_edit(key).map(SaveAsInput::Edit))
}

/// The open prompt's panel: the path typed after a space with its selection in the selection
/// colors, and below it what `Enter` does, or why writing failed.
pub(super) struct SaveAsPanel<'a> {
    view: &'a SaveAsView<'a>,
    theme: &'a Theme,
}

impl<'a> SaveAsPanel<'a> {
    /// The panel's height, its border included.
    pub(super) const HEIGHT: u16 = 4;

    /// The panel showing `view` in `theme`.
    pub(super) fn new(view: &'a SaveAsView<'a>, theme: &'a Theme) -> Self {
        Self { view, theme }
    }

    /// Where the terminal's cursor goes when the panel is drawn over `area`, as for any prompt.
    pub(super) fn caret(&self, area: Rect) -> Option<Position> {
        field::prompt_caret(self.view.field, area)
    }

    /// What the prompt is doing, in words.
    fn status(&self) -> Line<'static> {
        let dim = Style::new().fg(self.theme.dim_text);
        let error = Style::new().fg(self.theme.error);
        match self.view.status {
            SaveAsStatus::Typing => Line::styled(" Enter saves here, Esc cancels", dim),
            SaveAsStatus::Failed(reason) => Line::styled(format!(" Cannot save: {reason}"), error),
            SaveAsStatus::Replacing => {
                Line::styled(" The file exists: Enter replaces it, Esc goes back", error)
            }
            SaveAsStatus::Writing => Line::styled(" Saving…", dim),
        }
    }
}

impl Widget for SaveAsPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let theme = self.theme;
        let accent = Style::new().fg(theme.accent);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(" Save As ");
        let inner = frame.inner(area);
        frame.render(area, buf);
        let mut rows = inner.rows();
        if let Some(row) = rows.next() {
            field::prompt_line(self.view.field, theme).render(row, buf);
        }
        if let Some(row) = rows.next() {
            self.status().render(row, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyEventKind};
    use ratatui::buffer::Cell;

    use super::*;
    use crate::{
        app::{TextEdit, TextField},
        ui::BuiltIn,
    };

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    // FR-STORE-11
    #[test]
    fn enter_saves_escape_cancels_and_text_keys_edit() {
        assert_eq!(
            save_as_input(&press(KeyCode::Enter)),
            Some(SaveAsInput::Apply)
        );
        assert_eq!(
            save_as_input(&press(KeyCode::Esc)),
            Some(SaveAsInput::Cancel)
        );
        assert_eq!(
            save_as_input(&press(KeyCode::Char('~'))),
            Some(SaveAsInput::Edit(TextEdit::Insert('~')))
        );
        let mut release = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(save_as_input(&Event::Key(release)), None);
    }

    /// The panel 40 columns wide with `field` doing `status`, drawn.
    fn drawn(field: &TextField, status: SaveAsStatus<'_>) -> Buffer {
        let theme = BuiltIn::default().colors();
        let view = SaveAsView { field, status };
        let area = Rect::new(0, 0, 40, SaveAsPanel::HEIGHT);
        let mut buf = Buffer::empty(area);
        SaveAsPanel::new(&view, &theme).render(area, &mut buf);
        buf
    }

    fn lines(buf: &Buffer) -> Vec<String> {
        buf.content()
            .chunks(usize::from(buf.area.width))
            .map(|line| line.iter().map(Cell::symbol).collect())
            .collect()
    }

    // FR-STORE-11
    #[test]
    fn the_panel_shows_the_path_and_what_enter_does() {
        let theme = BuiltIn::default().colors();
        let field = TextField::new("~/todo.taskpaper".to_owned()).edited(TextEdit::SelectAll);
        let buf = drawn(&field, SaveAsStatus::Typing);
        assert_eq!(
            lines(&buf),
            [
                "┌ Save As ─────────────────────────────┐",
                "│ ~/todo.taskpaper                     │",
                "│ Enter saves here, Esc cancels        │",
                "└──────────────────────────────────────┘",
            ]
        );
        assert_eq!(buf[(2, 0)].fg, theme.accent, "the title");
        assert_eq!(buf[(2, 1)].bg, theme.selection, "the path is selected");
        assert_eq!(buf[(2, 2)].fg, theme.dim_text);
        for (status, said, color) in [
            (
                SaveAsStatus::Failed("the folder /x does not exist"),
                "│ Cannot save: the folder /x does not e│",
                theme.error,
            ),
            (
                SaveAsStatus::Replacing,
                "│ The file exists: Enter replaces it, E│",
                theme.error,
            ),
            (
                SaveAsStatus::Writing,
                "│ Saving…                              │",
                theme.dim_text,
            ),
        ] {
            let buf = drawn(&field, status);
            assert_eq!(lines(&buf)[2], said);
            assert_eq!(buf[(2, 2)].fg, color, "{said}");
        }
    }

    // FR-STORE-11
    #[test]
    fn the_caret_follows_the_path() {
        let theme = BuiltIn::default().colors();
        let field = TextField::new("a.taskpaper".to_owned());
        let view = SaveAsView {
            field: &field,
            status: SaveAsStatus::Typing,
        };
        let area = Rect::new(0, 10, 40, SaveAsPanel::HEIGHT);
        assert_eq!(
            SaveAsPanel::new(&view, &theme).caret(area),
            Some(Position::new(13, 11))
        );
    }
}
