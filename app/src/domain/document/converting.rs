//! Converting an item to another kind in its place, keeping its identifier and everything it
//! holds (FR-TASK-06, FR-ORG-06).

use super::{Document, PlacementError};
use crate::domain::{
    Op, Transaction,
    item::{ItemId, ItemKind, Node},
};

impl Document {
    /// The transaction that makes `id` an item of kind `kind` where it is, with everything it
    /// holds; a task's checklist items become tasks (FR-TASK-06). An empty one when it is of that
    /// kind already.
    ///
    /// # Errors
    ///
    /// [`PlacementError::NoSuchNode`] when `id` is not an item of this document, and
    /// [`PlacementError::DoesNotFit`] when it is the Inbox, or when the hierarchy does not allow
    /// an item of `kind` where it is or what it holds in one (OQ-2).
    pub fn convert(&self, id: ItemId, kind: ItemKind) -> Result<Transaction, PlacementError> {
        let item = self
            .node(id)
            .and_then(Node::item)
            .ok_or(PlacementError::NoSuchNode(id))?;
        if item.kind == kind {
            return Ok(Transaction::default());
        }
        let (parent, index) = self.position(id).ok_or(PlacementError::DoesNotFit)?;
        let parent_kind = parent.and_then(|parent| Some(self.node(parent)?.item()?.kind));
        if !kind.fits_in(parent_kind) {
            return Err(PlacementError::DoesNotFit);
        }
        let before = self.snapshot(id).ok_or(PlacementError::NoSuchNode(id))?;
        let mut after = before.clone();
        after.retype(id, kind);
        for &child in self.children(id) {
            let Some(child_kind) = self.node(child).and_then(Node::item).map(|it| it.kind) else {
                continue;
            };
            let child_kind = match child_kind {
                ItemKind::ChecklistItem => ItemKind::Task,
                other => other,
            };
            if !child_kind.fits_in(Some(kind)) {
                return Err(PlacementError::DoesNotFit);
            }
            after.retype(child, child_kind);
        }
        Ok(Transaction::new(vec![
            Op::Remove {
                parent,
                index,
                subtree: before,
            },
            Op::Insert {
                parent,
                index,
                subtree: after,
            },
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{Item, Tag};
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    fn item(kind: ItemKind, title: &str) -> Node {
        Node::Item(Item::new(kind, title))
    }

    fn kind(document: &Document, id: ItemId) -> Option<ItemKind> {
        Some(document.node(id)?.item()?.kind)
    }

    /// Applies the conversion and checks that undoing it restores the document.
    fn converted(document: &mut Document, id: ItemId, to: ItemKind) -> Result<(), PlacementError> {
        let original = document.clone();
        let transaction = document.convert(id, to)?;
        document.apply(&transaction)?;
        let mut undone = document.clone();
        undone.apply(&transaction.inverse())?;
        assert_eq!(undone, original, "undo restores the document");
        Ok(())
    }

    // FR-TASK-06
    #[test]
    fn a_task_becomes_a_project_in_its_place_its_checklist_its_tasks() -> Result<(), PlacementError>
    {
        let mut document = Document::new();
        let area = document.push(None, item(Area, "Home"))?;
        let first = document.push(Some(area), item(Task, "first"))?;
        let mut original = Item::new(Task, "Move house");
        original.description = "Before May.".to_owned();
        original.note = "Boxes".to_owned();
        original.comments = vec!["call them".to_owned()];
        let task = document.push(Some(area), Node::Item(original.clone()))?;
        let check = document.push(Some(task), item(ChecklistItem, "Pack"))?;
        let line = document.push(Some(task), Node::Kept("kept".to_owned()))?;
        converted(&mut document, task, Project)?;
        assert_eq!(document.children(area), [first, task], "in its place");
        assert_eq!(
            document.node(task),
            Some(&Node::Item(Item {
                kind: Project,
                ..original
            })),
            "everything else carries over"
        );
        assert_eq!(document.children(task), [check, line]);
        assert_eq!(kind(&document, check), Some(Task));
        assert_eq!(document.node(line), Some(&Node::Kept("kept".to_owned())));
        Ok(())
    }

    // FR-ORG-06
    #[test]
    fn areas_and_sections_become_projects_and_projects_sections() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, item(Area, "Work"))?;
        let section = document.push(Some(area), item(Section, "Soon"))?;
        let task = document.push(Some(section), item(Task, "Write"))?;
        let project = document.push(Some(area), item(Project, "Site"))?;
        converted(&mut document, section, Project)?;
        assert_eq!(kind(&document, section), Some(Project));
        assert_eq!(kind(&document, task), Some(Task));
        converted(&mut document, section, Section)?;
        assert_eq!(kind(&document, section), Some(Section));
        converted(&mut document, area, Project)?;
        assert_eq!(kind(&document, area), Some(Project));
        assert_eq!(document.children(area), [section, project]);
        assert_eq!(document.top_level(), [area]);
        Ok(())
    }

    // D-120
    #[test]
    fn a_bare_tag_the_new_kind_reads_as_a_field_becomes_that_field() -> Result<(), PlacementError> {
        let tagged = |kind, title, tags: &[Tag]| {
            let mut item = Item::new(kind, title);
            item.tags = tags.to_vec();
            Node::Item(item)
        };
        let valued = Tag {
            name: "pinned".to_owned(),
            value: Some("yes".to_owned()),
        };
        let (pinned, flagged, later) = (
            Tag::plain("pinned"),
            Tag::plain("flagged"),
            Tag::plain("later"),
        );
        let task = tagged(
            Task,
            "t",
            &[pinned.clone(), valued.clone(), Tag::plain("home")],
        );
        let check = tagged(
            ChecklistItem,
            "c",
            &[flagged.clone(), pinned.clone(), later],
        );
        let section = tagged(Section, "s", &[flagged, pinned.clone()]);
        let mut document = Document::new();
        let area = document.push(None, item(Area, "Home"))?;
        let task = document.push(Some(area), task)?;
        let check = document.push(Some(task), check)?;
        let section = document.push(Some(area), section)?;
        converted(&mut document, task, Project)?;
        converted(&mut document, section, Project)?;
        let item = |id| document.node(id).and_then(Node::item);
        let task = item(task).ok_or(PlacementError::NoSuchNode(task))?;
        assert!(task.pinned && !task.flagged && !task.later);
        assert_eq!(
            task.tags,
            [valued, Tag::plain("home")],
            "a valued tag stays"
        );
        let check = item(check).ok_or(PlacementError::NoSuchNode(check))?;
        assert!(check.flagged && check.later && !check.pinned);
        assert_eq!(
            check.tags,
            [pinned],
            "a task does not read @pinned as a pin"
        );
        let section = item(section).ok_or(PlacementError::NoSuchNode(section))?;
        assert!(section.pinned && section.flagged);
        assert_eq!(section.tags, []);
        Ok(())
    }

    #[test]
    fn converting_to_its_own_kind_changes_nothing() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let project = document.push(None, item(Project, "p"))?;
        assert_eq!(
            document.convert(project, Project),
            Ok(Transaction::default())
        );
        Ok(())
    }

    // FR-TASK-06, FR-ORG-06, OQ-2
    #[test]
    fn a_kind_that_cannot_go_there_or_hold_the_contents_is_refused() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inboxed = document.push(Some(document.inbox()), item(Task, "Call"))?;
        let area = document.push(None, item(Area, "Work"))?;
        let section = document.push(Some(area), item(Section, "Soon"))?;
        let in_section = document.push(Some(section), item(Task, "Write"))?;
        let top = document.push(None, item(Project, "Top"))?;
        let holder = document.push(Some(area), item(Project, "Holder"))?;
        document.push(Some(holder), item(Section, "Inner"))?;
        let line = document.push(None, Node::Kept("kept".to_owned()))?;
        let missing = ItemId::new(99);
        for (id, to, error) in [
            (inboxed, Project, PlacementError::DoesNotFit),
            (in_section, Project, PlacementError::DoesNotFit),
            (top, Section, PlacementError::DoesNotFit),
            (holder, Section, PlacementError::DoesNotFit),
            (document.inbox(), Project, PlacementError::DoesNotFit),
            (line, Project, PlacementError::NoSuchNode(line)),
            (missing, Project, PlacementError::NoSuchNode(missing)),
        ] {
            assert_eq!(document.convert(id, to), Err(error), "{id:?} to {to:?}");
        }
        Ok(())
    }
}
