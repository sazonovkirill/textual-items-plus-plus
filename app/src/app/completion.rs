//! Completing and uncompleting items: the change one press of the key makes to the items it is
//! given (FR-TASK-02, FR-ORG-09).

use jiff::civil::Date;

use super::{container, navigator};
use crate::domain::{Completion, Document, Item, ItemId, ItemKind, Node, Op, Transaction};

/// The transaction that uncompletes `items` when every one of them is completed, and otherwise
/// completes those that are not, on `today`. Only projects, sections and tasks are completed;
/// anything else among `items`, such as an area, is left alone (D-81).
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

/// How many open tasks the projects among `items` that `toggle` would complete hold, at any
/// depth, which completing hides with them (FR-ORG-09, D-140).
pub(super) fn left_open(document: &Document, items: &[ItemId]) -> usize {
    let item = |id: ItemId| document.node(id).and_then(Node::item);
    let completing = items.iter().filter_map(|&id| item(id)).any(|item| {
        matches!(
            item.kind,
            ItemKind::Project | ItemKind::Section | ItemKind::Task
        ) && item.completed.is_none()
    });
    if !completing {
        return 0;
    }
    container::outermost_rows(document, items.iter().copied())
        .into_iter()
        .filter(|&id| {
            item(id).is_some_and(|item| item.kind == ItemKind::Project && item.completed.is_none())
        })
        .map(|id| navigator::held_open_tasks(document, id))
        .sum()
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

    // FR-ORG-09, D-140
    #[test]
    fn counts_the_open_tasks_left_in_the_projects_completing_would_complete()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let add = |document: &mut Document, parent, kind, title| {
            document.push(parent, Node::Item(Item::new(kind, title)))
        };
        let project = add(&mut document, None, ItemKind::Project, "p")?;
        let section = add(&mut document, Some(project), ItemKind::Section, "s")?;
        let open = add(&mut document, Some(section), ItemKind::Task, "open")?;
        let mut done = Item::new(ItemKind::Task, "done");
        done.completed = Some(Completion::Undated);
        document.push(Some(section), Node::Item(done))?;
        let mut deleted = Item::new(ItemKind::Task, "deleted");
        deleted.deleted = Some(TODAY);
        document.push(Some(project), Node::Item(deleted))?;
        let inner = add(&mut document, Some(project), ItemKind::Project, "inner")?;
        add(&mut document, Some(inner), ItemKind::Task, "deep")?;
        let empty = add(&mut document, None, ItemKind::Project, "empty")?;
        assert_eq!(left_open(&document, &[project]), 2, "at any depth");
        assert_eq!(
            left_open(&document, &[project, open, inner]),
            2,
            "counted once"
        );
        assert_eq!(left_open(&document, &[inner, empty]), 1);
        assert_eq!(left_open(&document, &[empty, open, section]), 0);
        document.apply(&toggle(&document, [project], TODAY))?;
        assert_eq!(left_open(&document, &[project]), 0, "uncompleting");
        assert_eq!(left_open(&document, &[project, inner]), 0, "already hidden");
        Ok(())
    }

    // FR-ORG-09
    #[test]
    fn sections_and_projects_complete_but_no_area_inbox_or_checklist_item()
    -> Result<(), PlacementError> {
        let (mut document, _, _, check) = document()?;
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "x")))?;
        let section = document.push(Some(area), Node::Item(Item::new(ItemKind::Section, "s")))?;
        let project = document.push(Some(area), Node::Item(Item::new(ItemKind::Project, "p")))?;
        let line = document.push(None, Node::Kept("text".to_owned()))?;
        let others = [area, document.inbox(), check, line, ItemId::new(99)];
        assert_eq!(toggle(&document, others, TODAY), Transaction::default());
        document.apply(&toggle(&document, [area, section, project], TODAY))?;
        assert_eq!(completed(&document, area), None);
        for id in [section, project] {
            assert_eq!(completed(&document, id), Some(Completion::On(TODAY)));
        }
        Ok(())
    }
}
