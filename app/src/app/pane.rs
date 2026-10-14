//! A pane: one column showing a container or a view, with its own focus and selection, history,
//! completed items shown or not and tag filter (FR-PANE-01). Hides how it keeps its focus on a row
//! and gives way when what it shows can no longer be shown.

use jiff::civil::Date;

use super::{
    Selection,
    container::{self, Row},
    focus::{self, Focus},
    tags,
    view::{self, Group, Shown},
    visits::{Visit, Visits},
};
use crate::domain::{Document, ItemId};

/// What one pane shows and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pane {
    pub(super) shown: Shown,
    pub(super) selection: Selection,
    /// Whether the pane shows completed items (FR-VIEW-11).
    pub(super) show_completed: bool,
    pub(super) visits: Visits,
    /// The tags every row the pane shows carries, none for every row (FR-TAG-04).
    pub(super) filter: Vec<String>,
}

impl Pane {
    /// A pane showing `shown` in `document` on `today`, its first row focused, with no history.
    pub(super) fn new(
        document: &Document,
        today: Date,
        shown: Shown,
        show_completed: bool,
    ) -> Self {
        let mut pane = Self {
            shown,
            selection: Selection::default(),
            show_completed,
            visits: Visits::default(),
            filter: Vec::new(),
        };
        pane.move_focus(document, today, Focus::First);
        pane
    }

    /// The container shown, unless the pane shows a view.
    pub(super) fn container(&self) -> Option<ItemId> {
        match self.shown {
            Shown::Container(id) => Some(id),
            Shown::View(_) => None,
        }
    }

    /// The containers from the top level down to the one shown; none for a view.
    pub(super) fn path(&self, document: &Document) -> Vec<ItemId> {
        self.container()
            .map(|id| container::path(document, id))
            .unwrap_or_default()
    }

    /// The rows the pane lists in `document` on `today`, in order, those the tag filter keeps.
    pub(super) fn rows(&self, document: &Document, today: Date) -> Vec<Row> {
        listed(
            document,
            (self.shown, today),
            self.show_completed,
            &self.filter,
        )
    }

    /// The groups of the view shown, each under its heading, with the rows the tag filter keeps;
    /// none for a container.
    pub(super) fn groups(&self, document: &Document, today: Date) -> Vec<Group> {
        let Shown::View(shown) = self.shown else {
            return Vec::new();
        };
        view::groups(document, shown, today, self.show_completed)
            .into_iter()
            .filter_map(|group| {
                let rows = tags::filtered(document, group.rows, &self.filter);
                (!rows.is_empty()).then_some(Group { rows, ..group })
            })
            .collect()
    }

    /// Moves the focus as `motion` says, keeping what is picked.
    pub(super) fn move_focus(&mut self, document: &Document, today: Date, motion: Focus) {
        let target = focus::target(
            &self.rows(document, today),
            self.selection.focused(),
            motion,
        );
        self.selection.focus(target);
    }

    /// Where the pane is.
    pub(super) fn visit(&self) -> Visit {
        Visit {
            shown: self.shown,
            focused: self.selection.focused(),
        }
    }

    /// Shows `shown`, focused on `focused` or its first row, so that Back returns to where the
    /// pane was. Showing what is shown changes nothing.
    pub(super) fn show(
        &mut self,
        document: &Document,
        today: Date,
        shown: Shown,
        focused: Option<ItemId>,
    ) {
        if shown != self.shown {
            self.visits.leave(self.visit());
            self.arrive(document, today, Visit { shown, focused });
        }
    }

    /// Shows what `visit` showed, focused on its item when that is a row there and on the first
    /// row otherwise, with nothing picked and no tag filter.
    pub(super) fn arrive(&mut self, document: &Document, today: Date, visit: Visit) {
        self.selection.clear();
        self.filter.clear();
        self.shown = visit.shown;
        let rows = self.rows(document, today);
        let focused = visit
            .focused
            .filter(|&id| rows.iter().any(|row| row.id() == id))
            .or_else(|| rows.first().map(|row| row.id()));
        self.selection.focus(focused);
    }

    /// Goes back to the latest place visited before that can be shown, if any (FR-NAV-04).
    pub(super) fn back(&mut self, document: &Document, today: Date) {
        let current = self.visit();
        if let Some(visit) = self
            .visits
            .back(current, |shown| view::opens(document, shown))
        {
            self.arrive(document, today, visit);
        }
    }

    /// Goes forward again to where Back left, as [`back`](Self::back) goes back.
    pub(super) fn forward(&mut self, document: &Document, today: Date) {
        let current = self.visit();
        if let Some(visit) = self
            .visits
            .forward(current, |shown| view::opens(document, shown))
        {
            self.arrive(document, today, visit);
        }
    }

    /// Keeps the focus on a row now that the rows `before` have changed, and when the container
    /// shown no longer opens, having gone to the Trash or into a completed item, shows the
    /// nearest one above it that does, else the Inbox, focused where it was.
    pub(super) fn settle(&mut self, document: &Document, today: Date, before: &[Row]) {
        if !view::opens(document, self.shown) {
            let path = self.path(document);
            let visit = path
                .array_windows()
                .rev()
                .find(|&&[parent, _]| container::opens(document, parent))
                .map_or(
                    Visit {
                        shown: Shown::Container(document.inbox()),
                        focused: None,
                    },
                    |&[parent, child]| Visit {
                        shown: Shown::Container(parent),
                        focused: Some(child),
                    },
                );
            self.arrive(document, today, visit);
            return;
        }
        let rows = self.rows(document, today);
        let focused = focus::kept(before, &rows, self.selection.focused());
        if focused != self.selection.focused() {
            self.selection.focus(focused);
        }
        self.selection.keep(&rows, Some);
    }

    /// Finds again in `document`, read again from the file, what the pane showed, its focus, what
    /// is picked and its history, as `nearest` and `exact` find each item by its text path. The
    /// tag filter stays while the container shown is found again as it was.
    pub(super) fn reload(
        &mut self,
        document: &Document,
        today: Date,
        nearest: impl Fn(ItemId) -> Option<ItemId>,
        exact: impl Fn(ItemId) -> Option<ItemId>,
    ) {
        let opens = |id: ItemId| container::opens(document, id).then_some(id);
        if let Shown::Container(id) = self.shown {
            let kept = exact(id).and_then(opens);
            if kept.is_none() {
                self.filter.clear();
            }
            self.shown = Shown::Container(
                kept.or_else(|| nearest(id).and_then(opens))
                    .unwrap_or_else(|| document.inbox()),
            );
        }
        let focused = self.selection.focused();
        let rows = self.rows(document, today);
        let row = |id: ItemId| rows.iter().any(|row| row.id() == id).then_some(id);
        let focus = focused
            .and_then(&exact)
            .and_then(row)
            .or_else(|| focused.and_then(&nearest).and_then(row))
            .or_else(|| rows.first().map(|row| row.id()));
        self.selection.focus(focus);
        self.selection.keep(&rows, &exact);
        self.visits.rematch(|id| nearest(id).and_then(opens), exact);
    }
}

/// The rows `shown` lists in `document` on the date it is shown, completed items among them when
/// `show_completed`, those carrying every tag of `filter`.
pub(super) fn listed(
    document: &Document,
    (shown, today): (Shown, Date),
    show_completed: bool,
    filter: &[String],
) -> Vec<Row> {
    tags::filtered(
        document,
        view::rows(document, shown, today, show_completed),
        filter,
    )
}
