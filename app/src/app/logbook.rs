//! The Logbook view: every completed item from every list, grouped by the day it was completed,
//! newest first (FR-VIEW-03). It only collects them: a completed item stays where it is in the
//! file (OQ-5).

use std::cmp::Reverse;

use jiff::civil::Date;

use super::{
    container::Row,
    view::{Group, Heading},
};
use crate::domain::{Completion, Document, ItemId, ItemKind, Node};

/// The groups the Logbook shows of `document`: the completed areas, projects, sections and tasks
/// under the day each was completed, the newest day first and those with no day recorded last, in
/// file order within a day, whatever Show Completed says (FR-VIEW-11). Items in the Trash, kept
/// content and the contents of a completed item, shown nowhere (FR-ORG-09), are left out;
/// checklist items are not items a list holds (FR-CHK-04).
pub(super) fn groups(document: &Document) -> Vec<Group> {
    let mut completed = Vec::new();
    let tops = std::iter::once(document.inbox()).chain(document.top_level().iter().copied());
    collect(document, tops, &mut completed);
    // Stable, so each day keeps the file's order.
    completed.sort_by_key(|&(day, _)| (day.is_none(), Reverse(day)));
    let mut groups: Vec<Group> = Vec::new();
    for (day, row) in completed {
        let heading = Some(Heading::Completed(day));
        match groups.last_mut() {
            Some(group) if group.heading == heading => group.rows.push(row),
            _ => groups.push(Group {
                heading,
                rows: vec![row],
            }),
        }
    }
    groups
}

/// Adds the completed items among `ids` and inside them to `completed`, in file order.
fn collect(
    document: &Document,
    ids: impl IntoIterator<Item = ItemId>,
    completed: &mut Vec<(Option<Date>, Row)>,
) {
    for id in ids {
        let Some(item) = document.node(id).and_then(Node::item) else {
            continue;
        };
        if item.deleted.is_some() || item.kind == ItemKind::ChecklistItem {
            continue;
        }
        match item.completion() {
            Some(completion) => completed.push((
                match completion {
                    Completion::On(day) => Some(day),
                    Completion::Undated => None,
                },
                match item.kind {
                    ItemKind::Task => Row::Task { id, section: None },
                    _ => Row::Project(id),
                },
            )),
            None => collect(document, document.children(id).iter().copied(), completed),
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{Item, PlacementError};

    fn push(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
        completed: Option<Completion>,
    ) -> Result<ItemId, PlacementError> {
        let mut item = Item::new(kind, title);
        item.completed = completed;
        document.push(parent, Node::Item(item))
    }

    fn titles(document: &Document) -> Vec<(Option<Heading>, Vec<String>)> {
        groups(document)
            .into_iter()
            .map(|group| {
                let titles = group
                    .rows
                    .into_iter()
                    .filter_map(|row| Some(document.node(row.id())?.item()?.title.clone()))
                    .collect();
                (group.heading, titles)
            })
            .collect()
    }

    // FR-VIEW-03
    #[test]
    fn lists_completed_items_by_day_newest_first_in_file_order_within_a_day()
    -> Result<(), PlacementError> {
        use ItemKind::{Area, Project, Section, Task};
        let (monday, tuesday) = (
            Some(Completion::On(date(2026, 10, 12))),
            Some(Completion::On(date(2026, 10, 13))),
        );
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        push(&mut document, inbox, Task, "Open", None)?;
        let undated = Some(Completion::Undated);
        push(&mut document, inbox, Task, "Undated", undated)?;
        push(&mut document, inbox, Task, "Called", monday)?;
        let work = Some(push(&mut document, None, Area, "Work", None)?);
        let site = Some(push(&mut document, work, Project, "Site", None)?);
        push(&mut document, site, Task, "Drafted", tuesday)?;
        push(&mut document, site, Section, "Launch", monday)?;
        push(&mut document, work, Project, "Moved", tuesday)?;
        assert_eq!(
            titles(&document),
            [
                (
                    Some(Heading::Completed(Some(date(2026, 10, 13)))),
                    vec!["Drafted".into(), "Moved".into()]
                ),
                (
                    Some(Heading::Completed(Some(date(2026, 10, 12)))),
                    vec!["Called".into(), "Launch".into()]
                ),
                (Some(Heading::Completed(None)), vec!["Undated".into()]),
            ]
        );
        Ok(())
    }

    // FR-ORG-09, FR-VIEW-04
    #[test]
    fn leaves_out_the_trash_checklists_and_what_a_completed_item_holds()
    -> Result<(), PlacementError> {
        use ItemKind::{ChecklistItem, Project, Task};
        let done = Some(Completion::On(date(2026, 10, 13)));
        let mut document = Document::new();
        let project = Some(push(&mut document, None, Project, "Done", done)?);
        push(&mut document, project, Task, "Inside", done)?;
        let mut trashed = Item::new(Project, "Trashed");
        trashed.deleted = Some(date(2026, 10, 13));
        let trashed = Some(document.push(None, Node::Item(trashed))?);
        push(&mut document, trashed, Task, "In the Trash", done)?;
        let open = Some(push(&mut document, None, Project, "Open", None)?);
        let task = Some(push(&mut document, open, Task, "Task", None)?);
        push(&mut document, task, ChecklistItem, "Checked", done)?;
        document.push(open, Node::Kept("kept".to_owned()))?;
        assert_eq!(
            titles(&document),
            [(
                Some(Heading::Completed(Some(date(2026, 10, 13)))),
                vec!["Done".into()]
            )]
        );
        Ok(())
    }
}
