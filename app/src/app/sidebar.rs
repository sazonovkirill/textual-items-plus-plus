//! The sidebar: the Inbox, the views, and the areas and projects as a tree beside the pane,
//! where they are created, renamed, deleted and reordered with keys, and collapsed (FR-VIEW-10,
//! OQ-10). Hides which entry it keeps focused as the tree changes, which areas it collapses and
//! where its changes put items in the document.

use std::collections::BTreeSet;

use super::{
    Direction, Focus, Side, container,
    navigator::{self, Stop},
    view::Shown,
};
use crate::domain::{Destination, Document, ItemId, ItemKind, Node, Transaction};

/// Whether the sidebar shows and has the keys, and the entry it focuses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Sidebar {
    shown: bool,
    keys: bool,
    focused: Option<Shown>,
    /// Where the focused entry was in the tree, so that the focus stays there once it leaves.
    index: usize,
    /// The entry focused before a new one was added, to go back to when it is left empty.
    left: Option<Shown>,
    /// The areas whose projects are hidden, for as long as the app runs (D-67, D-75).
    collapsed: BTreeSet<ItemId>,
}

/// What a frontend draws of the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarView {
    /// The Inbox, the views, and the areas and projects in the order of the tree.
    pub entries: Vec<Stop>,
    /// The index of the focused entry, if any.
    pub focused: Option<usize>,
    /// Whether the sidebar has the keys, rather than the pane.
    pub keys: bool,
    /// The areas among the entries whose projects are hidden.
    pub collapsed: Vec<ItemId>,
}

impl Sidebar {
    /// A sidebar that shows without the keys, focused on `focused`, as at start (FR-VIEW-10).
    pub(super) fn open(focused: Shown) -> Self {
        Self {
            shown: true,
            focused: Some(focused),
            ..Self::default()
        }
    }

    /// Shows the sidebar with the keys, focused on `near` when given, or hides it when it has
    /// the keys, giving them back to the pane, or gives it the keys when it shows without them.
    pub(super) fn toggle(&mut self, near: Option<Shown>) {
        match (self.shown, self.keys) {
            (false, _) => {
                self.shown = true;
                self.keys = true;
                self.focused = near.or(self.focused);
            }
            (true, true) => {
                self.shown = false;
                self.keys = false;
            }
            (true, false) => self.keys = true,
        }
    }

    /// Gives the keys to the sidebar, or to the pane, while the sidebar shows.
    pub(super) fn focus(&mut self, keys: bool) {
        self.keys = self.shown && keys;
    }

    /// Whether the sidebar shows.
    pub(super) fn shows(&self) -> bool {
        self.shown
    }

    /// Whether the sidebar has the keys.
    pub(super) fn has_keys(&self) -> bool {
        self.keys
    }

    /// What the focused entry opens, if any.
    pub(super) fn focused(&self) -> Option<Shown> {
        self.focused
    }

    /// Focuses the new entry `id`, remembering the one focused before for [`finish`].
    ///
    /// [`finish`]: Self::finish
    pub(super) fn focus_new(&mut self, id: ItemId) {
        self.left = self.focused;
        self.focused = Some(Shown::Container(id));
    }

    /// Ends adding an entry: one left empty, and so not added, gives the focus back to the entry
    /// focused before.
    pub(super) fn finish(&mut self, kept: bool) {
        if let Some(left) = self.left.take().filter(|_| !kept) {
            self.focused = Some(left);
        }
    }

    /// Moves the focus among `entries` as `motion` says: to the next or previous entry, or the
    /// first or last.
    pub(super) fn move_focus(&mut self, entries: &[Stop], motion: Focus) {
        let Some(at) = self.at(entries) else {
            return;
        };
        let target = match motion {
            Focus::Next => (at + 1).min(entries.len() - 1),
            Focus::Previous => at.saturating_sub(1),
            Focus::First => 0,
            Focus::Last => entries.len() - 1,
        };
        if let Some(stop) = entries.get(target) {
            self.focused = Some(stop.shown());
        }
    }

    /// Keeps the focus on one of `entries` now that the tree may have changed: on the entry
    /// focused while it is there, else on the one now where it was, else on the last.
    pub(super) fn settle(&mut self, entries: &[Stop]) {
        if !self.shown {
            return;
        }
        let at = self.at(entries);
        self.index = at.unwrap_or(0);
        self.focused = at
            .and_then(|index| entries.get(index))
            .map(|stop| stop.shown());
    }

    /// What a frontend draws of the sidebar of `document`, while it shows.
    pub(super) fn view(&self, document: &Document) -> Option<SidebarView> {
        self.shown.then(|| {
            let entries = self.visible(document);
            SidebarView {
                focused: self.at(&entries),
                keys: self.keys,
                collapsed: entries
                    .iter()
                    .filter_map(|stop| match stop {
                        Stop::Place(place) => Some(place.id),
                        Stop::View(_) => None,
                    })
                    .filter(|&id| self.collapsed.contains(&id) && holds_projects(document, id))
                    .collect(),
                entries,
            }
        })
    }

    /// The entries the sidebar lists of `document`: [`entries`] less those inside a collapsed
    /// area.
    pub(super) fn visible(&self, document: &Document) -> Vec<Stop> {
        let mut hidden_below = None;
        entries(document)
            .into_iter()
            .filter(|&stop| {
                let depth = stop.depth();
                if hidden_below.is_some_and(|top| depth > top) {
                    return false;
                }
                hidden_below = match stop {
                    Stop::Place(place) if self.collapsed.contains(&place.id) => Some(depth),
                    _ => None,
                };
                true
            })
            .collect()
    }

    /// Collapses the focused area, or a project a file nests projects in, when it holds projects
    /// the sidebar lists, or expands it when it is collapsed (owner, D-67, D-75).
    pub(super) fn toggle_collapsed(&mut self, document: &Document) {
        let Some(Shown::Container(id)) = self.focused else {
            return;
        };
        if !self.collapsed.remove(&id) && holds_projects(document, id) {
            self.collapsed.insert(id);
        }
    }

    /// Expands the areas holding the focused entry, so that a project added or moved into a
    /// collapsed one shows where it went.
    pub(super) fn reveal(&mut self, document: &Document) {
        if let Some(Shown::Container(id)) = self.focused {
            for above in container::path(document, id) {
                if above != id {
                    self.collapsed.remove(&above);
                }
            }
        }
    }

    /// Keeps the focused entry, the one to go back to and the collapsed areas on the items
    /// `exact` finds them as in a document read again; a focused entry not found gives way to
    /// the one now where it was, and the others are dropped.
    pub(super) fn rematch(&mut self, exact: impl Fn(ItemId) -> Option<ItemId>) {
        let rematched = |shown| match shown {
            Shown::Container(id) => exact(id).map(Shown::Container),
            Shown::View(_) => Some(shown),
        };
        self.focused = self.focused.and_then(rematched);
        self.left = self.left.and_then(rematched);
        self.collapsed = self.collapsed.iter().filter_map(|&id| exact(id)).collect();
    }

    /// The index of the focused entry among `entries`, or of the one now where it was.
    fn at(&self, entries: &[Stop]) -> Option<usize> {
        let last = entries.len().checked_sub(1)?;
        let found = self
            .focused
            .and_then(|shown| entries.iter().position(|stop| stop.shown() == shown));
        Some(found.unwrap_or(self.index.min(last)))
    }
}

/// The entries of `document` as the sidebar lists them: the Navigator's with the Inbox and the
/// views first, then the projects in no area and then the areas, each followed by what it holds,
/// as Things 3's sidebar groups them. Within each group they keep the order of the file.
pub(super) fn entries(document: &Document) -> Vec<Stop> {
    let (mut first, mut projects, mut areas) = (Vec::new(), Vec::new(), Vec::new());
    let mut group = ItemKind::Inbox;
    for stop in navigator::stops(document)
        .into_iter()
        .map(|candidate| candidate.value)
    {
        let Stop::Place(place) = stop else {
            first.push(stop);
            continue;
        };
        if place.depth == 0 {
            group = place.kind;
        }
        match group {
            ItemKind::Inbox => first.push(stop),
            ItemKind::Area => areas.push(stop),
            _ => projects.push(stop),
        }
    }
    [first, projects, areas].concat()
}

/// The transaction that moves `id` one step `direction` among its siblings in the tree, past the
/// next entry of its kind listed beside it, since the sidebar lists the projects in no area apart
/// from the areas; an empty one when it is already at that end.
pub(super) fn shift(document: &Document, id: ItemId, direction: Direction) -> Transaction {
    let Some((siblings, index)) = siblings(document, id) else {
        return Transaction::default();
    };
    let alike =
        |other: ItemId| listed(document, other) && kind(document, other) == kind(document, id);
    let destination = match direction {
        Direction::Up => siblings
            .get(..index)
            .and_then(|earlier| earlier.iter().rfind(|&&other| alike(other)))
            .map(|&other| Destination::Before(other)),
        Direction::Down => siblings
            .get(index + 1..)
            .and_then(|later| later.iter().find(|&&other| alike(other)))
            .map(|&other| Destination::After(other)),
    };
    destination.map_or_else(Transaction::default, |to| {
        document
            .move_to(id, to)
            .expect("an entry moves beside a sibling, which holds what it holds")
    })
}

/// Where a new item of `kind` goes from the entry `focused` among `entries`, on `side` of it, as
/// its parent, `None` for the top level, and its index there. An area goes beside the area
/// holding the focus, or first among the areas from the Inbox, a view or a project in no area. A
/// project goes above or below the focused project, or the outermost project holding it, or first
/// in the focused area on either side, as a new task below a section heading is its first, and
/// from the Inbox or a view first among the projects in no area. With nothing focused either goes
/// last at the top level, and any other kind goes nowhere.
pub(super) fn place(
    document: &Document,
    entries: &[Stop],
    focused: Option<Shown>,
    kind: ItemKind,
    side: Side,
) -> Option<(Option<ItemId>, usize)> {
    let end = (None, document.top_level().len());
    let Some(focused) = focused else {
        return matches!(kind, ItemKind::Area | ItemKind::Project).then_some(end);
    };
    // The top level lists projects apart from areas, each kind in the order of the file, so the
    // first place there is the first of either kind. The views are listed with the Inbox.
    let focused = match focused {
        Shown::Container(id) if id != document.inbox() => id,
        _ => return matches!(kind, ItemKind::Area | ItemKind::Project).then_some((None, 0)),
    };
    let below = usize::from(side == Side::Below);
    match kind {
        ItemKind::Area => {
            let top = container::path(document, focused).first().copied()?;
            if self::kind(document, top) != Some(ItemKind::Area) {
                return Some((None, 0));
            }
            let (_, index) = document.position(top)?;
            Some((None, index + below))
        }
        // Above an area the sidebar lists another area's projects or those in no area, so a
        // project added there goes into it as from below.
        ItemKind::Project if self::kind(document, focused) == Some(ItemKind::Area) => {
            let at = entries
                .iter()
                .position(|stop| stop.shown() == Shown::Container(focused))?;
            match entries.get(at + 1).filter(|next| next.depth() > 0) {
                Some(&Stop::Place(first)) => document.position(first.id),
                // The views are listed at the top level, so no entry holds one.
                None | Some(Stop::View(_)) => {
                    Some((Some(focused), document.children(focused).len()))
                }
            }
        }
        // Beside the outermost project holding the focus, since v1 nests no project in another
        // (D-57), though a file may hold one.
        ItemKind::Project => {
            entries
                .iter()
                .find(|stop| stop.shown() == Shown::Container(focused))?;
            let outermost = container::path(document, focused)
                .into_iter()
                .find(|&id| self::kind(document, id) == Some(ItemKind::Project))?;
            let (parent, index) = document.position(outermost)?;
            Some((parent, index + below))
        }
        _ => None,
    }
}

/// The siblings of `id`, the top-level nodes for one at the top level, and its index among them.
fn siblings(document: &Document, id: ItemId) -> Option<(&[ItemId], usize)> {
    let (parent, index) = document.position(id)?;
    let siblings = parent.map_or(document.top_level(), |parent| document.children(parent));
    Some((siblings, index))
}

/// Whether the area or project `id` holds a project the sidebar lists.
fn holds_projects(document: &Document, id: ItemId) -> bool {
    document
        .children(id)
        .iter()
        .any(|&child| listed(document, child))
}

/// Whether `id` is an area or project the sidebar lists, given that its parent is listed.
fn listed(document: &Document, id: ItemId) -> bool {
    navigator::shown(document, id)
        .is_some_and(|item| matches!(item.kind, ItemKind::Area | ItemKind::Project))
}

/// The kind of the item `id`, if it is one.
fn kind(document: &Document, id: ItemId) -> Option<ItemKind> {
    document.node(id).and_then(Node::item).map(|item| item.kind)
}

/// The area that is `id` or holds it, if any.
pub(super) fn area(document: &Document, id: ItemId) -> Option<ItemId> {
    container::path(document, id)
        .into_iter()
        .find(|&above| kind(document, above) == Some(ItemKind::Area))
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::view::{REGISTRY, View};
    use crate::domain::{Item, PlacementError};
    use ItemKind::{Area, Project, Section, Task};

    fn add(document: &mut Document, parent: Option<ItemId>, kind: ItemKind, title: &str) -> ItemId {
        document
            .push(parent, Node::Item(Item::new(kind, title)))
            .expect("the test builds a valid hierarchy")
    }

    /// Area `Work` holding task `t`, section `S` and projects `Site`, holding project `Blog`, and
    /// `Docs`; project `Gone` in the Trash; project `Solo`; and the empty area `Home`.
    fn document() -> Document {
        let mut document = Document::new();
        let work = add(&mut document, None, Area, "Work");
        add(&mut document, Some(work), Task, "t");
        add(&mut document, Some(work), Section, "S");
        let site = add(&mut document, Some(work), Project, "Site");
        add(&mut document, Some(site), Project, "Blog");
        add(&mut document, Some(work), Project, "Docs");
        let mut gone = Item::new(Project, "Gone");
        gone.deleted = Some(date(2026, 10, 1));
        document
            .push(None, Node::Item(gone))
            .expect("a project goes at the top level");
        add(&mut document, None, Project, "Solo");
        add(&mut document, None, Area, "Home");
        document
    }

    fn find(document: &Document, title: &str) -> ItemId {
        let mut pending = document.top_level().to_vec();
        let mut found = None;
        while let Some(id) = pending.pop() {
            if document
                .node(id)
                .and_then(Node::item)
                .is_some_and(|it| it.title == title)
            {
                found = Some(id);
            }
            pending.extend(document.children(id));
        }
        found.expect("the test's document holds the title")
    }

    /// The entries of containers, each title indented two spaces per level.
    fn tree(document: &Document) -> Vec<String> {
        entries(document)
            .iter()
            .filter_map(|stop| {
                let Stop::Place(place) = stop else {
                    return None;
                };
                let title = document
                    .node(place.id)
                    .and_then(Node::item)
                    .map_or("", |it| &it.title);
                Some(format!("{}{title}", "  ".repeat(place.depth)))
            })
            .collect()
    }

    /// Applies the transaction `change` makes of `document`, checking that undoing it restores
    /// the document.
    fn apply(
        document: &mut Document,
        change: impl FnOnce(&Document) -> Transaction,
    ) -> Result<(), PlacementError> {
        let before = document.clone();
        let transaction = change(document);
        document.apply(&transaction)?;
        let mut undone = document.clone();
        undone.apply(&transaction.inverse())?;
        assert_eq!(undone, before, "undo restores the document");
        Ok(())
    }

    // FR-VIEW-10
    #[test]
    fn lists_the_inbox_and_the_views_then_the_projects_in_no_area_then_the_areas_without_the_trash()
    {
        let document = document();
        let views: Vec<Stop> = REGISTRY
            .iter()
            .map(|entry| Stop::View(entry.view))
            .collect();
        let listed = entries(&document);
        assert_eq!(listed.get(1..=views.len()), Some(&views[..]));
        assert_eq!(
            listed.get(3..=5),
            Some(&[View::All, View::Logbook, View::Trash].map(Stop::View)[..]),
            "the Logbook between All and the Trash (D-78)"
        );
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Solo", "Work", "  Site", "    Blog", "  Docs", "Home"
            ]
        );
    }

    // FR-VIEW-10, FR-ORG-06
    #[test]
    fn an_entry_changes_places_with_the_entry_of_its_kind_beside_it() -> Result<(), PlacementError>
    {
        let mut document = document();
        let docs = find(&document, "Docs");
        apply(&mut document, |d| shift(d, docs, Direction::Up))?;
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Solo", "Work", "  Docs", "  Site", "    Blog", "Home"
            ]
        );
        let up = shift(&document, docs, Direction::Up);
        assert!(
            up.ops().is_empty(),
            "the task and section above are no entries"
        );
        let solo = find(&document, "Solo");
        for direction in [Direction::Up, Direction::Down] {
            assert_eq!(
                shift(&document, solo, direction).ops(),
                [],
                "past the project in the Trash, only areas are beside it, listed apart"
            );
        }
        let work = find(&document, "Work");
        apply(&mut document, |d| shift(d, work, Direction::Down))?;
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Solo", "Home", "Work", "  Docs", "  Site", "    Blog"
            ],
            "past the project in no area, after the next area"
        );
        let home = find(&document, "Home");
        assert_eq!(shift(&document, home, Direction::Up).ops(), []);
        assert_eq!(shift(&document, document.inbox(), Direction::Up).ops(), []);
        Ok(())
    }

    /// Where a new `kind` goes from the entry titled `focused`, as the titles of the parent and
    /// of the sibling it goes before, if any.
    fn placed(
        document: &Document,
        focused: Option<&str>,
        kind: ItemKind,
        side: Side,
    ) -> Option<(String, String)> {
        let focused = focused.map(|title| Shown::Container(find(document, title)));
        let (parent, index) = place(document, &entries(document), focused, kind, side)?;
        let title = |id: Option<&ItemId>| {
            id.and_then(|&id| document.node(id).and_then(Node::item))
                .map_or(String::new(), |it| it.title.clone())
        };
        let siblings = parent.map_or(document.top_level(), |parent| document.children(parent));
        Some((title(parent.as_ref()), title(siblings.get(index))))
    }

    fn at(parent: &str, before: &str) -> (String, String) {
        (parent.to_owned(), before.to_owned())
    }

    // FR-VIEW-10, FR-EDIT-05, FR-ORG-06
    #[test]
    fn a_new_project_goes_beside_the_focused_project_or_first_in_the_focused_area() {
        let document = document();
        let new = |focused, side| placed(&document, Some(focused), Project, side);
        assert_eq!(new("Docs", Side::Above), Some(at("Work", "Docs")));
        assert_eq!(
            new("Docs", Side::Below),
            Some(at("Work", "")),
            "last in its area"
        );
        assert_eq!(
            new("Site", Side::Below),
            Some(at("Work", "Docs")),
            "beside Site, though it holds Blog, since v1 nests no project in another"
        );
        assert_eq!(
            new("Blog", Side::Above),
            Some(at("Work", "Site")),
            "beside the outermost project holding it"
        );
        assert_eq!(
            new("Work", Side::Below),
            Some(at("Work", "Site")),
            "first in Work"
        );
        assert_eq!(
            new("Home", Side::Below),
            Some(at("Home", "")),
            "into the empty area"
        );
        assert_eq!(
            new("Home", Side::Above),
            Some(at("Home", "")),
            "into the area from above too"
        );
        assert_eq!(new("Solo", Side::Below), Some(at("", "Home")));
        assert_eq!(
            placed(&document, None, Project, Side::Above),
            Some(at("", "")),
            "with nothing focused, last at the top level"
        );
        let gone = find(&document, "Gone");
        let entries = entries(&document);
        assert_eq!(
            place(
                &document,
                &entries,
                Some(Shown::Container(gone)),
                Project,
                Side::Below
            ),
            None,
            "from no entry"
        );
    }

    // FR-VIEW-10, FR-EDIT-05, FR-ORG-06
    #[test]
    fn a_new_area_goes_beside_the_top_level_item_holding_the_focus() {
        let document = document();
        let new = |focused, side| placed(&document, focused, Area, side);
        assert_eq!(new(Some("Blog"), Side::Below), Some(at("", "Gone")));
        assert_eq!(new(Some("Blog"), Side::Above), Some(at("", "Work")));
        assert_eq!(new(Some("Home"), Side::Below), Some(at("", "")));
        assert_eq!(new(None, Side::Below), Some(at("", "")));
        assert_eq!(
            new(Some("Solo"), Side::Above),
            Some(at("", "Work")),
            "first among the areas from a project in no area"
        );
        let entries = entries(&document);
        let inbox = Some(Shown::Container(document.inbox()));
        let today = Some(Shown::View(View::Today));
        for (kind, side) in [(Area, Side::Above), (Project, Side::Below)] {
            for from in [inbox, today] {
                assert_eq!(
                    place(&document, &entries, from, kind, side),
                    Some((None, 0)),
                    "first of its kind from the Inbox and the views listed with it"
                );
            }
        }
        assert_eq!(
            place(&document, &entries, inbox, Section, Side::Below),
            None
        );
        for kind in [Section, Task] {
            assert_eq!(placed(&document, Some("Work"), kind, Side::Below), None);
            assert_eq!(placed(&document, None, kind, Side::Below), None);
        }
    }

    /// The titles of the entries from the one focused in `sidebar`, after `motion`.
    fn moved(document: &Document, sidebar: &mut Sidebar, motion: Focus) -> Option<String> {
        let entries = entries(document);
        sidebar.move_focus(&entries, motion);
        sidebar.settle(&entries);
        match sidebar.focused()? {
            Shown::Container(id) => Some(document.node(id)?.item()?.title.clone()),
            Shown::View(view) => Some(view.title().to_owned()),
        }
    }

    // FR-VIEW-10
    #[test]
    fn the_focus_moves_through_the_tree() {
        let document = document();
        let mut sidebar = Sidebar::default();
        assert_eq!(sidebar.view(&document), None, "hidden at first");
        sidebar.focus(true);
        assert!(!sidebar.has_keys(), "a hidden sidebar takes no keys");
        sidebar.toggle(None);
        assert!(sidebar.has_keys());
        let views: Vec<&str> = entries(&document)
            .into_iter()
            .filter_map(|stop| match stop {
                Stop::View(view) => Some(view.title()),
                Stop::Place(_) => None,
            })
            .collect();
        let mut go = |motion| moved(&document, &mut sidebar, motion);
        assert_eq!(go(Focus::Previous).as_deref(), Some("Inbox"));
        for &view in &views {
            assert_eq!(go(Focus::Next).as_deref(), Some(view));
        }
        assert_eq!(go(Focus::Next).as_deref(), Some("Solo"));
        assert_eq!(go(Focus::Next).as_deref(), Some("Work"));
        assert_eq!(go(Focus::Next).as_deref(), Some("Site"));
        assert_eq!(go(Focus::Next).as_deref(), Some("Blog"));
        assert_eq!(go(Focus::Previous).as_deref(), Some("Site"));
        assert_eq!(go(Focus::Last).as_deref(), Some("Home"));
        assert_eq!(go(Focus::Next).as_deref(), Some("Home"));
        assert_eq!(go(Focus::First).as_deref(), Some("Inbox"));
        let view = sidebar.view(&document);
        assert_eq!(
            view.map(|view| (view.focused, view.keys, view.entries.len())),
            Some((Some(0), true, 7 + views.len()))
        );
        let mut empty = Sidebar::default();
        empty.toggle(None);
        assert_eq!(
            moved(&Document::new(), &mut empty, Focus::Last).as_deref(),
            views.last().copied(),
            "a new document lists its Inbox and the views"
        );
    }

    // FR-VIEW-10
    #[test]
    fn showing_hiding_and_giving_the_keys() {
        let document = document();
        let blog = Shown::Container(find(&document, "Blog"));
        let mut sidebar = Sidebar::default();
        sidebar.toggle(Some(blog));
        assert_eq!((sidebar.has_keys(), sidebar.focused()), (true, Some(blog)));
        sidebar.focus(false);
        let view = sidebar.view(&document);
        assert_eq!(view.map(|view| view.keys), Some(false), "still shown");
        sidebar.toggle(None);
        assert!(sidebar.has_keys(), "the key gives a shown sidebar the keys");
        sidebar.toggle(None);
        assert_eq!(sidebar.view(&document), None);
        assert!(!sidebar.has_keys());
        sidebar.toggle(None);
        assert_eq!(
            sidebar.focused(),
            Some(blog),
            "showing again keeps the focus"
        );
    }

    // FR-VIEW-10, D-67
    #[test]
    fn an_area_collapses_hiding_the_projects_inside_it_until_it_expands() {
        let document = document();
        let (work, site, blog) = (
            find(&document, "Work"),
            find(&document, "Site"),
            find(&document, "Blog"),
        );
        let titles = |sidebar: &Sidebar| -> Vec<String> {
            sidebar
                .visible(&document)
                .into_iter()
                .filter_map(|stop| match stop {
                    Stop::Place(place) => {
                        document.node(place.id)?.item().map(|it| it.title.clone())
                    }
                    Stop::View(_) => None,
                })
                .collect()
        };
        let mut sidebar = Sidebar::default();
        sidebar.toggle(Some(Shown::View(View::Today)));
        sidebar.toggle_collapsed(&document);
        assert!(sidebar.collapsed.is_empty(), "a view does not collapse");
        sidebar.focus_new(work);
        sidebar.toggle_collapsed(&document);
        assert_eq!(titles(&sidebar), ["Inbox", "Solo", "Work", "Home"]);
        assert_eq!(
            sidebar.view(&document).map(|view| view.collapsed),
            Some(vec![work])
        );
        sidebar.toggle_collapsed(&document);
        assert_eq!(
            titles(&sidebar),
            ["Inbox", "Solo", "Work", "Site", "Blog", "Docs", "Home"]
        );
        let mut leaf = Sidebar::default();
        leaf.toggle(Some(Shown::Container(blog)));
        leaf.toggle_collapsed(&document);
        assert_eq!(
            leaf.view(&document).map(|view| view.collapsed),
            Some(vec![])
        );
        for project in [work, site] {
            leaf.focus_new(project);
            leaf.toggle_collapsed(&document);
        }
        leaf.focus_new(blog);
        leaf.reveal(&document);
        assert!(
            titles(&leaf).contains(&"Blog".to_owned()),
            "the projects holding the focus expand"
        );
    }

    // FR-VIEW-10
    #[test]
    fn the_focus_stays_where_an_entry_left() -> Result<(), PlacementError> {
        let mut document = document();
        let mut sidebar = Sidebar::default();
        let docs = find(&document, "Docs");
        sidebar.toggle(Some(Shown::Container(docs)));
        sidebar.settle(&entries(&document));
        let mut gone = Item::new(Project, "Docs");
        gone.deleted = Some(date(2026, 10, 2));
        document.set(docs, gone)?;
        sidebar.settle(&entries(&document));
        assert_eq!(
            sidebar.focused(),
            Some(Shown::Container(find(&document, "Home")))
        );
        let mut hidden = Sidebar::default();
        hidden.settle(&entries(&document));
        assert_eq!(hidden.focused(), None, "a hidden sidebar is left alone");
        sidebar.settle(&[]);
        sidebar.move_focus(&[], Focus::Next);
        assert_eq!(sidebar.focused(), None);
        Ok(())
    }

    // FR-VIEW-10, F-FILE-06
    #[test]
    fn a_reread_document_keeps_the_entries_found_again_and_drops_the_rest() {
        let document = document();
        let (work, site, home) = (
            find(&document, "Work"),
            find(&document, "Site"),
            find(&document, "Home"),
        );
        let mut sidebar = Sidebar::default();
        sidebar.toggle(Some(Shown::View(View::Today)));
        sidebar.focus_new(work);
        sidebar.collapsed = BTreeSet::from([work, site]);
        sidebar.settle(&entries(&document));
        let at = sidebar.index;
        sidebar.rematch(|id| (id == work).then_some(home));
        assert_eq!(sidebar.focused(), Some(Shown::Container(home)));
        assert_eq!(sidebar.left, Some(Shown::View(View::Today)));
        assert_eq!(sidebar.collapsed, BTreeSet::from([home]));
        sidebar.rematch(|_| None);
        assert_eq!(sidebar.left, Some(Shown::View(View::Today)));
        assert!(sidebar.collapsed.is_empty());
        let entries = entries(&document);
        sidebar.settle(&entries);
        assert_eq!(
            sidebar.focused(),
            Some(entries[at].shown()),
            "the entry now where the one not found was"
        );
    }

    // FR-VIEW-10, FR-EDIT-05
    #[test]
    fn a_new_entry_left_empty_gives_the_focus_back() {
        let document = document();
        let (work, solo) = (find(&document, "Work"), find(&document, "Solo"));
        let mut sidebar = Sidebar::default();
        sidebar.toggle(Some(Shown::Container(work)));
        sidebar.focus_new(solo);
        assert_eq!(sidebar.focused(), Some(Shown::Container(solo)));
        sidebar.finish(false);
        assert_eq!(sidebar.focused(), Some(Shown::Container(work)));
        sidebar.focus_new(solo);
        sidebar.finish(true);
        sidebar.finish(false);
        assert_eq!(
            sidebar.focused(),
            Some(Shown::Container(solo)),
            "kept, and finished only once"
        );
    }
}
