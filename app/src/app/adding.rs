//! Where a new task goes: below or above the focused row, among the tasks it is shown with
//! (FR-EDIT-05). Hides how a place in the rows maps to a place in the document.

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
}
