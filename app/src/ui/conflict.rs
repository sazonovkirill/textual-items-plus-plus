//! The conflict prompt: the window that opens over the panes when another program changed the
//! file while the document held changes it lacks, saying so, with its keys and, when asked, the
//! diff from the file on disk to the document
//! ([file-io.md §3](../../../specs/reference/taskpaper-terminal/specs/file-io.md#3-external-changes)).

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Widget, Wrap},
};
use similar::{ChangeTag, TextDiff};

use super::{Theme, keymap};
use crate::app::{Command, ConflictView};

/// The largest the window gets.
const SIZE: (u16, u16) = (80, 20);

/// Unchanged lines the diff shows around each change.
const CONTEXT: usize = 3;

/// One line of the diff from the file on disk to the document.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DiffLine {
    /// A line both have.
    Same(String),
    /// A line only the file on disk has.
    Theirs(String),
    /// A line only the document has.
    Mine(String),
    /// Unchanged lines left out between two changes.
    Gap,
}

/// The prompt of `view`, with the diff from `theirs` to `mine` while it is shown, in `theme`.
pub(super) struct ConflictPrompt<'a> {
    view: ConflictView,
    theirs: &'a str,
    mine: &'a str,
    theme: &'a Theme,
}

impl<'a> ConflictPrompt<'a> {
    /// The prompt of `view` over the file on disk holding `theirs` and the document `mine`.
    pub(super) fn new(
        view: ConflictView,
        theirs: &'a str,
        mine: &'a str,
        theme: &'a Theme,
    ) -> Self {
        Self {
            view,
            theirs,
            mine,
            theme,
        }
    }

    /// What the prompt says has happened.
    fn message(&self) -> &'static str {
        match (self.view.unsaved, self.view.readable) {
            (true, true) => "The file changed on disk. Your changes are not saved yet.",
            (true, false) => {
                "The file changed on disk and is not UTF-8. Your changes are not saved yet."
            }
            (false, _) => "The file on disk is not UTF-8, so it cannot be reloaded.",
        }
    }

    /// The keys that answer the prompt, each before what it does.
    fn keys(&self) -> Line<'static> {
        let diff = if self.view.diff {
            "Hide diff"
        } else {
            "Show diff"
        };
        let choices = [
            (Command::KeepMine, "Keep mine"),
            (Command::TakeTheirs, "Take theirs"),
            (Command::ShowDiff, diff),
            (Command::Quit, "Quit"),
        ];
        let key = Style::new().fg(self.theme.accent).bold();
        let spans = choices
            .into_iter()
            .filter_map(|(command, what)| Some((keymap::written(command)?, what)))
            .enumerate()
            .flat_map(|(index, (written, what))| {
                let gap = (index > 0).then(|| Span::raw("  "));
                gap.into_iter().chain([
                    Span::styled(shown_key(written), key),
                    Span::raw(format!(" {what}")),
                ])
            });
        Line::from_iter(spans)
    }

    /// The lines of the diff, each marked and colored by where it is from, with tabs, which a
    /// terminal cell cannot hold, as four spaces.
    fn diff(&self) -> Vec<Line<'static>> {
        let theme = self.theme;
        let line = |mark: &str, text: &str, color| {
            Line::styled(
                format!("{mark} {}", text.replace('\t', "    ")),
                Style::new().fg(color),
            )
        };
        diff(self.theirs, self.mine)
            .into_iter()
            .map(|diff_line| match diff_line {
                DiffLine::Same(text) => line(" ", &text, theme.dim_text),
                DiffLine::Theirs(text) => line("-", &text, theme.error),
                DiffLine::Mine(text) => line("+", &text, theme.start),
                DiffLine::Gap => line(" ", "…", theme.dim_text),
            })
            .collect()
    }
}

impl Widget for ConflictPrompt<'_> {
    /// Draws the window centered in `area`: the message, wrapped, then the diff while it is
    /// shown, as much of it as fits, and the keys on the last rows, which are kept first since
    /// they are the only way to answer.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = SIZE.0.min(area.width);
        let inner_width = width.saturating_sub(2);
        let message = Paragraph::new(self.message()).wrap(Wrap { trim: true });
        let keys = Paragraph::new(self.keys()).wrap(Wrap { trim: true });
        let height = |paragraph: &Paragraph| {
            u16::try_from(paragraph.line_count(inner_width)).unwrap_or(u16::MAX)
        };
        let (message_rows, key_rows) = (height(&message), height(&keys));
        let diff = if self.view.diff {
            self.diff()
        } else {
            Vec::new()
        };
        let diff_rows = u16::try_from(diff.len()).unwrap_or(u16::MAX);
        let wanted = message_rows
            .saturating_add(key_rows)
            .saturating_add(if diff.is_empty() {
                1
            } else {
                diff_rows.saturating_add(2)
            })
            .saturating_add(2);
        let area = area.centered(
            Constraint::Length(width),
            Constraint::Length(wanted.min(SIZE.1).min(area.height)),
        );
        let theme = self.theme;
        Clear.render(area, buf);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(Style::new().fg(theme.primary))
            .title("Changed on disk");
        let inner = frame.inner(area);
        frame.render(area, buf);
        let key_rows = key_rows.min(inner.height);
        let [rest, _, keys_area] = ratatui::layout::Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(u16::from(inner.height > key_rows)),
            Constraint::Length(key_rows),
        ])
        .areas(inner);
        keys.render(keys_area, buf);
        let [message_area, _, diff_area] = ratatui::layout::Layout::vertical([
            Constraint::Length(message_rows),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(rest);
        message.render(message_area, buf);
        Paragraph::new(diff).render(diff_area, buf);
    }
}

/// `written`, a key as the keymap writes it, as the prompt shows it: `ctrl+k` as `Ctrl+K`.
fn shown_key(written: &str) -> String {
    written
        .split('+')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// The lines that differ from `theirs` to `mine`, with [`CONTEXT`] unchanged lines around each
/// change and a [`DiffLine::Gap`] between changes further apart.
fn diff(theirs: &str, mine: &str) -> Vec<DiffLine> {
    let diff = TextDiff::from_lines(theirs, mine);
    let mut lines = Vec::new();
    for (index, group) in diff.grouped_ops(CONTEXT).iter().enumerate() {
        if index > 0 {
            lines.push(DiffLine::Gap);
        }
        for change in group.iter().flat_map(|op| diff.iter_changes(op)) {
            let text = change.value().trim_end_matches(['\n', '\r']).to_owned();
            lines.push(match change.tag() {
                ChangeTag::Equal => DiffLine::Same(text),
                ChangeTag::Delete => DiffLine::Theirs(text),
                ChangeTag::Insert => DiffLine::Mine(text),
            });
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-FILE-06
    #[test]
    fn the_diff_shows_the_changes_with_a_few_lines_around_each() {
        let theirs = (1..=12).map(|n| n.to_string() + "\n").collect::<String>();
        let mine = theirs
            .replacen("2\n", "two\n", 1)
            .replace("11\n", "eleven\n");
        let same = |n: u32| DiffLine::Same(n.to_string());
        assert_eq!(
            diff(&theirs, &mine),
            [
                same(1),
                DiffLine::Theirs("2".to_owned()),
                DiffLine::Mine("two".to_owned()),
                same(3),
                same(4),
                same(5),
                DiffLine::Gap,
                same(8),
                same(9),
                same(10),
                DiffLine::Theirs("11".to_owned()),
                DiffLine::Mine("eleven".to_owned()),
                same(12),
            ]
        );
        assert_eq!(diff("a\n", "a\n"), []);
    }

    // F-FILE-06
    #[test]
    fn the_diff_shows_tabs_as_spaces_and_a_gap_between_changes_far_apart() {
        let theirs = (1..=12)
            .map(|n| "\t".to_owned() + &n.to_string() + "\n")
            .collect::<String>();
        let mine = theirs
            .replacen("\t1\n", "\tone\n", 1)
            .replace("\t12\n", "\ttwelve\n");
        let view = ConflictView {
            unsaved: true,
            readable: true,
            diff: true,
        };
        let theme = crate::ui::BuiltIn::default().colors();
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 20));
        ConflictPrompt::new(view, &theirs, &mine, &theme).render(buf.area, &mut buf);
        let rows: Vec<String> = (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect();
        for row in [
            "│-     1",
            "│+     one",
            "│      4",
            "│  …",
            "│      9",
            "│+     twelve",
        ] {
            assert!(
                rows.iter().any(|shown| shown.starts_with(row)),
                "{row} in {rows:#?}"
            );
        }
    }

    #[test]
    fn keys_show_as_they_are_pressed() {
        assert_eq!(shown_key("ctrl+k"), "Ctrl+K");
        assert_eq!(shown_key("x"), "X");
        assert_eq!(shown_key(""), "");
    }
}
