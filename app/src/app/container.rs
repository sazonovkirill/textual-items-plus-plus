//! The container view: which rows an opened area, project, section or the Inbox shows, and in
//! what arrangement (FR-VIEW-05, D-26). Hides how the arrangement follows from the document.

use crate::domain::{Document, ItemId, ItemKind, Node};

/// One row of a container view, the items the focus moves through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A task, with the section of the view it is listed under, if any.
    Task {
        /// The task.
        id: ItemId,
        /// The section whose heading it is listed under.
        section: Option<ItemId>,
    },
    /// A section's heading, followed by the section's tasks.
    Section(ItemId),
    /// A child project, as one row; its contents show only when it is opened.
    Project(ItemId),
}

impl Row {
    /// The item the row shows.
    #[must_use]
    pub fn id(self) -> ItemId {
        match self {
            Self::Task { id, .. } | Self::Section(id) | Self::Project(id) => id,
        }
    }
}

/// The rows `container` shows, in order: its tasks outside any section, then each of its
/// sections as a heading with the section's tasks below it, then its child projects, each group
/// in file order whatever order the file mixes them in (FR-VIEW-05, FR-ORG-08). The Inbox and a
/// section hold only tasks. Content kept as read and items in the Trash are not shown; an item
/// that is not a container, or no item at all, shows nothing.
#[must_use]
pub fn rows(document: &Document, container: ItemId) -> Vec<Row> {
    let Some(kind) = document
        .node(container)
        .and_then(Node::item)
        .map(|item| item.kind)
    else {
        return Vec::new();
    };
    match kind {
        ItemKind::Inbox | ItemKind::Section => tasks(document, container, None).collect(),
        ItemKind::Area | ItemKind::Project => {
            let mut rows: Vec<Row> = tasks(document, container, None).collect();
            for section in shown(document, container, ItemKind::Section) {
                rows.push(Row::Section(section));
                rows.extend(tasks(document, section, Some(section)));
            }
            rows.extend(shown(document, container, ItemKind::Project).map(Row::Project));
            rows
        }
        ItemKind::Task | ItemKind::ChecklistItem => Vec::new(),
    }
}

/// The task rows of `parent`, listed under `section`.
fn tasks(
    document: &Document,
    parent: ItemId,
    section: Option<ItemId>,
) -> impl Iterator<Item = Row> {
    shown(document, parent, ItemKind::Task).map(move |id| Row::Task { id, section })
}

/// The children of `parent` of kind `kind` that are not in the Trash, in file order.
fn shown(document: &Document, parent: ItemId, kind: ItemKind) -> impl Iterator<Item = ItemId> {
    document
        .children(parent)
        .iter()
        .copied()
        .filter(move |&id| {
            document
                .node(id)
                .and_then(Node::item)
                .is_some_and(|item| item.kind == kind && item.deleted.is_none())
        })
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{Item, PlacementError};
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    fn add(document: &mut Document, parent: ItemId, kind: ItemKind) -> ItemId {
        document
            .push(Some(parent), Node::Item(Item::new(kind, "x")))
            .expect("the test builds a valid hierarchy")
    }

    // FR-ORG-07
    #[test]
    fn the_inbox_shows_its_tasks_in_order() {
        let mut document = Document::new();
        let inbox = document.inbox();
        let first = add(&mut document, inbox, Task);
        let second = add(&mut document, inbox, Task);
        assert_eq!(
            rows(&document, inbox),
            [
                Row::Task {
                    id: first,
                    section: None
                },
                Row::Task {
                    id: second,
                    section: None
                }
            ]
        );
    }

    // FR-VIEW-05, FR-ORG-08, D-26
    #[test]
    fn a_project_shows_tasks_then_sections_then_projects_whatever_the_file_order()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let project = document.push(None, Node::Item(Item::new(Project, "p")))?;
        let child = add(&mut document, project, Project);
        let section = add(&mut document, project, Section);
        let late_task = add(&mut document, project, Task);
        let in_section = add(&mut document, section, Task);
        let in_child = add(&mut document, child, Task);
        let other_section = add(&mut document, project, Section);
        let early_task = add(&mut document, project, Task);
        assert_eq!(
            rows(&document, project),
            [
                Row::Task {
                    id: late_task,
                    section: None
                },
                Row::Task {
                    id: early_task,
                    section: None
                },
                Row::Section(section),
                Row::Task {
                    id: in_section,
                    section: Some(section)
                },
                Row::Section(other_section),
                Row::Project(child),
            ]
        );
        assert!(
            !rows(&document, project)
                .iter()
                .any(|row| row.id() == in_child)
        );
        Ok(())
    }

    // FR-VIEW-05
    #[test]
    fn an_area_shows_its_sections_and_projects_too() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(Area, "a")))?;
        let project = add(&mut document, area, Project);
        let section = add(&mut document, area, Section);
        assert_eq!(
            rows(&document, area),
            [Row::Section(section), Row::Project(project)]
        );
        Ok(())
    }

    // FR-VIEW-05
    #[test]
    fn an_opened_section_shows_its_tasks_outside_any_section() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(Area, "a")))?;
        let section = add(&mut document, area, Section);
        let task = add(&mut document, section, Task);
        assert_eq!(
            rows(&document, section),
            [Row::Task {
                id: task,
                section: None
            }]
        );
        Ok(())
    }

    // FR-STORE-09, OQ-18, OQ-19
    #[test]
    fn kept_content_checklists_and_the_trash_are_not_rows() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(Area, "a")))?;
        document.push(Some(area), Node::Kept("text".to_owned()))?;
        let task = add(&mut document, area, Task);
        add(&mut document, task, ChecklistItem);
        let mut deleted = Item::new(Section, "gone");
        deleted.deleted = Some(date(2026, 10, 1));
        let section = document.push(Some(area), Node::Item(deleted))?;
        add(&mut document, section, Task);
        let mut deleted_task = Item::new(Task, "gone");
        deleted_task.deleted = Some(date(2026, 10, 1));
        document.push(Some(area), Node::Item(deleted_task))?;
        assert_eq!(
            rows(&document, area),
            [Row::Task {
                id: task,
                section: None
            }]
        );
        Ok(())
    }

    #[test]
    fn a_task_kept_content_or_a_missing_node_shows_nothing() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let task = add(&mut document, inbox, Task);
        add(&mut document, task, ChecklistItem);
        let line = document.push(None, Node::Kept("text".to_owned()))?;
        assert_eq!(rows(&document, task), []);
        assert_eq!(rows(&document, line), []);
        assert_eq!(rows(&document, ItemId::new(99)), []);
        Ok(())
    }

    #[test]
    fn a_row_names_its_item() {
        let id = ItemId::new(4);
        for row in [
            Row::Task { id, section: None },
            Row::Section(id),
            Row::Project(id),
        ] {
            assert_eq!(row.id(), id);
        }
    }
}
