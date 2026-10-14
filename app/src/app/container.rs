//! The container view: which rows an opened area, project, section or the Inbox shows, and in
//! what arrangement (FR-VIEW-05, D-26). Hides how the arrangement follows from the document.

use crate::domain::{Document, Item, ItemId, ItemKind, Node};

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
/// in file order whatever order the file mixes them in (FR-VIEW-05, FR-ORG-08). Within each group
/// Later items come after the open ones, and completed items after both, left out unless
/// `show_completed` (FR-VIEW-11, D-89).
/// The Inbox and a section hold only tasks, and the Inbox leaves out its scheduled ones
/// (FR-VIEW-01). Content kept as read, items in the Trash and the
/// contents of a completed item are not shown (FR-ORG-09); an item that is not a container, or
/// no item at all, shows nothing.
#[must_use]
pub fn rows(document: &Document, container: ItemId, show_completed: bool) -> Vec<Row> {
    let Some(kind) = document
        .node(container)
        .and_then(Node::item)
        .map(|item| item.kind)
    else {
        return Vec::new();
    };
    if document.conceals(container) {
        return Vec::new();
    }
    let group = Group {
        document,
        show_completed,
    };
    match kind {
        ItemKind::Inbox | ItemKind::Section => group.tasks(container, None).collect(),
        ItemKind::Area | ItemKind::Project => {
            let mut rows: Vec<Row> = group.tasks(container, None).collect();
            for section in group.shown(container, ItemKind::Section) {
                rows.push(Row::Section(section));
                if !document.conceals(section) {
                    rows.extend(group.tasks(section, Some(section)));
                }
            }
            rows.extend(group.shown(container, ItemKind::Project).map(Row::Project));
            rows
        }
        ItemKind::Task | ItemKind::ChecklistItem => Vec::new(),
    }
}

/// The containers from the top level down to `container`, itself last. The Inbox's is itself
/// alone; no node at all has none.
#[must_use]
pub(super) fn path(document: &Document, container: ItemId) -> Vec<ItemId> {
    if document.node(container).is_none() {
        return Vec::new();
    }
    let mut path = vec![container];
    while let Some(parent) = path.last().and_then(|&id| document.parent(id)) {
        path.push(parent);
    }
    path.reverse();
    path
}

/// The container whose rows list task `task`, completed ones shown: the one holding it, or the
/// one holding its section, which lists it under the section's heading (FR-VIEW-05); none for
/// anything else, or a task no container lists, as a scheduled one in the Inbox (FR-VIEW-01).
pub(super) fn listing(document: &Document, task: ItemId) -> Option<ItemId> {
    let kind = |id| document.node(id).and_then(Node::item).map(|item| item.kind);
    let parent = |id| document.position(id).and_then(|(parent, _)| parent);
    if kind(task)? != ItemKind::Task {
        return None;
    }
    let holder = parent(task)?;
    let container = match kind(holder)? {
        ItemKind::Section => parent(holder)?,
        _ => holder,
    };
    rows(document, container, true)
        .iter()
        .any(|row| row.id() == task)
        .then_some(container)
}

/// The projects, sections and tasks among `items`, the kinds rows show, without those inside
/// another of them, which go along with it, in order: the items a command on the selection takes
/// with everything inside them.
pub(super) fn outermost_rows(
    document: &Document,
    items: impl IntoIterator<Item = ItemId>,
) -> Vec<ItemId> {
    let rows: Vec<ItemId> = items
        .into_iter()
        .filter(|&id| {
            document.node(id).and_then(Node::item).is_some_and(|item| {
                matches!(
                    item.kind,
                    ItemKind::Project | ItemKind::Section | ItemKind::Task
                )
            })
        })
        .collect();
    rows.iter()
        .copied()
        .filter(|&id| {
            !path(document, id)
                .iter()
                .rev()
                .skip(1)
                .any(|above| rows.contains(above))
        })
        .collect()
}

/// Whether `id` is a container that can be opened: the Inbox, an area or a project, not a
/// section, which shows only in its container (D-80), neither it nor anything above it in the
/// Trash, and not inside a completed item, whose contents are shown nowhere (FR-ORG-09). A
/// completed container itself opens, as its row does.
pub(super) fn opens(document: &Document, id: ItemId) -> bool {
    let item = |id| document.node(id).and_then(Node::item);
    item(id).is_some_and(|item| {
        matches!(
            item.kind,
            ItemKind::Inbox | ItemKind::Area | ItemKind::Project
        )
    }) && path(document, id)
        .into_iter()
        .all(|id| item(id).is_some_and(|item| item.deleted.is_none()))
        && document
            .position(id)
            .and_then(|(parent, _)| parent)
            .is_none_or(|parent| !document.conceals(parent))
}

/// Where a pane shows `id`, as the container shown and the item focused there: a container that
/// opens itself, and a section the container holding it, when that opens, focused on the
/// section's heading, since a section opens in no view of its own (D-80); none for anything else.
pub(super) fn place(document: &Document, id: ItemId) -> Option<(ItemId, Option<ItemId>)> {
    if opens(document, id) {
        return Some((id, None));
    }
    let item = document.node(id).and_then(Node::item)?;
    let (parent, _) = document.position(id)?;
    let parent = parent.filter(|&parent| opens(document, parent))?;
    (item.kind == ItemKind::Section).then_some((parent, Some(id)))
}

/// Whether `item`, a child of `parent`, is an Inbox task with a start date, which the Inbox view
/// leaves out as scheduling does in Things 3; the file keeps it under the Inbox (FR-VIEW-01).
pub(super) fn scheduled_out_of_inbox(document: &Document, parent: ItemId, item: &Item) -> bool {
    parent == document.inbox() && item.start.is_some()
}

/// `ids` in their order, but those marked Later after the others (D-89).
pub(super) fn later_last(
    document: &Document,
    ids: &[ItemId],
) -> impl Iterator<Item = ItemId> + use<> {
    let later = |id| {
        document
            .node(id)
            .and_then(Node::item)
            .is_some_and(Item::is_later)
    };
    let (open, later): (Vec<ItemId>, Vec<ItemId>) = ids.iter().partition(|&&id| !later(id));
    open.into_iter().chain(later)
}

/// How the children of one container are picked and ordered into a group of rows.
#[derive(Clone, Copy)]
struct Group<'a> {
    document: &'a Document,
    show_completed: bool,
}

impl<'a> Group<'a> {
    /// The task rows of `parent`, listed under `section`.
    fn tasks(self, parent: ItemId, section: Option<ItemId>) -> impl Iterator<Item = Row> + 'a {
        self.shown(parent, ItemKind::Task)
            .map(move |id| Row::Task { id, section })
    }

    /// The children of `parent` of kind `kind` that are not in the Trash or scheduled out of the
    /// Inbox: the open ones, then the Later ones, then, if shown, the completed ones, each in file
    /// order (D-89).
    fn shown(self, parent: ItemId, kind: ItemKind) -> impl Iterator<Item = ItemId> + 'a {
        let children = move |stage: Stage| {
            self.document
                .children(parent)
                .iter()
                .copied()
                .filter(move |&id| {
                    self.document
                        .node(id)
                        .and_then(Node::item)
                        .is_some_and(|item| {
                            item.kind == kind
                                && item.deleted.is_none()
                                && Stage::of(item) == stage
                                && !scheduled_out_of_inbox(self.document, parent, item)
                        })
                })
        };
        let completed = self.show_completed.then(|| children(Stage::Completed));
        children(Stage::Open)
            .chain(children(Stage::Later))
            .chain(completed.into_iter().flatten())
    }
}

/// Where an item comes among its siblings in a group: open, Later, or completed, in this order
/// (FR-VIEW-11, D-89).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    Open,
    Later,
    Completed,
}

impl Stage {
    /// The stage `item` is at; a completed item is completed whether or not it is Later.
    pub(super) fn of(item: &Item) -> Self {
        if item.completion().is_some() {
            Self::Completed
        } else if item.is_later() {
            Self::Later
        } else {
            Self::Open
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{Completion, PlacementError};
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
            rows(&document, inbox, false),
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

    // FR-VIEW-01
    #[test]
    fn scheduled_tasks_leave_the_inbox_but_not_a_project() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let project = document.push(None, Node::Item(Item::new(Project, "p")))?;
        let mut scheduled = Item::new(Task, "later");
        scheduled.start = Some(date(2026, 10, 20));
        let mut started = Item::new(Task, "done");
        started.start = Some(date(2026, 10, 1));
        started.completed = Some(Completion::Undated);
        let mut shown = |parent| -> Result<(Vec<ItemId>, Vec<ItemId>), PlacementError> {
            let open = add(&mut document, parent, Task);
            let scheduled = document.push(Some(parent), Node::Item(scheduled.clone()))?;
            let started = document.push(Some(parent), Node::Item(started.clone()))?;
            let ids = rows(&document, parent, true).into_iter().map(Row::id);
            Ok((ids.collect(), vec![open, scheduled, started]))
        };
        let (in_inbox, added) = shown(inbox)?;
        assert_eq!(in_inbox, added[..1], "completed or not");
        let (in_project, added) = shown(project)?;
        assert_eq!(in_project, added);
        Ok(())
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
            rows(&document, project, false),
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
            !rows(&document, project, false)
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
            rows(&document, area, false),
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
            rows(&document, section, false),
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
            rows(&document, area, false),
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
        assert_eq!(rows(&document, task, false), []);
        assert_eq!(rows(&document, line, false), []);
        assert_eq!(rows(&document, ItemId::new(99), false), []);
        Ok(())
    }

    fn complete(document: &mut Document, id: ItemId) {
        let mut item = document
            .node(id)
            .and_then(Node::item)
            .cloned()
            .expect("the test completes an item");
        item.completed = Some(Completion::On(date(2026, 10, 14)));
        document.set(id, item).expect("the kind stays");
    }

    // FR-VIEW-11, FR-TASK-02
    #[test]
    fn completed_items_come_last_in_each_group_and_only_when_shown() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let project = document.push(None, Node::Item(Item::new(Project, "p")))?;
        let done_task = add(&mut document, project, Task);
        let task = add(&mut document, project, Task);
        let done_section = add(&mut document, project, Section);
        let section = add(&mut document, project, Section);
        let done_in_section = add(&mut document, section, Task);
        let in_section = add(&mut document, section, Task);
        let done_project = add(&mut document, project, Project);
        let child = add(&mut document, project, Project);
        for id in [done_task, done_section, done_in_section, done_project] {
            complete(&mut document, id);
        }
        let loose = |id| Row::Task { id, section: None };
        let under = |id| Row::Task {
            id,
            section: Some(section),
        };
        assert_eq!(
            rows(&document, project, false),
            [
                loose(task),
                Row::Section(section),
                under(in_section),
                Row::Project(child)
            ]
        );
        assert_eq!(
            rows(&document, project, true),
            [
                loose(task),
                loose(done_task),
                Row::Section(section),
                under(in_section),
                under(done_in_section),
                Row::Section(done_section),
                Row::Project(child),
                Row::Project(done_project),
            ]
        );
        Ok(())
    }

    // FR-ORG-11, FR-VIEW-11
    #[test]
    fn later_items_come_after_the_open_ones_and_before_the_completed() -> Result<(), PlacementError>
    {
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(Area, "a")))?;
        let later_task = add(&mut document, area, Task);
        let done_later = add(&mut document, area, Task);
        let task = add(&mut document, area, Task);
        let later_section = add(&mut document, area, Section);
        let section = add(&mut document, area, Section);
        let later_project = add(&mut document, area, Project);
        let project = add(&mut document, area, Project);
        let inbox = document.inbox();
        let later_in_inbox = add(&mut document, inbox, Task);
        let in_inbox = add(&mut document, inbox, Task);
        for id in [
            later_task,
            done_later,
            later_section,
            later_project,
            later_in_inbox,
        ] {
            let mut item = document
                .node(id)
                .and_then(Node::item)
                .cloned()
                .expect("an item");
            item.later = true;
            document.set(id, item)?;
        }
        complete(&mut document, done_later);
        let loose = |id| Row::Task { id, section: None };
        assert_eq!(
            rows(&document, area, true),
            [
                loose(task),
                loose(later_task),
                loose(done_later),
                Row::Section(later_section),
                Row::Section(section),
                Row::Project(project),
                Row::Project(later_project),
            ],
            "a section is never Later, so it keeps its place"
        );
        assert_eq!(
            rows(&document, inbox, false),
            [loose(in_inbox), loose(later_in_inbox)]
        );
        Ok(())
    }

    // FR-ORG-09
    #[test]
    fn a_completed_container_shows_none_of_its_contents() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(Area, "a")))?;
        let section = add(&mut document, area, Section);
        add(&mut document, section, Task);
        let project = add(&mut document, area, Project);
        add(&mut document, project, Task);
        complete(&mut document, section);
        assert_eq!(
            rows(&document, area, true),
            [Row::Section(section), Row::Project(project)]
        );
        assert_eq!(rows(&document, section, true), []);
        complete(&mut document, project);
        assert_eq!(rows(&document, project, true), []);
        complete(&mut document, area);
        assert_eq!(
            rows(&document, area, true),
            [Row::Section(section), Row::Project(project)],
            "an area marked done is not completed (D-81)"
        );
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

    // FR-VIEW-07
    #[test]
    fn a_path_runs_from_the_top_level_down_to_the_container() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(Area, "a")))?;
        let project = add(&mut document, area, Project);
        let section = add(&mut document, project, Section);
        assert_eq!(path(&document, section), [area, project, section]);
        assert_eq!(path(&document, area), [area]);
        assert_eq!(path(&document, document.inbox()), [document.inbox()]);
        assert_eq!(path(&document, ItemId::new(99)), []);
        Ok(())
    }

    // FR-NAV-02, OQ-19, D-80
    #[test]
    fn only_containers_but_sections_outside_the_trash_open() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let area = document.push(None, Node::Item(Item::new(Area, "a")))?;
        let project = add(&mut document, area, Project);
        let section = add(&mut document, project, Section);
        let task = add(&mut document, section, Task);
        let line = document.push(None, Node::Kept("text".to_owned()))?;
        for id in [inbox, area, project] {
            assert!(opens(&document, id), "{id:?}");
            assert_eq!(place(&document, id), Some((id, None)));
        }
        assert!(!opens(&document, section));
        assert_eq!(
            place(&document, section),
            Some((project, Some(section))),
            "a section shows in its project"
        );
        assert!(!opens(&document, task));
        assert_eq!(place(&document, task), None);
        assert_eq!(place(&document, ItemId::new(99)), None);
        assert!(!opens(&document, line));
        assert!(!opens(&document, ItemId::new(99)));
        let mut trashed = Item::new(Project, "x");
        trashed.deleted = Some(date(2026, 10, 1));
        document.set(project, trashed)?;
        assert!(!opens(&document, project));
        assert_eq!(
            place(&document, section),
            None,
            "inside the trashed project"
        );
        assert!(opens(&document, area));
        let mut completed = Item::new(Project, "x");
        completed.completed = Some(Completion::Undated);
        document.set(project, completed)?;
        assert!(opens(&document, project), "a completed container opens");
        assert!(!opens(&document, section), "what it holds does not");
        Ok(())
    }
}
