//! Completing and uncompleting items: the change one press of the key makes to the items it is
//! given (FR-TASK-02, FR-ORG-09).

use jiff::civil::Date;

use crate::domain::{Completion, Document, Item, ItemId, ItemKind, Node, Op, Transaction};

/// The transaction that uncompletes `items` when every one of them is completed, and otherwise
/// completes those that are not, on `today`. Only projects, sections and tasks are
/// completed; anything else among `items` is left alone.
pub(super) fn toggle(
    document: &Document,
    items: impl IntoIterator<Item = ItemId>,
    today: Date,
) -> Transaction {
    let items: Vec<(ItemId, &Item)> = items
        .into_iter()
        .filter_map(|id| Some((id, document.node(id).and_then(Node::item)?)))
        .filter(|(_, item)| {
            matches!(
                item.kind,
                ItemKind::Project | ItemKind::Section | ItemKind::Task
            )
        })
        .collect();
    let completing = items.iter().any(|(_, item)| item.completed.is_none());
    let ops = items
        .into_iter()
        .filter(|(_, item)| item.completed.is_none() == completing)
        .map(|(id, item)| Op::Set {
            id,
            before: item.clone(),
            after: Item {
                completed: completing.then_some(Completion::On(today)),
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

    fn completed(document: &Document, id: ItemId) -> Option<Completion> {
        document.node(id).and_then(Node::item)?.completed
    }

    /// A task `a`, a task `b` completed with no day, and a checklist item under `a`.
    fn document() -> Result<(Document, ItemId, ItemId, ItemId), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let a = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "a")))?;
        let mut done = Item::new(ItemKind::Task, "b");
        done.completed = Some(Completion::Undated);
        let b = document.push(inbox, Node::Item(done))?;
        let check = document.push(Some(a), Node::Item(Item::new(ItemKind::ChecklistItem, "c")))?;
        Ok((document, a, b, check))
    }

    // FR-TASK-02
    #[test]
    fn completing_records_today_and_uncompleting_clears_it() -> Result<(), PlacementError> {
        let (mut document, a, b, _) = document()?;
        document.apply(&toggle(&document, [a], TODAY))?;
        assert_eq!(completed(&document, a), Some(Completion::On(TODAY)));
        document.apply(&toggle(&document, [a, b], TODAY))?;
        assert_eq!(completed(&document, a), None);
        assert_eq!(completed(&document, b), None);
        Ok(())
    }

    // FR-TASK-02
    #[test]
    fn a_mixed_selection_completes_only_what_is_not_completed() -> Result<(), PlacementError> {
        let (document, a, b, _) = document()?;
        let transaction = toggle(&document, [b, a], TODAY);
        assert_eq!(transaction.ops().len(), 1);
        let mut changed = document.clone();
        changed.apply(&transaction)?;
        assert_eq!(completed(&changed, a), Some(Completion::On(TODAY)));
        assert_eq!(completed(&changed, b), Some(Completion::Undated));
        Ok(())
    }

    // FR-ORG-09
    #[test]
    fn containers_complete_but_not_the_inbox_or_checklist_items() -> Result<(), PlacementError> {
        let (mut document, _, _, check) = document()?;
        let area = document.push(None, Node::Item(Item::new(ItemKind::Project, "x")))?;
        let section = document.push(Some(area), Node::Item(Item::new(ItemKind::Section, "s")))?;
        let project = document.push(Some(area), Node::Item(Item::new(ItemKind::Project, "p")))?;
        let line = document.push(None, Node::Kept("text".to_owned()))?;
        let others = [document.inbox(), check, line, ItemId::new(99)];
        assert_eq!(toggle(&document, others, TODAY), Transaction::default());
        document.apply(&toggle(&document, [area, section, project], TODAY))?;
        for id in [area, section, project] {
            assert_eq!(completed(&document, id), Some(Completion::On(TODAY)));
        }
        Ok(())
    }
}
