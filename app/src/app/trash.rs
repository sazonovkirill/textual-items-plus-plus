//! The Trash: deleting items by tagging them in place, with what they hold, so that undo or the
//! Trash view can put them back exactly, and what the Trash holds, restored and emptied
//! (FR-OPS-05, FR-VIEW-04, OQ-19).

use std::cmp::Reverse;

use jiff::civil::Date;

use super::{container::Row, view::Group};
use crate::domain::{Document, Item, ItemId, ItemKind, Node, Op, Transaction};

/// Whether `item` is of a kind that is deleted to the Trash and listed there.
fn trashable(item: &Item) -> bool {
    matches!(
        item.kind,
        ItemKind::Area | ItemKind::Project | ItemKind::Section | ItemKind::Task
    )
}

/// The transaction that moves `items` to the Trash on `today`. Only areas, projects, sections and
/// tasks are deleted this way, and only those not in the Trash already; anything else among
/// `items` is left alone.
pub(super) fn delete(
    document: &Document,
    items: impl IntoIterator<Item = ItemId>,
    today: Date,
) -> Transaction {
    let ops = items
        .into_iter()
        .filter_map(|id| Some((id, document.node(id).and_then(Node::item)?)))
        .filter(|(_, item)| item.deleted.is_none() && trashable(item))
        .map(|(id, item)| Op::Set {
            id,
            before: item.clone(),
            after: Item {
                deleted: Some(today),
                ..item.clone()
            },
        })
        .collect();
    Transaction::new(ops)
}

/// What the Trash lists: the areas, projects, sections and tasks deleted with nothing above them
/// deleted too, which they hold along with the rest of their contents, the latest deleted first
/// and those deleted on the same day in outline order, the Inbox's first (FR-VIEW-04).
pub(super) fn items(document: &Document) -> Vec<ItemId> {
    let mut found = Vec::new();
    let mut pending: Vec<ItemId> = document.top_level().iter().rev().copied().collect();
    pending.push(document.inbox());
    while let Some(id) = pending.pop() {
        let Some(item) = document.node(id).and_then(Node::item) else {
            continue;
        };
        if let Some(deleted) = item.deleted.filter(|_| trashable(item)) {
            found.push((deleted, id));
        } else {
            pending.extend(document.children(id).iter().rev());
        }
    }
    found.sort_by_key(|&(deleted, _)| Reverse(deleted));
    found.into_iter().map(|(_, id)| id).collect()
}

/// The Trash view's rows, in one group without a heading, none when the Trash is empty: each
/// [item](items) in the Trash as one row, a task as a task and an area, project or section as a
/// project, since nothing it holds is listed.
pub(super) fn groups(document: &Document) -> Vec<Group> {
    let rows: Vec<Row> = items(document)
        .into_iter()
        .map(|id| match document.node(id).and_then(Node::item) {
            Some(item) if item.kind == ItemKind::Task => Row::Task { id, section: None },
            _ => Row::Project(id),
        })
        .collect();
    if rows.is_empty() {
        return Vec::new();
    }
    vec![Group {
        heading: None,
        rows,
    }]
}

/// The transaction that takes `items` out of the Trash, back where they were; anything among
/// them not in the Trash is left alone (FR-VIEW-04).
pub(super) fn restore(document: &Document, items: impl IntoIterator<Item = ItemId>) -> Transaction {
    let ops = items
        .into_iter()
        .filter_map(|id| Some((id, document.node(id).and_then(Node::item)?)))
        .filter(|(_, item)| item.deleted.is_some())
        .map(|(id, item)| Op::Set {
            id,
            before: item.clone(),
            after: Item {
                deleted: None,
                ..item.clone()
            },
        })
        .collect();
    Transaction::new(ops)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::PlacementError;

    const TODAY: Date = date(2026, 10, 14);

    fn deleted(document: &Document, id: ItemId) -> Option<Date> {
        document.node(id).and_then(Node::item)?.deleted
    }

    // FR-OPS-05, OQ-19
    #[test]
    fn deleting_tags_the_items_in_place_and_keeps_what_they_hold() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "a")))?;
        let project = document.push(Some(area), Node::Item(Item::new(ItemKind::Project, "p")))?;
        let section =
            document.push(Some(project), Node::Item(Item::new(ItemKind::Section, "s")))?;
        let task = document.push(Some(section), Node::Item(Item::new(ItemKind::Task, "t")))?;
        let before = document.clone();
        document.apply(&delete(&document, [task, section, project, area], TODAY))?;
        for id in [task, section, project, area] {
            assert_eq!(deleted(&document, id), Some(TODAY));
            assert_eq!(document.position(id), before.position(id));
        }
        Ok(())
    }

    #[test]
    fn the_inbox_checklist_items_kept_lines_and_trashed_items_stay() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let task = document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, "t")))?;
        let check = Node::Item(Item::new(ItemKind::ChecklistItem, "c"));
        let check = document.push(Some(task), check)?;
        let line = document.push(None, Node::Kept("text".to_owned()))?;
        let mut trashed = Item::new(ItemKind::Task, "old");
        trashed.deleted = Some(date(2026, 10, 1));
        let trashed = document.push(Some(inbox), Node::Item(trashed))?;
        let others = [inbox, check, line, trashed, ItemId::new(99)];
        assert_eq!(delete(&document, others, TODAY), Transaction::default());
        Ok(())
    }

    fn put(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
        day: Option<Date>,
    ) -> Result<ItemId, PlacementError> {
        let mut item = Item::new(kind, title);
        item.deleted = day;
        document.push(parent, Node::Item(item))
    }

    // FR-VIEW-04
    #[test]
    fn the_trash_lists_the_outermost_deleted_items_latest_first() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let early = Some(date(2026, 10, 1));
        let late = Some(date(2026, 10, 9));
        let old = put(&mut document, inbox, ItemKind::Task, "old", early)?;
        let kept = put(&mut document, inbox, ItemKind::Task, "kept", None)?;
        let check = Node::Item(Item {
            deleted: late,
            ..Item::new(ItemKind::ChecklistItem, "c")
        });
        let check = document.push(Some(kept), check)?;
        let area = put(&mut document, None, ItemKind::Area, "area", None)?;
        let project = put(&mut document, Some(area), ItemKind::Project, "p", late)?;
        put(&mut document, Some(project), ItemKind::Task, "inside", late)?;
        let section = put(&mut document, Some(area), ItemKind::Section, "s", early)?;
        let line = document.push(None, Node::Kept("line".to_owned()))?;
        document.push(Some(line), Node::Kept("\tbelow".to_owned()))?;
        let new = put(&mut document, None, ItemKind::Project, "new", late)?;
        assert_eq!(items(&document), [project, new, old, section]);
        assert!(!items(&document).contains(&check));
        Ok(())
    }

    // FR-VIEW-04
    #[test]
    fn restoring_takes_items_out_of_the_trash_where_they_were() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let task = put(&mut document, inbox, ItemKind::Task, "t", Some(TODAY))?;
        let open = put(&mut document, inbox, ItemKind::Task, "open", None)?;
        let before = document.clone();
        let restoring = restore(&document, [task, open, ItemId::new(99)]);
        assert_eq!(restoring.ops().len(), 1);
        document.apply(&restoring)?;
        assert_eq!(deleted(&document, task), None);
        assert_eq!(document.position(task), before.position(task));
        assert_eq!(items(&document), []);
        Ok(())
    }
}
