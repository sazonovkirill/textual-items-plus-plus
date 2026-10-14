//! The attachments panel: an item's note, read and edited in a modal panel that takes the keys
//! while it is open, as taskpaper-terminal's attachments panel is
//! ([attachments.md §3, §4](../../../specs/reference/taskpaper-terminal/specs/attachments.md#3-attachments-panel-fr-att-03),
//! FR-ATT-01–03, FR-ATT-05). The note is edited as an item's description is, so a session is one
//! step to undo and is saved like any edit.

use super::{Part, State, TextField, Whose, editing::Editing};
use crate::domain::{Item, ItemId, ItemKind, Node};

/// What the user does in the open panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentsInput {
    /// Edit the note as raw text (FR-ATT-03).
    EditNote,
    /// Close the panel, keeping the note being edited.
    Close,
}

/// The open panel as a frontend shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentsView<'a> {
    /// The item the panel is on.
    pub id: ItemId,
    /// That item, whose note shows as it is typed while it is edited.
    pub item: &'a Item,
    /// The note's text as it is edited, while it is.
    pub editing: Option<&'a TextField>,
}

impl State {
    /// The attachments panel, while it is open (FR-ATT-01).
    #[must_use]
    pub fn attachments(&self) -> Option<AttachmentsView<'_>> {
        let id = self.panel?;
        let item = self.document.node(id).and_then(Node::item)?;
        let editing = self
            .editing()
            .filter(|&(edited, part, _)| edited == id && part == Part::Note)
            .map(|(_, _, field)| field);
        Some(AttachmentsView { id, item, editing })
    }

    /// Opens the panel on the note of the focused item or of the container shown, as `whose`
    /// says. Nothing opens with nothing focused.
    pub(super) fn open_note(&mut self, whose: Whose) {
        let id = match whose {
            Whose::Focused => self.selection.focused(),
            Whose::Shown => self.container(),
        };
        if let Some(id) = id {
            self.open_panel(id);
        }
    }

    /// Opens the panel on item `id`, unless it is a checklist item, which has no attachments
    /// (FR-ATT-01).
    pub(super) fn open_panel(&mut self, id: ItemId) {
        let attached = self
            .document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| item.kind != ItemKind::ChecklistItem);
        if attached {
            self.panel = Some(id);
        }
    }

    /// Applies `input` to the open panel. Nothing happens while it is closed.
    pub(super) fn attachments_input(&mut self, input: AttachmentsInput) {
        let Some(id) = self.panel else {
            return;
        };
        match input {
            AttachmentsInput::EditNote => {
                if self.editing().is_none() {
                    let editing = Editing::note(&self.document, id, self.selection.clone());
                    self.start_editing(editing);
                }
            }
            AttachmentsInput::Close => {
                self.finish_editing();
                self.panel = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::app::{Command, EditInput, Opened, TextEdit};
    use crate::domain::{Document, PlacementError};

    const TODAY: Date = date(2026, 10, 14);

    /// A state on a file whose Inbox holds task `a` with checklist item `c`, focused on `a`.
    fn state() -> Result<(State, ItemId, ItemId), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let a = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "a")))?;
        let c = document.push(Some(a), Node::Item(Item::new(ItemKind::ChecklistItem, "c")))?;
        let state = State::new(document, Opened::File("a.taskpaper".into()), TODAY);
        Ok((state, a, c))
    }

    fn note(state: &State, id: ItemId) -> Option<&str> {
        Some(state.document().node(id)?.item()?.note.as_str())
    }

    fn type_text(state: &mut State, text: &str) {
        for c in text.chars() {
            state.edit_input(EditInput::Text(TextEdit::Insert(c)));
        }
    }

    // FR-ATT-01, FR-ATT-03, FR-ATT-05, FR-TASK-04
    #[test]
    fn a_note_is_edited_in_the_open_panel_as_one_step_to_undo() -> Result<(), PlacementError> {
        let (mut state, a, _) = state()?;
        state.open_note(Whose::Focused);
        let view = state.attachments().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!((view.id, view.editing), (a, None));
        state.attachments_input(AttachmentsInput::EditNote);
        type_text(&mut state, "# Plan\n- one\n\n");
        let view = state.attachments().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(view.item.note, "# Plan\n- one\n\n", "it shows as typed");
        assert_eq!(view.editing.map(TextField::text), Some("# Plan\n- one\n\n"));
        assert_eq!(
            state.editing().map(|(id, part, _)| (id, part)),
            Some((a, Part::Note))
        );
        state.attachments_input(AttachmentsInput::EditNote);
        assert_eq!(
            note(&state, a),
            Some("# Plan\n- one\n\n"),
            "editing goes on"
        );
        state.attachments_input(AttachmentsInput::Close);
        assert!(state.attachments().is_none());
        assert_eq!(
            note(&state, a),
            Some("# Plan\n- one"),
            "empty lines at the end go"
        );
        state.undo();
        assert_eq!(note(&state, a), Some(""));
        state.redo();
        assert_eq!(note(&state, a), Some("# Plan\n- one"));
        Ok(())
    }

    // FR-ATT-01, FR-ATT-02
    #[test]
    fn the_panel_opens_on_the_focused_item_or_the_container_shown() -> Result<(), PlacementError> {
        let (mut state, a, c) = state()?;
        let inbox = state.document().inbox();
        state.open_note(Whose::Shown);
        assert_eq!(state.attachments().map(|view| view.id), Some(inbox));
        state.attachments_input(AttachmentsInput::Close);
        state.open_panel(c);
        assert!(
            state.attachments().is_none(),
            "a checklist item has no attachments"
        );
        state.open_panel(a);
        assert_eq!(state.attachments().map(|view| view.id), Some(a));
        state.attachments_input(AttachmentsInput::Close);
        state.toggle_sidebar();
        assert!(state.sidebar_command(Command::OpenNote(Whose::Focused), TODAY));
        assert_eq!(
            state.attachments().map(|view| view.id),
            Some(inbox),
            "the sidebar opens it on its focused entry"
        );
        let mut empty = State::new(Document::new(), Opened::Demo, TODAY);
        empty.open_note(Whose::Focused);
        assert!(empty.attachments().is_none(), "nothing is focused");
        empty.attachments_input(AttachmentsInput::EditNote);
        assert!(empty.editing().is_none());
        Ok(())
    }
}
