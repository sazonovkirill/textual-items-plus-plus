//! The attachments panel: an item's note and comments, read and edited in a modal panel that
//! takes the keys while it is open, as taskpaper-terminal's attachments panel is
//! ([attachments.md §3–5](../../../specs/reference/taskpaper-terminal/specs/attachments.md#3-attachments-panel-fr-att-03),
//! FR-ATT-01–05). One of them is selected, which the keys act on. The note and a comment are
//! edited as an item's description and title are, so a session is one step to undo and is saved
//! like any edit.

use jiff::civil::DateTime;

use super::{Focus, Part, State, TextField, Whose, editing::Editing};
use crate::domain::{Document, Item, ItemId, ItemKind, Node, Op, Transaction, comments};

/// What the user does in the open panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentsInput {
    /// Edit what is selected: the note as raw text (FR-ATT-03) or a comment's text (FR-ATT-04).
    Edit,
    /// Select another of the note and the comments, in the order they show: the next, the one
    /// before, the first or the last; other motions select nothing else.
    Select(Focus),
    /// Add a comment stamped with this local time, the newest, and edit its text (FR-ATT-04).
    AddComment(DateTime),
    /// Delete the selected comment (FR-ATT-04).
    Delete,
    /// Close the panel, keeping the note being edited.
    Close,
}

/// What is selected in the open panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selected {
    /// The note.
    Note,
    /// The comment at this index, newest first.
    Comment(usize),
}

/// The open panel: the item it is on and what is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Panel {
    id: ItemId,
    selected: Selected,
}

/// The open panel as a frontend shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentsView<'a> {
    /// The item the panel is on.
    pub id: ItemId,
    /// That item, whose note or comment shows as it is typed while it is edited.
    pub item: &'a Item,
    /// What is selected, one of what the item has.
    pub selected: Selected,
    /// The text of what is selected as it is edited, while it is.
    pub editing: Option<&'a TextField>,
}

impl State {
    /// The attachments panel, while it is open (FR-ATT-01).
    #[must_use]
    pub fn attachments(&self) -> Option<AttachmentsView<'_>> {
        let Panel { id, selected } = self.panel?;
        let item = self.document.node(id).and_then(Node::item)?;
        let editing = self
            .editing()
            .filter(|&(edited, part, _)| {
                edited == id && matches!(part, Part::Note | Part::Comment(_))
            })
            .map(|(_, _, field)| field);
        Some(AttachmentsView {
            id,
            item,
            selected: kept(item, selected),
            editing,
        })
    }

    /// Opens the panel on the focused item or on the container shown, as `whose` says, with
    /// `selected` selected as far as it has it. Nothing opens with nothing focused.
    pub(super) fn open_attachments(&mut self, whose: Whose, selected: Selected) {
        let id = match whose {
            Whose::Focused => self.pane().selection.focused(),
            Whose::Shown => self.container(),
        };
        if let Some(id) = id {
            self.open_panel(id, selected);
        }
    }

    /// Opens the panel on item `id` with `selected` selected as far as it has it, unless it is a
    /// checklist item, which has no attachments (FR-ATT-01, FR-ATT-02).
    pub(super) fn open_panel(&mut self, id: ItemId, selected: Selected) {
        let attached = self
            .document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| item.kind != ItemKind::ChecklistItem);
        if attached {
            self.panel = Some(Panel { id, selected });
        }
    }

    /// Applies `input` to the open panel. Nothing happens while it is closed, and only closing
    /// while something in it is edited.
    pub(super) fn attachments_input(&mut self, input: AttachmentsInput) {
        let Some(view) = self.attachments() else {
            return;
        };
        let (id, selected) = (view.id, view.selected);
        let count = view.item.comments.len();
        let editing = self.editing().is_some();
        let before = self.pane().selection.clone();
        match input {
            AttachmentsInput::Close => {
                self.finish_editing();
                self.panel = None;
            }
            _ if editing => {}
            AttachmentsInput::Edit => {
                let editing = match selected {
                    Selected::Note => Editing::note(&self.document, id, before),
                    Selected::Comment(index) => Editing::comment(&self.document, id, index, before),
                };
                self.start_editing(editing);
            }
            AttachmentsInput::Select(motion) => {
                let at = match selected {
                    Selected::Note => 0,
                    Selected::Comment(index) => index + 1,
                };
                let at = match motion {
                    Focus::Next => (at + 1).min(count),
                    Focus::Previous => at.saturating_sub(1),
                    Focus::First => 0,
                    Focus::Last => count,
                    Focus::Parent | Focus::NextSection | Focus::PreviousSection => at,
                };
                let selected = at.checked_sub(1).map_or(Selected::Note, Selected::Comment);
                self.panel = Some(Panel { id, selected });
            }
            AttachmentsInput::AddComment(now) => {
                let editing =
                    Editing::new_comment(&mut self.document, id, comments::stamp(now), before);
                self.start_editing(editing);
                self.panel = Some(Panel {
                    id,
                    selected: Selected::Comment(0),
                });
            }
            AttachmentsInput::Delete => {
                if let Selected::Comment(index) = selected {
                    let transaction = without_comment(&self.document, id, index);
                    self.change(transaction);
                }
            }
        }
    }

    /// The panel once the document read again as `new` replaced `old`: on its item while
    /// `exact` finds it again, with the comment selected found again as the same line among the
    /// comments read, or at the same place when it is not (FR-ATT-01, FR-ATT-04).
    pub(super) fn rematched_panel(
        &self,
        old: &Document,
        exact: impl Fn(ItemId) -> Option<ItemId>,
    ) -> Option<Panel> {
        let Panel { id, selected } = self.panel?;
        let found = exact(id)?;
        let selected = match selected {
            Selected::Comment(index) => {
                let line = old
                    .node(id)
                    .and_then(Node::item)
                    .and_then(|item| item.comments.get(index));
                let comments = self
                    .document
                    .node(found)
                    .and_then(Node::item)
                    .map(|item| item.comments.as_slice())
                    .unwrap_or_default();
                let again = comments.iter().position(|comment| Some(comment) == line);
                Selected::Comment(again.unwrap_or(index))
            }
            Selected::Note => Selected::Note,
        };
        Some(Panel {
            id: found,
            selected,
        })
    }
}

/// `selected` as far as `item` has it: a comment past the last is the last, and with no
/// comments the note.
fn kept(item: &Item, selected: Selected) -> Selected {
    match selected {
        Selected::Comment(index) if index >= item.comments.len() => item
            .comments
            .len()
            .checked_sub(1)
            .map_or(Selected::Note, Selected::Comment),
        selected => selected,
    }
}

/// The transaction that deletes `id`'s comment at `index`.
fn without_comment(document: &Document, id: ItemId, index: usize) -> Transaction {
    let item = document
        .node(id)
        .and_then(Node::item)
        .expect("the panel is on an item of the document");
    let mut after = item.clone();
    after.comments.remove(index);
    Transaction::new(vec![Op::Set {
        id,
        before: item.clone(),
        after,
    }])
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::app::{Command, EditInput, Motion, Opened, TextEdit};
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
        state.open_attachments(Whose::Focused, Selected::Note);
        let view = state.attachments().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!((view.id, view.editing), (a, None));
        state.attachments_input(AttachmentsInput::Edit);
        type_text(&mut state, "# Plan\n- one\n\n");
        let view = state.attachments().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(view.item.note, "# Plan\n- one\n\n", "it shows as typed");
        assert_eq!(view.editing.map(TextField::text), Some("# Plan\n- one\n\n"));
        assert_eq!(
            state.editing().map(|(id, part, _)| (id, part)),
            Some((a, Part::Note))
        );
        state.attachments_input(AttachmentsInput::Edit);
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
        state.open_attachments(Whose::Shown, Selected::Note);
        assert_eq!(state.attachments().map(|view| view.id), Some(inbox));
        state.attachments_input(AttachmentsInput::Close);
        state.open_panel(c, Selected::Note);
        assert!(
            state.attachments().is_none(),
            "a checklist item has no attachments"
        );
        state.open_panel(a, Selected::Note);
        assert_eq!(state.attachments().map(|view| view.id), Some(a));
        state.attachments_input(AttachmentsInput::Close);
        state.toggle_sidebar();
        assert!(state.sidebar_command(Command::OpenNote(Whose::Focused), TODAY));
        assert_eq!(
            state.attachments().map(|view| view.id),
            Some(inbox),
            "the sidebar opens it on its focused entry"
        );
        state.attachments_input(AttachmentsInput::Close);
        assert!(state.sidebar_command(Command::OpenComments(Whose::Focused), TODAY));
        assert_eq!(state.attachments().map(|view| view.id), Some(inbox));
        let mut empty = State::new(Document::new(), Opened::Demo, TODAY);
        empty.open_attachments(Whose::Focused, Selected::Note);
        assert!(empty.attachments().is_none(), "nothing is focused");
        empty.attachments_input(AttachmentsInput::Edit);
        assert!(empty.editing().is_none());
        Ok(())
    }

    fn comments(state: &State, id: ItemId) -> Vec<String> {
        state
            .document()
            .node(id)
            .and_then(Node::item)
            .map(|item| item.comments.clone())
            .unwrap_or_default()
    }

    fn selected(state: &State) -> Option<Selected> {
        state.attachments().map(|view| view.selected)
    }

    // FR-ATT-02, FR-ATT-04, FR-ATT-05
    #[test]
    fn comments_are_added_newest_first_edited_and_deleted_each_one_step_to_undo()
    -> Result<(), PlacementError> {
        let (mut state, a, _) = state()?;
        state.open_attachments(Whose::Focused, Selected::Comment(0));
        assert_eq!(selected(&state), Some(Selected::Note), "there is none yet");
        state.attachments_input(AttachmentsInput::AddComment(TODAY.at(9, 5, 0, 0)));
        assert_eq!(selected(&state), Some(Selected::Comment(0)));
        type_text(&mut state, " Called ");
        assert_eq!(comments(&state, a), ["[2026-10-14 09:05]  Called "]);
        let view = state.attachments().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(view.editing.map(TextField::text), Some(" Called "));
        state.attachments_input(AttachmentsInput::Delete);
        state.edit_input(EditInput::Done);
        assert_eq!(
            comments(&state, a),
            ["[2026-10-14 09:05] Called"],
            "trimmed"
        );
        state.attachments_input(AttachmentsInput::AddComment(TODAY.at(17, 30, 0, 0)));
        type_text(&mut state, "Again");
        state.edit_input(EditInput::Done);
        assert_eq!(
            comments(&state, a),
            ["[2026-10-14 17:30] Again", "[2026-10-14 09:05] Called"]
        );
        state.attachments_input(AttachmentsInput::Select(Focus::Next));
        state.attachments_input(AttachmentsInput::Edit);
        state.edit_input(EditInput::Text(TextEdit::Insert('!')));
        state.edit_input(EditInput::Done);
        assert_eq!(
            comments(&state, a),
            ["[2026-10-14 17:30] Again", "[2026-10-14 09:05] Called!"],
            "the timestamp is kept"
        );
        state.attachments_input(AttachmentsInput::Delete);
        assert_eq!(comments(&state, a), ["[2026-10-14 17:30] Again"]);
        assert_eq!(
            selected(&state),
            Some(Selected::Comment(0)),
            "the last left"
        );
        state.attachments_input(AttachmentsInput::Close);
        let before_each = [
            vec!["[2026-10-14 17:30] Again", "[2026-10-14 09:05] Called!"],
            vec!["[2026-10-14 17:30] Again", "[2026-10-14 09:05] Called"],
            vec!["[2026-10-14 09:05] Called"],
            vec![],
        ];
        for before in before_each {
            state.undo();
            assert_eq!(comments(&state, a), before);
        }
        Ok(())
    }

    // FR-ATT-04
    #[test]
    fn a_comment_left_empty_or_cancelled_changes_nothing() -> Result<(), PlacementError> {
        let (mut state, a, _) = state()?;
        state.open_attachments(Whose::Focused, Selected::Note);
        state.attachments_input(AttachmentsInput::AddComment(TODAY.at(9, 5, 0, 0)));
        type_text(&mut state, "  ");
        state.edit_input(EditInput::Done);
        assert!(comments(&state, a).is_empty(), "only spaces add nothing");
        state.attachments_input(AttachmentsInput::AddComment(TODAY.at(9, 5, 0, 0)));
        type_text(&mut state, "typed");
        state.edit_input(EditInput::Cancel);
        assert!(comments(&state, a).is_empty(), "cancelled");
        assert!(state.editing().is_none());
        state.attachments_input(AttachmentsInput::AddComment(TODAY.at(9, 5, 0, 0)));
        type_text(&mut state, "kept");
        state.edit_input(EditInput::Done);
        for edits in [
            vec![EditInput::Text(TextEdit::Insert('x')), EditInput::Cancel],
            vec![
                EditInput::Text(TextEdit::Move {
                    motion: Motion::Start,
                    select: true,
                }),
                EditInput::Text(TextEdit::DeleteForward),
                EditInput::Done,
            ],
        ] {
            state.attachments_input(AttachmentsInput::Edit);
            for edit in edits {
                state.edit_input(edit);
            }
            assert_eq!(comments(&state, a), ["[2026-10-14 09:05] kept"]);
        }
        state.attachments_input(AttachmentsInput::Close);
        state.undo();
        assert!(
            comments(&state, a).is_empty(),
            "only the comment kept was a step"
        );
        Ok(())
    }

    // FR-ATT-01, FR-ATT-04
    #[test]
    fn selection_goes_from_the_note_through_the_comments_newest_first() -> Result<(), PlacementError>
    {
        let (mut state, a, _) = state()?;
        let mut document = state.document().clone();
        let item = document.node(a).and_then(Node::item).cloned();
        let mut item = item.ok_or(PlacementError::DoesNotFit)?;
        item.comments = vec!["new".to_owned(), "middle".to_owned(), "old".to_owned()];
        document.set(a, item)?;
        state = State::new(document, Opened::Demo, TODAY);
        state.open_attachments(Whose::Focused, Selected::Comment(0));
        assert_eq!(selected(&state), Some(Selected::Comment(0)));
        for (motion, expected) in [
            (Focus::Previous, Selected::Note),
            (Focus::Previous, Selected::Note),
            (Focus::Next, Selected::Comment(0)),
            (Focus::Last, Selected::Comment(2)),
            (Focus::Next, Selected::Comment(2)),
            (Focus::Parent, Selected::Comment(2)),
            (Focus::First, Selected::Note),
        ] {
            state.attachments_input(AttachmentsInput::Select(motion));
            assert_eq!(selected(&state), Some(expected), "{motion:?}");
        }
        state.attachments_input(AttachmentsInput::Delete);
        assert_eq!(comments(&state, a).len(), 3, "the note is not deleted");
        state.attachments_input(AttachmentsInput::Edit);
        state.attachments_input(AttachmentsInput::Select(Focus::Last));
        state.attachments_input(AttachmentsInput::Delete);
        state.attachments_input(AttachmentsInput::AddComment(TODAY.at(9, 5, 0, 0)));
        assert_eq!(
            (selected(&state), comments(&state, a).len()),
            (Some(Selected::Note), 3),
            "nothing but closing while the note is edited"
        );
        Ok(())
    }
}
