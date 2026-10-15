//! Quick Comment in the terminal (FR-ATT-06): the keys that drive its panel and the panel, one
//! line across the screen just above the status line where Quick Add opens, titled with the item
//! the comment goes to.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use jiff::civil::DateTime;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Style,
    widgets::{Block, Clear, Widget},
};

use super::{
    Theme, field,
    keymap::{self, Key},
    text_keys,
};
use crate::app::{QuickCommentInput, QuickCommentView};

/// What a key of the open panel besides the text keys does; adding takes the time it is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommentKey {
    /// Adds the comment.
    Add,
    /// Closes the panel.
    Cancel,
}

/// The keys of the open panel besides the text keys.
pub(super) const KEYS: &[(Key, CommentKey)] = &[
    ((KeyModifiers::NONE, KeyCode::Enter), CommentKey::Add),
    ((KeyModifiers::NONE, KeyCode::Esc), CommentKey::Cancel),
];

/// What `event` does in the open panel: `Enter` adds the comment, stamped with what `now` says,
/// `Esc` cancels, and the text keys edit the text. Only key presses count.
pub(super) fn quick_comment_input(
    event: &Event,
    now: impl FnOnce() -> DateTime,
) -> Option<QuickCommentInput> {
    let key = keymap::key_press(event)?;
    match KEYS.iter().find(|(bound, _)| *bound == key) {
        Some((_, CommentKey::Add)) => Some(QuickCommentInput::Add(now())),
        Some((_, CommentKey::Cancel)) => Some(QuickCommentInput::Cancel),
        None => text_keys::text_edit(key).map(QuickCommentInput::Edit),
    }
}

/// The open panel: titled with the item the comment goes to, the text typed after a space with
/// its selection in the selection colors.
pub(super) struct QuickCommentPanel<'a> {
    view: &'a QuickCommentView<'a>,
    theme: &'a Theme,
}

impl<'a> QuickCommentPanel<'a> {
    /// The panel's height, its border included.
    pub(super) const HEIGHT: u16 = 3;

    /// The panel showing `view` in `theme`.
    pub(super) fn new(view: &'a QuickCommentView<'a>, theme: &'a Theme) -> Self {
        Self { view, theme }
    }

    /// Where the terminal's cursor goes when the panel is drawn over `area`, as for a prompt.
    pub(super) fn caret(&self, area: Rect) -> Option<Position> {
        field::prompt_caret(self.view.field, area)
    }
}

impl Widget for QuickCommentPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let theme = self.theme;
        let accent = Style::new().fg(theme.accent);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(format!(" Comment on {} ", self.view.title));
        let inner = frame.inner(area);
        frame.render(area, buf);
        if let Some(row) = inner.rows().next() {
            let field = self.view.field;
            field::draw_prompt(field, field::prompt_line(field, theme), row, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyEventKind};
    use jiff::civil::date;
    use ratatui::buffer::Cell;

    use super::*;
    use crate::{
        app::{TextEdit, TextField},
        ui::BuiltIn,
    };

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    // FR-ATT-06
    #[test]
    fn enter_adds_at_the_time_it_is_pressed_escape_cancels_and_text_keys_edit() {
        let now = date(2026, 10, 15).at(21, 5, 0, 0);
        assert_eq!(
            quick_comment_input(&press(KeyCode::Enter), || now),
            Some(QuickCommentInput::Add(now))
        );
        assert_eq!(
            quick_comment_input(&press(KeyCode::Esc), || now),
            Some(QuickCommentInput::Cancel)
        );
        assert_eq!(
            quick_comment_input(&press(KeyCode::Char('c')), || now),
            Some(QuickCommentInput::Edit(TextEdit::Insert('c')))
        );
        let mut release = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(quick_comment_input(&Event::Key(release), || now), None);
    }

    // FR-ATT-06
    #[test]
    fn the_panel_is_one_line_titled_with_the_item_and_its_caret_follows_the_text() {
        let theme = BuiltIn::default().colors();
        let field = TextField::new("Called back".to_owned()).edited(TextEdit::SelectAll);
        let view = QuickCommentView {
            field: &field,
            title: "Call the bank",
        };
        let area = Rect::new(0, 0, 40, QuickCommentPanel::HEIGHT);
        let mut buf = Buffer::empty(area);
        QuickCommentPanel::new(&view, &theme).render(area, &mut buf);
        let lines: Vec<String> = buf
            .content()
            .chunks(40)
            .map(|line| line.iter().map(Cell::symbol).collect())
            .collect();
        assert_eq!(
            lines,
            [
                "┌ Comment on Call the bank ────────────┐",
                "│ Called back                          │",
                "└──────────────────────────────────────┘",
            ]
        );
        assert_eq!(buf[(2, 0)].fg, theme.accent, "the title");
        assert_eq!(buf[(2, 1)].bg, theme.selection, "the text is selected");
        let typed = TextField::new("ab".to_owned());
        let view = QuickCommentView {
            field: &typed,
            title: "Call",
        };
        assert_eq!(
            QuickCommentPanel::new(&view, &theme).caret(Rect::new(
                0,
                10,
                20,
                QuickCommentPanel::HEIGHT
            )),
            Some(Position::new(4, 11))
        );
    }
}
