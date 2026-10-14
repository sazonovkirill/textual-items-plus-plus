//! A text field as it is drawn while it is edited: its text with the selected part in the
//! selection colors, on one line or wrapped on several, and where its caret stands.

use std::ops::Range;

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

use super::Theme;
use crate::app::{TextField, grapheme_columns};

/// The bytes `line` of `field`'s text, its selected part in the selection colors of `theme`.
pub(super) fn selected<'a>(field: &'a TextField, line: Range<usize>, theme: &Theme) -> Line<'a> {
    let text = field.text();
    let selection = field.selection().unwrap_or(field.caret()..field.caret());
    let start = selection.start.clamp(line.start, line.end);
    let end = selection.end.clamp(line.start, line.end);
    let style = Style::new().bg(theme.selection).fg(theme.band_text);
    Line::from(vec![
        Span::raw(text.get(line.start..start).unwrap_or_default()),
        Span::styled(text.get(start..end).unwrap_or_default(), style),
        Span::raw(text.get(end..line.end).unwrap_or_default()),
    ])
}

/// A prompt's `field`, its selection in the selection colors of `theme`, as [`draw_prompt`] draws
/// it.
pub(super) fn prompt_line<'a>(field: &'a TextField, theme: &Theme) -> Line<'a> {
    selected(field, 0..field.text().len(), theme)
}

/// Draws `line`, a prompt's `field` as typed, in `row`, the first line inside the prompt's
/// border, after a space and scrolled sideways to keep the caret in view.
pub(super) fn draw_prompt(field: &TextField, line: Line<'_>, row: Rect, buf: &mut Buffer) {
    let text = prompt_text(row);
    let shift = u16::try_from(prompt_shift(field, text)).unwrap_or(u16::MAX);
    Paragraph::new(line).scroll((0, shift)).render(text, buf);
}

/// Where the terminal's cursor goes when a prompt with `field` drawn as [`draw_prompt`] has the
/// bordered `area`: at the caret; nowhere when the prompt has no room for it inside its border.
pub(super) fn prompt_caret(field: &TextField, area: Rect) -> Option<Position> {
    let text = prompt_text(Block::bordered().inner(area).rows().next()?);
    let column = caret_width(field) - prompt_shift(field, text);
    let column = u16::try_from(column).unwrap_or(u16::MAX);
    (!text.is_empty()).then(|| Position::new(text.x.saturating_add(column), text.y))
}

/// The part of a prompt's first `row` that its text takes, after a space.
fn prompt_text(row: Rect) -> Rect {
    let x = row.x.saturating_add(1).min(row.right());
    Rect::new(x, row.y, row.right() - x, row.height)
}

/// The display columns of `field`'s text before its caret.
fn caret_width(field: &TextField) -> usize {
    Span::raw(field.text().get(..field.caret()).unwrap_or_default()).width()
}

/// The columns a prompt's `field` is scrolled sideways in its `text` area, as [`shift`] scrolls.
fn prompt_shift(field: &TextField, text: Rect) -> usize {
    shift(field.text(), caret_width(field), usize::from(text.width))
}

/// The lines of `field`'s text wrapped as the field wraps them in `columns`, each drawn as
/// [`selected`] draws it, and the caret's line among them and its column. A caret after spaces
/// past the line's end stands in its last column, so that the lines never scroll sideways.
pub(super) fn wrapped<'a>(
    field: &'a TextField,
    columns: usize,
    theme: &Theme,
) -> (Vec<Line<'a>>, (usize, usize)) {
    let text = field.text();
    let lines = field.screen_lines(columns);
    let line = lines
        .iter()
        .rposition(|line| line.start <= field.caret())
        .unwrap_or_default();
    let column = lines
        .get(line)
        .and_then(|line| text.get(line.start..field.caret()))
        .map_or(0, |before| Span::raw(before).width())
        .min(columns);
    let lines = lines
        .into_iter()
        .map(|line| selected(field, line, theme))
        .collect();
    (lines, (line, column))
}

/// The display columns `text` is scrolled sideways to keep its caret, `caret` columns into it,
/// in view in `columns`: none while it fits, else the fewest whole graphemes that keep it in the
/// last column, and never past the caret.
pub(super) fn shift(text: &str, caret: usize, columns: usize) -> usize {
    let needed = (caret + 1).saturating_sub(columns);
    grapheme_columns(text)
        .map(|(_, before)| before)
        .find(|&before| before >= needed)
        .map_or(caret, |before| before.min(caret))
}
