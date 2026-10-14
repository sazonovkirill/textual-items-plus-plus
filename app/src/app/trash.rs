//! Deleting items: moving them to the Trash in place, with what they hold, so that undo or the
//! Trash view can put them back exactly (FR-OPS-05, OQ-19).

use jiff::civil::Date;

use crate::domain::{Document, Item, ItemId, ItemKind, Node, Op, Transaction};

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
        .filter(|(_, item)| {
            item.deleted.is_none()
                && matches!(
                    item.kind,
                    ItemKind::Area | ItemKind::Project | ItemKind::Section | ItemKind::Task
                )
        })
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
}
