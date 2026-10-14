//! Moving: where moving an item up or down among the rows it is shown with puts it in the
//! document (FR-OPS-02, FR-OPS-10). Hides how a place in the rows maps to a place in the
//! document.

use super::container::{Row, Stage};
use crate::domain::{Destination, Document, Item, ItemId, ItemKind, Node, Transaction};

/// Which way an item moves among its rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Toward the top.
    Up,
    /// Toward the bottom.
    Down,
}

/// Why the hierarchy refused to indent, outdent, convert or group items, for the status line to
/// say (FR-OPS-03, FR-TASK-06, FR-ORG-06, FR-OPS-07, FR-OPS-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No item of its kind is above it to go into.
    NothingAbove,
    /// It is not inside an item of its own kind to come out of.
    NotNested,
    /// A section would be where only an area or a project holds one.
    NoRoomForSection,
    /// A project would be in a section or the Inbox, which hold none.
    NoRoomForProject,
    /// A section would hold something other than tasks.
    SectionHoldsTasks,
    /// A task's title starts with `-` and a space, so it would read back as a task's again.
    ReadsAsTask,
    /// A task has a bare `@area` or `@section` tag, so it would read back as another kind.
    KindTag,
}

/// The transaction that moves `id` one step `direction` among `rows`, the rows of the container
/// shown; an empty one when it has nowhere to go. An item changes places with the next item
/// shown in its own group, a task among the tasks listed under the same heading, a section among
/// the sections and a child project among the child projects, open, Later and completed ones
/// apart (FR-VIEW-11, D-89). Only a task crosses the edge of its group: past the heading below it, it is the
/// first task of that section, and up from the top of a section it is the last task of the
/// section above, or of those outside sections (FR-OPS-10).
fn shift(document: &Document, rows: &[Row], id: ItemId, direction: Direction) -> Transaction {
    let Some((at, &row)) = rows.iter().enumerate().find(|(_, row)| row.id() == id) else {
        return Transaction::default();
    };
    let stage = |row: &Row| item(document, row.id()).map(Stage::of);
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
        same_group && stage(other) == stage(&row)
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

/// The transaction that moves each of `items`, given in the order of the rows, one step
/// `direction` as [`shift`] moves one among the rows `rows` lists of the document as the ones
/// moved before left it: the last first when moving down, so that they keep their order. An item
/// stays when its move would take it past another of them (FR-OPS-02, D-18).
pub(super) fn shift_all(
    document: &Document,
    items: &[ItemId],
    direction: Direction,
    rows: impl Fn(&Document) -> Vec<Row>,
) -> Transaction {
    let ordered: Vec<ItemId> = match direction {
        Direction::Up => items.to_vec(),
        Direction::Down => items.iter().rev().copied().collect(),
    };
    let (transaction, _) = each(document, ordered, |document, id| {
        let before = rows(document);
        let transaction = shift(document, &before, id, direction);
        if items.len() == 1 {
            return Ok(transaction);
        }
        let mut moved = document.clone();
        moved
            .apply(&transaction)
            .expect("a transaction made from the document as it is fits it");
        let after = rows(&moved);
        let passes = items.iter().any(|&other| {
            let above = |rows: &[Row]| Some(position(rows, other)? < position(rows, id)?);
            other != id
                && above(&before)
                    .zip(above(&after))
                    .is_some_and(|(was, is)| was != is)
        });
        Ok(if passes {
            Transaction::default()
        } else {
            transaction
        })
    });
    transaction
}

/// The transaction of `step` taken for each of `ordered` in turn, on the document as the steps
/// before left it, and the first refusal, if any step refused. The document is copied to take
/// the steps on only when there is more than one, since copying a large one takes long
/// (N-PERF-02).
pub(super) fn each(
    document: &Document,
    ordered: impl IntoIterator<Item = ItemId>,
    mut step: impl FnMut(&Document, ItemId) -> Result<Transaction, Refusal>,
) -> (Transaction, Option<Refusal>) {
    let ordered: Vec<ItemId> = ordered.into_iter().collect();
    if let &[id] = ordered.as_slice() {
        return match step(document, id) {
            Ok(transaction) => (transaction, None),
            Err(refusal) => (Transaction::default(), Some(refusal)),
        };
    }
    let mut scratch = document.clone();
    let mut ops = Vec::new();
    let mut refused = None;
    for id in ordered {
        match step(&scratch, id) {
            Ok(transaction) => {
                scratch
                    .apply(&transaction)
                    .expect("a transaction made from the document as it is fits it");
                ops.extend_from_slice(transaction.ops());
            }
            Err(refusal) => {
                refused.get_or_insert(refusal);
            }
        }
    }
    (Transaction::new(ops), refused)
}

/// Where `id` is among `rows`.
fn position(rows: &[Row], id: ItemId) -> Option<usize> {
    rows.iter().position(|row| row.id() == id)
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
    use ItemKind::{Area, Project, Section, Task};

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

    // FR-OPS-10, FR-ORG-11
    #[test]
    fn later_and_open_items_move_apart() -> Result<(), PlacementError> {
        let (mut document, area) = area();
        for title in ["a", "P"] {
            let id = find(&document, title);
            let later = item(&document, id)
                .cloned()
                .map(|it| Item { later: true, ..it });
            document.set(id, later.ok_or(PlacementError::NoSuchNode(id))?)?;
        }
        assert_eq!(
            shown(&document, area, false),
            ["b", "a", "#S", "c", "d", "#T", "#U", "e", "Q", "P"]
        );
        assert_eq!(
            shifted(&mut document, area, "a", Direction::Up, false)?,
            ["b", "a", "#S", "c", "d", "#T", "#U", "e", "Q", "P"],
            "the Later task stays below the open ones"
        );
        assert_eq!(
            shifted(&mut document, area, "Q", Direction::Down, false)?,
            ["b", "a", "#S", "c", "d", "#T", "#U", "e", "Q", "P"],
            "the open project stays above the Later one"
        );
        assert_eq!(
            shifted(&mut document, area, "b", Direction::Down, false)?,
            ["a", "#S", "b", "c", "d", "#T", "#U", "e", "Q", "P"],
            "the open task passes the Later one into the section below"
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

    /// Moves the items titled `titles` in `container` together and returns the rows shown after,
    /// checking undo.
    fn shifted_all(
        document: &mut Document,
        container: ItemId,
        titles: &[&str],
        direction: Direction,
    ) -> Result<Vec<String>, PlacementError> {
        let items: Vec<ItemId> = titles.iter().map(|title| find(document, title)).collect();
        let before = document.clone();
        let transaction = shift_all(document, &items, direction, |document| {
            rows(document, container, false)
        });
        document.apply(&transaction)?;
        let mut undone = document.clone();
        undone.apply(&transaction.inverse())?;
        assert_eq!(undone, before, "undo restores the document");
        Ok(shown(document, container, false))
    }

    // FR-OPS-01, FR-OPS-02, FR-OPS-10
    #[test]
    fn selected_tasks_move_together_keeping_their_order() -> Result<(), PlacementError> {
        let (mut document, area) = area();
        assert_eq!(
            shifted_all(&mut document, area, &["a", "b"], Direction::Down)?,
            ["#S", "a", "b", "c", "d", "#T", "#U", "e", "P", "Q"]
        );
        assert_eq!(
            shifted_all(&mut document, area, &["b", "d"], Direction::Up)?,
            ["#S", "b", "a", "d", "c", "#T", "#U", "e", "P", "Q"],
            "apart, each moves on its own"
        );
        assert_eq!(
            shifted_all(&mut document, area, &["d", "e"], Direction::Up)?,
            ["#S", "b", "d", "a", "c", "#T", "e", "#U", "P", "Q"],
            "each crosses its own heading"
        );
        assert_eq!(
            shifted_all(&mut document, area, &["a"], Direction::Down)?,
            ["#S", "b", "d", "c", "a", "#T", "e", "#U", "P", "Q"],
            "one moves as Move does"
        );
        Ok(())
    }

    // FR-OPS-02, FR-OPS-10
    #[test]
    fn a_selected_item_that_would_pass_another_that_stays_stays_too() -> Result<(), PlacementError>
    {
        let (mut document, area) = area();
        let before = shown(&document, area, false);
        assert_eq!(
            shifted_all(&mut document, area, &["a", "b"], Direction::Up)?,
            before,
            "the first stays, and the second with it"
        );
        assert_eq!(
            shifted_all(&mut document, area, &["P", "Q"], Direction::Down)?,
            before
        );
        assert_eq!(
            shifted_all(&mut document, area, &["S", "c"], Direction::Up)?,
            before,
            "the first section stays, and its task does not pass its heading"
        );
        Ok(())
    }
}
