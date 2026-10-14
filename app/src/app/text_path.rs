//! Finding an item of one document again in another read from the same file, such as after a
//! reload: by its text path, the kind and title of each item from the top level down to it, each
//! counted among its siblings of that kind and title
//! ([file-io.md §3](../../../specs/reference/taskpaper-terminal/specs/file-io.md#3-external-changes)).
//! Hides how the path is made and followed.

use super::container;
use crate::domain::{Document, ItemId, ItemKind, Node};

/// Where `id` of `old` is in `new`: the item there with the same text path, and `true`; else the
/// deepest item found of those above it, and `false`; `None` when not even the first is found.
/// The Inbox is always found, as itself.
pub(super) fn find(old: &Document, new: &Document, id: ItemId) -> Option<(ItemId, bool)> {
    let path = container::path(old, id);
    let mut found = None;
    let mut siblings = new.top_level();
    for &step in &path {
        let next = if step == old.inbox() {
            Some(new.inbox())
        } else {
            place(old, step).and_then(|(kind, title, nth)| {
                siblings
                    .iter()
                    .copied()
                    .filter(|&sibling| is(new, sibling, kind, title))
                    .nth(nth)
            })
        };
        let Some(next) = next else {
            return found.map(|found| (found, false));
        };
        found = Some(next);
        siblings = new.children(next);
    }
    found.map(|found| (found, true))
}

/// The kind and title of the item `id`, and how many of its siblings before it have both too.
fn place(document: &Document, id: ItemId) -> Option<(ItemKind, &str, usize)> {
    let item = document.node(id).and_then(Node::item)?;
    let siblings = match document.position(id)? {
        (Some(parent), _) => document.children(parent),
        (None, _) => document.top_level(),
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
