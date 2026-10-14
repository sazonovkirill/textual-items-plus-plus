//! The view registry: the views a user can open besides containers, such as Today, each in a
//! module of its own and registered here, and what a pane shows, a container or a view.

use jiff::civil::Date;

use super::{
    container::{self, Row},
    today,
};
use crate::domain::{Document, ItemId};

/// A view besides a container. Each view step adds its variant and its [`Entry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    /// The open items whose start or due date has come (FR-VIEW-06).
    Today,
}

/// A view with what the Navigator shows of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The view.
    pub view: View,
    /// Its name as a user reads it.
    pub title: &'static str,
}

/// Every view, in the order the Navigator lists them.
pub const REGISTRY: &[Entry] = &[Entry {
    view: View::Today,
    title: View::Today.title(),
}];

impl View {
    /// Its name as a user reads it.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Today => "Today",
        }
    }
}

/// What a pane shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shown {
    /// A container, as FR-VIEW-05 arranges it.
    Container(ItemId),
    /// A view.
    View(View),
}

/// A run of a view's rows under one heading, drawn above them and not a row itself, so that the
/// focus and the selection pass over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The area or project the rows are listed under, if any.
    pub heading: Option<ItemId>,
    /// The rows, in order.
    pub rows: Vec<Row>,
}

/// The groups `view` shows of `document` on `today`, completed items among them only when
/// `show_completed` (FR-VIEW-11); no group is empty.
#[must_use]
pub(super) fn groups(
    document: &Document,
    view: View,
    today: Date,
    show_completed: bool,
) -> Vec<Group> {
    match view {
        View::Today => today::groups(document, today, show_completed),
    }
}

/// Whether `shown` can be shown: a view always, a container when it [opens](container::opens).
pub(super) fn opens(document: &Document, shown: Shown) -> bool {
    match shown {
        Shown::Container(id) => container::opens(document, id),
        Shown::View(_) => true,
    }
}

/// The rows `shown` lists of `document` on `today`, in order.
#[must_use]
pub(super) fn rows(
    document: &Document,
    shown: Shown,
    today: Date,
    show_completed: bool,
) -> Vec<Row> {
    match shown {
        Shown::Container(id) => container::rows(document, id, show_completed),
        Shown::View(view) => groups(document, view, today, show_completed)
            .into_iter()
            .flat_map(|group| group.rows)
            .collect(),
    }
}
