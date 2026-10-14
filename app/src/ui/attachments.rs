//! The attachments panel in the terminal (FR-ATT-01–04, FR-CHK-02): the keys that drive it and
//! the panel itself, over the right side of the screen from its top to the status line, as
//! taskpaper-terminal draws it
//! ([attachments.md §3–5](../../../specs/reference/taskpaper-terminal/specs/attachments.md#3-attachments-panel-fr-att-03)):
//! a border titled with the item, the Note section under its heading, the note rendered as
//! Markdown or, while it is edited, as typed, a task's Checklist section under its heading, the
//! Comments section under its heading, newest first, what is selected on the current-line band,
//! and a hint naming the keys.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget, Wrap},
};

use super::{
    Icon, IconSet, Theme,
    container::tag_text,
    field,
    keymap::{self, Key},
    markdown::{self, Format, NoteLine},
};
use crate::{
    app::{AttachmentsInput, AttachmentsView, ChecklistLine, Focus, Selected},
    domain::comments,
};

/// The narrowest and widest the panel is drawn, screen width allowing.
const WIDTH: (u16, u16) = (40, 72);

/// What a key of the open panel does: something the core does, adding a comment, which takes the
/// frontend's clock, or a motion, which only the frontend knows how far goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelKey {
    /// Gives the core this input.
    Input(AttachmentsInput),
    /// Adds a comment stamped now.
    AddComment,
    /// Scrolls through what is selected, or selects another.
    Scroll(Scroll),
}

/// How far a motion of the panel goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scroll {
    /// This many lines down, or up when negative.
    Lines(isize),
    /// This many pages down, or up when negative.
    Pages(isize),
    /// To the top.
    Top,
    /// To the bottom.
    Bottom,
}

impl Scroll {
    /// Where the motion goes from the content shown from line `top`, with pages of `page` lines,
    /// when what is selected takes lines `selected`: the first line shown, and what else to
    /// select. While what is selected goes past the panel's edge that way the content scrolls
    /// through it, no further than its end; otherwise the next or previous is selected, and
    /// drawing scrolls it into view. The top selects the first with the first line, and the
    /// bottom the last with the last line, as far as there is; drawing keeps the content in view.
    pub(super) fn from(
        self,
        top: usize,
        page: usize,
        selected: (usize, usize),
    ) -> (usize, Option<Focus>) {
        let lines = match self {
            Self::Lines(lines) => lines,
            Self::Pages(pages) => pages.saturating_mul(page.cast_signed()),
            Self::Top => return (0, Some(Focus::First)),
            Self::Bottom => return (usize::MAX, Some(Focus::Last)),
        };
        let (start, end) = selected;
        let (beyond, focus) = if lines < 0 {
            (top.saturating_sub(start), Focus::Previous)
        } else {
            (end.saturating_sub(top + page), Focus::Next)
        };
        let lines = lines.unsigned_abs().min(beyond);
        match (lines, focus) {
            (0, focus) => (top, Some(focus)),
            (lines, Focus::Previous) => (top - lines, None),
            (lines, _) => (top + lines, None),
        }
    }
}

/// The keys of the open panel while nothing in it is edited, as taskpaper-terminal's
/// (keybindings.md §4).
pub(super) const KEYS: &[(Key, PanelKey)] = &[
    (
        (KeyModifiers::NONE, KeyCode::Esc),
        PanelKey::Input(AttachmentsInput::Close),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('e')),
        PanelKey::Input(AttachmentsInput::Edit),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Enter),
        PanelKey::Input(AttachmentsInput::Edit),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('d')),
        PanelKey::Input(AttachmentsInput::Delete),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Delete),
        PanelKey::Input(AttachmentsInput::Delete),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('a')),
        PanelKey::AddComment,
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('j')),
        PanelKey::Scroll(Scroll::Lines(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Down),
        PanelKey::Scroll(Scroll::Lines(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('k')),
        PanelKey::Scroll(Scroll::Lines(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Up),
        PanelKey::Scroll(Scroll::Lines(-1)),
    ),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('d')),
        PanelKey::Scroll(Scroll::Pages(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::PageDown),
        PanelKey::Scroll(Scroll::Pages(1)),
    ),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('u')),
        PanelKey::Scroll(Scroll::Pages(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::PageUp),
        PanelKey::Scroll(Scroll::Pages(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('g')),
        PanelKey::Scroll(Scroll::Top),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Home),
        PanelKey::Scroll(Scroll::Top),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('G')),
        PanelKey::Scroll(Scroll::Bottom),
    ),
    (
        (KeyModifiers::NONE, KeyCode::End),
        PanelKey::Scroll(Scroll::Bottom),
    ),
];

/// What `event` does in the open panel while nothing in it is edited: `Esc` closes it, `e` and
/// `Enter` edit what is selected, `d` and `Delete` delete the selected checklist item or comment,
/// `a` adds a comment, and the motion keys scroll or select. Only key presses count.
pub(super) fn panel_input(event: &Event) -> Option<PanelKey> {
    let key = keymap::key_press(event)?;
    KEYS.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
}

/// The panel's content laid out: its lines, the first line of each of the note, the checklist
/// items and the comments in the order they show, how many lines it takes, where the band of
/// what is selected ends at the latest, the Comments section being its own, and the line and
/// column of the caret while something is edited.
struct Content<'a> {
    lines: Vec<Line<'a>>,
    starts: Vec<(Selected, usize)>,
    total: usize,
    banded: usize,
    caret: Option<(usize, usize)>,
}

/// How the open panel is scrolled: the first line of the content shown, and what was selected
/// when it was last drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Scrolled {
    pub(super) top: usize,
    drawn: Option<Selected>,
}

/// The open panel. Its state is how it is scrolled, which it keeps within the content, on what is
/// selected and on the caret while something is edited.
pub(super) struct AttachmentsPanel<'a> {
    view: AttachmentsView<'a>,
    theme: &'a Theme,
    icons: IconSet,
}

impl<'a> AttachmentsPanel<'a> {
    /// The panel showing `view` in `theme` with `icons`.
    pub(super) fn new(view: AttachmentsView<'a>, theme: &'a Theme, icons: IconSet) -> Self {
        Self { view, theme, icons }
    }

    /// Where the panel goes over `screen`, the screen above the status line: at its right, half
    /// as wide within 40 to 72 columns, its whole height.
    pub(super) fn area(screen: Rect) -> Rect {
        let width = (screen.width / 2).clamp(WIDTH.0, WIDTH.1).min(screen.width);
        Rect::new(
            screen.right().saturating_sub(width),
            screen.y,
            width,
            screen.height,
        )
    }

    /// Where the content goes when the panel is drawn over `area`: inside the border, a column in
    /// from each side, above a blank line and the hint.
    fn content(area: Rect) -> Rect {
        let inner = Block::bordered().inner(area);
        Rect::new(
            inner.x.saturating_add(1),
            inner.y,
            inner.width.saturating_sub(2),
            inner.height.saturating_sub(2),
        )
    }

    /// How many lines a page of the content holds when the panel is drawn over `area`, one at
    /// least.
    pub(super) fn page(area: Rect) -> usize {
        usize::from(Self::content(area).height).max(1)
    }

    /// The columns the text being edited wraps in when the panel is drawn over `area`: all the
    /// content has but the last, kept for the caret at the end of a line.
    pub(super) fn columns(area: Rect) -> usize {
        usize::from(Self::content(area).width).saturating_sub(1)
    }

    /// Where the terminal's cursor goes when the panel is drawn over `area` scrolled to line
    /// `top`: at the caret of the text being edited, while its line is in view.
    pub(super) fn caret(&self, area: Rect, top: usize) -> Option<Position> {
        let content = Self::content(area);
        let (line, column) = self.laid_out(area).caret?;
        let y = line.checked_sub(top)?;
        (y < usize::from(content.height)).then(|| {
            let offset = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
            Position::new(
                content.x.saturating_add(offset(column)),
                content.y.saturating_add(offset(y)),
            )
        })
    }

    /// The content lines from the first to past the last of what is selected when the panel is
    /// drawn over `area`. The last of what it has runs to the end of the content.
    pub(super) fn selected_lines(&self, area: Rect) -> (usize, usize) {
        self.selected_in(&self.laid_out(area))
    }

    /// The lines of `content` from the first to past the last of what is selected.
    fn selected_in(&self, content: &Content<'a>) -> (usize, usize) {
        let at = content
            .starts
            .iter()
            .position(|&(entry, _)| entry == self.view.selected)
            .unwrap_or_default();
        let start = content.starts.get(at).map_or(0, |&(_, start)| start);
        let end = content
            .starts
            .get(at + 1)
            .map_or(content.total, |&(_, end)| end);
        (start, end)
    }

    /// The content laid out in the panel drawn over `area`: the Note section, its heading, then
    /// the note, rendered and wrapping, or a dimmed "No note", or while it is edited its text as
    /// typed, wrapped as the field wraps it; a blank line; for a task the Checklist section, its
    /// heading, then each item, or a dimmed "No checklist", and a blank line; the Comments
    /// section, its heading, then each comment, its timestamp dimmed and its text with its inline formatting, or a
    /// dimmed "No comments", the one edited as its timestamp above its text as typed.
    fn laid_out(&self, area: Rect) -> Content<'a> {
        let width = Self::content(area).width;
        let columns = Self::columns(area);
        // Lines wrap one by one, so the lines so far take what each takes.
        let mut counted = (0, 0);
        let mut height = |lines: &[Line<'a>]| {
            let (measured, total) = counted;
            let more = Paragraph::new(lines.get(measured..).unwrap_or_default().to_vec())
                .wrap(Wrap { trim: false })
                .line_count(width);
            counted = (lines.len(), total + more);
            counted.1
        };
        let dim = Style::new().fg(self.theme.dim_text);
        let item = self.view.item;
        let edited = self
            .view
            .editing
            .map(|field| field::wrapped(field, columns, self.theme));
        let editing = |at| edited.as_ref().filter(|_| self.view.selected == at);
        let mut lines = vec![self.heading(Icon::Notes, "Note")];
        let mut caret = None;
        if let Some((field, (line, column))) = editing(Selected::Note) {
            caret = Some((line + 1, *column));
            lines.extend(field.iter().cloned());
        } else {
            if item.note.is_empty() {
                lines.push(Line::styled("No note", dim));
            }
            lines.extend(
                markdown::note_lines(&item.note)
                    .into_iter()
                    .map(|line| self.line(line)),
            );
        }
        lines.push(Line::default());
        let mut starts = vec![(Selected::Note, 0)];
        if let Some(checklist) = &self.view.checklist {
            // The first item takes the section's heading with it.
            let mut start = height(&lines);
            lines.push(self.heading(Icon::Checklist, "Checklist"));
            if checklist.is_empty() {
                starts.push((Selected::Checklist(None), start));
                lines.push(Line::styled("No checklist", dim));
            }
            for (index, line) in checklist.iter().enumerate() {
                if index > 0 {
                    start = height(&lines);
                }
                starts.push((Selected::Checklist(Some(line.id)), start));
                let (shown, at) = self.checklist_line(line, usize::from(width));
                if let Some((line, column)) = at {
                    caret = Some((height(&lines) + line, column));
                }
                lines.extend(shown);
            }
            lines.push(Line::default());
        }
        let mut start = height(&lines);
        let comments_start = start;
        lines.push(self.heading(Icon::Comments, "Comments"));
        if item.comments.is_empty() {
            lines.push(Line::styled("No comments", dim));
        }
        for (index, comment) in item.comments.iter().enumerate() {
            if index > 0 {
                start = height(&lines);
            }
            starts.push((Selected::Comment(index), start));
            let (stamp, text) = comments::split(comment);
            if let Some((field, (line, column))) = editing(Selected::Comment(index)) {
                if !stamp.is_empty() {
                    lines.push(Line::styled(stamp.trim_end(), dim));
                }
                caret = Some((height(&lines) + line, *column));
                lines.extend(field.iter().cloned());
            } else {
                let mut line = Line::from(Span::styled(stamp, dim));
                line.spans.extend(self.formatted(text, Style::new()));
                lines.push(line);
            }
        }
        let total = height(&lines);
        Content {
            total,
            banded: if item.comments.is_empty() {
                comments_start
            } else {
                total
            },
            lines,
            starts,
            caret,
        }
    }

    /// A section's heading, with its icon, aligned to the right in the headers color.
    fn heading(&self, icon: Icon, name: &str) -> Line<'a> {
        Line::styled(
            format!("{} {name}", icon.glyph(self.icons)),
            Style::new().fg(self.theme.headers).bold(),
        )
        .right_aligned()
    }

    /// The lines of a checklist item in content `width`: indented by its depth, its marker,
    /// checked or not, then its text and tags, a checked item's struck through, all dimmed while
    /// it or an item above it is checked (FR-CHK-04); while its text is edited, the text as
    /// typed, wrapping after the marker, and the caret's line and column among them.
    fn checklist_line(
        &self,
        line: &ChecklistLine<'a>,
        width: usize,
    ) -> (Vec<Line<'a>>, Option<(usize, usize)>) {
        let item = line.item;
        let dim = if line.dimmed {
            Style::new().add_modifier(Modifier::DIM)
        } else {
            Style::new()
        };
        let (marker, title) = if item.completed.is_some() {
            let done = dim.fg(self.theme.done);
            (Icon::DoneTask, done.add_modifier(Modifier::CROSSED_OUT))
        } else {
            (Icon::Task, dim)
        };
        let prefix = format!("{}{} ", "  ".repeat(line.depth), marker.glyph(self.icons));
        let indent = Span::raw(&prefix).width();
        let marker_style = title.remove_modifier(Modifier::CROSSED_OUT);
        if let Some(field) = self.view.editing
            && self.view.selected == Selected::Checklist(Some(line.id))
        {
            let columns = width.saturating_sub(indent + 1);
            let (lines, (at, column)) = field::wrapped(field, columns, self.theme);
            let lines = lines
                .into_iter()
                .enumerate()
                .map(|(n, mut text)| {
                    let lead = if n == 0 {
                        prefix.clone()
                    } else {
                        " ".repeat(indent)
                    };
                    text.spans.insert(0, Span::styled(lead, marker_style));
                    text
                })
                .collect();
            return (lines, Some((at, indent + column)));
        }
        let mut spans = vec![
            Span::styled(prefix, marker_style),
            Span::styled(drawable(&item.title), title),
        ];
        let tag_color = if item.completed.is_some() {
            self.theme.done
        } else {
            self.theme.tag
        };
        for tag in item.shown_tags() {
            spans.push(Span::styled(
                tag_text(tag),
                dim.fg(tag_color).add_modifier(Modifier::DIM),
            ));
        }
        (vec![Line::from(spans)], None)
    }

    /// A line of the note as Markdown shows it: a heading bold, the first level also
    /// underlined; a list item after its marker, a bullet for `-`, `*` or `+`; a quote dimmed
    /// after a bar; code dimmed as typed; text after its indentation; each with its inline
    /// formatting.
    fn line(&self, line: NoteLine<'a>) -> Line<'a> {
        let dim = Style::new().add_modifier(Modifier::DIM);
        let (prefix, spans) = match line {
            NoteLine::Heading(level, text) => {
                let style = if level == 1 {
                    Style::new().bold().underlined()
                } else {
                    Style::new().bold()
                };
                (String::new(), self.formatted(text, style))
            }
            NoteLine::Item {
                depth,
                marker,
                text,
            } => {
                let marker = if matches!(marker, "-" | "*" | "+") {
                    "•"
                } else {
                    marker
                };
                (
                    format!("{}{marker} ", "  ".repeat(depth)),
                    self.formatted(text, Style::new()),
                )
            }
            NoteLine::Quote(text) => ("│ ".to_owned(), self.formatted(text, dim)),
            NoteLine::Code(text) => (String::new(), vec![Span::styled(drawable(text), dim)]),
            NoteLine::Text(text) => {
                let body = text.trim_start();
                let indent = text.get(..text.len() - body.len()).unwrap_or_default();
                (drawable(indent), self.formatted(body, Style::new()))
            }
        };
        let mut shown = Line::from(Span::styled(prefix, dim));
        shown.spans.extend(spans);
        shown
    }

    /// `text` with its inline formatting over `style`: bold, italic, code dimmed and links
    /// underlined in the link color.
    fn formatted(&self, text: &'a str, style: Style) -> Vec<Span<'a>> {
        markdown::runs(text)
            .into_iter()
            .map(|run| {
                let Format {
                    bold,
                    code,
                    italic,
                    link,
                } = run.format;
                let mut style = style;
                if bold {
                    style = style.bold();
                }
                if italic {
                    style = style.italic();
                }
                if code {
                    style = style.add_modifier(Modifier::DIM);
                }
                if link {
                    style = style.underlined().fg(self.theme.link);
                }
                Span::styled(drawable(run.text), style)
            })
            .collect()
    }

    /// The hint naming the keys of what is selected, or of what is edited.
    fn hint(&self) -> Line<'static> {
        let keys = match (self.view.editing, self.view.selected) {
            (Some(_), Selected::Note) => "Enter new line · Esc done",
            (Some(_), Selected::Checklist(_)) => "Enter next · Esc done",
            (Some(_), Selected::Comment(_)) => "Enter done · Esc cancel",
            (None, Selected::Note) => "a add · e edit · Esc close",
            (None, Selected::Checklist(Some(_))) => "o add · x check · d del · Esc close",
            (None, Selected::Checklist(None)) => "o add · Esc close",
            (None, Selected::Comment(_)) => "a add · e edit · d del · Esc close",
        };
        Line::styled(keys, Style::new().fg(self.theme.dim_text))
    }
}

/// `text` as the buffer can draw it: a tab shows as a space, the one column its width counts.
fn drawable(text: &str) -> String {
    text.replace('\t', " ")
}

impl StatefulWidget for AttachmentsPanel<'_> {
    type State = Scrolled;

    /// Draws the panel over the right side of `area`, its content from line `scrolled.top`, which
    /// it keeps within the content and on what is selected, scrolling a newly selected one into
    /// view from its start when it is taller than the panel, and while something is edited on the
    /// caret's line.
    fn render(self, area: Rect, buf: &mut Buffer, scrolled: &mut Scrolled) {
        let newly = scrolled.drawn.replace(self.view.selected) != Some(self.view.selected);
        let top = &mut scrolled.top;
        let area = Self::area(area);
        let theme = self.theme;
        Clear.render(area, buf);
        let accent = Style::new().fg(theme.accent);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(format!(" {} ", self.view.item.title));
        let inner = frame.inner(area);
        frame.render(area, buf);
        let content = Self::content(area);
        let laid_out = self.laid_out(area);
        let page = Self::page(area);
        let (start, end) = self.selected_in(&laid_out);
        *top = (*top).min(laid_out.total.saturating_sub(page));
        if let Some((line, _)) = laid_out.caret {
            *top = (*top).clamp((line + 1).saturating_sub(page), line);
        } else if end - start > page {
            if newly || end <= *top || start >= *top + page {
                *top = start;
            }
        } else {
            *top = (*top).clamp(end.saturating_sub(page), start);
        }
        let band = Style::new().bg(theme.current_line);
        // With no comments the last of what is selected runs to the end, but its band stops at its
        // own section's end.
        for line in start.max(*top)..end.min(laid_out.banded).min(*top + page) {
            let y = content.y + u16::try_from(line - *top).unwrap_or(u16::MAX);
            buf.set_style(Rect::new(inner.x, y, inner.width, 1), band);
        }
        Paragraph::new(laid_out.lines)
            .wrap(Wrap { trim: false })
            .scroll((u16::try_from(*top).unwrap_or(u16::MAX), 0))
            .render(content, buf);
        if inner.height >= 2 {
            let hint = Rect::new(content.x, inner.bottom() - 1, content.width, 1);
            self.hint().render(hint, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyEvent;

    use super::*;
    use crate::app::{Motion, TextField};
    use crate::{
        app::TextEdit,
        domain::{Item, ItemId, ItemKind},
        ui::BuiltIn,
    };

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    // FR-ATT-02, FR-ATT-03, FR-ATT-04
    #[test]
    fn escape_closes_e_and_enter_edit_d_deletes_a_adds_and_the_motion_keys_scroll() {
        let none = KeyModifiers::NONE;
        for (code, modifiers, key) in [
            (KeyCode::Esc, none, PanelKey::Input(AttachmentsInput::Close)),
            (
                KeyCode::Char('e'),
                none,
                PanelKey::Input(AttachmentsInput::Edit),
            ),
            (
                KeyCode::Enter,
                none,
                PanelKey::Input(AttachmentsInput::Edit),
            ),
            (
                KeyCode::Char('d'),
                none,
                PanelKey::Input(AttachmentsInput::Delete),
            ),
            (
                KeyCode::Delete,
                none,
                PanelKey::Input(AttachmentsInput::Delete),
            ),
            (KeyCode::Char('a'), none, PanelKey::AddComment),
            (KeyCode::Char('j'), none, PanelKey::Scroll(Scroll::Lines(1))),
            (KeyCode::Up, none, PanelKey::Scroll(Scroll::Lines(-1))),
            (
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
                PanelKey::Scroll(Scroll::Pages(1)),
            ),
            (KeyCode::PageUp, none, PanelKey::Scroll(Scroll::Pages(-1))),
            (KeyCode::Char('g'), none, PanelKey::Scroll(Scroll::Top)),
            (
                KeyCode::Char('G'),
                KeyModifiers::SHIFT,
                PanelKey::Scroll(Scroll::Bottom),
            ),
        ] {
            assert_eq!(
                panel_input(&press(code, modifiers)),
                Some(key),
                "{code:?} {modifiers:?}"
            );
        }
        assert_eq!(panel_input(&press(KeyCode::Char('x'), none)), None);
    }

    // FR-ATT-01
    #[test]
    fn motions_scroll_through_what_is_selected_past_the_edge_else_select_another() {
        let tall = (2, 40);
        assert_eq!(Scroll::Lines(1).from(3, 10, tall), (4, None));
        assert_eq!(Scroll::Pages(2).from(3, 10, tall), (23, None));
        assert_eq!(Scroll::Pages(3).from(3, 10, tall), (30, None), "to its end");
        assert_eq!(Scroll::Lines(1).from(30, 10, tall), (30, Some(Focus::Next)));
        assert_eq!(Scroll::Lines(-1).from(3, 10, tall), (2, None));
        assert_eq!(
            Scroll::Pages(-1).from(5, 10, tall),
            (2, None),
            "to its start"
        );
        assert_eq!(
            Scroll::Lines(-5).from(2, 10, tall),
            (2, Some(Focus::Previous))
        );
        let short = (4, 6);
        assert_eq!(Scroll::Lines(1).from(3, 10, short), (3, Some(Focus::Next)));
        assert_eq!(
            Scroll::Pages(-1).from(3, 10, short),
            (3, Some(Focus::Previous))
        );
        assert_eq!(Scroll::Top.from(3, 10, short), (0, Some(Focus::First)));
        assert_eq!(
            Scroll::Bottom.from(3, 10, short),
            (usize::MAX, Some(Focus::Last))
        );
    }

    /// Every row of `buf`, without trailing spaces.
    fn rows(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    /// The panel on an item titled `Milk` with note `note`, its note selected and edited in
    /// `editing` if given, drawn in Plain icons over `width` × `height` cells from line `top`;
    /// the rows, the line drawn from and the caret.
    fn draw(
        note: &str,
        editing: Option<&TextField>,
        size: (u16, u16),
        top: usize,
    ) -> (Buffer, usize, Option<Position>) {
        let item = Item {
            note: note.to_owned(),
            ..Item::new(ItemKind::Task, "Milk")
        };
        draw_item(&item, Selected::Note, editing, size, top)
    }

    /// The panel on `item` with `selected` selected and edited in `editing` if given, drawn in
    /// Plain icons over `width` × `height` cells from line `top`; the rows, the line drawn from
    /// and the caret.
    fn draw_item(
        item: &Item,
        selected: Selected,
        editing: Option<&TextField>,
        (width, height): (u16, u16),
        top: usize,
    ) -> (Buffer, usize, Option<Position>) {
        let theme = BuiltIn::default().colors();
        let view = AttachmentsView {
            id: ItemId::new(1),
            item,
            selected,
            checklist: None,
            editing,
        };
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        let mut scrolled = Scrolled {
            top,
            drawn: Some(selected),
        };
        AttachmentsPanel::new(view.clone(), &theme, IconSet::Plain).render(
            area,
            &mut buf,
            &mut scrolled,
        );
        let caret = AttachmentsPanel::new(view, &theme, IconSet::Plain)
            .caret(AttachmentsPanel::area(area), scrolled.top);
        (buf, scrolled.top, caret)
    }

    // FR-ATT-01
    #[test]
    fn the_panel_covers_the_right_half_within_40_to_72_columns() {
        assert_eq!(
            AttachmentsPanel::area(Rect::new(0, 0, 100, 20)),
            Rect::new(50, 0, 50, 20)
        );
        assert_eq!(
            AttachmentsPanel::area(Rect::new(0, 0, 60, 20)),
            Rect::new(20, 0, 40, 20)
        );
        assert_eq!(
            AttachmentsPanel::area(Rect::new(0, 0, 200, 20)),
            Rect::new(128, 0, 72, 20)
        );
        assert_eq!(
            AttachmentsPanel::area(Rect::new(0, 0, 30, 20)),
            Rect::new(0, 0, 30, 20)
        );
    }

    // FR-ATT-01, FR-ATT-03
    #[test]
    fn a_note_shows_rendered_as_markdown_under_its_heading() {
        let note = "# Plan\n## Steps\n- **buy** milk\n  * see [shop](https://a.b)\n1. one\n> said\n```\n  code\n```\n\tindented";
        let (buf, top, caret) = draw(note, None, (80, 16), 0);
        assert_eq!((top, caret), (0, None));
        assert!(
            (0..40).all(|x| (0..16).all(|y| buf[(x, y)].symbol() == " ")),
            "only the right half is drawn"
        );
        let panel: Vec<String> = rows(&buf)
            .iter()
            .map(|row| row.get(40..).unwrap_or_default().to_owned())
            .collect();
        assert_eq!(
            panel,
            [
                "┌ Milk ────────────────────────────────┐",
                "│                               ≡ Note │",
                "│ Plan                                 │",
                "│ Steps                                │",
                "│ • buy milk                           │",
                "│   • see shop                         │",
                "│ 1. one                               │",
                "│ │ said                               │",
                "│   code                               │",
                "│  indented                            │",
                "│                                      │",
                "│                           ❞ Comments │",
                "│ No comments                          │",
                "│                                      │",
                "│ a add · e edit · Esc close           │",
                "└──────────────────────────────────────┘",
            ]
        );
        let theme = BuiltIn::default().colors();
        let cell = |x: u16, y: u16| &buf[(x, y)];
        assert!(cell(42, 2).modifier.contains(Modifier::UNDERLINED), "h1");
        assert!(cell(42, 3).modifier.contains(Modifier::BOLD), "h2");
        assert!(!cell(42, 3).modifier.contains(Modifier::UNDERLINED));
        assert!(cell(44, 4).modifier.contains(Modifier::BOLD), "bold");
        assert_eq!(cell(52, 5).fg, theme.link, "link");
        assert!(cell(42, 7).modifier.contains(Modifier::DIM), "quote");
        assert!(cell(44, 8).modifier.contains(Modifier::DIM), "code");
        assert_eq!(cell(72, 1).fg, theme.headers, "heading");
    }

    // FR-ATT-03
    #[test]
    fn inline_formats_show_in_their_styles() {
        let (buf, ..) = draw("*it* `c` **b**", None, (40, 8), 0);
        let cell = |x: u16| &buf[(x, 2)];
        assert!(cell(2).modifier.contains(Modifier::ITALIC));
        assert!(cell(5).modifier.contains(Modifier::DIM));
        assert!(cell(7).modifier.contains(Modifier::BOLD));
        assert_eq!(rows(&buf)[2], "│ it c b                               │");
    }

    // FR-ATT-01
    #[test]
    fn no_note_shows_dimmed() {
        let (buf, ..) = draw("", None, (40, 6), 0);
        assert_eq!(rows(&buf)[2], "│ No note                              │");
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(2, 2)].fg, theme.dim_text);
    }

    // FR-ATT-01
    #[test]
    fn the_content_scrolls_no_further_than_its_last_line_and_wraps() {
        let note = (1..=10)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (buf, top, _) = draw(&note, None, (40, 8), usize::MAX);
        assert_eq!(top, 10, "fourteen lines, four in view");
        assert_eq!(
            rows(&buf)[1..5].to_vec(),
            [
                "│ line 10                              │",
                "│                                      │",
                "│                           ❞ Comments │",
                "│ No comments                          │",
            ]
        );
        let long = "word ".repeat(12);
        let (buf, ..) = draw(&long, None, (40, 8), 0);
        assert_eq!(rows(&buf)[2], "│ word word word word word word word   │");
        assert_eq!(rows(&buf)[3], "│ word word word word word             │");
    }

    // FR-ATT-03
    #[test]
    fn a_note_edited_shows_as_typed_with_the_caret_in_view() {
        let mut field = TextField::multi_line("# Plan\n**b**");
        let (buf, top, caret) = draw("# Plan\n**b**", Some(&field), (40, 8), 0);
        assert_eq!(top, 0);
        assert_eq!(
            rows(&buf)[1..4].to_vec(),
            [
                "│                               ≡ Note │",
                "│ # Plan                               │",
                "│ **b**                                │",
            ]
        );
        assert_eq!(rows(&buf)[6], "│ Enter new line · Esc done            │");
        assert_eq!(caret, Some(Position::new(7, 3)));
        let text = (1..=10)
            .map(|n| format!("{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        field = TextField::multi_line(&text);
        let (_, top, caret) = draw(&text, Some(&field), (40, 8), 0);
        assert_eq!((top, caret), (7, Some(Position::new(4, 4))));
        field = field.edited(TextEdit::Move {
            motion: Motion::Start,
            select: false,
        });
        let (_, top, caret) = draw(&text, Some(&field), (40, 8), 7);
        assert_eq!((top, caret), (1, Some(Position::new(2, 1))));
        assert_eq!(
            AttachmentsPanel::new(
                AttachmentsView {
                    id: ItemId::new(1),
                    item: &Item::new(ItemKind::Task, "x"),
                    selected: Selected::Note,
                    checklist: None,
                    editing: Some(&field),
                },
                &BuiltIn::default().colors(),
                IconSet::Plain,
            )
            .caret(Rect::new(0, 0, 40, 8), 5),
            None,
            "a caret scrolled out of view shows nowhere"
        );
        let (_, _, caret) = draw(&text, Some(&field), (40, 4), 0);
        assert_eq!(caret, None, "a panel too short for its content still draws");
    }

    /// An item titled `Milk` with note `note` and comments `comments`.
    fn commented(note: &str, comments: &[&str]) -> Item {
        Item {
            note: note.to_owned(),
            comments: comments.iter().map(|&comment| comment.to_owned()).collect(),
            ..Item::new(ItemKind::Task, "Milk")
        }
    }

    // FR-ATT-01, FR-ATT-04
    #[test]
    fn comments_show_newest_first_under_their_heading_the_selected_one_on_the_band() {
        let item = commented(
            "n",
            &[
                "[2026-10-05 09:30] **Done**",
                "plain by hand",
                "[2026-10-04 18:02] Called",
            ],
        );
        let (buf, top, caret) = draw_item(&item, Selected::Comment(1), None, (40, 12), 0);
        assert_eq!((top, caret), (0, None));
        assert_eq!(
            rows(&buf),
            [
                "┌ Milk ────────────────────────────────┐",
                "│                               ≡ Note │",
                "│ n                                    │",
                "│                                      │",
                "│                           ❞ Comments │",
                "│ [2026-10-05 09:30] Done              │",
                "│ plain by hand                        │",
                "│ [2026-10-04 18:02] Called            │",
                "│                                      │",
                "│                                      │",
                "│ a add · e edit · d del · Esc close   │",
                "└──────────────────────────────────────┘",
            ]
        );
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(2, 5)].fg, theme.dim_text, "the timestamp is dimmed");
        assert!(
            buf[(21, 5)].modifier.contains(Modifier::BOLD),
            "inline Markdown"
        );
        let banded = |y: u16| (1..39).all(|x| buf[(x, y)].bg == theme.current_line);
        assert!(banded(6));
        assert!(!banded(5) && !banded(7) && !banded(2));
        let (buf, ..) = draw_item(&item, Selected::Comment(0), None, (40, 12), 0);
        assert!(
            (4..6).all(|y| buf[(1, y)].bg == theme.current_line),
            "the first comment takes the heading with it"
        );
        let (buf, ..) = draw_item(&item, Selected::Note, None, (40, 12), 0);
        assert!((1..4).all(|y| buf[(1, y)].bg == theme.current_line));
        assert_ne!(buf[(1, 4)].bg, theme.current_line);
    }

    // FR-ATT-04
    #[test]
    fn a_comment_edited_shows_its_timestamp_above_its_text_as_typed() {
        let item = commented("", &["[2026-10-05 09:30] Called", "by hand"]);
        let field = TextField::new("Called them".to_owned());
        let (buf, _, caret) = draw_item(&item, Selected::Comment(0), Some(&field), (40, 11), 0);
        assert_eq!(
            rows(&buf)[4..8].to_vec(),
            [
                "│                           ❞ Comments │",
                "│ [2026-10-05 09:30]                   │",
                "│ Called them                          │",
                "│ by hand                              │",
            ]
        );
        assert_eq!(rows(&buf)[9], "│ Enter done · Esc cancel              │");
        assert_eq!(caret, Some(Position::new(13, 6)));
        let (buf, _, caret) = draw_item(&item, Selected::Comment(1), Some(&field), (40, 11), 0);
        assert_eq!(
            rows(&buf)[6],
            "│ Called them                          │",
            "no timestamp"
        );
        assert_eq!(caret, Some(Position::new(13, 6)));
    }

    // FR-ATT-01, FR-ATT-04
    #[test]
    fn a_newly_selected_item_scrolls_into_view_from_its_start_when_taller() {
        let lines = (1..=10).map(|n| format!("c{n}")).collect::<Vec<_>>();
        let comments: Vec<&str> = lines.iter().map(String::as_str).collect();
        let item = commented("", &comments);
        let (_, top, _) = draw_item(&item, Selected::Comment(9), None, (40, 8), 0);
        assert_eq!(top, 10, "the last comment is the last line in view");
        let (_, top, _) = draw_item(&item, Selected::Note, None, (40, 8), 10);
        assert_eq!(top, 0);
        let note = (1..=10)
            .map(|n| format!("n{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let item = commented(&note, &["c"]);
        let theme = BuiltIn::default().colors();
        let view = |selected| AttachmentsView {
            id: ItemId::new(1),
            item: &item,
            selected,
            checklist: None,
            editing: None,
        };
        let area = Rect::new(0, 0, 40, 8);
        let mut buf = Buffer::empty(area);
        let mut scrolled = Scrolled {
            top: 10,
            drawn: Some(Selected::Comment(0)),
        };
        AttachmentsPanel::new(view(Selected::Note), &theme, IconSet::Plain).render(
            area,
            &mut buf,
            &mut scrolled,
        );
        assert_eq!(
            scrolled.top, 0,
            "from the start of a note taller than the panel"
        );
        scrolled.top = 5;
        AttachmentsPanel::new(view(Selected::Note), &theme, IconSet::Plain).render(
            area,
            &mut buf,
            &mut scrolled,
        );
        assert_eq!(scrolled.top, 5, "scrolling through it stays");
        let (_, top, _) = draw_item(&item, Selected::Note, None, (40, 8), 20);
        assert_eq!(top, 10, "in view");
        let panel = AttachmentsPanel::new(view(Selected::Comment(0)), &theme, IconSet::Plain);
        assert_eq!(panel.selected_lines(area), (12, 14));
    }

    /// A checklist line of `item` at `depth`, dimmed or not.
    fn line(item: &Item, id: u64, depth: usize, dimmed: bool) -> ChecklistLine<'_> {
        ChecklistLine {
            id: ItemId::new(id),
            item,
            depth,
            dimmed,
        }
    }

    /// The panel on a task `Pack` with checklist items `Books`, checked, holding `Novels`, and
    /// `Plates @home(kitchen)`, `selected` selected and edited in `editing` if given, drawn over
    /// 40 × `height` cells from line `top`; the buffer, the line drawn from and the caret.
    fn draw_checklist(
        selected: Selected,
        editing: Option<&TextField>,
        height: u16,
        top: usize,
    ) -> (Buffer, usize, Option<Position>) {
        let task = Item::new(ItemKind::Task, "Pack");
        let mut books = Item::new(ItemKind::ChecklistItem, "Books");
        books.completed = Some(crate::domain::Completion::Undated);
        let novels = Item::new(ItemKind::ChecklistItem, "Novels");
        let mut plates = Item::new(ItemKind::ChecklistItem, "Plates");
        plates.tags = vec![crate::domain::Tag {
            name: "home".to_owned(),
            value: Some("kitchen".to_owned()),
        }];
        let theme = BuiltIn::default().colors();
        let view = AttachmentsView {
            id: ItemId::new(1),
            item: &task,
            selected,
            checklist: Some(vec![
                line(&books, 2, 0, true),
                line(&novels, 3, 1, true),
                line(&plates, 4, 0, false),
            ]),
            editing,
        };
        let area = Rect::new(0, 0, 40, height);
        let mut buf = Buffer::empty(area);
        let mut scrolled = Scrolled {
            top,
            drawn: Some(selected),
        };
        AttachmentsPanel::new(view.clone(), &theme, IconSet::Plain).render(
            area,
            &mut buf,
            &mut scrolled,
        );
        let caret = AttachmentsPanel::new(view, &theme, IconSet::Plain).caret(area, scrolled.top);
        (buf, scrolled.top, caret)
    }

    // FR-CHK-02, FR-CHK-04
    #[test]
    fn the_checklist_shows_between_the_note_and_the_comments_checked_items_dimmed() {
        let selected = Selected::Checklist(Some(ItemId::new(3)));
        let (buf, top, caret) = draw_checklist(selected, None, 16, 0);
        assert_eq!((top, caret), (0, None));
        assert_eq!(
            rows(&buf),
            [
                "┌ Pack ────────────────────────────────┐",
                "│                               ≡ Note │",
                "│ No note                              │",
                "│                                      │",
                "│                          ☑ Checklist │",
                "│ ✔ Books                              │",
                "│   - Novels                           │",
                "│ - Plates @home(kitchen)              │",
                "│                                      │",
                "│                           ❞ Comments │",
                "│ No comments                          │",
                "│                                      │",
                "│                                      │",
                "│                                      │",
                "│ o add · x check · d del · Esc close  │",
                "└──────────────────────────────────────┘",
            ]
        );
        let theme = BuiltIn::default().colors();
        let books = &buf[(4, 5)];
        assert_eq!(books.fg, theme.done);
        assert!(
            books
                .modifier
                .contains(Modifier::DIM | Modifier::CROSSED_OUT)
        );
        let novels = &buf[(6, 6)];
        assert!(
            novels.modifier.contains(Modifier::DIM),
            "under a checked item"
        );
        assert!(!novels.modifier.contains(Modifier::CROSSED_OUT));
        assert_eq!(
            novels.bg, theme.current_line,
            "the item selected is on the band"
        );
        assert_eq!(
            buf[(1, 6)].bg,
            theme.current_line,
            "the band spans the panel"
        );
        assert_eq!(buf[(4, 7)].bg, theme.background);
        assert!(!buf[(4, 7)].modifier.contains(Modifier::DIM));
        assert_eq!(buf[(11, 7)].fg, theme.tag);
    }

    // FR-CHK-02
    #[test]
    fn the_first_item_takes_the_heading_with_it() {
        let selected = Selected::Checklist(Some(ItemId::new(2)));
        let (buf, top, _) = draw_checklist(selected, None, 7, 0);
        assert_eq!(top, 2, "the heading above it in view");
        assert_eq!(rows(&buf)[2], "│                          ☑ Checklist │");
        assert_eq!(
            buf[(1, 2)].bg,
            BuiltIn::default().colors().current_line,
            "on the band with it"
        );
    }

    // FR-CHK-03
    #[test]
    fn an_item_edited_shows_its_text_as_typed_with_the_caret() {
        let field = TextField::new("Plates and bowls and cups".to_owned());
        let selected = Selected::Checklist(Some(ItemId::new(4)));
        let (buf, top, caret) = draw_checklist(selected, Some(&field), 16, 0);
        assert_eq!(top, 0);
        assert_eq!(
            (rows(&buf)[7].as_str(), rows(&buf)[14].as_str()),
            (
                "│ - Plates and bowls and cups          │",
                "│ Enter next · Esc done                │",
            )
        );
        assert_eq!(caret, Some(Position::new(29, 7)));
        let long = TextField::new("Plates and bowls and cups and glasses".to_owned());
        let (buf, _, caret) = draw_checklist(selected, Some(&long), 16, 0);
        assert_eq!(
            rows(&buf)[7..9],
            [
                "│ - Plates and bowls and cups and      │",
                "│   glasses                            │",
            ],
            "wrapping after the marker"
        );
        assert_eq!(caret, Some(Position::new(11, 8)));
    }

    // FR-CHK-02
    #[test]
    fn an_empty_checklist_says_so() {
        let task = Item::new(ItemKind::Task, "Pack");
        let theme = BuiltIn::default().colors();
        let view = AttachmentsView {
            id: ItemId::new(1),
            item: &task,
            selected: Selected::Checklist(None),
            checklist: Some(Vec::new()),
            editing: None,
        };
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        AttachmentsPanel::new(view.clone(), &theme, IconSet::Plain).render(
            area,
            &mut buf,
            &mut Scrolled::default(),
        );
        assert_eq!(rows(&buf)[5], "│ No checklist                         │");
        assert_eq!(buf[(2, 5)].bg, theme.current_line);
        assert_eq!(
            AttachmentsPanel::new(view, &theme, IconSet::Plain).selected_lines(area),
            (3, 8),
            "its heading and line, running to the end with no comments after it"
        );
        assert_eq!(banded(&buf), [4, 5, 6], "the Comments section off the band");
    }

    /// The rows of `buf` on the current-line band.
    fn banded(buf: &Buffer) -> Vec<u16> {
        let band = BuiltIn::default().colors().current_line;
        (0..buf.area.height)
            .filter(|&y| buf[(1, y)].bg == band)
            .collect()
    }

    // FR-ATT-04, FR-CHK-02
    #[test]
    fn with_no_comments_the_band_stops_at_its_own_section() {
        let (buf, ..) = draw_checklist(Selected::Checklist(Some(ItemId::new(4))), None, 16, 0);
        assert_eq!(rows(&buf)[9], "│                           ❞ Comments │");
        assert_eq!(banded(&buf), [7, 8], "the last checklist item");
        let (buf, ..) = draw("n", None, (40, 10), 0);
        assert_eq!(rows(&buf)[4], "│                           ❞ Comments │");
        assert_eq!(banded(&buf), [1, 2, 3], "the note");
    }
}
