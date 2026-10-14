//! The Today view: every item whose start or due date has come, grouped by the area or project it
//! is in, overdue ones first, as in Things 3 (FR-VIEW-06, OQ-9). Hides how the groups follow from
//! the document.

use jiff::civil::Date;

use super::{
    container::Row,
    view::{Group, Heading},
};
use crate::domain::{Document, Item, ItemId, ItemKind, Node};

/// The groups Today shows of `document` on `today`: the tasks, sections, projects and areas whose
/// start or due date is `today` or earlier, those in no project or area, as in the Inbox, first
/// and without a heading, then those of each project or area under it, the one nearest above them,
/// in file order. Within each group the overdue ones, due before `today` (FR-DATE-04), come first,
/// then the others, each in file order, then, only when `show_completed`, the completed ones
/// (FR-VIEW-11). A container's dates are its own (OQ-20). Items in the Trash and the contents of
/// a completed item are not shown (FR-ORG-09); no group is empty.
#[must_use]
pub(super) fn groups(document: &Document, today: Date, show_completed: bool) -> Vec<Group> {
    let mut walk = Walk {
        today,
        show_completed,
        headings: vec![None],
        entries: Vec::new(),
    };
    let tops = std::iter::once(document.inbox()).chain(document.top_level().iter().copied());
    for id in tops {
        walk.visit(document, id, None);
    }
    let Walk {
        headings,
        mut entries,
        ..
    } = walk;
    // Stable, so each bucket keeps the file's order.
    entries.sort_by_key(|entry| entry.bucket);
    headings
        .into_iter()
        .filter_map(|heading| {
            let rows: Vec<Row> = entries
                .iter()
                .filter(|entry| entry.heading == heading)
                .map(|entry| entry.row)
                .collect();
            (!rows.is_empty()).then_some(Group {
                heading: heading.map(Heading::Item),
                rows,
            })
        })
        .collect()
}

/// The walk through the document in file order, collecting Today's rows.
struct Walk {
    today: Date,
    show_completed: bool,
    /// The headings of the groups, in file order: none for the first, then every project and
    /// area passed.
    headings: Vec<Option<ItemId>>,
    entries: Vec<Entry>,
}

/// A row of Today, with the heading of its group and where in the group it goes.
struct Entry {
    heading: Option<ItemId>,
    bucket: Bucket,
    row: Row,
}

/// Where in its group a row goes, in order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Bucket {
    Overdue,
    Due,
    Completed,
}

impl Walk {
    /// Lists `id` and what it holds that shows, under `heading`, or under itself for what a
    /// project or area holds.
    fn visit(&mut self, document: &Document, id: ItemId, heading: Option<ItemId>) {
        let Some(item) = document.node(id).and_then(Node::item) else {
            return;
        };
        if item.deleted.is_some() {
            return;
        }
        if let Some(row) = self.row(id, item) {
            let bucket = if item.completed.is_some() {
                Bucket::Completed
            } else if item.overdue(self.today) {
                Bucket::Overdue
            } else {
                Bucket::Due
            };
            self.entries.push(Entry {
                heading,
                bucket,
                row,
            });
        }
        if item.completed.is_some() {
            return;
        }
        let inner = if matches!(item.kind, ItemKind::Area | ItemKind::Project) {
            self.headings.push(Some(id));
            Some(id)
        } else {
            heading
        };
        for &child in document.children(id) {
            self.visit(document, child, inner);
        }
    }

    /// The row Today lists `item`, numbered `id`, in, if it lists it.
    fn row(&self, id: ItemId, item: &Item) -> Option<Row> {
        let come = |date: Option<Date>| date.is_some_and(|date| date <= self.today);
        if !(come(item.start) || come(item.due)) || item.completed.is_some() && !self.show_completed
        {
            return None;
        }
        match item.kind {
            ItemKind::Task => Some(Row::Task { id, section: None }),
            ItemKind::Section | ItemKind::Project | ItemKind::Area => Some(Row::Project(id)),
            ItemKind::Inbox | ItemKind::ChecklistItem => None,
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

    // FR-VIEW-06, FR-DATE-04, OQ-9
    #[test]
    fn items_whose_date_has_come_group_under_their_project_or_area_overdue_first() {
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
        let dated_area = add(&mut document, None, Area, Some(0), None);
        assert_eq!(
            groups(&document, TODAY, false),
            [
                Group {
                    heading: None,
                    rows: vec![task(in_inbox), Row::Project(dated_area)],
                },
                Group {
                    heading: Some(Heading::Item(work)),
                    rows: vec![task(overdue), task(started), Row::Project(site)],
                },
                Group {
                    heading: Some(Heading::Item(site)),
                    rows: vec![task(in_launch), task(in_site), Row::Project(launch)],
                },
            ]
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
        let open_group = Group {
            heading: Some(Heading::Item(project)),
            rows: vec![task(open)],
        };
        assert_eq!(
            groups(&document, TODAY, false),
            [
                Group {
                    heading: None,
                    rows: vec![Row::Project(project)],
                },
                open_group,
            ]
        );
        assert_eq!(
            groups(&document, TODAY, true)[1].rows,
            [task(open), task(done)],
            "the completed section's task is hidden with it, and an overdue date no longer counts"
        );
        change(&mut document, project, |item| {
            item.completed = Some(Completion::On(TODAY));
        });
        assert_eq!(groups(&document, TODAY, false), []);
    }

    #[test]
    fn an_empty_document_has_nothing_today() {
        assert_eq!(groups(&Document::new(), TODAY, true), []);
    }
}
