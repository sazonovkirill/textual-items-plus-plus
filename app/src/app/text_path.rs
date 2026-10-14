//! Finding an item of one document again in another read from the same file, such as after a
//! reload: by its text path, the kind and title of each item from the top level down to it, each
//! counted among its siblings of that kind and title
//! ([file-io.md §3](../../../specs/reference/taskpaper-terminal/specs/file-io.md#3-external-changes)).
//! Hides how the path is made and followed.

use serde::{Deserialize, Serialize};

use super::container;
use crate::domain::{Document, ItemId, ItemKind, Node};

/// Where an item is, as its text path, which outlives the ids of one document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct TextPath(Vec<Step>);

/// One item of a [`TextPath`]: its kind and title, and how many of its siblings before it have
/// both too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Step {
    kind: ItemKind,
    title: String,
    nth: usize,
}

impl TextPath {
    /// The text path of `id` in `document`, which finds nothing when `document` has no such
    /// item.
    pub(super) fn of(document: &Document, id: ItemId) -> Self {
        let steps: Option<Vec<_>> = container::path(document, id)
            .into_iter()
            .map(|step| {
                place(document, step).map(|(kind, title, nth)| Step {
                    kind,
                    title: title.to_owned(),
                    nth,
                })
            })
            .collect();
        Self(steps.unwrap_or_default())
    }

    /// The item at this path in `document`, and `true`; else the deepest item found of those
    /// above it, and `false`; `None` when not even the first is found. The Inbox is always found,
    /// as itself.
    pub(super) fn find(&self, document: &Document) -> Option<(ItemId, bool)> {
        let mut found = None;
        let mut siblings = document.top_level();
        for step in &self.0 {
            let next = if step.kind == ItemKind::Inbox {
                Some(document.inbox())
            } else {
                siblings
                    .iter()
                    .copied()
                    .filter(|&sibling| is(document, sibling, step.kind, &step.title))
                    .nth(step.nth)
            };
            let Some(next) = next else {
                return found.map(|found| (found, false));
            };
            found = Some(next);
            siblings = document.children(next);
        }
        found.map(|found| (found, true))
    }

    /// The item at the whole of this path in `document`, if there is one.
    pub(super) fn exact(&self, document: &Document) -> Option<ItemId> {
        self.find(document)
            .and_then(|(id, whole)| whole.then_some(id))
    }
}

/// Where `id` of `old` is in `new`, as [`TextPath::find`] finds it.
pub(super) fn find(old: &Document, new: &Document, id: ItemId) -> Option<(ItemId, bool)> {
    TextPath::of(old, id).find(new)
}

/// The kind and title of the item `id`, and how many of its siblings before it have both too.
fn place(document: &Document, id: ItemId) -> Option<(ItemKind, &str, usize)> {
    let item = document.node(id).and_then(Node::item)?;
    let siblings = match document.position(id) {
        Some((Some(parent), _)) => document.children(parent),
        Some((None, _)) => document.top_level(),
        // The Inbox, which has none.
        None => &[],
    };
    let nth = siblings
        .iter()
        .take_while(|&&sibling| sibling != id)
        .filter(|&&sibling| is(document, sibling, item.kind, &item.title))
        .count();
    Some((item.kind, &item.title, nth))
}

/// Whether `id` is an item of `kind` titled `title`.
fn is(document: &Document, id: ItemId, kind: ItemKind, title: &str) -> bool {
    document
        .node(id)
        .and_then(Node::item)
        .is_some_and(|item| item.kind == kind && item.title == title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Item, PlacementError};

    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
    ) -> Result<ItemId, PlacementError> {
        document.push(parent, Node::Item(Item::new(kind, title)))
    }

    // F-FILE-06
    #[test]
    fn finds_an_item_by_the_titles_above_it_and_its_place_among_equal_siblings()
    -> Result<(), PlacementError> {
        let mut old = Document::new();
        let work = add(&mut old, None, ItemKind::Area, "Work")?;
        add(&mut old, Some(work), ItemKind::Task, "x")?;
        let second = add(&mut old, Some(work), ItemKind::Task, "x")?;
        let inbox = old.inbox();
        let inboxed = add(&mut old, Some(inbox), ItemKind::Task, "i")?;
        let mut new = Document::new();
        new.push(None, Node::Kept("text".to_owned()))?;
        add(&mut new, None, ItemKind::Project, "Work")?;
        let new_work = add(&mut new, None, ItemKind::Area, "Work")?;
        add(&mut new, Some(new_work), ItemKind::Task, "y")?;
        add(&mut new, Some(new_work), ItemKind::Task, "x")?;
        let new_second = add(&mut new, Some(new_work), ItemKind::Task, "x")?;
        let inbox = new.inbox();
        let new_inboxed = add(&mut new, Some(inbox), ItemKind::Task, "i")?;
        assert_eq!(find(&old, &new, work), Some((new_work, true)));
        assert_eq!(find(&old, &new, second), Some((new_second, true)));
        assert_eq!(find(&old, &new, inboxed), Some((new_inboxed, true)));
        assert_eq!(find(&old, &new, old.inbox()), Some((new.inbox(), true)));
        Ok(())
    }

    // F-FILE-06
    #[test]
    fn an_item_not_found_gives_way_to_the_deepest_item_above_it_found() -> Result<(), PlacementError>
    {
        let mut old = Document::new();
        let work = add(&mut old, None, ItemKind::Area, "Work")?;
        let site = add(&mut old, Some(work), ItemKind::Project, "Site")?;
        let task = add(&mut old, Some(site), ItemKind::Task, "t")?;
        let inbox = old.inbox();
        let inboxed = add(&mut old, Some(inbox), ItemKind::Task, "i")?;
        let mut new = Document::new();
        let new_work = add(&mut new, None, ItemKind::Area, "Work")?;
        assert_eq!(find(&old, &new, task), Some((new_work, false)));
        assert_eq!(find(&old, &new, inboxed), Some((new.inbox(), false)));
        assert_eq!(find(&old, &Document::new(), task), None);
        assert_eq!(find(&old, &new, ItemId::new(99)), None, "not in old");
        Ok(())
    }
}
