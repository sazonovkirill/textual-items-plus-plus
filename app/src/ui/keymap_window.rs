//! The keymap window (FR-KEY-08): every key binding as a cheat sheet over most of the screen,
//! each group under its name, the groups laid out in columns as wide as the window takes, and
//! the keys that scroll it when they do not fit.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Clear, Widget},
};

use super::{
    Theme,
    attachments::Scroll,
    keymap::{self, Key},
    shortcuts::{self, Listing, Row},
};

/// What a key of the open window does: close it, or scroll it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeymapKey {
    /// Close the window.
    Close,
    /// Scroll the columns, all together.
    Scroll(Scroll),
}

/// The keys of the open window, as the attachments panel scrolls (keybindings.md §4).
pub(super) const KEYS: &[(Key, KeymapKey)] = &[
    ((KeyModifiers::NONE, KeyCode::Esc), KeymapKey::Close),
    (
        (KeyModifiers::NONE, KeyCode::Char('j')),
        KeymapKey::Scroll(Scroll::Lines(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Down),
        KeymapKey::Scroll(Scroll::Lines(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('k')),
        KeymapKey::Scroll(Scroll::Lines(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Up),
        KeymapKey::Scroll(Scroll::Lines(-1)),
    ),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('d')),
        KeymapKey::Scroll(Scroll::Pages(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::PageDown),
        KeymapKey::Scroll(Scroll::Pages(1)),
    ),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('u')),
        KeymapKey::Scroll(Scroll::Pages(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::PageUp),
        KeymapKey::Scroll(Scroll::Pages(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Home),
        KeymapKey::Scroll(Scroll::Top),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('G')),
        KeymapKey::Scroll(Scroll::Bottom),
    ),
    (
        (KeyModifiers::NONE, KeyCode::End),
        KeymapKey::Scroll(Scroll::Bottom),
    ),
];

/// What `event` does in the open window. Only key presses count.
pub(super) fn keymap_input(event: &Event) -> Option<KeymapKey> {
    let key = keymap::key_press(event)?;
    KEYS.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
}

/// The fewest columns a column of the window takes across.
const COLUMN: u16 = 32;

/// The columns between two columns.
const GAP: u16 = 2;

/// The window: titled Keymap in the accent color, its keys named on its bottom edge, listing
/// `groups` in columns, scrolled.
pub(super) struct KeymapWindow<'a> {
    groups: &'a [Listing],
    theme: &'a Theme,
    scroll: usize,
}

impl<'a> KeymapWindow<'a> {
    /// The window listing `groups` from the top, drawn in `theme`.
    pub(super) fn new(groups: &'a [Listing], theme: &'a Theme) -> Self {
        Self {
            groups,
            theme,
            scroll: 0,
        }
    }

    /// The window with its columns scrolled `scroll` lines down.
    pub(super) fn scrolled(self, scroll: usize) -> Self {
        Self { scroll, ..self }
    }

    /// How far the columns scroll when the window is drawn over `area`, and how many lines a
    /// page scrolls: the lines shown, at least one.
    pub(super) fn scroll_range(&self, area: Rect) -> (usize, usize) {
        let inner = self.frame().inner(placed(area));
        let height = usize::from(inner.height);
        let tallest = self.columns(inner.width).iter().map(Vec::len).max();
        (
            tallest.unwrap_or_default().saturating_sub(height),
            height.max(1),
        )
    }

    /// The window's frame.
    fn frame(&self) -> Block<'static> {
        let accent = Style::new().fg(self.theme.accent);
        Block::bordered()
            .style(Style::new().bg(self.theme.background).fg(self.theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(" Keymap ")
            .title_bottom(Line::from(" j/k scroll · Esc close ").right_aligned())
    }

    /// The columns of a window `width` columns wide inside its frame: as many as fit at least
    /// [`COLUMN`] wide, each group whole in one column, a blank row between two groups, the
    /// columns no taller than it takes for the groups to go in them in order. A group taller
    /// than that has a column of its own.
    fn columns(&self, width: u16) -> Vec<Vec<Row>> {
        let count = ((width + GAP) / (COLUMN + GAP)).max(1);
        let column = (width.saturating_sub(GAP * (count - 1))) / count;
        let groups: Vec<Vec<Row>> = self
            .groups
            .iter()
            .map(|group| shortcuts::rows(std::slice::from_ref(group), column, self.theme))
            .collect();
        let total = groups.iter().map(Vec::len).sum::<usize>() + groups.len().saturating_sub(1);
        let mut height = total.div_ceil(usize::from(count));
        loop {
            let columns = packed(&groups, height);
            if columns.len() <= usize::from(count) {
                return columns;
            }
            height += 1;
        }
    }
}

/// `groups` in columns no taller than `height` but for a group taller than that, each group
/// whole, in order, a blank row between two groups in one column.
fn packed(groups: &[Vec<Row>], height: usize) -> Vec<Vec<Row>> {
    let mut columns: Vec<Vec<Row>> = Vec::new();
    for group in groups {
        match columns.last_mut() {
            Some(column) if column.len() + 1 + group.len() <= height => {
                column.push(Row::default());
                column.extend(group.iter().cloned());
            }
            _ => columns.push(group.clone()),
        }
    }
    columns
}

/// The window's place in `area`: nine tenths of it each way, centered.
fn placed(area: Rect) -> Rect {
    area.centered(Constraint::Percentage(90), Constraint::Percentage(90))
}

impl Widget for KeymapWindow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = placed(area);
        Clear.render(area, buf);
        let frame = self.frame();
        let inner = frame.inner(area);
        frame.render(area, buf);
        let columns = self.columns(inner.width);
        let count = u16::try_from(columns.len()).unwrap_or(u16::MAX).max(1);
        let width = (inner.width.saturating_sub(GAP * (count - 1))) / count;
        for (index, column) in (0..).zip(columns) {
            let x = inner.x + index * (width + GAP);
            let rows = Rect::new(x, inner.y, width, inner.height).rows();
            for (row, area) in column.into_iter().skip(self.scroll).zip(rows) {
                shortcuts::render_row(row, area, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::BuiltIn;

    /// Groups named `a`, `b` and so on, each with `sizes` entries.
    fn groups(sizes: &[usize]) -> Vec<Listing> {
        const NAMES: [&str; 4] = ["a", "b", "c", "d"];
        NAMES
            .into_iter()
            .zip(sizes)
            .map(|(name, &size)| {
                let entries = (0..size)
                    .map(|entry| (format!("{name}{entry}"), "x".to_owned()))
                    .collect();
                (name, entries)
            })
            .collect()
    }

    /// The window over a `width` by `height` area, scrolled `scroll` lines, as rows of text.
    fn drawn(groups: &[Listing], (width, height): (u16, u16), scroll: usize) -> Vec<String> {
        let theme = BuiltIn::default().colors();
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        KeymapWindow::new(groups, &theme)
            .scrolled(scroll)
            .render(area, &mut buf);
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    // FR-KEY-08
    #[test]
    fn the_groups_go_in_balanced_columns_each_whole_over_nine_tenths_of_the_screen() {
        let rows = drawn(&groups(&[2, 2, 3]), (100, 10), 0);
        assert_eq!(
            rows,
            [
                "",
                "     ┌ Keymap ────────────────────────────────────────────────────────────────────────────────┐",
                "     │a                                            c                                          │",
                "     │ a0                                       x   c0                                       x│",
                "     │ a1                                       x   c1                                       x│",
                "     │                                              c2                                       x│",
                "     │b                                                                                       │",
                "     │ b0                                       x                                             │",
                "     │ b1                                       x                                             │",
                "     └──────────────────────────────────────────────────────────────── j/k scroll · Esc close ┘",
            ]
        );
    }

    // FR-KEY-08
    #[test]
    fn a_window_too_short_scrolls_its_columns_together() {
        let listed = groups(&[6, 6]);
        let theme = BuiltIn::default().colors();
        let window = KeymapWindow::new(&listed, &theme);
        assert_eq!(
            window.scroll_range(Rect::new(0, 0, 100, 6)),
            (3, 4),
            "seven rows in each column, four shown"
        );
        let rows = drawn(&listed, (100, 6), 3);
        assert!(rows[1].starts_with("     │ a2 "), "{rows:?}");
        assert!(rows[1].contains(" b2 "), "{rows:?}");
    }

    // FR-KEY-08
    #[test]
    fn a_narrow_window_has_one_column_and_columns_grow_to_hold_the_tallest_group() {
        let theme = BuiltIn::default().colors();
        let listed = groups(&[1, 9, 1]);
        let window = KeymapWindow::new(&listed, &theme);
        assert_eq!(window.columns(30).len(), 1);
        let columns = window.columns(80);
        let lengths: Vec<_> = columns.iter().map(Vec::len).collect();
        assert_eq!(
            lengths,
            [13, 2],
            "a and b share a column as tall as b needs"
        );
    }

    // FR-KEY-08
    #[test]
    fn its_keys_close_and_scroll_it() {
        let none = KeyModifiers::NONE;
        let press = |code, modifiers| Event::Key(crossterm::event::KeyEvent::new(code, modifiers));
        for (code, modifiers, key) in [
            (KeyCode::Esc, none, KeymapKey::Close),
            (
                KeyCode::Char('j'),
                none,
                KeymapKey::Scroll(Scroll::Lines(1)),
            ),
            (
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
                KeymapKey::Scroll(Scroll::Pages(-1)),
            ),
            (
                KeyCode::Char('G'),
                KeyModifiers::SHIFT,
                KeymapKey::Scroll(Scroll::Bottom),
            ),
        ] {
            assert_eq!(keymap_input(&press(code, modifiers)), Some(key), "{code:?}");
        }
        assert_eq!(keymap_input(&press(KeyCode::Char('x'), none)), None);
    }
}
