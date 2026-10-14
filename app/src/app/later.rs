//! Marking items Later, as Things 3's Someday: the change one press of the key makes to the
//! items it is given (FR-ORG-11, D-89).

use crate::domain::{Document, Item, ItemId, Node, Op, Transaction};

/// The transaction that takes `items` out of Later when every one of them is Later, and
/// otherwise marks Later those that are not. Only projects and tasks are Later; anything else
/// among `items`, such as an area or a section, is left alone.
pub(super) fn toggle(document: &Document, items: impl IntoIterator<Item = ItemId>) -> Transaction {
    let items: Vec<(ItemId, &Item)> = items
        .into_iter()
        .filter_map(|id| Some((id, document.node(id).and_then(Node::item)?)))
        .filter(|(_, item)| item.kind.goes_later())
        .collect();
    let marking = items.iter().any(|(_, item)| !item.later);
    let ops = items
        .into_iter()
        .filter(|(_, item)| item.later != marking)
        .map(|(id, item)| Op::Set {
            id,
            before: item.clone(),
            after: Item {
                later: marking,
                ..item.clone()
            },
        })
        .collect();
    Transaction::new(ops)
}

/// Whether some of `items` can be marked Later: a project or a task among them.
pub(super) fn takes_any(document: &Document, items: &[ItemId]) -> bool {
    items.iter().any(|&id| {
        document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| item.kind.goes_later())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ItemKind, PlacementError};

    fn later(document: &Document, id: ItemId) -> Option<bool> {
        Some(document.node(id).and_then(Node::item)?.later)
    }

    // FR-ORG-11
    #[test]
    fn marks_what_is_not_later_and_unmarks_once_all_are() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let project = document.push(None, Node::Item(Item::new(ItemKind::Project, "p")))?;
        let task = document.push(Some(project), Node::Item(Item::new(ItemKind::Task, "t")))?;
        document.apply(&toggle(&document, [task]))?;
        assert_eq!(later(&document, task), Some(true));
        let both = toggle(&document, [task, project]);
        assert_eq!(both.ops().len(), 1, "only the project changes");
        document.apply(&both)?;
        assert_eq!(later(&document, project), Some(true));
        document.apply(&toggle(&document, [project, task]))?;
        assert_eq!(
            (later(&document, project), later(&document, task)),
            (Some(false), Some(false))
        );
        Ok(())
    }

    // FR-ORG-11
    #[test]
    fn no_area_section_inbox_or_checklist_item_is_later() -> Result<(), PlacementError> {
        use ItemKind::{Area, ChecklistItem, Section, Task};
        let mut document = Document::new();
        let mut add = |parent, kind| document.push(parent, Node::Item(Item::new(kind, "x")));
        let area = add(None, Area)?;
        let section = add(Some(area), Section)?;
        let task = add(Some(section), Task)?;
        let check = add(Some(task), ChecklistItem)?;
        let line = document.push(None, Node::Kept("text".to_owned()))?;
        let others = [
            area,
            section,
            document.inbox(),
            check,
            line,
            ItemId::new(99),
        ];
        assert_eq!(toggle(&document, others), Transaction::default());
        assert!(!takes_any(&document, &others));
        assert!(takes_any(&document, &[area, task]));
        Ok(())
    }
}
