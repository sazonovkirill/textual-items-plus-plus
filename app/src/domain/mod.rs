//! The Domain layer: the data model and the rules that change it, free of storage and terminal
//! concerns.

mod document;
mod item;

pub use document::{Branch, Destination, Document, Op, PlacementError, Snapshot, Transaction};
pub use item::{Completion, Item, ItemId, ItemKind, Node, Tag};
