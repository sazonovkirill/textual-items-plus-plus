//! Edits as operations that can be applied and inverted, so that every change, structural ones
//! included, can be undone and redone (FR-EDIT-06, N-UX-02).

use super::{Document, Entry, PlacementError};
use crate::domain::item::{Item, ItemId, Node};

/// A primitive change to a document; every edit, its undo and its redo is a sequence of these.
/// Each records the state it expects, so applying it to a document that no longer matches fails
/// instead of changing the wrong thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Replaces an item with another of the same kind.
    Set {
        /// The item.
        id: ItemId,
        /// What it holds before the change.
        before: Item,
        /// What it holds after the change.
        after: Item,
    },
    /// Puts a node with its subtree at a position.
    Insert {
        /// The new parent, or `None` for the top level.
        parent: Option<ItemId>,
        /// The position among the parent's children.
        index: usize,
        /// The nodes to put there.
        subtree: Snapshot,
    },
    /// Takes a node with its subtree out of the document.
    Remove {
        /// The parent it is taken from, or `None` for the top level.
        parent: Option<ItemId>,
        /// Its position among the parent's children.
        index: usize,
        /// The nodes taken, exactly as they are in the document.
        subtree: Snapshot,
    },
}

/// A node with its whole subtree and their identifiers, as [`Op::Insert`] puts it in and
/// [`Op::Remove`] takes it out. Removing and inserting the same snapshot moves a node, keeping
/// its identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    top: (ItemId, Entry),
    below: Vec<(ItemId, Entry)>,
}

/// The operations of one user command, applied together and undone as one step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Transaction {
    ops: Vec<Op>,
}

impl Op {
    /// The operation that undoes this one.
    #[must_use]
    pub fn inverse(self) -> Self {
        match self {
            Self::Set { id, before, after } => Self::Set {
                id,
                before: after,
                after: before,
            },
            Self::Insert {
                parent,
                index,
                subtree,
            } => Self::Remove {
                parent,
                index,
                subtree,
            },
            Self::Remove {
                parent,
                index,
                subtree,
            } => Self::Insert {
                parent,
                index,
                subtree,
            },
        }
    }
}

impl Snapshot {
    /// A new node without children; `id` comes from [`Document::new_id`].
    #[must_use]
    pub fn new(id: ItemId, node: Node) -> Self {
        Self {
            top: (
                id,
                Entry {
                    node,
                    children: Vec::new(),
                },
            ),
            below: Vec::new(),
        }
    }

    /// The identifier of the top node.
    #[must_use]
    pub fn id(&self) -> ItemId {
        self.top.0
    }

    /// Every node, top node first.
    fn entries(&self) -> impl Iterator<Item = &(ItemId, Entry)> {
        std::iter::once(&self.top).chain(&self.below)
    }
}

impl Transaction {
    /// The operations of one command, in the order they apply.
    #[must_use]
    pub fn new(ops: Vec<Op>) -> Self {
        Self { ops }
    }

    /// The operations in the order they apply.
    #[must_use]
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// The transaction that undoes this one: the inverse operations in reverse order.
    #[must_use]
    pub fn inverse(self) -> Self {
        Self {
            ops: self.ops.into_iter().rev().map(Op::inverse).collect(),
        }
    }
}

impl Document {
    /// A fresh identifier for a node to put in with [`Op::Insert`]; it names nothing until then.
    pub fn new_id(&mut self) -> ItemId {
        let id = ItemId::new(self.next_number);
        self.next_number += 1;
        id
    }

    /// The parent of `id`, `None` for the top level, and its position among the parent's
    /// children; `None` for the Inbox and for nodes not in this document.
    #[must_use]
    pub fn position(&self, id: ItemId) -> Option<(Option<ItemId>, usize)> {
        let index_in = |siblings: &[ItemId]| siblings.iter().position(|&child| child == id);
        if let Some(index) = index_in(&self.top_level) {
            return Some((None, index));
        }
        self.entries
            .iter()
            .find_map(|(&parent, entry)| Some((Some(parent), index_in(&entry.children)?)))
    }

    /// The node `id` with its subtree as it is now, for building an [`Op::Remove`]; `None` when
    /// this document has no such node.
    #[must_use]
    pub fn snapshot(&self, id: ItemId) -> Option<Snapshot> {
        let top = (id, self.entries.get(&id)?.clone());
        let mut below = Vec::new();
        let mut pending: Vec<ItemId> = top.1.children.iter().rev().copied().collect();
        while let Some(next) = pending.pop() {
            let entry = self.entries.get(&next)?.clone();
            pending.extend(entry.children.iter().rev());
            below.push((next, entry));
        }
        Some(Snapshot { top, below })
    }

    /// Applies every operation of `transaction` in order, or none of them.
    ///
    /// # Errors
    ///
    /// The [`PlacementError`] of the first operation that does not fit the document; the
    /// document is then left as it was.
    ///
    /// # Panics
    ///
    /// Never: undoing the operations that applied before a failing one always fits the document.
    pub fn apply(&mut self, transaction: &Transaction) -> Result<(), PlacementError> {
        for (applied, op) in transaction.ops.iter().enumerate() {
            if let Err(error) = self.apply_op(op) {
                for op in transaction.ops.iter().take(applied).rev() {
                    self.apply_op(&op.clone().inverse())
                        .expect("the inverse of an operation that just applied fits the document");
                }
                return Err(error);
            }
        }
        Ok(())
    }

    /// Applies one operation, or changes nothing when it does not fit.
    fn apply_op(&mut self, op: &Op) -> Result<(), PlacementError> {
        match op {
            Op::Set { id, before, after } => {
                if self.node(*id).and_then(Node::item) != Some(before) {
                    return Err(match self.node(*id) {
                        Some(Node::Item(_)) => PlacementError::Stale(*id),
                        _ => PlacementError::NoSuchNode(*id),
                    });
                }
                self.set(*id, after.clone()).map(drop)
            }
            Op::Insert {
                parent,
                index,
                subtree,
            } => self.insert(*parent, *index, subtree),
            Op::Remove {
                parent,
                index,
                subtree,
            } => self.remove(*parent, *index, subtree),
        }
    }

    fn insert(
        &mut self,
        parent: Option<ItemId>,
        index: usize,
        subtree: &Snapshot,
    ) -> Result<(), PlacementError> {
        self.check_fit(parent, &subtree.top.1.node)?;
        if index > self.siblings(parent)?.len() {
            return Err(PlacementError::Position { parent, index });
        }
        let made = ItemId::new(self.next_number);
        if let Some(&(id, _)) = subtree
            .entries()
            .find(|&&(id, _)| id >= made || self.entries.contains_key(&id))
        {
            return Err(PlacementError::IdUnavailable(id));
        }
        self.entries.extend(subtree.entries().cloned());
        self.siblings_mut(parent)?.insert(index, subtree.id());
        Ok(())
    }

    fn remove(
        &mut self,
        parent: Option<ItemId>,
        index: usize,
        subtree: &Snapshot,
    ) -> Result<(), PlacementError> {
        let id = subtree.id();
        if self.siblings(parent)?.get(index) != Some(&id)
            || self.snapshot(id).as_ref() != Some(subtree)
        {
            return Err(PlacementError::Stale(id));
        }
        for (id, _) in subtree.entries() {
            self.entries.remove(id);
        }
        self.siblings_mut(parent)?.remove(index);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proptest::prelude::*;

    use super::*;
    use crate::domain::ItemKind;
    use crate::domain::ItemKind::{Area, ChecklistItem, Inbox, Project, Section, Task};

    fn item(kind: ItemKind, title: &str) -> Node {
        Node::Item(Item::new(kind, title))
    }

    /// An area holding a section with a task and kept text, and a project; the Inbox holds a
    /// task with a checklist item.
    struct Fixture {
        document: Document,
        area: ItemId,
        section: ItemId,
        task: ItemId,
        project: ItemId,
        inboxed: ItemId,
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, item(Area, "Work"))?;
        let section = document.push(Some(area), item(Section, "Soon"))?;
        let task = document.push(Some(section), item(Task, "Write"))?;
        document.push(Some(section), Node::Kept("kept".to_owned()))?;
        let project = document.push(None, item(Project, "Home"))?;
        let inboxed = document.push(Some(document.inbox()), item(Task, "Call"))?;
        document.push(Some(inboxed), item(ChecklistItem, "Number"))?;
        Ok(Fixture {
            document,
            area,
            section,
            task,
            project,
            inboxed,
        })
    }

    fn renamed(document: &Document, id: ItemId, title: &str) -> Result<Op, PlacementError> {
        let before = document
            .node(id)
            .and_then(Node::item)
            .cloned()
            .ok_or(PlacementError::NoSuchNode(id))?;
        let after = Item {
            title: title.to_owned(),
            ..before.clone()
        };
        Ok(Op::Set { id, before, after })
    }

    fn removal(document: &Document, id: ItemId) -> Option<Op> {
        let (parent, index) = document.position(id)?;
        Some(Op::Remove {
            parent,
            index,
            subtree: document.snapshot(id)?,
        })
    }

    fn title(document: &Document, id: ItemId) -> Option<&str> {
        Some(document.node(id)?.item()?.title.as_str())
    }

    // FR-EDIT-06
    #[test]
    fn a_set_replaces_the_item_and_its_inverse_restores_it() -> Result<(), PlacementError> {
        let Fixture {
            mut document, task, ..
        } = fixture()?;
        let original = document.clone();
        let transaction = Transaction::new(vec![renamed(&document, task, "Rewrite")?]);
        document.apply(&transaction)?;
        assert_eq!(title(&document, task), Some("Rewrite"));
        document.apply(&transaction.inverse())?;
        assert_eq!(document, original);
        Ok(())
    }

    // FR-EDIT-06, N-UX-02
    #[test]
    fn a_removed_subtree_comes_back_where_it_was_with_its_identifiers() -> Result<(), PlacementError>
    {
        let Fixture {
            mut document,
            area,
            section,
            task,
            ..
        } = fixture()?;
        let original = document.clone();
        let transaction = Transaction::new(vec![
            removal(&document, section).ok_or(PlacementError::NoSuchNode(section))?,
        ]);
        document.apply(&transaction)?;
        assert_eq!(document.children(area), []);
        assert_eq!(document.node(section), None);
        assert_eq!(document.node(task), None);
        document.apply(&transaction.inverse())?;
        assert_eq!(document, original);
        assert_eq!(title(&document, task), Some("Write"));
        Ok(())
    }

    // FR-EDIT-06
    #[test]
    fn a_new_node_goes_in_at_its_position_and_its_inverse_takes_it_out()
    -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            section,
            task,
            ..
        } = fixture()?;
        let original = document.clone();
        let id = document.new_id();
        let insert = Op::Insert {
            parent: Some(section),
            index: 0,
            subtree: Snapshot::new(id, item(Task, "Plan")),
        };
        let transaction = Transaction::new(vec![insert]);
        document.apply(&transaction)?;
        assert_eq!(document.children(section).first(), Some(&id));
        assert_eq!(document.children(section).get(1), Some(&task));
        assert_eq!(document.position(id), Some((Some(section), 0)));
        document.apply(&transaction.inverse())?;
        assert_eq!(document.children(section), original.children(section));
        assert_eq!(document.node(id), None);
        Ok(())
    }

    // FR-EDIT-06
    #[test]
    fn removing_and_inserting_a_snapshot_moves_it_with_its_identifiers()
    -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            area,
            section,
            project,
            ..
        } = fixture()?;
        let original = document.clone();
        let subtree = document
            .snapshot(section)
            .ok_or(PlacementError::NoSuchNode(section))?;
        let transaction = Transaction::new(vec![
            Op::Remove {
                parent: Some(area),
                index: 0,
                subtree: subtree.clone(),
            },
            Op::Insert {
                parent: Some(project),
                index: 0,
                subtree,
            },
        ]);
        document.apply(&transaction)?;
        assert_eq!(document.children(project), [section]);
        assert_eq!(document.children(area), []);
        assert_eq!(document.position(section), Some((Some(project), 0)));
        document.apply(&transaction.inverse())?;
        assert_eq!(document, original);
        Ok(())
    }

    #[test]
    fn top_level_nodes_are_placed_like_children() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            area,
            project,
            ..
        } = fixture()?;
        assert_eq!(document.position(project), Some((None, 1)));
        let transaction = Transaction::new(vec![
            removal(&document, area).ok_or(PlacementError::NoSuchNode(area))?,
        ]);
        document.apply(&transaction)?;
        assert_eq!(document.top_level(), [project]);
        document.apply(&transaction.inverse())?;
        assert_eq!(document.top_level(), [area, project]);
        Ok(())
    }

    #[test]
    fn the_inbox_and_missing_nodes_have_no_position() -> Result<(), PlacementError> {
        let Fixture { document, .. } = fixture()?;
        assert_eq!(document.position(document.inbox()), None);
        assert_eq!(document.position(ItemId::new(99)), None);
        assert_eq!(document.snapshot(ItemId::new(99)), None);
        Ok(())
    }

    #[test]
    fn fresh_identifiers_name_nothing_and_are_never_repeated() {
        let mut document = Document::new();
        let first = document.new_id();
        let second = document.new_id();
        assert_ne!(first, second);
        assert_eq!(document.node(first), None);
        assert_ne!(first, document.inbox());
    }

    // FR-EDIT-06
    #[test]
    fn a_failing_operation_leaves_the_whole_transaction_unapplied() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            task,
            project,
            inboxed,
            ..
        } = fixture()?;
        let original = document.clone();
        let id = document.new_id();
        let original_with_id = document.clone();
        let transaction = Transaction::new(vec![
            renamed(&document, task, "Rewrite")?,
            removal(&document, inboxed).ok_or(PlacementError::NoSuchNode(inboxed))?,
            Op::Insert {
                parent: Some(project),
                index: 0,
                subtree: Snapshot::new(id, item(Task, "Plan")),
            },
            renamed(&document, task, "Again")?,
        ]);
        assert_eq!(
            document.apply(&transaction),
            Err(PlacementError::Stale(task))
        );
        assert_eq!(document, original_with_id);
        assert_ne!(document, original, "the reserved identifier stays used");
        Ok(())
    }

    #[test]
    fn a_set_refuses_a_changed_item_kept_content_and_missing_nodes() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            section,
            task,
            ..
        } = fixture()?;
        let original = document.clone();
        let stale = Op::Set {
            id: task,
            before: Item::new(Task, "Other"),
            after: Item::new(Task, "New"),
        };
        let kept = document
            .children(section)
            .get(1)
            .copied()
            .ok_or(PlacementError::NoSuchNode(section))?;
        let on_kept = Op::Set {
            id: kept,
            before: Item::new(Task, "kept"),
            after: Item::new(Task, "New"),
        };
        let missing = ItemId::new(99);
        let on_missing = Op::Set {
            id: missing,
            before: Item::new(Task, "x"),
            after: Item::new(Task, "New"),
        };
        let other_kind = Op::Set {
            id: task,
            before: Item::new(Task, "Write"),
            after: Item::new(Project, "Write"),
        };
        let refusals = [
            (stale, PlacementError::Stale(task)),
            (on_kept, PlacementError::NoSuchNode(kept)),
            (on_missing, PlacementError::NoSuchNode(missing)),
            (other_kind, PlacementError::KindChanged),
        ];
        for (op, error) in refusals {
            assert_eq!(document.apply(&Transaction::new(vec![op])), Err(error));
        }
        assert_eq!(document, original);
        Ok(())
    }

    // FR-ORG-01, FR-ORG-02, FR-ORG-03, FR-ORG-04
    #[test]
    fn an_insert_refuses_what_the_hierarchy_does_not_allow() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            area,
            task,
            ..
        } = fixture()?;
        let id = document.new_id();
        let original = document.clone();
        let insert = |parent, index, node| {
            Transaction::new(vec![Op::Insert {
                parent,
                index,
                subtree: Snapshot::new(id, node),
            }])
        };
        let missing = ItemId::new(99);
        let refusals = [
            (
                insert(None, 0, item(Section, "s")),
                PlacementError::DoesNotFit,
            ),
            (
                insert(Some(task), 0, item(Task, "t")),
                PlacementError::DoesNotFit,
            ),
            (
                insert(None, 0, item(Inbox, "i")),
                PlacementError::DoesNotFit,
            ),
            (
                insert(Some(missing), 0, item(Task, "t")),
                PlacementError::NoSuchNode(missing),
            ),
            (
                insert(Some(area), 2, item(Task, "t")),
                PlacementError::Position {
                    parent: Some(area),
                    index: 2,
                },
            ),
        ];
        for (transaction, error) in refusals {
            assert_eq!(document.apply(&transaction), Err(error));
        }
        assert_eq!(document, original);
        Ok(())
    }

    #[test]
    fn an_insert_refuses_identifiers_in_use_or_never_made() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            task,
            project,
            ..
        } = fixture()?;
        let original = document.clone();
        let unmade = ItemId::new(99);
        for id in [task, unmade] {
            let transaction = Transaction::new(vec![Op::Insert {
                parent: Some(project),
                index: 0,
                subtree: Snapshot::new(id, item(Task, "t")),
            }]);
            assert_eq!(
                document.apply(&transaction),
                Err(PlacementError::IdUnavailable(id))
            );
        }
        assert_eq!(document, original);
        Ok(())
    }

    #[test]
    fn a_remove_refuses_a_node_that_moved_or_changed() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            area,
            section,
            task,
            ..
        } = fixture()?;
        let subtree = document
            .snapshot(section)
            .ok_or(PlacementError::NoSuchNode(section))?;
        let change = Transaction::new(vec![renamed(&document, task, "Changed")?]);
        document.apply(&change)?;
        let original = document.clone();
        let remove = |parent, index, subtree| {
            Transaction::new(vec![Op::Remove {
                parent,
                index,
                subtree,
            }])
        };
        let current = document
            .snapshot(section)
            .ok_or(PlacementError::NoSuchNode(section))?;
        let missing = ItemId::new(99);
        let refusals = [
            (
                remove(Some(area), 0, subtree),
                PlacementError::Stale(section),
            ),
            (
                remove(Some(area), 1, current.clone()),
                PlacementError::Stale(section),
            ),
            (
                remove(None, 0, current.clone()),
                PlacementError::Stale(section),
            ),
            (
                remove(Some(missing), 0, current),
                PlacementError::NoSuchNode(missing),
            ),
        ];
        for (transaction, error) in refusals {
            assert_eq!(document.apply(&transaction), Err(error));
        }
        assert_eq!(document, original);
        Ok(())
    }

    #[test]
    fn a_snapshot_names_its_top_node() {
        let id = ItemId::new(5);
        assert_eq!(Snapshot::new(id, Node::Kept("x".to_owned())).id(), id);
    }

    /// What a document holds, leaving out which identifiers it has handed out, since undoing an
    /// insert does not take back its identifier.
    fn nodes(document: &Document) -> (&HashMap<ItemId, Entry>, &[ItemId]) {
        (&document.entries, &document.top_level)
    }

    /// Picks one of the fixture's edits by `choice`: a rename, a removal, an insert or a move.
    fn edit(document: &mut Document, ids: &[ItemId], choice: (usize, usize, u8)) -> Option<Op> {
        let (pick, place, kind) = choice;
        let id = *ids.get(pick % ids.len())?;
        match kind % 3 {
            0 => renamed(document, id, "renamed").ok(),
            1 => removal(document, id),
            _ => {
                let parent = *ids.get(place % ids.len())?;
                let index = document.children(parent).len();
                let fresh = document.new_id();
                Some(Op::Insert {
                    parent: Some(parent),
                    index,
                    subtree: Snapshot::new(fresh, Node::Kept("new".to_owned())),
                })
            }
        }
    }

    proptest! {
        // FR-EDIT-06, N-UX-02
        #[test]
        fn undoing_any_sequence_of_edits_restores_the_document(
            choices in prop::collection::vec((0..10usize, 0..10usize, 0..3u8), 0..12)
        ) {
            let Fixture { mut document, .. } = fixture()?;
            let mut ids = vec![document.inbox()];
            ids.extend(document.top_level().iter().copied());
            let mut pending = ids.clone();
            while let Some(id) = pending.pop() {
                pending.extend(document.children(id).iter().copied());
                ids.extend(document.children(id).iter().copied());
            }
            let mut applied = Vec::new();
            let mut states = Vec::new();
            for choice in choices {
                let Some(op) = edit(&mut document, &ids, choice) else { continue };
                let before = document.clone();
                let transaction = Transaction::new(vec![op]);
                if document.apply(&transaction).is_ok() {
                    states.push(before);
                    applied.push(transaction);
                } else {
                    prop_assert_eq!(&document, &before);
                }
            }
            while let (Some(transaction), Some(before)) = (applied.pop(), states.pop()) {
                document.apply(&transaction.inverse())?;
                prop_assert_eq!(nodes(&document), nodes(&before));
            }
        }
    }
}
