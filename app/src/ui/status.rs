//! The status line at the bottom of the screen: the mode, what is open and whether changes to it
//! are kept, and the keys of an unfinished sequence at its right end
//! ([ui.md §1](../../../specs/reference/taskpaper-terminal/specs/ui.md#1-layout)).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{Icon, IconSet, Theme};
use crate::app::Opened;

/// What stands between the parts of the line.
const SEPARATOR: &str = " · ";

/// The status line for a document opened from `opened`, in edit mode or not, with the keys
/// `pending` waiting.
pub(super) struct StatusLine<'a> {
    opened: &'a Opened,
    editing: bool,
    pending: &'a str,
    theme: &'a Theme,
    icons: IconSet,
}

impl<'a> StatusLine<'a> {
    /// The line for `opened`, in edit mode when `editing`, with `pending` keys, drawn in `theme`
    /// with `icons`.
    pub(super) fn new(
        opened: &'a Opened,
        editing: bool,
        pending: &'a str,
        theme: &'a Theme,
        icons: IconSet,
    ) -> Self {
        Self {
            opened,
            editing,
            pending,
            theme,
            icons,
        }
    }
}

impl Widget for StatusLine<'_> {
    /// Draws the mode, then the file's path and that it is read-only until saving arrives, or
    /// that the demo data is not saved (FR-STORE-01), the path giving up its start first, and then
    /// itself, when the line is too narrow, and the pending keys at the right end.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode = if self.editing {
            format!(" {} EDIT", Icon::Edit.glyph(self.icons))
        } else {
            format!(" {} NAVIGATE", Icon::Navigate.glyph(self.icons))
        };
        let (name, kept) = match self.opened {
            Opened::Demo => ("Demo data".to_owned(), "not saved"),
            Opened::File(path) => (path.display().to_string(), "read-only"),
        };
        let pending = if self.pending.is_empty() {
            String::new()
        } else {
            format!(" {} ", self.pending)
        };
        let fixed = [mode.as_str(), SEPARATOR, SEPARATOR, kept, &pending]
            .map(UnicodeWidthStr::width)
            .iter()
            .sum::<usize>();
        let room = usize::from(area.width).saturating_sub(fixed);
        let dim = Style::new().fg(self.theme.dim_text);
        let mut spans = vec![Span::styled(
            mode,
            Style::new().fg(self.theme.accent).bold(),
        )];
        let name = tail(&name, room);
        if !name.is_empty() {
            spans.extend([Span::styled(SEPARATOR, dim), Span::styled(name, dim)]);
        }
        spans.extend([Span::styled(SEPARATOR, dim), Span::styled(kept, dim)]);
        Line::from(spans).render(area, buf);
        let width = u16::try_from(pending.width()).unwrap_or(u16::MAX);
        let at = Rect::new(area.right().saturating_sub(width), area.y, width, 1);
        Span::styled(pending, Style::new().fg(self.theme.accent))
            .render(at.intersection(area), buf);
    }
}

/// `text` if it fits in `room` columns, else as much of its end as fits behind `…`.
fn tail(text: &str, room: usize) -> String {
    if text.width() <= room {
        return text.to_owned();
    }
    if room == 0 {
        return String::new();
    }
    let mut width = 1;
    let mut kept = Vec::new();
    for grapheme in text.graphemes(true).rev() {
        width += grapheme.width();
        if width > room {
            break;
        }
        kept.push(grapheme);
    }
    std::iter::once("…").chain(kept.into_iter().rev()).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::ui::BuiltIn;

    #[track_caller]
    fn check(opened: &Opened, pending: &str, width: u16, expected: &str) {
        check_mode(opened, false, pending, width, expected);
    }

    #[track_caller]
    fn check_mode(opened: &Opened, editing: bool, pending: &str, width: u16, expected: &str) {
        let theme = BuiltIn::default().colors();
        let mut buf = Buffer::empty(Rect::new(0, 0, width, 1));
        StatusLine::new(opened, editing, pending, &theme, IconSet::Plain)
            .render(buf.area, &mut buf);
        let shown: String = buf
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert_eq!(shown, expected);
    }

    // FR-STORE-01, D-29
    #[test]
    fn the_demo_data_is_not_saved_which_the_line_says() {
        check(
            &Opened::Demo,
            "",
            40,
            " ↕ NAVIGATE · Demo data · not saved     ",
        );
    }

    // FR-EDIT-01
    #[test]
    fn edit_mode_is_named_first() {
        check_mode(
            &Opened::Demo,
            true,
            "",
            36,
            " ✎ EDIT · Demo data · not saved     ",
        );
    }

    // FR-UI-02
    #[test]
    fn a_file_shows_its_path_and_that_it_is_read_only() {
        check(
            &Opened::File(PathBuf::from("todo.taskpaper")),
            "",
            48,
            " ↕ NAVIGATE · todo.taskpaper · read-only        ",
        );
    }

    // FR-UI-02, FR-UI-06
    #[test]
    fn a_narrow_line_cuts_the_path_from_its_start() {
        let long = Opened::File(PathBuf::from("notes/work/todo.taskpaper"));
        check(&long, "", 40, " ↕ NAVIGATE · …odo.taskpaper · read-only");
        check(&long, "", 30, " ↕ NAVIGATE · …per · read-only");
        check(&long, "", 29, " ↕ NAVIGATE · …er · read-only");
        check(&long, "", 27, " ↕ NAVIGATE · … · read-only");
        check(&long, "", 26, " ↕ NAVIGATE · read-only   ");
        check(&long, "", 10, " ↕ NAVIGAT");
    }

    // FR-UI-02
    #[test]
    fn pending_keys_stand_at_the_right_end() {
        check(
            &Opened::Demo,
            "g",
            40,
            " ↕ NAVIGATE · Demo data · not saved   g ",
        );
        let long = Opened::File(PathBuf::from("notes/work/todo.taskpaper"));
        check(&long, "g", 35, " ↕ NAVIGATE · …paper · read-only g ");
    }

    #[test]
    fn the_tail_keeps_whole_graphemes_and_wide_characters() {
        assert_eq!(tail("abc", 3), "abc");
        assert_eq!(tail("abcd", 3), "…cd");
        assert_eq!(tail("日本語", 4), "…語");
        assert_eq!(tail("日本語", 3), "…語");
        assert_eq!(tail("日本語", 2), "…");
        assert_eq!(tail("e\u{301}x", 2), "e\u{301}x");
        assert_eq!(tail("abcd", 1), "…");
        assert_eq!(tail("abcd", 0), "");
    }
}
