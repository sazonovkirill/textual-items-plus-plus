//! The view registry: the views a user can open besides containers, such as Today, each in a
//! module of its own and registered here, and what a pane shows, a container or a view.

use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::{
    all,
    container::{self, Row},
    flagged, logbook, search, today, trash,
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
    /// The flagged projects and tasks, projects first (FR-VIEW-14).
    Flagged,
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
        view: View::Flagged,
        title: View::Flagged.title(),
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
            Self::Flagged => "Flagged",
            Self::All => "All",
            Self::Logbook => "Logbook",
            Self::Trash => "Trash",
            Self::Search => "Search",
        }
    }

    /// How each row says where its item is, if it does: the rows of Flagged, the Logbook, the
    /// Trash and search results, from every list, after their titles (FR-VIEW-14, FR-VIEW-03,
    /// FR-VIEW-04, FR-SRCH-01),
    /// and Today's, from no one container, under them (D-51).
    #[must_use]
    pub const fn locates(self) -> Option<Locating> {
        match self {
            Self::Flagged | Self::Logbook | Self::Trash | Self::Search => Some(Locating::After),
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
        View::Flagged => flagged::groups(document, show_completed),
        View::All => all::groups(document, show_completed),
        View::Logbook => logbook::groups(document),
        View::Trash => trash::groups(document),
        View::Search => search::groups(document, query),
    }
}

/// How many tasks `view` lists of `document` on `today`, completed ones hidden: the Logbook's
/// completed tasks and the Trash's deleted ones. Sections, projects and areas are not counted.
#[must_use]
pub(super) fn tasks(document: &Document, view: View, today: Date) -> usize {
    groups(document, view, today, false, "")
        .iter()
        .flat_map(|group| &group.rows)
        .filter(|row| matches!(row, Row::Task { .. }))
        .count()
}

/// The containers holding `id`, from the top level down, the Inbox left out: where a view's row
/// says its item is (FR-VIEW-03, D-126).
#[must_use]
pub fn location(document: &Document, id: ItemId) -> Vec<ItemId> {
    let mut path = container::path(document, id);
    path.pop();
    path.retain(|&container| container != document.inbox());
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

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{Completion, Item, ItemKind, Node, PlacementError};

    const TODAY: Date = date(2026, 10, 14);

    fn push(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
    ) -> Result<ItemId, PlacementError> {
        document.push(parent, Node::Item(Item::new(kind, title)))
    }

    // FR-VIEW-06
    #[test]
    fn the_location_is_the_containers_holding_an_item_from_the_top_level_down()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = push(&mut document, None, ItemKind::Area, "Work")?;
        let site = push(&mut document, Some(work), ItemKind::Project, "Site")?;
        let launch = push(&mut document, Some(site), ItemKind::Section, "Launch")?;
        let ship = push(&mut document, Some(launch), ItemKind::Task, "Ship")?;
        let hire = push(&mut document, Some(work), ItemKind::Task, "Hire")?;
        assert_eq!(location(&document, ship), [work, site, launch]);
        assert_eq!(location(&document, hire), [work]);
        assert_eq!(location(&document, launch), [work, site]);
        Ok(())
    }

    // FR-VIEW-06, D-126
    #[test]
    fn the_inbox_is_no_location() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let call = push(&mut document, Some(inbox), ItemKind::Task, "Call")?;
        assert_eq!(location(&document, call), [], "an Inbox task is nowhere");
        assert_eq!(location(&document, inbox), []);
        Ok(())
    }

    #[test]
    fn an_item_at_the_top_level_or_no_item_has_no_location() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = push(&mut document, None, ItemKind::Area, "Work")?;
        assert_eq!(location(&document, work), []);
        assert_eq!(location(&Document::new(), work), [], "no such node");
        Ok(())
    }

    // FR-VIEW-10
    #[test]
    fn each_view_counts_the_tasks_it_lists_with_completed_ones_hidden() -> Result<(), PlacementError>
    {
        use ItemKind::{Project, Section, Task};
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let mut push = |parent, item| document.push(parent, Node::Item(item));
        push(inbox, Item::new(Task, "Call the bank"))?;
        let mut home = Item::new(Project, "Home");
        home.flagged = true;
        let home = Some(push(None, home)?);
        let mut due = Item::new(Task, "Due today");
        due.due = Some(TODAY);
        due.flagged = true;
        push(home, due.clone())?;
        let mut later = Item::new(Task, "Starts later");
        later.start = Some(date(2026, 10, 17));
        push(home, later)?;
        let mut done = due;
        done.title = "Done".to_owned();
        done.completed = Some(Completion::On(TODAY));
        push(home, done)?;
        let mut deleted = Item::new(Task, "Deleted");
        deleted.deleted = Some(TODAY);
        push(home, deleted)?;
        let errands = Some(push(home, Item::new(Section, "Errands"))?);
        push(errands, Item::new(Task, "Milk"))?;
        let mut old = Item::new(Project, "Old");
        old.deleted = Some(TODAY);
        let old = Some(push(None, old)?);
        push(old, Item::new(Task, "Inside the Trash"))?;
        let counts: Vec<_> = REGISTRY
            .iter()
            .map(|entry| (entry.title, tasks(&document, entry.view, TODAY)))
            .collect();
        assert_eq!(
            counts,
            [
                ("Today", 1),
                ("Upcoming", 1),
                ("Flagged", 1),
                ("All", 4),
                ("Logbook", 1),
                ("Trash", 1),
            ],
            "neither the flagged project, the section nor the deleted project count"
        );
        assert_eq!(tasks(&Document::new(), View::All, TODAY), 0, "empty");
        Ok(())
    }
}
