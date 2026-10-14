//! Moving a node with its subtree to another place, as one transaction that keeps its
//! identifiers (FR-OPS-02, FR-OPS-03, FR-ORG-06).

use super::{Document, PlacementError};
use crate::domain::{Op, Transaction, item::ItemId};

/// Where [`Document::move_to`] puts a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// Right before this node, among its siblings.
    Before(ItemId),
    /// Right after this node, among its siblings.
    After(ItemId),
    /// First among this node's children.
    First(ItemId),
    /// Last among this node's children, or last at the top level for `None`.
    Last(Option<ItemId>),
}

impl Document {
    /// The transaction that moves `id` with its subtree to `to`; an empty one when that is where
    /// it already is.
    ///
    /// # Errors
    ///
    /// [`PlacementError::NoSuchNode`] when `id` or the node `to` names is not in this document,
    /// and [`PlacementError::DoesNotFit`] when `id` is the Inbox, when the hierarchy does not
    /// allow it at `to`, or when `to` is inside it.
    pub fn move_to(&self, id: ItemId, to: Destination) -> Result<Transaction, PlacementError> {
        let node = self.node(id).ok_or(PlacementError::NoSuchNode(id))?;
        let (from, at) = self.position(id).ok_or(PlacementError::DoesNotFit)?;
        let (parent, index) = self.place(to)?;
        self.check_fit(parent, node)?;
        let mut above = parent;
        while let Some(ancestor) = above {
            if ancestor == id {
                return Err(PlacementError::DoesNotFit);
            }
            above = self.position(ancestor).and_then(|(parent, _)| parent);
        }
        if parent == from && (at..=at + 1).contains(&index) {
            return Ok(Transaction::default());
        }
        let subtree = self.snapshot(id).ok_or(PlacementError::NoSuchNode(id))?;
        let index = if parent == from && index > at {
            index - 1
        } else {
            index
        };
        Ok(Transaction::new(vec![
            Op::Remove {
                parent: from,
                index: at,
                subtree: subtree.clone(),
            },
            Op::Insert {
                parent,
                index,
                subtree,
            },
        ]))
    }

    /// The parent and the position among its children, before anything moves, that `to` names.
    fn place(&self, to: Destination) -> Result<(Option<ItemId>, usize), PlacementError> {
        match to {
            Destination::Before(sibling) | Destination::After(sibling) => {
                let (parent, index) = self
                    .position(sibling)
                    .ok_or(PlacementError::NoSuchNode(sibling))?;
                let after = usize::from(matches!(to, Destination::After(_)));
                Ok((parent, index + after))
            }
            Destination::First(parent) => {
                self.siblings(Some(parent))?;
                Ok((Some(parent), 0))
            }
            Destination::Last(parent) => Ok((parent, self.siblings(parent)?.len())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{Item, ItemKind, Node};
    use ItemKind::{ChecklistItem, Project, Section, Task};

    fn item(kind: ItemKind, title: &str) -> Node {
        Node::Item(Item::new(kind, title))
    }

    fn kept(text: &str) -> Node {
        Node::Kept(text.to_owned())
    }

    /// Applies the move and checks that undoing it restores the document.
    fn moved(document: &mut Document, id: ItemId, to: Destination) -> Result<(), PlacementError> {
        let original = document.clone();
        let transaction = document.move_to(id, to)?;
        document.apply(&transaction)?;
        let mut undone = document.clone();
        undone.apply(&transaction.inverse())?;
        assert_eq!(undone, original, "undo restores the document");
        Ok(())
    }

    /// A project holding tasks `a`, `b`, kept text and `c`, then a section holding
    /// task `d`.
    struct Fixture {
        document: Document,
        project: ItemId,
        a: ItemId,
        b: ItemId,
        text: ItemId,
        c: ItemId,
        section: ItemId,
        d: ItemId,
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let project = document.push(None, item(Project, "p"))?;
        let a = document.push(Some(project), item(Task, "a"))?;
        let b = document.push(Some(project), item(Task, "b"))?;
        let text = document.push(Some(project), kept("about b"))?;
        let c = document.push(Some(project), item(Task, "c"))?;
        let section = document.push(Some(project), item(Section, "s"))?;
        let d = document.push(Some(section), item(Task, "d"))?;
        Ok(Fixture {
            document,
            project,
            a,
            b,
            text,
            c,
            section,
            d,
        })
    }

    // FR-OPS-02, FR-TASK-05
    #[test]
    fn a_node_moves_before_or_after_a_sibling_keeping_its_identifier() -> Result<(), PlacementError>
    {
        let Fixture {
            mut document,
            project,
            a,
            b,
            text,
            c,
            section,
            ..
        } = fixture()?;
        moved(&mut document, a, Destination::After(c))?;
        assert_eq!(document.children(project), [b, text, c, a, section]);
        moved(&mut document, a, Destination::Before(b))?;
        assert_eq!(document.children(project), [a, b, text, c, section]);
        moved(&mut document, a, Destination::After(b))?;
        assert_eq!(document.children(project), [b, a, text, c, section]);
        Ok(())
    }

    // FR-OPS-10, FR-TASK-05
    #[test]
    fn a_node_moves_first_or_last_into_another_parent() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            project,
            a,
            c,
            section,
            d,
            ..
        } = fixture()?;
        moved(&mut document, c, Destination::First(section))?;
        assert_eq!(document.children(section), [c, d]);
        moved(&mut document, a, Destination::Last(Some(section)))?;
        assert_eq!(document.children(section), [c, d, a]);
        assert_eq!(document.position(a), Some((Some(section), 2)));
        let other = document.push(None, item(Project, "q"))?;
        moved(&mut document, other, Destination::Last(Some(project)))?;
        assert_eq!(document.children(project).last(), Some(&other));
        moved(&mut document, other, Destination::Last(None))?;
        assert_eq!(document.top_level(), [project, other]);
        Ok(())
    }

    // FR-OPS-02
    #[test]
    fn moving_a_node_to_where_it_is_changes_nothing() -> Result<(), PlacementError> {
        let Fixture {
            document,
            project,
            a,
            b,
            text,
            ..
        } = fixture()?;
        for (id, to) in [
            (a, Destination::Before(a)),
            (a, Destination::After(a)),
            (a, Destination::Before(b)),
            (b, Destination::After(b)),
            (b, Destination::Before(text)),
            (b, Destination::After(a)),
            (a, Destination::First(project)),
            (text, Destination::After(text)),
        ] {
            assert_eq!(
                document.move_to(id, to),
                Ok(Transaction::default()),
                "{to:?}"
            );
        }
        Ok(())
    }

    // FR-OPS-03, FR-ORG-02
    #[test]
    fn a_move_the_hierarchy_does_not_allow_is_refused() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            project,
            a,
            section,
            d,
            ..
        } = fixture()?;
        let top = document.push(None, item(Project, "x"))?;
        let check = document.push(Some(d), item(ChecklistItem, "c"))?;
        let missing = ItemId::new(99);
        let refused = [
            (a, Destination::Last(None), PlacementError::DoesNotFit),
            (section, Destination::Last(None), PlacementError::DoesNotFit),
            (
                top,
                Destination::Last(Some(section)),
                PlacementError::DoesNotFit,
            ),
            (
                check,
                Destination::Last(Some(section)),
                PlacementError::DoesNotFit,
            ),
            (
                project,
                Destination::First(section),
                PlacementError::DoesNotFit,
            ),
            (project, Destination::Before(d), PlacementError::DoesNotFit),
            (
                document.inbox(),
                Destination::Last(None),
                PlacementError::DoesNotFit,
            ),
            (
                missing,
                Destination::Last(None),
                PlacementError::NoSuchNode(missing),
            ),
            (
                a,
                Destination::After(missing),
                PlacementError::NoSuchNode(missing),
            ),
            (
                a,
                Destination::First(missing),
                PlacementError::NoSuchNode(missing),
            ),
        ];
        for (id, to, error) in refused {
            assert_eq!(document.move_to(id, to), Err(error), "{id:?} to {to:?}");
        }
        Ok(())
    }

    // FR-ORG-02
    #[test]
    fn a_node_cannot_move_inside_itself_or_kept_content() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let outer = document.push(None, item(Project, "outer"))?;
        let inner = document.push(Some(outer), item(Project, "inner"))?;
        let deepest = document.push(Some(inner), item(Project, "deepest"))?;
        let line = document.push(None, kept("text"))?;
        let held = document.push(Some(line), kept("held"))?;
        for (id, to) in [
            (outer, Destination::Last(Some(outer))),
            (outer, Destination::Last(Some(deepest))),
            (outer, Destination::First(line)),
            (outer, Destination::Before(held)),
        ] {
            assert_eq!(
                document.move_to(id, to),
                Err(PlacementError::DoesNotFit),
                "{to:?}"
            );
        }
        moved(&mut document, deepest, Destination::After(outer))?;
        assert_eq!(document.top_level(), [outer, deepest, line]);
        Ok(())
    }
}
