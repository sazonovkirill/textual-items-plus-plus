//! Editing the title of a task, section, project or area in place, an existing one's or one just
//! added (FR-EDIT-01, FR-EDIT-02, FR-EDIT-05, FR-ORG-06). The title is written to the document as
//! it is typed, so the item is drawn like any other, and the whole session becomes one change to
//! undo when it ends.

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

/// An item whose title is being edited, its kind, the text edited, the selection before editing
/// started and how the item was before it, and the document's revision then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Editing {
    id: ItemId,
    kind: ItemKind,
    field: TextField,
    before: Selection,
    origin: Origin,
    revision: u64,
}

/// How the edited item was before editing started.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Origin {
    /// It was there, as this item.
    Existing(Item),
    /// It was added for editing, at this index among this parent's children, or the top-level
    /// nodes for `None`.
    New {
        parent: Option<ItemId>,
        index: usize,
    },
}

impl Editing {
    /// Starts editing the title of `id`, the caret at its end, when it is a task, section,
    /// project or area; `before` is the selection to restore when the session is undone.
    pub(super) fn existing(document: &Document, id: ItemId, before: Selection) -> Option<Self> {
        let item = document.node(id).and_then(Node::item).filter(|item| {
            matches!(
                item.kind,
                ItemKind::Task | ItemKind::Section | ItemKind::Project | ItemKind::Area
            )
        })?;
        Some(Self {
            id,
            kind: item.kind,
            field: TextField::new(item.title.clone()),
            before,
            origin: Origin::Existing(item.clone()),
            revision: document.revision(),
        })
    }

    /// Adds an empty item of `kind` at `index` among `parent`'s children, or the top-level nodes
    /// for `None`, and starts editing it; `before` is the selection to restore when it is undone
    /// or left empty. The caller picks a place where the kind fits.
    pub(super) fn new_item(
        document: &mut Document,
        kind: ItemKind,
        (parent, index): (Option<ItemId>, usize),
        before: Selection,
    ) -> Self {
        let revision = document.revision();
        let id = document.new_id();
        document
            .apply(&insert(parent, index, id, kind, String::new()))
            .expect("the caller places a new item where its kind fits");
        Self {
            id,
            kind,
            field: TextField::new(String::new()),
            before,
            origin: Origin::New { parent, index },
            revision,
        }
    }

    /// The item edited.
    pub(super) fn id(&self) -> ItemId {
        self.id
    }

    /// What the item edited is.
    pub(super) fn kind(&self) -> ItemKind {
        self.kind
    }

    /// The document's revision when editing started.
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }

    /// The text edited.
    pub(super) fn field(&self) -> &TextField {
        &self.field
    }

    /// Applies `edit` to the text and shows it as the item's title. An edit that puts `@` at the
    /// start of a word, by typing it or by deleting what stood before it, is refused: TaskPaper
    /// has no escape for it, so the word would read back as a tag. So is an edit that starts a
    /// section's, project's or area's title with `-` and a space, which would read back as a
    /// task.
    pub(super) fn input(&mut self, document: &mut Document, edit: TextEdit) {
        let edited = self.field.edited(edit);
        if word_starts(edited.text()) > word_starts(self.field.text())
            || (self.kind != ItemKind::Task && reads_as_task(edited.text()))
        {
            return;
        }
        self.field = edited;
        let item = document
            .node(self.id)
            .and_then(Node::item)
            .expect("the edited item stays in the document while it is edited");
        let item = Item {
            title: self.field.text().to_owned(),
            ..item.clone()
        };
        document
            .set(self.id, item)
            .expect("an item keeps its kind when its title changes");
    }

    /// Ends editing: puts the document back as it was before and returns the change the session
    /// made, to apply as one step, with the selection from before it. The title is trimmed. A new
    /// item left empty is not added, and an existing one emptied keeps its title, so either
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
                    .expect("an item keeps its kind when its title changes");
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
                let draft = self.field.text().to_owned();
                document
                    .apply(&insert(parent, index, self.id, self.kind, draft).inverse())
                    .expect("the new item stays where it was added while it is edited");
                if title.is_empty() {
                    Transaction::default()
                } else {
                    insert(parent, index, self.id, self.kind, title)
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

/// Whether `title`, as a project's line, would read back as a task: it starts, once trimmed as
/// it will be, with `-` and a space.
fn reads_as_task(title: &str) -> bool {
    let mut chars = title.trim_start().chars();
    chars.next() == Some('-') && chars.next().is_some_and(char::is_whitespace)
}

/// The transaction that puts an item of `kind` titled `title`, numbered `id`, at `index` among
/// `parent`'s children, or the top-level nodes for `None`.
fn insert(
    parent: Option<ItemId>,
    index: usize,
    id: ItemId,
    kind: ItemKind,
    title: String,
) -> Transaction {
    Transaction::new(vec![Op::Insert {
        parent,
        index,
        subtree: Snapshot::new(id, Node::Item(Item::new(kind, title))),
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
    fn the_inbox_and_checklist_items_are_not_edited_here() -> Result<(), PlacementError> {
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
        let mut editing = Editing::new_item(
            &mut document,
            ItemKind::Task,
            (Some(inbox), 0),
            selection.clone(),
        );
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
        let mut editing =
            Editing::new_item(&mut document, ItemKind::Task, (Some(inbox), 1), selection);
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

    // FR-ORG-06, FR-EDIT-05
    #[test]
    fn a_new_area_is_added_at_the_top_level_and_renamed_like_a_task() -> Result<(), PlacementError>
    {
        let (mut document, _, _, selection) = document()?;
        let mut editing =
            Editing::new_item(&mut document, ItemKind::Area, (None, 0), selection.clone());
        let id = editing.id();
        assert_eq!(editing.kind(), ItemKind::Area);
        assert_eq!(document.top_level(), [id]);
        typed(&mut editing, &mut document, "Home");
        let (transaction, _) = editing.finish(&mut document);
        assert_eq!(document.top_level(), []);
        document.apply(&transaction)?;
        assert_eq!(title(&document, id), Some("Home"));
        let mut editing =
            Editing::existing(&document, id, selection).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut editing, &mut document, " and garden");
        let (transaction, _) = editing.finish(&mut document);
        document.apply(&transaction)?;
        assert_eq!(title(&document, id), Some("Home and garden"));
        Ok(())
    }

    // FR-ORG-06, FR-STORE-02
    #[test]
    fn a_container_title_never_starts_with_a_dash_and_a_space() -> Result<(), PlacementError> {
        let (mut document, a, _, selection) = document()?;
        let mut editing =
            Editing::new_item(&mut document, ItemKind::Area, (None, 0), selection.clone());
        let area = editing.id();
        typed(&mut editing, &mut document, " - x -y");
        assert_eq!(title(&document, area), Some(" -x -y"));
        editing.input(&mut document, TextEdit::SelectAll);
        typed(&mut editing, &mut document, "x- y");
        editing.input(
            &mut document,
            TextEdit::Move {
                motion: Motion::Start,
                select: false,
            },
        );
        editing.input(&mut document, TextEdit::DeleteForward);
        assert_eq!(title(&document, area), Some("x- y"));
        let mut task =
            Editing::existing(&document, a, selection).ok_or(PlacementError::DoesNotFit)?;
        task.input(&mut document, TextEdit::SelectAll);
        typed(&mut task, &mut document, "- a");
        assert_eq!(title(&document, a), Some("- a"), "a task's may");
        Ok(())
    }
}
