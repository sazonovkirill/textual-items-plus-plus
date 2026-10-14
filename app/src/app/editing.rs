//! Editing in place the title of a task, section, project or area, an existing one's or one just
//! added, or any item's description (FR-EDIT-01, FR-EDIT-02, FR-EDIT-03, FR-EDIT-05, FR-ORG-06,
//! FR-ORG-10). The text is written to the document as it is typed, so the item is drawn like any
//! other, and the whole session becomes one change to undo when it ends.

use super::{Selection, TextEdit, TextField};
use crate::domain::{
    Destination, Document, Item, ItemId, ItemKind, Node, Op, Snapshot, Transaction,
};

/// What the user does while a title or a description is edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditInput {
    /// Changes the text being edited.
    Text(TextEdit),
    /// Ends editing and, for a title, adds the next task below, unless the task edited was new
    /// and is left empty, which only ends editing.
    Next,
    /// Ends editing.
    Done,
}

/// Which text of an item is edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Its title, on one line.
    Title,
    /// Its description, Markdown on any number of lines (FR-ORG-10).
    Description,
}

/// An item whose title or description is being edited, its kind, which text is edited, the text
/// edited, the selection before editing started and how the item was before it, and the
/// document's revision then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Editing {
    id: ItemId,
    kind: ItemKind,
    part: Part,
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
    /// nodes for `None`, and then had the items this transaction moves moved into it.
    New {
        parent: Option<ItemId>,
        index: usize,
        contents: Transaction,
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
            part: Part::Title,
            field: TextField::new(item.title.clone()),
            before,
            origin: Origin::Existing(item.clone()),
            revision: document.revision(),
        })
    }

    /// Starts editing the description of `id`, the caret at its end, unless it is a checklist
    /// item, which has none; `before` is the selection to restore when the session is undone.
    pub(super) fn description(document: &Document, id: ItemId, before: Selection) -> Option<Self> {
        let item = document
            .node(id)
            .and_then(Node::item)
            .filter(|item| item.kind != ItemKind::ChecklistItem)?;
        Some(Self {
            id,
            kind: item.kind,
            part: Part::Description,
            field: TextField::multi_line(&item.description),
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
        place: (Option<ItemId>, usize),
        before: Selection,
    ) -> Self {
        Self::group(document, kind, place, &[], before)
    }

    /// Adds an empty item of `kind` as [`new_item`](Self::new_item) does, moves `items` into it,
    /// each the last of its children in turn, and starts editing it: adding it, moving them and
    /// naming it are one change, which leaving it empty does not make (FR-OPS-07, FR-OPS-08). The
    /// caller groups items that fit in the new one and do not hold its place.
    pub(super) fn group(
        document: &mut Document,
        kind: ItemKind,
        (parent, index): (Option<ItemId>, usize),
        items: &[ItemId],
        before: Selection,
    ) -> Self {
        let revision = document.revision();
        let id = document.new_id();
        document
            .apply(&insert(parent, index, id, kind, String::new()))
            .expect("the caller places a new item where its kind fits");
        let mut ops = Vec::new();
        for &item in items {
            let moved = document
                .move_to(item, Destination::Last(Some(id)))
                .expect("the caller groups items that fit in the new one");
            document
                .apply(&moved)
                .expect("a transaction made from the document as it is fits it");
            ops.extend_from_slice(moved.ops());
        }
        Self {
            id,
            kind,
            part: Part::Title,
            field: TextField::new(String::new()),
            before,
            origin: Origin::New {
                parent,
                index,
                contents: Transaction::new(ops),
            },
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

    /// Which of its texts is edited.
    pub(super) fn part(&self) -> Part {
        self.part
    }

    /// The document's revision when editing started.
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }

    /// The text edited.
    pub(super) fn field(&self) -> &TextField {
        &self.field
    }

    /// Applies `edit` to the text and shows it as the item's title or description. An edit that
    /// puts `@` at the start of a word of a title, by typing it or by deleting what stood before
    /// it, is refused: TaskPaper has no escape for it, so the word would read back as a tag. So is
    /// an edit that starts a section's, project's or area's title with `-` and a space, which
    /// would read back as a task. A description's lines are guarded, so they hold any text
    /// (FR-STORE-04).
    pub(super) fn input(&mut self, document: &mut Document, edit: TextEdit) {
        let edited = self.field.edited(edit);
        if self.part == Part::Title
            && (word_starts(edited.text()) > word_starts(self.field.text())
                || (self.kind != ItemKind::Task && reads_as_task(edited.text())))
        {
            return;
        }
        self.field = edited;
        let item = document
            .node(self.id)
            .and_then(Node::item)
            .expect("the edited item stays in the document while it is edited");
        let item = self.with_text(item, self.field.text().to_owned());
        document
            .set(self.id, item)
            .expect("an item keeps its kind when its text changes");
    }

    /// `item` with `text` as the part edited.
    fn with_text(&self, item: &Item, text: String) -> Item {
        match self.part {
            Part::Title => Item {
                title: text,
                ..item.clone()
            },
            Part::Description => Item {
                description: text,
                ..item.clone()
            },
        }
    }

    /// Ends editing: puts the document back as it was before and returns the change the session
    /// made, to apply as one step, with the selection from before it. A title is trimmed: a new
    /// item left empty is not added, and an existing one emptied keeps its title, so either
    /// makes no change. A description loses the space and empty lines at its end, which the file
    /// does not keep, so emptying it removes it. Text left as it was makes no change.
    pub(super) fn finish(self, document: &mut Document) -> (Transaction, Selection) {
        let text = match self.part {
            Part::Title => self.field.text().trim(),
            Part::Description => self.field.text().trim_end(),
        };
        let transaction = match &self.origin {
            Origin::Existing(item) => {
                let after = self.with_text(item, text.to_owned());
                document
                    .set(self.id, item.clone())
                    .expect("an item keeps its kind when its text changes");
                if (self.part == Part::Title && text.is_empty()) || after == *item {
                    Transaction::default()
                } else {
                    Transaction::new(vec![Op::Set {
                        id: self.id,
                        before: item.clone(),
                        after,
                    }])
                }
            }
            Origin::New {
                parent,
                index,
                contents,
            } => {
                let added = |title: String| {
                    let insert = insert(*parent, *index, self.id, self.kind, title);
                    Transaction::new([insert.ops(), contents.ops()].concat())
                };
                let draft = self.field.text().to_owned();
                document
                    .apply(&added(draft).inverse())
                    .expect("the new item stays where it was added while it is edited");
                if text.is_empty() {
                    Transaction::default()
                } else {
                    added(text.to_owned())
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
pub(super) fn reads_as_task(title: &str) -> bool {
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

    fn description(document: &Document, id: ItemId) -> Option<&str> {
        Some(document.node(id)?.item()?.description.as_str())
    }

    // FR-ORG-10, FR-EDIT-03, FR-STORE-04
    #[test]
    fn a_description_takes_lines_and_any_text_and_ends_as_one_change() -> Result<(), PlacementError>
    {
        let (mut document, a, _, selection) = document()?;
        let original = document.clone();
        let mut editing = Editing::description(&document, a, selection.clone())
            .ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(editing.part(), Part::Description);
        typed(&mut editing, &mut document, "@due Goals:\n- one  \n\n");
        assert_eq!(description(&document, a), Some("@due Goals:\n- one  \n\n"));
        assert_eq!(title(&document, a), Some("a"));
        let (transaction, before) = editing.finish(&mut document);
        assert_eq!(document, original, "finishing puts the document back");
        assert_eq!(before, selection);
        document.apply(&transaction)?;
        assert_eq!(
            description(&document, a),
            Some("@due Goals:\n- one"),
            "space and empty lines at the end go"
        );
        let mut editing =
            Editing::description(&document, a, selection).ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(editing.field().caret(), "@due Goals:\n- one".len());
        editing.input(&mut document, TextEdit::SelectAll);
        editing.input(&mut document, TextEdit::DeleteBackward);
        let (transaction, _) = editing.finish(&mut document);
        document.apply(&transaction)?;
        assert_eq!(
            description(&document, a),
            Some(""),
            "emptying it removes it"
        );
        Ok(())
    }

    // FR-ORG-10
    #[test]
    fn a_description_left_as_it_was_changes_nothing() -> Result<(), PlacementError> {
        let (mut document, a, _, selection) = document()?;
        let mut editing =
            Editing::description(&document, a, selection).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut editing, &mut document, " \n");
        assert_eq!(editing.finish(&mut document).0, Transaction::default());
        Ok(())
    }

    // FR-ORG-05, FR-ORG-10
    #[test]
    fn every_item_but_a_checklist_item_has_a_description() -> Result<(), PlacementError> {
        let (mut document, a, check, selection) = document()?;
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        for id in [document.inbox(), area, a] {
            assert!(Editing::description(&document, id, selection.clone()).is_some());
        }
        for id in [check, ItemId::new(99)] {
            assert_eq!(Editing::description(&document, id, selection.clone()), None);
        }
        Ok(())
    }
}
