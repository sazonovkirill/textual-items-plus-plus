//! The Today view: every item whose start or due date has come, in one list, overdue ones first,
//! each row saying where its item is (FR-VIEW-06, OQ-9, D-51). Hides how the list follows from
//! the document.

use jiff::civil::Date;

use super::{
    container::Row,
    view::{Group, Heading},
};
use crate::domain::{Document, Item, ItemId, ItemKind, Node};

/// The one group Today shows of `document` on `today`, without a heading: the tasks, sections
/// and projects whose start or due date is `today` or earlier, the overdue ones, due before
/// `today` (FR-DATE-04), first, then the others, each in file order, then, only when
/// `show_completed`, the completed ones (FR-VIEW-11). A container's dates are its own (OQ-20).
/// A Later item, or one inside a Later project, shows only once it is due (D-92).
/// Items in the Trash and the contents of a completed item are not shown (FR-ORG-09). The items
/// put in the Evening on `today` follow in a group of their own under the Evening heading, in the
/// same order (FR-VIEW-13); a group with nothing to show is left out.
#[must_use]
pub(super) fn groups(document: &Document, today: Date, show_completed: bool) -> Vec<Group> {
    let mut walk = Walk {
        today,
        show_completed,
        entries: Vec::new(),
    };
    let tops = std::iter::once(document.inbox()).chain(document.top_level().iter().copied());
    for id in tops {
        walk.visit(document, id);
    }
    // Stable, so each bucket keeps the file's order.
    walk.entries
        .sort_by_key(|&(evening, bucket, _)| (evening, bucket));
    let (evening, day): (Vec<_>, Vec<_>) = walk
        .entries
        .into_iter()
        .partition(|&(evening, _, _)| evening);
    [(None, day), (Some(Heading::Evening), evening)]
        .into_iter()
        .filter(|(_, entries)| !entries.is_empty())
        .map(|(heading, entries)| Group {
            heading,
            rows: entries.into_iter().map(|(_, _, row)| row).collect(),
        })
        .collect()
}

/// The walk through the document in file order, collecting Today's rows with where in the list
/// each goes.
struct Walk {
    today: Date,
    show_completed: bool,
    /// Each row, with whether it is in the Evening and where in its part it goes.
    entries: Vec<(bool, Bucket, Row)>,
}

/// Where in the list a row goes, in order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Bucket {
    Overdue,
    Due,
    Completed,
}

impl Walk {
    /// Lists `id` and what it holds that shows.
    fn visit(&mut self, document: &Document, id: ItemId) {
        let Some(item) = document.node(id).and_then(Node::item) else {
            return;
        };
        if item.deleted.is_some() {
            return;
        }
        // A Later item waits for its due date, not its start date, as a deadline is not missed
        // (D-92).
        let waits = document.defers(id) && item.due.is_none_or(|due| due > self.today);
        if let Some(row) = self.row(id, item).filter(|_| !waits) {
            let bucket = if item.completion().is_some() {
                Bucket::Completed
            } else if item.overdue(self.today) {
                Bucket::Overdue
            } else {
                Bucket::Due
            };
            self.entries
                .push((item.in_evening(self.today), bucket, row));
        }
        if item.completion().is_some() {
            return;
        }
        for &child in document.children(id) {
            self.visit(document, child);
        }
    }

    /// The row Today lists `item`, numbered `id`, in, if it lists it.
    fn row(&self, id: ItemId, item: &Item) -> Option<Row> {
        let come = |date: Option<Date>| date.is_some_and(|date| date <= self.today);
        if !(come(item.start) || come(item.due))
            || item.completion().is_some() && !self.show_completed
        {
            return None;
        }
        match item.kind {
            ItemKind::Task => Some(Row::Task { id, section: None }),
            ItemKind::Section | ItemKind::Project => Some(Row::Project(id)),
            // An area has no dates; ones a file gives it are not shown (D-75).
            ItemKind::Inbox | ItemKind::Area | ItemKind::ChecklistItem => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::Completion;
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    const TODAY: Date = date(2026, 10, 14);

    /// Adds an item of `kind` to `parent`, starting and due on the days `start` and `due` from
    /// today, if any.
    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        start: Option<i8>,
        due: Option<i8>,
    ) -> ItemId {
        let mut item = Item::new(kind, "x");
        let day = |days: i8| TODAY.checked_add(jiff::Span::new().days(days)).ok();
        item.start = start.and_then(day);
        item.due = due.and_then(day);
        document
            .push(parent, Node::Item(item))
            .expect("the test builds a valid hierarchy")
    }

    fn task(id: ItemId) -> Row {
        Row::Task { id, section: None }
    }

    fn change(document: &mut Document, id: ItemId, edit: impl FnOnce(&mut Item)) {
        let mut item = document
            .node(id)
            .and_then(Node::item)
            .cloned()
            .expect("the test changes an item");
        edit(&mut item);
        document.set(id, item).expect("the kind stays");
    }

    // FR-ORG-11, FR-VIEW-06
    #[test]
    fn later_items_and_what_a_later_project_holds_show_only_once_due() {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let started = add(&mut document, inbox, Task, Some(0), None);
        let due = add(&mut document, inbox, Task, Some(-1), Some(0));
        let project = add(&mut document, None, Project, Some(0), None);
        let inside = add(&mut document, Some(project), Task, Some(-1), None);
        let overdue_inside = add(&mut document, Some(project), Task, None, Some(-1));
        let open = add(&mut document, inbox, Task, Some(0), None);
        for id in [started, due, project] {
            change(&mut document, id, |item| item.later = true);
        }
        assert!(document.defers(inside));
        assert_eq!(
            groups(&document, TODAY, false),
            [Group {
                heading: None,
                rows: vec![task(overdue_inside), task(due), task(open)],
            }]
        );
    }

    // FR-VIEW-06, FR-DATE-04, OQ-9, D-51
    #[test]
    fn items_whose_date_has_come_are_listed_together_overdue_first() {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let in_inbox = add(&mut document, inbox, Task, Some(0), None);
        add(&mut document, inbox, Task, None, None);
        add(&mut document, inbox, Task, Some(1), Some(2));
        let work = add(&mut document, None, Area, None, None);
        let started = add(&mut document, Some(work), Task, Some(-3), None);
        let site = add(&mut document, Some(work), Project, None, Some(0));
        let in_site = add(&mut document, Some(site), Task, None, Some(0));
        let launch = add(&mut document, Some(site), Section, Some(-1), None);
        let in_launch = add(&mut document, Some(launch), Task, None, Some(-2));
        let overdue = add(&mut document, Some(work), Task, None, Some(-1));
        let later = add(&mut document, None, Project, Some(5), None);
        add(&mut document, Some(later), Task, None, None);
        add(&mut document, None, Area, Some(0), None);
        assert_eq!(
            groups(&document, TODAY, false),
            [Group {
                heading: None,
                rows: vec![
                    task(in_launch),
                    task(overdue),
                    task(in_inbox),
                    task(started),
                    Row::Project(site),
                    task(in_site),
                    Row::Project(launch),
                ],
            }],
            "one list without headings, the overdue first, each part in file order, and no area, \
             which has no dates (D-75)"
        );
    }

    // FR-VIEW-13
    #[test]
    fn items_put_in_the_evening_today_follow_under_its_heading() {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let open = add(&mut document, inbox, Task, Some(0), None);
        let evening = add(&mut document, inbox, Task, Some(0), None);
        let yesterday = add(&mut document, inbox, Task, Some(-1), None);
        let overdue = add(&mut document, inbox, Task, None, Some(-1));
        let done = add(&mut document, inbox, Task, Some(0), None);
        let previous = TODAY.yesterday().expect("a day before");
        change(&mut document, yesterday, |item| {
            item.evening = Some(previous);
        });
        for id in [evening, overdue, done] {
            change(&mut document, id, |item| item.evening = Some(TODAY));
        }
        change(&mut document, done, |item| {
            item.completed = Some(Completion::On(TODAY));
        });
        assert_eq!(
            groups(&document, TODAY, true),
            [
                Group {
                    heading: None,
                    rows: vec![task(open), task(yesterday)],
                },
                Group {
                    heading: Some(Heading::Evening),
                    rows: vec![task(overdue), task(evening), task(done)],
                },
            ],
            "the Evening of an earlier day is over, and the Evening orders as the rest does"
        );
        for id in [open, yesterday] {
            change(&mut document, id, |item| item.evening = Some(TODAY));
        }
        assert_eq!(
            groups(&document, TODAY, false),
            [Group {
                heading: Some(Heading::Evening),
                rows: vec![task(overdue), task(open), task(evening), task(yesterday)],
            }]
        );
    }

    // FR-VIEW-11, FR-ORG-09, OQ-19, OQ-20
    #[test]
    fn completed_items_show_last_when_shown_and_hide_what_they_hold() {
        let mut document = Document::new();
        let project = add(&mut document, None, Project, None, Some(0));
        let done = add(&mut document, Some(project), Task, None, Some(-1));
        let open = add(&mut document, Some(project), Task, Some(0), None);
        let done_section = add(&mut document, Some(project), Section, None, None);
        add(&mut document, Some(done_section), Task, Some(0), None);
        for id in [done, done_section] {
            change(&mut document, id, |item| {
                item.completed = Some(Completion::On(TODAY));
            });
        }
        let undated = add(&mut document, None, Project, None, None);
        let trashed = add(&mut document, Some(undated), Task, Some(0), None);
        change(&mut document, trashed, |item| item.deleted = Some(TODAY));
        let trashed_section = add(&mut document, Some(undated), Section, None, None);
        add(&mut document, Some(trashed_section), Task, Some(0), None);
        change(&mut document, trashed_section, |item| {
            item.deleted = Some(TODAY);
        });
        let checked = add(&mut document, Some(project), Task, None, None);
        add(&mut document, Some(checked), ChecklistItem, Some(0), None);
        document
            .push(Some(project), Node::Kept("- @due(2026-10-14)".to_owned()))
            .expect("kept text goes anywhere");
        assert_eq!(
            groups(&document, TODAY, false),
            [Group {
                heading: None,
                rows: vec![Row::Project(project), task(open)],
            }]
        );
        assert_eq!(
            groups(&document, TODAY, true)[0].rows,
            [Row::Project(project), task(open), task(done)],
            "the completed section's task is hidden with it, and an overdue date no longer counts"
        );
        change(&mut document, project, |item| {
            item.completed = Some(Completion::On(TODAY));
        });
        assert_eq!(groups(&document, TODAY, false), []);
    }

    // D-81, FR-VIEW-06
    #[test]
    fn an_area_a_file_marks_done_is_open_and_lists_what_it_holds() {
        let mut document = Document::new();
        let area = add(&mut document, None, Area, None, None);
        change(&mut document, area, |item| {
            item.completed = Some(Completion::Undated);
        });
        let inside = add(&mut document, Some(area), Task, Some(0), None);
        assert_eq!(
            groups(&document, TODAY, false),
            [Group {
                heading: None,
                rows: vec![task(inside)],
            }]
        );
    }

    #[test]
    fn an_empty_document_has_nothing_today() {
        assert_eq!(groups(&Document::new(), TODAY, true), []);
    }
}
