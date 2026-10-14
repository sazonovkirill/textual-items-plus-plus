//! Cutting a line short to fit its columns, ending it in `…`, as text does where it does not wrap
//! (FR-UI-05, [ui.md §2](../../../specs/reference/taskpaper-terminal/specs/ui.md#2-blocks)).
//! Widths are display widths, and graphemes are never split.

use ratatui::{
    style::Style,
    text::{Line, Span},
};
use unicode_segmentation::UnicodeSegmentation;

/// `line` cut to `columns` columns, its last column a `…` in the style of the first grapheme cut,
/// when it is wider; as it is otherwise.
pub(super) fn truncate(line: &Line, columns: usize) -> Line<'static> {
    let mut room = if line.width() <= columns {
        usize::MAX
    } else {
        columns.saturating_sub(1)
    };
    let mut spans = Vec::new();
    for span in &line.spans {
        let kept = fitting(&span.content, &mut room);
        push(&mut spans, kept, span.style);
        if kept.len() < span.content.len() {
            if columns > 0 {
                push(&mut spans, "…", span.style);
            }
            break;
        }
    }
    Line::from(spans).style(line.style)
}

/// The longest start of `text` made of whole graphemes that fits in `room` columns, which it
/// takes from `room`.
fn fitting<'a>(text: &'a str, room: &mut usize) -> &'a str {
    let mut end = 0;
    for grapheme in text.graphemes(true) {
        let width = Span::raw(grapheme).width();
        if width > *room {
            break;
        }
        *room -= width;
        end += grapheme.len();
    }
    text.get(..end).unwrap_or_default()
}

/// Adds `text` in `style` to the end of `spans`, as part of the last span when that has the same
/// style.
fn push(spans: &mut Vec<Span<'static>>, text: &str, style: Style) {
    if text.is_empty() {
        return;
    }
    match spans.last_mut() {
        Some(span) if span.style == style => span.content.to_mut().push_str(text),
        _ => spans.push(Span::styled(text.to_owned(), style)),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Stylize;

    use super::*;

    // FR-UI-05
    #[test]
    fn a_line_too_wide_ends_in_an_ellipsis_in_the_style_of_what_it_cuts() {
        let line = Line::from(vec![Span::raw("- "), "abc".bold(), "def".italic()]);
        assert_eq!(
            truncate(&line, 8),
            Line::from(vec![Span::raw("- "), "abc".bold(), "def".italic()])
        );
        assert_eq!(
            truncate(&line, 7),
            Line::from(vec![Span::raw("- "), "abc".bold(), "d…".italic()])
        );
        assert_eq!(
            truncate(&line, 5),
            Line::from(vec![Span::raw("- "), "ab…".bold()])
        );
        assert_eq!(truncate(&line, 1), Line::from("…"));
        assert_eq!(truncate(&line, 0), Line::default());
        assert_eq!(
            truncate(&Line::from("ab").dim(), 1),
            Line::from("…").dim(),
            "the line keeps its style"
        );
    }

    // FR-UI-05
    #[test]
    fn an_ellipsis_never_splits_a_wide_character_or_a_grapheme() {
        assert_eq!(truncate(&Line::from("日本語"), 4), Line::from("日…"));
        assert_eq!(truncate(&Line::from("日本語"), 6), Line::from("日本語"));
        assert_eq!(
            truncate(&Line::from("e\u{301}e\u{301}e\u{301}"), 2),
            Line::from("e\u{301}…")
        );
    }
}
