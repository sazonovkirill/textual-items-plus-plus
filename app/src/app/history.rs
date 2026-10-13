//! The undo and redo history: which committed transactions can be reverted and replayed, in what
//! order, and what was selected before and after each (FR-EDIT-06, N-UX-02).

use super::Selection;
use crate::domain::{Document, PlacementError, Transaction};

/// The steps to undo, latest last, and the undone ones to redo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
}

/// A transaction ready to apply, with the selection before it and the one to restore after it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Step {
    transaction: Transaction,
    before: Selection,
    after: Selection,
}

impl Step {
    /// The step that takes this one back.
    fn inverse(self) -> Self {
        Self {
            transaction: self.transaction.inverse(),
            before: self.after,
            after: self.before,
        }
    }
}

impl History {
    /// Applies `transaction` to `document` and makes it the next step to undo, dropping
    /// everything undone before it; `before` and `after` are the selections around the command.
    /// A transaction without operations changes nothing and is not recorded.
    ///
    /// # Errors
    ///
    /// The [`PlacementError`] of an operation that does not fit; then nothing changes.
    pub fn commit(
        &mut self,
        document: &mut Document,
        transaction: Transaction,
        before: Selection,
        after: Selection,
    ) -> Result<(), PlacementError> {
        if transaction.ops().is_empty() {
            return Ok(());
        }
        document.apply(&transaction)?;
        let step = Step {
            transaction,
            before,
            after,
        };
        self.undo.push(step.inverse());
        self.redo.clear();
        Ok(())
    }

    /// Reverts the latest step and returns the selection from before it, or `None` when there is
    /// nothing to undo.
    ///
    /// # Errors
    ///
    /// The [`PlacementError`] when the document no longer matches the step; then nothing
    /// changes.
    pub fn undo(&mut self, document: &mut Document) -> Result<Option<Selection>, PlacementError> {
        step(&mut self.undo, &mut self.redo, document)
    }

    /// Replays the latest undone step and returns the selection from after it, or `None` when
    /// there is nothing to redo.
    ///
    /// # Errors
    ///
    /// The [`PlacementError`] when the document no longer matches the step; then nothing
    /// changes.
    pub fn redo(&mut self, document: &mut Document) -> Result<Option<Selection>, PlacementError> {
        step(&mut self.redo, &mut self.undo, document)
    }
}

/// Applies the latest step of `from` and moves its inverse onto `to`.
fn step(
    from: &mut Vec<Step>,
    to: &mut Vec<Step>,
    document: &mut Document,
) -> Result<Option<Selection>, PlacementError> {
    let Some(step) = from.pop() else {
        return Ok(None);
    };
    if let Err(error) = document.apply(&step.transaction) {
        from.push(step);
        return Err(error);
    }
    let selection = step.after.clone();
    to.push(step.inverse());
    Ok(Some(selection))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Item, ItemId, ItemKind, Node, Op};

    fn title(document: &Document, id: ItemId) -> Option<&str> {
        Some(document.node(id)?.item()?.title.as_str())
    }

    fn focused(id: ItemId) -> Selection {
        let mut selection = Selection::default();
        selection.focus(Some(id));
        selection
    }

    /// A transaction renaming the item `id` to `to`.
    fn rename(document: &Document, id: ItemId, to: &str) -> Result<Transaction, PlacementError> {
        let before = document
            .node(id)
            .and_then(Node::item)
            .cloned()
            .ok_or(PlacementError::NoSuchNode(id))?;
        let after = Item {
            title: to.to_owned(),
            ..before.clone()
        };
        Ok(Transaction::new(vec![Op::Set { id, before, after }]))
    }

    /// A document with tasks `a` and `b` in the Inbox.
    fn document() -> Result<(Document, ItemId, ItemId), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let a = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "a")))?;
        let b = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "b")))?;
        Ok((document, a, b))
    }

    fn commit(
        history: &mut History,
        document: &mut Document,
        id: ItemId,
        to: &str,
    ) -> Result<(), PlacementError> {
        let transaction = rename(document, id, to)?;
        history.commit(document, transaction, Selection::default(), focused(id))
    }

    // FR-EDIT-06
    #[test]
    fn undo_and_redo_walk_the_committed_steps_and_restore_the_selection()
    -> Result<(), PlacementError> {
        let (mut document, a, b) = document()?;
        let mut history = History::default();
        commit(&mut history, &mut document, a, "A")?;
        commit(&mut history, &mut document, b, "B")?;
        assert_eq!(
            (title(&document, a), title(&document, b)),
            (Some("A"), Some("B"))
        );
        assert_eq!(history.undo(&mut document)?, Some(Selection::default()));
        assert_eq!(
            (title(&document, a), title(&document, b)),
            (Some("A"), Some("b"))
        );
        assert_eq!(history.undo(&mut document)?, Some(Selection::default()));
        assert_eq!(
            (title(&document, a), title(&document, b)),
            (Some("a"), Some("b"))
        );
        assert_eq!(history.undo(&mut document)?, None);
        assert_eq!(history.redo(&mut document)?, Some(focused(a)));
        assert_eq!(
            (title(&document, a), title(&document, b)),
            (Some("A"), Some("b"))
        );
        assert_eq!(history.redo(&mut document)?, Some(focused(b)));
        assert_eq!(
            (title(&document, a), title(&document, b)),
            (Some("A"), Some("B"))
        );
        assert_eq!(history.redo(&mut document)?, None);
        Ok(())
    }

    // FR-EDIT-06
    #[test]
    fn committing_drops_what_was_undone() -> Result<(), PlacementError> {
        let (mut document, a, _) = document()?;
        let mut history = History::default();
        commit(&mut history, &mut document, a, "b")?;
        history.undo(&mut document)?;
        commit(&mut history, &mut document, a, "c")?;
        assert_eq!(history.redo(&mut document)?, None);
        assert_eq!(title(&document, a), Some("c"));
        history.undo(&mut document)?;
        assert_eq!(title(&document, a), Some("a"));
        assert_eq!(history.undo(&mut document)?, None);
        Ok(())
    }

    #[test]
    fn empty_and_refused_commits_keep_the_history() -> Result<(), PlacementError> {
        let (mut document, a, _) = document()?;
        let mut history = History::default();
        commit(&mut history, &mut document, a, "b")?;
        history.undo(&mut document)?;
        let kept = history.clone();
        let empty = Transaction::default();
        assert_eq!(
            history.commit(&mut document, empty, Selection::default(), focused(a)),
            Ok(())
        );
        let stale = rename(&document, a, "c")?.inverse();
        assert_eq!(
            history.commit(&mut document, stale, Selection::default(), focused(a)),
            Err(PlacementError::Stale(a))
        );
        assert_eq!(history, kept);
        assert_eq!(title(&document, a), Some("a"));
        Ok(())
    }

    #[test]
    fn a_step_that_no_longer_fits_stays_to_try_again() -> Result<(), PlacementError> {
        let (mut document, a, _) = document()?;
        let mut history = History::default();
        commit(&mut history, &mut document, a, "A")?;
        let original = document.clone();
        document.apply(&rename(&document, a, "elsewhere")?)?;
        assert_eq!(history.undo(&mut document), Err(PlacementError::Stale(a)));
        assert_eq!(title(&document, a), Some("elsewhere"));
        document = original;
        assert_eq!(history.undo(&mut document)?, Some(Selection::default()));
        document.apply(&rename(&document, a, "elsewhere")?)?;
        assert_eq!(history.redo(&mut document), Err(PlacementError::Stale(a)));
        Ok(())
    }
}
