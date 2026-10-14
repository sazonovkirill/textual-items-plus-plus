//! A task's checklist: its items in tree order, how far it is done, and where checking,
//! deleting, moving, adding and pasting its items puts them (FR-CHK-01–04). Hides how the items
//! nest in the document.

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
    /// What it holds: its text and whether it is checked.
    pub item: &'a Item,
    /// How deep it is nested, 0 for an item right under the task.
    pub depth: usize,
    /// Whether it is drawn dimmed: it is checked, or an item above it is (FR-CHK-04).
    pub dimmed: bool,
}

/// The checklist items of `task` at any depth, in tree order, each with its depth and whether it
/// is dimmed.
pub(super) fn lines(document: &Document, task: ItemId) -> Vec<ChecklistLine<'_>> {
    let mut lines = Vec::new();
    let mut pending: Vec<(ItemId, usize, bool)> = children(document, task)
        .rev()
        .map(|id| (id, 0, false))
        .collect();
    while let Some((id, depth, under_checked)) = pending.pop() {
        let item = item(document, id).expect("only checklist items are pending");
        let dimmed = under_checked || item.completed.is_some();
        lines.push(ChecklistLine {
            id,
            item,
            depth,
            dimmed,
        });
        pending.extend(
            children(document, id)
                .rev()
                .map(|id| (id, depth + 1, dimmed)),
        );
    }
    lines
}

/// How many of the checklist items of `id` at any depth are checked, and how many there are;
/// `None` when it has none (FR-TASK-03).
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

/// The transaction that takes checklist item `id` out of the document with the items under it.
/// Checklist items do not go to the Trash, which lists only what the pane shows as rows.
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

/// The transaction that moves checklist item `id` past the checklist item next to it among its
/// siblings, `direction` of it, with what each holds; an empty one when there is none that way.
pub(super) fn shift(document: &Document, id: ItemId, direction: Direction) -> Transaction {
    let siblings: Vec<ItemId> = document
        .position(id)
        .and_then(|(parent, _)| parent)
        .map_or_else(Vec::new, |parent| children(document, parent).collect());
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
            .expect("a checklist item fits beside a sibling of its own kind")
    })
}

/// The transaction that nests checklist item `id` as the last item of the item above it among
/// its siblings, checked or not, since checking an item changes nothing else (FR-CHK-03,
/// FR-CHK-04); `None` when there is none above it.
pub(super) fn indent(document: &Document, id: ItemId) -> Option<Transaction> {
    let (parent, _) = document.position(id)?;
    let above = children(document, parent?)
        .take_while(|&sibling| sibling != id)
        .last()?;
    let transaction = document
        .move_to(id, Destination::Last(Some(above)))
        .expect("a checklist item fits in another");
    Some(transaction)
}

/// The transaction that takes checklist item `id` out of the checklist item holding it, right
/// after it (FR-CHK-03); `None` when the task holds it.
pub(super) fn outdent(document: &Document, id: ItemId) -> Option<Transaction> {
    let (parent, _) = document.position(id)?;
    let parent = parent.filter(|&parent| {
        item(document, parent).is_some_and(|it| it.kind == ItemKind::ChecklistItem)
    })?;
    let transaction = document
        .move_to(id, Destination::After(parent))
        .expect("a checklist item fits beside the one holding it");
    Some(transaction)
}

/// Where a new checklist item of `task` goes on `side` of its item `selected`: beside it, in the
/// same item; with none selected, first or last in the task.
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
/// `selected`, where [`place`] puts a new one, in order, and the last of them; `None` when there
/// is nothing to paste. Every item in them becomes a checklist item, keeping its text, tags,
/// whether it is done and how the items nest (FR-CHK-03).
pub(super) fn paste(
    document: &mut Document,
    task: ItemId,
    selected: Option<ItemId>,
    side: Side,
    branches: &[Branch],
) -> Option<(Transaction, ItemId)> {
    let (parent, index) = place(document, task, selected, side);
    let mut last = None;
    let ops = branches
        .iter()
        .zip(index..)
        .map(|(branch, index)| {
            let subtree = document.graft(&checklist(branch));
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

/// `branch` with every item in it a checklist item keeping only what a checklist item shows: its
/// title, tags and whether it is done. A copied task's note, comments and dates stay with the
/// original; the clipboard reads each description line as an item of its own.
fn checklist(branch: &Branch) -> Branch {
    let node = match &branch.node {
        Node::Item(item) => Node::Item(Item {
            completed: item.completed,
            tags: item.tags.clone(),
            ..Item::new(ItemKind::ChecklistItem, item.title.clone())
        }),
        Node::Kept(text) => Node::Kept(text.clone()),
    };
    Branch {
        node,
        children: branch.children.iter().map(checklist).collect(),
    }
}

/// The children of `id` that are checklist items, in order.
fn children(document: &Document, id: ItemId) -> impl DoubleEndedIterator<Item = ItemId> + '_ {
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

    /// A task `task` in the Inbox with checklist items `pack`, holding `books`, which holds
    /// `novels`, and `plates` checked, holding `cups`, kept text between `pack` and `plates`.
    fn document() -> Result<(Document, [ItemId; 6]), PlacementError> {
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
        let books = add(&mut document, pack, "b")?;
        let novels = add(&mut document, books, "c")?;
        document.push(Some(task), Node::Kept("kept".to_owned()))?;
        let mut checked = Item::new(ItemKind::ChecklistItem, "d");
        checked.completed = Some(Completion::Undated);
        let plates = document.push(Some(task), Node::Item(checked))?;
        let cups = add(&mut document, plates, "e")?;
        Ok((document, [task, pack, books, novels, plates, cups]))
    }

    fn titles(document: &Document, task: ItemId) -> Vec<(&str, usize, bool)> {
        lines(document, task)
            .into_iter()
            .map(|line| (line.item.title.as_str(), line.depth, line.dimmed))
            .collect()
    }

    // FR-CHK-01, FR-CHK-04, FR-TASK-03
    #[test]
    fn a_checklist_lists_its_items_nested_and_dims_a_checked_one_with_what_it_holds()
    -> Result<(), PlacementError> {
        let (document, [task, pack, ..]) = document()?;
        assert_eq!(
            titles(&document, task),
            [
                ("a", 0, false),
                ("b", 1, false),
                ("c", 2, false),
                ("d", 0, true),
                ("e", 1, true),
            ]
        );
        assert_eq!(progress(&document, task), Some((1, 5)));
        assert_eq!(progress(&document, pack), Some((0, 2)));
        assert_eq!(progress(&document, document.inbox()), None);
        Ok(())
    }

    // FR-CHK-03, FR-CHK-04
    #[test]
    fn checking_an_item_changes_only_that_item() -> Result<(), PlacementError> {
        let (mut document, [task, pack, _, _, plates, _]) = document()?;
        document.apply(&toggle(&document, pack, TODAY))?;
        let checked = |document: &Document, id| item(document, id).and_then(|it| it.completed);
        assert_eq!(checked(&document, pack), Some(Completion::On(TODAY)));
        assert_eq!(
            titles(&document, task)[..3],
            [("a", 0, true), ("b", 1, true), ("c", 2, true)],
            "its children keep their state but are dimmed"
        );
        assert_eq!(progress(&document, task), Some((2, 5)));
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
    fn deleting_an_item_takes_what_it_holds_with_it() -> Result<(), PlacementError> {
        let (mut document, [task, _, books, ..]) = document()?;
        let original = document.clone();
        let deleted = delete(&document, books);
        document.apply(&deleted)?;
        assert_eq!(
            titles(&document, task),
            [("a", 0, false), ("d", 0, true), ("e", 1, true)]
        );
        document.apply(&deleted.inverse())?;
        assert_eq!(document, original);
        assert_eq!(delete(&document, ItemId::new(99)), Transaction::default());
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn an_item_moves_past_its_sibling_with_what_each_holds() -> Result<(), PlacementError> {
        let (mut document, [task, pack, books, _, plates, _]) = document()?;
        document.apply(&shift(&document, pack, Direction::Down))?;
        assert_eq!(
            titles(&document, task),
            [
                ("d", 0, true),
                ("e", 1, true),
                ("a", 0, false),
                ("b", 1, false),
                ("c", 2, false),
            ],
            "past the kept text to the next item"
        );
        assert_eq!(
            shift(&document, pack, Direction::Down),
            Transaction::default()
        );
        assert_eq!(
            shift(&document, books, Direction::Up),
            Transaction::default()
        );
        document.apply(&shift(&document, pack, Direction::Up))?;
        assert_eq!(titles(&document, task)[0], ("a", 0, false));
        assert_eq!(
            shift(&document, plates, Direction::Down),
            Transaction::default()
        );
        assert_eq!(
            shift(&document, ItemId::new(99), Direction::Up),
            Transaction::default()
        );
        Ok(())
    }

    // FR-CHK-03, FR-CHK-04
    #[test]
    fn an_item_nests_in_the_item_above_it_even_a_checked_one() -> Result<(), PlacementError> {
        let (mut document, [task, pack, books, ..]) = document()?;
        let last = Item::new(ItemKind::ChecklistItem, "f");
        let last = document.push(Some(task), Node::Item(last))?;
        assert_eq!(indent(&document, pack), None, "nothing above it");
        assert_eq!(indent(&document, books), None);
        assert_eq!(indent(&document, ItemId::new(99)), None);
        let transaction = indent(&document, last).ok_or(PlacementError::DoesNotFit)?;
        document.apply(&transaction)?;
        assert_eq!(
            titles(&document, task)[3..],
            [("d", 0, true), ("e", 1, true), ("f", 1, true)],
            "under the checked item, dimmed with it"
        );
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn an_item_comes_out_of_the_item_holding_it_right_after_it() -> Result<(), PlacementError> {
        let (mut document, [task, pack, books, novels, ..]) = document()?;
        assert_eq!(outdent(&document, pack), None, "the task holds it");
        assert_eq!(outdent(&document, ItemId::new(99)), None);
        let transaction = outdent(&document, novels).ok_or(PlacementError::DoesNotFit)?;
        document.apply(&transaction)?;
        assert_eq!(document.children(pack), [books, novels]);
        assert_eq!(
            titles(&document, task)[..3],
            [("a", 0, false), ("b", 1, false), ("c", 1, false)]
        );
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn a_new_item_goes_beside_the_one_selected_or_first_or_last() -> Result<(), PlacementError> {
        let (document, [task, pack, books, ..]) = document()?;
        assert_eq!(
            place(&document, task, Some(books), Side::Above),
            (Some(pack), 0)
        );
        assert_eq!(
            place(&document, task, Some(books), Side::Below),
            (Some(pack), 1)
        );
        assert_eq!(place(&document, task, None, Side::Above), (Some(task), 0));
        assert_eq!(place(&document, task, None, Side::Below), (Some(task), 3));
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn pasted_items_become_checklist_items_in_order() -> Result<(), PlacementError> {
        let (mut document, [task, pack, ..]) = document()?;
        let branch = |kind, title: &str, children| Branch {
            node: Node::Item(Item::new(kind, title)),
            children,
        };
        let pasted = [
            branch(
                ItemKind::Task,
                "x",
                vec![branch(ItemKind::Task, "y", Vec::new())],
            ),
            Branch {
                node: Node::Kept("text".to_owned()),
                children: Vec::new(),
            },
            branch(ItemKind::Project, "z", Vec::new()),
        ];
        let (transaction, last) = paste(&mut document, task, Some(pack), Side::Below, &pasted)
            .ok_or(PlacementError::DoesNotFit)?;
        document.apply(&transaction)?;
        assert_eq!(
            titles(&document, task)[3..6],
            [("x", 0, false), ("y", 1, false), ("z", 0, false)]
        );
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
