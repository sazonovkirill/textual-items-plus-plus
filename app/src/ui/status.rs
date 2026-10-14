//! The status line at the bottom of the screen: the mode, what is open and whether changes to it
//! are saved, or what just happened to its file, and the keys of an unfinished sequence at its
//! right end
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

use super::{Icon, IconSet, Theme, keymap};
use crate::app::{Command, Notice, Opened, Refusal, State, autosave::SaveState};

/// What stands between the parts of the line.
const SEPARATOR: &str = " · ";

/// The status line for `state`, with the keys `pending` waiting.
pub(super) struct StatusLine<'a> {
    state: &'a State,
    pending: &'a str,
    theme: &'a Theme,
    icons: IconSet,
}

impl<'a> StatusLine<'a> {
    /// The line for `state` with `pending` keys, drawn in `theme` with `icons`.
    pub(super) fn new(
        state: &'a State,
        pending: &'a str,
        theme: &'a Theme,
        icons: IconSet,
    ) -> Self {
        Self {
            state,
            pending,
            theme,
            icons,
        }
    }
}

impl Widget for StatusLine<'_> {
    /// Draws the mode, then the file's path and whether its changes are saved, or that the demo
    /// data is not saved (FR-STORE-01), the path giving up its start first, and then itself,
    /// when the line is too narrow, and the pending keys at the right end.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode = if self.state.editing().is_some() {
            format!(" {} EDIT", Icon::Edit.glyph(self.icons))
        } else {
            format!(" {} NAVIGATE", Icon::Navigate.glyph(self.icons))
        };
        let name = match self.state.opened() {
            Opened::Demo => "Demo data".to_owned(),
            Opened::File(path) => path.display().to_string(),
        };
        let kept = kept(self.state);
        let kept = kept.as_str();
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

/// What the line says of the document's file: what just happened to it, else whether it holds
/// every change. That quitting again ends without saving comes first, so it is never cut off.
fn kept(state: &State) -> String {
    let quit_again = if state.confirming_quit() {
        "quit again to quit without saving; "
    } else {
        ""
    };
    let notice = state.notice().map(|notice| match notice {
        Notice::Reloaded => "reloaded (changed on disk)".to_owned(),
        Notice::KeptMine(copy) => format!("kept mine; theirs is in {}", copy.display()),
        Notice::TookTheirs(copy) => format!("took theirs; mine is in {}", copy.display()),
        Notice::CannotTakeTheirs => "cannot take theirs: the file on disk is not UTF-8".to_owned(),
        Notice::CopyFailed(reason) => format!("{quit_again}conflict copy failed: {reason}"),
        Notice::BackupFailed(reason) => format!("backup failed: {reason}"),
        Notice::Refused(Refusal::DoesNotNest) => {
            "only projects and checklist items nest".to_owned()
        }
        Notice::Refused(Refusal::NothingAbove) => "nothing above to indent into".to_owned(),
        Notice::Refused(Refusal::NotNested) => "nothing to outdent from".to_owned(),
        Notice::Refused(Refusal::NoRoomForSection) => {
            "a section goes only in an area or a project".to_owned()
        }
        Notice::Refused(Refusal::NoRoomForProject) => {
            "a project can't go in a section or the Inbox".to_owned()
        }
        Notice::Refused(Refusal::SectionHoldsTasks) => "a section holds only tasks".to_owned(),
        Notice::Refused(Refusal::ReadsAsTask) => {
            "only a task's title can start with \"- \"".to_owned()
        }
        Notice::TagInTitle => format!(
            "a title has no word starting with @; add tags with {}",
            keymap::written(Command::Tags).unwrap_or_default()
        ),
    });
    if let Some(notice) = notice {
        return notice;
    }
    match (state.opened(), state.save_state()) {
        (Opened::Demo, _) => "not saved".to_owned(),
        (Opened::File(_), SaveState::Failed { reason }) => {
            format!("{quit_again}save failed: {reason}")
        }
        (Opened::File(_), _) if state.gone_from_disk() => "not on disk".to_owned(),
        (Opened::File(_), SaveState::Unsaved) => "unsaved".to_owned(),
        (Opened::File(_), SaveState::Saved) => "saved".to_owned(),
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
    use crate::{app, domain::Document, ui::BuiltIn};

    #[track_caller]
    fn check(opened: &Opened, pending: &str, width: u16, expected: &str) {
        check_state(
            &State::new(Document::new(), opened.clone()),
            pending,
            width,
            expected,
        );
    }

    #[track_caller]
    fn check_state(state: &State, pending: &str, width: u16, expected: &str) {
        let theme = BuiltIn::default().colors();
        let mut buf = Buffer::empty(Rect::new(0, 0, width, 1));
        StatusLine::new(state, pending, &theme, IconSet::Plain).render(buf.area, &mut buf);
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
        let mut state = State::new(Document::new(), Opened::Demo);
        app::update(
            &mut state,
            app::Message::Command(app::Command::NewTask(app::Side::Below)),
            jiff::civil::date(2026, 10, 14),
        );
        check_state(&state, "", 36, " ✎ EDIT · Demo data · not saved     ");
    }

    // FR-UI-02
    #[test]
    fn a_file_shows_its_path_and_that_it_is_saved() {
        check(
            &Opened::File(PathBuf::from("todo.taskpaper")),
            "",
            48,
            " ↕ NAVIGATE · todo.taskpaper · saved            ",
        );
    }

    // FR-UI-02, FR-UI-06
    #[test]
    fn a_narrow_line_cuts_the_path_from_its_start() {
        let long = Opened::File(PathBuf::from("notes/work/todo.taskpaper"));
        check(&long, "", 36, " ↕ NAVIGATE · …odo.taskpaper · saved");
        check(&long, "", 26, " ↕ NAVIGATE · …per · saved");
        check(&long, "", 25, " ↕ NAVIGATE · …er · saved");
        check(&long, "", 23, " ↕ NAVIGATE · … · saved");
        check(&long, "", 22, " ↕ NAVIGATE · saved   ");
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
        check(&long, "g", 31, " ↕ NAVIGATE · …paper · saved g ");
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
