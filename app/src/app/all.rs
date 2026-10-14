//! The All view: every task in every list, grouped by where it lives, in place of Things 3's
//! Anytime and Someday (FR-VIEW-08, OQ-9). Hides how the groups follow from the document.

use super::view::{Group, Heading};
use super::{Row, container};
use crate::domain::{Document, ItemId, ItemKind, Node};

/// All's groups, as Things 3's Anytime groups them: the Inbox's tasks first under no heading,
/// scheduled ones included (FR-VIEW-01), then one group for each area and project with tasks of
/// its own, in the order of the outline, its tasks listed as its view lists them (FR-VIEW-05), so
/// its sections' tasks follow its other tasks, under no section heading. Completed tasks come after the open ones of their
/// list and are left out unless `show_completed` (FR-VIEW-11); tasks in the Trash and inside a
/// completed item are not shown (FR-ORG-09), and a group with no tasks is left out.
#[must_use]
pub(super) fn groups(document: &Document, show_completed: bool) -> Vec<Group> {
    let mut groups = Vec::new();
    let inbox = inbox_tasks(document, show_completed);
    if !inbox.is_empty() {
        groups.push(Group {
            heading: None,
            rows: inbox,
        });
    }
    for &id in document.top_level() {
        list(document, id, show_completed, &mut groups);
    }
    groups
}

/// The Inbox's tasks not in the Trash, the open ones and then, if shown, the completed ones, each
/// in file order.
fn inbox_tasks(document: &Document, show_completed: bool) -> Vec<Row> {
    let inbox = document.inbox();
    let tasks = |completed: bool| {
        document.children(inbox).iter().copied().filter(move |&id| {
            document.node(id).and_then(Node::item).is_some_and(|item| {
                item.kind == ItemKind::Task
                    && item.deleted.is_none()
                    && item.completed.is_some() == completed
            })
        })
    };
    let completed = show_completed.then(|| tasks(true));
    tasks(false)
        .chain(completed.into_iter().flatten())
        .map(|id| Row::Task { id, section: None })
        .collect()
}

/// Adds the group of `id`, if it is an area or project with tasks shown, and those of the areas
/// and projects inside it to `groups`. Items in the Trash and completed ones are passed over with
/// what they hold.
fn list(document: &Document, id: ItemId, show_completed: bool, groups: &mut Vec<Group>) {
    let Some(item) = document.node(id).and_then(Node::item) else {
        return;
    };
    if !matches!(item.kind, ItemKind::Area | ItemKind::Project)
        || item.deleted.is_some()
        || item.completed.is_some()
    {
        return;
    }
    // All shows no section headings, so its tasks are listed under none.
    let rows: Vec<Row> = container::rows(document, id, show_completed)
        .into_iter()
        .filter_map(|row| match row {
            Row::Task { id, .. } => Some(Row::Task { id, section: None }),
            Row::Section(_) | Row::Project(_) => None,
        })
        .collect();
    if !rows.is_empty() {
        groups.push(Group {
            heading: Some(Heading::Item(id)),
            rows,
        });
    }
    for &child in document.children(id) {
        list(document, child, show_completed, groups);
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{Completion, Item, PlacementError};
    use ItemKind::{Area, Project, Section, Task};

    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
    ) -> Result<ItemId, PlacementError> {
        document.push(parent, Node::Item(Item::new(kind, "x")))
    }

    fn task(id: ItemId, section: Option<ItemId>) -> Row {
        Row::Task { id, section }
    }

    // FR-VIEW-08, FR-VIEW-01
    #[test]
    fn inbox_tasks_come_first_without_a_heading_scheduled_ones_included()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let project = add(&mut document, None, Project)?;
        let in_project = add(&mut document, Some(project), Task)?;
        let open = add(&mut document, Some(inbox), Task)?;
        let mut scheduled = Item::new(Task, "later");
        scheduled.start = Some(date(2026, 10, 20));
        let scheduled = document.push(Some(inbox), Node::Item(scheduled))?;
        assert_eq!(
            groups(&document, false),
            [
                Group {
                    heading: None,
                    rows: vec![task(open, None), task(scheduled, None)]
                },
                Group {
                    heading: Some(Heading::Item(project)),
                    rows: vec![task(in_project, None)]
                }
            ]
        );
        Ok(())
    }

    // FR-VIEW-08, FR-VIEW-05
    #[test]
    fn tasks_group_under_their_direct_project_or_area_in_outline_order()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = add(&mut document, None, Area)?;
        let project = add(&mut document, Some(area), Project)?;
        let section = add(&mut document, Some(project), Section)?;
        let in_section = add(&mut document, Some(section), Task)?;
        let child = add(&mut document, Some(project), Project)?;
        let in_child = add(&mut document, Some(child), Task)?;
        let in_project = add(&mut document, Some(project), Task)?;
        let in_area = add(&mut document, Some(area), Task)?;
        let empty = add(&mut document, None, Project)?;
        add(&mut document, Some(empty), Section)?;
        document.push(None, Node::Kept("a line".to_owned()))?;
        assert_eq!(
            groups(&document, false),
            [
                Group {
                    heading: Some(Heading::Item(area)),
                    rows: vec![task(in_area, None)]
                },
                Group {
                    heading: Some(Heading::Item(project)),
                    rows: vec![task(in_project, None), task(in_section, None)]
                },
                Group {
                    heading: Some(Heading::Item(child)),
                    rows: vec![task(in_child, None)]
                }
            ],
            "the empty project, the empty Inbox and kept content are left out"
        );
        Ok(())
    }

    // FR-VIEW-08, FR-VIEW-11, FR-ORG-09
    #[test]
    fn completed_tasks_show_last_only_when_shown_and_hidden_ones_never()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let completed = |kind| {
            let mut item = Item::new(kind, "done");
            item.completed = Some(Completion::Undated);
            Node::Item(item)
        };
        let deleted = |kind| {
            let mut item = Item::new(kind, "gone");
            item.deleted = Some(date(2026, 10, 1));
            Node::Item(item)
        };
        let done_in_inbox = document.push(Some(inbox), completed(Task))?;
        let open_in_inbox = add(&mut document, Some(inbox), Task)?;
        document.push(Some(inbox), deleted(Task))?;
        let project = add(&mut document, None, Project)?;
        let done = document.push(Some(project), completed(Task))?;
        let open = add(&mut document, Some(project), Task)?;
        let done_section = document.push(Some(project), completed(Section))?;
        add(&mut document, Some(done_section), Task)?;
        let done_project = document.push(Some(project), completed(Project))?;
        let under_done = add(&mut document, Some(done_project), Project)?;
        add(&mut document, Some(under_done), Task)?;
        let trashed = document.push(None, deleted(Project))?;
        add(&mut document, Some(trashed), Task)?;
        assert_eq!(
            groups(&document, false),
            [
                Group {
                    heading: None,
                    rows: vec![task(open_in_inbox, None)]
                },
                Group {
                    heading: Some(Heading::Item(project)),
                    rows: vec![task(open, None)]
                }
            ]
        );
        assert_eq!(
            groups(&document, true),
            [
                Group {
                    heading: None,
                    rows: vec![task(open_in_inbox, None), task(done_in_inbox, None)]
                },
                Group {
                    heading: Some(Heading::Item(project)),
                    rows: vec![task(open, None), task(done, None)]
                }
            ]
        );
        Ok(())
    }
}
