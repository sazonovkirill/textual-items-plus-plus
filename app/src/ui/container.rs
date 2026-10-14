//! The container pane: the opened container drawn as FR-VIEW-05 arranges it, in the look of
//! taskpaper-terminal ([ui.md §2](../../../specs/reference/taskpaper-terminal/specs/ui.md#2-blocks)):
//! its title and description, then a row for each of its items, with the focus marker and band
//! on the focused one and the marker dimmed on the picked ones, wrapping long text and scrolling
//! to keep the focus in view, and the title being edited, if any, as a text field in its row, or
//! in the heading for the container's own; a description being edited shows whole where it shows.

use std::{borrow::Cow, collections::HashMap, ops::Range, rc::Rc};

use jiff::civil::Date;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, StatefulWidget, Widget, Wrap},
};

use super::{Icon, IconSet, Theme, cut, field};
use crate::{
    app::{
        self, PaneView, Part, Query, Row, Selection, Source, State, TextField,
        view::{self, Group, Heading, Locating, Shown, View},
    },
    domain::{Document, Item, ItemId, ItemKind, Node, Tag, dates},
};

/// The columns left of the rows that hold the focus marker.
const GUTTER: u16 = 2;

/// The glyph marking the focused row (D-30 of taskpaper-terminal).
const FOCUS_MARKER: &str = "▌";

/// The number standing for the heading among the rows, which the focus takes as a row above the
/// first (D-68).
const HEADING: usize = usize::MAX;

/// The pane showing a container of a document with its rows, or a view with its groups. Its state
/// is the first line of the content shown, which it moves only as far as it takes to keep the
/// focused row in view.
pub(super) struct ContainerPane<'a> {
    document: &'a Document,
    shown: Shown,
    rows: Rc<[Row]>,
    /// The container shown when its title is a row the focus takes.
    heading: Option<ItemId>,
    groups: Rc<[Group]>,
    selection: &'a Selection,
    editing: Option<(ItemId, Part, &'a TextField)>,
    /// The words searched for, while the pane shows search results, and whether they are typed.
    search: Option<(&'a TextField, bool)>,
    /// Whether the pane has the keys, which its focus marker and band show.
    keys: bool,
    /// The local date, which date chips are named relative to.
    today: Date,
    theme: &'a Theme,
    icons: IconSet,
    /// Whether long text wraps, as the settings say (FR-UI-05).
    wrap: bool,
    /// The tags the pane is filtered by (FR-TAG-04).
    filter: &'a [String],
    /// The pane's number, while there are several (FR-PANE-02).
    number: Option<usize>,
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
            rows: pane.shared_rows(),
            heading: pane.heading(),
            groups: pane.groups(),
            selection: pane.selection(),
            editing: state.editing().filter(|_| keys),
            search: pane
                .query()
                .map(|query| (query, keys && state.search().is_some())),
            keys,
            today,
            theme,
            icons,
            wrap: state.settings().word_wrap,
            filter: pane.tag_filter(),
            number: None,
        }
    }

    /// The pane numbered `number`, which its title line ends in (FR-PANE-02).
    pub(super) fn numbered(mut self, number: usize) -> Self {
        self.number = Some(number);
        self
    }

    /// How the text of a title or a description line fits its columns: wrapping, or with word
    /// wrap off cut short on its one line (FR-UI-05).
    fn fit(&self) -> Fit {
        if self.wrap { Fit::Wrap } else { Fit::Cut }
    }
}

/// How the text of a piece fits its columns.
#[derive(Clone, Copy)]
enum Fit {
    /// Wrapping between words (FR-UI-05).
    Wrap,
    /// Each line cut short with `…`, word wrap being off (FR-UI-05).
    Cut,
    /// As it is laid out, as a text field lays out its lines.
    Keep,
}

/// A run of lines of the pane: text starting `x` columns after the gutter, fitting its columns
/// as `fit` says, with what is drawn right after the gutter and at the right of its first line,
/// part of the row numbered `row`, if any. A text field is laid out in lines of its own instead,
/// with its caret on line `caret.0`, `caret.1` columns into it.
struct Piece<'a> {
    x: u16,
    text: Text<'a>,
    fit: Fit,
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
            text: Text::default(),
            fit: Fit::Keep,
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

    /// Its text fitted to content `width`.
    fn paragraph(&self, width: u16) -> Paragraph<'_> {
        match self.fit {
            Fit::Wrap => Paragraph::new(self.text.clone()).wrap(Wrap { trim: true }),
            Fit::Cut => {
                let columns = usize::from(self.text_width(width));
                let lines: Vec<_> = self
                    .text
                    .lines
                    .iter()
                    .map(|line| cut::truncate(line, columns))
                    .collect();
                Paragraph::new(Text::from(lines).style(self.text.style))
            }
            Fit::Keep => Paragraph::new(self.text.clone()),
        }
    }

    /// The lines it takes in content `width`: at least one.
    fn height(&self, width: u16) -> usize {
        self.paragraph(width)
            .line_count(self.text_width(width))
            .max(1)
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

/// Where a pane is scrolled to: the first line it shows is `line` lines into its content's part
/// numbered `part`. Counting by parts rather than by lines lets a pane lay out only the parts it
/// shows, however many rows it has (N-PERF-02).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Top {
    part: usize,
    line: usize,
}

impl Top {
    /// `lines` lines further down.
    fn down(self, lines: usize) -> Self {
        Self {
            line: self.line + lines,
            ..self
        }
    }
}

/// A part of the pane's content, laid out on its own: the heading, a blank line opening a group
/// with the group's heading, if any, or a row with the number it has among the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    Heading,
    Gap(Option<Heading>),
    Row(usize),
}

/// A part laid out: its pieces, the line of the part each starts at, the lines it takes in all
/// and the lines of its row, if it shows one.
struct Laid<'a> {
    pieces: Vec<Piece<'a>>,
    starts: Vec<usize>,
    height: usize,
    row: Option<(usize, usize)>,
}

/// The pane's parts in content `width`, each laid out when first asked for, so that drawing lays
/// out only those around the focus and on screen.
struct Layout<'p, 'a> {
    pane: &'p ContainerPane<'a>,
    parts: Vec<Block>,
    /// The part of each row, by the number it has among the rows.
    row_parts: Vec<usize>,
    width: u16,
    laid: HashMap<usize, Laid<'a>>,
}

impl<'p, 'a> Layout<'p, 'a> {
    fn new(pane: &'p ContainerPane<'a>, width: u16) -> Self {
        let parts = pane.parts();
        let row_parts = (parts.iter().enumerate())
            .filter_map(|(part, block)| matches!(block, Block::Row(_)).then_some(part))
            .collect();
        Self {
            parts,
            row_parts,
            pane,
            width,
            laid: HashMap::new(),
        }
    }

    /// The part numbered `part` laid out.
    fn laid(&mut self, part: usize) -> &Laid<'a> {
        let (pane, width) = (self.pane, self.width);
        let kind = self.parts.get(part).copied();
        self.laid.entry(part).or_insert_with(|| {
            let pieces = kind.map_or_else(Vec::new, |kind| pane.part_pieces(kind, width));
            let mut starts = Vec::with_capacity(pieces.len());
            let mut height = 0;
            let mut row: Option<(usize, usize)> = None;
            for piece in &pieces {
                starts.push(height);
                let end = height + piece.height(width);
                if piece.row.is_some() {
                    row = Some(row.map_or((height, end), |(first, _)| (first, end)));
                }
                height = end;
            }
            Laid {
                pieces,
                starts,
                height,
                row,
            }
        })
    }

    fn height(&mut self, part: usize) -> usize {
        self.laid(part).height
    }

    /// Past the last line.
    fn end(&self) -> Top {
        Top {
            part: self.parts.len(),
            line: 0,
        }
    }

    /// `at` with its line inside its part, moving on past the parts it counts beyond.
    fn settled(&mut self, mut at: Top) -> Top {
        while at.part < self.parts.len() {
            let height = self.height(at.part);
            if at.line < height {
                break;
            }
            at.line -= height;
            at.part += 1;
        }
        if at.part >= self.parts.len() {
            return self.end();
        }
        at
    }

    /// The position `lines` lines above `at`, or the first line.
    fn up(&mut self, at: Top, mut lines: usize) -> Top {
        let mut at = at;
        while at.line < lines {
            lines -= at.line;
            let Some(part) = at.part.checked_sub(1) else {
                return self.settled(Top::default());
            };
            at = Top {
                part,
                line: self.height(part),
            };
        }
        self.settled(Top {
            line: at.line - lines,
            ..at
        })
    }

    /// The lines from `from` down to `to`, counted no further than past `limit`.
    fn between(&mut self, from: Top, to: Top, limit: usize) -> usize {
        let mut lines = 0;
        let mut part = from.part;
        while part < to.part && lines <= limit + from.line {
            lines += self.height(part);
            part += 1;
        }
        if part < to.part {
            return limit + 1;
        }
        (lines + to.line).saturating_sub(from.line)
    }

    /// The part showing row `row`, or the heading, the first part, for [`HEADING`].
    fn part_of(&self, row: usize) -> Option<usize> {
        if row == HEADING {
            Some(0)
        } else {
            self.row_parts.get(row).copied()
        }
    }

    /// The lines of row `row`, from its first to past its last.
    fn row_span(&mut self, row: usize) -> Option<(Top, Top)> {
        let part = self.part_of(row)?;
        let (start, end) = self.laid(part).row?;
        let start = self.settled(Top { part, line: start });
        Some((start, Top { part, line: end }))
    }

    /// The line of the caret, if a piece has one.
    fn caret(&mut self) -> Option<(Top, usize)> {
        let edited = self.pane.editing.and_then(|(id, ..)| {
            self.pane
                .rows
                .iter()
                .position(|row| row.id() == id)
                .and_then(|row| self.part_of(row))
        });
        let width = self.width;
        [Some(0), edited].into_iter().flatten().find_map(|part| {
            let laid = self.laid(part);
            let (piece, start) = laid
                .pieces
                .iter()
                .zip(&laid.starts)
                .find(|(piece, _)| piece.caret.is_some())?;
            let (line, column) = piece.caret?;
            let column = column - piece.shift(width);
            let x = usize::from(GUTTER + piece.x) + column;
            Some((
                Top {
                    part,
                    line: start + line,
                },
                x,
            ))
        })
    }

    /// Scrolls `top` to keep the lines `span` in view in a pane `height` lines tall, moving it
    /// no further than needed; to the first of them when they are more than `height`.
    fn scrolled(&mut self, top: Top, span: Option<(Top, Top)>, height: usize) -> Top {
        let end = self.end();
        let top = if self.between(top, end, height) < height {
            self.up(end, height).min(top)
        } else {
            top
        };
        match span {
            Some((start, end)) if start < top || self.between(start, end, height) > height => start,
            Some((_, end)) if self.between(top, end, height) > height => self.up(end, height),
            _ => top,
        }
    }
}

impl StatefulWidget for ContainerPane<'_> {
    type State = Top;

    fn render(self, area: Rect, buf: &mut Buffer, top: &mut Top) {
        self.draw(area, buf, top);
    }
}

impl ContainerPane<'_> {
    /// Draws the pane over `area` from `top`, scrolled as little as keeps the focused row in
    /// view, and returns the screen lines of each row on screen, the heading's among them, in
    /// order.
    pub(super) fn draw(self, area: Rect, buf: &mut Buffer, top: &mut Top) -> Vec<(ItemId, Rect)> {
        let width = area.width.saturating_sub(GUTTER);
        let height = usize::from(area.height);
        let mut layout = Layout::new(&self, width);
        let focused = self.focused_row();
        let span = focused.and_then(|row| layout.row_span(row));
        // Focusing the first row shows the container's title and description above it, when
        // they fit with the row, as does editing that description. A caret stays in view.
        let caret = layout.caret().map(|(line, _)| line);
        let in_row = |line| span.is_some_and(|(start, end)| start <= line && line < end);
        let first = layout.settled(Top::default());
        let shown = match caret {
            Some(line) if !in_row(line) => {
                let from = if layout.between(first, line, height) < height {
                    first
                } else {
                    line
                };
                Some((from, line.down(1)))
            }
            _ => span.map(|(start, end)| match focused {
                Some(0) if layout.between(first, end, height) <= height => (first, end),
                _ => (start, end),
            }),
        };
        let settled = layout.settled(*top);
        *top = layout.scrolled(settled, shown, height);
        *top = layout.scrolled(*top, caret.map(|line| (line, line.down(1))), height);
        let mut shown = Vec::new();
        let mut start = 0;
        let mut part = top.part;
        while part < layout.parts.len() && start < top.line + height {
            shown.push((part, start));
            start += layout.height(part);
            part += 1;
        }
        let focused_part = focused.and_then(|row| layout.part_of(row));
        let band = shown
            .iter()
            .filter(|_| self.keys)
            .find(|&&(part, _)| Some(part) == focused_part)
            .and_then(|&(part, at)| {
                let (start, end) = layout.laid(part).row?;
                Some((at + start, at + end))
            });
        if let Some((start, end)) = band {
            self.band(area, buf, start, end, top.line);
        }
        for &(part, at) in &shown {
            let picked = self.keys
                && Some(part) != focused_part
                && matches!(layout.parts.get(part), Some(&Block::Row(index))
                    if self.rows.get(index).is_some_and(|row| self.selection.is_picked(row.id())));
            let laid = layout.laid(part);
            if let Some((start, end)) = laid.row.filter(|_| picked) {
                self.picked(area, buf, at + start, at + end, top.line);
            }
            for (piece, &start) in laid.pieces.iter().zip(&laid.starts) {
                draw(piece, at + start, top.line, area, buf);
            }
        }
        if let Some((start, end)) = band {
            self.band_text(area, buf, start, end, top.line);
        }
        self.on_screen(&mut layout, &shown, area, top.line)
    }

    /// The screen lines over `area` of each row the parts `shown` show, each with the line it
    /// starts at counted from the first part's start, `top` lines of which are scrolled away.
    fn on_screen(
        &self,
        layout: &mut Layout,
        shown: &[(usize, usize)],
        area: Rect,
        top: usize,
    ) -> Vec<(ItemId, Rect)> {
        let bottom = top + usize::from(area.height);
        shown
            .iter()
            .filter_map(|&(part, at)| {
                let id = match layout.parts.get(part)? {
                    Block::Heading => self.heading?,
                    Block::Row(index) => self.rows.get(*index)?.id(),
                    Block::Gap(_) => return None,
                };
                let (start, end) = layout.laid(part).row?;
                let (start, end) = ((at + start).max(top), (at + end).min(bottom));
                let y = |line: usize| area.y + u16::try_from(line - top).unwrap_or(u16::MAX);
                (start < end).then(|| {
                    (
                        id,
                        Rect::new(area.x, y(start), area.width, y(end) - y(start)),
                    )
                })
            })
            .collect()
    }
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
        .paragraph(width)
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
    /// Where the terminal's cursor goes when the pane is drawn over `area` scrolled to `top`: at
    /// the caret of the title being edited, when it is in view and the pane has room for it.
    pub(super) fn caret(&self, area: Rect, top: Top) -> Option<Position> {
        let height = usize::from(area.height);
        let mut layout = Layout::new(self, area.width.saturating_sub(GUTTER));
        let (line, x) = layout.caret()?;
        let top = layout.settled(top);
        if line < top {
            return None;
        }
        let row = layout.between(top, line, height);
        let row = u16::try_from(row).ok().filter(|&row| row < area.height)?;
        let x = u16::try_from(usize::from(area.x) + x)
            .ok()
            .filter(|&x| x < area.right())?;
        Some(Position::new(x, area.y + row))
    }

    /// The index of the focused row among the state's rows, or [`HEADING`] for the heading.
    fn focused_row(&self) -> Option<usize> {
        let focused = self.selection.focused()?;
        if Some(focused) == self.heading {
            return Some(HEADING);
        }
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

    /// The parts of the pane's content: the heading, then a part for each row, after a gap
    /// opening each group of a container's rows; or, for a view, its name, then each of its groups
    /// after a gap with its heading.
    fn parts(&self) -> Vec<Block> {
        let mut parts = vec![Block::Heading];
        match self.shown {
            Shown::Container(_) => {
                for (index, row) in self.rows.iter().enumerate() {
                    let before = index
                        .checked_sub(1)
                        .and_then(|before| self.rows.get(before));
                    let opens_group = index == 0
                        || match row {
                            Row::Section(_) => true,
                            Row::Project(_) => !matches!(before, Some(Row::Project(_))),
                            Row::Task { .. } => false,
                        };
                    if opens_group {
                        parts.push(Block::Gap(None));
                    }
                    parts.push(Block::Row(index));
                }
            }
            Shown::View(_) => {
                let mut indexes = 0..;
                for group in self.groups.iter() {
                    parts.push(Block::Gap(group.heading));
                    parts.extend(
                        group
                            .rows
                            .iter()
                            .zip(&mut indexes)
                            .map(|(_, index)| Block::Row(index)),
                    );
                }
            }
        }
        parts
    }

    /// The pieces of `part` in content `width`: the container's title and description, or the
    /// view's name; a blank line with the group's heading under it; or a row's lines.
    fn part_pieces(&self, part: Block, width: u16) -> Vec<Piece<'a>> {
        let mut pieces = Vec::new();
        match (part, self.shown) {
            (Block::Heading, Shown::Container(id)) => {
                if let Some(container) = self.item(id) {
                    let row = self.heading.map(|_| HEADING);
                    pieces.push(self.ended(match self.edited(id, Part::Title) {
                        Some(field) => self.field(container, field, row),
                        None => self.title(container, row),
                    }));
                    if let Some(field) = self.edited(id, Part::Description) {
                        pieces.push(self.description_field(field, 0, width, row));
                    } else if !container.description.is_empty() {
                        pieces.push(Piece {
                            x: 0,
                            text: Text::styled(
                                container.description.as_str(),
                                self.description_style(),
                            ),
                            fit: self.fit(),
                            left: Line::default(),
                            right: Line::default(),
                            row,
                            caret: None,
                        });
                    }
                }
            }
            (Block::Heading, Shown::View(view)) => pieces.push(self.ended(self.view_title(view))),
            (Block::Gap(heading), _) => {
                pieces.push(Piece::blank());
                pieces.extend(heading.and_then(|heading| self.heading(heading)));
            }
            (Block::Row(index), _) => {
                let row = self.rows.get(index);
                if let Some((row, item)) = row.and_then(|row| Some((*row, self.item(row.id())?))) {
                    self.row_pieces(index, row, item, width, &mut pieces);
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
                text: Text::styled(text, self.description_style()),
                fit: self.fit(),
                left: Line::default(),
                right: Line::default(),
                row: Some(index),
                caret: None,
            });
        }
        if let Some(field) = self.edited(row.id(), Part::Description) {
            pieces.push(self.description_field(field, x, width, Some(index)));
        } else if let Some(excerpt) = self.excerpt(row.id()) {
            pieces.push(Piece {
                x,
                text: Text::from(excerpt),
                fit: self.fit(),
                left: Line::default(),
                right: Line::default(),
                row: Some(index),
                caret: None,
            });
        } else if !under && let Some(first) = shown.description.lines().next() {
            pieces.push(Piece {
                x,
                text: Text::styled(first, self.description_style()),
                fit: self.fit(),
                left: Line::default(),
                right: Line::default(),
                row: Some(index),
                caret: None,
            });
        }
    }

    /// `title`, the pane's title line, with the tags the pane is filtered by after its icon in the
    /// tag color and the pane's number dimmed at the right end, after what it has there.
    fn ended(&self, mut title: Piece<'a>) -> Piece<'a> {
        if !self.filter.is_empty() {
            let tags: Vec<String> = self.filter.iter().map(|name| format!("@{name}")).collect();
            title.right.spans.push(Span::styled(
                format!("{} {} ", Icon::Tag.glyph(self.icons), tags.join(" ")),
                Style::new().fg(self.theme.tag),
            ));
        }
        if let Some(number) = self.number {
            title.right.spans.push(Span::styled(
                format!("{number} "),
                Style::new().fg(self.theme.dim_text),
            ));
        }
        title
    }

    /// The line naming `view` at the top of the pane: its icon, then its name, in the Inbox's
    /// style, and for search results the words searched for after it, with their caret and
    /// selection while they are typed.
    fn view_title(&self, view: View) -> Piece<'a> {
        let style = Style::new().fg(self.theme.primary).bold();
        let icon = super::view_icon(view).glyph(self.icons);
        let mut piece = Piece {
            x: u16::try_from(Span::raw(icon).width() + 1).unwrap_or(u16::MAX),
            text: Text::styled(view.title(), style),
            fit: self.fit(),
            left: Line::styled(icon, style),
            right: Line::default(),
            row: None,
            caret: None,
        };
        if let Some((query, typing)) = self.search {
            let name = format!("{}  ", view.title());
            let before = Span::raw(name.clone()).width();
            let mut spans = vec![Span::styled(name, style)];
            let text = query.text();
            if typing {
                spans.extend(field::selected(query, 0..text.len(), self.theme).spans);
                let caret = text.get(..query.caret()).unwrap_or_default();
                piece.caret = Some((0, before + Span::raw(caret).width()));
                piece.fit = Fit::Keep;
            } else {
                spans.push(Span::raw(text));
            }
            piece.text = Text::from(Line::from(spans));
        }
        piece
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
                text: Text::styled(period.heading(self.today), Modifier::BOLD),
                fit: self.fit(),
                ..Piece::blank()
            }),
            Heading::Completed(day) => Some(Piece {
                text: Text::styled(
                    day.map_or_else(
                        || "No date".to_owned(),
                        |day| dates::day_heading(day, self.today),
                    ),
                    Modifier::BOLD,
                ),
                fit: self.fit(),
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
        let mut spans = self.marked(Cow::Borrowed(&item.title), self.title_style(item));
        for tag in item.shown_tags() {
            spans.extend(self.marked(Cow::Owned(tag_text(tag)), self.tag_style(item)));
        }
        spans.extend(after);
        Piece {
            x: self.title_column(item),
            text: Text::from(Line::from(spans)),
            fit: self.fit(),
            left: Line::styled(self.marker(item), self.marker_style(item)),
            right: Line::from([self.attachments(item).spans, self.chips(item)].concat()),
            row,
            caret: None,
        }
    }

    /// `text` in `style`, with the words searched for in it marked, while the pane shows search
    /// results (FR-SRCH-01).
    fn marked(&self, text: Cow<'a, str>, style: Style) -> Vec<Span<'a>> {
        let ranges = self
            .search
            .map(|(query, _)| Query::new(query.text()).ranges(&text))
            .unwrap_or_default();
        if ranges.is_empty() {
            return vec![Span::styled(text, style)];
        }
        let mark = style
            .fg(self.theme.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        let mut spans = Vec::with_capacity(ranges.len() * 2 + 1);
        let mut at = 0;
        let part = |range: Range<usize>| text.get(range).unwrap_or_default().to_owned();
        for range in ranges {
            spans.push(Span::styled(part(at..range.start), style));
            spans.push(Span::styled(part(range.clone()), mark));
            at = range.end;
        }
        spans.push(Span::styled(part(at..text.len()), style));
        spans
    }

    /// The line of item `id` outside its title holding a word searched for that its title does
    /// not, after the icon of where it comes from, the words in it marked, while the pane shows
    /// search results (FR-SRCH-01).
    fn excerpt(&self, id: ItemId) -> Option<Line<'a>> {
        let (query, _) = self.search?;
        let excerpt = Query::new(query.text()).excerpt(self.document, id)?;
        let style = self.description_style();
        let icon = match excerpt.source {
            Source::Description => None,
            Source::Note => Some(Icon::Notes),
            Source::Comment => Some(Icon::Comments),
            Source::Checklist => Some(Icon::Checklist),
        };
        let mut spans: Vec<Span<'a>> = icon
            .map(|icon| Span::styled(format!("{} ", icon.glyph(self.icons)), style))
            .into_iter()
            .collect();
        spans.extend(self.marked(Cow::Owned(excerpt.text), style));
        Some(Line::from(spans))
    }

    /// The line of `item`, numbered `row` if it is one, while `field` edits its title: its marker,
    /// then the text edited, without tags or date chips, the selected part in the selection
    /// colors, on one line that scrolls sideways to keep the caret in view.
    fn field(&self, item: &'a Item, field: &'a TextField, row: Option<usize>) -> Piece<'a> {
        let text = field.text();
        let caret = text.get(..field.caret()).unwrap_or_default();
        Piece {
            caret: Some((0, Span::raw(caret).width())),
            text: Text::from(field::selected(field, 0..text.len(), self.theme)),
            fit: Fit::Keep,
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
            text: Text::from(lines),
            fit: Fit::Keep,
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

    /// The chips of `item`'s start and due dates, none for an area, which has no dates (D-75),
    /// each its icon and the date named relative to today, a space after each: start in the start
    /// color and due in the due color, or, when it is overdue, with the overdue icon, bold and
    /// reversed in the overdue color; all dimmed once it is completed (FR-DATE-02, FR-DATE-04).
    fn chips(&self, item: &Item) -> Vec<Span<'a>> {
        if !item.kind.takes_tags_and_dates() {
            return Vec::new();
        }
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
        size: (u16, u16),
        top: &mut Top,
    ) -> Buffer {
        draw_wrapped(document, container, focused, size, top, true)
    }

    /// Draws as [`draw_pane`] does, with word wrap on or off as `wrap` says.
    fn draw_wrapped(
        document: &Document,
        container: ItemId,
        focused: Option<&str>,
        (width, height): (u16, u16),
        top: &mut Top,
        wrap: bool,
    ) -> Buffer {
        let theme = BuiltIn::default().colors();
        let mut selection = Selection::default();
        selection.focus(focused.map(|title| find(document, title)));
        let pane = ContainerPane {
            document,
            shown: Shown::Container(container),
            groups: Rc::from([]),
            rows: app::rows(document, container, true).into(),
            heading: (container != document.inbox()).then_some(container),
            selection: &selection,
            editing: None,
            search: None,
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
            wrap,
            filter: &[],
            number: None,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        pane.render(buf.area, &mut buf, top);
        buf
    }

    /// Draws the Inbox of the demo document, `width` by 10, while `field` edits the title of
    /// its task titled `title`, and returns the buffer and where the caret is.
    fn draw_editing(title: &str, field: &TextField, width: u16) -> (Buffer, Option<Position>) {
        draw_editing_from(title, field, width, Top::default())
    }

    /// As [`draw_editing`], with the caret placed for the pane scrolled to `top`.
    fn draw_editing_from(
        title: &str,
        field: &TextField,
        width: u16,
        top: Top,
    ) -> (Buffer, Option<Position>) {
        let document = demo();
        let theme = BuiltIn::default().colors();
        let id = find(&document, title);
        let mut selection = Selection::default();
        selection.focus(Some(id));
        let pane = ContainerPane {
            document: &document,
            shown: Shown::Container(document.inbox()),
            groups: Rc::from([]),
            rows: app::rows(&document, document.inbox(), false).into(),
            heading: None,
            selection: &selection,
            editing: Some((id, Part::Title, field)),
            search: None,
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
            wrap: true,
            filter: &[],
            number: None,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, width, 10));
        let caret = pane.caret(buf.area, top);
        pane.render(buf.area, &mut buf, &mut Top::default());
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
            groups: Rc::from([]),
            rows: app::rows(&document, document.inbox(), false).into(),
            heading: None,
            selection: &selection,
            editing: Some((document.inbox(), Part::Title, &field)),
            search: None,
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
            wrap: true,
            filter: &[],
            number: None,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 4));
        pane.render(buf.area, &mut buf, &mut Top::default());
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
            groups: Rc::from([]),
            rows: app::rows(&document, document.inbox(), false).into(),
            heading: None,
            selection: &selection,
            editing: Some((id, Part::Description, field)),
            search: None,
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
            wrap: true,
            filter: &[],
            number: None,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        let area = buf.area;
        let mut top = Top::default();
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
        pane.render(buf.area, &mut buf, &mut Top::default());
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
            pane.render(buf.area, &mut buf, &mut Top::default());
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
        pane.render(buf.area, &mut buf, &mut Top::default());
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
        pane.render(buf.area, &mut buf, &mut Top::default());
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

    // FR-SRCH-01
    #[test]
    fn search_results_mark_the_words_found_and_show_the_line_holding_one_their_title_lacks() {
        let mut state = app::State::new(demo(), app::Opened::Demo, TODAY);
        app::update(
            &mut state,
            app::Message::Command(app::Command::Search),
            TODAY,
        );
        for c in "SHARED writing".chars() {
            let edit = app::SearchInput::Edit(app::TextEdit::Insert(c));
            app::update(&mut state, app::Message::Search(edit), TODAY);
        }
        let theme = BuiltIn::default().colors();
        let draw = |state: &app::State| {
            let pane = ContainerPane::new(
                state,
                state.pane_view(),
                true,
                TODAY,
                &theme,
                IconSet::Plain,
            );
            let mut buf = Buffer::empty(Rect::new(0, 0, 70, 5));
            pane.render(buf.area, &mut buf, &mut Top::default());
            buf
        };
        let buf = draw(&state);
        assert_eq!(
            lines(&buf)
                .iter()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>(),
            [
                "  ⌕ Search  SHARED writing",
                "",
                "▌ - Write the announcement @writing · Work ›      ☑ 1/5 ≡ ❞ 1 ↦ today",
                "▌   Website › Launch",
                "▌   ≡ Draft in the shared doc.",
            ]
        );
        let mark = |x: u16, y: u16| buf[(x, y)].modifier.contains(Modifier::UNDERLINED);
        let marked: String = (0..70)
            .filter(|&x| mark(x, 2))
            .map(|x| buf[(x, 2)].symbol().to_owned())
            .collect();
        assert_eq!(marked, "writing", "the tag holds a word");
        let marked: String = (0..70)
            .filter(|&x| mark(x, 4))
            .map(|x| buf[(x, 4)].symbol().to_owned())
            .collect();
        assert_eq!(
            marked, "shared",
            "the note's line holding a word not in the title"
        );
        for edit in [app::TextEdit::SelectAll, app::TextEdit::Insert('o')] {
            let edit = app::SearchInput::Edit(edit);
            app::update(&mut state, app::Message::Search(edit), TODAY);
        }
        for c in "utline".chars() {
            let edit = app::SearchInput::Edit(app::TextEdit::Insert(c));
            app::update(&mut state, app::Message::Search(edit), TODAY);
        }
        assert_eq!(
            lines(&draw(&state))[4].trim_end(),
            "▌   ☑ Outline",
            "a checklist item holding a word shows under its task"
        );
        for (words, line) in [
            ("screenshots", "▌   ❞ Ask design for screenshots"),
            ("300", "▌   Keep it under 300 words."),
        ] {
            let edit = app::SearchInput::Edit(app::TextEdit::SelectAll);
            app::update(&mut state, app::Message::Search(edit), TODAY);
            for c in words.chars() {
                let edit = app::SearchInput::Edit(app::TextEdit::Insert(c));
                app::update(&mut state, app::Message::Search(edit), TODAY);
            }
            assert_eq!(lines(&draw(&state))[4].trim_end(), line);
        }
        app::update(
            &mut state,
            app::Message::Search(app::SearchInput::Done),
            TODAY,
        );
        app::update(
            &mut state,
            app::Message::Command(app::Command::Search),
            TODAY,
        );
        assert_eq!(
            draw(&state)[(12, 0)].style().bg,
            Some(theme.selection),
            "searching again selects the words, so that typing replaces them"
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
        pane.render(buf.area, &mut buf, &mut Top::default());
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
        let below = Top {
            part: usize::MAX,
            line: 0,
        };
        assert_eq!(
            draw_editing_from("Buy coffee beans", &start, 20, below).1,
            None,
            "a row scrolled above the top has no caret"
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
    fn a_project_shows_its_title_description_tasks_and_sections() {
        let document = demo();
        let website = find(&document, "Website");
        let buf = draw_pane(
            &document,
            website,
            Some("Write the announcement"),
            (50, 20),
            &mut Top::default(),
        );
        assert_eq!(
            lines(&buf),
            padded(
                50,
                &[
                    "  # Website                    ≡ ❞ 2 ⇥ in 20 days",
                    "  The new marketing site, built with the design",
                    "  team.",
                    "",
                    "  - Collect logos from partners",
                    "",
                    "  / Launch",
                    "    What has to happen before launch.",
                    "▌ - Write the announcement    ☑ 1/5 ≡ ❞ 1 ↦ today",
                    "▌   @writing",
                    "▌   Keep it under 300 words.",
                    "  - Book the launch call @meeting           ⇥ Sat",
                    "",
                    "  / Polish",
                    "  - Fix the footer links",
                    "  ✔ Compress the hero image",
                    "",
                    "",
                    "",
                    "",
                ]
            )
        );
    }

    // FR-NAV-01, D-68
    #[test]
    fn the_heading_takes_the_focus_as_a_row_with_its_description() {
        let document = demo();
        let website = find(&document, "Website");
        let buf = draw_pane(
            &document,
            website,
            Some("Website"),
            (50, 4),
            &mut Top::default(),
        );
        assert_eq!(
            lines(&buf),
            [
                "▌ # Website                    ≡ ❞ 2 ⇥ in 20 days ",
                "▌ The new marketing site, built with the design   ",
                "▌ team.                                           ",
                "                                                  ",
            ]
        );
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(4, 2)].bg, theme.current_line);
        assert_ne!(buf[(4, 3)].bg, theme.current_line);
    }

    // FR-VIEW-05, D-26
    #[test]
    fn an_area_shows_its_child_projects_as_one_row_each() {
        let document = demo();
        let home = find(&document, "Home");
        let buf = draw_pane(&document, home, None, (40, 6), &mut Top::default());
        assert_eq!(
            lines(&buf),
            padded(
                40,
                &[
                    "  ◆ Home",
                    "",
                    "  - Water the plants        ↦ yesterday",
                    "",
                    "  # Move to the new flat",
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
            &mut Top::default(),
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
    fn without_word_wrap_titles_and_descriptions_take_a_line_each_cut_short() {
        let document = demo();
        let inbox = document.inbox();
        let buf = draw_wrapped(
            &document,
            inbox,
            Some("Call the bank about the card"),
            (20, 6),
            &mut Top::default(),
            false,
        );
        assert_eq!(
            lines(&buf),
            padded(
                20,
                &[
                    "  ⊔ Inbox",
                    "  Everything not so…",
                    "",
                    "▌ - Call the… ⇥ Wed",
                    "  - Buy coffee bean…",
                    "  - Read the articl…",
                ]
            )
        );
        let website = find(&document, "Website");
        let buf = draw_wrapped(
            &document,
            website,
            None,
            (36, 10),
            &mut Top::default(),
            false,
        );
        assert_eq!(
            lines(&buf),
            padded(
                36,
                &[
                    "  # Website      ≡ ❞ 2 ⇥ in 20 days",
                    "  The new marketing site, built wit…",
                    "",
                    "  - Collect logos from partners",
                    "",
                    "  / Launch",
                    "    What has to happen before launc…",
                    "  - Write the … ☑ 1/5 ≡ ❞ 1 ↦ today",
                    "    Keep it under 300 words.",
                    "  - Book the launch call @me… ⇥ Sat",
                ]
            ),
            "a description shows its first line under the title"
        );
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
            &mut Top::default(),
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
        let mut top = Top::default();
        let first = |focused, height, top: &mut Top| {
            let buf = draw_pane(&document, website, Some(focused), (50, height), top);
            (
                lines(&buf)[0].trim_end().to_owned(),
                lines(&buf).last().cloned(),
            )
        };
        let (line, last) = first("Fix the footer links", 5, &mut top);
        assert_eq!(line, "    Keep it under 300 words.");
        assert_eq!(
            last.as_deref().map(str::trim_end),
            Some("▌ - Fix the footer links")
        );
        assert_eq!(
            first("Compress the hero image", 5, &mut top).0,
            "  - Book the launch call @meeting           ⇥ Sat",
            "one row further down scrolls one line"
        );
        assert_eq!(
            first("Fix the footer links", 5, &mut top).0,
            "  - Book the launch call @meeting           ⇥ Sat",
            "a row in view does not scroll"
        );
        assert_eq!(first("Launch", 5, &mut top).0, "▌ / Launch");
        assert_eq!(
            first("Collect logos from partners", 5, &mut top).0,
            "  # Website                    ≡ ❞ 2 ⇥ in 20 days",
            "the first row shows the title above it"
        );
        assert_eq!(
            first("Write the announcement", 1, &mut top).0,
            "▌ - Write the announcement    ☑ 1/5 ≡ ❞ 1 ↦ today",
            "a row taller than the pane shows its first line"
        );
    }

    /// Scrolls a pane `height` lines tall, holding the Inbox's title, a blank line and
    /// `tasks` one-line tasks, from content line `top` to keep content lines `span` in view, and
    /// gives the content line it scrolls to.
    fn scrolled(tasks: usize, top: usize, span: Option<(usize, usize)>, height: usize) -> usize {
        let mut document = Document::new();
        for task in 0..tasks {
            let item = Item::new(ItemKind::Task, format!("Task {task}"));
            document
                .push(Some(document.inbox()), Node::Item(item))
                .expect("a task fits the Inbox");
        }
        let theme = BuiltIn::default().colors();
        let selection = Selection::default();
        let pane = ContainerPane {
            document: &document,
            shown: Shown::Container(document.inbox()),
            groups: Rc::from([]),
            rows: app::rows(&document, document.inbox(), false).into(),
            heading: None,
            selection: &selection,
            editing: None,
            search: None,
            keys: true,
            today: TODAY,
            theme: &theme,
            icons: IconSet::Plain,
            wrap: true,
            filter: &[],
            number: None,
        };
        let mut layout = Layout::new(&pane, 40);
        let mut at = |line| layout.settled(Top { part: 0, line });
        let (top, span) = (at(top), span.map(|(start, end)| (at(start), at(end))));
        let scrolled = layout.scrolled(top, span, height);
        layout.between(Top::default(), scrolled, usize::MAX - 1)
    }

    #[test]
    fn scrolling_keeps_the_top_within_the_content() {
        assert_eq!(scrolled(9, 30, None, 4), 7);
        assert_eq!(scrolled(1, 30, None, 4), 0);
        assert_eq!(scrolled(9, 2, Some((0, 1)), 4), 0);
        assert_eq!(scrolled(9, 2, Some((7, 9)), 4), 5);
        assert_eq!(scrolled(9, 2, Some((3, 5)), 4), 2);
        assert_eq!(scrolled(9, 0, Some((3, 9)), 4), 3);
    }

    // FR-UI-06
    #[test]
    fn a_tiny_pane_draws_what_fits() {
        let document = demo();
        let website = find(&document, "Website");
        for size in [(0, 0), (1, 1), (2, 3), (3, 2), (50, 0)] {
            draw_pane(
                &document,
                website,
                Some("Polish"),
                size,
                &mut Top::default(),
            );
        }
        let buf = draw_pane(
            &document,
            website,
            Some("Polish"),
            (4, 1),
            &mut Top::default(),
        );
        assert_eq!(lines(&buf), ["▌ / "]);
    }

    #[test]
    fn a_container_without_rows_shows_its_title_alone() {
        let document = Document::new();
        let buf = draw_pane(
            &document,
            document.inbox(),
            None,
            (12, 2),
            &mut Top::default(),
        );
        assert_eq!(lines(&buf), padded(12, &["  ⊔ Inbox", ""]));
        let missing = draw_pane(
            &document,
            ItemId::new(9),
            None,
            (12, 1),
            &mut Top::default(),
        );
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
        let buf = draw_pane(&document, inbox, None, (30, 4), &mut Top::default());
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
