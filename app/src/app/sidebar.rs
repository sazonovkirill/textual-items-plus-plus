//! The sidebar: the Inbox, the views, the pinned projects, and the areas and projects as a tree
//! beside the pane, where they are created, renamed, deleted and reordered with keys, and
//! collapsed, each group but the Inbox and the areas under a heading that folds it (FR-VIEW-10,
//! FR-VIEW-12, OQ-10, D-144). Hides which entry it keeps focused as the tree changes, which areas
//! it collapses and groups it folds, and where its changes put items in the document.

use std::{cell::RefCell, collections::BTreeSet};

use jiff::civil::Date;

use super::{
    Direction, Focus, Refusal, Side, container,
    navigator::{self, Stop},
    view::{self, Shown, View},
};
use crate::domain::{Destination, Document, Item, ItemId, ItemKind, Node, Transaction};

/// Whether the sidebar shows and has the keys, and the entry it focuses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Sidebar {
    shown: bool,
    keys: bool,
    focused: Option<Entry>,
    /// Where the focused entry was in the tree, so that the focus stays there once it leaves.
    index: usize,
    /// The entry focused before a new one was added, to go back to when it is left empty.
    left: Option<Entry>,
    /// The areas whose projects are hidden, for as long as the app runs (D-67, D-75).
    collapsed: BTreeSet<ItemId>,
    /// The groups listed by their heading alone, for as long as the app runs (D-144).
    folded: BTreeSet<SidebarGroup>,
    counted: Counted,
}

/// How many tasks each view of the registry listed when last counted, with the revision of the
/// document and the day they were counted on. Counting every view takes a walk of the document
/// for each, too long to add to a keystroke on a large file (N-PERF-02), so they are counted when
/// first drawn and then again only once a frame has shown the change, by
/// [`recount`](Sidebar::recount). They are counted only in the document of the state holding the
/// sidebar, whose revisions never repeat, and a reload, which brings another document, forgets
/// them. Every count is equal to every other, since they only stand for the document.
#[derive(Debug, Clone, Default)]
struct Counted(RefCell<Option<ViewCounts>>);

/// How many tasks each view of the registry lists, and in what.
#[derive(Debug, Clone)]
struct ViewCounts {
    revision: u64,
    today: Date,
    views: Vec<(View, usize)>,
}

impl ViewCounts {
    /// The views' counts in `document` on `today`.
    fn of(document: &Document, today: Date) -> Self {
        Self {
            revision: document.revision(),
            today,
            views: view::REGISTRY
                .iter()
                .map(|entry| (entry.view, view::tasks(document, entry.view, today)))
                .collect(),
        }
    }
}

impl PartialEq for Counted {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Counted {}

/// A group of entries the sidebar lists under a heading, which collapses it to the heading alone
/// (D-144). The Inbox stands alone and each area heads its own projects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SidebarGroup {
    /// The views.
    Views,
    /// The pinned projects (FR-VIEW-12).
    Pinned,
    /// The projects in no area.
    Projects,
}

impl SidebarGroup {
    /// Its heading's title.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Views => "Views",
            Self::Pinned => "Pinned",
            Self::Projects => "Projects",
        }
    }
}

/// A line the sidebar lists: a group's heading, or an entry that opens what it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarRow {
    /// The heading of a group (D-144).
    Heading(SidebarGroup),
    /// The entry of a container or a view.
    Stop(Stop),
}

impl SidebarRow {
    /// The entry, unless it is a heading.
    #[must_use]
    pub const fn stop(self) -> Option<Stop> {
        match self {
            Self::Stop(stop) => Some(stop),
            Self::Heading(_) => None,
        }
    }

    /// What the entry opens, unless it is a heading.
    #[must_use]
    pub const fn shown(self) -> Option<Shown> {
        match self {
            Self::Stop(stop) => Some(stop.shown()),
            Self::Heading(_) => None,
        }
    }
}

/// A row the sidebar focuses: a group's heading, or an entry by what it opens and whether it is
/// that project's entry among the pinned projects, which list a pinned project a second time
/// (FR-VIEW-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Heading(SidebarGroup),
    Stop { shown: Shown, in_pinned: bool },
}

impl Entry {
    /// The entry of what `shown` opens in the tree, or the Inbox's or a view's.
    const fn of(shown: Shown) -> Self {
        Self::Stop {
            shown,
            in_pinned: false,
        }
    }

    /// The entry that is `row`.
    const fn at(row: SidebarRow) -> Self {
        match row {
            SidebarRow::Heading(group) => Self::Heading(group),
            SidebarRow::Stop(stop) => Self::Stop {
                shown: stop.shown(),
                in_pinned: stop.in_pinned(),
            },
        }
    }

    /// What it opens, unless it is a heading.
    const fn shown(self) -> Option<Shown> {
        match self {
            Self::Stop { shown, .. } => Some(shown),
            Self::Heading(_) => None,
        }
    }
}

/// What a frontend draws of the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarView {
    /// The Inbox, the views, the pinned projects, and the areas and projects in the order of the
    /// tree, each group but the Inbox and the areas under its heading.
    pub entries: Vec<SidebarRow>,
    /// The index of the focused entry, if any.
    pub focused: Option<usize>,
    /// Whether the sidebar has the keys, rather than the pane.
    pub keys: bool,
    /// The areas among the entries whose projects are hidden.
    pub collapsed: Vec<ItemId>,
    /// The groups among the entries listed by their heading alone.
    pub folded: Vec<SidebarGroup>,
    /// How many tasks each of the entries lists, in their order: a container's open tasks, a
    /// view's tasks as it lists them with completed ones hidden (FR-VIEW-10); none while the
    /// settings hide task counts.
    pub counts: Vec<usize>,
}

impl Sidebar {
    /// A sidebar that shows without the keys, focused on `focused`, as at start (FR-VIEW-10).
    pub(super) fn open(focused: Shown) -> Self {
        Self {
            shown: true,
            focused: Some(Entry::of(focused)),
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
                self.focused = near.map(Entry::of).or(self.focused);
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
        self.focused.and_then(Entry::shown)
    }

    /// Whether the focused row is a group's heading.
    pub(super) fn on_heading(&self) -> bool {
        matches!(self.focused, Some(Entry::Heading(_)))
    }

    /// Focuses the new entry `id`, remembering the one focused before for [`finish`].
    ///
    /// [`finish`]: Self::finish
    pub(super) fn focus_new(&mut self, id: ItemId) {
        self.left = self.focused;
        self.focused = Some(Entry::of(Shown::Container(id)));
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
    pub(super) fn move_focus(&mut self, entries: &[SidebarRow], motion: Focus) {
        let Some(at) = self.at(entries) else {
            return;
        };
        let target = match motion {
            Focus::Next => (at + 1).min(entries.len() - 1),
            Focus::Previous => at.saturating_sub(1),
            Focus::First => 0,
            Focus::Last => entries.len() - 1,
        };
        if let Some(&row) = entries.get(target) {
            self.focused = Some(Entry::at(row));
        }
    }

    /// Moves the focus among `entries` as `motion` says, on past the headings, which open nothing,
    /// unless one ends them (D-144).
    pub(super) fn move_past_headings(&mut self, entries: &[SidebarRow], motion: Focus) {
        loop {
            let before = self.focused;
            self.move_focus(entries, motion);
            if !self.on_heading() || self.focused == before {
                return;
            }
        }
    }

    /// Keeps the focus on one of `entries` now that the tree may have changed: on the entry
    /// focused while it is there, else on the one now where it was, else on the last.
    pub(super) fn settle(&mut self, entries: &[SidebarRow]) {
        if !self.shown {
            return;
        }
        let at = self.at(entries);
        self.index = at.unwrap_or(0);
        self.focused = at
            .and_then(|index| entries.get(index))
            .map(|&row| Entry::at(row));
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
                    .filter_map(|row| match row {
                        SidebarRow::Stop(Stop::Place(place)) => Some(place.id),
                        SidebarRow::Stop(Stop::View(_)) | SidebarRow::Heading(_) => None,
                    })
                    .filter(|&id| self.collapsed.contains(&id) && holds_projects(document, id))
                    .collect(),
                folded: entries
                    .iter()
                    .filter_map(|row| match row {
                        SidebarRow::Heading(group) => self.folded.contains(group).then_some(*group),
                        SidebarRow::Stop(_) => None,
                    })
                    .collect(),
                counts: Vec::new(),
                entries,
            }
        })
    }

    /// How many tasks each of `entries` lists in `document`, in their order: a container's open
    /// tasks, and a view's tasks as [`view::tasks`] counts them, as last counted, or now on
    /// `today` when they have not been counted in this document yet.
    pub(super) fn counts(
        &self,
        document: &Document,
        entries: &[SidebarRow],
        today: Date,
    ) -> Vec<usize> {
        let mut counted = self.counted.0.borrow_mut();
        let views = &counted
            .get_or_insert_with(|| ViewCounts::of(document, today))
            .views;
        entries
            .iter()
            .map(|row| match row {
                SidebarRow::Stop(Stop::Place(place)) => place.open_tasks,
                SidebarRow::Stop(Stop::View(shown)) => views
                    .iter()
                    .find(|(view, _)| view == shown)
                    .map_or(0, |&(_, tasks)| tasks),
                SidebarRow::Heading(_) => 0,
            })
            .collect()
    }

    /// Whether the views' tasks were counted before the latest change to `document` or on another
    /// day than `today`.
    pub(super) fn stale(&self, document: &Document, today: Date) -> bool {
        self.counted
            .0
            .borrow()
            .as_ref()
            .is_some_and(|counts| counts.revision != document.revision() || counts.today != today)
    }

    /// Counts the views' tasks again in `document` on `today` when they are [stale](Self::stale).
    pub(super) fn recount(&mut self, document: &Document, today: Date) {
        if self.stale(document, today) {
            *self.counted.0.get_mut() = Some(ViewCounts::of(document, today));
        }
    }

    /// The rows the sidebar lists of `document`: the [`entries`] less those inside a collapsed
    /// area, each group but the Inbox and the areas under its heading, a folded group's entries
    /// left out (D-144).
    pub(super) fn visible(&self, document: &Document) -> Vec<SidebarRow> {
        let Grouped {
            inbox,
            views,
            pinned,
            projects,
            areas,
        } = grouped(document);
        let mut rows: Vec<SidebarRow> = inbox.into_iter().map(SidebarRow::Stop).collect();
        for (group, stops) in [
            (SidebarGroup::Views, views),
            (SidebarGroup::Pinned, pinned),
            (SidebarGroup::Projects, projects),
        ] {
            if stops.is_empty() {
                continue;
            }
            rows.push(SidebarRow::Heading(group));
            if !self.folded.contains(&group) {
                rows.extend(self.expanded(stops).map(SidebarRow::Stop));
            }
        }
        rows.extend(self.expanded(areas).map(SidebarRow::Stop));
        rows
    }

    /// The entries the sidebar lists of `document` with every group unfolded: the [`entries`]
    /// less those inside a collapsed area.
    pub(super) fn stops(&self, document: &Document) -> Vec<Stop> {
        self.expanded(entries(document)).collect()
    }

    /// `stops` less those inside a collapsed area.
    fn expanded(&self, stops: Vec<Stop>) -> impl Iterator<Item = Stop> {
        let mut hidden_below = None;
        stops.into_iter().filter(move |&stop| {
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
    }

    /// Folds the group whose heading is focused to its heading, or unfolds it (owner, D-144), or
    /// collapses the focused area, or a project a file nests projects in, when it holds projects
    /// the sidebar lists, or expands it when it is collapsed (owner, D-67, D-75).
    pub(super) fn toggle_collapsed(&mut self, document: &Document) {
        match self.focused {
            Some(Entry::Heading(group)) => {
                if !self.folded.remove(&group) {
                    self.folded.insert(group);
                }
            }
            Some(Entry::Stop {
                shown: Shown::Container(id),
                ..
            }) => {
                if !self.collapsed.remove(&id) && holds_projects(document, id) {
                    self.collapsed.insert(id);
                }
            }
            Some(Entry::Stop { .. }) | None => {}
        }
    }

    /// Expands the areas holding the focused entry, and unfolds its group, so that a project
    /// added or moved into a collapsed one shows where it went.
    pub(super) fn reveal(&mut self, document: &Document) {
        let Some(Entry::Stop { shown, in_pinned }) = self.focused else {
            return;
        };
        if let Shown::Container(id) = shown {
            for above in container::path(document, id) {
                if above != id {
                    self.collapsed.remove(&above);
                }
            }
        }
        // Only a folded group needs the walk that finds the group of the focused entry.
        if self.folded.is_empty() {
            return;
        }
        let Grouped {
            views,
            pinned,
            projects,
            ..
        } = grouped(document);
        let holds = |stops: &[Stop]| {
            (stops.iter()).any(|stop| stop.shown() == shown && stop.in_pinned() == in_pinned)
        };
        for (group, stops) in [
            (SidebarGroup::Views, views),
            (SidebarGroup::Pinned, pinned),
            (SidebarGroup::Projects, projects),
        ] {
            if holds(&stops) {
                self.folded.remove(&group);
            }
        }
    }

    /// The transaction that moves the focused entry's area or project `id` one step `direction`:
    /// from its entry in the tree as [`shift`] does, and from its entry among the pinned projects
    /// past the pinned project beside it there, so that both lists change together; an empty one
    /// when it is the first or last pinned project. Refused when that project is not a sibling
    /// [`alike`], since moving past another would leave the pinned projects as they were (D-107).
    pub(super) fn shift(
        &self,
        document: &Document,
        id: ItemId,
        direction: Direction,
    ) -> Result<Transaction, Refusal> {
        if !matches!(
            self.focused,
            Some(Entry::Stop {
                in_pinned: true,
                ..
            })
        ) {
            return Ok(shift(document, id, direction));
        }
        let pinned: Vec<ItemId> = entries(document)
            .into_iter()
            .filter_map(|stop| match stop {
                Stop::Place(place) if place.in_pinned => Some(place.id),
                _ => None,
            })
            .collect();
        let at = pinned.iter().position(|&other| other == id);
        let beside = at.and_then(|at| match direction {
            Direction::Up => at.checked_sub(1).and_then(|before| pinned.get(before)),
            Direction::Down => pinned.get(at + 1),
        });
        let Some(&beside) = beside else {
            return Ok(Transaction::default());
        };
        let sibling =
            siblings(document, id).is_some_and(|(siblings, _)| siblings.contains(&beside));
        if !sibling || !alike(document, id, beside) {
            return Err(Refusal::NotPinnedSibling);
        }
        let to = match direction {
            Direction::Up => Destination::Before(beside),
            Direction::Down => Destination::After(beside),
        };
        Ok(document
            .move_to(id, to)
            .expect("a pinned project moves beside its sibling, which holds what it holds"))
    }

    /// Keeps the focused entry, the one to go back to and the collapsed areas on the items
    /// `exact` finds them as in a document read again; a focused entry not found gives way to
    /// the one now where it was, and the others are dropped. The views' counts are forgotten.
    pub(super) fn rematch(&mut self, exact: impl Fn(ItemId) -> Option<ItemId>) {
        let rematched = |entry: Entry| match entry {
            Entry::Stop {
                shown: Shown::Container(id),
                in_pinned,
            } => exact(id).map(|id| Entry::Stop {
                shown: Shown::Container(id),
                in_pinned,
            }),
            Entry::Stop { .. } | Entry::Heading(_) => Some(entry),
        };
        self.focused = self.focused.and_then(rematched);
        self.left = self.left.and_then(rematched);
        self.collapsed = self.collapsed.iter().filter_map(|&id| exact(id)).collect();
        self.counted = Counted::default();
    }

    /// The index of the focused entry among `entries`, or of the one now where it was.
    fn at(&self, entries: &[SidebarRow]) -> Option<usize> {
        let last = entries.len().checked_sub(1)?;
        let found = self
            .focused
            .and_then(|entry| entries.iter().position(|&row| Entry::at(row) == entry));
        Some(found.unwrap_or(self.index.min(last)))
    }
}

/// The entries of `document` as the sidebar lists them: the Navigator's with the Inbox and the
/// views first, then the pinned projects, then the projects in no area and then the areas, each
/// followed by what it holds, as Things 3's sidebar groups them. Within each group they keep the
/// order of the file.
pub(super) fn entries(document: &Document) -> Vec<Stop> {
    let Grouped {
        inbox,
        views,
        pinned,
        projects,
        areas,
    } = grouped(document);
    [inbox, views, pinned, projects, areas].concat()
}

/// The sidebar's entries of `document` by group, as [`entries`] lists them.
struct Grouped {
    inbox: Vec<Stop>,
    views: Vec<Stop>,
    pinned: Vec<Stop>,
    projects: Vec<Stop>,
    areas: Vec<Stop>,
}

/// The sidebar's entries of `document` by group.
fn grouped(document: &Document) -> Grouped {
    let mut grouped = Grouped {
        inbox: Vec::new(),
        views: Vec::new(),
        pinned: Vec::new(),
        projects: Vec::new(),
        areas: Vec::new(),
    };
    let mut group = ItemKind::Inbox;
    for stop in navigator::stops(document)
        .into_iter()
        .map(|candidate| candidate.value)
    {
        let Stop::Place(place) = stop else {
            grouped.views.push(stop);
            continue;
        };
        if place.in_pinned {
            grouped.pinned.push(stop);
            continue;
        }
        if place.depth == 0 {
            group = place.kind;
        }
        match group {
            ItemKind::Inbox => grouped.inbox.push(stop),
            ItemKind::Area => grouped.areas.push(stop),
            _ => grouped.projects.push(stop),
        }
    }
    grouped
}

/// The transaction that moves `id` one step `direction` among its siblings in the tree, past the
/// next entry of its kind listed beside it, since the sidebar lists the projects in no area apart
/// from the areas, and Later projects after the others (D-89); an empty one when it is already at
/// that end.
fn shift(document: &Document, id: ItemId, direction: Direction) -> Transaction {
    let Some((siblings, index)) = siblings(document, id) else {
        return Transaction::default();
    };
    let alike = |other: ItemId| alike(document, id, other);
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

/// Whether `other` is an entry the sidebar lists beside `id` among those of its kind: of the same
/// kind, and both Later or both not (D-89).
fn alike(document: &Document, id: ItemId, other: ItemId) -> bool {
    let later = |id| {
        document
            .node(id)
            .and_then(Node::item)
            .is_some_and(Item::is_later)
    };
    listed(document, other)
        && kind(document, other) == kind(document, id)
        && later(other) == later(id)
}

/// Where a new project goes from the entry `focused` among `entries`, on `side` of it, as its
/// parent, `None` for the top level, and its index there (D-127). It goes above or below the
/// focused project, or the outermost project holding it. Below an area it is the area's first,
/// as a new task below a section heading is the section's first; above one it goes where the
/// entry above stands: last in an area listing none of its projects, below a project, and else
/// last among the projects in no area. From the Inbox or a view it goes first among the projects
/// in no area, and with nothing focused last at the top level.
pub(super) fn place(
    document: &Document,
    entries: &[Stop],
    focused: Option<Shown>,
    side: Side,
) -> Option<(Option<ItemId>, usize)> {
    let end = (None, document.top_level().len());
    let Some(focused) = focused else {
        return Some(end);
    };
    // The top level lists projects apart from areas, each kind in the order of the file, so the
    // first place there is the first project's. The views are listed with the Inbox.
    let focused = match focused {
        Shown::Container(id) if id != document.inbox() => id,
        Shown::Container(_) | Shown::View(_) => return Some((None, 0)),
    };
    let at = entries
        .iter()
        .position(|stop| stop.shown() == Shown::Container(focused))?;
    if kind(document, focused) != Some(ItemKind::Area) {
        return beside_project(document, focused, side);
    }
    match side {
        // From the document, since a collapsed area lists none of its projects (D-67).
        Side::Below => {
            let children = document.children(focused);
            let first = children
                .iter()
                .position(|&child| listed(document, child))
                .unwrap_or(children.len());
            Some((Some(focused), first))
        }
        Side::Above => match at.checked_sub(1).and_then(|before| entries.get(before)) {
            Some(&Stop::Place(before)) if kind(document, before.id) == Some(ItemKind::Area) => {
                Some((Some(before.id), document.children(before.id).len()))
            }
            Some(&Stop::Place(before)) => beside_project(document, before.id, Side::Below),
            None | Some(Stop::View(_)) => Some(end),
        },
    }
}

/// The parent and index of a new project on `side` of the outermost project holding `id`, or
/// `id` itself, since v1 nests no project in another (D-57), though a file may hold one.
fn beside_project(document: &Document, id: ItemId, side: Side) -> Option<(Option<ItemId>, usize)> {
    let outermost = container::path(document, id)
        .into_iter()
        .find(|&id| kind(document, id) == Some(ItemKind::Project))?;
    let (parent, index) = document.position(outermost)?;
    Some((parent, index + usize::from(side == Side::Below)))
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

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

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
            listed.get(4..=6),
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

    // FR-VIEW-10, FR-ORG-11
    #[test]
    fn later_projects_are_listed_and_moved_after_the_others() -> Result<(), PlacementError> {
        let mut document = document();
        let site = find(&document, "Site");
        let later = document.node(site).and_then(Node::item).cloned();
        let later = later.map(|it| Item { later: true, ..it });
        document.set(site, later.ok_or(PlacementError::NoSuchNode(site))?)?;
        let order = [
            "Inbox", "Solo", "Work", "  Docs", "  Site", "    Blog", "Home",
        ];
        assert_eq!(tree(&document), order);
        let docs = find(&document, "Docs");
        apply(&mut document, |d| shift(d, docs, Direction::Up))?;
        apply(&mut document, |d| shift(d, site, Direction::Down))?;
        assert_eq!(tree(&document), order, "neither passes the other");
        Ok(())
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

    // FR-VIEW-12, D-107
    #[test]
    fn a_pinned_entry_moves_only_past_the_pinned_sibling_beside_it() -> Result<(), PlacementError> {
        let mut document = document();
        let titles = ["Site", "Blog", "Docs", "Solo"].map(|title| find(&document, title));
        for id in titles {
            pin(&mut document, id, true)?;
        }
        let [site, blog, docs, solo] = titles;
        let pinned = |id| Sidebar {
            focused: Some(Entry::Stop {
                shown: Shown::Container(id),
                in_pinned: true,
            }),
            ..Sidebar::default()
        };
        for (id, direction) in [
            (site, Direction::Down),
            (docs, Direction::Up),
            (docs, Direction::Down),
        ] {
            assert_eq!(
                pinned(id).shift(&document, id, direction),
                Err(Refusal::NotPinnedSibling),
                "the pinned project beside it is no sibling"
            );
        }
        assert_eq!(
            Sidebar::open(Shown::Container(site)).shift(&document, site, Direction::Down),
            Ok(shift(&document, site, Direction::Down)),
            "its entry in the tree moves as an unpinned project's"
        );
        assert_eq!(
            pinned(solo).shift(&document, solo, Direction::Down),
            Ok(Transaction::default()),
            "the last of the pinned"
        );
        pin(&mut document, blog, false)?;
        let moved = |document: &Document, direction| {
            pinned(site)
                .shift(document, site, direction)
                .unwrap_or_default()
        };
        apply(&mut document, |d| moved(d, Direction::Down))?;
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Docs", "Site", "Solo", "Solo", "Work", "  Docs", "  Site", "    Blog",
                "Home"
            ],
            "past its pinned sibling in both lists"
        );
        apply(&mut document, |d| moved(d, Direction::Up))?;
        assert_eq!(
            tree(&document).get(1..3),
            Some(&["Site".to_owned(), "Docs".to_owned()][..]),
            "and back"
        );
        let later = document.node(docs).and_then(Node::item).cloned();
        let later = later.map(|it| Item { later: true, ..it });
        document.set(docs, later.ok_or(PlacementError::NoSuchNode(docs))?)?;
        assert_eq!(
            pinned(site).shift(&document, site, Direction::Down),
            Err(Refusal::NotPinnedSibling),
            "a Later sibling is listed apart"
        );
        Ok(())
    }

    /// Where a new project goes from the entry titled `focused`, as the titles of the parent and
    /// of the sibling it goes before, if any.
    fn placed(document: &Document, focused: Option<&str>, side: Side) -> Option<(String, String)> {
        let focused = focused.map(|title| Shown::Container(find(document, title)));
        let (parent, index) = place(document, &entries(document), focused, side)?;
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

    // FR-VIEW-10, FR-EDIT-05, FR-ORG-06, D-127
    #[test]
    fn a_new_project_goes_beside_the_focused_project_or_first_in_the_focused_area() {
        let document = document();
        let new = |focused, side| placed(&document, Some(focused), side);
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
        assert_eq!(new("Solo", Side::Below), Some(at("", "Home")));
        assert_eq!(
            placed(&document, None, Side::Above),
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
                Side::Below
            ),
            None,
            "from no entry"
        );
    }

    // FR-VIEW-10, FR-EDIT-05, FR-ORG-06, D-67, D-127
    #[test]
    fn below_a_collapsed_area_a_new_project_still_goes_first_in_it() {
        let document = document();
        let work = find(&document, "Work");
        let mut sidebar = Sidebar::open(Shown::Container(work));
        sidebar.toggle_collapsed(&document);
        let (parent, index) = place(
            &document,
            &sidebar.stops(&document),
            Some(Shown::Container(work)),
            Side::Below,
        )
        .expect("Work is listed");
        assert_eq!(parent, Some(work));
        assert_eq!(
            document.children(work).get(index),
            Some(&find(&document, "Site"))
        );
    }

    // FR-VIEW-10, FR-EDIT-05, FR-ORG-06, D-127
    #[test]
    fn above_an_area_a_new_project_goes_where_the_entry_above_stands() {
        let mut document = document();
        let new = |document: &Document, focused| placed(document, Some(focused), Side::Above);
        assert_eq!(
            new(&document, "Home"),
            Some(at("Work", "")),
            "below Docs, last in Work"
        );
        assert_eq!(
            new(&document, "Work"),
            Some(at("", "Home")),
            "below Solo, last among the projects in no area"
        );
        add(&mut document, None, Area, "Garden");
        assert_eq!(
            new(&document, "Garden"),
            Some(at("Home", "")),
            "last in the empty area above"
        );
        let mut alone = Document::new();
        add(&mut alone, None, Area, "Work");
        assert_eq!(
            new(&alone, "Work"),
            Some(at("", "")),
            "below the views, at the top level"
        );
        let entries = entries(&document);
        for from in [Shown::Container(document.inbox()), Shown::View(View::Today)] {
            assert_eq!(
                place(&document, &entries, Some(from), Side::Below),
                Some((None, 0)),
                "first of the projects from the Inbox and the views listed with it"
            );
        }
    }

    /// The rows of `document` with no group folded and no area collapsed.
    fn rows(document: &Document) -> Vec<SidebarRow> {
        Sidebar::default().visible(document)
    }

    /// The title of `row` of `document`: its heading's, its view's or its container's.
    fn titled(document: &Document, row: SidebarRow) -> String {
        match row {
            SidebarRow::Heading(group) => group.title().to_owned(),
            SidebarRow::Stop(Stop::Place(place)) => document
                .node(place.id)
                .and_then(Node::item)
                .map_or_else(String::new, |item| item.title.clone()),
            SidebarRow::Stop(Stop::View(view)) => view.title().to_owned(),
        }
    }

    /// The title of the row focused in `sidebar` after `motion`.
    fn moved(document: &Document, sidebar: &mut Sidebar, motion: Focus) -> Option<String> {
        let rows = sidebar.visible(document);
        sidebar.move_focus(&rows, motion);
        sidebar.settle(&rows);
        let view = sidebar.view(document)?;
        let row = view.entries.get(view.focused?)?;
        Some(titled(document, *row))
    }

    /// The title of each row `sidebar` lists of `document`.
    fn titles(sidebar: &Sidebar, document: &Document) -> Vec<String> {
        (sidebar.visible(document).into_iter())
            .map(|row| titled(document, row))
            .collect()
    }

    /// Each row's title with the count `sidebar` gives it in `document` on `today`.
    fn counted(sidebar: &Sidebar, document: &Document, today: Date) -> Vec<(String, usize)> {
        let entries = sidebar.visible(document);
        let counts = sidebar.counts(document, &entries, today);
        let titles = entries.iter().map(|&row| titled(document, row));
        titles.zip(counts).collect()
    }

    // FR-VIEW-10
    #[test]
    fn counts_the_tasks_of_each_entry_again_after_a_change_or_on_another_day() {
        let today = date(2026, 10, 14);
        let mut document = document();
        let mut sidebar = Sidebar::open(Shown::View(View::Today));
        assert_eq!(
            sidebar.view(&document).map(|view| view.counts),
            Some(Vec::new()),
            "counted only when asked"
        );
        let count = |sidebar: &Sidebar, document: &Document, today, title: &str| {
            counted(sidebar, document, today)
                .into_iter()
                .find(|(entry, _)| entry == title)
                .map(|(_, count)| count)
        };
        assert_eq!(
            counted(&sidebar, &document, today),
            [
                ("Inbox", 0),
                ("Views", 0),
                ("Today", 0),
                ("Upcoming", 0),
                ("Flagged", 0),
                ("All", 1),
                ("Logbook", 0),
                ("Trash", 0),
                ("Projects", 0),
                ("Solo", 0),
                ("Work", 1),
                ("Site", 0),
                ("Blog", 0),
                ("Docs", 0),
                ("Home", 0),
            ]
            .map(|(title, count)| (title.to_owned(), count))
        );
        assert!(!sidebar.stale(&document, today));
        let mut tomorrow = Item::new(Task, "Tomorrow");
        tomorrow.start = Some(date(2026, 10, 15));
        document
            .push(Some(document.inbox()), Node::Item(tomorrow))
            .expect("a task goes in the Inbox");
        assert!(sidebar.stale(&document, today), "changed");
        assert_eq!(
            count(&sidebar, &document, today, "All"),
            Some(1),
            "as last counted"
        );
        assert_eq!(count(&sidebar, &document, today, "Inbox"), Some(0));
        sidebar.recount(&document, today);
        assert!(!sidebar.stale(&document, today));
        assert_eq!(count(&sidebar, &document, today, "All"), Some(2));
        assert_eq!(count(&sidebar, &document, today, "Upcoming"), Some(1));
        assert_eq!(count(&sidebar, &document, today, "Today"), Some(0));
        let next_day = date(2026, 10, 15);
        assert!(sidebar.stale(&document, next_day), "another day");
        sidebar.recount(&document, next_day);
        assert_eq!(count(&sidebar, &document, next_day, "Today"), Some(1));
        assert_eq!(count(&sidebar, &document, next_day, "Upcoming"), Some(0));
    }

    // FR-VIEW-10
    #[test]
    fn a_reload_counts_the_document_read_again_whatever_its_revision() {
        let today = date(2026, 10, 14);
        let mut before = Document::new();
        let inbox = before.inbox();
        add(&mut before, Some(inbox), Task, "a");
        let mut after = Document::new();
        add(&mut after, None, Area, "Home");
        assert_eq!(before.revision(), after.revision());
        let all = |sidebar: &Sidebar, document: &Document| {
            counted(sidebar, document, today)
                .into_iter()
                .find(|(title, _)| title == "All")
                .map(|(_, count)| count)
        };
        let mut sidebar = Sidebar::open(Shown::View(View::All));
        assert_eq!(all(&sidebar, &before), Some(1));
        sidebar.rematch(|_| None);
        assert_eq!(all(&sidebar, &after), Some(0));
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
        assert_eq!(go(Focus::Next).as_deref(), Some("Views"), "the heading");
        for &view in &views {
            assert_eq!(go(Focus::Next).as_deref(), Some(view));
        }
        assert_eq!(go(Focus::Next).as_deref(), Some("Projects"));
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
            Some((Some(0), true, 9 + views.len()))
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
                .stops(&document)
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

    // FR-VIEW-10, D-144
    #[test]
    fn a_group_folds_to_its_heading_until_it_unfolds_or_its_entry_is_focused()
    -> Result<(), PlacementError> {
        let mut document = document();
        let solo = find(&document, "Solo");
        pin(&mut document, solo, true)?;
        let mut sidebar = Sidebar::default();
        sidebar.toggle(None);
        assert_eq!(
            moved(&document, &mut sidebar, Focus::Next).as_deref(),
            Some("Views")
        );
        sidebar.toggle_collapsed(&document);
        assert_eq!(
            titles(&sidebar, &document).get(..4),
            Some(&["Inbox", "Views", "Pinned", "Solo"].map(String::from)[..]),
            "the views hide under their heading"
        );
        assert_eq!(
            sidebar.view(&document).map(|view| view.folded),
            Some(vec![SidebarGroup::Views])
        );
        for _ in 0..3 {
            moved(&document, &mut sidebar, Focus::Next);
        }
        assert_eq!(
            moved(&document, &mut sidebar, Focus::Previous).as_deref(),
            Some("Solo")
        );
        moved(&document, &mut sidebar, Focus::Previous);
        sidebar.toggle_collapsed(&document);
        moved(&document, &mut sidebar, Focus::Next);
        sidebar.toggle_collapsed(&document);
        assert_eq!(
            titles(&sidebar, &document),
            [
                "Inbox", "Views", "Pinned", "Projects", "Work", "Site", "Blog", "Docs", "Home"
            ],
            "every group but the areas folds; an entry is no heading and folds nothing"
        );
        let view = sidebar.view(&document).map(|view| view.folded);
        assert_eq!(
            view,
            Some(vec![
                SidebarGroup::Views,
                SidebarGroup::Pinned,
                SidebarGroup::Projects
            ])
        );
        sidebar.reveal(&document);
        assert_eq!(sidebar.folded.len(), 3, "a heading focused unfolds nothing");
        sidebar.focus_new(solo);
        sidebar.reveal(&document);
        assert_eq!(
            sidebar.folded,
            BTreeSet::from([SidebarGroup::Views, SidebarGroup::Pinned]),
            "the projects in no area unfold to show the entry focused there"
        );
        moved(&document, &mut sidebar, Focus::First);
        moved(&document, &mut sidebar, Focus::Next);
        sidebar.toggle_collapsed(&document);
        sidebar.reveal(&document);
        assert_eq!(sidebar.folded, BTreeSet::from([SidebarGroup::Pinned]));
        assert_eq!(
            sidebar.stops(&document).len(),
            entries(&document).len(),
            "the entries beside which a new project goes take no fold"
        );
        Ok(())
    }

    // FR-VIEW-10
    #[test]
    fn the_focus_stays_where_an_entry_left() -> Result<(), PlacementError> {
        let mut document = document();
        let mut sidebar = Sidebar::default();
        let docs = find(&document, "Docs");
        sidebar.toggle(Some(Shown::Container(docs)));
        sidebar.settle(&rows(&document));
        let mut gone = Item::new(Project, "Docs");
        gone.deleted = Some(date(2026, 10, 2));
        document.set(docs, gone)?;
        sidebar.settle(&rows(&document));
        assert_eq!(
            sidebar.focused(),
            Some(Shown::Container(find(&document, "Home")))
        );
        let mut hidden = Sidebar::default();
        hidden.settle(&rows(&document));
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
        sidebar.settle(&rows(&document));
        let at = sidebar.index;
        sidebar.rematch(|id| (id == work).then_some(home));
        assert_eq!(sidebar.focused(), Some(Shown::Container(home)));
        assert_eq!(sidebar.left, Some(Entry::of(Shown::View(View::Today))));
        assert_eq!(sidebar.collapsed, BTreeSet::from([home]));
        sidebar.rematch(|_| None);
        assert_eq!(sidebar.left, Some(Entry::of(Shown::View(View::Today))));
        assert!(sidebar.collapsed.is_empty());
        let entries = rows(&document);
        sidebar.settle(&entries);
        assert_eq!(
            sidebar.focused(),
            entries[at].shown(),
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

    /// Pins or unpins the project `id` of `document`.
    fn pin(document: &mut Document, id: ItemId, pinned: bool) -> Result<(), PlacementError> {
        let item = document.node(id).and_then(Node::item).cloned();
        let item = item.ok_or(PlacementError::NoSuchNode(id))?;
        document.set(id, Item { pinned, ..item })?;
        Ok(())
    }

    // FR-VIEW-12
    #[test]
    fn pinned_projects_are_listed_again_after_the_views_each_entry_focused_apart()
    -> Result<(), PlacementError> {
        let mut document = document();
        let (blog, solo) = (find(&document, "Blog"), find(&document, "Solo"));
        let gone = document
            .top_level()
            .get(1)
            .copied()
            .ok_or(PlacementError::DoesNotFit)?;
        for id in [solo, blog, gone] {
            pin(&mut document, id, true)?;
        }
        assert_eq!(
            tree(&document),
            [
                "Inbox", "Blog", "Solo", "Solo", "Work", "  Site", "    Blog", "  Docs", "Home"
            ],
            "in the order of the file, without the Trash"
        );
        let views = REGISTRY.len();
        let mut sidebar = Sidebar::default();
        sidebar.toggle(Some(Shown::Container(solo)));
        let focused = |sidebar: &Sidebar, document: &Document| {
            sidebar.view(document).and_then(|view| view.focused)
        };
        assert_eq!(
            focused(&sidebar, &document),
            Some(views + 6),
            "the project's entry in the tree, under the Projects heading"
        );
        assert_eq!(
            moved(&document, &mut sidebar, Focus::Previous).as_deref(),
            Some("Projects")
        );
        assert_eq!(
            moved(&document, &mut sidebar, Focus::Previous).as_deref(),
            Some("Solo")
        );
        assert_eq!(
            focused(&sidebar, &document),
            Some(views + 4),
            "its entry among the pinned"
        );
        assert_eq!(
            moved(&document, &mut sidebar, Focus::Previous).as_deref(),
            Some("Blog")
        );
        assert_eq!(
            moved(&document, &mut sidebar, Focus::Next).as_deref(),
            Some("Solo")
        );
        assert_eq!(focused(&sidebar, &document), Some(views + 4));
        pin(&mut document, solo, false)?;
        sidebar.settle(&rows(&document));
        assert_eq!(
            (sidebar.focused(), focused(&sidebar, &document)),
            (None, Some(views + 4)),
            "unpinned, the focus stays where its entry was, now the Projects heading"
        );
        Ok(())
    }
}
