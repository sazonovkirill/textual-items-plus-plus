//! The sidebar left of the pane: the areas and projects as a tree, each after its kind's icon
//! with its open tasks counted at the right, as the Navigator lists them, the entry of the
//! container shown on the selection background, the focused one marked while the sidebar has the
//! keys and the title being edited there as a text field in its row (FR-VIEW-10).

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::{Icon, IconSet, Theme};
use crate::{
    app::{Part, Place, SidebarView, State, TextField},
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
    /// The entry of the container shown, or of the closest one above it.
    current: Option<ItemId>,
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
        let current = state
            .path()
            .into_iter()
            .rev()
            .find(|&id| view.entries.iter().any(|place| place.id == id));
        Self {
            document: state.document(),
            editing: state
                .editing()
                .filter(|&(_, part, _)| view.keys && part == Part::Title)
                .map(|(id, _, field)| (id, field)),
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
        let (index, place) = self
            .view
            .entries
            .iter()
            .enumerate()
            .find(|(_, place)| place.id == id)?;
        let row = index.checked_sub(offset)?;
        let y = content.y
            + u16::try_from(row)
                .ok()
                .filter(|&row| row < content.height)?;
        let title = self.title(place, content);
        let caret = Span::raw(field.text().get(..field.caret()).unwrap_or_default()).width();
        let x = usize::from(title.x) + caret - title.shift;
        let x = u16::try_from(x).ok().filter(|&x| x < content.right())?;
        Some(Position::new(x, y))
    }

    /// The first entry shown in `content`, so that the focused one is in view.
    fn offset(&self, content: Rect) -> usize {
        self.view.focused.map_or(0, |focused| {
            (focused + 1).saturating_sub(usize::from(content.height))
        })
    }

    /// Where the title of `place` starts in `content`, and how far it is scrolled sideways to
    /// keep the caret in view while it is edited.
    fn title(&self, place: &Place, content: Rect) -> Title {
        let column = |columns: usize| u16::try_from(columns).unwrap_or(u16::MAX);
        let icon_x = content.x.saturating_add(column(place.depth * 2 + 1));
        let glyph = Span::raw(icon(place.kind).glyph(self.icons)).width() + 1;
        let x = icon_x.saturating_add(column(glyph));
        let shift = match self.editing {
            Some((id, field)) if id == place.id => {
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

    /// How the title of `place` is drawn: in its kind's color, areas bold as headers.
    fn title_style(&self, place: &Place) -> Style {
        let style = Style::new().fg(self.theme.kind(place.kind));
        if place.kind == ItemKind::Area {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    /// The line of the title of `place`: its text, or the text edited with the selected part in
    /// the selection colors.
    fn title_line(&self, place: &Place) -> Line<'a> {
        let style = self.title_style(place);
        match self.editing {
            Some((id, field)) if id == place.id => {
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
            _ => {
                let title = self
                    .document
                    .node(place.id)
                    .and_then(Node::item)
                    .map_or("", |item| item.title.as_str());
                Line::styled(title, style)
            }
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
        if self.view.entries.is_empty() {
            Line::styled(" No projects", Style::new().fg(theme.dim_text)).render(content, buf);
            return;
        }
        let offset = self.offset(content);
        let entries = self.view.entries.iter().enumerate().skip(offset);
        for ((index, place), row) in entries.zip(content.rows()) {
            let focused = self.view.keys && Some(index) == self.view.focused;
            let editing = self.editing.is_some_and(|(id, _)| id == place.id);
            // The text field marks its selection on the selection background.
            if Some(place.id) == self.current && !editing {
                buf.set_style(row, Style::new().bg(theme.selection));
            } else if focused {
                buf.set_style(row, Style::new().bg(theme.current_line));
            }
            if focused {
                buf.set_string(row.x, row.y, FOCUS_MARKER, Style::new().fg(theme.focus));
            }
            let title = self.title(place, content);
            let count = match place.open_tasks {
                0 => Span::raw(""),
                count => Span::styled(format!(" {count}"), Style::new().fg(theme.dim_text)),
            };
            let count_width = u16::try_from(count.width()).unwrap_or(u16::MAX);
            let icon_area = Rect::new(title.icon, row.y, title.x.saturating_sub(title.icon), 1);
            Span::styled(icon(place.kind).glyph(self.icons), self.title_style(place))
                .render(icon_area.intersection(row), buf);
            let right = if editing { 0 } else { count_width };
            let text = Rect::new(
                title.x,
                row.y,
                row.right().saturating_sub(title.x).saturating_sub(right),
                1,
            );
            Paragraph::new(self.title_line(place))
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

/// The icon of an entry of `kind`.
fn icon(kind: ItemKind) -> Icon {
    match kind {
        ItemKind::Area => Icon::Area,
        _ => Icon::Project,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{Command, Message, Motion, Opened, TextEdit, update},
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
        let mut state = State::new(document, Opened::Demo);
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

    // FR-VIEW-10, FR-UI-04
    #[test]
    fn draws_the_tree_with_counts_and_marks_the_focus_while_it_has_the_keys()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::Focus(crate::app::Focus::Next));
        let (rows, buf, caret) = draw(&state, 20, 5);
        assert_eq!(
            rows,
            [
                " ◆ Work           3│",
                "▌  ○ Website      3│",
                "     ○ Blog        │",
                " ○ Learn Rust      │",
                "                   │",
            ]
        );
        assert_eq!(caret, None, "nothing is edited");
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(5, 1)].bg, theme.current_line);
        assert_eq!(buf[(0, 1)].fg, theme.focus);
        assert_eq!(buf[(3, 0)].fg, theme.headers, "an area");
        assert_eq!(buf[(7, 2)].fg, theme.primary, "a project");
        assert_eq!(buf[(17, 0)].fg, theme.dim_text, "a count");
        assert_eq!(buf[(19, 0)].fg, theme.rule);
        give(&mut state, Command::FocusPane);
        let (rows, buf, _) = draw(&state, 20, 5);
        assert!(rows[1].starts_with("   ○ Website"), "no marker: {rows:?}");
        assert_ne!(buf[(5, 1)].bg, theme.current_line);
        give(&mut state, Command::FocusSidebar);
        give(&mut state, Command::Go(crate::app::Go::Into));
        let (_, buf, _) = draw(&state, 20, 5);
        assert_eq!(
            (buf[(5, 1)].fg, buf[(5, 1)].bg),
            (theme.primary, theme.selection),
            "the container shown keeps its kind's color on the selection background"
        );
        Ok(())
    }

    // FR-VIEW-10
    #[test]
    fn scrolls_to_keep_the_focus_in_view_and_says_when_it_is_empty() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::Focus(crate::app::Focus::Last));
        let (rows, _, _) = draw(&state, 20, 2);
        assert_eq!(rows, ["     ○ Blog        │", "▌○ Learn Rust      │"]);
        let mut empty = State::new(Document::new(), Opened::Demo);
        give(&mut empty, Command::Sidebar);
        let (rows, _, _) = draw(&empty, 20, 2);
        assert_eq!(rows, [" No projects       │", "                   │"]);
        Ok(())
    }

    // FR-VIEW-10, FR-EDIT-02
    #[test]
    fn the_title_edited_shows_in_its_row_scrolled_to_keep_the_caret() -> Result<(), PlacementError>
    {
        let mut state = state()?;
        give(&mut state, Command::Focus(crate::app::Focus::Next));
        give(&mut state, Command::RenameContainer);
        let (rows, _, caret) = draw(&state, 20, 3);
        assert_eq!(rows[1], "▌  ○ Website       │", "no count while edited");
        assert_eq!(caret, Some(Position::new(12, 1)));
        let word = Message::Edit(crate::app::EditInput::Text(TextEdit::Move {
            motion: Motion::WordLeft,
            select: true,
        }));
        update(&mut state, word, jiff::civil::date(2026, 10, 14));
        let (_, buf, caret) = draw(&state, 20, 3);
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(6, 1)].bg, theme.selection, "the selected word");
        assert_eq!(caret, Some(Position::new(5, 1)));
        for c in " and its blog".chars() {
            let typed = Message::Edit(crate::app::EditInput::Text(TextEdit::Insert(c)));
            update(&mut state, typed, jiff::civil::date(2026, 10, 14));
        }
        let (rows, _, caret) = draw(&state, 20, 3);
        assert_eq!(rows[1], "▌  ○  and its blog │", "scrolled sideways");
        assert_eq!(caret, Some(Position::new(18, 1)));
        let (_, _, caret) = draw(&state, 20, 1);
        assert_eq!(caret, Some(Position::new(18, 0)), "scrolled into view");
        let (_, _, caret) = draw(&state, 5, 3);
        assert_eq!(caret, None, "too narrow");
        Ok(())
    }

    #[test]
    fn takes_a_third_of_the_screen_up_to_28_columns() {
        assert_eq!((width(120), width(60), width(9)), (28, 20, 3));
    }
}
