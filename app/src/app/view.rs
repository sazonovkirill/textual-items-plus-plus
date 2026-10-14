//! The view registry: the views a user can open besides containers, such as Today, each in a
//! module of its own and registered here, and what a pane shows, a container or a view.

use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::{
    all,
    container::{self, Row},
    logbook, search, today, trash,
    upcoming::{self, Period},
};
use crate::domain::{Document, ItemId};

/// A view besides a container. Each view step adds its variant and its [`Entry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    /// The open items whose start or due date has come (FR-VIEW-06).
    Today,
    /// The open items dated after today, by day and then by month (FR-VIEW-02).
    Upcoming,
    /// Every task in every list, grouped by where it lives (FR-VIEW-08).
    All,
    /// Every completed item, by the day it was completed (FR-VIEW-03).
    Logbook,
    /// The deleted items (FR-VIEW-04).
    Trash,
    /// The items holding the words of the pane's search (FR-SRCH-01). Not in the registry: it is
    /// opened by searching, not as a place.
    Search,
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
pub const REGISTRY: &[Entry] = &[
    Entry {
        view: View::Today,
        title: View::Today.title(),
    },
    Entry {
        view: View::Upcoming,
        title: View::Upcoming.title(),
    },
    Entry {
        view: View::All,
        title: View::All.title(),
    },
    Entry {
        view: View::Logbook,
        title: View::Logbook.title(),
    },
    Entry {
        view: View::Trash,
        title: View::Trash.title(),
    },
];

impl View {
    /// Its name as a user reads it.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Upcoming => "Upcoming",
            Self::All => "All",
            Self::Logbook => "Logbook",
            Self::Trash => "Trash",
            Self::Search => "Search",
        }
    }

    /// How each row says where its item is, if it does: the rows of the Logbook, the Trash and
    /// search results, from every list, after their titles (FR-VIEW-03, FR-VIEW-04, FR-SRCH-01),
    /// and Today's, from no one container, under them (D-51).
    #[must_use]
    pub const fn locates(self) -> Option<Locating> {
        match self {
            Self::Logbook | Self::Trash | Self::Search => Some(Locating::After),
            Self::Today => Some(Locating::Under),
            Self::Upcoming | Self::All => None,
        }
    }
}

/// Where a view's row says where its item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locating {
    /// After its title and tags.
    After,
    /// On the line under its title.
    Under,
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
    /// What the rows are listed under, if anything.
    pub heading: Option<Heading>,
    /// The rows, in order.
    pub rows: Vec<Row>,
}

/// What a group of a view's rows is listed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heading {
    /// The area or project they are in.
    Item(ItemId),
    /// The days they are dated in.
    Period(Period),
    /// The day they were completed, `None` for none recorded.
    Completed(Option<Date>),
    /// Today's Evening, which they were put in (FR-VIEW-13).
    Evening,
}

/// The groups `view` shows of `document` on `today`, completed items among them only when
/// `show_completed` (FR-VIEW-11), search results for `query`; no group is empty.
#[must_use]
pub(super) fn groups(
    document: &Document,
    view: View,
    today: Date,
    show_completed: bool,
    query: &str,
) -> Vec<Group> {
    match view {
        View::Today => today::groups(document, today, show_completed),
        View::Upcoming => upcoming::groups(document, today, show_completed),
        View::All => all::groups(document, show_completed),
        View::Logbook => logbook::groups(document),
        View::Trash => trash::groups(document),
        View::Search => search::groups(document, query),
    }
}

/// The containers holding `id`, from the top level down: where a view's row says its item is
/// (FR-VIEW-03).
#[must_use]
pub fn location(document: &Document, id: ItemId) -> Vec<ItemId> {
    let mut path = container::path(document, id);
    path.pop();
    path
}

/// Whether `shown` can be shown: a view always, a container when it [opens](container::opens).
pub(super) fn opens(document: &Document, shown: Shown) -> bool {
    match shown {
        Shown::Container(id) => container::opens(document, id),
        Shown::View(_) => true,
    }
}

/// The rows `shown` lists of `document` on `today`, in order, search results for `query`.
#[must_use]
pub(super) fn rows(
    document: &Document,
    shown: Shown,
    today: Date,
    show_completed: bool,
    query: &str,
) -> Vec<Row> {
    match shown {
        Shown::Container(id) => container::rows(document, id, show_completed),
        Shown::View(view) => groups(document, view, today, show_completed, query)
            .into_iter()
            .flat_map(|group| group.rows)
            .collect(),
    }
}
