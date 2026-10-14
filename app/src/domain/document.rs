//! The document: every node in its place and order, the Inbox apart from the top-level list, with
//! the hierarchy of FR-ORG-01–04 kept on every change.

use std::collections::HashMap;

use thiserror::Error;

use super::item::{Item, ItemId, ItemKind, Node};

mod converting;
mod edit;
mod moving;

pub use edit::{Branch, Op, Snapshot, Transaction};
pub use moving::Destination;

/// The whole todo list: the Inbox, always first (FR-ORG-07), then the top-level nodes in the
/// user's order (FR-ORG-08), each with its children. Two documents are equal when they hold the
/// same nodes, whatever their revisions.
#[derive(Debug, Clone)]
pub struct Document {
    entries: HashMap<ItemId, Entry>,
    inbox: ItemId,
    top_level: Vec<ItemId>,
    next_number: u64,
    revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    node: Node,
    children: Vec<ItemId>,
}

/// Why a node could not be placed or changed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlacementError {
    /// The document has no node with this identifier.
    #[error("the document has no node {0:?}")]
    NoSuchNode(ItemId),
    /// The hierarchy does not allow the node there: a section at the top level, say, or a
    /// second Inbox ([`ItemKind::fits_in`]).
    #[error("an item of that kind cannot go there")]
    DoesNotFit,
    /// Changing an item cannot change its kind; converting is its own operation (FR-ORG-06).
    #[error("changing an item cannot change its kind")]
    KindChanged,
    /// The node is not as the edit expects: its item, place or subtree changed since the edit
    /// was made.
    #[error("node {0:?} has changed since the edit was made")]
    Stale(ItemId),
    /// A node to put in has an identifier already in use, or one this document never made.
    #[error("node {0:?} cannot be put in: its identifier is in use or unknown")]
    IdUnavailable(ItemId),
    /// The position is past the end of the parent's children, or of the top level for `None`.
    #[error("position {index} is past the children of {parent:?}")]
    Position {
        /// The parent, or `None` for the top level.
        parent: Option<ItemId>,
        /// The position.
        index: usize,
    },
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            entries,
            inbox,
            top_level,
            next_number,
            revision: _,
        } = self;
        *entries == other.entries
            && *inbox == other.inbox
            && *top_level == other.top_level
            && *next_number == other.next_number
    }
}

impl Eq for Document {}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// A document holding only an empty Inbox.
    #[must_use]
    pub fn new() -> Self {
        let inbox = ItemId::new(0);
        let entry = Entry {
            node: Node::Item(Item::new(ItemKind::Inbox, "Inbox")),
            children: Vec::new(),
        };
        Self {
            entries: HashMap::from([(inbox, entry)]),
            inbox,
            top_level: Vec::new(),
            next_number: 1,
            revision: 0,
        }
    }

    /// A number that changes with every change to the document, so that one revision of it can
    /// be told from another without comparing them; only revisions of one document compare.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The Inbox.
    #[must_use]
    pub fn inbox(&self) -> ItemId {
        self.inbox
    }

    /// The top-level nodes in order, without the Inbox.
    #[must_use]
    pub fn top_level(&self) -> &[ItemId] {
        &self.top_level
    }

    /// The node `id`, or `None` when this document has none.
    #[must_use]
    pub fn node(&self, id: ItemId) -> Option<&Node> {
        self.entries.get(&id).map(|entry| &entry.node)
    }

    /// The children of `id` in order; none when this document has no such node.
    #[must_use]
    pub fn children(&self, id: ItemId) -> &[ItemId] {
        self.entries.get(&id).map_or(&[], |entry| &entry.children)
    }

    /// Whether the contents of `id` are hidden everywhere because it, or an item it is inside,
    /// is completed (FR-ORG-09).
    #[must_use]
    pub fn conceals(&self, id: ItemId) -> bool {
        let mut at = id;
        loop {
            if self
                .node(at)
                .and_then(Node::item)
                .is_some_and(|item| item.completed.is_some())
            {
                return true;
            }
            match self.position(at) {
                Some((Some(parent), _)) => at = parent,
                _ => return false,
            }
        }
    }

    /// Adds `node` as the last child of `parent`, or last at the top level when `parent` is
    /// `None`. Kept content goes anywhere; an item only where its kind fits, and never inside
    /// kept content.
    ///
    /// # Errors
    ///
    /// [`PlacementError::NoSuchNode`] when `parent` is not in this document, and
    /// [`PlacementError::DoesNotFit`] when the hierarchy does not allow `node` there.
    pub fn push(&mut self, parent: Option<ItemId>, node: Node) -> Result<ItemId, PlacementError> {
        self.check_fit(parent, &node)?;
        let id = self.new_id();
        self.siblings_mut(parent)?.push(id);
        self.revision += 1;
        self.entries.insert(
            id,
            Entry {
                node,
                children: Vec::new(),
            },
        );
        Ok(id)
    }

    /// Replaces the item `id` with `item`, of the same kind, and returns what it was; an item
    /// replaced by its equal is no change.
    ///
    /// # Errors
    ///
    /// [`PlacementError::NoSuchNode`] when `id` is not an item of this document, and
    /// [`PlacementError::KindChanged`] when `item` is of another kind.
    pub fn set(&mut self, id: ItemId, item: Item) -> Result<Item, PlacementError> {
        let Some(Node::Item(old)) = self.entries.get_mut(&id).map(|entry| &mut entry.node) else {
            return Err(PlacementError::NoSuchNode(id));
        };
        if old.kind != item.kind {
            return Err(PlacementError::KindChanged);
        }
        if *old != item {
            self.revision += 1;
        }
        Ok(std::mem::replace(old, item))
    }

    /// Whether `node` may go in `parent`, or at the top level for `None`: kept content anywhere,
    /// an item only where its kind fits, and never inside kept content.
    fn check_fit(&self, parent: Option<ItemId>, node: &Node) -> Result<(), PlacementError> {
        let parent_node = parent
            .map(|id| self.node(id).ok_or(PlacementError::NoSuchNode(id)))
            .transpose()?;
        let fits = match (node, parent_node) {
            (Node::Kept(_), _) => true,
            (Node::Item(_), Some(Node::Kept(_))) => false,
            (Node::Item(item), parent) => item
                .kind
                .fits_in(parent.and_then(Node::item).map(|parent| parent.kind)),
        };
        if fits {
            Ok(())
        } else {
            Err(PlacementError::DoesNotFit)
        }
    }

    /// The children of `parent` in order, or the top-level nodes for `None`.
    fn siblings(&self, parent: Option<ItemId>) -> Result<&[ItemId], PlacementError> {
        match parent {
            Some(id) => self
                .entries
                .get(&id)
                .map(|entry| entry.children.as_slice())
                .ok_or(PlacementError::NoSuchNode(id)),
            None => Ok(&self.top_level),
        }
    }

    /// The children of `parent` to change, or the top-level nodes for `None`.
    fn siblings_mut(&mut self, parent: Option<ItemId>) -> Result<&mut Vec<ItemId>, PlacementError> {
        match parent {
            Some(id) => self
                .entries
                .get_mut(&id)
                .map(|entry| &mut entry.children)
                .ok_or(PlacementError::NoSuchNode(id)),
            None => Ok(&mut self.top_level),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Completion;
    use ItemKind::{ChecklistItem, Inbox, Project, Section, Task};

    fn item(kind: ItemKind) -> Node {
        Node::Item(Item::new(kind, "x"))
    }

    fn kept() -> Node {
        Node::Kept("text".to_owned())
    }

    // FR-STORE-09
    #[test]
    fn every_change_makes_a_new_revision_that_equality_ignores() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let start = document.revision();
        let task = document.push(Some(document.inbox()), item(Task))?;
        let pushed = document.revision();
        assert_ne!(pushed, start);
        let before = document.clone();
        document.set(task, Item::new(Task, "x"))?;
        assert_eq!(document.revision(), pushed, "no change");
        document.set(task, Item::new(Task, "y"))?;
        assert_ne!(document.revision(), pushed);
        document.set(task, Item::new(Task, "x"))?;
        assert_eq!(document, before, "the same nodes at another revision");
        assert_ne!(document.revision(), before.revision());
        Ok(())
    }

    // FR-ORG-07
    #[test]
    fn a_new_document_holds_only_an_empty_inbox() {
        let document = Document::new();
        let inbox = document.inbox();
        assert_eq!(
            document.node(inbox),
            Some(&Node::Item(Item::new(Inbox, "Inbox")))
        );
        assert_eq!(document.children(inbox), []);
        assert_eq!(document.top_level(), []);
        assert_eq!(Document::default(), document);
    }

    // FR-ORG-08
    #[test]
    fn pushed_nodes_keep_their_order_under_their_parent() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = document.push(None, item(Project))?;
        let project = document.push(None, item(Project))?;
        let first = document.push(Some(area), item(Task))?;
        let second = document.push(Some(area), kept())?;
        let in_inbox = document.push(Some(document.inbox()), item(Task))?;
        assert_eq!(document.top_level(), [area, project]);
        assert_eq!(document.children(area), [first, second]);
        assert_eq!(document.children(document.inbox()), [in_inbox]);
        assert_eq!(document.node(second), Some(&kept()));
        Ok(())
    }

    #[test]
    fn every_node_gets_a_new_identifier() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let ids = [
            document.push(None, kept())?,
            document.push(None, kept())?,
            document.push(Some(document.inbox()), kept())?,
        ];
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
        assert!(!ids.contains(&document.inbox()));
        Ok(())
    }

    // FR-ORG-02, FR-ORG-03, FR-ORG-04, FR-CHK-01
    #[test]
    fn items_go_only_where_their_kind_fits() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let project = document.push(None, item(Project))?;
        document.push(Some(project), item(Project))?;
        let section = document.push(Some(project), item(Section))?;
        let task = document.push(Some(section), item(Task))?;
        let check = document.push(Some(task), item(ChecklistItem))?;
        document.push(Some(check), item(ChecklistItem))?;
        let refused = [
            (None, Section),
            (None, Task),
            (Some(section), Project),
            (Some(document.inbox()), Project),
            (Some(task), Task),
            (Some(document.inbox()), Section),
            (None, Inbox),
        ];
        for (parent, kind) in refused {
            assert_eq!(
                document.push(parent, item(kind)),
                Err(PlacementError::DoesNotFit),
                "{kind:?} in {parent:?}"
            );
        }
        Ok(())
    }

    // FR-STORE-09
    #[test]
    fn kept_content_goes_anywhere_and_holds_only_kept_content() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let top = document.push(None, kept())?;
        let task = document.push(Some(document.inbox()), item(Task))?;
        document.push(Some(task), kept())?;
        document.push(Some(top), kept())?;
        assert_eq!(
            document.push(Some(top), item(Project)),
            Err(PlacementError::DoesNotFit)
        );
        Ok(())
    }

    #[test]
    fn refused_pushes_change_nothing() {
        let mut document = Document::new();
        let missing = ItemId::new(99);
        assert_eq!(
            document.push(Some(missing), kept()),
            Err(PlacementError::NoSuchNode(missing))
        );
        assert_eq!(
            document.push(None, item(Task)),
            Err(PlacementError::DoesNotFit)
        );
        assert_eq!(document, Document::new());
        assert_eq!(document.node(missing), None);
        assert_eq!(document.children(missing), []);
    }

    // FR-ORG-09
    #[test]
    fn a_completed_item_conceals_everything_inside_it() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let mut done = Item::new(Project, "done");
        done.completed = Some(Completion::Undated);
        let area = document.push(None, Node::Item(done))?;
        let line = document.push(Some(area), kept())?;
        let project = document.push(Some(area), item(Project))?;
        let task = document.push(Some(project), item(Task))?;
        let open = document.push(None, item(Project))?;
        let inside_open = document.push(Some(open), item(Task))?;
        for id in [area, line, project, task] {
            assert!(document.conceals(id), "{id:?}");
        }
        for id in [open, inside_open, document.inbox(), ItemId::new(99)] {
            assert!(!document.conceals(id), "{id:?}");
        }
        Ok(())
    }

    #[test]
    fn set_replaces_an_item_of_the_same_kind() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let mut renamed = Item::new(Inbox, "Inbox");
        renamed.description = "Everything new.".to_owned();
        assert_eq!(
            document.set(inbox, renamed.clone())?,
            Item::new(Inbox, "Inbox")
        );
        assert_eq!(document.node(inbox), Some(&Node::Item(renamed)));
        Ok(())
    }

    #[test]
    fn set_refuses_another_kind_and_kept_content() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let line = document.push(None, kept())?;
        let missing = ItemId::new(99);
        assert_eq!(
            document.set(inbox, Item::new(Project, "Inbox")),
            Err(PlacementError::KindChanged)
        );
        assert_eq!(
            document.set(line, Item::new(Project, "x")),
            Err(PlacementError::NoSuchNode(line))
        );
        assert_eq!(
            document.set(missing, Item::new(Project, "x")),
            Err(PlacementError::NoSuchNode(missing))
        );
        assert_eq!(document.node(line), Some(&kept()));
        Ok(())
    }

    #[test]
    fn errors_say_what_went_wrong() {
        assert_eq!(
            PlacementError::NoSuchNode(ItemId::new(3)).to_string(),
            "the document has no node ItemId(3)"
        );
        assert_eq!(
            PlacementError::DoesNotFit.to_string(),
            "an item of that kind cannot go there"
        );
        assert_eq!(
            PlacementError::KindChanged.to_string(),
            "changing an item cannot change its kind"
        );
        assert_eq!(
            PlacementError::Stale(ItemId::new(3)).to_string(),
            "node ItemId(3) has changed since the edit was made"
        );
        assert_eq!(
            PlacementError::IdUnavailable(ItemId::new(3)).to_string(),
            "node ItemId(3) cannot be put in: its identifier is in use or unknown"
        );
        assert_eq!(
            PlacementError::Position {
                parent: None,
                index: 2
            }
            .to_string(),
            "position 2 is past the children of None"
        );
    }
}
