//! The Navigator: the places a user can go, the Inbox and every area and project as a tree, in
//! the fuzzy picker a key opens over the panes (FR-VIEW-07, OQ-10). Hides how the tree and its
//! counts follow from the document; the views of the view registry join it as they arrive.

use super::{Candidate, Picker, PickerInput, TextEdit, container};
use crate::domain::{Document, Item, ItemId, ItemKind, Node};

/// A place the Navigator lists, with what it shows of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    /// The container it opens.
    pub id: ItemId,
    /// What the container is.
    pub kind: ItemKind,
    /// How deep in the tree it is: 0 for the Inbox and the top level.
    pub depth: usize,
    /// How many tasks not completed it holds, at any depth.
    pub open_tasks: usize,
}

/// The Navigator over `document`, focused on the entry of `shown`, else on that of the closest
/// container above it that has one, such as a section's project, as an editor's file tree follows
/// the open file.
pub(super) fn open(document: &Document, shown: ItemId) -> Picker<Place> {
    let mut picker = Picker::new(places(document));
    for id in container::path(document, shown).into_iter().rev() {
        if picker.focus_where(|place| place.id == id) {
            break;
        }
    }
    picker
}

/// The Navigator over `document` as [`open`] opens it, with `query` typed again, as after a
/// reload, which lists the places anew.
pub(super) fn reopen(document: &Document, shown: ItemId, query: &str) -> Picker<Place> {
    let mut picker = open(document, shown);
    for c in query.chars() {
        picker.input(PickerInput::Edit(TextEdit::Insert(c)));
    }
    picker
}

/// Every place, in the order of the tree: the Inbox, then each area and project at the top level
/// in file order, each followed by the projects inside it. Each matches by its title and is told
/// apart by the titles above it. Items in the Trash and completed ones are left out with what
/// they hold, as Things 3 takes completed projects out of its sidebar (FR-ORG-09).
pub(super) fn places(document: &Document) -> Vec<Candidate<Place>> {
    let mut places = Vec::new();
    let tops = std::iter::once(document.inbox()).chain(document.top_level().iter().copied());
    for id in tops {
        list(document, id, &mut Vec::new(), &mut places);
    }
    places
}

/// Adds the place `id` is, if it is one, and those inside it to `places`, under the titles
/// `above`.
fn list<'a>(
    document: &'a Document,
    id: ItemId,
    above: &mut Vec<&'a str>,
    places: &mut Vec<Candidate<Place>>,
) {
    let Some(item) = shown(document, id) else {
        return;
    };
    if !matches!(
        item.kind,
        ItemKind::Inbox | ItemKind::Area | ItemKind::Project
    ) {
        return;
    }
    places.push(Candidate {
        value: Place {
            id,
            kind: item.kind,
            depth: above.len(),
            open_tasks: open_tasks(document, id),
        },
        text: item.title.clone(),
        detail: above.join(" › "),
    });
    above.push(&item.title);
    for &child in document.children(id) {
        list(document, child, above, places);
    }
    above.pop();
}

/// How many tasks not completed `id` holds, at any depth, leaving out the Trash and what
/// completed items hold (FR-ORG-09).
fn open_tasks(document: &Document, id: ItemId) -> usize {
    document
        .children(id)
        .iter()
        .filter_map(|&child| Some((child, shown(document, child)?)))
        .map(|(child, item)| match item.kind {
            ItemKind::Task => 1,
            // Only tasks hold checklist items, so below a task there are no tasks to count.
            _ => open_tasks(document, child),
        })
        .sum()
}

/// The item `id`, unless it is not one, is in the Trash or is completed.
pub(super) fn shown(document: &Document, id: ItemId) -> Option<&Item> {
    document
        .node(id)
        .and_then(Node::item)
        .filter(|item| item.deleted.is_none() && item.completed.is_none())
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::{Picked, PickerInput};
    use crate::domain::{Completion, PlacementError};
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
    ) -> Result<ItemId, PlacementError> {
        document.push(parent, Node::Item(Item::new(kind, title)))
    }

    fn deleted(kind: ItemKind, title: &str) -> Node {
        let mut item = Item::new(kind, title);
        item.deleted = Some(date(2026, 10, 1));
        Node::Item(item)
    }

    /// The text, detail and place of each entry.
    fn listed(document: &Document) -> Vec<(String, String, Place)> {
        places(document)
            .into_iter()
            .map(|candidate| (candidate.text, candidate.detail, candidate.value))
            .collect()
    }

    // FR-VIEW-07
    #[test]
    fn lists_the_inbox_then_areas_and_projects_as_a_tree() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let work = add(&mut document, None, Area, "Work")?;
        let site = add(&mut document, Some(work), Project, "Site")?;
        add(&mut document, Some(work), Section, "Hiring")?;
        let blog = add(&mut document, Some(site), Project, "Blog")?;
        let solo = add(&mut document, None, Project, "Solo")?;
        document.push(None, Node::Kept("a line".to_owned()))?;
        let place = |id, kind, depth| Place {
            id,
            kind,
            depth,
            open_tasks: 0,
        };
        assert_eq!(
            listed(&document),
            [
                (
                    "Inbox".into(),
                    String::new(),
                    place(inbox, ItemKind::Inbox, 0)
                ),
                ("Work".into(), String::new(), place(work, Area, 0)),
                ("Site".into(), "Work".into(), place(site, Project, 1)),
                ("Blog".into(), "Work › Site".into(), place(blog, Project, 2)),
                ("Solo".into(), String::new(), place(solo, Project, 0)),
            ]
        );
        Ok(())
    }

    // FR-VIEW-07, OQ-19
    #[test]
    fn counts_open_tasks_at_any_depth_leaving_out_the_trash_and_completed_items()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        add(&mut document, Some(inbox), Task, "a")?;
        let work = add(&mut document, None, Area, "Work")?;
        let task = add(&mut document, Some(work), Task, "b")?;
        add(&mut document, Some(task), ChecklistItem, "not a task")?;
        let section = add(&mut document, Some(work), Section, "S")?;
        add(&mut document, Some(section), Task, "c")?;
        let mut done = Item::new(Task, "done");
        done.completed = Some(Completion::Undated);
        document.push(Some(section), Node::Item(done))?;
        document.push(Some(section), deleted(Task, "gone"))?;
        let project = add(&mut document, Some(work), Project, "P")?;
        add(&mut document, Some(project), Task, "d")?;
        let trashed = document.push(Some(work), deleted(Project, "Trashed"))?;
        add(&mut document, Some(trashed), Task, "e")?;
        add(&mut document, Some(trashed), Project, "Inner")?;
        document.push(None, deleted(Area, "Old"))?;
        let mut finished = Item::new(Project, "Finished");
        finished.completed = Some(Completion::Undated);
        let finished = document.push(Some(work), Node::Item(finished))?;
        add(&mut document, Some(finished), Task, "f")?;
        add(&mut document, Some(finished), Project, "Within")?;
        let counts: Vec<(String, usize)> = listed(&document)
            .into_iter()
            .map(|(text, _, place)| (text, place.open_tasks))
            .collect();
        assert_eq!(
            counts,
            [("Inbox".into(), 1), ("Work".into(), 3), ("P".into(), 1)]
        );
        Ok(())
    }

    // FR-VIEW-07
    #[test]
    fn opens_on_the_shown_container_or_the_closest_one_above_it_listed()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = add(&mut document, None, Area, "Work")?;
        let site = add(&mut document, Some(work), Project, "Site")?;
        let section = add(&mut document, Some(site), Section, "Launch")?;
        let chosen = |shown| {
            let mut picker = open(&document, shown);
            picker.input(PickerInput::Choose)
        };
        let place = |id| {
            places(&document)
                .into_iter()
                .find(|candidate| candidate.value.id == id)
                .map(|candidate| Picked::Chosen(candidate.value))
        };
        assert_eq!(chosen(site), place(site));
        assert_eq!(chosen(section), place(site));
        assert_eq!(chosen(document.inbox()), place(document.inbox()));
        assert_eq!(
            chosen(ItemId::new(99)),
            place(document.inbox()),
            "with no entry it opens on the first"
        );
        Ok(())
    }
}
