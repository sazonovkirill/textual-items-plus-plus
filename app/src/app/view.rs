//! The view registry: the views a user can open besides containers, such as Today, each in a
//! module of its own and registered here, which the Navigator lists.

/// A view besides a container. Each view step adds its variant and its [`Entry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {}

/// A view with what the Navigator shows of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The view.
    pub view: View,
    /// Its name as a user reads it.
    pub title: &'static str,
}

/// Every view, in the order the Navigator lists them.
pub const REGISTRY: &[Entry] = &[];
