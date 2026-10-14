//! The container pane: the opened container drawn as FR-VIEW-05 arranges it, in the look of
//! taskpaper-terminal ([ui.md §2](../../../specs/reference/taskpaper-terminal/specs/ui.md#2-blocks)):
//! its title and description, then a row for each of its items, with the focus marker and band
//! on the focused one and the marker dimmed on the picked ones, wrapping long text and scrolling
//! to keep the focus in view, and the title being edited, if any, as a text field in its row, or
//! in the heading for the container's own; a description being edited shows whole where it shows.

use jiff::civil::Date;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, StatefulWidget, Widget, Wrap},
};

use super::{Icon, IconSet, Theme, field};
use crate::{
    app::{
        self, PaneView, Part, Row, Selection, State, TextField,
        view::{self, Group, Heading, Locating, Shown, View},
    },
    domain::{Document, Item, ItemId, ItemKind, Node, Tag, dates},
};

/// The columns left of the rows that hold the focus marker.
const GUTTER: u16 = 2;

/// The glyph marking the focused row (D-30 of taskpaper-terminal).
const FOCUS_MARKER: &str = "▌";

/// The pane showing a container of a document with its rows, or a view with its groups. Its state
/// is the first line of the content shown, which it moves only as far as it takes to keep the
/// focused row in view.
pub(super) struct ContainerPane<'a> {
    document: &'a Document,
    shown: Shown,
    rows: Vec<Row>,
    groups: Vec<Group>,
    selection: &'a Selection,
    editing: Option<(ItemId, Part, &'a TextField)>,
    /// Whether the pane has the keys, which its focus marker and band show.
    keys: bool,
    /// The local date, which date chips are named relative to.
    today: Date,
    theme: &'a Theme,
    icons: IconSet,
}

impl<'a> ContainerPane<'a> {
    /// `pane` of `state`, showing its container or view on `today`, drawn in `theme` with
    /// `icons`; when it is the `current` pane, the one with the keys, with the title edited and
    /// the focus marked unless the sidebar has the keys.
    pub(super) fn new(
        state: &'a State,
        pane: PaneView<'a>,
        current: bool,
        today: Date,
        theme: &'a Theme,
        icons: IconSet,
    ) -> Self {
        let keys = current && !state.sidebar_has_keys();
        Self {
            document: state.document(),
            shown: pane.shown(),
            rows: pane.rows(),
            groups: pane.groups(),
            selection: pane.selection(),
            editing: state.editing().filter(|_| keys),
            keys,
            today,
            theme,
            icons,
        }
    }
}

/// A run of lines of the pane: a wrapped paragraph starting `x` columns after the gutter, with
/// what is drawn right after the gutter and at the right of its first line, part of the row
/// numbered `row`, if any. A text field is laid out in lines of its own instead, with its caret
/// on line `caret.0`, `caret.1` columns into it.
struct Piece<'a> {
    x: u16,
    text: Paragraph<'a>,
    left: Line<'a>,
    right: Line<'a>,
    row: Option<usize>,
    caret: Option<(usize, usize)>,
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
            caret: None,
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

    /// The columns a text field is scrolled sideways in content `width`: none while its caret
    /// fits, else as many as keep the caret in the last column, and never past the caret.
    fn shift(&self, width: u16) -> usize {
        let columns = usize::from(self.text_width(width));
        self.caret.map_or(0, |(_, caret)| {
            (caret + 1).saturating_sub(columns).min(caret)
        })
    }
}

impl StatefulWidget for ContainerPane<'_> {
    type State = usize;

    fn render(self, area: Rect, buf: &mut Buffer, top: &mut usize) {
        let width = area.width.saturating_sub(GUTTER);
        let (pieces, lines, y) = self.laid_out(width);
        let focused = self.focused_row();
        let span = focused.map(|row| row_span(&pieces, &lines, width, row));
        // Focusing the first row shows the container's title and description above it, when
        // they fit with the row, as does editing that description. A caret stays in view.
        let height = usize::from(area.height);
        let caret = pieces
            .iter()
            .zip(&lines)
            .find_map(|(piece, start)| Some(start + piece.caret?.0));
        let in_row = |line| span.is_some_and(|(start, end)| (start..end).contains(&line));
        let shown = match caret {
            Some(line) if !in_row(line) => Some((if line < height { 0 } else { line }, line + 1)),
            _ => span.map(|(start, end)| match focused {
                Some(0) if end <= height => (0, end),
                _ => (start, end),
            }),
        };
        *top = scrolled(*top, shown, y, height);
        *top = scrolled(*top, caret.map(|line| (line, line + 1)), y, height);
        let band = span.filter(|_| self.keys);
        if let Some((start, end)) = band {
            self.band(area, buf, start, end, *top);
        }
        for (index, row) in self.rows.iter().enumerate() {
            if self.keys && Some(index) != focused && self.selection.is_picked(row.id()) {
                let (start, end) = row_span(&pieces, &lines, width, index);
                self.picked(area, buf, start, end, *top);
            }
        }
        for (piece, &start) in pieces.iter().zip(&lines) {
            draw(piece, start, *top, area, buf);
        }
        if let Some((start, end)) = band {
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
    let shift = u16::try_from(piece.shift(width)).unwrap_or(u16::MAX);
    piece
        .text
        .clone()
        .scroll((u16::try_from(skip).unwrap_or(u16::MAX), shift))
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
    /// Where the terminal's cursor goes when the pane is drawn over `area` from content line
    /// `top`: at the caret of the title being edited, when it is in view and the pane has room
    /// for it.
    pub(super) fn caret(&self, area: Rect, top: usize) -> Option<Position> {
        let width = area.width.saturating_sub(GUTTER);
        let (pieces, lines, _) = self.laid_out(width);
        let (piece, &start) = pieces
            .iter()
            .zip(&lines)
            .find(|(piece, _)| piece.caret.is_some())?;
        let (line, column) = piece.caret?;
        let row = (start + line)
            .checked_sub(top)
            .filter(|&row| row < usize::from(area.height))?;
        let column = column - piece.shift(width);
        let x = usize::from(area.x + GUTTER + piece.x) + column;
        let x = u16::try_from(x).ok().filter(|&x| x < area.right())?;
        Some(Position::new(x, area.y + u16::try_from(row).ok()?))
    }

    /// The pane's content in content `width`: its pieces, the content line each starts at and
    /// the lines they take in all.
    fn laid_out(&self, width: u16) -> (Vec<Piece<'a>>, Vec<usize>, usize) {
        let pieces = self.pieces(width);
        let mut lines = Vec::with_capacity(pieces.len());
        let mut y = 0;
        for piece in &pieces {
            lines.push(y);
            y += piece.height(width);
        }
        (pieces, lines, y)
    }

    /// The index of the focused row among the state's rows.
    fn focused_row(&self) -> Option<usize> {
        let focused = self.selection.focused()?;
        self.rows.iter().position(|row| row.id() == focused)
    }

    /// The columns the description being edited wraps in when the pane is drawn over `area`, if
    /// one is: the container's, under its title, or a row's, under the row's title.
    pub(super) fn description_columns(&self, area: Rect) -> Option<usize> {
        let (id, ..) = self
            .editing
            .filter(|&(_, part, _)| part == Part::Description)?;
        let item = self.document.node(id).and_then(Node::item)?;
        let x = if self.shown == Shown::Container(id) {
            0
        } else {
            self.title_column(item)
        };
        Some(field_columns(area.width.saturating_sub(GUTTER), x))
    }

    /// The pane's content in content `width`: the container's title and description, then its
    /// rows; or the view's name, then each of its groups after a blank line, under its heading.
    fn pieces(&self, width: u16) -> Vec<Piece<'a>> {
        let mut pieces = Vec::new();
        match self.shown {
            Shown::Container(id) => {
                if let Some(container) = self.item(id) {
                    pieces.push(match self.edited(id, Part::Title) {
                        Some(field) => self.field(container, field, None),
                        None => self.title(container, None),
                    });
                    if let Some(field) = self.edited(id, Part::Description) {
                        pieces.push(self.description_field(field, 0, width, None));
                    } else if !container.description.is_empty() {
                        pieces.push(Piece {
                            x: 0,
                            text: wrapped(Text::styled(
                                container.description.as_str(),
                                self.description_style(),
                            )),
                            left: Line::default(),
                            right: Line::default(),
                            row: None,
                            caret: None,
                        });
                    }
                }
                let rows = &self.rows;
                let shown = rows
                    .iter()
                    .enumerate()
                    .filter_map(|(index, row)| Some((index, row, self.item(row.id())?)));
                for (index, row, item) in shown {
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
                    self.row_pieces(index, *row, item, width, &mut pieces);
                }
            }
            Shown::View(view) => {
                pieces.push(self.view_title(view));
                let mut indexes = 0..;
                for group in &self.groups {
                    pieces.push(Piece::blank());
                    pieces.extend(group.heading.and_then(|heading| self.heading(heading)));
                    let shown = group
                        .rows
                        .iter()
                        .zip(&mut indexes)
                        .filter_map(|(row, index)| Some((index, row, self.item(row.id())?)));
                    for (index, row, item) in shown {
                        self.row_pieces(index, *row, item, width, &mut pieces);
                    }
                }
            }
        }
        pieces
    }

    /// Where item `id` is, when the view shown says so of its rows, and how: the containers
    /// holding it from the top level down, joined by ` › ` (FR-VIEW-03), or under the title by
    /// ` / `, and by ` › ` before a section, as `Work / Website › Launch` (D-51); nothing for an
    /// item at the top level, or in a container's rows.
    fn location(&self, id: ItemId) -> Option<(Locating, String)> {
        let Shown::View(view) = self.shown else {
            return None;
        };
        let locating = view.locates()?;
        let mut text = String::new();
        let containers = view::location(self.document, id)
            .into_iter()
            .filter_map(|id| self.item(id));
        for container in containers {
            if !text.is_empty() {
                text.push_str(match (locating, container.kind) {
                    (Locating::Under, ItemKind::Section) | (Locating::After, _) => " › ",
                    (Locating::Under, _) => " / ",
                });
            }
            text.push_str(&container.title);
        }
        (!text.is_empty()).then_some((locating, text))
    }

    /// The item numbered `id`, if it is one.
    fn item(&self, id: ItemId) -> Option<&'a Item> {
        self.document.node(id).and_then(Node::item)
    }

    /// The field editing `part` of item `id`, if it is edited.
    fn edited(&self, id: ItemId, part: Part) -> Option<&'a TextField> {
        match self.editing {
            Some((edited, edited_part, field)) if edited == id && edited_part == part => {
                Some(field)
            }
            _ => None,
        }
    }

    /// Adds to `pieces` the lines of `row`, numbered `index`, showing `shown`, in content
    /// `width`: its title, or the field editing it, with a task's checklist progress at the right,
    /// then, when the view says where it is under the title, that line, and otherwise the first
    /// line of its description; the field editing its description shows either way.
    fn row_pieces(
        &self,
        index: usize,
        row: Row,
        shown: &'a Item,
        width: u16,
        pieces: &mut Vec<Piece<'a>>,
    ) {
        let location = self.location(row.id());
        let after = match &location {
            Some((Locating::After, text)) => {
                Some(Span::styled(format!(" · {text}"), self.description_style()))
            }
            _ => None,
        };
        let mut title = match self.edited(row.id(), Part::Title) {
            Some(field) => self.field(shown, field, Some(index)),
            None => self.title_then(shown, Some(index), after),
        };
        if let Some((checked, total)) = app::progress(self.document, row.id()) {
            let style = Style::new()
                .fg(self.theme.headers)
                .add_modifier(Modifier::DIM);
            let glyph = Icon::Checklist.glyph(self.icons);
            let progress = Span::styled(format!("{glyph} {checked}/{total} "), style);
            title.right.spans.insert(0, progress);
        }
        pieces.push(title);
        let x = self.title_column(shown);
        // Where a view says under the title where its rows are, the second line is kept for that,
        // so its rows show no description and none takes three lines (owner, D-51).
        let under =
            matches!(self.shown, Shown::View(view) if view.locates() == Some(Locating::Under));
        if let Some((Locating::Under, text)) = location {
            pieces.push(Piece {
                x,
                text: wrapped(Text::styled(text, self.description_style())),
                left: Line::default(),
                right: Line::default(),
                row: Some(index),
                caret: None,
            });
        }
        if let Some(field) = self.edited(row.id(), Part::Description) {
            pieces.push(self.description_field(field, x, width, Some(index)));
        } else if !under && let Some(first) = shown.description.lines().next() {
            pieces.push(Piece {
                x,
                text: wrapped(Text::styled(first, self.description_style())),
                left: Line::default(),
                right: Line::default(),
                row: Some(index),
                caret: None,
            });
        }
    }

    /// The line naming `view` at the top of the pane: its icon, then its name, in the Inbox's
    /// style.
    fn view_title(&self, view: View) -> Piece<'a> {
        let style = Style::new().fg(self.theme.primary).bold();
        let icon = super::view_icon(view).glyph(self.icons);
        Piece {
            x: u16::try_from(Span::raw(icon).width() + 1).unwrap_or(u16::MAX),
            text: wrapped(Text::styled(view.title(), style)),
            left: Line::styled(icon, style),
            right: Line::default(),
            row: None,
            caret: None,
        }
    }

    /// The line naming what a view's group is listed under: an area or project as its title
    /// line without chips, or a period of days or the day of completion in bold; none for an item
    /// that is not one.
    fn heading(&self, heading: Heading) -> Option<Piece<'a>> {
        match heading {
            Heading::Item(id) => self.item(id).map(|item| Piece {
                right: Line::default(),
                ..self.title(item, None)
            }),
            Heading::Period(period) => Some(Piece {
                text: wrapped(Text::styled(period.heading(self.today), Modifier::BOLD)),
                ..Piece::blank()
            }),
            Heading::Completed(day) => Some(Piece {
                text: wrapped(Text::styled(
                    day.map_or_else(
                        || "No date".to_owned(),
                        |day| dates::day_heading(day, self.today),
                    ),
                    Modifier::BOLD,
                )),
                ..Piece::blank()
            }),
        }
    }

    /// The line naming `item`: its marker or icon, then its title and its tags wrapping under
    /// the title's start, with its attachment icons and date chips at the right.
    fn title(&self, item: &'a Item, row: Option<usize>) -> Piece<'a> {
        self.title_then(item, row, None)
    }

    /// The line naming `item` as [`title`](Self::title) draws it, with `after` following its
    /// tags.
    fn title_then(&self, item: &'a Item, row: Option<usize>, after: Option<Span<'a>>) -> Piece<'a> {
        let mut spans = vec![Span::styled(item.title.as_str(), self.title_style(item))];
        for tag in &item.tags {
            spans.push(Span::styled(tag_text(tag), self.tag_style(item)));
        }
        spans.extend(after);
        Piece {
            x: self.title_column(item),
            text: wrapped(Text::from(Line::from(spans))),
            left: Line::styled(self.marker(item), self.marker_style(item)),
            right: Line::from([self.attachments(item).spans, self.chips(item)].concat()),
            row,
            caret: None,
        }
    }

    /// The line of `item`, numbered `row` if it is one, while `field` edits its title: its marker,
    /// then the text edited, without tags or date chips, the selected part in the selection
    /// colors, on one line that scrolls sideways to keep the caret in view.
    fn field(&self, item: &'a Item, field: &'a TextField, row: Option<usize>) -> Piece<'a> {
        let text = field.text();
        let caret = text.get(..field.caret()).unwrap_or_default();
        Piece {
            caret: Some((0, Span::raw(caret).width())),
            text: Paragraph::new(field::selected(field, 0..text.len(), self.theme)),
            right: self.attachments(item),
            ..self.title(item, row)
        }
    }

    /// The lines of `field` while it edits a description `x` columns after the gutter, wrapped
    /// as the field wraps them in content `width`, part of the row numbered `row`, if any.
    fn description_field(
        &self,
        field: &'a TextField,
        x: u16,
        width: u16,
        row: Option<usize>,
    ) -> Piece<'a> {
        let (lines, caret) = field::wrapped(field, field_columns(width, x), self.theme);
        Piece {
            x,
            text: Paragraph::new(lines),
            left: Line::default(),
            right: Line::default(),
            row,
            caret: Some(caret),
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

    /// The chips of `item`'s start and due dates, each its icon and the date named relative to
    /// today, a space after each: start in the start color and due in the due color, or, when it
    /// is overdue, with the overdue icon, bold and reversed in the overdue color; all dimmed once
    /// it is completed (FR-DATE-02, FR-DATE-04).
    fn chips(&self, item: &Item) -> Vec<Span<'a>> {
        let theme = self.theme;
        let chip = |icon: Icon, date: Date, style: Style| {
            let style = if item.completed.is_some() {
                style.add_modifier(Modifier::DIM)
            } else {
                style
            };
            let name = dates::relative(date, self.today);
            [
                Span::styled(format!("{} {name}", icon.glyph(self.icons)), style),
                Span::raw(" "),
            ]
        };
        let start = item
            .start
            .map(|date| chip(Icon::Start, date, Style::new().fg(theme.start)));
        let due = item.due.map(|date| {
            if item.overdue(self.today) {
                let style = Style::new().fg(theme.overdue).bold().reversed();
                chip(Icon::Overdue, date, style)
            } else {
                chip(Icon::Due, date, Style::new().fg(theme.due))
            }
        });
        start.into_iter().chain(due).flatten().collect()
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
            ItemKind::Inbox | ItemKind::Project | ItemKind::Area => {
                Style::new().fg(theme.kind(item.kind)).bold()
            }
            ItemKind::Section => Style::new().fg(theme.kind(item.kind)).bold().italic(),
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

    /// Puts the focus marker, dimmed, in the gutter beside a picked row's lines `start..end`
    /// (FR-OPS-01).
    fn picked(&self, area: Rect, buf: &mut Buffer, start: usize, end: usize, top: usize) {
        let style = Style::new()
            .fg(self.theme.focus)
            .add_modifier(Modifier::DIM);
        for y in band_rows(area, start, end, top) {
            buf.set_string(area.x, y, FOCUS_MARKER, style);
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

/// The columns a text field `x` columns after the gutter wraps in, in content `width`: all it
/// has but the last, kept for the caret at the end of a line.
fn field_columns(width: u16, x: u16) -> usize {
    usize::from(width.saturating_sub(x)).saturating_sub(1)
}

/// `text` wrapping between words (FR-UI-05).
fn wrapped(text: Text<'_>) -> Paragraph<'_> {
    Paragraph::new(text).wrap(Wrap { trim: true })
}

/// `tag` as it follows a title: a space, then `@name` or `@name(value)`.
pub(super) fn tag_text(tag: &Tag) -> String {
    match &tag.value {
        Some(value) => format!(" @{}({value})", tag.name),
        None => format!(" @{}", tag.name),
    }
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

    const TODAY: Date = date(2026, 10, 5);

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
        let mut selection = Selection::default();
        selection.focus(focused.map(|title| find(document, title)));
        let pane = ContainerPane {
            document,
            shown: Shown::Container(container),
            groups: Vec::new(),
            rows: app::rows(document, container, true),
            selection: &selection,
            editing: None,
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        pane.render(buf.area, &mut buf, top);
        buf
    }

    /// Draws the Inbox of the demo document, `width` by 10, while `field` edits the title of
    /// its task titled `title`, and returns the buffer and where the caret is.
    fn draw_editing(title: &str, field: &TextField, width: u16) -> (Buffer, Option<Position>) {
        let document = demo();
        let theme = BuiltIn::default().colors();
        let id = find(&document, title);
        let mut selection = Selection::default();
        selection.focus(Some(id));
        let pane = ContainerPane {
            document: &document,
            shown: Shown::Container(document.inbox()),
            groups: Vec::new(),
            rows: app::rows(&document, document.inbox(), false),
            selection: &selection,
            editing: Some((id, Part::Title, field)),
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, width, 10));
        let caret = pane.caret(buf.area, 0);
        pane.render(buf.area, &mut buf, &mut 0);
        (buf, caret)
    }

    // FR-ORG-06, FR-EDIT-02
    #[test]
    fn the_title_of_the_container_shown_is_edited_in_its_heading() {
        let document = demo();
        let theme = BuiltIn::default().colors();
        let field = TextField::new("Tray".to_owned());
        let mut selection = Selection::default();
        selection.focus(document.children(document.inbox()).first().copied());
        let pane = ContainerPane {
            document: &document,
            shown: Shown::Container(document.inbox()),
            groups: Vec::new(),
            rows: app::rows(&document, document.inbox(), false),
            selection: &selection,
            editing: Some((document.inbox(), Part::Title, &field)),
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 4));
        pane.render(buf.area, &mut buf, &mut 0);
        assert_eq!(
            lines(&buf).first().map(String::as_str),
            Some("  ⊔ Tray            ")
        );
    }

    /// Draws the Inbox of the demo document, `width` by `height`, while `field` edits the
    /// description of its item titled `title`, and returns the buffer, where the caret is and the
    /// columns the description wraps in.
    fn draw_description(
        title: &str,
        field: &TextField,
        (width, height): (u16, u16),
    ) -> (Buffer, Option<Position>, Option<usize>) {
        let document = demo();
        let theme = BuiltIn::default().colors();
        let id = find(&document, title);
        let mut selection = Selection::default();
        selection.focus(document.children(document.inbox()).first().copied());
        let pane = || ContainerPane {
            document: &document,
            shown: Shown::Container(document.inbox()),
            groups: Vec::new(),
            rows: app::rows(&document, document.inbox(), false),
            selection: &selection,
            editing: Some((id, Part::Description, field)),
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        let area = buf.area;
        let mut top = 0;
        pane().render(area, &mut buf, &mut top);
        (
            buf,
            pane().caret(area, top),
            pane().description_columns(area),
        )
    }

    // FR-ORG-10, FR-EDIT-02, FR-EDIT-04
    #[test]
    fn the_description_edited_shows_whole_wrapped_as_the_field_wraps_it() {
        let field = TextField::multi_line("one two three\nfour").edited(app::TextEdit::Move {
            motion: app::Motion::To(8),
            select: true,
        });
        let (buf, caret, columns) = draw_description("Inbox", &field, (12, 8));
        assert_eq!(
            lines(&buf)[..5],
            [
                "  ⊔ Inbox   ",
                "  one two   ",
                "  three     ",
                "  four      ",
                "            ",
            ],
            "the container's description under its title, the last column kept for the caret"
        );
        assert_eq!(columns, Some(9));
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(2, 2)].bg, theme.selection, "selected from the caret");
        assert_eq!(
            buf[(5, 3)].bg,
            theme.selection,
            "across the line break to the end"
        );
        assert_eq!(buf[(8, 1)].bg, Color::Reset, "not before the caret");
        assert_eq!(caret, Some(Position::new(2, 2)));
        let (buf, caret, columns) =
            draw_description("Call the bank about the card", &field, (12, 4));
        assert_eq!(
            lines(&buf),
            [
                "▌   r       ",
                "▌   d       ",
                "▌   one two ",
                "▌   three   ",
            ],
            "a row's description under its title, scrolled to keep the caret in view"
        );
        assert_eq!(columns, Some(7));
        assert_eq!(caret, Some(Position::new(4, 3)));
        let spaced = TextField::multi_line("abcdefghi  \nfour").edited(app::TextEdit::Move {
            motion: app::Motion::To(11),
            select: false,
        });
        let (buf, caret, _) = draw_description("Inbox", &spaced, (12, 8));
        assert_eq!(
            lines(&buf)[1..3],
            ["  abcdefghi ", "  four      "],
            "spaces past the end of a line scroll nothing sideways"
        );
        assert_eq!(caret, Some(Position::new(11, 1)));
        let long = TextField::multi_line("a\nb\nc\nd");
        let (buf, caret, _) = draw_description("Inbox", &long, (12, 3));
        assert_eq!(
            lines(&buf),
            ["  b         ", "  c         ", "  d         "],
            "a caret below the pane's height scrolls the title away"
        );
        assert_eq!(caret, Some(Position::new(3, 2)));
    }

    // FR-VIEW-06, D-51
    #[test]
    fn today_lists_its_rows_without_headings_each_saying_where_it_is_under_its_title() {
        let mut state = app::State::new(demo(), app::Opened::Demo, TODAY);
        let today = app::Command::Go(app::Go::View(View::Today));
        app::update(&mut state, app::Message::Command(today), TODAY);
        let theme = BuiltIn::default().colors();
        let pane = ContainerPane::new(
            &state,
            state.pane_view(),
            true,
            TODAY,
            &theme,
            IconSet::Plain,
        );
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 9));
        pane.render(buf.area, &mut buf, &mut 0);
        assert_eq!(
            lines(&buf)
                .iter()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>(),
            [
                "  ☼ Today",
                "",
                "▌ - Answer the recruiter's  ! yesterday",
                "▌   email",
                "▌   Work",
                "  - Write the       ☑ 1/5 ≡ ❞ 1 ↦ today",
                "    announcement",
                "    @writing",
                "    Work / Website › Launch",
            ]
        );
        assert_eq!(buf[(4, 4)].fg, theme.dim_text, "where it is, dimmed");
    }

    // FR-VIEW-06, D-51
    #[test]
    fn today_shows_a_description_only_while_it_is_edited() {
        let mut state = app::State::new(demo(), app::Opened::Demo, TODAY);
        let theme = BuiltIn::default().colors();
        let draw = |state: &mut app::State, message| {
            app::update(state, message, TODAY);
            let pane = ContainerPane::new(
                state,
                state.pane_view(),
                true,
                TODAY,
                &theme,
                IconSet::Plain,
            );
            let mut buf = Buffer::empty(Rect::new(0, 0, 40, 6));
            pane.render(buf.area, &mut buf, &mut 0);
            lines(&buf)[2..]
                .iter()
                .map(|line| line.trim_end().to_owned())
                .collect::<Vec<_>>()
        };
        let today = app::Command::Go(app::Go::View(View::Today));
        draw(&mut state, app::Message::Command(today));
        let describe = app::Command::EditDescription(app::Whose::Focused);
        draw(&mut state, app::Message::Command(describe));
        let typed = app::Message::Edit(app::EditInput::Text(app::TextEdit::Insert('N')));
        assert_eq!(
            draw(&mut state, typed),
            [
                "▌ - Answer the recruiter's  ! yesterday",
                "▌   email",
                "▌   Work",
                "▌   N",
            ]
        );
        assert_eq!(
            draw(&mut state, app::Message::Edit(app::EditInput::Done)),
            [
                "▌ - Answer the recruiter's  ! yesterday",
                "▌   email",
                "▌   Work",
                "  - Write the       ☑ 1/5 ≡ ❞ 1 ↦ today",
            ],
            "the row says where it is instead"
        );
    }

    // FR-VIEW-06, D-51
    #[test]
    fn today_shows_no_description_of_a_top_level_project_either() -> Result<(), PlacementError> {
        let mut document = demo();
        let mut garden = Item::new(ItemKind::Project, "Garden");
        garden.start = Some(TODAY);
        garden.description = "Plans for spring".to_owned();
        document.push(None, Node::Item(garden))?;
        let mut state = app::State::new(document, app::Opened::Demo, TODAY);
        let today = app::Command::Go(app::Go::View(View::Today));
        app::update(&mut state, app::Message::Command(today), TODAY);
        let theme = BuiltIn::default().colors();
        let pane = ContainerPane::new(
            &state,
            state.pane_view(),
            true,
            TODAY,
            &theme,
            IconSet::Plain,
        );
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 16));
        pane.render(buf.area, &mut buf, &mut 0);
        let lines = lines(&buf);
        assert!(
            lines.iter().any(|line| line.contains("Garden")),
            "{lines:#?}"
        );
        assert!(
            !lines.iter().any(|line| line.contains("Plans")),
            "{lines:#?}"
        );
        Ok(())
    }

    // FR-VIEW-04
    #[test]
    fn the_trash_shows_where_each_item_was_after_it() {
        let mut state = app::State::new(demo(), app::Opened::Demo, TODAY);
        let trash = app::Command::Go(app::Go::View(View::Trash));
        app::update(&mut state, app::Message::Command(trash), TODAY);
        let theme = BuiltIn::default().colors();
        let pane = ContainerPane::new(
            &state,
            state.pane_view(),
            true,
            TODAY,
            &theme,
            IconSet::Plain,
        );
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 6));
        pane.render(buf.area, &mut buf, &mut 0);
        assert_eq!(
            lines(&buf)
                .iter()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>(),
            [
                "  ✕ Trash",
                "",
                "▌ - Cancel the newspaper · Inbox",
                "  - Paint the hallway · Home › Move to",
                "    the new flat",
                "",
            ]
        );
    }

    // FR-OPS-01
    #[test]
    fn picked_rows_get_the_focus_marker_dimmed() {
        let mut state = app::State::new(demo(), app::Opened::Demo, TODAY);
        let extend = app::Message::Command(app::Command::Extend(app::Direction::Down));
        app::update(&mut state, extend, TODAY);
        let theme = BuiltIn::default().colors();
        let pane = ContainerPane::new(
            &state,
            state.pane_view(),
            true,
            TODAY,
            &theme,
            IconSet::Plain,
        );
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 8));
        pane.render(buf.area, &mut buf, &mut 0);
        let rows = lines(&buf);
        let marked: Vec<_> = (0..8u16)
            .filter(|&y| buf[(0, y)].symbol() == FOCUS_MARKER)
            .map(|y| {
                let cell = &buf[(0, y)];
                (
                    rows[usize::from(y)].trim_end().to_owned(),
                    cell.fg,
                    cell.modifier,
                )
            })
            .collect();
        let line = |text: &str, modifier| (text.to_owned(), theme.focus, modifier);
        assert_eq!(
            marked,
            [
                line("▌ - Call the bank about ⇥ Wed", Modifier::DIM),
                line("▌   the card", Modifier::DIM),
                line("▌ - Buy coffee beans @errands", Modifier::empty()),
            ],
            "every line of a picked row; the focused one as ever"
        );
    }

    // FR-EDIT-02, FR-EDIT-04
    #[test]
    fn the_title_edited_shows_in_its_row_without_tags_its_selection_marked() {
        let field = TextField::new("Buy beans".to_owned()).edited(app::TextEdit::Move {
            motion: app::Motion::WordLeft,
            select: true,
        });
        let (buf, caret) = draw_editing("Buy coffee beans", &field, 30);
        let rows = lines(&buf);
        assert_eq!(
            rows.get(6).map(String::as_str),
            Some("▌ - Buy beans                 ")
        );
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(8, 6)].bg, theme.selection, "the selected word");
        assert_eq!(buf[(7, 6)].bg, theme.current_line, "the space before it");
        assert_eq!(caret, Some(Position::new(8, 6)));
    }

    // FR-EDIT-02
    #[test]
    fn a_long_title_edited_scrolls_sideways_to_keep_the_caret_in_view() {
        let field = TextField::new("Call the bank about the card".to_owned());
        let (buf, caret) = draw_editing("Call the bank about the card", &field, 20);
        let rows = lines(&buf);
        assert_eq!(
            rows.get(5).map(String::as_str),
            Some("▌ -  about the card ")
        );
        assert_eq!(caret, Some(Position::new(19, 5)));
        let start = field.edited(app::TextEdit::Move {
            motion: app::Motion::Start,
            select: false,
        });
        let (buf, caret) = draw_editing("Call the bank about the card", &start, 20);
        assert_eq!(
            lines(&buf).get(5).map(String::as_str),
            Some("▌ - Call the bank ab")
        );
        assert_eq!(caret, Some(Position::new(4, 5)));
        assert_eq!(
            draw_editing("Buy coffee beans", &start, 20).1,
            Some(Position::new(4, 9)),
            "the caret goes where the edited row is"
        );
        assert_eq!(
            draw_editing("Read the article Sam sent", &start, 20).1,
            None,
            "a row out of view has no caret"
        );
    }

    /// The text of each row of `buf`.
    fn lines(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    fn demo() -> Document {
        demo::document(TODAY)
    }

    /// `rows` padded to `width` columns.
    fn padded(width: usize, rows: &[&str]) -> Vec<String> {
        rows.iter().map(|row| format!("{row:width$}")).collect()
    }

    // FR-VIEW-05, FR-ORG-05, FR-ORG-10, FR-TASK-03, FR-TAG-03, FR-UI-02, FR-DATE-02, FR-DATE-05
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
                    "  ○ Website                    ≡ ❞ 2 ⇥ in 20 days",
                    "  The new marketing site, built with the design",
                    "  team.",
                    "",
                    "  - Collect logos from partners",
                    "",
                    "  § Launch",
                    "    What has to happen before launch.",
                    "▌ - Write the announcement    ☑ 1/5 ≡ ❞ 1 ↦ today",
                    "▌   @writing",
                    "▌   Keep it under 300 words.",
                    "  - Book the launch call @meeting           ⇥ Sat",
                    "",
                    "  § Polish",
                    "  - Fix the footer links",
                    "  ✔ Compress the hero image",
                    "",
                    "  ○ Blog",
                    "    A project inside a project.",
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
                    "  - Water the plants        ↦ yesterday",
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
        assert_eq!(section.fg, Some(theme.sections));
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
        let tag = at(4, 9);
        assert_eq!(tag.fg, Some(theme.tag));
        assert!(tag.add_modifier.contains(Modifier::DIM));
        assert_eq!(at(4, 10).fg, Some(theme.dim_text), "a task's description");
        let progress = at(30, 8);
        assert_eq!(progress.fg, Some(theme.headers), "checklist progress");
        assert!(progress.add_modifier.contains(Modifier::DIM));
        let icons = at(36, 8);
        assert_eq!(icons.fg, Some(theme.headers));
        assert_eq!(at(42, 8).fg, Some(theme.start), "a start date chip");
        assert_eq!(
            at(4, 11).bg,
            Some(Color::Reset),
            "rows without focus have no band"
        );
        let done = at(4, 15);
        assert_eq!(done.fg, Some(theme.done));
        assert!(done.add_modifier.contains(Modifier::CROSSED_OUT));
        assert_eq!(at(2, 15).fg, Some(theme.done), "a done task's marker");
        assert!(!at(2, 15).add_modifier.contains(Modifier::CROSSED_OUT));
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
                    "▌ - Call the  ⇥ Wed",
                    "▌   bank",
                    "▌   about the",
                    "▌   card",
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
        assert_eq!(top, 10);
        assert_eq!(lines(&buf)[4].trim_end(), "▌ - Fix the footer links");
        draw_pane(
            &document,
            website,
            Some("Compress the hero image"),
            (50, 5),
            &mut top,
        );
        assert_eq!(top, 11, "one row further down scrolls one line");
        draw_pane(
            &document,
            website,
            Some("Fix the footer links"),
            (50, 5),
            &mut top,
        );
        assert_eq!(top, 11, "a row in view does not scroll");
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
            "▌ - Write the announcement    ☑ 1/5 ≡ ❞ 1 ↦ today"
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
