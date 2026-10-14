//! The sidebar left of the pane: the Inbox, the views, the areas and projects as a tree, each after its icon
//! with a container's open tasks counted at the right, as the Navigator lists them, after a marker
//! on a collapsed area, the entry of what the pane shows on the selection background, the
//! focused one marked while the sidebar has the keys and the title being edited there as a text
//! field in its row (FR-VIEW-10).

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::{Icon, IconSet, Theme};
use crate::{
    app::{Part, SidebarView, State, Stop, TextField, view::Shown},
    domain::{Document, ItemId, ItemKind, Node},
};

/// The glyph marking the focused entry, as the pane marks its focused row.
const FOCUS_MARKER: &str = "▌";

/// The widest the sidebar gets, in columns.
const WIDTH: u16 = 28;

/// The columns the sidebar takes of a screen `width` wide: a third at most.
pub(super) fn width(width: u16) -> u16 {
    WIDTH.min(width / 3)
}

/// The sidebar of a state, as it shows.
pub(super) struct SidebarPane<'a> {
    document: &'a Document,
    view: SidebarView,
    /// The line each entry is drawn on, the blank lines between its groups left out.
    lines: Vec<usize>,
    /// The entry of what the pane shows, or of the closest container above it.
    current: Option<Shown>,
    editing: Option<(ItemId, &'a TextField)>,
    theme: &'a Theme,
    icons: IconSet,
}

/// Where an entry's icon and title start, and how far a text field in it is scrolled sideways.
struct Title {
    icon: u16,
    x: u16,
    shift: usize,
}

impl<'a> SidebarPane<'a> {
    /// The sidebar `view` of `state`, drawn in `theme` with `icons`.
    pub(super) fn new(
        state: &'a State,
        view: SidebarView,
        theme: &'a Theme,
        icons: IconSet,
    ) -> Self {
        let current = std::iter::once(state.shown())
            .chain(state.path().into_iter().rev().map(Shown::Container))
            .find(|&shown| view.entries.iter().any(|stop| stop.shown() == shown));
        Self {
            document: state.document(),
            editing: state
                .editing()
                .filter(|&(_, part, _)| view.keys && part == Part::Title)
                .map(|(id, _, field)| (id, field)),
            lines: lines(&view.entries),
            view,
            current,
            theme,
            icons,
        }
    }

    /// Where the terminal's cursor goes when the sidebar is drawn over `area`: at the caret of
    /// the title being edited in it, when there is one and it fits.
    pub(super) fn caret(&self, area: Rect) -> Option<Position> {
        let (id, field) = self.editing?;
        let content = content(area);
        let offset = self.offset(content);
        let (index, stop) = self
            .view
            .entries
            .iter()
            .enumerate()
            .find(|(_, stop)| stop.shown() == Shown::Container(id))?;
        let row = self.lines.get(index)?.checked_sub(offset)?;
        let y = content.y
            + u16::try_from(row)
                .ok()
                .filter(|&row| row < content.height)?;
        let title = self.title(*stop, content);
        let caret = Span::raw(field.text().get(..field.caret()).unwrap_or_default()).width();
        let x = usize::from(title.x) + caret - title.shift;
        let x = u16::try_from(x).ok().filter(|&x| x < content.right())?;
        Some(Position::new(x, y))
    }

    /// The first line shown in `content`, so that the focused entry is in view.
    fn offset(&self, content: Rect) -> usize {
        self.view
            .focused
            .and_then(|focused| self.lines.get(focused))
            .map_or(0, |&line| {
                (line + 1).saturating_sub(usize::from(content.height))
            })
    }

    /// Where the title of `stop` starts in `content`, and how far it is scrolled sideways to
    /// keep the caret in view while it is edited.
    fn title(&self, stop: Stop, content: Rect) -> Title {
        let column = |columns: usize| u16::try_from(columns).unwrap_or(u16::MAX);
        // v1 nests no project in another (D-57), so the groups' blank lines show the tree.
        let icon_x = content.x.saturating_add(1);
        let glyph = Span::raw(super::stop_icon(stop).glyph(self.icons)).width() + 1;
        let x = icon_x.saturating_add(column(glyph));
        let shift = match self.editing {
            Some((id, field)) if stop.shown() == Shown::Container(id) => {
                let caret = Span::raw(field.text().get(..field.caret()).unwrap_or_default());
                let room = usize::from(content.right().saturating_sub(x));
                (caret.width() + 1).saturating_sub(room).min(caret.width())
            }
            _ => 0,
        };
        Title {
            icon: icon_x,
            x,
            shift,
        }
    }

    /// How the title of `stop` is drawn: in its color, areas bold as headers (D-78).
    fn title_style(&self, stop: Stop) -> Style {
        let style = Style::new().fg(super::stop_title_color(self.theme, stop));
        if matches!(stop, Stop::Place(place) if place.kind == ItemKind::Area) {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    /// The line of the title of `stop`: its text, an area's in capitals as a header, or the text
    /// edited with the selected part in the selection colors.
    fn title_line(&self, stop: Stop) -> Line<'a> {
        let style = self.title_style(stop);
        match (self.editing, stop) {
            (Some((id, field)), Stop::Place(place)) if id == place.id => {
                let text = field.text();
                let selection = field.selection().unwrap_or(field.caret()..field.caret());
                let selected = Style::new()
                    .bg(self.theme.selection)
                    .fg(self.theme.band_text);
                Line::from(vec![
                    Span::styled(text.get(..selection.start).unwrap_or_default(), style),
                    Span::styled(text.get(selection.clone()).unwrap_or_default(), selected),
                    Span::styled(text.get(selection.end..).unwrap_or_default(), style),
                ])
            }
            (_, Stop::Place(place)) => {
                let title = self
                    .document
                    .node(place.id)
                    .and_then(Node::item)
                    .map_or("", |item| item.title.as_str());
                if place.kind == ItemKind::Area {
                    Line::styled(title.to_uppercase(), style)
                } else {
                    Line::styled(title, style)
                }
            }
            (_, Stop::View(view)) => Line::styled(view.title(), style),
        }
    }
}

impl Widget for SidebarPane<'_> {
    /// Draws the entries from the one that keeps the focused entry in view, each indented by
    /// its depth after the focus gutter, with a rule down the right edge between the sidebar and
    /// the pane.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let rule = Rect::new(area.right().saturating_sub(1), area.y, 1, area.height);
        for y in rule.intersection(area).rows() {
            buf.set_string(y.x, y.y, "│", Style::new().fg(theme.rule));
        }
        let content = content(area);
        let offset = self.offset(content);
        let rows: Vec<Rect> = content.rows().collect();
        let entries = self.view.entries.iter().zip(&self.lines).enumerate();
        for (index, (&stop, &line)) in entries {
            let Some(&row) = line.checked_sub(offset).and_then(|row| rows.get(row)) else {
                continue;
            };
            let focused = self.view.keys && Some(index) == self.view.focused;
            let editing = self
                .editing
                .is_some_and(|(id, _)| stop.shown() == Shown::Container(id));
            // The text field marks its selection on the selection background.
            let band = if Some(stop.shown()) == self.current && !editing {
                Some(theme.selection)
            } else {
                focused.then_some(theme.current_line)
            };
            if let Some(band) = band {
                buf.set_style(row, Style::new().bg(band));
            }
            if focused {
                buf.set_string(row.x, row.y, FOCUS_MARKER, Style::new().fg(theme.focus));
            }
            let title = self.title(stop, content);
            // A collapsed project's marker stands before its count, as Todoist's does (D-67).
            let (marker, tasks) = match stop {
                Stop::Place(place) => (
                    self.view
                        .collapsed
                        .contains(&place.id)
                        .then(|| Icon::Collapsed.glyph(self.icons).to_owned()),
                    (place.open_tasks > 0).then(|| place.open_tasks.to_string()),
                ),
                Stop::View(_) => (None, None),
            };
            let count = marker
                .into_iter()
                .chain(tasks)
                .fold(String::new(), |count, part| count + " " + &part);
            let count = Span::styled(count, Style::new().fg(theme.dim_text));
            let count_width = u16::try_from(count.width()).unwrap_or(u16::MAX);
            let icon_area = Rect::new(title.icon, row.y, title.x.saturating_sub(title.icon), 1);
            Span::styled(
                super::stop_icon(stop).glyph(self.icons),
                Style::new().fg(super::stop_icon_color(theme, stop)),
            )
            .render(icon_area.intersection(row), buf);
            let right = if editing { 0 } else { count_width };
            let text = Rect::new(
                title.x,
                row.y,
                row.right().saturating_sub(title.x).saturating_sub(right),
                1,
            );
            Paragraph::new(self.title_line(stop))
                .scroll((0, u16::try_from(title.shift).unwrap_or(u16::MAX)))
                .render(text.intersection(row), buf);
            if !editing {
                let at = Rect::new(
                    row.right().saturating_sub(count_width),
                    row.y,
                    count_width,
                    1,
                );
                count.render(at.intersection(row), buf);
            }
            if band.is_some() {
                super::band_text(buf, row, theme.band_text);
            }
        }
    }
}

/// The part of the sidebar's `area` that lists the entries: all but the rule at its right.
fn content(area: Rect) -> Rect {
    Rect {
        width: area.width.saturating_sub(1),
        ..area
    }
}

/// The line each of `entries` is drawn on: a blank line sets off the Inbox, the views, the
/// projects in no area and each area with what it holds (D-45).
fn lines(entries: &[Stop]) -> Vec<usize> {
    #[derive(PartialEq)]
    enum Group {
        Inbox,
        Views,
        Projects,
        Area,
    }
    let mut line = 0;
    let mut group = None;
    let mut lines = Vec::with_capacity(entries.len());
    for &stop in entries {
        if stop.depth() == 0 {
            let this = match stop {
                Stop::Place(place) => match place.kind {
                    ItemKind::Inbox => Group::Inbox,
                    ItemKind::Area => Group::Area,
                    _ => Group::Projects,
                },
                Stop::View(_) => Group::Views,
            };
            if group
                .as_ref()
                .is_some_and(|group| this == Group::Area || *group != this)
            {
                line += 1;
            }
            group = Some(this);
        }
        lines.push(line);
        line += 1;
    }
    lines
}

#[cfg(test)]
mod tests {
    const TODAY: jiff::civil::Date = jiff::civil::date(2026, 10, 14);

    use super::*;
    use crate::{
        app::{Command, Message, Motion, Opened, TextEdit, update, view::View},
        domain::{Item, PlacementError},
        ui::BuiltIn,
    };

    /// Area `Work` holding project `Website` with 3 open tasks, holding project `Blog`, then
    /// project `Learn Rust`.
    fn state() -> Result<State, PlacementError> {
        let mut document = Document::new();
        let add = |document: &mut Document, parent, kind, title| {
            document.push(parent, Node::Item(Item::new(kind, title)))
        };
        let work = add(&mut document, None, ItemKind::Area, "Work")?;
        let website = add(&mut document, Some(work), ItemKind::Project, "Website")?;
        for title in ["a", "b", "c"] {
            add(&mut document, Some(website), ItemKind::Task, title)?;
        }
        add(&mut document, Some(website), ItemKind::Project, "Blog")?;
        add(&mut document, None, ItemKind::Project, "Learn Rust")?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Sidebar);
        Ok(state)
    }

    fn give(state: &mut State, command: Command) {
        update(
            state,
            Message::Command(command),
            jiff::civil::date(2026, 10, 14),
        );
    }

    /// Draws `state`'s sidebar `width` by `height` and returns its rows and the caret.
    fn draw(state: &State, width: u16, height: u16) -> (Vec<String>, Buffer, Option<Position>) {
        let theme = BuiltIn::default().colors();
        let view = state.sidebar().expect("the sidebar shows");
        let pane = || SidebarPane::new(state, view.clone(), &theme, IconSet::Plain);
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        let caret = pane().caret(buf.area);
        pane().render(buf.area, &mut buf);
        let rows = (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect())
            .collect();
        (rows, buf, caret)
    }

    // FR-VIEW-10, FR-UI-04, D-78
    #[test]
    fn draws_the_groups_with_counts_and_marks_the_focus_while_it_has_the_keys()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        for motion in [crate::app::Focus::Last, crate::app::Focus::Previous] {
            give(&mut state, Command::Focus(motion));
        }
        let (rows, buf, caret) = draw(&state, 20, 14);
        assert_eq!(
            rows,
            [
                " ⊔ Inbox           │",
                "                   │",
                " ☼ Today           │",
                " » Upcoming        │",
                " ∷ All             │",
                " ▤ Logbook         │",
                " ✕ Trash           │",
                "                   │",
                " # Learn Rust      │",
                "                   │",
                " ◆ WORK           3│",
                "▌# Website        3│",
                " # Blog            │",
                "                   │",
            ]
        );
        assert_eq!(caret, None, "nothing is edited");
        let line = |title: &str| {
            let at = rows.iter().position(|row| row.contains(title));
            at.and_then(|at| u16::try_from(at).ok()).unwrap_or(u16::MAX)
        };
        let (today, work, website) = (line("Today"), line("WORK"), line("Website"));
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(5, website)].bg, theme.current_line);
        assert_eq!(buf[(0, website)].fg, theme.focus);
        assert_eq!(
            (buf[(1, 0)].fg, buf[(3, 0)].fg, buf[(5, 0)].bg),
            (theme.inbox_icon, theme.band_text, theme.selection),
            "the Inbox, shown in the pane, its title in the band's text color"
        );
        let icons = [
            ("Today", theme.today_icon),
            ("Upcoming", theme.upcoming_icon),
            ("All", theme.all_icon),
            ("Logbook", theme.logbook_icon),
            ("Trash", theme.trash_icon),
        ];
        for (view, color) in icons {
            let at = line(view);
            assert_eq!(
                (buf[(1, at)].fg, buf[(3, at)].fg),
                (color, theme.text),
                "{view}"
            );
        }
        let (work_icon, work_title) = (&buf[(1, work)], &buf[(3, work)]);
        assert_eq!(
            (work_icon.fg, work_title.fg),
            (theme.headers, theme.headers)
        );
        assert!(work_title.modifier.contains(Modifier::BOLD), "an area");
        let rust = line("Learn Rust");
        assert_eq!(
            (buf[(1, rust)].fg, buf[(3, rust)].fg),
            (theme.primary, theme.text),
            "a project"
        );
        assert_eq!(buf[(3, website)].fg, theme.band_text, "on the band");
        assert_eq!(buf[(17, work)].fg, theme.dim_text, "a count");
        assert_eq!(buf[(19, 0)].fg, theme.rule);
        give(&mut state, Command::FocusRight);
        let (rows, buf, _) = draw(&state, 20, 14);
        assert!(
            rows[usize::from(website)].starts_with(" # Website"),
            "no marker: {rows:?}"
        );
        assert_ne!(buf[(5, website)].bg, theme.current_line);
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::Go(crate::app::Go::Into));
        let (_, buf, _) = draw(&state, 20, 14);
        assert_eq!(
            (
                buf[(1, website)].fg,
                buf[(5, website)].fg,
                buf[(5, website)].bg
            ),
            (theme.primary, theme.band_text, theme.selection),
            "the container shown keeps its colors on the selection background"
        );
        assert_ne!(buf[(5, 0)].bg, theme.selection);
        give(&mut state, Command::Go(crate::app::Go::View(View::Today)));
        let (_, buf, _) = draw(&state, 20, 14);
        assert_eq!(buf[(5, today)].bg, theme.selection, "the view shown");
        Ok(())
    }

    // FR-VIEW-10, D-67, D-78
    #[test]
    fn a_collapsed_area_shows_a_marker_before_its_count() -> Result<(), PlacementError> {
        let mut state = state()?;
        for motion in [
            crate::app::Focus::Last,
            crate::app::Focus::Previous,
            crate::app::Focus::Previous,
        ] {
            give(&mut state, Command::Focus(motion));
        }
        give(&mut state, Command::Collapse);
        let (rows, buf, _) = draw(&state, 20, 11);
        assert_eq!(
            rows.get(8..11),
            Some(
                &[
                    " # Learn Rust      │".to_owned(),
                    "                   │".to_owned(),
                    "▌◆ WORK         ▸ 3│".to_owned(),
                ][..]
            )
        );
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(15, 10)].fg, theme.dim_text);
        give(&mut state, Command::RenameContainer);
        let (rows, _, _) = draw(&state, 20, 11);
        assert_eq!(rows[10], "▌◆ Work            │", "edited as it is written");
        Ok(())
    }

    // FR-VIEW-10
    #[test]
    fn scrolls_to_keep_the_focus_in_view() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::Focus(crate::app::Focus::Last));
        let (rows, _, _) = draw(&state, 20, 2);
        assert_eq!(rows, [" # Website        3│", "▌# Blog            │"]);
        give(
            &mut state,
            Command::Focus(crate::app::Focus::PreviousSection),
        );
        let (rows, _, _) = draw(&state, 20, 2);
        assert_eq!(rows, ["                   │", "▌◆ WORK           3│"]);
        let mut new = State::new(Document::new(), Opened::Demo, TODAY);
        give(&mut new, Command::Sidebar);
        let (rows, _, _) = draw(&new, 20, 2);
        assert_eq!(rows, ["▌⊔ Inbox           │", "                   │"]);
        Ok(())
    }

    // FR-VIEW-10, FR-EDIT-02
    #[test]
    fn the_title_edited_shows_in_its_row_scrolled_to_keep_the_caret() -> Result<(), PlacementError>
    {
        let mut state = state()?;
        for motion in [crate::app::Focus::Last, crate::app::Focus::Previous] {
            give(&mut state, Command::Focus(motion));
        }
        give(&mut state, Command::RenameContainer);
        let (rows, _, caret) = draw(&state, 20, 3);
        assert_eq!(rows[2], "▌# Website         │", "no count while edited");
        assert_eq!(caret, Some(Position::new(10, 2)));
        let word = Message::Edit(crate::app::EditInput::Text(TextEdit::Move {
            motion: Motion::WordLeft,
            select: true,
        }));
        update(&mut state, word, jiff::civil::date(2026, 10, 14));
        let (_, buf, caret) = draw(&state, 20, 3);
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(4, 2)].bg, theme.selection, "the selected word");
        assert_eq!(caret, Some(Position::new(3, 2)));
        for c in " and its blog posts".chars() {
            let typed = Message::Edit(crate::app::EditInput::Text(TextEdit::Insert(c)));
            update(&mut state, typed, jiff::civil::date(2026, 10, 14));
        }
        let (rows, _, caret) = draw(&state, 20, 3);
        assert_eq!(rows[2], "▌#  its blog posts │", "scrolled sideways");
        assert_eq!(caret, Some(Position::new(18, 2)));
        let (_, _, caret) = draw(&state, 20, 1);
        assert_eq!(caret, Some(Position::new(18, 0)), "scrolled into view");
        let (_, _, caret) = draw(&state, 3, 3);
        assert_eq!(caret, None, "too narrow");
        Ok(())
    }

    #[test]
    fn takes_a_third_of_the_screen_up_to_28_columns() {
        assert_eq!((width(120), width(60), width(9)), (28, 20, 3));
    }
}
