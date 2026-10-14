//! The container pane: the opened container drawn as FR-VIEW-05 arranges it, in the look of
//! taskpaper-terminal ([ui.md §2](../../../specs/reference/taskpaper-terminal/specs/ui.md#2-blocks)):
//! its title and description, then a row for each of its items, with the focus marker and band
//! on the focused one, wrapping long text and scrolling to keep the focus in view.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, StatefulWidget, Widget, Wrap},
};

use super::{Icon, IconSet, Theme};
use crate::{
    app::{Row, State},
    domain::{Document, Item, ItemId, ItemKind, Node},
};

/// The columns left of the rows that hold the focus marker.
const GUTTER: u16 = 2;

/// The glyph marking the focused row (D-30 of taskpaper-terminal).
const FOCUS_MARKER: &str = "▌";

/// The pane showing a container of a document with its rows. Its state is the first line of the
/// content shown, which it moves only as far as it takes to keep the focused row in view.
pub(super) struct ContainerPane<'a> {
    document: &'a Document,
    container: ItemId,
    rows: Vec<Row>,
    focused: Option<ItemId>,
    theme: &'a Theme,
    icons: IconSet,
}

impl<'a> ContainerPane<'a> {
    /// The pane of the container `state` shows, drawn in `theme` with `icons`.
    pub(super) fn new(state: &'a State, theme: &'a Theme, icons: IconSet) -> Self {
        Self {
            document: state.document(),
            container: state.container(),
            rows: state.rows(),
            focused: state.selection().focused(),
            theme,
            icons,
        }
    }
}

/// A run of lines of the pane: a wrapped paragraph starting `x` columns after the gutter, with
/// what is drawn right after the gutter and at the right of its first line, part of the row
/// numbered `row`, if any.
struct Piece<'a> {
    x: u16,
    text: Paragraph<'a>,
    left: Line<'a>,
    right: Line<'a>,
    row: Option<usize>,
}

impl Piece<'_> {
    /// A blank line between groups.
    fn blank() -> Self {
        Self {
            x: 0,
            text: Paragraph::new(""),
            left: Line::default(),
            right: Line::default(),
            row: None,
        }
    }

    /// The columns its text wraps in, in content `width`.
    fn text_width(&self, width: u16) -> u16 {
        let right = u16::try_from(self.right.width()).unwrap_or(u16::MAX);
        let gap = u16::from(right > 0);
        width.saturating_sub(self.x).saturating_sub(right + gap)
    }

    /// The lines it takes in content `width`: at least one.
    fn height(&self, width: u16) -> usize {
        self.text.line_count(self.text_width(width)).max(1)
    }
}

impl StatefulWidget for ContainerPane<'_> {
    type State = usize;

    fn render(self, area: Rect, buf: &mut Buffer, top: &mut usize) {
        let width = area.width.saturating_sub(GUTTER);
        let pieces = self.pieces();
        let mut lines = Vec::with_capacity(pieces.len());
        let mut y = 0;
        for piece in &pieces {
            lines.push(y);
            y += piece.height(width);
        }
        let focused = self.focused_row();
        let span = focused.map(|row| row_span(&pieces, &lines, width, row));
        // Focusing the first row shows the container's title and description above it, when
        // they fit with the row.
        let height = usize::from(area.height);
        let shown = span.map(|(start, end)| match focused {
            Some(0) if end <= height => (0, end),
            _ => (start, end),
        });
        *top = scrolled(*top, shown, y, height);
        if let Some((start, end)) = span {
            self.band(area, buf, start, end, *top);
        }
        for (piece, &start) in pieces.iter().zip(&lines) {
            draw(piece, start, *top, area, buf);
        }
        if let Some((start, end)) = span {
            self.band_text(area, buf, start, end, *top);
        }
    }
}

/// The first line to show so that the lines `span` of the focused row are in view, moving `top`
/// no further than needed; the focused row's first lines when it is taller than `height`.
fn scrolled(top: usize, span: Option<(usize, usize)>, total: usize, height: usize) -> usize {
    let top = top.min(total.saturating_sub(height));
    match span {
        Some((start, end)) if start < top || end - start > height => start,
        Some((_, end)) if end > top + height => end - height,
        _ => top,
    }
}

/// The content lines from the first to past the last of row `row`.
fn row_span(pieces: &[Piece], lines: &[usize], width: u16, row: usize) -> (usize, usize) {
    let mut span: Option<(usize, usize)> = None;
    for (piece, &start) in pieces.iter().zip(lines) {
        if piece.row == Some(row) {
            let end = start + piece.height(width);
            span = Some(span.map_or((start, end), |(first, _)| (first, end)));
        }
    }
    span.unwrap_or_default()
}

/// Draws `piece`, whose first line is content line `start`, where it falls in `area` when the
/// pane shows from content line `top`.
fn draw(piece: &Piece, start: usize, top: usize, area: Rect, buf: &mut Buffer) {
    let width = area.width.saturating_sub(GUTTER);
    let height = piece.height(width);
    let shown = usize::from(area.height);
    if start + height <= top || start >= top + shown {
        return;
    }
    let skip = top.saturating_sub(start);
    let row_y = |line: usize| u16::try_from(line - top).unwrap_or(u16::MAX) + area.y;
    let first = start.max(top);
    let rows = u16::try_from((start + height).min(top + shown) - first).unwrap_or(u16::MAX);
    let x = area.x + GUTTER + piece.x;
    let text = Rect::new(x, row_y(first), piece.text_width(width), rows);
    piece
        .text
        .clone()
        .scroll((u16::try_from(skip).unwrap_or(u16::MAX), 0))
        .render(text, buf);
    if skip == 0 {
        let left = Rect::new(area.x + GUTTER, row_y(start), piece.x, 1);
        (&piece.left).render(left.intersection(area), buf);
        let right = u16::try_from(piece.right.width()).unwrap_or(u16::MAX);
        let at = Rect::new(area.right().saturating_sub(right), row_y(start), right, 1);
        (&piece.right).render(at.intersection(area), buf);
    }
}

impl<'a> ContainerPane<'a> {
    /// The index of the focused row among the state's rows.
    fn focused_row(&self) -> Option<usize> {
        let focused = self.focused?;
        self.rows.iter().position(|row| row.id() == focused)
    }

    /// The pane's content: the container's title and description, then its rows.
    fn pieces(&self) -> Vec<Piece<'a>> {
        let document = self.document;
        let item = |id: ItemId| document.node(id).and_then(Node::item);
        let mut pieces = Vec::new();
        if let Some(container) = item(self.container) {
            pieces.push(self.title(container, None));
            if !container.description.is_empty() {
                pieces.push(Piece {
                    x: 0,
                    text: wrapped(Text::styled(
                        container.description.as_str(),
                        self.description_style(),
                    )),
                    left: Line::default(),
                    right: Line::default(),
                    row: None,
                });
            }
        }
        let rows = &self.rows;
        let shown = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| Some((index, row, item(row.id())?)));
        for (index, row, shown) in shown {
            let before = index.checked_sub(1).and_then(|before| rows.get(before));
            let opens_group = index == 0
                || match row {
                    Row::Section(_) => true,
                    Row::Project(_) => !matches!(before, Some(Row::Project(_))),
                    Row::Task { .. } => false,
                };
            if opens_group {
                pieces.push(Piece::blank());
            }
            pieces.push(self.title(shown, Some(index)));
            if let Some(first) = shown.description.lines().next() {
                pieces.push(Piece {
                    x: self.title_column(shown),
                    text: wrapped(Text::styled(first, self.description_style())),
                    left: Line::default(),
                    right: Line::default(),
                    row: Some(index),
                });
            }
        }
        pieces
    }

    /// The line naming `item`: its marker or icon, then its title and its tags wrapping under
    /// the title's start, with its attachment icons at the right.
    fn title(&self, item: &'a Item, row: Option<usize>) -> Piece<'a> {
        let mut spans = vec![Span::styled(item.title.as_str(), self.title_style(item))];
        for tag in &item.tags {
            let text = match &tag.value {
                Some(value) => format!(" @{}({value})", tag.name),
                None => format!(" @{}", tag.name),
            };
            spans.push(Span::styled(text, self.tag_style(item)));
        }
        Piece {
            x: self.title_column(item),
            text: wrapped(Text::from(Line::from(spans))),
            left: Line::styled(self.marker(item), self.marker_style(item)),
            right: self.attachments(item),
            row,
        }
    }

    /// The column after the marker and its space, where a row's title starts.
    fn title_column(&self, item: &Item) -> u16 {
        u16::try_from(Span::raw(self.marker(item)).width() + 1).unwrap_or(u16::MAX)
    }

    /// The icons for `item`'s notes and its comments with their count, a space after each.
    fn attachments(&self, item: &Item) -> Line<'a> {
        let style = Style::new()
            .fg(self.theme.headers)
            .add_modifier(Modifier::DIM);
        let mut spans = Vec::new();
        if !item.note.is_empty() {
            spans.push(Span::styled(
                format!("{} ", Icon::Notes.glyph(self.icons)),
                style,
            ));
        }
        if !item.comments.is_empty() {
            let count = item.comments.len();
            spans.push(Span::styled(
                format!("{} {count} ", Icon::Comments.glyph(self.icons)),
                style,
            ));
        }
        Line::from(spans)
    }

    /// The marker of a task or the icon of a container.
    fn marker(&self, item: &Item) -> &'static str {
        let icon = match item.kind {
            ItemKind::Inbox => Icon::Inbox,
            ItemKind::Area => Icon::Area,
            ItemKind::Project => Icon::Project,
            ItemKind::Section => Icon::Section,
            ItemKind::Task | ItemKind::ChecklistItem if item.completed.is_some() => Icon::DoneTask,
            ItemKind::Task | ItemKind::ChecklistItem => Icon::Task,
        };
        icon.glyph(self.icons)
    }

    /// How `item`'s title is drawn: a task in the text color, struck through and faded once
    /// completed; containers bold, each kind in its color, a section also in italics.
    fn title_style(&self, item: &Item) -> Style {
        let theme = self.theme;
        let style = match item.kind {
            ItemKind::Inbox | ItemKind::Project => Style::new().fg(theme.primary).bold(),
            ItemKind::Area => Style::new().fg(theme.headers).bold(),
            ItemKind::Section => Style::new().fg(theme.headers).bold().italic(),
            ItemKind::Task | ItemKind::ChecklistItem => Style::new(),
        };
        if item.completed.is_some() {
            style.fg(theme.done).add_modifier(Modifier::CROSSED_OUT)
        } else {
            style
        }
    }

    /// How `item`'s marker is drawn: in its title's color, faded once completed.
    fn marker_style(&self, item: &Item) -> Style {
        let style = self.title_style(item);
        style.remove_modifier(Modifier::CROSSED_OUT | Modifier::ITALIC)
    }

    /// How `item`'s tags are drawn: dimmed after its title (FR-TAG-03).
    fn tag_style(&self, item: &Item) -> Style {
        let color = if item.completed.is_some() {
            self.theme.done
        } else {
            self.theme.tag
        };
        Style::new().fg(color).add_modifier(Modifier::DIM)
    }

    /// How descriptions are drawn: dimmed (FR-ORG-10).
    fn description_style(&self) -> Style {
        Style::new().fg(self.theme.dim_text)
    }

    /// Paints the current-line band behind the focused row's lines `start..end` and puts the
    /// focus marker in the gutter beside them (FR-UI-02).
    fn band(&self, area: Rect, buf: &mut Buffer, start: usize, end: usize, top: usize) {
        for y in band_rows(area, start, end, top) {
            let line = Rect::new(area.x, y, area.width, 1);
            buf.set_style(line, Style::new().bg(self.theme.current_line));
            buf.set_string(area.x, y, FOCUS_MARKER, Style::new().fg(self.theme.focus));
        }
    }

    /// Gives the text on the band that has no color of its own the band's text color.
    fn band_text(&self, area: Rect, buf: &mut Buffer, start: usize, end: usize, top: usize) {
        for y in band_rows(area, start, end, top) {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y))
                    && cell.fg == Color::Reset
                {
                    cell.set_fg(self.theme.band_text);
                }
            }
        }
    }
}

/// `text` wrapping between words (FR-UI-05).
fn wrapped(text: Text<'_>) -> Paragraph<'_> {
    Paragraph::new(text).wrap(Wrap { trim: true })
}

/// The screen rows of content lines `start..end` shown in `area` from content line `top`.
fn band_rows(area: Rect, start: usize, end: usize, top: usize) -> impl Iterator<Item = u16> {
    let shown = top + usize::from(area.height);
    (start..end.min(shown))
        .filter_map(move |line| u16::try_from(line.checked_sub(top)?).ok())
        .map(move |offset| area.y + offset)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use ratatui::style::Color;

    use super::*;
    use crate::{
        app,
        domain::{Completion, PlacementError, Tag},
        store::demo,
        ui::BuiltIn,
    };

    /// The node of `document` titled `title`, depth first.
    fn find(document: &Document, title: &str) -> ItemId {
        fn walk(document: &Document, ids: &[ItemId], title: &str) -> Option<ItemId> {
            ids.iter().find_map(|&id| {
                if document.node(id).and_then(Node::item)?.title == title {
                    Some(id)
                } else {
                    walk(document, document.children(id), title)
                }
            })
        }
        let roots: Vec<ItemId> = std::iter::once(document.inbox())
            .chain(document.top_level().iter().copied())
            .collect();
        walk(document, &roots, title).expect("the document has the title")
    }

    /// Draws `container` of `document` with the row titled `focused` focused, in an area of
    /// `width` by `height` scrolled to `top`, and returns the buffer.
    fn draw_pane(
        document: &Document,
        container: ItemId,
        focused: Option<&str>,
        (width, height): (u16, u16),
        top: &mut usize,
    ) -> Buffer {
        let theme = BuiltIn::default().colors();
        let pane = ContainerPane {
            document,
            container,
            rows: app::rows(document, container),
            focused: focused.map(|title| find(document, title)),
            theme: &theme,
            icons: IconSet::Plain,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        pane.render(buf.area, &mut buf, top);
        buf
    }

    /// The text of each row of `buf`.
    fn lines(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    fn demo() -> Document {
        demo::document(date(2026, 10, 5))
    }

    /// `rows` padded to `width` columns.
    fn padded(width: usize, rows: &[&str]) -> Vec<String> {
        rows.iter().map(|row| format!("{row:width$}")).collect()
    }

    // FR-VIEW-05, FR-ORG-05, FR-ORG-10, FR-TASK-03, FR-TAG-03, FR-UI-02
    #[test]
    fn a_project_shows_its_title_description_tasks_sections_and_projects() {
        let document = demo();
        let website = find(&document, "Website");
        let buf = draw_pane(
            &document,
            website,
            Some("Write the announcement"),
            (50, 20),
            &mut 0,
        );
        assert_eq!(
            lines(&buf),
            padded(
                50,
                &[
                    "  ○ Website                                 ≡ ❞ 2",
                    "  The new marketing site, built with the design",
                    "  team.",
                    "",
                    "  - Collect logos from partners",
                    "",
                    "  § Launch",
                    "    What has to happen before launch.",
                    "▌ - Write the announcement @writing         ≡ ❞ 1",
                    "▌   Keep it under 300 words.",
                    "  - Book the launch call @meeting",
                    "",
                    "  § Polish",
                    "  - Fix the footer links",
                    "  ✔ Compress the hero image",
                    "",
                    "  ○ Blog",
                    "    A project inside a project.",
                    "",
                    "",
                ]
            )
        );
    }

    // FR-VIEW-05, D-26
    #[test]
    fn an_area_shows_its_child_projects_as_one_row_each() {
        let document = demo();
        let home = find(&document, "Home");
        let buf = draw_pane(&document, home, None, (40, 6), &mut 0);
        assert_eq!(
            lines(&buf),
            padded(
                40,
                &[
                    "  ◆ Home",
                    "",
                    "  - Water the plants",
                    "",
                    "  ○ Move to the new flat",
                    "",
                ]
            )
        );
    }

    // FR-UI-02, FR-TAG-03, FR-ORG-10
    #[test]
    fn rows_are_drawn_in_the_styles_of_their_kind() {
        let document = demo();
        let theme = BuiltIn::default().colors();
        let website = find(&document, "Website");
        let buf = draw_pane(
            &document,
            website,
            Some("Write the announcement"),
            (50, 20),
            &mut 0,
        );
        let at = |x, y| buf[(x, y)].style();
        let title = at(4, 0);
        assert_eq!(
            (title.fg, title.add_modifier),
            (Some(theme.primary), Modifier::BOLD)
        );
        assert_eq!(
            at(2, 1).fg,
            Some(theme.dim_text),
            "the project's description"
        );
        let section = at(4, 6);
        assert_eq!(section.fg, Some(theme.headers));
        assert!(
            section
                .add_modifier
                .contains(Modifier::BOLD | Modifier::ITALIC)
        );
        let focus = at(0, 8);
        assert_eq!(
            (focus.fg, focus.bg),
            (Some(theme.focus), Some(theme.current_line))
        );
        assert_eq!(at(0, 9).fg, Some(theme.focus), "the band's second line");
        assert_eq!(
            (at(4, 8).fg, at(4, 8).bg),
            (Some(theme.band_text), Some(theme.current_line)),
            "a focused task's title"
        );
        assert_eq!(
            at(49, 8).bg,
            Some(theme.current_line),
            "the band spans the pane"
        );
        let tag = at(28, 8);
        assert_eq!(tag.fg, Some(theme.tag));
        assert!(tag.add_modifier.contains(Modifier::DIM));
        assert_eq!(at(4, 9).fg, Some(theme.dim_text), "a task's description");
        let icons = at(44, 8);
        assert_eq!(icons.fg, Some(theme.headers));
        assert_eq!(
            at(4, 10).bg,
            Some(Color::Reset),
            "rows without focus have no band"
        );
        let done = at(4, 14);
        assert_eq!(done.fg, Some(theme.done));
        assert!(done.add_modifier.contains(Modifier::CROSSED_OUT));
        assert_eq!(at(2, 14).fg, Some(theme.done), "a done task's marker");
        assert!(!at(2, 14).add_modifier.contains(Modifier::CROSSED_OUT));
    }

    // FR-UI-05
    #[test]
    fn long_titles_wrap_under_their_start_and_the_band_covers_every_line() {
        let document = demo();
        let inbox = document.inbox();
        let buf = draw_pane(
            &document,
            inbox,
            Some("Call the bank about the card"),
            (20, 9),
            &mut 0,
        );
        assert_eq!(
            lines(&buf),
            padded(
                20,
                &[
                    "  ⊔ Inbox",
                    "  Everything not",
                    "  sorted yet lands",
                    "  here.",
                    "",
                    "▌ - Call the bank",
                    "▌   about the card",
                    "  - Buy coffee beans",
                    "    @errands",
                ]
            )
        );
    }

    // FR-UI-02, FR-UI-06
    #[test]
    fn the_pane_scrolls_just_enough_to_keep_the_focused_row_in_view() {
        let document = demo();
        let website = find(&document, "Website");
        let mut top = 0;
        let buf = draw_pane(
            &document,
            website,
            Some("Fix the footer links"),
            (50, 5),
            &mut top,
        );
        assert_eq!(top, 9);
        assert_eq!(lines(&buf)[4].trim_end(), "▌ - Fix the footer links");
        draw_pane(
            &document,
            website,
            Some("Compress the hero image"),
            (50, 5),
            &mut top,
        );
        assert_eq!(top, 10, "one row further down scrolls one line");
        draw_pane(
            &document,
            website,
            Some("Fix the footer links"),
            (50, 5),
            &mut top,
        );
        assert_eq!(top, 10, "a row in view does not scroll");
        let buf = draw_pane(&document, website, Some("Launch"), (50, 5), &mut top);
        assert_eq!((top, lines(&buf)[0].trim_end()), (6, "▌ § Launch"));
        draw_pane(
            &document,
            website,
            Some("Collect logos from partners"),
            (50, 5),
            &mut top,
        );
        assert_eq!(top, 0, "the first row shows the title above it");
        let buf = draw_pane(
            &document,
            website,
            Some("Write the announcement"),
            (50, 1),
            &mut top,
        );
        assert_eq!(top, 8, "a row taller than the pane shows its first line");
        assert_eq!(
            lines(&buf)[0].trim_end(),
            "▌ - Write the announcement @writing         ≡ ❞ 1"
        );
    }

    #[test]
    fn scrolling_keeps_the_top_within_the_content() {
        assert_eq!(scrolled(30, None, 10, 4), 6);
        assert_eq!(scrolled(30, None, 3, 4), 0);
        assert_eq!(scrolled(2, Some((0, 1)), 10, 4), 0);
        assert_eq!(scrolled(2, Some((7, 9)), 10, 4), 5);
        assert_eq!(scrolled(2, Some((3, 5)), 10, 4), 2);
        assert_eq!(scrolled(0, Some((3, 9)), 10, 4), 3);
    }

    // FR-UI-06
    #[test]
    fn a_tiny_pane_draws_what_fits() {
        let document = demo();
        let website = find(&document, "Website");
        for size in [(0, 0), (1, 1), (2, 3), (3, 2), (50, 0)] {
            draw_pane(&document, website, Some("Polish"), size, &mut 0);
        }
        let buf = draw_pane(&document, website, Some("Polish"), (4, 1), &mut 0);
        assert_eq!(lines(&buf), ["▌ § "]);
    }

    #[test]
    fn a_container_without_rows_shows_its_title_alone() {
        let document = Document::new();
        let buf = draw_pane(&document, document.inbox(), None, (12, 2), &mut 0);
        assert_eq!(lines(&buf), padded(12, &["  ⊔ Inbox", ""]));
        let missing = draw_pane(&document, ItemId::new(9), None, (12, 1), &mut 0);
        assert_eq!(lines(&missing), padded(12, &[""]));
    }

    // FR-TASK-03, FR-TAG-03
    #[test]
    fn a_row_shows_tag_values_its_notes_and_its_comment_count() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let mut task = Item::new(ItemKind::Task, "Plan");
        task.tags = vec![
            Tag {
                name: "where".to_owned(),
                value: Some("home".to_owned()),
            },
            Tag {
                name: "soon".to_owned(),
                value: None,
            },
        ];
        task.note = "n".to_owned();
        let mut talked = Item::new(ItemKind::Task, "Talk");
        talked.comments = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        talked.completed = Some(Completion::Undated);
        talked.tags = vec![Tag {
            name: "x".to_owned(),
            value: None,
        }];
        document.push(Some(inbox), Node::Item(task))?;
        document.push(Some(inbox), Node::Item(talked))?;
        let buf = draw_pane(&document, inbox, None, (30, 4), &mut 0);
        assert_eq!(
            lines(&buf),
            padded(
                30,
                &[
                    "  ⊔ Inbox",
                    "",
                    "  - Plan @where(home) @soon ≡",
                    "  ✔ Talk @x               ❞ 3"
                ]
            )
        );
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(9, 2)].style().fg, Some(theme.tag));
        assert_eq!(
            buf[(9, 3)].style().fg,
            Some(theme.done),
            "a done task's tags"
        );
        Ok(())
    }
}
