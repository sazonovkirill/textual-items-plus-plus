//! Editing a task's title in place, an existing task's or one just added (FR-EDIT-01, FR-EDIT-02,
//! FR-EDIT-05). The title is written to the document as it is typed, so the task is drawn like
//! any other, and the whole session becomes one change to undo when it ends.

use super::{Selection, TextEdit, TextField};
use crate::domain::{Document, Item, ItemId, ItemKind, Node, Op, Snapshot, Transaction};

/// What the user does while a title is edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditInput {
    /// Changes the text being edited.
    Text(TextEdit),
    /// Ends editing and adds the next task below, unless the task edited was new and is left
    /// empty, which only ends editing.
    Next,
    /// Ends editing.
    Done,
}

/// A task whose title is being edited, the text edited, the selection before editing started and
/// how the task was before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Editing {
    id: ItemId,
    field: TextField,
    before: Selection,
    origin: Origin,
}

/// How the edited task was before editing started.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Origin {
    /// It was there, as this item.
    Existing(Item),
    /// It was added for editing, at this index among this parent's children.
    New { parent: ItemId, index: usize },
}

impl Editing {
    /// Starts editing the title of `id`, the caret at its end, when it is a task; `before` is
    /// the selection to restore when the session is undone.
    pub(super) fn existing(document: &Document, id: ItemId, before: Selection) -> Option<Self> {
        let item = document
            .node(id)
            .and_then(Node::item)
            .filter(|item| item.kind == ItemKind::Task)?;
        Some(Self {
            id,
            field: TextField::new(item.title.clone()),
            before,
            origin: Origin::Existing(item.clone()),
        })
    }

    /// Adds an empty task at `index` among `parent`'s children and starts editing it; `before` is
    /// the selection to restore when it is undone or left empty.
    pub(super) fn new_task(
        document: &mut Document,
        (parent, index): (ItemId, usize),
        before: Selection,
    ) -> Self {
        let id = document.new_id();
        document
            .apply(&insert(parent, index, id, String::new()))
            .expect("a task fits in every container at every index up to its children's count");
        Self {
            id,
            field: TextField::new(String::new()),
            before,
            origin: Origin::New { parent, index },
        }
    }

    /// The task edited.
    pub(super) fn id(&self) -> ItemId {
        self.id
    }

    /// The text edited.
    pub(super) fn field(&self) -> &TextField {
        &self.field
    }

    /// Applies `edit` to the text and shows it as the task's title. An edit that puts `@` at the
    /// start of a word, by typing it or by deleting what stood before it, is refused: TaskPaper
    /// has no escape for it, so the word would read back as a tag.
    pub(super) fn input(&mut self, document: &mut Document, edit: TextEdit) {
        let edited = self.field.edited(edit);
        if word_starts(edited.text()) > word_starts(self.field.text()) {
            return;
        }
        self.field = edited;
        let item = document
            .node(self.id)
            .and_then(Node::item)
            .expect("the edited task stays in the document while it is edited");
        let item = Item {
            title: self.field.text().to_owned(),
            ..item.clone()
        };
        document
            .set(self.id, item)
            .expect("a task stays a task when its title changes");
    }

    /// Ends editing: puts the document back as it was before and returns the change the session
    /// made, to apply as one step, with the selection from before it. The title is trimmed. A new
    /// task left empty is not added, and an existing one emptied keeps its title, so either
    /// makes no change, as does a title left as it was.
    pub(super) fn finish(self, document: &mut Document) -> (Transaction, Selection) {
        let title = self.field.text().trim().to_owned();
        let transaction = match self.origin {
            Origin::Existing(item) => {
                let after = Item {
                    title,
                    ..item.clone()
                };
                document
                    .set(self.id, item.clone())
                    .expect("a task stays a task when its title changes");
                if after.title.is_empty() || after.title == item.title {
                    Transaction::default()
                } else {
                    Transaction::new(vec![Op::Set {
                        id: self.id,
                        before: item,
                        after,
                    }])
                }
            }
            Origin::New { parent, index } => {
                document
                    .apply(&insert(parent, index, self.id, self.field.text().to_owned()).inverse())
                    .expect("the new task stays where it was added while it is edited");
                if title.is_empty() {
                    Transaction::default()
                } else {
                    insert(parent, index, self.id, title)
                }
            }
        };
        (transaction, self.before)
    }
}

/// How many words of `text` start with `@`.
fn word_starts(text: &str) -> usize {
    text.char_indices()
        .filter(|&(at, c)| {
            c == '@'
                && text
                    .get(..at)
                    .and_then(|before| before.chars().next_back())
                    .is_none_or(char::is_whitespace)
        })
        .count()
}

/// The transaction that puts a task titled `title`, numbered `id`, at `index` among `parent`'s
/// children.
fn insert(parent: ItemId, index: usize, id: ItemId, title: String) -> Transaction {
    Transaction::new(vec![Op::Insert {
        parent: Some(parent),
        index,
        subtree: Snapshot::new(id, Node::Item(Item::new(ItemKind::Task, title))),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::Motion, domain::PlacementError};

    fn title(document: &Document, id: ItemId) -> Option<&str> {
        Some(document.node(id)?.item()?.title.as_str())
    }

    fn typed(editing: &mut Editing, document: &mut Document, text: &str) {
        for c in text.chars() {
            editing.input(document, TextEdit::Insert(c));
        }
    }

    /// A document whose Inbox holds task `a` with a checklist item, and a selection on `a`.
    fn document() -> Result<(Document, ItemId, ItemId, Selection), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let a = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "a")))?;
        let check = document.push(Some(a), Node::Item(Item::new(ItemKind::ChecklistItem, "c")))?;
        let mut selection = Selection::default();
        selection.focus(Some(a));
        Ok((document, a, check, selection))
    }

    // FR-EDIT-01, FR-EDIT-02
    #[test]
    fn a_title_changes_as_typed_and_ends_as_one_change() -> Result<(), PlacementError> {
        let (mut document, a, _, selection) = document()?;
        let original = document.clone();
        let mut editing =
            Editing::existing(&document, a, selection.clone()).ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(editing.id(), a);
        assert_eq!(editing.field().caret(), 1, "the caret starts at the end");
        typed(&mut editing, &mut document, "nd b ");
        assert_eq!(title(&document, a), Some("and b "));
        let (transaction, before) = editing.finish(&mut document);
        assert_eq!(document, original, "finishing puts the document back");
        assert_eq!(before, selection);
        document.apply(&transaction)?;
        assert_eq!(title(&document, a), Some("and b"));
        document.apply(&transaction.inverse())?;
        assert_eq!(document, original);
        Ok(())
    }

    // FR-EDIT-01
    #[test]
    fn a_title_emptied_or_left_as_it_was_changes_nothing() -> Result<(), PlacementError> {
        let (mut document, a, _, selection) = document()?;
        let original = document.clone();
        for edits in [
            vec![TextEdit::DeleteBackward, TextEdit::Insert(' ')],
            vec![TextEdit::Insert('x'), TextEdit::DeleteBackward],
        ] {
            let mut editing = Editing::existing(&document, a, selection.clone())
                .ok_or(PlacementError::DoesNotFit)?;
            for edit in edits {
                editing.input(&mut document, edit);
            }
            assert_eq!(editing.finish(&mut document).0, Transaction::default());
            assert_eq!(document, original);
        }
        Ok(())
    }

    #[test]
    fn only_tasks_are_edited_here() -> Result<(), PlacementError> {
        let (document, _, check, selection) = document()?;
        let inbox = document.inbox();
        for id in [inbox, check, ItemId::new(99)] {
            assert_eq!(Editing::existing(&document, id, selection.clone()), None);
        }
        Ok(())
    }

    // FR-EDIT-05
    #[test]
    fn a_new_task_is_shown_while_edited_and_added_as_one_change() -> Result<(), PlacementError> {
        let (mut document, a, _, selection) = document()?;
        let inbox = document.inbox();
        let original = document.clone();
        let mut editing = Editing::new_task(&mut document, (inbox, 0), selection.clone());
        let id = editing.id();
        assert_eq!(document.children(inbox), [id, a]);
        assert_eq!(title(&document, id), Some(""));
        typed(&mut editing, &mut document, " new");
        assert_eq!(title(&document, id), Some(" new"));
        let (transaction, before) = editing.finish(&mut document);
        assert_eq!(
            document.children(inbox),
            [a],
            "finishing takes the draft out"
        );
        assert_eq!(before, selection);
        document.apply(&transaction)?;
        assert_eq!(document.children(inbox), [id, a]);
        assert_eq!(title(&document, id), Some("new"));
        document.apply(&transaction.inverse())?;
        assert_eq!(document.children(inbox), original.children(inbox));
        Ok(())
    }

    // FR-EDIT-05
    #[test]
    fn a_new_task_left_empty_is_not_added() -> Result<(), PlacementError> {
        let (mut document, a, _, selection) = document()?;
        let inbox = document.inbox();
        let mut editing = Editing::new_task(&mut document, (inbox, 1), selection);
        typed(&mut editing, &mut document, "  ");
        assert_eq!(editing.finish(&mut document).0, Transaction::default());
        assert_eq!(document.children(inbox), [a]);
        Ok(())
    }

    // FR-EDIT-04
    #[test]
    fn at_the_start_of_a_word_an_at_sign_is_refused() -> Result<(), PlacementError> {
        let (mut document, a, _, selection) = document()?;
        let mut editing =
            Editing::existing(&document, a, selection).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut editing, &mut document, "@b @c");
        assert_eq!(
            title(&document, a),
            Some("a@b c"),
            "inside a word it is text"
        );
        editing.input(
            &mut document,
            TextEdit::Move {
                motion: Motion::Start,
                select: false,
            },
        );
        typed(&mut editing, &mut document, "@");
        editing.input(&mut document, TextEdit::SelectAll);
        typed(&mut editing, &mut document, "@");
        assert_eq!(title(&document, a), Some("a@b c"));
        for (motion, select) in [(Motion::End, false), (Motion::Left, true)] {
            editing.input(&mut document, TextEdit::Move { motion, select });
        }
        typed(&mut editing, &mut document, "@");
        assert_eq!(
            title(&document, a),
            Some("a@b c"),
            "nor in place of a selection after a space"
        );
        editing.input(&mut document, TextEdit::SelectAll);
        typed(&mut editing, &mut document, "x@y");
        editing.input(
            &mut document,
            TextEdit::Move {
                motion: Motion::Start,
                select: false,
            },
        );
        editing.input(&mut document, TextEdit::DeleteForward);
        assert_eq!(
            title(&document, a),
            Some("x@y"),
            "nor by deleting what stands before it"
        );
        Ok(())
    }
}
