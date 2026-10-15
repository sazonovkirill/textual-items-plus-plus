//! A pane: one column showing a container or a view, with its own focus and selection, history,
//! completed items shown or not, tag filter and width (FR-PANE-01). Hides how it keeps its focus
//! on a row and gives way when what it shows can no longer be shown.

use std::{cell::RefCell, rc::Rc};

use jiff::civil::Date;

use super::{
    AttachmentsView, Selection, TextField,
    attachments::{self, Panel},
    container::{self, Row},
    focus::{self, Focus},
    search, tags,
    view::{self, Group, Shown, View},
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
    /// The words searched for, which the pane keeps while it shows other places (FR-SRCH-01).
    pub(super) query: TextField,
    /// The width the pane keeps, as the frontend measures it, once narrowed or widened; without
    /// one it shares what the others leave (FR-PANE-02).
    pub(super) width: Option<u16>,
    /// The attachments panel open in the pane, if any (FR-ATT-01).
    pub(super) panel: Option<Panel>,
    /// Whether its attachments panel, while open, covers the whole of it as the frontend last
    /// drew it, which keeps the keys from its list (D-112).
    pub(super) covered: bool,
    listing: Listing,
}

/// The rows and groups a pane listed last, with what they were listed from, so that they are
/// found once for each change rather than each time they are asked for (N-PERF-02). They are
/// asked for only of the document of the state holding the pane, whose revisions never repeat,
/// and a reload, which brings another document, forgets them. It only saves work, so every
/// listing is equal to every other.
#[derive(Debug, Clone, Default)]
struct Listing(RefCell<Option<Listed>>);

impl PartialEq for Listing {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Listing {}

/// What a pane lists and what from.
#[derive(Debug, Clone)]
struct Listed {
    revision: u64,
    shown: Shown,
    today: Date,
    show_completed: bool,
    query: String,
    filter: Vec<String>,
    rows: Rc<[Row]>,
    groups: Rc<[Group]>,
}

impl Pane {
    /// A pane showing what `visit` shows in `document` on `today`, focused as
    /// [`arrive`](Self::arrive) focuses, completed items shown when `show_completed`, with no
    /// history, no tag filter and no width of its own.
    pub(super) fn new(
        document: &Document,
        today: Date,
        visit: Visit,
        show_completed: bool,
    ) -> Self {
        let mut pane = Self {
            shown: visit.shown,
            selection: Selection::default(),
            show_completed,
            visits: Visits::default(),
            filter: Vec::new(),
            query: TextField::new(String::new()),
            width: None,
            panel: None,
            covered: false,
            listing: Listing::default(),
        };
        pane.arrive(document, today, visit);
        pane
    }

    /// The container shown, unless the pane shows a view.
    pub(super) fn container(&self) -> Option<ItemId> {
        match self.shown {
            Shown::Container(id) => Some(id),
            Shown::View(_) => None,
        }
    }

    /// The container shown when its title takes the focus as a row above the others (D-68).
    pub(super) fn heading(&self, document: &Document) -> Option<ItemId> {
        heading(document, self.shown)
    }

    /// The containers from the top level down to the one shown; none for a view.
    pub(super) fn path(&self, document: &Document) -> Vec<ItemId> {
        self.container()
            .map(|id| container::path(document, id))
            .unwrap_or_default()
    }

    /// The rows the pane lists in `document` on `today`, in order, those the tag filter keeps.
    pub(super) fn rows(&self, document: &Document, today: Date) -> Rc<[Row]> {
        self.listed(document, today, |listed| Rc::clone(&listed.rows))
    }

    /// The groups of the view shown, each under its heading, with the rows the tag filter keeps;
    /// none for a container.
    pub(super) fn groups(&self, document: &Document, today: Date) -> Rc<[Group]> {
        self.listed(document, today, |listed| Rc::clone(&listed.groups))
    }

    /// What `read` reads of what the pane lists in `document` on `today`, listed again only when
    /// the document or what the pane shows has changed since it was last listed.
    fn listed<T>(&self, document: &Document, today: Date, read: impl FnOnce(&Listed) -> T) -> T {
        let mut listing = self.listing.0.borrow_mut();
        let query = match self.shown {
            Shown::View(View::Search) => self.query.text(),
            _ => "",
        };
        let current = listing.as_ref().filter(|listed| {
            listed.revision == document.revision()
                && listed.shown == self.shown
                && listed.today == today
                && listed.show_completed == self.show_completed
                && listed.query == query
                && listed.filter == self.filter
        });
        if let Some(listed) = current {
            return read(listed);
        }
        let narrowed = listing.take_if(|listed| {
            listed.revision == document.revision()
                && listed.shown == Shown::View(View::Search)
                && self.shown == listed.shown
                && listed.show_completed == self.show_completed
                && listed.filter == self.filter
                && search::narrows(&listed.query, query)
        });
        if let Some(listed) = narrowed {
            let rows = search::refined(document, query, &listed.rows);
            let groups = if rows.is_empty() {
                Vec::new()
            } else {
                vec![Group {
                    heading: None,
                    rows: rows.clone(),
                }]
            };
            return read(listing.insert(Listed {
                query: query.to_owned(),
                rows: rows.into(),
                groups: groups.into(),
                ..listed
            }));
        }
        let (rows, groups) = match self.shown {
            Shown::Container(id) => (
                container::rows(document, id, self.show_completed),
                Vec::new(),
            ),
            Shown::View(shown) => {
                let groups = view::groups(document, shown, today, self.show_completed, query);
                let rows = groups.iter().flat_map(|group| group.rows.clone()).collect();
                let groups = groups
                    .into_iter()
                    .filter_map(|group| {
                        let rows = tags::filtered(document, group.rows, &self.filter);
                        (!rows.is_empty()).then_some(Group { rows, ..group })
                    })
                    .collect();
                (rows, groups)
            }
        };
        read(listing.insert(Listed {
            revision: document.revision(),
            shown: self.shown,
            today,
            show_completed: self.show_completed,
            query: query.to_owned(),
            filter: self.filter.clone(),
            rows: tags::filtered(document, rows, &self.filter).into(),
            groups: groups.into(),
        }))
    }

    /// Moves the focus as `motion` says, keeping what is picked, or in visual line mode among
    /// the rows alone, picking the rows from where the mode began to it (D-82).
    pub(super) fn move_focus(&mut self, document: &Document, today: Date, motion: Focus) {
        let rows = self.rows(document, today);
        let focused = self.selection.focused();
        if self.selection.visual() {
            let target = focus::target(&rows, None, focused, motion);
            self.selection.extend(&rows, target);
        } else {
            let target = focus::target(&rows, self.heading(document), focused, motion);
            self.selection.focus(target);
        }
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

    /// Shows what `visit` showed, focused on its item when that is a row there or the heading,
    /// and on the first row otherwise, else the heading, with nothing picked and no tag filter;
    /// the panel stays on its item as far as it [stays](attachments::stays) open there.
    pub(super) fn arrive(&mut self, document: &Document, today: Date, visit: Visit) {
        self.selection.clear();
        self.filter.clear();
        self.shown = visit.shown;
        self.keep_panel(document);
        let rows = self.rows(document, today);
        let heading = self.heading(document);
        let focused = visit
            .focused
            .filter(|&id| Some(id) == heading || rows.iter().any(|row| row.id() == id))
            .or_else(|| rows.first().map(|row| row.id()))
            .or(heading);
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

    /// Whether its list is hidden under its open panel, which then takes the keys in its place
    /// (D-112).
    pub(super) fn list_covered(&self) -> bool {
        self.covered && self.panel.is_some()
    }

    /// Closes the panel once it no longer [stays](attachments::stays) open on its item.
    pub(super) fn keep_panel(&mut self, document: &Document) {
        let shown = self.shown;
        self.panel = self
            .panel
            .filter(|panel| attachments::stays(document, panel.id(), shown));
    }

    /// Keeps the focus on a row now that the rows `before` have changed, and when the container
    /// shown no longer opens, having gone to the Trash or into a completed item, shows the
    /// nearest one above it that does, else the Inbox, focused where it was. The panel closes
    /// once its item is gone.
    pub(super) fn settle(&mut self, document: &Document, today: Date, before: &[Row]) {
        self.keep_panel(document);
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
        let focused = focus::kept(
            before,
            &rows,
            self.heading(document),
            self.selection.focused(),
        );
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
        self.listing = Listing::default();
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
        let heading = self.heading(document);
        let row = |id: ItemId| rows.iter().any(|row| row.id() == id).then_some(id);
        // The nearest of a row gone may be the container holding it: its first row, not its
        // heading, takes the focus then.
        let focus = focused
            .and_then(&exact)
            .and_then(|id| (Some(id) == heading).then_some(id).or_else(|| row(id)))
            .or_else(|| focused.and_then(&nearest).and_then(row))
            .or_else(|| rows.first().map(|row| row.id()))
            .or(heading);
        self.selection.focus(focus);
        self.selection.keep(&rows, &exact);
        self.visits.rematch(|id| nearest(id).and_then(opens), exact);
    }
}

/// The container `shown` when its title takes the focus as a row above the others: a project's
/// or a section's, so that the commands on the focused item reach the container shown, but not
/// the Inbox's, which takes none of them, nor a view's (owner, D-68).
pub(super) fn heading(document: &Document, shown: Shown) -> Option<ItemId> {
    match shown {
        Shown::Container(id) if id != document.inbox() => Some(id),
        Shown::Container(_) | Shown::View(_) => None,
    }
}

/// A pane as a frontend draws it.
#[derive(Debug, Clone, Copy)]
pub struct PaneView<'a> {
    pane: &'a Pane,
    document: &'a Document,
    today: Date,
    editing: Option<&'a TextField>,
}

impl<'a> PaneView<'a> {
    /// `pane` showing `document` on `today`, with `editing` the text edited in its panel, while
    /// its panel has the keys and something in it is edited.
    pub(super) fn new(
        pane: &'a Pane,
        document: &'a Document,
        today: Date,
        editing: Option<&'a TextField>,
    ) -> Self {
        Self {
            pane,
            document,
            today,
            editing,
        }
    }

    /// The pane's attachments panel, while it is open (FR-ATT-01).
    #[must_use]
    pub fn attachments(&self) -> Option<AttachmentsView<'a>> {
        attachments::view(self.document, self.pane.panel?, self.editing)
    }

    /// What the pane shows.
    #[must_use]
    pub fn shown(&self) -> Shown {
        self.pane.shown
    }

    /// The rows the pane lists, in order, those the tag filter keeps.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        self.shared_rows().to_vec()
    }

    /// As [`rows`](Self::rows), shared with the pane rather than copied, for a frontend that
    /// reads them for each frame (N-PERF-02).
    #[must_use]
    pub fn shared_rows(&self) -> Rc<[Row]> {
        self.pane.rows(self.document, self.today)
    }

    /// The groups of the view shown, each under its heading; none for a container. They are
    /// shared with the pane rather than copied (N-PERF-02).
    #[must_use]
    pub fn groups(&self) -> Rc<[Group]> {
        self.pane.groups(self.document, self.today)
    }

    /// The container shown when its title takes the focus as a row above the others (D-68).
    #[must_use]
    pub fn heading(&self) -> Option<ItemId> {
        self.pane.heading(self.document)
    }

    /// Where the Completed Items start among the rows of the container shown, listed last under
    /// a heading of their own (D-141); none while it lists none, and for a view.
    #[must_use]
    pub fn completed_items(&self) -> Option<usize> {
        self.pane.container()?;
        container::completed_from(self.document, &self.shared_rows())
    }

    /// The items the next command in the pane acts on.
    #[must_use]
    pub fn selection(&self) -> &'a Selection {
        &self.pane.selection
    }

    /// The tags the pane is filtered by, none when it is not (FR-TAG-04).
    #[must_use]
    pub fn tag_filter(&self) -> &'a [String] {
        &self.pane.filter
    }

    /// The words searched for, while the pane shows search results (FR-SRCH-01).
    #[must_use]
    pub fn query(&self) -> Option<&'a TextField> {
        (self.pane.shown == Shown::View(View::Search)).then_some(&self.pane.query)
    }

    /// The width the pane keeps, once narrowed or widened (FR-PANE-02).
    #[must_use]
    pub fn width(&self) -> Option<u16> {
        self.pane.width
    }
}

/// The rows `shown` lists in `document` on the date it is shown, for the words searched for,
/// completed items among them when `show_completed`, those carrying every tag of `filter`.
pub(super) fn listed(
    document: &Document,
    (shown, today, query): (Shown, Date, &str),
    show_completed: bool,
    filter: &[String],
) -> Vec<Row> {
    tags::filtered(
        document,
        view::rows(document, shown, today, show_completed, query),
        filter,
    )
}
