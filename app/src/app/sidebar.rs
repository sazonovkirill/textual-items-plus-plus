//! The sidebar: the Inbox, the views, and the projects as a tree beside the pane, where they are
//! created, renamed, deleted, reordered, indented and outdented with keys (FR-VIEW-10, OQ-10).
//! Hides which entry it keeps focused as the tree changes and where its changes put items in the
//! document.

use super::{
    Direction, Focus, Refusal, Side,
    navigator::{self, Stop},
    view::{Shown, View},
};
use crate::domain::{Destination, Document, ItemId, ItemKind, Transaction};

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
}

/// What a frontend draws of the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarView {
    /// The Inbox, the views, and the projects in the order of the tree.
    pub entries: Vec<Stop>,
    /// The index of the focused entry, if any.
    pub focused: Option<usize>,
    /// Whether the sidebar has the keys, rather than the pane.
    pub keys: bool,
}

impl Sidebar {
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

    /// Moves the focus among `entries` as `motion` says: to the next or previous entry, the first
    /// or last, the project holding it, or the next or previous one at the top level.
    pub(super) fn move_focus(&mut self, entries: &[Stop], motion: Focus) {
        let Some(at) = self.at(entries) else {
            return;
        };
        let depth = |index: usize| entries.get(index).map_or(0, |stop| stop.depth());
        let earlier = (0..at).rev();
        let target = match motion {
            Focus::Next => Some((at + 1).min(entries.len() - 1)),
            Focus::Previous => Some(at.saturating_sub(1)),
            Focus::First => Some(0),
            Focus::Last => Some(entries.len() - 1),
            Focus::Parent => earlier.clone().find(|&index| depth(index) < depth(at)),
            Focus::NextSection => (at + 1..entries.len()).find(|&index| depth(index) == 0),
            Focus::PreviousSection => earlier.clone().find(|&index| depth(index) == 0),
        };
        if let Some(stop) = target.and_then(|index| entries.get(index)) {
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

    /// What a frontend draws of the sidebar with `entries`, while it shows.
    pub(super) fn view(&self, entries: Vec<Stop>) -> Option<SidebarView> {
        self.shown.then(|| SidebarView {
            focused: self.at(&entries),
            keys: self.keys,
            entries,
        })
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

/// The entries of `document` as the sidebar lists them: the Navigator's, the Inbox, the views
/// and the projects as a tree in the order of the file, without the Logbook, as the owner's
/// order of the sidebar leaves it out (D-45).
pub(super) fn entries(document: &Document) -> Vec<Stop> {
    navigator::stops(document)
        .into_iter()
        .map(|candidate| candidate.value)
        .filter(|&stop| stop != Stop::View(View::Logbook))
        .collect()
}

/// The transaction that moves the project `id` one step `direction` among its siblings in the
/// tree, past the next project listed beside it; an empty one when it is already at that end.
pub(super) fn shift(document: &Document, id: ItemId, direction: Direction) -> Transaction {
    let Some((siblings, index)) = siblings(document, id) else {
        return Transaction::default();
    };
    let listed = |other: &&ItemId| listed(document, **other);
    let destination = match direction {
        Direction::Up => siblings
            .get(..index)
            .and_then(|earlier| earlier.iter().rfind(listed))
            .map(|&other| Destination::Before(other)),
        Direction::Down => siblings
            .get(index + 1..)
            .and_then(|later| later.iter().find(listed))
            .map(|&other| Destination::After(other)),
    };
    destination.map_or_else(Transaction::default, |to| {
        document
            .move_to(id, to)
            .expect("a project moves beside a sibling project")
    })
}

/// The transaction that indents the project `id` into the nearest project listed above it among
/// its siblings, as its last child (FR-OPS-03).
///
/// # Errors
///
/// The [`Refusal`] when `id` is no project, as the Inbox, or no project is above it.
pub(super) fn indent(document: &Document, id: ItemId) -> Result<Transaction, Refusal> {
    project(document, id)?;
    let (siblings, index) = siblings(document, id).ok_or(Refusal::NothingAbove)?;
    let into = siblings
        .get(..index)
        .and_then(|earlier| earlier.iter().rfind(|&&other| listed(document, other)))
        .ok_or(Refusal::NothingAbove)?;
    Ok(document
        .move_to(id, Destination::Last(Some(*into)))
        .expect("a project fits in another project"))
}

/// The transaction that outdents the project `id` out of the project holding it, right after it
/// (FR-OPS-03).
///
/// # Errors
///
/// The [`Refusal`] when `id` is no project, as the Inbox, or is at the top level.
pub(super) fn outdent(document: &Document, id: ItemId) -> Result<Transaction, Refusal> {
    project(document, id)?;
    let parent = document
        .position(id)
        .and_then(|(parent, _)| parent)
        .ok_or(Refusal::NotNested)?;
    Ok(document
        .move_to(id, Destination::After(parent))
        .expect("a project fits beside the project holding it"))
}

/// Where a new project goes from the entry `focused` on `side` of it, as its parent, `None` for
/// the top level, and its index there: beside the focused project, as its sibling, or first at
/// the top level from the Inbox, a view or no entry (D-45).
pub(super) fn beside(
    document: &Document,
    focused: Option<Shown>,
    side: Side,
) -> (Option<ItemId>, usize) {
    match focused.and_then(|shown| match shown {
        Shown::Container(id) => document.position(id),
        Shown::View(_) => None,
    }) {
        Some((parent, index)) => (parent, index + usize::from(side == Side::Below)),
        None => (None, 0),
    }
}

/// The siblings of `id`, the top-level nodes for one at the top level, and its index among them.
fn siblings(document: &Document, id: ItemId) -> Option<(&[ItemId], usize)> {
    let (parent, index) = document.position(id)?;
    let siblings = parent.map_or(document.top_level(), |parent| document.children(parent));
    Some((siblings, index))
}

/// Whether `id` is a project the sidebar lists, given that its parent is listed.
fn listed(document: &Document, id: ItemId) -> bool {
    navigator::shown(document, id).is_some_and(|item| item.kind == ItemKind::Project)
}

/// Refuses `id` unless it is a project, the only entry that nests.
fn project(document: &Document, id: ItemId) -> Result<(), Refusal> {
    navigator::shown(document, id)
        .filter(|item| item.kind == ItemKind::Project)
        .map(|_| ())
        .ok_or(Refusal::DoesNotNest)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::view::REGISTRY;
    use crate::domain::{Item, Node, PlacementError};
    use ItemKind::{Project, Section, Task};

    fn add(document: &mut Document, parent: Option<ItemId>, kind: ItemKind, title: &str) -> ItemId {
        document
            .push(parent, Node::Item(Item::new(kind, title)))
            .expect("the test builds a valid hierarchy")
    }

    /// Project `Work` holding task `t`, section `S` and projects `Site`, holding project `Blog`, and
    /// `Docs`; project `Gone` in the Trash; project `Solo`; and the empty project `Home`.
    fn document() -> Document {
        let mut document = Document::new();
        let work = add(&mut document, None, Project, "Work");
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
        add(&mut document, None, Project, "Home");
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
        change: impl FnOnce(&Document) -> Result<Transaction, Refusal>,
    ) -> Result<(), PlacementError> {
        let before = document.clone();
        let transaction = change(document).map_err(|_| PlacementError::DoesNotFit)?;
        document.apply(&transaction)?;
        let mut undone = document.clone();
        undone.apply(&transaction.inverse())?;
        assert_eq!(undone, before, "undo restores the document");
        Ok(())
    }

    // FR-VIEW-10
    #[test]
    fn lists_the_inbox_and_the_views_then_the_projects_as_a_tree_without_the_trash() {
        let document = document();
        let views: Vec<Stop> = REGISTRY
            .iter()
            .map(|entry| Stop::View(entry.view))
            .filter(|&stop| stop != Stop::View(View::Logbook))
            .collect();
        let listed = entries(&document);
        assert_eq!(listed.get(1..=views.len()), Some(&views[..]));
        assert!(
            !listed.contains(&Stop::View(View::Logbook)),
            "the owner's order leaves the Logbook out"
        );
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Work", "  Site", "    Blog", "  Docs", "Solo", "Home"
            ]
        );
    }

    // FR-VIEW-10, FR-ORG-06
    #[test]
    fn a_project_changes_places_with_the_project_beside_it() -> Result<(), PlacementError> {
        let mut document = document();
        let docs = find(&document, "Docs");
        apply(&mut document, |d| Ok(shift(d, docs, Direction::Up)))?;
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Work", "  Docs", "  Site", "    Blog", "Solo", "Home"
            ]
        );
        let up = shift(&document, docs, Direction::Up);
        assert!(
            up.ops().is_empty(),
            "the task and section above are no entries"
        );
        let solo = find(&document, "Solo");
        apply(&mut document, |d| Ok(shift(d, solo, Direction::Up)))?;
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Solo", "Work", "  Docs", "  Site", "    Blog", "Home"
            ],
            "past the project in the Trash"
        );
        let home = find(&document, "Home");
        assert_eq!(shift(&document, home, Direction::Down).ops(), []);
        assert_eq!(shift(&document, document.inbox(), Direction::Up).ops(), []);
        Ok(())
    }

    // FR-VIEW-10, FR-OPS-03
    #[test]
    fn a_project_indents_into_the_project_above_it() -> Result<(), PlacementError> {
        let mut document = document();
        let docs = find(&document, "Docs");
        apply(&mut document, |d| indent(d, docs))?;
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Work", "  Site", "    Blog", "    Docs", "Solo", "Home"
            ]
        );
        let solo = find(&document, "Solo");
        apply(&mut document, |d| indent(d, solo))?;
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Work", "  Site", "    Blog", "    Docs", "  Solo", "Home"
            ],
            "past the project in the Trash"
        );
        let work = find(&document, "Work");
        assert_eq!(indent(&document, work), Err(Refusal::NothingAbove));
        assert_eq!(
            indent(&document, document.inbox()),
            Err(Refusal::DoesNotNest)
        );
        Ok(())
    }

    // FR-VIEW-10, FR-OPS-03
    #[test]
    fn a_project_outdents_after_the_project_holding_it() -> Result<(), PlacementError> {
        let mut document = document();
        let blog = find(&document, "Blog");
        apply(&mut document, |d| outdent(d, blog))?;
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Work", "  Site", "  Blog", "  Docs", "Solo", "Home"
            ]
        );
        apply(&mut document, |d| outdent(d, blog))?;
        assert_eq!(
            tree(&document),
            ["Inbox", "Work", "  Site", "  Docs", "Blog", "Solo", "Home"]
        );
        assert_eq!(outdent(&document, blog), Err(Refusal::NotNested));
        assert_eq!(
            outdent(&document, document.inbox()),
            Err(Refusal::DoesNotNest)
        );
        Ok(())
    }

    // FR-VIEW-10, FR-EDIT-05, FR-ORG-06
    #[test]
    fn a_new_project_goes_beside_the_focused_one_or_first_at_the_top_level() {
        let document = document();
        let title = |id: Option<ItemId>| {
            id.and_then(|id| document.node(id).and_then(Node::item))
                .map_or(String::new(), |it| it.title.clone())
        };
        let placed = |focused: Option<Shown>, side| {
            let (parent, index) = beside(&document, focused, side);
            let siblings = parent.map_or(document.top_level(), |parent| document.children(parent));
            (title(parent), title(siblings.get(index).copied()))
        };
        let project = |name| Some(Shown::Container(find(&document, name)));
        let at = |parent: &str, before: &str| (parent.to_owned(), before.to_owned());
        assert_eq!(placed(project("Docs"), Side::Above), at("Work", "Docs"));
        assert_eq!(placed(project("Docs"), Side::Below), at("Work", ""));
        assert_eq!(placed(project("Site"), Side::Below), at("Work", "Docs"));
        assert_eq!(placed(project("Work"), Side::Below), at("", "Gone"));
        let first = at("", "Work");
        assert_eq!(
            placed(Some(Shown::Container(document.inbox())), Side::Below),
            first
        );
        assert_eq!(placed(Some(Shown::View(View::Today)), Side::Above), first);
        assert_eq!(placed(None, Side::Below), first);
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
        assert_eq!(sidebar.view(entries(&document)), None, "hidden at first");
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
        assert_eq!(go(Focus::Next).as_deref(), Some("Work"));
        assert_eq!(go(Focus::Next).as_deref(), Some("Site"));
        assert_eq!(go(Focus::Next).as_deref(), Some("Blog"));
        assert_eq!(go(Focus::Parent).as_deref(), Some("Site"));
        assert_eq!(go(Focus::Parent).as_deref(), Some("Work"));
        assert_eq!(
            go(Focus::Parent).as_deref(),
            Some("Work"),
            "nothing holds it"
        );
        assert_eq!(go(Focus::NextSection).as_deref(), Some("Solo"));
        assert_eq!(go(Focus::Last).as_deref(), Some("Home"));
        assert_eq!(go(Focus::Next).as_deref(), Some("Home"));
        assert_eq!(go(Focus::NextSection).as_deref(), Some("Home"));
        assert_eq!(go(Focus::PreviousSection).as_deref(), Some("Solo"));
        assert_eq!(go(Focus::First).as_deref(), Some("Inbox"));
        let view = sidebar.view(entries(&document));
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
        let view = sidebar.view(entries(&document));
        assert_eq!(view.map(|view| view.keys), Some(false), "still shown");
        sidebar.toggle(None);
        assert!(sidebar.has_keys(), "the key gives a shown sidebar the keys");
        sidebar.toggle(None);
        assert_eq!(sidebar.view(entries(&document)), None);
        assert!(!sidebar.has_keys());
        sidebar.toggle(None);
        assert_eq!(
            sidebar.focused(),
            Some(blog),
            "showing again keeps the focus"
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
            Some(Shown::Container(find(&document, "Solo")))
        );
        let mut hidden = Sidebar::default();
        hidden.settle(&entries(&document));
        assert_eq!(hidden.focused(), None, "a hidden sidebar is left alone");
        sidebar.settle(&[]);
        sidebar.move_focus(&[], Focus::Next);
        assert_eq!(sidebar.focused(), None);
        Ok(())
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
