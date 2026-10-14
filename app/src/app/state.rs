//! The application state a frontend renders, changed only by [`update`](super::update).

use std::path::PathBuf;

use jiff::civil::Date;

use super::{
    History, Selection, completion,
    container::{self, Row},
    focus::{self, Focus},
};
use crate::domain::{Document, ItemId};

/// Where the document came from, which decides whether changes to it can be kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opened {
    /// The built-in demo document, kept in memory and never saved (FR-STORE-01, D-29).
    Demo,
    /// The file at this path, as the user named it.
    File(PathBuf),
}

/// Everything the app knows while it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    document: Document,
    opened: Opened,
    container: ItemId,
    selection: Selection,
    history: History,
    /// Whether the pane shows completed items (FR-VIEW-11); per pane once there are several.
    show_completed: bool,
}

impl State {
    /// The app showing `document`, opened from `opened`, at home: the Inbox, its first row
    /// focused (FR-NAV-02).
    #[must_use]
    pub fn new(document: Document, opened: Opened) -> Self {
        let mut state = Self {
            container: document.inbox(),
            document,
            opened,
            selection: Selection::default(),
            history: History::default(),
            show_completed: false,
        };
        state.move_focus(Focus::First);
        state
    }

    /// The document.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Where the document came from.
    #[must_use]
    pub fn opened(&self) -> &Opened {
        &self.opened
    }

    /// The container shown.
    #[must_use]
    pub fn container(&self) -> ItemId {
        self.container
    }

    /// The rows the shown container lists, in order.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        container::rows(&self.document, self.container, self.show_completed)
    }

    /// Whether completed items are shown.
    #[must_use]
    pub fn shows_completed(&self) -> bool {
        self.show_completed
    }

    /// The items the next command acts on.
    #[must_use]
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Moves the focus as `motion` says, selecting only the newly focused row.
    pub(super) fn move_focus(&mut self, motion: Focus) {
        let target = focus::target(&self.rows(), self.selection.focused(), motion);
        self.selection.focus(target);
    }

    /// Completes the selected items on `today`, or uncompletes them when all are completed, as
    /// one step to undo, moving the focus on when its row hides.
    pub(super) fn toggle_completed(&mut self, today: Date) {
        let transaction = completion::toggle(&self.document, self.selection.items(), today);
        let before = self.rows();
        let (container, show_completed) = (self.container, self.show_completed);
        let focused = self.selection.focused();
        let mut after = self.selection.clone();
        self.history
            .commit(
                &mut self.document,
                transaction,
                self.selection.clone(),
                |document| {
                    let rows = container::rows(document, container, show_completed);
                    after.focus(focus::kept(&before, &rows, focused));
                    after
                },
            )
            .expect("a transaction made from the document as it is fits it");
        self.settle(&before);
    }

    /// Shows completed items, or hides them again, moving the focus on when its row hides.
    pub(super) fn toggle_show_completed(&mut self) {
        let before = self.rows();
        self.show_completed = !self.show_completed;
        self.settle(&before);
    }

    /// Takes back the latest change and restores the selection from before it.
    pub(super) fn undo(&mut self) {
        let before = self.rows();
        let restored = self
            .history
            .undo(&mut self.document)
            .expect("every change goes through the history, so its steps fit the document");
        self.restore(restored, &before);
    }

    /// Makes the latest change taken back again and restores the selection from after it.
    pub(super) fn redo(&mut self) {
        let before = self.rows();
        let restored = self
            .history
            .redo(&mut self.document)
            .expect("every change goes through the history, so its steps fit the document");
        self.restore(restored, &before);
    }

    /// Selects `selection`, if there is one, keeping the focus on a row shown.
    fn restore(&mut self, selection: Option<Selection>, before: &[Row]) {
        if let Some(selection) = selection {
            self.selection = selection;
            self.settle(before);
        }
    }

    /// Keeps the focus on a row now that the rows `before` have changed.
    fn settle(&mut self, before: &[Row]) {
        let focused = focus::kept(before, &self.rows(), self.selection.focused());
        if focused != self.selection.focused() {
            self.selection.focus(focused);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Item, ItemKind, Node, PlacementError};

    // FR-NAV-02, FR-STORE-01
    #[test]
    fn a_new_state_shows_the_inbox_with_its_first_task_focused() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let first = document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, "a")))?;
        document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, "b")))?;
        let state = State::new(document.clone(), Opened::Demo);
        assert_eq!(state.container(), inbox);
        assert_eq!(state.selection().focused(), Some(first));
        assert_eq!(state.rows(), container::rows(&document, inbox, false));
        assert_eq!(state.document(), &document);
        assert_eq!(state.opened(), &Opened::Demo);
        Ok(())
    }

    #[test]
    fn an_empty_inbox_focuses_nothing() {
        let state = State::new(
            Document::new(),
            Opened::File(PathBuf::from("todo.taskpaper")),
        );
        assert_eq!(state.selection(), &Selection::default());
        assert_eq!(
            state.opened(),
            &Opened::File(PathBuf::from("todo.taskpaper"))
        );
    }

    // FR-NAV-01
    #[test]
    fn moving_the_focus_selects_the_row_it_lands_on() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, "a")))?;
        let last = document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, "b")))?;
        let mut state = State::new(document, Opened::Demo);
        state.move_focus(Focus::Last);
        assert_eq!(state.selection().items().collect::<Vec<_>>(), [last]);
        Ok(())
    }
}
