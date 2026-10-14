//! The Flagged view: every flagged project and task from every list, the projects first
//! (FR-VIEW-14). It only collects them: a flagged item stays where it is in the file.

use super::{container::Row, view::Group};
use crate::domain::{Document, ItemId, ItemKind, Node};

/// The groups the Flagged view shows of `document`: the flagged projects, then the flagged tasks,
/// each under no heading and in file order, the completed ones after the open ones and left out
/// unless `show_completed` (FR-VIEW-11). Items in the Trash and inside a completed item, shown
/// nowhere (FR-ORG-09), are left out; a group with nothing in it is too.
pub(super) fn groups(document: &Document, show_completed: bool) -> Vec<Group> {
    let mut flagged = Vec::new();
    for &id in std::iter::once(&document.inbox()).chain(document.top_level()) {
        collect(document, id, &mut flagged);
    }
    flagged.retain(|&(_, completed, _)| show_completed || !completed);
    // Stable, so each keeps the file's order.
    flagged.sort_by_key(|&(project, completed, _)| (!project, completed));
    let (projects, tasks): (Vec<_>, Vec<_>) = flagged
        .into_iter()
        .map(|(project, _, id)| {
            if project {
                Row::Project(id)
            } else {
                Row::Task { id, section: None }
            }
        })
        .partition(|row| matches!(row, Row::Project(_)));
    [projects, tasks]
        .into_iter()
        .filter(|rows| !rows.is_empty())
        .map(|rows| Group {
            heading: None,
            rows,
        })
        .collect()
}

/// Adds `id`, if it is flagged, and the flagged items inside it to `flagged`, in file order,
/// each with whether it is a project and whether it is completed; an item in the Trash and what
/// it or a completed item holds are passed over.
fn collect(document: &Document, id: ItemId, flagged: &mut Vec<(bool, bool, ItemId)>) {
    let Some(item) = document.node(id).and_then(Node::item) else {
        return;
    };
    if item.deleted.is_some() {
        return;
    }
    let completed = item.completion().is_some();
    if item.flagged {
        flagged.push((item.kind == ItemKind::Project, completed, id));
    }
    if !completed {
        for &child in document.children(id) {
            collect(document, child, flagged);
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{Completion, Item, PlacementError};
    use ItemKind::{Area, Project, Section, Task};

    fn push(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
        flagged: bool,
    ) -> Result<ItemId, PlacementError> {
        let mut item = Item::new(kind, title);
        item.flagged = flagged;
        document.push(parent, Node::Item(item))
    }

    fn titles(document: &Document, show_completed: bool) -> Vec<Vec<String>> {
        groups(document, show_completed)
            .into_iter()
            .map(|group| {
                assert_eq!(group.heading, None);
                group
                    .rows
                    .into_iter()
                    .filter_map(|row| Some(document.node(row.id())?.item()?.title.clone()))
                    .collect()
            })
            .collect()
    }

    // FR-VIEW-14
    #[test]
    fn lists_the_flagged_projects_then_the_flagged_tasks_in_file_order()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        push(&mut document, inbox, Task, "Call", true)?;
        push(&mut document, inbox, Task, "Plain", false)?;
        let work = Some(push(&mut document, None, Area, "Work", false)?);
        let site = Some(push(&mut document, work, Project, "Site", true)?);
        let launch = Some(push(&mut document, site, Section, "Launch", false)?);
        push(&mut document, launch, Task, "Ship", true)?;
        let inner = Some(push(&mut document, site, Project, "Blog", true)?);
        push(&mut document, inner, Task, "Post", true)?;
        assert_eq!(
            titles(&document, false),
            [
                vec!["Site".to_owned(), "Blog".to_owned()],
                vec!["Call".to_owned(), "Ship".to_owned(), "Post".to_owned()],
            ]
        );
        Ok(())
    }

    // FR-VIEW-14, FR-VIEW-11, FR-ORG-09
    #[test]
    fn completed_ones_come_last_when_shown_and_the_trash_and_what_completed_items_hold_never()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let mut done = Item::new(Project, "Done");
        done.flagged = true;
        done.completed = Some(Completion::On(date(2026, 10, 13)));
        let done = Some(document.push(None, Node::Item(done))?);
        push(&mut document, done, Task, "Inside done", true)?;
        let mut trashed = Item::new(Project, "Trashed");
        trashed.deleted = Some(date(2026, 10, 13));
        let trashed = Some(document.push(None, Node::Item(trashed))?);
        push(&mut document, trashed, Task, "In the Trash", true)?;
        let open = Some(push(&mut document, None, Project, "Open", false)?);
        let mut checked = Item::new(Task, "Checked");
        checked.flagged = true;
        checked.completed = Some(Completion::Undated);
        document.push(open, Node::Item(checked))?;
        push(&mut document, open, Task, "Task", true)?;
        document.push(open, Node::Kept("kept".to_owned()))?;
        assert_eq!(titles(&document, false), [vec!["Task".to_owned()]]);
        assert_eq!(
            titles(&document, true),
            [
                vec!["Done".to_owned()],
                vec!["Task".to_owned(), "Checked".to_owned()],
            ]
        );
        Ok(())
    }
}
