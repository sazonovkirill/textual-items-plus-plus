//! The sidebar: the areas and projects as a tree beside the pane, where they are created,
//! renamed, deleted, reordered, indented and outdented with keys (FR-VIEW-10, OQ-10). Hides which
//! entry it keeps focused as the tree changes and where its changes put items in the document.

use super::{
    Direction, Focus, Refusal, Side,
    navigator::{self, Place},
};
use crate::domain::{Destination, Document, ItemId, ItemKind, Node, Transaction};

/// Whether the sidebar shows and has the keys, and the entry it focuses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Sidebar {
    shown: bool,
    keys: bool,
    focused: Option<ItemId>,
    /// Where the focused entry was in the tree, so that the focus stays there once it leaves.
    index: usize,
    /// The entry focused before a new one was added, to go back to when it is left empty.
    left: Option<ItemId>,
}

/// What a frontend draws of the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarView {
    /// The areas and projects, in the order of the tree.
    pub entries: Vec<Place>,
    /// The index of the focused entry, if any.
    pub focused: Option<usize>,
    /// Whether the sidebar has the keys, rather than the pane.
    pub keys: bool,
}

impl Sidebar {
    /// Shows the sidebar with the keys, focused on `near` when given, or hides it when it has
    /// the keys, giving them back to the pane, or gives it the keys when it shows without them.
    pub(super) fn toggle(&mut self, near: Option<ItemId>) {
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

    /// The focused entry's item, if any.
    pub(super) fn focused(&self) -> Option<ItemId> {
        self.focused
    }

    /// Focuses the new entry `id`, remembering the one focused before for [`finish`].
    ///
    /// [`finish`]: Self::finish
    pub(super) fn focus_new(&mut self, id: ItemId) {
        self.left = self.focused;
        self.focused = Some(id);
    }

    /// Ends adding an entry: one left empty, and so not added, gives the focus back to the entry
    /// focused before.
    pub(super) fn finish(&mut self, kept: bool) {
        if let Some(left) = self.left.take().filter(|_| !kept) {
            self.focused = Some(left);
        }
    }

    /// Moves the focus among `entries` as `motion` says: to the next or previous entry, the first
    /// or last, the area or project holding it, or the next or previous one at the top level.
    pub(super) fn move_focus(&mut self, entries: &[Place], motion: Focus) {
        let Some(at) = self.at(entries) else {
            return;
        };
        let depth = |index: usize| entries.get(index).map_or(0, |place| place.depth);
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
        if let Some(place) = target.and_then(|index| entries.get(index)) {
            self.focused = Some(place.id);
        }
    }

    /// Keeps the focus on one of `entries` now that the tree may have changed: on the entry
    /// focused while it is there, else on the one now where it was, else on the last.
    pub(super) fn settle(&mut self, entries: &[Place]) {
        if !self.shown {
            return;
        }
        let at = self.at(entries);
        self.index = at.unwrap_or(0);
        self.focused = at
            .and_then(|index| entries.get(index))
            .map(|place| place.id);
    }

    /// What a frontend draws of the sidebar with `entries`, while it shows.
    pub(super) fn view(&self, entries: Vec<Place>) -> Option<SidebarView> {
        self.shown.then(|| SidebarView {
            focused: self.at(&entries),
            keys: self.keys,
            entries,
        })
    }

    /// The index of the focused entry among `entries`, or of the one now where it was.
    fn at(&self, entries: &[Place]) -> Option<usize> {
        let last = entries.len().checked_sub(1)?;
        let found = self
            .focused
            .and_then(|id| entries.iter().position(|place| place.id == id));
        Some(found.unwrap_or(self.index.min(last)))
    }
}

/// The areas and projects of `document` as the sidebar lists them: the Navigator's tree without
/// the Inbox, which is not created, renamed, moved or deleted.
pub(super) fn entries(document: &Document) -> Vec<Place> {
    navigator::places(document)
        .into_iter()
        .map(|candidate| candidate.value)
        .filter(|place| place.kind != ItemKind::Inbox)
        .collect()
}

/// The transaction that moves `id` one step `direction` among its siblings in the tree, past the
/// next area or project listed beside it; an empty one when it is already at that end.
pub(super) fn shift(document: &Document, id: ItemId, direction: Direction) -> Transaction {
    let Some((siblings, index)) = siblings(document, id) else {
        return Transaction::default();
    };
    let destination = match direction {
        Direction::Up => siblings
            .get(..index)
            .and_then(|earlier| earlier.iter().rfind(|&&other| listed(document, other)))
            .map(|&other| Destination::Before(other)),
        Direction::Down => siblings
            .get(index + 1..)
            .and_then(|later| later.iter().find(|&&other| listed(document, other)))
            .map(|&other| Destination::After(other)),
    };
    destination.map_or_else(Transaction::default, |to| {
        document
            .move_to(id, to)
            .expect("an entry moves beside a sibling, which holds what it holds")
    })
}

/// The transaction that indents the project `id` into the nearest area or project listed above
/// it among its siblings, as its last child (FR-OPS-03).
///
/// # Errors
///
/// The [`Refusal`] when `id` is an area, which nests in nothing, or nothing is above it.
pub(super) fn indent(document: &Document, id: ItemId) -> Result<Transaction, Refusal> {
    project(document, id)?;
    let (siblings, index) = siblings(document, id).ok_or(Refusal::NothingAbove)?;
    let into = siblings
        .get(..index)
        .and_then(|earlier| earlier.iter().rfind(|&&other| listed(document, other)))
        .ok_or(Refusal::NothingAbove)?;
    Ok(document
        .move_to(id, Destination::Last(Some(*into)))
        .expect("a project fits in an area or another project"))
}

/// The transaction that outdents the project `id` out of the area or project holding it, right
/// after it (FR-OPS-03).
///
/// # Errors
///
/// The [`Refusal`] when `id` is an area, which nests in nothing, or is at the top level.
pub(super) fn outdent(document: &Document, id: ItemId) -> Result<Transaction, Refusal> {
    project(document, id)?;
    let parent = document
        .position(id)
        .and_then(|(parent, _)| parent)
        .ok_or(Refusal::NotNested)?;
    Ok(document
        .move_to(id, Destination::After(parent))
        .expect("a project fits beside the area or project holding it"))
}

/// Where a new item of `kind` goes from the entry `focused` among `entries`, on `side` of it, as
/// its parent, `None` for the top level, and its index there. An area goes beside the area or
/// top-level project holding the focus. A project goes above the focused entry, or below it, as
/// the first of the entries it holds when it is an area or holds any, as a new task below a
/// section heading is its first. With nothing focused either goes last at the top level, and
/// any other kind goes nowhere.
pub(super) fn place(
    document: &Document,
    entries: &[Place],
    focused: Option<ItemId>,
    kind: ItemKind,
    side: Side,
) -> Option<(Option<ItemId>, usize)> {
    let end = (None, document.top_level().len());
    let Some(focused) = focused else {
        return matches!(kind, ItemKind::Area | ItemKind::Project).then_some(end);
    };
    let below = usize::from(side == Side::Below);
    match kind {
        ItemKind::Area => {
            let top = super::container::path(document, focused).first().copied()?;
            let (_, index) = document.position(top)?;
            Some((None, index + below))
        }
        ItemKind::Project => {
            let (parent, index) = document.position(focused)?;
            let at = entries.iter().position(|place| place.id == focused)?;
            let holds = entries
                .get(at + 1)
                .filter(|next| next.depth > entries.get(at).map_or(0, |place| place.depth));
            let area = document
                .node(focused)
                .and_then(Node::item)
                .is_some_and(|item| item.kind == ItemKind::Area);
            if side == Side::Above {
                return Some((parent, index));
            }
            match (holds, area) {
                (Some(first), _) => document.position(first.id),
                (None, true) => Some((Some(focused), document.children(focused).len())),
                (None, false) => Some((parent, index + 1)),
            }
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

/// Whether `id` is an area or project the sidebar lists, given that its parent is listed.
fn listed(document: &Document, id: ItemId) -> bool {
    navigator::shown(document, id)
        .is_some_and(|item| matches!(item.kind, ItemKind::Area | ItemKind::Project))
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

    /// The entries, each title indented two spaces per level.
    fn tree(document: &Document) -> Vec<String> {
        entries(document)
            .iter()
            .map(|place| {
                let title = document
                    .node(place.id)
                    .and_then(Node::item)
                    .map_or("", |it| &it.title);
                format!("{}{title}", "  ".repeat(place.depth))
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
    fn lists_the_areas_and_projects_as_a_tree_without_the_inbox_or_the_trash() {
        assert_eq!(
            tree(&document()),
            ["Work", "  Site", "    Blog", "  Docs", "Solo", "Home"]
        );
    }

    // FR-VIEW-10, FR-ORG-06
    #[test]
    fn an_entry_changes_places_with_the_entry_beside_it() -> Result<(), PlacementError> {
        let mut document = document();
        let docs = find(&document, "Docs");
        apply(&mut document, |d| Ok(shift(d, docs, Direction::Up)))?;
        assert_eq!(
            tree(&document),
            ["Work", "  Docs", "  Site", "    Blog", "Solo", "Home"]
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
            ["Solo", "Work", "  Docs", "  Site", "    Blog", "Home"],
            "past the project in the Trash"
        );
        apply(&mut document, |d| Ok(shift(d, solo, Direction::Down)))?;
        let work = find(&document, "Work");
        apply(&mut document, |d| Ok(shift(d, work, Direction::Down)))?;
        assert_eq!(
            tree(&document),
            ["Solo", "Work", "  Docs", "  Site", "    Blog", "Home"]
        );
        let home = find(&document, "Home");
        assert_eq!(shift(&document, home, Direction::Down).ops(), []);
        assert_eq!(shift(&document, document.inbox(), Direction::Up).ops(), []);
        Ok(())
    }

    // FR-VIEW-10, FR-OPS-03
    #[test]
    fn a_project_indents_into_the_area_or_project_above_it() -> Result<(), PlacementError> {
        let mut document = document();
        let solo = find(&document, "Solo");
        apply(&mut document, |d| indent(d, solo))?;
        assert_eq!(
            tree(&document),
            ["Work", "  Site", "    Blog", "  Docs", "  Solo", "Home"],
            "into the area above, past the project in the Trash"
        );
        apply(&mut document, |d| indent(d, solo))?;
        assert_eq!(
            tree(&document),
            ["Work", "  Site", "    Blog", "  Docs", "    Solo", "Home"]
        );
        let site = find(&document, "Site");
        assert_eq!(indent(&document, site), Err(Refusal::NothingAbove));
        let work = find(&document, "Work");
        assert_eq!(indent(&document, work), Err(Refusal::DoesNotNest));
        assert_eq!(
            indent(&document, document.inbox()),
            Err(Refusal::DoesNotNest)
        );
        Ok(())
    }

    // FR-VIEW-10, FR-OPS-03
    #[test]
    fn a_project_outdents_after_the_area_or_project_holding_it() -> Result<(), PlacementError> {
        let mut document = document();
        let blog = find(&document, "Blog");
        apply(&mut document, |d| outdent(d, blog))?;
        assert_eq!(
            tree(&document),
            ["Work", "  Site", "  Blog", "  Docs", "Solo", "Home"]
        );
        apply(&mut document, |d| outdent(d, blog))?;
        assert_eq!(
            tree(&document),
            ["Work", "  Site", "  Docs", "Blog", "Solo", "Home"]
        );
        assert_eq!(outdent(&document, blog), Err(Refusal::NotNested));
        let home = find(&document, "Home");
        assert_eq!(outdent(&document, home), Err(Refusal::DoesNotNest));
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
        let focused = focused.map(|title| find(document, title));
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
    fn a_new_project_goes_beside_the_focus_or_first_in_what_it_holds() {
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
            Some(at("Site", "Blog")),
            "first in Site"
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
        assert_eq!(new("Home", Side::Above), Some(at("", "Home")));
        assert_eq!(new("Solo", Side::Below), Some(at("", "Home")));
        assert_eq!(
            placed(&document, None, Project, Side::Above),
            Some(at("", "")),
            "with nothing focused, last at the top level"
        );
        let gone = find(&document, "Gone");
        let entries = entries(&document);
        assert_eq!(
            place(&document, &entries, Some(gone), Project, Side::Below),
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
        let id = sidebar.focused()?;
        Some(document.node(id)?.item()?.title.clone())
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
        let mut go = |motion| moved(&document, &mut sidebar, motion);
        assert_eq!(go(Focus::Previous).as_deref(), Some("Work"));
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
        assert_eq!(go(Focus::First).as_deref(), Some("Work"));
        let view = sidebar.view(entries(&document));
        assert_eq!(
            view.map(|view| (view.focused, view.keys, view.entries.len())),
            Some((Some(0), true, 6))
        );
        let mut empty = Sidebar::default();
        empty.toggle(None);
        assert_eq!(moved(&Document::new(), &mut empty, Focus::Next), None);
    }

    // FR-VIEW-10
    #[test]
    fn showing_hiding_and_giving_the_keys() {
        let document = document();
        let blog = find(&document, "Blog");
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
        sidebar.toggle(Some(docs));
        sidebar.settle(&entries(&document));
        let mut gone = Item::new(Project, "Docs");
        gone.deleted = Some(date(2026, 10, 2));
        document.set(docs, gone)?;
        sidebar.settle(&entries(&document));
        assert_eq!(sidebar.focused(), Some(find(&document, "Solo")));
        let mut hidden = Sidebar::default();
        hidden.settle(&entries(&document));
        assert_eq!(hidden.focused(), None, "a hidden sidebar is left alone");
        sidebar.settle(&[]);
        assert_eq!(sidebar.focused(), None);
        Ok(())
    }

    // FR-VIEW-10, FR-EDIT-05
    #[test]
    fn a_new_entry_left_empty_gives_the_focus_back() {
        let document = document();
        let (work, solo) = (find(&document, "Work"), find(&document, "Solo"));
        let mut sidebar = Sidebar::default();
        sidebar.toggle(Some(work));
        sidebar.focus_new(solo);
        assert_eq!(sidebar.focused(), Some(solo));
        sidebar.finish(false);
        assert_eq!(sidebar.focused(), Some(work));
        sidebar.focus_new(solo);
        sidebar.finish(true);
        sidebar.finish(false);
        assert_eq!(
            sidebar.focused(),
            Some(solo),
            "kept, and finished only once"
        );
    }
}
