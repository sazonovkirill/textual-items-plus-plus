//! A task's checklist: its items, which do not nest, how far it is done, and where checking,
//! deleting, moving, adding and pasting its items puts them (FR-CHK-01–04). Hides where the items
//! sit in the document.

use jiff::civil::Date;

use super::{Side, moving::Direction};
use crate::domain::{
    Branch, Completion, Destination, Document, Item, ItemId, ItemKind, Node, Op, Transaction,
};

/// One item of a checklist as a frontend lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChecklistLine<'a> {
    /// The checklist item.
    pub id: ItemId,
    /// What it holds: its text and whether it is checked, which draws it dimmed (FR-CHK-04).
    pub item: &'a Item,
}

/// The checklist items of `task`, in order.
pub(super) fn lines(document: &Document, task: ItemId) -> Vec<ChecklistLine<'_>> {
    children(document, task)
        .map(|id| ChecklistLine {
            id,
            item: item(document, id).expect("checklist items are items"),
        })
        .collect()
}

/// How many of the checklist items of `id` are checked, and how many there are; `None` when it
/// has none (FR-TASK-03).
#[must_use]
pub fn progress(document: &Document, id: ItemId) -> Option<(usize, usize)> {
    let lines = lines(document, id);
    let checked = lines
        .iter()
        .filter(|line| line.item.completed.is_some())
        .count();
    (!lines.is_empty()).then_some((checked, lines.len()))
}

/// The transaction that checks checklist item `id` on `today`, or unchecks it when it is
/// checked, leaving everything else as it is (FR-CHK-04).
pub(super) fn toggle(document: &Document, id: ItemId, today: Date) -> Transaction {
    let Some(item) = item(document, id) else {
        return Transaction::default();
    };
    let completed = match item.completed {
        Some(_) => None,
        None => Some(Completion::On(today)),
    };
    Transaction::new(vec![Op::Set {
        id,
        before: item.clone(),
        after: Item {
            completed,
            ..item.clone()
        },
    }])
}

/// The transaction that takes checklist item `id` out of the document. Checklist items do not go
/// to the Trash, which lists only what the pane shows as rows.
pub(super) fn delete(document: &Document, id: ItemId) -> Transaction {
    let Some(((parent, index), subtree)) = document.position(id).zip(document.snapshot(id)) else {
        return Transaction::default();
    };
    Transaction::new(vec![Op::Remove {
        parent,
        index,
        subtree,
    }])
}

/// The transaction that moves checklist item `id` past the checklist item next to it,
/// `direction` of it; an empty one when there is none that way.
pub(super) fn shift(document: &Document, id: ItemId, direction: Direction) -> Transaction {
    let siblings: Vec<ItemId> = document
        .parent(id)
        .map_or_else(Vec::new, |task| children(document, task).collect());
    let at = siblings.iter().position(|&sibling| sibling == id);
    let to = match direction {
        Direction::Up => at
            .and_then(|at| at.checked_sub(1))
            .and_then(|before| siblings.get(before))
            .map(|&other| Destination::Before(other)),
        Direction::Down => at
            .and_then(|at| siblings.get(at + 1))
            .map(|&other| Destination::After(other)),
    };
    to.map_or_else(Transaction::default, |to| {
        document
            .move_to(id, to)
            .expect("a checklist item fits beside another")
    })
}

/// Where a new checklist item of `task` goes on `side` of its item `selected`: beside it; with
/// none selected, first or last.
pub(super) fn place(
    document: &Document,
    task: ItemId,
    selected: Option<ItemId>,
    side: Side,
) -> (Option<ItemId>, usize) {
    if let Some((parent, index)) = selected.and_then(|id| document.position(id)) {
        return match side {
            Side::Above => (parent, index),
            Side::Below => (parent, index + 1),
        };
    }
    match side {
        Side::Above => (Some(task), 0),
        Side::Below => (Some(task), document.children(task).len()),
    }
}

/// The transaction that puts `branches` in as checklist items of `task` on `side` of its item
/// `selected`, where [`place`] puts a new one, and the last of them; `None` when there is nothing
/// to paste. Every item in them becomes a checklist item of its own, in the order they hold them,
/// keeping its text, tags and whether it is done (FR-CHK-03).
pub(super) fn paste(
    document: &mut Document,
    task: ItemId,
    selected: Option<ItemId>,
    side: Side,
    branches: &[Branch],
) -> Option<(Transaction, ItemId)> {
    let (parent, index) = place(document, task, selected, side);
    let mut flat = Vec::new();
    for branch in branches {
        flatten(branch, &mut flat);
    }
    let mut last = None;
    let ops = flat
        .iter()
        .zip(index..)
        .map(|(branch, index)| {
            let subtree = document.graft(branch);
            last = Some(subtree.id());
            Op::Insert {
                parent,
                index,
                subtree,
            }
        })
        .collect();
    Some((Transaction::new(ops), last?))
}

/// Pushes `branch` to `flat` as checklist items, each keeping only what a checklist item shows:
/// its title, tags and whether it is done, and the kept text it holds. Every item in it follows
/// the one holding it. A copied task's note, comments and dates stay with the original; the
/// clipboard reads each description line as an item of its own.
fn flatten(branch: &Branch, flat: &mut Vec<Branch>) {
    let Node::Item(item) = &branch.node else {
        flat.push(branch.clone());
        return;
    };
    let (kept, items): (Vec<_>, Vec<_>) = branch
        .children
        .iter()
        .partition(|child| matches!(child.node, Node::Kept(_)));
    flat.push(Branch {
        node: Node::Item(Item {
            completed: item.completed,
            tags: item.tags.clone(),
            ..Item::new(ItemKind::ChecklistItem, item.title.clone())
        }),
        children: kept.into_iter().cloned().collect(),
    });
    for child in items {
        flatten(child, flat);
    }
}

/// The children of `id` that are checklist items, in order.
fn children(document: &Document, id: ItemId) -> impl Iterator<Item = ItemId> + '_ {
    document
        .children(id)
        .iter()
        .copied()
        .filter(|&child| item(document, child).is_some_and(|it| it.kind == ItemKind::ChecklistItem))
}

fn item(document: &Document, id: ItemId) -> Option<&Item> {
    document.node(id).and_then(Node::item)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::PlacementError;

    const TODAY: Date = date(2026, 10, 14);

    /// A task `task` in the Inbox with checklist items `pack`, holding kept text, `books`, kept
    /// text, and `plates`, checked.
    fn document() -> Result<(Document, [ItemId; 4]), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let add = |document: &mut Document, parent, title: &str| {
            document.push(
                Some(parent),
                Node::Item(Item::new(ItemKind::ChecklistItem, title)),
            )
        };
        let task = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "t")))?;
        let pack = add(&mut document, task, "a")?;
        document.push(Some(pack), Node::Kept("held".to_owned()))?;
        let books = add(&mut document, task, "b")?;
        document.push(Some(task), Node::Kept("kept".to_owned()))?;
        let mut checked = Item::new(ItemKind::ChecklistItem, "c");
        checked.completed = Some(Completion::Undated);
        let plates = document.push(Some(task), Node::Item(checked))?;
        Ok((document, [task, pack, books, plates]))
    }

    fn titles(document: &Document, task: ItemId) -> Vec<(&str, bool)> {
        lines(document, task)
            .into_iter()
            .map(|line| (line.item.title.as_str(), line.item.completed.is_some()))
            .collect()
    }

    // FR-CHK-01, FR-TASK-03
    #[test]
    fn a_checklist_lists_its_items_in_order() -> Result<(), PlacementError> {
        let (document, [task, pack, ..]) = document()?;
        assert_eq!(
            titles(&document, task),
            [("a", false), ("b", false), ("c", true)]
        );
        assert_eq!(progress(&document, task), Some((1, 3)));
        assert_eq!(progress(&document, pack), None);
        assert_eq!(progress(&document, document.inbox()), None);
        Ok(())
    }

    // FR-CHK-03, FR-CHK-04
    #[test]
    fn checking_an_item_changes_only_that_item() -> Result<(), PlacementError> {
        let (mut document, [task, pack, _, plates]) = document()?;
        document.apply(&toggle(&document, pack, TODAY))?;
        let checked = |document: &Document, id| item(document, id).and_then(|it| it.completed);
        assert_eq!(checked(&document, pack), Some(Completion::On(TODAY)));
        assert_eq!(
            titles(&document, task),
            [("a", true), ("b", false), ("c", true)]
        );
        assert_eq!(progress(&document, task), Some((2, 3)));
        document.apply(&toggle(&document, plates, TODAY))?;
        assert_eq!(checked(&document, plates), None);
        assert_eq!(
            toggle(&document, ItemId::new(99), TODAY),
            Transaction::default()
        );
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn deleting_an_item_takes_only_that_item() -> Result<(), PlacementError> {
        let (mut document, [task, pack, ..]) = document()?;
        let original = document.clone();
        let deleted = delete(&document, pack);
        document.apply(&deleted)?;
        assert_eq!(titles(&document, task), [("b", false), ("c", true)]);
        document.apply(&deleted.inverse())?;
        assert_eq!(document, original);
        assert_eq!(delete(&document, ItemId::new(99)), Transaction::default());
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn an_item_moves_past_the_item_next_to_it() -> Result<(), PlacementError> {
        let (mut document, [task, pack, books, plates]) = document()?;
        document.apply(&shift(&document, books, Direction::Down))?;
        assert_eq!(
            titles(&document, task),
            [("a", false), ("c", true), ("b", false)],
            "past the kept text to the next item"
        );
        assert_eq!(
            shift(&document, books, Direction::Down),
            Transaction::default()
        );
        assert_eq!(
            shift(&document, pack, Direction::Up),
            Transaction::default()
        );
        document.apply(&shift(&document, plates, Direction::Up))?;
        assert_eq!(titles(&document, task)[0], ("c", true));
        assert_eq!(
            shift(&document, ItemId::new(99), Direction::Up),
            Transaction::default()
        );
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn a_new_item_goes_beside_the_one_selected_or_first_or_last() -> Result<(), PlacementError> {
        let (document, [task, _, books, _]) = document()?;
        assert_eq!(
            place(&document, task, Some(books), Side::Above),
            (Some(task), 1)
        );
        assert_eq!(
            place(&document, task, Some(books), Side::Below),
            (Some(task), 2)
        );
        assert_eq!(place(&document, task, None, Side::Above), (Some(task), 0));
        assert_eq!(place(&document, task, None, Side::Below), (Some(task), 4));
        Ok(())
    }

    // FR-CHK-01, FR-CHK-03
    #[test]
    fn pasted_items_become_checklist_items_in_order_none_nested() -> Result<(), PlacementError> {
        let (mut document, [task, pack, ..]) = document()?;
        let branch = |kind, title: &str, children| Branch {
            node: Node::Item(Item::new(kind, title)),
            children,
        };
        let kept = |text: &str| Branch {
            node: Node::Kept(text.to_owned()),
            children: Vec::new(),
        };
        let pasted = [
            branch(
                ItemKind::Task,
                "x",
                vec![
                    branch(
                        ItemKind::Task,
                        "y",
                        vec![branch(ItemKind::Task, "w", Vec::new())],
                    ),
                    kept("x's"),
                ],
            ),
            kept("text"),
            branch(ItemKind::Project, "z", Vec::new()),
        ];
        let (transaction, last) = paste(&mut document, task, Some(pack), Side::Below, &pasted)
            .ok_or(PlacementError::DoesNotFit)?;
        document.apply(&transaction)?;
        assert_eq!(
            titles(&document, task),
            [
                ("a", false),
                ("x", false),
                ("y", false),
                ("w", false),
                ("z", false),
                ("b", false),
                ("c", true),
            ]
        );
        let x = lines(&document, task)[1].id;
        let held = document.children(x);
        assert_eq!(held.len(), 1);
        assert_eq!(document.node(held[0]), Some(&Node::Kept("x's".to_owned())));
        assert_eq!(item(&document, last).map(|it| it.title.as_str()), Some("z"));
        assert!(paste(&mut document, task, None, Side::Above, &[]).is_none());
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn a_pasted_item_keeps_its_title_tags_and_done_state_and_nothing_a_checklist_cannot_show()
    -> Result<(), PlacementError> {
        let (mut document, [task, ..]) = document()?;
        let mut copied = Item::new(ItemKind::Task, "x");
        copied.description = "a description".to_owned();
        copied.note = "a note".to_owned();
        copied.comments = vec!["a comment".to_owned()];
        copied.start = Some(TODAY);
        copied.due = Some(TODAY);
        copied.completed = Some(Completion::On(TODAY));
        copied.tags = vec![crate::domain::Tag {
            name: "home".to_owned(),
            value: None,
        }];
        let pasted = Branch {
            node: Node::Item(copied.clone()),
            children: Vec::new(),
        };
        let (transaction, last) = paste(&mut document, task, None, Side::Above, &[pasted])
            .ok_or(PlacementError::DoesNotFit)?;
        document.apply(&transaction)?;
        assert_eq!(
            item(&document, last),
            Some(&Item {
                completed: copied.completed,
                tags: copied.tags,
                ..Item::new(ItemKind::ChecklistItem, "x")
            })
        );
        Ok(())
    }
}
