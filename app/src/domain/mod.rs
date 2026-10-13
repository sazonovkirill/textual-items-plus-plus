//! The Domain layer: the data model and the rules that change it, free of storage and terminal
//! concerns.

mod document;
mod item;

pub use document::{Document, PlacementError};
pub use item::{Completion, Item, ItemId, ItemKind, Node, Tag};
