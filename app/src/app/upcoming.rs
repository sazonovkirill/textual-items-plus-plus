//! The Upcoming view: every open item with a date after today, listed once on its start date, or
//! its due date when it has no start date, grouped by day for the week ahead and then by month,
//! as in Things 3 (FR-VIEW-02, OQ-8, OQ-20).

use jiff::{ToSpan, civil::Date};

use super::{
    container::Row,
    view::{Group, Heading},
};
use crate::domain::{
    Document, ItemId, ItemKind, Node,
    dates::{day_heading, month_name},
};

/// How many days after today are each a group of their own.
const DAYS: i64 = 7;

/// A group of the Upcoming view: the dates whose items it lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// One of the days of the week after today.
    Day(Date),
    /// The days of the month holding the last of those days that come after it, from this day on.
    RestOfMonth(Date),
    /// A whole later month, from its first day.
    Month(Date),
}

impl Period {
    /// The period holding `date` on `today`, a day after it.
    fn of(date: Date, today: Date) -> Self {
        let last_day = today.saturating_add(DAYS.days());
        if date <= last_day {
            Self::Day(date)
        } else if (date.year(), date.month()) == (last_day.year(), last_day.month()) {
            Self::RestOfMonth(last_day.saturating_add(1.day()))
        } else {
            Self::Month(date.first_of_month())
        }
    }

    /// Its heading on `today`, such as `Tomorrow · Thu 15 Oct`, `Rest of October` or `November`.
    #[must_use]
    pub fn heading(self, today: Date) -> String {
        match self {
            Self::Day(day) => day_heading(day, today),
            Self::RestOfMonth(first) => format!("Rest of {}", month_name(first, today)),
            Self::Month(first) => month_name(first, today),
        }
    }
}

/// The groups Upcoming shows of `document` on `today`: one for each period holding a date an item
/// is listed on, in date order, with its items in date order and then in file order. An item is
/// listed on its start date, else its due date, when that comes after `today` (OQ-8); a
/// container's dates are its own, so its contents are listed by theirs (OQ-20). It lists tasks,
/// sections and projects outside the Trash, and not the contents of a completed item
/// (FR-ORG-09); within each group the completed ones come last, and only when `show_completed`
/// (FR-VIEW-11). No group is empty.
#[must_use]
pub(super) fn groups(document: &Document, today: Date, show_completed: bool) -> Vec<Group> {
    let mut dated = Vec::new();
    for id in std::iter::once(document.inbox()).chain(document.top_level().iter().copied()) {
        collect(document, id, today, &mut dated);
    }
    // Stable, so each day keeps the file's order.
    dated.sort_by_key(|entry| entry.date);
    let mut periods: Vec<(Period, Vec<Entry>)> = Vec::new();
    for entry in dated
        .into_iter()
        .filter(|entry| show_completed || !entry.completed)
    {
        let period = Period::of(entry.date, today);
        match periods.last_mut() {
            Some((last, entries)) if *last == period => entries.push(entry),
            _ => periods.push((period, vec![entry])),
        }
    }
    periods
        .into_iter()
        .map(|(period, mut entries)| {
            entries.sort_by_key(|entry| entry.completed);
            Group {
                heading: Some(Heading::Period(period)),
                rows: entries.into_iter().map(|entry| entry.row).collect(),
            }
        })
        .collect()
}

/// An item Upcoming lists, with the day it is listed on.
struct Entry {
    date: Date,
    completed: bool,
    row: Row,
}

/// Adds `id` and the items inside it Upcoming lists to `dated`, in file order.
fn collect(document: &Document, id: ItemId, today: Date, dated: &mut Vec<Entry>) {
    let Some(item) = document.node(id).and_then(Node::item) else {
        return;
    };
    if item.deleted.is_some() {
        return;
    }
    let row = match item.kind {
        ItemKind::Task => Some(Row::Task { id, section: None }),
        ItemKind::Section | ItemKind::Project => Some(Row::Project(id)),
        // An area has no dates; ones a file gives it are not shown (D-75).
        ItemKind::Inbox | ItemKind::Area => None,
        ItemKind::ChecklistItem => return,
    };
    let date = item.start.or(item.due).filter(|&date| date > today);
    if let (Some(row), Some(date)) = (row, date) {
        dated.push(Entry {
            date,
            completed: item.completed.is_some(),
            row,
        });
    }
    if item.completed.is_none() {
        for &child in document.children(id) {
            collect(document, child, today, dated);
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{Completion, Item, PlacementError};

    /// A Wednesday.
    const TODAY: Date = date(2026, 10, 14);

    const TOMORROW: Option<Date> = Some(date(2026, 10, 15));

    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        start: Option<Date>,
        due: Option<Date>,
    ) -> Result<ItemId, PlacementError> {
        let mut item = Item::new(kind, "item");
        item.start = start;
        item.due = due;
        document.push(parent, Node::Item(item))
    }

    fn task(
        document: &mut Document,
        parent: ItemId,
        start: Option<Date>,
        due: Option<Date>,
    ) -> Result<ItemId, PlacementError> {
        add(document, Some(parent), ItemKind::Task, start, due)
    }

    /// Each group of Upcoming on `today`, as its heading and the items of its rows.
    fn listed(
        document: &Document,
        today: Date,
        show_completed: bool,
    ) -> Vec<(Option<Heading>, Vec<ItemId>)> {
        groups(document, today, show_completed)
            .into_iter()
            .map(|group| (group.heading, group.rows.into_iter().map(Row::id).collect()))
            .collect()
    }

    // FR-VIEW-02, OQ-8
    #[test]
    fn lists_each_item_once_on_its_start_date_else_its_due_date_after_today()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let due = task(&mut document, inbox, None, Some(date(2026, 10, 16)))?;
        let both = task(&mut document, inbox, TOMORROW, Some(date(2026, 10, 20)))?;
        task(&mut document, inbox, Some(TODAY), Some(date(2026, 10, 16)))?;
        task(&mut document, inbox, None, Some(TODAY))?;
        task(&mut document, inbox, Some(date(2026, 10, 1)), None)?;
        task(&mut document, inbox, None, None)?;
        assert_eq!(
            listed(&document, TODAY, false),
            [
                (
                    Some(Heading::Period(Period::Day(date(2026, 10, 15)))),
                    vec![both]
                ),
                (
                    Some(Heading::Period(Period::Day(date(2026, 10, 16)))),
                    vec![due]
                ),
            ]
        );
        let rows = &groups(&document, TODAY, false)[0].rows;
        assert_eq!(
            rows,
            &[Row::Task {
                id: both,
                section: None
            }]
        );
        Ok(())
    }

    // FR-VIEW-02, OQ-8: the week ahead day by day, then the rest of the month, then each month.
    #[test]
    fn groups_the_week_by_day_then_the_rest_of_the_month_then_by_month()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let dates = [
            date(2027, 2, 1),
            date(2026, 10, 21),
            date(2026, 10, 22),
            date(2026, 11, 30),
            date(2026, 10, 31),
            date(2026, 11, 1),
        ];
        let mut ids = Vec::new();
        for start in dates {
            ids.push(task(&mut document, inbox, Some(start), None)?);
        }
        let listed = listed(&document, TODAY, false);
        assert_eq!(
            listed,
            [
                (
                    Some(Heading::Period(Period::Day(date(2026, 10, 21)))),
                    vec![ids[1]]
                ),
                (
                    Some(Heading::Period(Period::RestOfMonth(date(2026, 10, 22)))),
                    vec![ids[2], ids[4]]
                ),
                (
                    Some(Heading::Period(Period::Month(date(2026, 11, 1)))),
                    vec![ids[5], ids[3]]
                ),
                (
                    Some(Heading::Period(Period::Month(date(2027, 2, 1)))),
                    vec![ids[0]]
                ),
            ]
        );
        let headings = [
            Period::Day(date(2026, 10, 21)),
            Period::RestOfMonth(date(2026, 10, 22)),
            Period::Month(date(2026, 11, 1)),
            Period::Month(date(2027, 2, 1)),
        ]
        .map(|period| period.heading(TODAY));
        assert_eq!(
            headings,
            [
                "Next Wed · Wed 21 Oct",
                "Rest of October",
                "November",
                "February 2027"
            ]
        );
        Ok(())
    }

    // OQ-8: when the week runs into the next month, the rest is of that month.
    #[test]
    fn the_rest_of_the_month_is_of_the_weeks_last_day() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let today = date(2026, 10, 28);
        let first = task(&mut document, inbox, Some(date(2026, 11, 4)), None)?;
        let rest = task(&mut document, inbox, Some(date(2026, 11, 5)), None)?;
        let december = task(&mut document, inbox, Some(date(2026, 12, 5)), None)?;
        assert_eq!(
            listed(&document, today, false),
            [
                (
                    Some(Heading::Period(Period::Day(date(2026, 11, 4)))),
                    vec![first]
                ),
                (
                    Some(Heading::Period(Period::RestOfMonth(date(2026, 11, 5)))),
                    vec![rest]
                ),
                (
                    Some(Heading::Period(Period::Month(date(2026, 12, 1)))),
                    vec![december]
                ),
            ]
        );
        assert_eq!(
            Period::RestOfMonth(date(2026, 11, 5)).heading(today),
            "Rest of November"
        );
        Ok(())
    }

    // FR-VIEW-02, D-13, OQ-20: containers are listed by their own dates, their contents by theirs,
    // a day's items in file order.
    #[test]
    fn lists_projects_and_sections_and_their_contents_in_file_order_but_no_area()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = add(&mut document, None, ItemKind::Area, TOMORROW, None)?;
        let project = add(&mut document, Some(area), ItemKind::Project, None, TOMORROW)?;
        let kind = ItemKind::Section;
        let section = add(&mut document, Some(project), kind, TOMORROW, None)?;
        let inside = task(&mut document, section, TOMORROW, None)?;
        let inbox = document.inbox();
        let loose = task(&mut document, inbox, TOMORROW, None)?;
        assert_eq!(
            listed(&document, TODAY, false),
            [(
                Some(Heading::Period(Period::Day(date(2026, 10, 15)))),
                vec![loose, project, section, inside]
            )],
            "an area has no dates (D-75)"
        );
        assert_eq!(
            groups(&document, TODAY, false)[0].rows[1],
            Row::Project(project)
        );
        Ok(())
    }

    // FR-VIEW-11, FR-ORG-09: completed items come last in their group and only when shown; what
    // they hold is not listed.
    #[test]
    fn completed_items_come_last_only_when_shown_without_what_they_hold()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let mut done = Item::new(ItemKind::Project, "done");
        done.completed = Some(Completion::On(TODAY));
        done.start = TOMORROW;
        let done = document.push(None, Node::Item(done))?;
        task(&mut document, done, TOMORROW, None)?;
        let open = add(&mut document, None, ItemKind::Project, TOMORROW, None)?;
        let tomorrow = Some(Heading::Period(Period::Day(date(2026, 10, 15))));
        assert_eq!(listed(&document, TODAY, false), [(tomorrow, vec![open])]);
        assert_eq!(
            listed(&document, TODAY, true),
            [(tomorrow, vec![open, done])]
        );
        Ok(())
    }

    // FR-VIEW-02, OQ-19: the Trash, checklist items and kept content are not listed.
    #[test]
    fn leaves_out_the_trash_checklist_items_and_kept_content() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let mut trashed = Item::new(ItemKind::Project, "trashed");
        trashed.deleted = Some(TODAY);
        trashed.start = TOMORROW;
        let trashed = document.push(None, Node::Item(trashed))?;
        task(&mut document, trashed, TOMORROW, None)?;
        let open = add(&mut document, None, ItemKind::Project, None, None)?;
        let holder = task(&mut document, open, None, None)?;
        let kind = ItemKind::ChecklistItem;
        add(&mut document, Some(holder), kind, TOMORROW, None)?;
        let kept = Node::Kept("- kept @start(2026-10-15)".to_owned());
        document.push(Some(open), kept)?;
        assert_eq!(groups(&document, TODAY, true), []);
        Ok(())
    }
}
