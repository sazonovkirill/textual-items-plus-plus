//! Quick Comment: a one-line panel that adds the comment typed in it to the focused item, the
//! newest of its comments, without opening the attachments panel (FR-ATT-06).

use jiff::civil::DateTime;

use super::{Notice, State, TextEdit, TextField, view::Shown};
use crate::domain::{ItemId, ItemKind, Node, Op, Transaction, comments};

/// What the user does in the open panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickCommentInput {
    /// Changes the text typed.
    Edit(TextEdit),
    /// Adds the text typed, trimmed, as a comment stamped with this local time, unless it is
    /// empty, and closes the panel.
    Add(DateTime),
    /// Closes the panel, adding nothing.
    Cancel,
}

/// The open panel as a frontend shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickCommentView<'a> {
    /// The text typed.
    pub field: &'a TextField,
    /// The title of the item the comment goes to.
    pub title: &'a str,
}

/// The open panel: the item the comment goes to and the text typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QuickComment {
    id: ItemId,
    field: TextField,
}

impl QuickComment {
    /// The panel on the item `exact` finds for its item in a reloaded document, `None` when it
    /// finds none.
    pub(super) fn rematched(self, exact: impl Fn(ItemId) -> Option<ItemId>) -> Option<Self> {
        Some(Self {
            id: exact(self.id)?,
            ..self
        })
    }
}

impl State {
    /// Quick Comment, while it is open.
    #[must_use]
    pub fn quick_comment(&self) -> Option<QuickCommentView<'_>> {
        let quick_comment = self.quick_comment.as_ref()?;
        let item = self.document.node(quick_comment.id)?.item()?;
        Some(QuickCommentView {
            field: &quick_comment.field,
            title: &item.title,
        })
    }

    /// Opens Quick Comment on the focused item; with nothing focused nothing opens.
    pub(super) fn open_quick_comment(&mut self) {
        if let Some(id) = self.pane().selection.focused() {
            self.open_quick_comment_on(id);
        }
    }

    /// Opens Quick Comment on the sidebar's focused entry, unless it is a view, which takes no
    /// comments, as the status line says.
    pub(super) fn quick_comment_in_sidebar(&mut self, entry: Shown) {
        match entry {
            Shown::Container(id) => self.open_quick_comment_on(id),
            Shown::View(_) => self.notify(Notice::NoComments),
        }
    }

    /// Opens Quick Comment on item `id`, unless it is a checklist item, which has no comments.
    fn open_quick_comment_on(&mut self, id: ItemId) {
        let commented = self
            .document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| item.kind != ItemKind::ChecklistItem);
        if commented {
            self.quick_comment = Some(QuickComment {
                id,
                field: TextField::new(String::new()),
            });
        }
    }

    /// Applies `input` to the open panel: adding puts the comment in as one step to undo and
    /// closes the panel, saying so. Nothing happens while it is closed.
    pub(super) fn quick_comment_input(&mut self, input: QuickCommentInput) {
        let Some(quick_comment) = self.quick_comment.as_mut() else {
            return;
        };
        match input {
            QuickCommentInput::Edit(edit) => quick_comment.field = quick_comment.field.edited(edit),
            QuickCommentInput::Add(now) => {
                let id = quick_comment.id;
                let text = quick_comment.field.text().trim().to_owned();
                self.quick_comment = None;
                if !text.is_empty() {
                    self.add_comment(id, &text, now);
                }
            }
            QuickCommentInput::Cancel => self.quick_comment = None,
        }
    }

    /// Adds `text` as the newest comment of item `id`, stamped `now`, and says so.
    fn add_comment(&mut self, id: ItemId, text: &str, now: DateTime) {
        let before = self
            .document
            .node(id)
            .and_then(Node::item)
            .expect("a reload keeps Quick Comment on an item of the document");
        let mut after = before.clone();
        after
            .comments
            .insert(0, format!("{}{text}", comments::stamp(now)));
        self.change(Transaction::new(vec![Op::Set {
            id,
            before: before.clone(),
            after,
        }]));
        self.notify(Notice::Commented);
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::{
        app::{Command, Message, Opened, update, view::View},
        domain::{Document, Item, PlacementError},
    };

    /// A Wednesday.
    const TODAY: Date = date(2026, 10, 14);

    /// The app on a document whose Inbox holds task `Call`, focused, with one comment.
    fn state() -> Result<(State, ItemId), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let task = Item {
            comments: vec!["[2026-10-13 09:00] Left a message".to_owned()],
            ..Item::new(ItemKind::Task, "Call")
        };
        let id = document.push(inbox, Node::Item(task))?;
        Ok((State::new(document, Opened::Demo, TODAY), id))
    }

    fn input(state: &mut State, input: QuickCommentInput) {
        assert_eq!(update(state, Message::QuickComment(input), TODAY), []);
    }

    fn typed(state: &mut State, text: &str) {
        for c in text.chars() {
            input(state, QuickCommentInput::Edit(TextEdit::Insert(c)));
        }
    }

    fn comments(state: &State, id: ItemId) -> Vec<String> {
        state
            .document()
            .node(id)
            .and_then(Node::item)
            .map(|item| item.comments.clone())
            .unwrap_or_default()
    }

    // FR-ATT-06
    #[test]
    fn enter_adds_the_text_trimmed_as_the_newest_comment_in_one_step_to_undo()
    -> Result<(), PlacementError> {
        let (mut state, id) = state()?;
        update(&mut state, Message::Command(Command::QuickComment), TODAY);
        assert_eq!(
            state.quick_comment().map(|view| view.title),
            Some("Call"),
            "titled with the focused item"
        );
        typed(&mut state, "  Called back  ");
        input(&mut state, QuickCommentInput::Add(TODAY.at(17, 30, 59, 0)));
        assert!(state.quick_comment().is_none());
        assert_eq!(
            comments(&state, id),
            [
                "[2026-10-14 17:30] Called back",
                "[2026-10-13 09:00] Left a message"
            ]
        );
        assert_eq!(state.notice(), Some(&Notice::Commented));
        assert!(state.attachments().is_none(), "no panel opens");
        update(&mut state, Message::Command(Command::Undo), TODAY);
        assert_eq!(comments(&state, id), ["[2026-10-13 09:00] Left a message"]);
        Ok(())
    }

    // FR-ATT-06
    #[test]
    fn esc_or_nothing_typed_closes_the_panel_adding_nothing() -> Result<(), PlacementError> {
        let (mut state, id) = state()?;
        for close in [
            QuickCommentInput::Cancel,
            QuickCommentInput::Add(TODAY.at(9, 0, 0, 0)),
        ] {
            state.open_quick_comment();
            typed(
                &mut state,
                if close == QuickCommentInput::Cancel {
                    "Hi"
                } else {
                    "   "
                },
            );
            input(&mut state, close);
            assert!(state.quick_comment().is_none(), "{close:?}");
            assert_eq!(comments(&state, id).len(), 1, "{close:?}");
        }
        assert_eq!(state.notice(), None);
        let before = state.clone();
        input(&mut state, QuickCommentInput::Add(TODAY.at(9, 0, 0, 0)));
        assert_eq!(state, before, "nothing happens while it is closed");
        Ok(())
    }

    // FR-ATT-06
    #[test]
    fn the_sidebar_comments_on_its_container_and_says_a_view_takes_none()
    -> Result<(), PlacementError> {
        let (mut state, _) = state()?;
        state.toggle_sidebar();
        assert!(state.sidebar_command(Command::QuickComment, TODAY));
        assert_eq!(state.quick_comment().map(|view| view.title), Some("Inbox"));
        state.quick_comment = None;
        state.quick_comment_in_sidebar(Shown::View(View::Today));
        assert!(state.quick_comment().is_none());
        assert_eq!(state.notice(), Some(&Notice::NoComments));
        Ok(())
    }

    // FR-ATT-06
    #[test]
    fn nothing_opens_without_a_focus_or_on_a_checklist_item() -> Result<(), PlacementError> {
        let mut empty = State::new(Document::new(), Opened::Demo, TODAY);
        empty.open_quick_comment();
        assert!(empty.quick_comment().is_none());
        let (mut state, id) = state()?;
        let dial = Node::Item(Item::new(ItemKind::ChecklistItem, "Dial"));
        let item = state.document.push(Some(id), dial)?;
        state.open_quick_comment_on(item);
        assert!(state.quick_comment().is_none());
        Ok(())
    }

    // FR-ATT-06
    #[test]
    fn a_reload_keeps_the_item_found_again_or_closes_the_panel() -> Result<(), PlacementError> {
        let (mut state, id) = state()?;
        state.open_quick_comment();
        let quick_comment = state
            .quick_comment
            .take()
            .ok_or(PlacementError::DoesNotFit)?;
        let other = state.document().inbox();
        let kept = quick_comment.clone().rematched(|_| Some(other));
        assert_eq!(kept.map(|kept| kept.id), Some(other));
        assert_eq!(quick_comment.rematched(|_| None), None);
        assert_ne!(id, other);
        Ok(())
    }
}
