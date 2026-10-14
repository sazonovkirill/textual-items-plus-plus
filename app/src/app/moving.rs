//! Moving and indenting: where moving an item up or down among the rows it is shown with, and
//! indenting or outdenting it, puts it in the document (FR-OPS-02, FR-OPS-03, FR-OPS-10). Hides
//! how a place in the rows maps to a place in the document.

use super::container::Row;
use crate::domain::{Destination, Document, Item, ItemId, ItemKind, Node, Transaction};

/// Which way an item moves among its rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Toward the top.
    Up,
    /// Toward the bottom.
    Down,
}

/// Why an item was not indented or outdented, for the status line to say (FR-OPS-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Only projects and checklist items nest in items of their own kind.
    DoesNotNest,
    /// No item of its kind is above it to go into.
    NothingAbove,
    /// It is not inside an item of its own kind to come out of.
    NotNested,
}

/// The transaction that moves `id` one step `direction` among `rows`, the rows of the container
/// shown; an empty one when it has nowhere to go. An item changes places with the next item
/// shown in its own group, a task among the tasks listed under the same heading, a section among
/// the sections and a child project among the child projects, completed and open ones apart
/// (FR-VIEW-11). Only a task crosses the edge of its group: past the heading below it, it is the
/// first task of that section, and up from the top of a section it is the last task of the
/// section above, or of those outside sections (FR-OPS-10).
pub(super) fn shift(
    document: &Document,
    rows: &[Row],
    id: ItemId,
    direction: Direction,
) -> Transaction {
    let Some((at, &row)) = rows.iter().enumerate().find(|(_, row)| row.id() == id) else {
        return Transaction::default();
    };
    let completed = |row: &Row| item(document, row.id()).is_some_and(|it| it.completed.is_some());
    let peer = |other: &&Row| {
        let same_group = match (row, **other) {
            (
                Row::Task { section, .. },
                Row::Task {
                    section: theirs, ..
                },
            ) => section == theirs,
            (Row::Section(_), Row::Section(_)) | (Row::Project(_), Row::Project(_)) => true,
            _ => false,
        };
        same_group && completed(other) == completed(&row)
    };
    let (earlier, later) = rows.split_at(at);
    let later = later.get(1..).unwrap_or_default();
    let is_open_section = |row: &&Row| matches!(row, Row::Section(id) if !document.conceals(*id));
    let destination = match direction {
        Direction::Down => later
            .iter()
            .find(peer)
            .map(|row| Destination::After(row.id())),
        Direction::Up => earlier
            .iter()
            .rfind(peer)
            .map(|row| Destination::Before(row.id())),
    }
    .or_else(|| {
        let Row::Task { section, .. } = row else {
            return None;
        };
        match direction {
            Direction::Down => later
                .iter()
                .find(is_open_section)
                .map(|row| Destination::First(row.id())),
            Direction::Up => {
                let section = section?;
                let heading = earlier.iter().position(|row| row.id() == section)?;
                let above = earlier.get(..heading).unwrap_or_default();
                Some(match above.iter().rfind(is_open_section) {
                    Some(row) => Destination::Last(Some(row.id())),
                    None => outside_sections(document, section),
                })
            }
        }
    });
    destination.map_or_else(Transaction::default, |to| {
        document
            .move_to(id, to)
            .expect("a row moves only among rows of its own kind, which hold it")
    })
}

/// Where a task goes to be the last of the tasks outside sections, in the container holding
/// `section`: after the last task there, or first when it holds none.
fn outside_sections(document: &Document, section: ItemId) -> Destination {
    let container = document
        .position(section)
        .and_then(|(parent, _)| parent)
        .expect("a section is always inside a container");
    let last = document
        .children(container)
        .iter()
        .rfind(|&&id| item(document, id).is_some_and(|it| it.kind == ItemKind::Task));
    last.map_or(Destination::First(container), |&task| {
        Destination::After(task)
    })
}

/// The transaction that indents `id`, making it the last child of the nearest item of its kind
/// above it among its siblings that is neither in the Trash nor completed, with that item; only
/// a project or a checklist item indents (FR-OPS-03).
///
/// # Errors
///
/// The [`Refusal`] when the hierarchy does not allow it.
pub(super) fn indent(document: &Document, id: ItemId) -> Result<(Transaction, ItemId), Refusal> {
    let kind = nesting(document, id)?;
    let (parent, index) = document
        .position(id)
        .expect("an item that nests is not the Inbox, so it has a place");
    let siblings = match parent {
        Some(parent) => document.children(parent),
        None => document.top_level(),
    };
    let into = siblings
        .iter()
        .take(index)
        .rev()
        .copied()
        .find(|&above| {
            item(document, above)
                .is_some_and(|it| it.kind == kind && it.deleted.is_none() && it.completed.is_none())
        })
        .ok_or(Refusal::NothingAbove)?;
    let transaction = document
        .move_to(id, Destination::Last(Some(into)))
        .expect("an item of a kind that nests fits in another of its kind");
    Ok((transaction, into))
}

/// The transaction that outdents `id`, putting it right after the item of its own kind it is
/// in; only a project or a checklist item outdents (FR-OPS-03).
///
/// # Errors
///
/// The [`Refusal`] when the hierarchy does not allow it.
pub(super) fn outdent(document: &Document, id: ItemId) -> Result<Transaction, Refusal> {
    let kind = nesting(document, id)?;
    let parent = document
        .position(id)
        .and_then(|(parent, _)| parent)
        .filter(|&parent| item(document, parent).is_some_and(|it| it.kind == kind))
        .ok_or(Refusal::NotNested)?;
    Ok(document
        .move_to(id, Destination::After(parent))
        .expect("an item fits beside the item of its kind that holds it"))
}

/// The kind of `id` when it is an item that nests in its own kind.
fn nesting(document: &Document, id: ItemId) -> Result<ItemKind, Refusal> {
    item(document, id)
        .map(|it| it.kind)
        .filter(|kind| matches!(kind, ItemKind::Project | ItemKind::ChecklistItem))
        .ok_or(Refusal::DoesNotNest)
}

fn item(document: &Document, id: ItemId) -> Option<&Item> {
    document.node(id).and_then(Node::item)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::{
        app::container::rows,
        domain::{Completion, PlacementError},
    };
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    fn add(document: &mut Document, parent: Option<ItemId>, item: Item) -> ItemId {
        document
            .push(parent, Node::Item(item))
            .expect("the test builds a valid hierarchy")
    }

    fn done(kind: ItemKind, title: &str) -> Item {
        let mut item = Item::new(kind, title);
        item.completed = Some(Completion::On(date(2026, 10, 1)));
        item
    }

    /// An area holding tasks `a`, `b` and a completed `x` outside sections, a section `S`
    /// holding `c` and `d`, an empty section `T`, a section `U` holding `e`, and child projects
    /// `P` and `Q`.
    fn area() -> (Document, ItemId) {
        let mut document = Document::new();
        let area = add(&mut document, None, Item::new(Area, "A"));
        for title in ["a", "b"] {
            add(&mut document, Some(area), Item::new(Task, title));
        }
        add(&mut document, Some(area), done(Task, "x"));
        for (section, tasks) in [("S", &["c", "d"][..]), ("T", &[]), ("U", &["e"])] {
            let section = add(&mut document, Some(area), Item::new(Section, section));
            for &task in tasks {
                add(&mut document, Some(section), Item::new(Task, task));
            }
        }
        for title in ["P", "Q"] {
            add(&mut document, Some(area), Item::new(Project, title));
        }
        (document, area)
    }

    fn find(document: &Document, title: &str) -> ItemId {
        let mut pending = vec![document.inbox()];
        pending.extend(document.top_level());
        let mut found = None;
        while let Some(id) = pending.pop() {
            if item(document, id).is_some_and(|it| it.title == title) {
                found = Some(id);
            }
            pending.extend(document.children(id));
        }
        found.expect("the test's document holds the title")
    }

    /// The titles of `container`'s rows, section headings marked `#`.
    fn shown(document: &Document, container: ItemId, show_completed: bool) -> Vec<String> {
        rows(document, container, show_completed)
            .into_iter()
            .map(|row| {
                let title = &item(document, row.id()).map_or("", |it| &it.title);
                match row {
                    Row::Section(_) => format!("#{title}"),
                    _ => (*title).to_owned(),
                }
            })
            .collect()
    }

    /// Moves `title` in `container` and returns the rows shown after, checking undo.
    fn shifted(
        document: &mut Document,
        container: ItemId,
        title: &str,
        direction: Direction,
        show_completed: bool,
    ) -> Result<Vec<String>, PlacementError> {
        let id = find(document, title);
        let before = document.clone();
        let rows = rows(document, container, show_completed);
        let transaction = shift(document, &rows, id, direction);
        document.apply(&transaction)?;
        let unchanged = transaction.ops().is_empty();
        let mut undone = document.clone();
        undone.apply(&transaction.inverse())?;
        assert!(unchanged || undone == before, "undo restores the document");
        Ok(shown(document, container, show_completed))
    }

    // FR-OPS-02, FR-TASK-05
    #[test]
    fn a_task_changes_places_with_the_task_beside_it() -> Result<(), PlacementError> {
        let (mut document, area) = area();
        assert_eq!(
            shifted(&mut document, area, "a", Direction::Down, false)?,
            ["b", "a", "#S", "c", "d", "#T", "#U", "e", "P", "Q"]
        );
        assert_eq!(
            shifted(&mut document, area, "d", Direction::Up, false)?,
            ["b", "a", "#S", "d", "c", "#T", "#U", "e", "P", "Q"]
        );
        Ok(())
    }

    // FR-OPS-10
    #[test]
    fn a_task_moving_down_past_a_heading_is_first_in_that_section() -> Result<(), PlacementError> {
        let (mut document, area) = area();
        assert_eq!(
            shifted(&mut document, area, "b", Direction::Down, false)?,
            ["a", "#S", "b", "c", "d", "#T", "#U", "e", "P", "Q"]
        );
        assert_eq!(
            shifted(&mut document, area, "d", Direction::Down, false)?,
            ["a", "#S", "b", "c", "#T", "d", "#U", "e", "P", "Q"]
        );
        assert_eq!(
            shifted(&mut document, area, "d", Direction::Down, false)?,
            ["a", "#S", "b", "c", "#T", "#U", "d", "e", "P", "Q"]
        );
        let last = shifted(&mut document, area, "e", Direction::Down, false)?;
        assert_eq!(
            last,
            ["a", "#S", "b", "c", "#T", "#U", "d", "e", "P", "Q"],
            "the last task of the last section stays"
        );
        Ok(())
    }

    // FR-OPS-10
    #[test]
    fn a_task_moving_up_from_the_top_of_a_section_is_last_in_the_one_above()
    -> Result<(), PlacementError> {
        let (mut document, area) = area();
        assert_eq!(
            shifted(&mut document, area, "e", Direction::Up, false)?,
            ["a", "b", "#S", "c", "d", "#T", "e", "#U", "P", "Q"]
        );
        assert_eq!(
            shifted(&mut document, area, "e", Direction::Up, false)?,
            ["a", "b", "#S", "c", "d", "e", "#T", "#U", "P", "Q"]
        );
        assert_eq!(
            shifted(&mut document, area, "c", Direction::Up, false)?,
            ["a", "b", "c", "#S", "d", "e", "#T", "#U", "P", "Q"],
            "from the first section, last among the tasks outside sections"
        );
        assert_eq!(
            shifted(&mut document, area, "c", Direction::Up, true)?,
            ["a", "c", "b", "x", "#S", "d", "e", "#T", "#U", "P", "Q"]
        );
        assert_eq!(
            shifted(&mut document, area, "a", Direction::Up, false)?,
            ["a", "c", "b", "#S", "d", "e", "#T", "#U", "P", "Q"],
            "the first task stays"
        );
        Ok(())
    }

    // FR-OPS-10
    #[test]
    fn up_from_the_first_section_with_no_tasks_outside_sections_it_goes_first()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let project = add(&mut document, None, Item::new(Project, "p"));
        let section = add(&mut document, Some(project), Item::new(Section, "S"));
        let task = add(&mut document, Some(section), Item::new(Task, "a"));
        assert_eq!(
            shifted(&mut document, project, "a", Direction::Up, false)?,
            ["a", "#S"]
        );
        assert_eq!(document.position(task), Some((Some(project), 0)));
        Ok(())
    }

    // FR-OPS-10, FR-VIEW-11
    #[test]
    fn completed_and_open_tasks_move_apart_and_hidden_sections_are_passed()
    -> Result<(), PlacementError> {
        let (mut document, area) = area();
        let t = find(&document, "T");
        let section = item(&document, t).cloned().map(|it| Item {
            completed: done(Section, "T").completed,
            ..it
        });
        document.set(t, section.ok_or(PlacementError::NoSuchNode(t))?)?;
        assert_eq!(
            shifted(&mut document, area, "b", Direction::Down, true)?,
            ["a", "x", "#S", "b", "c", "d", "#U", "e", "#T", "P", "Q"],
            "the open task passes the completed one"
        );
        assert_eq!(
            shifted(&mut document, area, "d", Direction::Down, true)?,
            ["a", "x", "#S", "b", "c", "#U", "d", "e", "#T", "P", "Q"],
            "the completed section is passed"
        );
        assert_eq!(
            shifted(&mut document, area, "x", Direction::Up, true)?,
            ["a", "x", "#S", "b", "c", "#U", "d", "e", "#T", "P", "Q"],
            "the completed task stays below the open ones"
        );
        Ok(())
    }

    // FR-OPS-10, FR-ORG-06
    #[test]
    fn sections_move_among_sections_and_projects_among_projects() -> Result<(), PlacementError> {
        let (mut document, area) = area();
        assert_eq!(
            shifted(&mut document, area, "S", Direction::Up, false)?,
            ["a", "b", "#S", "c", "d", "#T", "#U", "e", "P", "Q"],
            "never above the tasks outside sections"
        );
        assert_eq!(
            shifted(&mut document, area, "S", Direction::Down, false)?,
            ["a", "b", "#T", "#S", "c", "d", "#U", "e", "P", "Q"]
        );
        assert_eq!(
            shifted(&mut document, area, "U", Direction::Up, false)?,
            ["a", "b", "#T", "#U", "e", "#S", "c", "d", "P", "Q"]
        );
        assert_eq!(
            shifted(&mut document, area, "S", Direction::Down, false)?,
            ["a", "b", "#T", "#U", "e", "#S", "c", "d", "P", "Q"],
            "the last section stays"
        );
        assert_eq!(
            shifted(&mut document, area, "Q", Direction::Up, false)?,
            ["a", "b", "#T", "#U", "e", "#S", "c", "d", "Q", "P"]
        );
        assert_eq!(
            shifted(&mut document, area, "Q", Direction::Up, false)?,
            ["a", "b", "#T", "#U", "e", "#S", "c", "d", "Q", "P"],
            "a project never moves among the sections"
        );
        Ok(())
    }

    // FR-OPS-02
    #[test]
    fn in_the_inbox_the_first_and_last_tasks_stay() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        for title in ["a", "b"] {
            add(&mut document, Some(inbox), Item::new(Task, title));
        }
        assert_eq!(
            shifted(&mut document, inbox, "a", Direction::Up, false)?,
            ["a", "b"]
        );
        assert_eq!(
            shifted(&mut document, inbox, "b", Direction::Down, false)?,
            ["a", "b"]
        );
        let rows = rows(&document, inbox, false);
        let missing = ItemId::new(99);
        assert_eq!(
            shift(&document, &rows, missing, Direction::Up),
            Transaction::default()
        );
        Ok(())
    }

    /// Projects `A`, a completed `B`, a deleted `C`, then `D` holding `E`, in an area, and a
    /// task holding checklist items `1` and `2`, `2` holding `3`.
    fn nested() -> Document {
        let mut document = Document::new();
        let area = add(&mut document, None, Item::new(Area, "area"));
        add(&mut document, Some(area), Item::new(Project, "A"));
        add(&mut document, Some(area), done(Project, "B"));
        let mut deleted = Item::new(Project, "C");
        deleted.deleted = Some(date(2026, 10, 1));
        add(&mut document, Some(area), deleted);
        add(&mut document, Some(area), Item::new(Task, "t"));
        let d = add(&mut document, Some(area), Item::new(Project, "D"));
        add(&mut document, Some(d), Item::new(Project, "E"));
        let inbox = document.inbox();
        let task = add(&mut document, Some(inbox), Item::new(Task, "task"));
        add(&mut document, Some(task), Item::new(ChecklistItem, "1"));
        let two = add(&mut document, Some(task), Item::new(ChecklistItem, "2"));
        add(&mut document, Some(two), Item::new(ChecklistItem, "3"));
        document
    }

    // FR-OPS-03, FR-ORG-02
    #[test]
    fn a_project_indents_into_the_open_project_above_it_and_outdents_back()
    -> Result<(), PlacementError> {
        let mut document = nested();
        let [a, d, e] = ["A", "D", "E"].map(|title| find(&document, title));
        let (transaction, into) = indent(&document, d).map_err(|_| PlacementError::DoesNotFit)?;
        assert_eq!(into, a);
        document.apply(&transaction)?;
        assert_eq!(document.children(a), [d]);
        assert_eq!(document.children(d), [e], "what it holds goes with it");
        let transaction = outdent(&document, d).map_err(|_| PlacementError::DoesNotFit)?;
        document.apply(&transaction)?;
        assert_eq!(
            document.position(d),
            document.position(a).map(|(p, i)| (p, i + 1))
        );
        let top = add(&mut document, None, Item::new(Project, "top"));
        let next = add(&mut document, None, Item::new(Project, "next"));
        let (transaction, into) =
            indent(&document, next).map_err(|_| PlacementError::DoesNotFit)?;
        assert_eq!(into, top, "at the top level too");
        document.apply(&transaction)?;
        assert_eq!(document.children(top), [next]);
        Ok(())
    }

    // FR-OPS-03, FR-CHK-03
    #[test]
    fn a_checklist_item_indents_under_the_one_above_it_and_outdents_out_of_it()
    -> Result<(), PlacementError> {
        let mut document = nested();
        let [one, two, three] = ["1", "2", "3"].map(|title| find(&document, title));
        document.apply(&outdent(&document, three).map_err(|_| PlacementError::DoesNotFit)?)?;
        assert_eq!(
            document.position(three),
            document.position(two).map(|(p, i)| (p, i + 1))
        );
        let (transaction, into) = indent(&document, two).map_err(|_| PlacementError::DoesNotFit)?;
        assert_eq!(into, one);
        document.apply(&transaction)?;
        assert_eq!(document.children(one), [two]);
        Ok(())
    }

    // FR-OPS-03
    #[test]
    fn a_move_the_hierarchy_does_not_allow_is_refused_with_why() {
        let document = nested();
        let [a, e, t, one, area] = ["A", "E", "t", "1", "area"].map(|title| find(&document, title));
        assert_eq!(indent(&document, a).err(), Some(Refusal::NothingAbove));
        assert_eq!(indent(&document, one).err(), Some(Refusal::NothingAbove));
        assert_eq!(
            indent(&document, find(&document, "B"))
                .ok()
                .map(|(_, into)| into),
            Some(a)
        );
        assert_eq!(indent(&document, e).err(), Some(Refusal::NothingAbove));
        assert_eq!(outdent(&document, a).err(), Some(Refusal::NotNested));
        assert_eq!(outdent(&document, one).err(), Some(Refusal::NotNested));
        for id in [t, area, document.inbox(), ItemId::new(99)] {
            assert_eq!(indent(&document, id).err(), Some(Refusal::DoesNotNest));
            assert_eq!(outdent(&document, id).err(), Some(Refusal::DoesNotNest));
        }
    }
}
