//! Where a new item goes: a task below or above the focused row, among the tasks it is shown
//! with, a section or project as near the focus as its kind can go, or any of them first or
//! last among its kind in the container shown (FR-EDIT-05, FR-ORG-06, D-40). Hides how a place in
//! the rows maps to a place in the document.

use super::container::Row;
use crate::domain::{Document, ItemId, ItemKind, Node};

/// Which side of the focused row a new task goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Right after it.
    Below,
    /// Right before it.
    Above,
}

/// Which end of its kind's items in a container a new item goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum End {
    /// Before the first of them.
    Top,
    /// After the last of them.
    Bottom,
}

/// The parent and the index among its children where a task added on `side` of `focused` goes,
/// when `container` shows `rows`. Next to a task it goes beside that task. Below a section
/// heading it is the section's first task, and above one it goes after the task row above, or
/// into the section above when that shows none. Beside a child project, which no task stands
/// among, and in a container without rows, it is the container's own last task.
pub(super) fn place(
    document: &Document,
    container: ItemId,
    rows: &[Row],
    focused: Option<ItemId>,
    side: Side,
) -> (ItemId, usize) {
    let at = focused.and_then(|id| rows.iter().position(|row| row.id() == id));
    let Some(at) = at else {
        return last_task(document, container);
    };
    let beside = |id: ItemId, side: Side| {
        let (parent, index) = document
            .position(id)
            .expect("a row shows an item of the document");
        let parent = parent.expect("a task is always inside a container");
        (parent, index + usize::from(side == Side::Below))
    };
    match (rows.get(at), side) {
        (Some(&Row::Task { id, .. }), side) => beside(id, side),
        (Some(&Row::Section(section)), Side::Below) => (section, 0),
        (Some(&Row::Section(_)), Side::Above) => {
            match at.checked_sub(1).and_then(|before| rows.get(before)) {
                Some(&Row::Task { id, .. }) => beside(id, Side::Below),
                Some(&Row::Section(section)) => last_task(document, section),
                _ => last_task(document, container),
            }
        }
        _ => last_task(document, container),
    }
}

/// The parent, `None` for the top level, and the index among its children where a new item of
/// `kind`, a section or project, goes on `side` of `focused` when `container` shows `rows`;
/// `None` when it can go nowhere near, as a section from the Inbox.
///
/// A section or project goes into `container` when it can hold it, nearest the focused row in the
/// arrangement of FR-VIEW-05, a task listed under a section standing for the section: beside it
/// when it is of the same kind, first among the items of its kind when it comes before them and
/// last otherwise. When `container` cannot hold it, it goes the same way beside `container` in the
/// nearest container above that can, or at the top level, first when coming from the Inbox.
pub(super) fn container_place(
    document: &Document,
    container: ItemId,
    rows: &[Row],
    focused: Option<ItemId>,
    kind: ItemKind,
    side: Side,
) -> Option<(Option<ItemId>, usize)> {
    if fits(document, kind, Some(container)) {
        let anchor = rows
            .iter()
            .find(|row| Some(row.id()) == focused)
            .map(|&row| match row {
                Row::Task {
                    section: Some(section),
                    ..
                } => section,
                row => row.id(),
            });
        return Some((
            Some(container),
            in_group(document, container, kind, anchor, side),
        ));
    }
    let mut anchor = container;
    loop {
        let parent = document.position(anchor).and_then(|(parent, _)| parent);
        if fits(document, kind, parent) {
            return Some(match parent {
                Some(parent) => (
                    Some(parent),
                    in_group(document, parent, kind, Some(anchor), side),
                ),
                // Only the Inbox stands outside the top level, a project from it going first.
                None => (None, 0),
            });
        }
        anchor = parent?;
    }
}

/// The parent, `None` for the top level, and the index among its children where a new item of
/// `kind` goes at `end` of the items of its kind in `container`: first or last among them, or
/// with none, as the arrangement of FR-VIEW-05 lists it whatever its index, last. When
/// `container` cannot hold it, it goes the same way in the nearest container above that can, or
/// at the top level; `None` when nothing can, as a section from the Inbox.
pub(super) fn end_place(
    document: &Document,
    container: ItemId,
    kind: ItemKind,
    end: End,
) -> Option<(Option<ItemId>, usize)> {
    let mut parent = Some(container);
    while !fits(document, kind, parent) {
        // The Inbox has no place among the others, but stands for the top level.
        parent = document.position(parent?).and_then(|(parent, _)| parent);
    }
    let children = parent.map_or(document.top_level(), |parent| document.children(parent));
    let of_kind = |id: &ItemId| {
        document
            .node(*id)
            .and_then(Node::item)
            .is_some_and(|item| item.kind == kind)
    };
    let index = match end {
        End::Top => children.iter().position(of_kind),
        End::Bottom => children.iter().rposition(of_kind).map(|at| at + 1),
    };
    Some((parent, index.unwrap_or(children.len())))
}

/// Whether an item of `kind` may go in `parent`, or at the top level for `None`.
fn fits(document: &Document, kind: ItemKind, parent: Option<ItemId>) -> bool {
    match parent {
        Some(id) => document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| kind.fits_in(Some(item.kind))),
        None => kind.fits_in(None),
    }
}

/// The index among `parent`'s children where a new item of `kind` goes nearest `anchor`, one of
/// them: beside it on `side` when it is of `kind`, first among the children of `kind` when it is
/// shown before them, and after the last of them otherwise. With no children of `kind`, it goes
/// last.
fn in_group(
    document: &Document,
    parent: ItemId,
    kind: ItemKind,
    anchor: Option<ItemId>,
    side: Side,
) -> usize {
    let kind_of = |id: ItemId| document.node(id).and_then(Node::item).map(|item| item.kind);
    let children = document.children(parent);
    let of_kind = |id: &ItemId| kind_of(*id) == Some(kind);
    let anchor = anchor.and_then(|id| {
        Some((
            children.iter().position(|&child| child == id)?,
            kind_of(id)?,
        ))
    });
    match anchor {
        Some((at, anchor)) if anchor == kind => at + usize::from(side == Side::Below),
        Some((_, anchor)) if shown_order(anchor) < shown_order(kind) => {
            children.iter().position(of_kind).unwrap_or(children.len())
        }
        _ => children
            .iter()
            .rposition(of_kind)
            .map_or(children.len(), |at| at + 1),
    }
}

/// Where the rows of `kind` come in a container's arrangement: tasks, then sections, then
/// projects (FR-VIEW-05).
fn shown_order(kind: ItemKind) -> u8 {
    match kind {
        ItemKind::Section => 1,
        ItemKind::Project => 2,
        _ => 0,
    }
}

/// The place after the last task among `parent`'s children, or its first place when it holds
/// none.
fn last_task(document: &Document, parent: ItemId) -> (ItemId, usize) {
    let is_task = |id: &ItemId| {
        document
            .node(*id)
            .and_then(Node::item)
            .is_some_and(|item| item.kind == ItemKind::Task)
    };
    let children = document.children(parent);
    (
        parent,
        children.iter().rposition(is_task).map_or(0, |at| at + 1),
    )
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::{
        app::container::rows,
        domain::{Completion, Item, PlacementError},
    };

    /// A project holding a loose task, a completed task, a section holding a task, an empty
    /// section and a child project.
    struct Fixture {
        document: Document,
        project: ItemId,
        loose: ItemId,
        section: ItemId,
        in_section: ItemId,
        empty: ItemId,
        child: ItemId,
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let add = |document: &mut Document, parent, kind, title| {
            document.push(Some(parent), Node::Item(Item::new(kind, title)))
        };
        let project = document.push(None, Node::Item(Item::new(ItemKind::Project, "p")))?;
        let loose = add(&mut document, project, ItemKind::Task, "a")?;
        let mut done = Item::new(ItemKind::Task, "done");
        done.completed = Some(Completion::On(date(2026, 10, 1)));
        document.push(Some(project), Node::Item(done))?;
        let section = add(&mut document, project, ItemKind::Section, "s")?;
        let in_section = add(&mut document, section, ItemKind::Task, "b")?;
        let empty = add(&mut document, project, ItemKind::Section, "t")?;
        let child = add(&mut document, project, ItemKind::Project, "c")?;
        Ok(Fixture {
            document,
            project,
            loose,
            section,
            in_section,
            empty,
            child,
        })
    }

    fn at(fixture: &Fixture, focused: Option<ItemId>, side: Side) -> (ItemId, usize) {
        let rows = rows(&fixture.document, fixture.project, false);
        place(&fixture.document, fixture.project, &rows, focused, side)
    }

    // FR-EDIT-05
    #[test]
    fn beside_a_task_it_goes_right_after_or_before_it() -> Result<(), PlacementError> {
        let fixture = fixture()?;
        assert_eq!(
            at(&fixture, Some(fixture.loose), Side::Below),
            (fixture.project, 1)
        );
        assert_eq!(
            at(&fixture, Some(fixture.loose), Side::Above),
            (fixture.project, 0)
        );
        assert_eq!(
            at(&fixture, Some(fixture.in_section), Side::Below),
            (fixture.section, 1)
        );
        assert_eq!(
            at(&fixture, Some(fixture.in_section), Side::Above),
            (fixture.section, 0)
        );
        Ok(())
    }

    // FR-EDIT-05
    #[test]
    fn at_a_section_heading_it_goes_first_in_it_or_after_the_row_above()
    -> Result<(), PlacementError> {
        let fixture = fixture()?;
        assert_eq!(
            at(&fixture, Some(fixture.section), Side::Below),
            (fixture.section, 0)
        );
        assert_eq!(
            at(&fixture, Some(fixture.section), Side::Above),
            (fixture.project, 1)
        );
        assert_eq!(
            at(&fixture, Some(fixture.empty), Side::Above),
            (fixture.section, 1)
        );
        assert_eq!(
            at(&fixture, Some(fixture.empty), Side::Below),
            (fixture.empty, 0)
        );
        Ok(())
    }

    // FR-EDIT-05
    #[test]
    fn at_a_child_project_or_with_nothing_focused_it_is_the_containers_last_task()
    -> Result<(), PlacementError> {
        let mut fixture = fixture()?;
        for side in [Side::Below, Side::Above] {
            assert_eq!(
                at(&fixture, Some(fixture.child), side),
                (fixture.project, 2)
            );
            assert_eq!(at(&fixture, None, side), (fixture.project, 2));
        }
        // Above a first row that is a section heading, past tasks the pane hides.
        let mut gone = Item::new(ItemKind::Task, "a");
        gone.deleted = Some(date(2026, 10, 1));
        fixture.document.set(fixture.loose, gone)?;
        let mut done = Item::new(ItemKind::Task, "b");
        done.completed = Some(Completion::Undated);
        fixture.document.set(fixture.in_section, done)?;
        assert_eq!(
            at(&fixture, Some(fixture.section), Side::Above),
            (fixture.project, 2)
        );
        assert_eq!(
            at(&fixture, Some(fixture.empty), Side::Above),
            (fixture.section, 1)
        );
        let empty = Node::Item(Item::new(ItemKind::Section, "e"));
        let empty = fixture.document.push(Some(fixture.project), empty)?;
        let rows = rows(&fixture.document, empty, false);
        assert_eq!(
            place(&fixture.document, empty, &rows, None, Side::Below),
            (empty, 0)
        );
        Ok(())
    }

    fn container_at(
        fixture: &Fixture,
        shown: ItemId,
        focused: Option<ItemId>,
        kind: ItemKind,
        side: Side,
    ) -> Option<(Option<ItemId>, usize)> {
        let rows = rows(&fixture.document, shown, false);
        container_place(&fixture.document, shown, &rows, focused, kind, side)
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn a_section_goes_among_the_sections_nearest_the_focus() -> Result<(), PlacementError> {
        let fixture = fixture()?;
        let (p, section) = (fixture.project, ItemKind::Section);
        let at = |focused, side| container_at(&fixture, p, focused, section, side);
        for side in [Side::Below, Side::Above] {
            assert_eq!(at(Some(fixture.loose), side), Some((Some(p), 2)));
        }
        assert_eq!(
            at(Some(fixture.in_section), Side::Below),
            Some((Some(p), 3))
        );
        assert_eq!(at(Some(fixture.section), Side::Above), Some((Some(p), 2)));
        assert_eq!(at(Some(fixture.empty), Side::Below), Some((Some(p), 4)));
        assert_eq!(at(Some(fixture.child), Side::Above), Some((Some(p), 4)));
        assert_eq!(at(None, Side::Above), Some((Some(p), 4)));
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn a_project_goes_among_the_child_projects_nearest_the_focus() -> Result<(), PlacementError> {
        let fixture = fixture()?;
        let (p, project) = (fixture.project, ItemKind::Project);
        let at = |focused, side| container_at(&fixture, p, focused, project, side);
        for focused in [fixture.loose, fixture.in_section, fixture.empty] {
            assert_eq!(at(Some(focused), Side::Below), Some((Some(p), 4)));
        }
        assert_eq!(at(Some(fixture.child), Side::Below), Some((Some(p), 5)));
        assert_eq!(at(Some(fixture.child), Side::Above), Some((Some(p), 4)));
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn where_the_container_shown_cannot_hold_it_it_goes_beside_it_further_up()
    -> Result<(), PlacementError> {
        let fixture = fixture()?;
        let (p, inbox) = (fixture.project, fixture.document.inbox());
        let section = fixture.section;
        let at = |shown, kind, side| container_at(&fixture, shown, None, kind, side);
        assert_eq!(
            at(section, ItemKind::Section, Side::Below),
            Some((Some(p), 3))
        );
        assert_eq!(
            at(section, ItemKind::Section, Side::Above),
            Some((Some(p), 2))
        );
        assert_eq!(
            at(section, ItemKind::Project, Side::Above),
            Some((Some(p), 4))
        );
        assert_eq!(at(inbox, ItemKind::Project, Side::Below), Some((None, 0)));
        assert_eq!(at(inbox, ItemKind::Section, Side::Below), None);
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn the_first_of_its_kind_goes_last() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let project = document.push(None, Node::Item(Item::new(ItemKind::Project, "a")))?;
        let task = document.push(Some(project), Node::Item(Item::new(ItemKind::Task, "t")))?;
        let rows = rows(&document, project, false);
        for kind in [ItemKind::Section, ItemKind::Project] {
            assert_eq!(
                container_place(&document, project, &rows, Some(task), kind, Side::Above),
                Some((Some(project), 1))
            );
        }
        Ok(())
    }
}
