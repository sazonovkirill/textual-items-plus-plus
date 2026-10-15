//! The attachments panel in the terminal (FR-ATT-01–04, FR-CHK-02): the keys that drive it and
//! the panel itself, over the right part of the pane it is open in, from its top to the status
//! line, as
//! taskpaper-terminal draws it
//! ([attachments.md §3–5](../../../specs/reference/taskpaper-terminal/specs/attachments.md#3-attachments-panel-fr-att-03)):
//! a border titled with the item, the Note section under its heading, the note rendered as
//! Markdown or, while it is edited, as typed, a task's Checklist section under its heading, the
//! Comments section under its heading, newest first, what is selected on the current-line band
//! with the focus marker while it has the keys, and a hint naming the keys.

use std::ops::Range;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use jiff::civil::{Date, DateTime};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget, Wrap},
};
use unicode_width::UnicodeWidthStr;

use super::{
    FOCUS_MARKER, Icon, IconSet, Theme,
    container::tag_text,
    field,
    keymap::{self, Key},
    markdown::{self, Format, NoteLine},
};
use crate::{
    app::{self, AttachmentsInput, AttachmentsView, ChecklistLine, Focus, Selected},
    domain::comments,
};

/// The narrowest and widest the panel is drawn, screen width allowing.
const WIDTH: (u16, u16) = (40, 72);

/// What a key of the open panel does: something the core does, a command the panel acts on,
/// adding a comment, which takes the frontend's clock, or a motion, which only the frontend knows
/// how far goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelKey {
    /// Gives the core this input.
    Input(AttachmentsInput),
    /// Adds a comment stamped now.
    AddComment,
    /// Widens the panel over the whole of its pane, or closes it once it is, so that `\\` goes
    /// round closed, half and full (D-129).
    Cycle,
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
    ((KeyModifiers::NONE, KeyCode::Char('\\')), PanelKey::Cycle),
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
/// `a` adds a comment, `\\` widens or narrows the panel, and the motion keys scroll or select;
/// `g` is left to start the keymap's sequences, as `gg` selecting the first entry and `gn` moving
/// the keys on. Only key presses count.
pub(super) fn panel_input(event: &Event) -> Option<PanelKey> {
    let key = keymap::key_press(event)?;
    KEYS.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
}

/// The panel's content laid out: its lines, where each of the note, the checklist items and the
/// comments shows, in the order they show, how many lines it takes, and the line and column of
/// the caret while something is edited.
struct Content<'a> {
    lines: Vec<Line<'a>>,
    entries: Vec<Entry>,
    total: usize,
    caret: Option<(usize, usize)>,
}

/// Where an entry of the panel shows: the entry, its first line, its section's heading when it is
/// the section's first, and its own lines, which the band covers.
struct Entry(Selected, usize, Range<usize>);

/// How the open panel is scrolled: the first line of the content shown, and what was selected
/// when it was last drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Scrolled {
    pub(super) top: usize,
    drawn: Option<Selected>,
}

/// The open panel on `today`, the local date, which says how far back a comment's time is
/// written, over half its pane or, when `full`, the whole of it. Its state is how it is
/// scrolled, which it keeps within the content, on what is selected and on the caret while
/// something is edited.
pub(super) struct AttachmentsPanel<'a> {
    view: AttachmentsView<'a>,
    theme: &'a Theme,
    icons: IconSet,
    today: Date,
    full: bool,
    focused: bool,
}

impl<'a> AttachmentsPanel<'a> {
    /// The panel showing `view` on `today` in `theme` with `icons`.
    pub(super) fn new(
        view: AttachmentsView<'a>,
        theme: &'a Theme,
        icons: IconSet,
        today: Date,
    ) -> Self {
        Self {
            view,
            theme,
            icons,
            today,
            full: false,
            focused: true,
        }
    }

    /// The panel over the whole of its pane when `full`, as its hint then says.
    pub(super) fn full(self, full: bool) -> Self {
        Self { full, ..self }
    }

    /// The panel with the keys when `focused`, what is selected marked as the focused row is;
    /// without them nothing is marked and its border is dim, as a pane without the keys marks
    /// no row.
    pub(super) fn focused(self, focused: bool) -> Self {
        Self { focused, ..self }
    }

    /// Where the panel goes over `pane`, the pane it is open in, while it is not full: at its
    /// right, half as wide within 40 to 72 columns, its whole height, or the whole pane when it
    /// is narrower than that.
    pub(super) fn area(pane: Rect) -> Rect {
        let width = (pane.width / 2).clamp(WIDTH.0, WIDTH.1).min(pane.width);
        Rect::new(
            pane.right().saturating_sub(width),
            pane.y,
            width,
            pane.height,
        )
    }

    /// Where the content goes when the panel is drawn over `area`: inside the border, a column in
    /// from each side, above a blank line and the hint's lines.
    fn content(&self, area: Rect) -> Rect {
        let inner = Block::bordered().inner(area);
        let hint = u16::try_from(self.hint(Self::width(area)).len()).unwrap_or(u16::MAX);
        Rect::new(
            inner.x.saturating_add(1),
            inner.y,
            Self::width(area),
            inner.height.saturating_sub(hint.saturating_add(1)),
        )
    }

    /// How wide the content is when the panel is drawn over `area`: inside the border, a column in
    /// from each side.
    fn width(area: Rect) -> u16 {
        Block::bordered().inner(area).width.saturating_sub(2)
    }

    /// How many lines a page of the content holds when the panel is drawn over `area`, one at
    /// least.
    pub(super) fn page(&self, area: Rect) -> usize {
        usize::from(self.content(area).height).max(1)
    }

    /// The columns the text being edited wraps in when the panel is drawn over `area`: all the
    /// content has but the last, kept for the caret at the end of a line.
    pub(super) fn columns(area: Rect) -> usize {
        usize::from(Self::width(area)).saturating_sub(1)
    }

    /// Where the terminal's cursor goes when the panel is drawn over `area` scrolled to line
    /// `top`: at the caret of the text being edited, while it is in view.
    pub(super) fn caret(&self, area: Rect, top: usize) -> Option<Position> {
        let content = self.content(area);
        let (line, column) = self.laid_out(area).caret?;
        let y = line.checked_sub(top)?;
        (y < usize::from(content.height) && column < usize::from(content.width)).then(|| {
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
        self.selected_in(&self.laid_out(area)).0
    }

    /// The lines of `content` from the first to past the last of what is selected, up to the
    /// next entry, and its own lines.
    fn selected_in(&self, content: &Content<'a>) -> ((usize, usize), Range<usize>) {
        let at = content
            .entries
            .iter()
            .position(|&Entry(entry, ..)| entry == self.view.selected)
            .unwrap_or_default();
        let mut from = content.entries.iter().skip(at);
        let (start, own) = from
            .next()
            .map_or((0, 0..0), |Entry(_, start, own)| (*start, own.clone()));
        let end = from.next().map_or(content.total, |&Entry(_, next, _)| next);
        ((start, end), own)
    }

    /// The content laid out in the panel drawn over `area`: the Note section, its heading, then
    /// the note, rendered and wrapping, or a dimmed "No note", or while it is edited its text as
    /// typed, wrapped as the field wraps it; a blank line; for a task the Checklist section, its
    /// heading, then each item, or a dimmed "No checklist", and a blank line; the Comments
    /// section, its heading, then each comment, its timestamp dimmed and its text with its inline formatting, or a
    /// dimmed "No comments", the one edited as its timestamp above its text as typed.
    fn laid_out(&self, area: Rect) -> Content<'a> {
        let width = Self::width(area);
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
        let own = height(&lines);
        let mut caret = None;
        if let Some((field, (line, column))) = editing(Selected::Note) {
            caret = Some((line + 1, *column));
            lines.extend(field.iter().cloned());
        } else {
            if item.note.is_empty() {
                lines.push(Line::styled("No note", dim));
            }
            for line in markdown::note_lines(&item.note) {
                lines.extend(self.line(line, usize::from(width)));
            }
        }
        let mut entries = vec![Entry(Selected::Note, 0, own..height(&lines))];
        lines.push(Line::default());
        if let Some(checklist) = &self.view.checklist {
            // The first item takes the section's heading with it.
            let mut start = height(&lines);
            lines.push(self.heading(Icon::Checklist, "Checklist"));
            if checklist.is_empty() {
                let own = height(&lines);
                lines.push(Line::styled("No checklist", dim));
                entries.push(Entry(Selected::Checklist(None), start, own..height(&lines)));
            }
            for (index, line) in checklist.iter().enumerate() {
                let own = height(&lines);
                if index > 0 {
                    start = own;
                }
                let (shown, at) = self.checklist_line(line, usize::from(width));
                if let Some((line, column)) = at {
                    caret = Some((own + line, column));
                }
                lines.extend(shown);
                entries.push(Entry(
                    Selected::Checklist(Some(line.id)),
                    start,
                    own..height(&lines),
                ));
            }
            lines.push(Line::default());
        }
        let mut start = height(&lines);
        lines.push(self.heading(Icon::Comments, "Comments"));
        if item.comments.is_empty() {
            let own = height(&lines);
            lines.push(Line::styled("No comments", dim));
            entries.push(Entry(Selected::Comment(None), start, own..height(&lines)));
        }
        for (index, comment) in item.comments.iter().enumerate() {
            let own = height(&lines);
            if index > 0 {
                start = own;
            }
            let (_, text) = comments::split(comment);
            let stamp = comments::time(comment).map(|time| self.stamp(time));
            if let Some((field, (line, column))) = editing(Selected::Comment(Some(index))) {
                lines.extend(stamp.map(|stamp| Line::styled(stamp, dim)));
                caret = Some((height(&lines) + line, *column));
                lines.extend(field.iter().cloned());
            } else {
                let mut line = Line::default();
                line.spans
                    .extend(stamp.map(|stamp| Span::styled(stamp + " ", dim)));
                line.spans.extend(self.formatted(text, Style::new()));
                lines.push(line);
            }
            entries.push(Entry(
                Selected::Comment(Some(index)),
                start,
                own..height(&lines),
            ));
        }
        Content {
            total: height(&lines),
            lines,
            entries,
            caret,
        }
    }

    /// A section's heading, with its icon, in the headers color.
    fn heading(&self, icon: Icon, name: &str) -> Line<'a> {
        Line::styled(
            format!("{} {name}", icon.glyph(self.icons)),
            Style::new().fg(self.theme.headers).bold(),
        )
    }

    /// The lines of a checklist item in content `width`: its marker, checked or not, then its
    /// text and tags, wrapping after the marker, a checked item's struck through and all dimmed
    /// (FR-CHK-04); while its text is edited, the text as typed, wrapping after the marker, and
    /// the caret's line and column among them.
    fn checklist_line(
        &self,
        line: &ChecklistLine<'a>,
        width: usize,
    ) -> (Vec<Line<'a>>, Option<(usize, usize)>) {
        let item = line.item;
        let dim = if item.completed.is_some() {
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
        let prefix = format!("{} ", marker.glyph(self.icons));
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
        let mut spans = vec![Span::styled(drawable(&item.title), title)];
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
        (
            hanging(Span::styled(prefix, marker_style), &spans, width),
            None,
        )
    }

    /// A line of the note as Markdown shows it in content `width`: a heading bold, the first
    /// level also underlined; a list item after its marker, a bullet for `-`, `*` or `+`,
    /// wrapping after it; a quote dimmed after a bar; code dimmed as typed; text after its
    /// indentation; each with its inline formatting.
    fn line(&self, line: NoteLine<'a>, width: usize) -> Vec<Line<'a>> {
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
                let prefix = format!("{}{marker} ", "  ".repeat(depth));
                return hanging(
                    Span::styled(prefix, dim),
                    &self.formatted(text, Style::new()),
                    width,
                );
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
        vec![shown]
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

    /// A comment's `time` as the panel writes it, shorter than its timestamp: the month's name,
    /// the day and the time, and the year too unless it is today's.
    fn stamp(&self, time: DateTime) -> String {
        let format = if time.year() == self.today.year() {
            "%b %-d %H:%M"
        } else {
            "%b %-d %Y %H:%M"
        };
        time.strftime(format).to_string()
    }

    /// The hint naming the keys of what is selected, or of what is edited, and what each does,
    /// in as many lines of `width` columns as it takes, each key with what it does on one line.
    fn hint(&self, width: u16) -> Vec<Line<'static>> {
        let keys: &[&str] = match (self.view.editing, self.view.selected) {
            (Some(_), Selected::Note) => &["Enter new line", "Esc done"],
            (Some(_), Selected::Checklist(_)) => &["Enter next item", "Esc done"],
            (Some(_), Selected::Comment(_)) => &["Enter done", "Esc cancel"],
            (None, Selected::Note) => &["e edit", "a comment"],
            (None, Selected::Checklist(Some(_))) => {
                &["e edit", "o add", "x check", "d del", "a comment"]
            }
            (None, Selected::Checklist(None)) => &["o add", "a comment"],
            (None, Selected::Comment(Some(_))) => &["e edit", "d del", "a comment"],
            (None, Selected::Comment(None)) => &["a comment"],
        };
        let width_key = if self.full { "\\ hide" } else { "\\ full" };
        let rest: &[&str] = if self.view.editing.is_some() {
            &[]
        } else {
            &[width_key, "Esc close"]
        };
        let mut lines: Vec<String> = Vec::new();
        for key in keys.iter().chain(rest) {
            match lines.last_mut() {
                Some(line) if line.width() + " · ".width() + key.width() <= usize::from(width) => {
                    line.push_str(" · ");
                    line.push_str(key);
                }
                _ => lines.push((*key).to_owned()),
            }
        }
        let style = Style::new().fg(self.theme.dim_text);
        lines
            .into_iter()
            .map(|line| Line::styled(line, style))
            .collect()
    }
}

/// `spans` after `lead`, wrapped between words in `width` columns as a text field wraps its
/// text, each line after the first indented as far as `lead` is wide, so that the text hangs
/// under its own start. A line's spaces at a wrap are left out.
fn hanging<'a>(lead: Span<'a>, spans: &[Span<'a>], width: usize) -> Vec<Line<'a>> {
    let indent = lead.width();
    let text: String = spans.iter().map(|span| span.content.as_ref()).collect();
    let mut lead = Some(lead);
    app::screen_lines(&text, width.saturating_sub(indent))
        .into_iter()
        .map(|line| {
            let shown = text.get(line.clone()).unwrap_or_default().trim_end();
            let line = line.start..line.start + shown.len();
            let first = lead.take().unwrap_or_else(|| Span::raw(" ".repeat(indent)));
            let mut parts = vec![first];
            let mut start = 0;
            for span in spans {
                let end = start + span.content.len();
                let part = line.start.max(start)..line.end.min(end);
                if let Some(part) = text.get(part).filter(|part| !part.is_empty()) {
                    parts.push(Span::styled(part.to_owned(), span.style));
                }
                start = end;
            }
            Line::from(parts)
        })
        .collect()
}

/// `text` as the buffer can draw it: a tab shows as a space, the one column its width counts.
fn drawable(text: &str) -> String {
    text.replace('\t', " ")
}

impl StatefulWidget for AttachmentsPanel<'_> {
    type State = Scrolled;

    /// Draws the panel over `area`, its content from line `scrolled.top`, which
    /// it keeps within the content and on what is selected, scrolling a newly selected one into
    /// view from its start when it is taller than the panel, and while something is edited on the
    /// caret's line.
    fn render(self, area: Rect, buf: &mut Buffer, scrolled: &mut Scrolled) {
        let newly = scrolled.drawn.replace(self.view.selected) != Some(self.view.selected);
        let top = &mut scrolled.top;
        let theme = self.theme;
        Clear.render(area, buf);
        let accent = Style::new().fg(if self.focused {
            theme.accent
        } else {
            theme.rule
        });
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(format!(" {} ", self.view.item.title));
        let inner = frame.inner(area);
        frame.render(area, buf);
        let content = self.content(area);
        let laid_out = self.laid_out(area);
        let page = self.page(area);
        let ((start, end), own) = self.selected_in(&laid_out);
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
        let banded: Vec<Rect> = (own.start.max(*top)..own.end.min(*top + page))
            .filter(|_| self.focused)
            .map(|line| {
                let y = content.y + u16::try_from(line - *top).unwrap_or(u16::MAX);
                Rect::new(inner.x, y, inner.width, 1)
            })
            .collect();
        for &row in &banded {
            buf.set_style(row, Style::new().bg(theme.current_line));
        }
        Paragraph::new(laid_out.lines)
            .wrap(Wrap { trim: false })
            .scroll((u16::try_from(*top).unwrap_or(u16::MAX), 0))
            .render(content, buf);
        for &row in &banded {
            buf.set_string(row.x, row.y, FOCUS_MARKER, Style::new().fg(theme.focus));
            super::band_text(buf, row, theme.band_text);
        }
        let hint = self.hint(content.width);
        if let Ok(lines) = u16::try_from(hint.len())
            && inner.height > lines
        {
            let area = Rect::new(content.x, inner.bottom() - lines, content.width, lines);
            Paragraph::new(hint).render(area, buf);
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

    /// The day the panels are drawn on.
    const TODAY: Date = jiff::civil::date(2026, 10, 14);

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    // FR-ATT-02, FR-ATT-03, FR-ATT-04, FR-CHK-03
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
            (KeyCode::Home, none, PanelKey::Scroll(Scroll::Top)),
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
        for (code, modifiers) in [
            (KeyCode::Char('x'), none),
            (KeyCode::Char(']'), none),
            (KeyCode::Char('['), none),
            (KeyCode::Tab, none),
            (KeyCode::BackTab, KeyModifiers::SHIFT),
        ] {
            assert_eq!(
                panel_input(&press(code, modifiers)),
                None,
                "{code:?} is the keymap's"
            );
        }
        assert_eq!(
            panel_input(&press(KeyCode::Char('g'), none)),
            None,
            "g starts the keymap's sequences, gg among them"
        );
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
        AttachmentsPanel::new(view.clone(), &theme, IconSet::Plain, TODAY).render(
            AttachmentsPanel::area(area),
            &mut buf,
            &mut scrolled,
        );
        let caret = AttachmentsPanel::new(view, &theme, IconSet::Plain, TODAY)
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
        let (buf, top, caret) = draw(note, None, (80, 17), 0);
        assert_eq!((top, caret), (0, None));
        assert!(
            (0..40).all(|x| (0..17).all(|y| buf[(x, y)].symbol() == " ")),
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
                "│ ≡ Note                               │",
                "│▌Plan                                 │",
                "│▌Steps                                │",
                "│▌• buy milk                           │",
                "│▌  • see shop                         │",
                "│▌1. one                               │",
                "│▌│ said                               │",
                "│▌  code                               │",
                "│▌ indented                            │",
                "│                                      │",
                "│ ❞ Comments                           │",
                "│ No comments                          │",
                "│                                      │",
                "│ e edit · a comment · \\ full          │",
                "│ Esc close                            │",
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
        assert_eq!(cell(42, 1).fg, theme.headers, "heading");
        assert_eq!(cell(41, 2).fg, theme.focus, "the focus marker");
    }

    // FR-ATT-03
    #[test]
    fn inline_formats_show_in_their_styles() {
        let (buf, ..) = draw("*it* `c` **b**", None, (40, 8), 0);
        let cell = |x: u16| &buf[(x, 2)];
        assert!(cell(2).modifier.contains(Modifier::ITALIC));
        assert!(cell(5).modifier.contains(Modifier::DIM));
        assert!(cell(7).modifier.contains(Modifier::BOLD));
        assert_eq!(rows(&buf)[2], "│▌it c b                               │");
    }

    // FR-ATT-01
    #[test]
    fn no_note_shows_dimmed() {
        let (buf, ..) = draw("", None, (40, 7), 0);
        assert_eq!(rows(&buf)[2], "│▌No note                              │");
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
        let (buf, top, _) = draw(&note, None, (40, 9), usize::MAX);
        assert_eq!(top, 10, "fourteen lines, four in view");
        assert_eq!(
            rows(&buf)[1..5].to_vec(),
            [
                "│▌line 10                              │",
                "│                                      │",
                "│ ❞ Comments                           │",
                "│ No comments                          │",
            ]
        );
        let long = "word ".repeat(12);
        let (buf, ..) = draw(&long, None, (40, 8), 0);
        assert_eq!(rows(&buf)[2], "│▌word word word word word word word   │");
        assert_eq!(rows(&buf)[3], "│▌word word word word word             │");
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
                "│ ≡ Note                               │",
                "│▌# Plan                               │",
                "│▌**b**                                │",
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
                TODAY,
            )
            .caret(Rect::new(0, 0, 40, 8), 5),
            None,
            "a caret scrolled out of view shows nowhere"
        );
        let (_, _, caret) = draw(&text, Some(&field), (40, 4), 0);
        assert_eq!(caret, None, "a panel too short for its content still draws");
    }

    // FR-ATT-03
    #[test]
    fn a_wrapped_list_item_hangs_under_its_text() {
        let note = "- buy **milk** and bread and butter at the shop\n  1. see the shop on the corner of the high street\n- Pneumonoultramicroscopicsilicovolcanoconiosis\nplain text wraps back to the start of the line";
        let (buf, ..) = draw(note, None, (40, 14), 0);
        assert_eq!(
            rows(&buf)[2..10],
            [
                "│▌• buy milk and bread and butter at   │",
                "│▌  the shop                           │",
                "│▌  1. see the shop on the corner of   │",
                "│▌     the high street                 │",
                "│▌• Pneumonoultramicroscopicsilicovolc │",
                "│▌  anoconiosis                        │",
                "│▌plain text wraps back to the start   │",
                "│▌of the line                          │",
            ]
        );
        assert!(buf[(8, 2)].modifier.contains(Modifier::BOLD), "milk");
        assert!(!buf[(13, 2)].modifier.contains(Modifier::BOLD));
    }

    // FR-CHK-02
    #[test]
    fn a_wrapped_checklist_item_hangs_under_its_text() {
        let task = Item::new(ItemKind::Task, "Pack");
        let mut long = Item::new(
            ItemKind::ChecklistItem,
            "Plates and bowls and cups and glasses",
        );
        long.tags = vec![crate::domain::Tag {
            name: "home".to_owned(),
            value: None,
        }];
        let theme = BuiltIn::default().colors();
        let view = AttachmentsView {
            id: ItemId::new(1),
            item: &task,
            selected: Selected::Note,
            checklist: Some(vec![line(&long, 2)]),
            editing: None,
        };
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        AttachmentsPanel::new(view, &theme, IconSet::Plain, TODAY).render(
            AttachmentsPanel::area(area),
            &mut buf,
            &mut Scrolled::default(),
        );
        assert_eq!(
            rows(&buf)[5..7],
            [
                "│ ☐ Plates and bowls and cups and      │",
                "│   glasses @home                      │",
            ]
        );
        assert_eq!(buf[(11, 6)].fg, theme.tag);
    }

    // FR-ATT-02
    #[test]
    fn a_hint_names_each_key_with_what_it_does_on_one_line_when_wide_enough() {
        let task = Item::new(ItemKind::Task, "Pack");
        let item = Item::new(ItemKind::ChecklistItem, "Plates");
        let theme = BuiltIn::default().colors();
        let view = AttachmentsView {
            id: ItemId::new(1),
            item: &task,
            selected: Selected::Checklist(Some(ItemId::new(2))),
            checklist: Some(vec![line(&item, 2)]),
            editing: None,
        };
        let panel = AttachmentsPanel::new(view, &theme, IconSet::Plain, TODAY);
        let hint = |width| {
            panel
                .hint(width)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            hint(68),
            ["e edit · o add · x check · d del · a comment · \\ full · Esc close"]
        );
        assert_eq!(
            hint(36),
            [
                "e edit · o add · x check · d del",
                "a comment · \\ full · Esc close"
            ],
            "whole keys go to the next line"
        );
        assert_eq!(
            panel.full(true).hint(68)[0].to_string(),
            "e edit · o add · x check · d del · a comment · \\ hide · Esc close",
            "over the whole of the panes it narrows"
        );
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
                "[2025-12-30 18:02] Called",
            ],
        );
        let (buf, top, caret) = draw_item(&item, Selected::Comment(Some(1)), None, (40, 12), 0);
        assert_eq!((top, caret), (0, None));
        assert_eq!(
            rows(&buf),
            [
                "┌ Milk ────────────────────────────────┐",
                "│ ≡ Note                               │",
                "│ n                                    │",
                "│                                      │",
                "│ ❞ Comments                           │",
                "│ Oct 5 09:30 Done                     │",
                "│▌plain by hand                        │",
                "│ Dec 30 2025 18:02 Called             │",
                "│                                      │",
                "│ e edit · d del · a comment · \\ full  │",
                "│ Esc close                            │",
                "└──────────────────────────────────────┘",
            ]
        );
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(2, 5)].fg, theme.dim_text, "the timestamp is dimmed");
        assert!(
            buf[(14, 5)].modifier.contains(Modifier::BOLD),
            "inline Markdown"
        );
        assert!((1..39).all(|x| buf[(x, 6)].bg == theme.current_line));
        assert_eq!(banded(&buf), [6]);
        let (buf, ..) = draw_item(&item, Selected::Comment(Some(0)), None, (40, 12), 0);
        assert_eq!(banded(&buf), [5], "the heading is not the comment's");
        let (buf, ..) = draw_item(&item, Selected::Note, None, (40, 12), 0);
        assert_eq!(banded(&buf), [2], "nor the blank line after the note");
    }

    // FR-ATT-04
    #[test]
    fn a_comment_edited_shows_its_timestamp_above_its_text_as_typed() {
        let item = commented("", &["[2026-10-05 09:30] Called", "by hand"]);
        let field = TextField::new("Called them".to_owned());
        let (buf, _, caret) =
            draw_item(&item, Selected::Comment(Some(0)), Some(&field), (40, 11), 0);
        assert_eq!(
            rows(&buf)[4..8].to_vec(),
            [
                "│ ❞ Comments                           │",
                "│▌Oct 5 09:30                          │",
                "│▌Called them                          │",
                "│ by hand                              │",
            ]
        );
        assert_eq!(rows(&buf)[9], "│ Enter done · Esc cancel              │");
        assert_eq!(caret, Some(Position::new(13, 6)));
        let (buf, _, caret) =
            draw_item(&item, Selected::Comment(Some(1)), Some(&field), (40, 11), 0);
        assert_eq!(
            rows(&buf)[6],
            "│▌Called them                          │",
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
        let (_, top, _) = draw_item(&item, Selected::Comment(Some(9)), None, (40, 9), 0);
        assert_eq!(top, 10, "the last comment is the last line in view");
        let (_, top, _) = draw_item(&item, Selected::Note, None, (40, 9), 10);
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
        let area = Rect::new(0, 0, 40, 9);
        let mut buf = Buffer::empty(area);
        let mut scrolled = Scrolled {
            top: 10,
            drawn: Some(Selected::Comment(Some(0))),
        };
        AttachmentsPanel::new(view(Selected::Note), &theme, IconSet::Plain, TODAY).render(
            AttachmentsPanel::area(area),
            &mut buf,
            &mut scrolled,
        );
        assert_eq!(
            scrolled.top, 0,
            "from the start of a note taller than the panel"
        );
        scrolled.top = 5;
        AttachmentsPanel::new(view(Selected::Note), &theme, IconSet::Plain, TODAY).render(
            AttachmentsPanel::area(area),
            &mut buf,
            &mut scrolled,
        );
        assert_eq!(scrolled.top, 5, "scrolling through it stays");
        let (_, top, _) = draw_item(&item, Selected::Note, None, (40, 9), 20);
        assert_eq!(top, 10, "in view");
        let panel = AttachmentsPanel::new(
            view(Selected::Comment(Some(0))),
            &theme,
            IconSet::Plain,
            TODAY,
        );
        assert_eq!(panel.selected_lines(area), (12, 14));
    }

    /// A checklist line of `item`.
    fn line(item: &Item, id: u64) -> ChecklistLine<'_> {
        ChecklistLine {
            id: ItemId::new(id),
            item,
        }
    }

    /// The panel on a task `Pack` with checklist items `Books`, checked, `Novels` and
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
            checklist: Some(vec![line(&books, 2), line(&novels, 3), line(&plates, 4)]),
            editing,
        };
        let area = Rect::new(0, 0, 40, height);
        let mut buf = Buffer::empty(area);
        let mut scrolled = Scrolled {
            top,
            drawn: Some(selected),
        };
        AttachmentsPanel::new(view.clone(), &theme, IconSet::Plain, TODAY).render(
            AttachmentsPanel::area(area),
            &mut buf,
            &mut scrolled,
        );
        let caret =
            AttachmentsPanel::new(view, &theme, IconSet::Plain, TODAY).caret(area, scrolled.top);
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
                "│ ≡ Note                               │",
                "│ No note                              │",
                "│                                      │",
                "│ ☑ Checklist                          │",
                "│ ✔ Books                              │",
                "│▌☐ Novels                             │",
                "│ ☐ Plates @home(kitchen)              │",
                "│                                      │",
                "│ ❞ Comments                           │",
                "│ No comments                          │",
                "│                                      │",
                "│                                      │",
                "│ e edit · o add · x check · d del     │",
                "│ a comment · \\ full · Esc close       │",
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
        let novels = &buf[(4, 6)];
        assert!(
            !novels
                .modifier
                .intersects(Modifier::DIM | Modifier::CROSSED_OUT),
            "after a checked item"
        );
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
    fn the_first_item_scrolls_into_view_with_the_heading_above_it() {
        let selected = Selected::Checklist(Some(ItemId::new(2)));
        let (buf, top, _) = draw_checklist(selected, None, 8, 0);
        assert_eq!(top, 2, "the heading above it in view");
        assert_eq!(rows(&buf)[2], "│ ☑ Checklist                          │");
        assert_eq!(banded(&buf), [3], "the heading off the band");
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
                "│▌☐ Plates and bowls and cups          │",
                "│ Enter next item · Esc done           │",
            )
        );
        assert_eq!(caret, Some(Position::new(29, 7)));
        let long = TextField::new("Plates and bowls and cups and glasses".to_owned());
        let (buf, _, caret) = draw_checklist(selected, Some(&long), 16, 0);
        assert_eq!(
            rows(&buf)[7..9],
            [
                "│▌☐ Plates and bowls and cups and      │",
                "│▌  glasses                            │",
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
        let area = Rect::new(0, 0, 40, 13);
        let mut buf = Buffer::empty(area);
        AttachmentsPanel::new(view.clone(), &theme, IconSet::Plain, TODAY).render(
            AttachmentsPanel::area(area),
            &mut buf,
            &mut Scrolled::default(),
        );
        assert_eq!(rows(&buf)[5], "│▌No checklist                         │");
        assert_eq!(buf[(2, 5)].bg, theme.current_line);
        assert_eq!(
            AttachmentsPanel::new(view, &theme, IconSet::Plain, TODAY).selected_lines(area),
            (3, 6),
            "its heading and line, running to the empty comments after it"
        );
        assert_eq!(
            banded(&buf),
            [5],
            "its heading and the blank line after it off the band"
        );
    }

    // FR-ATT-04
    #[test]
    fn empty_comments_are_selected_as_an_entry() {
        let task = Item::new(ItemKind::Task, "Pack");
        let theme = BuiltIn::default().colors();
        let view = AttachmentsView {
            id: ItemId::new(1),
            item: &task,
            selected: Selected::Comment(None),
            checklist: Some(Vec::new()),
            editing: None,
        };
        let area = Rect::new(0, 0, 40, 13);
        let mut buf = Buffer::empty(area);
        let panel = AttachmentsPanel::new(view, &theme, IconSet::Plain, TODAY);
        assert_eq!(
            panel.selected_lines(area),
            (6, 8),
            "its heading and line, to the end"
        );
        panel.render(
            AttachmentsPanel::area(area),
            &mut buf,
            &mut Scrolled::default(),
        );
        let rows = rows(&buf);
        assert_eq!(rows[8], "│▌No comments                          │");
        assert_eq!(banded(&buf), [8], "its heading off the band");
        assert_eq!(rows[11], "│ a comment · \\ full · Esc close       │");
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
    fn the_band_covers_only_what_is_selected_without_the_blank_line_after_it() {
        let (buf, ..) = draw_checklist(Selected::Checklist(Some(ItemId::new(4))), None, 16, 0);
        assert_eq!(rows(&buf)[9], "│ ❞ Comments                           │");
        assert_eq!(banded(&buf), [7], "the last checklist item");
        let (buf, ..) = draw("n", None, (40, 10), 0);
        assert_eq!(rows(&buf)[4], "│ ❞ Comments                           │");
        assert_eq!(banded(&buf), [2], "the note");
    }
}
