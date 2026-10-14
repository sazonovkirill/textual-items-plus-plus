//! The document's store of nodes: each node's entry and parent, found by its identifier without
//! hashing or searching, since identifiers are numbered from zero up (N-PERF-01, N-PERF-02).

use super::Entry;
use crate::domain::ItemId;

/// Every node's entry and parent, at the number of its identifier. Two stores are equal when
/// they hold the same entries, whatever the parents, which follow from the entries.
#[derive(Debug, Clone, Default)]
pub(super) struct Nodes {
    slots: Vec<Option<Slot>>,
}

#[derive(Debug, Clone)]
struct Slot {
    entry: Entry,
    parent: Option<ItemId>,
}

impl PartialEq for Nodes {
    fn eq(&self, other: &Self) -> bool {
        self.entries().eq(other.entries())
    }
}

impl Eq for Nodes {}

impl Nodes {
    /// The entry of `id`, if this store has one.
    pub(super) fn get(&self, id: ItemId) -> Option<&Entry> {
        self.slot(id).map(|slot| &slot.entry)
    }

    /// The entry of `id` to change, if this store has one.
    pub(super) fn get_mut(&mut self, id: ItemId) -> Option<&mut Entry> {
        self.slot_mut(id).map(|slot| &mut slot.entry)
    }

    /// The parent of `id`, `None` for a node at the top level or not in this store.
    pub(super) fn parent(&self, id: ItemId) -> Option<ItemId> {
        self.slot(id)?.parent
    }

    /// Puts in `entry` as `id`, under `parent`, or at the top level for `None`.
    pub(super) fn insert(&mut self, id: ItemId, entry: Entry, parent: Option<ItemId>) {
        let index = index(id);
        if self.slots.len() <= index {
            self.slots.resize(index + 1, None);
        }
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = Some(Slot { entry, parent });
        }
    }

    /// Takes `id` out.
    pub(super) fn remove(&mut self, id: ItemId) {
        if let Some(slot) = self.slots.get_mut(index(id)) {
            *slot = None;
        }
    }

    /// Makes `parent` the parent of `id`.
    pub(super) fn set_parent(&mut self, id: ItemId, parent: Option<ItemId>) {
        if let Some(slot) = self.slot_mut(id) {
            slot.parent = parent;
        }
    }

    fn slot(&self, id: ItemId) -> Option<&Slot> {
        self.slots.get(index(id))?.as_ref()
    }

    fn slot_mut(&mut self, id: ItemId) -> Option<&mut Slot> {
        self.slots.get_mut(index(id))?.as_mut()
    }

    /// Every entry with the number of its identifier, in that order.
    fn entries(&self) -> impl Iterator<Item = (usize, &Entry)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| Some((index, &slot.as_ref()?.entry)))
    }
}

/// Where the node `id` is kept.
fn index(id: ItemId) -> usize {
    usize::try_from(id.number()).expect("a document numbers no more nodes than memory holds")
}
