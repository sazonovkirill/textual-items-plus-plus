//! The breadcrumbs at the top of the pane: the path to the container it shows, after the home
//! icon, as Workflowy and taskpaper-terminal show it (FR-NAV-03,
//! [ui.md §1](../../../specs/reference/taskpaper-terminal/specs/ui.md#1-layout)).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use super::{Icon, IconSet, Theme};
use crate::domain::{Document, ItemId, Node};

/// What stands between two crumbs.
const SEPARATOR: &str = " › ";

/// What stands for the crumbs left out at the start.
const ELLIPSIS: &str = "…";

/// The breadcrumb line of a pane showing the last container of `path`.
pub(super) struct Crumbs<'a> {
    document: &'a Document,
    path: &'a [ItemId],
    theme: &'a Theme,
    icons: IconSet,
}

impl<'a> Crumbs<'a> {
    /// The crumbs of `path` in `document`, drawn in `theme` with `icons`.
    pub(super) fn new(
        document: &'a Document,
        path: &'a [ItemId],
        theme: &'a Theme,
        icons: IconSet,
    ) -> Self {
        Self {
            document,
            path,
            theme,
            icons,
        }
    }
}

impl Widget for Crumbs<'_> {
    /// Draws, after the focus gutter so the crumbs line up with the rows, the home icon and each
    /// container's title from the top down, joined by `›`, the earlier ones dimmed. When they do
    /// not fit, the earliest give way to `…`, and the last is cut at the right edge.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let titles: Vec<&str> = self
            .path
            .iter()
            .filter_map(|&id| Some(self.document.node(id).and_then(Node::item)?.title.as_str()))
            .collect();
        let dim = Style::new().fg(self.theme.dim_text);
        let home = format!("  {} ", Icon::Home.glyph(self.icons));
        let room = usize::from(area.width).saturating_sub(Span::raw(&home).width());
        let ellipsis = Span::raw(format!("{ELLIPSIS}{SEPARATOR}")).width();
        // The columns the crumbs from the one numbered `first` on take, with `…` before them.
        let needed = |first: usize| -> usize {
            let shown = titles.get(first..).unwrap_or_default();
            let separators = Span::raw(SEPARATOR).width() * shown.len().saturating_sub(1);
            let elided = if first > 0 { ellipsis } else { 0 };
            shown
                .iter()
                .map(|title| Span::raw(*title).width())
                .sum::<usize>()
                + separators
                + elided
        };
        let mut first = 0;
        while first + 1 < titles.len() && needed(first) > room {
            first += 1;
        }
        let mut spans = vec![Span::styled(home, dim)];
        if first > 0 {
            spans.extend([Span::styled(ELLIPSIS, dim), Span::styled(SEPARATOR, dim)]);
        }
        let shown = titles.get(first..).unwrap_or_default();
        for (index, title) in shown.iter().enumerate() {
            if index + 1 == shown.len() {
                spans.push(Span::styled(
                    *title,
                    Style::new().fg(self.theme.text).bold(),
                ));
            } else {
                spans.extend([Span::styled(*title, dim), Span::styled(SEPARATOR, dim)]);
            }
        }
        Line::from(spans).render(area, buf);
    }
}
