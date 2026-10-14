//! Editing the title of a task, section, project or area in place, an existing one's or one just
//! added, any item's description in place, or its note or a comment, an existing one or one just
//! added, in the attachments panel (FR-EDIT-01, FR-EDIT-02, FR-EDIT-03, FR-EDIT-05, FR-ORG-06,
//! FR-ORG-10, FR-ATT-03, FR-ATT-04). The text is written to the document as it is typed, so the
//! item is drawn like any other, and the whole session becomes one change to undo when it ends.

use super::{Selection, TextEdit, TextField};
use crate::domain::{
    Destination, Document, Item, ItemId, ItemKind, Node, Op, Snapshot, Transaction, comments,
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
    /// Ends editing without what was typed, as a comment's input does on `Esc` (FR-ATT-04).
    Cancel,
}

/// Which text of an item is edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Its title, on one line.
    Title,
    /// Its description, Markdown on any number of lines (FR-ORG-10).
    Description,
    /// Its note, Markdown on any number of lines, in the attachments panel (FR-ATT-03).
    Note,
    /// The text of its comment at this index, newest first, on one line after the timestamp it
    /// keeps, in the attachments panel (FR-ATT-04).
    Comment(usize),
}

/// An item whose title, description, note or comment is being edited, its kind, which text is
/// edited, the text edited, how a comment edited is written, the selection before editing started
/// and how the item was before it, and the document's revision then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Editing {
    id: ItemId,
    kind: ItemKind,
    part: Part,
    field: TextField,
    comment: Option<Comment>,
    before: Selection,
    origin: Origin,
    revision: u64,
}

/// How the comment edited is written: after the timestamp it keeps, and added at its index or
/// replacing the comment there.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Comment {
    stamp: String,
    added: bool,
}

/// How the edited item was before editing started.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Origin {
    /// It was there, as this item.
    Existing(Item),
    /// It was added for editing as this item, without its title, at this index among this
    /// parent's children, or the top-level nodes for `None`, and then had the items this
    /// transaction moves moved into it.
    New {
        item: Item,
        parent: Option<ItemId>,
        index: usize,
        contents: Transaction,
    },
}

/// Why an edit to a title was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refused {
    /// It put `@` at the start of a word, which would read back as a tag.
    Tag,
    /// It started a container's title with `-` and a space, which would read back as a task.
    Task,
}

impl Editing {
    /// Starts editing the title of `id`, the caret at its end, when it is a task, section,
    /// project, area or checklist item; `before` is the selection to restore when the session is
    /// undone.
    pub(super) fn existing(document: &Document, id: ItemId, before: Selection) -> Option<Self> {
        let item = document.node(id).and_then(Node::item).filter(|item| {
            matches!(
                item.kind,
                ItemKind::Task
                    | ItemKind::Section
                    | ItemKind::Project
                    | ItemKind::Area
                    | ItemKind::ChecklistItem
            )
        })?;
        Some(Self {
            id,
            kind: item.kind,
            part: Part::Title,
            field: TextField::new(text(item, Part::Title).to_owned()),
            comment: None,
            before,
            origin: Origin::Existing(item.clone()),
            revision: document.revision(),
        })
    }

    /// Starts editing the description of `id`, the caret at its end, unless it is a checklist
    /// item, which has none; `before` is the selection to restore when the session is undone.
    pub(super) fn description(document: &Document, id: ItemId, before: Selection) -> Option<Self> {
        Self::lines(document, id, Part::Description, before)
    }

    /// Starts editing the note of `id`, the caret at its end, unless it is a checklist item,
    /// which has none (FR-ATT-01); `before` is the selection to restore when the session is
    /// undone.
    pub(super) fn note(document: &Document, id: ItemId, before: Selection) -> Option<Self> {
        Self::lines(document, id, Part::Note, before)
    }

    /// Starts editing `part` of `id`, its description or its note, the caret at its end, unless
    /// it is a checklist item.
    fn lines(document: &Document, id: ItemId, part: Part, before: Selection) -> Option<Self> {
        let item = document
            .node(id)
            .and_then(Node::item)
            .filter(|item| item.kind != ItemKind::ChecklistItem)?;
        Some(Self {
            id,
            kind: item.kind,
            part,
            field: TextField::multi_line(text(item, part)),
            comment: None,
            before,
            origin: Origin::Existing(item.clone()),
            revision: document.revision(),
        })
    }

    /// Starts editing the text of `id`'s comment at `index`, newest first, the caret at its end,
    /// keeping its timestamp, when it has one there (FR-ATT-04); `before` is the selection to
    /// restore when the session is undone.
    pub(super) fn comment(
        document: &Document,
        id: ItemId,
        index: usize,
        before: Selection,
    ) -> Option<Self> {
        let item = document.node(id).and_then(Node::item)?;
        let (stamp, text) = comments::split(item.comments.get(index)?);
        Some(Self {
            id,
            kind: item.kind,
            part: Part::Comment(index),
            field: TextField::new(text.to_owned()),
            comment: Some(Comment {
                stamp: stamp.to_owned(),
                added: false,
            }),
            before,
            origin: Origin::Existing(item.clone()),
            revision: document.revision(),
        })
    }

    /// Adds a comment stamped `stamp`, with no text yet, as the newest of `id`'s, and starts
    /// editing its text, unless `id` is a checklist item, which has none (FR-ATT-04); `before`
    /// is the selection to restore when it is undone. Leaving it empty adds nothing.
    pub(super) fn new_comment(
        document: &mut Document,
        id: ItemId,
        stamp: String,
        before: Selection,
    ) -> Option<Self> {
        let item = document
            .node(id)
            .and_then(Node::item)
            .filter(|item| item.kind != ItemKind::ChecklistItem)?;
        let editing = Self {
            id,
            kind: item.kind,
            part: Part::Comment(0),
            field: TextField::new(String::new()),
            comment: Some(Comment { stamp, added: true }),
            before,
            origin: Origin::Existing(item.clone()),
            revision: document.revision(),
        };
        editing.show(document);
        Some(editing)
    }

    /// Adds `item`, which has no title, at `index` among `parent`'s children, or the top-level
    /// nodes for `None`, and starts editing its title; `before` is the selection to restore when
    /// it is undone or left empty. The caller picks a place where its kind fits.
    pub(super) fn new_item(
        document: &mut Document,
        item: Item,
        place: (Option<ItemId>, usize),
        before: Selection,
    ) -> Self {
        Self::group(document, item, place, &[], before)
    }

    /// Adds `item` as [`new_item`](Self::new_item) does, moves `items` into it, each the last of
    /// its children in turn, and starts editing it: adding it, moving them and naming it are one
    /// change, which leaving it empty does not make (FR-OPS-07, FR-OPS-08). The caller groups
    /// items that fit in the new one and do not hold its place.
    pub(super) fn group(
        document: &mut Document,
        item: Item,
        (parent, index): (Option<ItemId>, usize),
        items: &[ItemId],
        before: Selection,
    ) -> Self {
        let revision = document.revision();
        let id = document.new_id();
        document
            .apply(&insert(parent, index, id, item.clone()))
            .expect("the caller places a new item where its kind fits");
        let mut ops = Vec::new();
        for &grouped in items {
            let moved = document
                .move_to(grouped, Destination::Last(Some(id)))
                .expect("the caller groups items that fit in the new one");
            document
                .apply(&moved)
                .expect("a transaction made from the document as it is fits it");
            ops.extend_from_slice(moved.ops());
        }
        Self {
            id,
            kind: item.kind,
            part: Part::Title,
            field: TextField::new(String::new()),
            comment: None,
            before,
            origin: Origin::New {
                item,
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

    /// Applies `edit` to the text and shows it as the item's title, description or note. An edit
    /// that puts `@` at the start of a word of a title, by typing it or by deleting what stood
    /// before it, is refused: TaskPaper has no escape for it, so the word would read back as a
    /// tag. So is an edit that starts a section's, project's or area's title with `-` and a space,
    /// which would read back as a task. A description's or a note's lines are guarded, so they
    /// hold any text (FR-STORE-04, FR-STORE-05).
    ///
    /// # Errors
    ///
    /// Why the edit was refused, when it was.
    pub(super) fn input(&mut self, document: &mut Document, edit: TextEdit) -> Result<(), Refused> {
        let edited = self.field.edited(edit);
        if self.part == Part::Title {
            title_edit(self.kind, self.field.text(), edited.text())?;
        }
        self.field = edited;
        self.show(document);
        Ok(())
    }

    /// Puts the text back as it was before editing, or empty for a comment or an item added for
    /// editing, so that finishing makes no change.
    pub(super) fn revert(&mut self, document: &mut Document) {
        let text = match (&self.origin, &self.comment) {
            (Origin::Existing(item), None | Some(Comment { added: false, .. })) => {
                text(item, self.part)
            }
            _ => "",
        };
        self.field = TextField::new(text.to_owned());
        self.show(document);
    }

    /// Shows the text edited as the item's part edited.
    fn show(&self, document: &mut Document) {
        let (Origin::Existing(item) | Origin::New { item, .. }) = &self.origin;
        let item = self.with_text(item, self.field.text().to_owned());
        document
            .set(self.id, item)
            .expect("an item keeps its kind when its text changes");
    }

    /// `item`, as it was before editing, with `text` as the part edited.
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
            Part::Note => Item {
                note: text,
                ..item.clone()
            },
            Part::Comment(index) => {
                let Comment { stamp, added } = self
                    .comment
                    .as_ref()
                    .expect("a comment is edited with how it is written");
                let mut comments = item.comments.clone();
                let line = format!("{stamp}{text}");
                if *added {
                    comments.insert(index, line);
                } else if let Some(comment) = comments.get_mut(index) {
                    *comment = line;
                }
                Item {
                    comments,
                    ..item.clone()
                }
            }
        }
    }

    /// Ends editing: puts the document back as it was before and returns the change the session
    /// made, to apply as one step, with the selection from before it. A title or a comment is
    /// trimmed: a new item or comment left empty is not added, and an existing one emptied keeps
    /// its text, so either makes no change. A description or a note loses the space and empty
    /// lines at its end, which the file does not keep, so emptying it removes it. Text left as it
    /// was makes no change.
    pub(super) fn finish(self, document: &mut Document) -> (Transaction, Selection) {
        let one_line = matches!(self.part, Part::Title | Part::Comment(_));
        let text = if one_line {
            self.field.text().trim()
        } else {
            self.field.text().trim_end()
        };
        let transaction = match &self.origin {
            Origin::Existing(item) => {
                let after = self.with_text(item, text.to_owned());
                document
                    .set(self.id, item.clone())
                    .expect("an item keeps its kind when its text changes");
                if (one_line && text.is_empty()) || after == *item {
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
                item,
                parent,
                index,
                contents,
            } => {
                let added = |title: String| {
                    let insert = insert(*parent, *index, self.id, self.with_text(item, title));
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

/// The text of `item` that `part` names.
fn text(item: &Item, part: Part) -> &str {
    match part {
        Part::Title => &item.title,
        Part::Description => &item.description,
        Part::Note => &item.note,
        Part::Comment(index) => item
            .comments
            .get(index)
            .map_or("", |line| comments::split(line).1),
    }
}

/// Whether a title of an item of kind `kind` may go from `before` to `after`.
///
/// # Errors
///
/// Why not, when `after` puts `@` at the start of a word, which would read back as a tag, or
/// starts a section's, project's or area's title with `-` and a space, which would read back as a
/// task.
pub(super) fn title_edit(kind: ItemKind, before: &str, after: &str) -> Result<(), Refused> {
    if word_starts(after) > word_starts(before) {
        return Err(Refused::Tag);
    }
    let line_of_a_task = matches!(kind, ItemKind::Task | ItemKind::ChecklistItem);
    if !line_of_a_task && reads_as_task(after) {
        return Err(Refused::Task);
    }
    Ok(())
}

/// How many words of `text` start with `@`.
pub(super) fn word_starts(text: &str) -> usize {
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

/// The transaction that puts `item`, numbered `id`, at `index` among `parent`'s children, or the
/// top-level nodes for `None`.
fn insert(parent: Option<ItemId>, index: usize, id: ItemId, item: Item) -> Transaction {
    Transaction::new(vec![Op::Insert {
        parent,
        index,
        subtree: Snapshot::new(id, Node::Item(item)),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::Motion, domain::PlacementError};

    fn title(document: &Document, id: ItemId) -> Option<&str> {
        Some(document.node(id)?.item()?.title.as_str())
    }

    /// Types `text` and returns why each character refused was refused.
    fn typed(editing: &mut Editing, document: &mut Document, text: &str) -> Vec<Refused> {
        text.chars()
            .filter_map(|c| editing.input(document, TextEdit::Insert(c)).err())
            .collect()
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
                assert_eq!(editing.input(&mut document, edit), Ok(()));
            }
            assert_eq!(editing.finish(&mut document).0, Transaction::default());
            assert_eq!(document, original);
        }
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn the_inbox_is_not_edited_here_and_a_checklist_item_is() -> Result<(), PlacementError> {
        let (mut document, _, check, selection) = document()?;
        let inbox = document.inbox();
        for id in [inbox, ItemId::new(99)] {
            assert_eq!(Editing::existing(&document, id, selection.clone()), None);
        }
        let mut editing =
            Editing::existing(&document, check, selection).ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(editing.input(&mut document, TextEdit::SelectAll), Ok(()));
        for c in "- step".chars() {
            assert_eq!(
                editing.input(&mut document, TextEdit::Insert(c)),
                Ok(()),
                "a checklist item's text is written as a task's"
            );
        }
        assert_eq!(editing.field().text(), "- step");
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
            Item::new(ItemKind::Task, ""),
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
        let mut editing = Editing::new_item(
            &mut document,
            Item::new(ItemKind::Task, ""),
            (Some(inbox), 1),
            selection,
        );
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
        assert_eq!(typed(&mut editing, &mut document, "@b @c"), [Refused::Tag]);
        assert_eq!(
            title(&document, a),
            Some("a@b c"),
            "inside a word it is text"
        );
        assert_eq!(
            editing.input(
                &mut document,
                TextEdit::Move {
                    motion: Motion::Start,
                    select: false,
                }
            ),
            Ok(())
        );
        assert_eq!(typed(&mut editing, &mut document, "@"), [Refused::Tag]);
        assert_eq!(editing.input(&mut document, TextEdit::SelectAll), Ok(()));
        assert_eq!(typed(&mut editing, &mut document, "@"), [Refused::Tag]);
        assert_eq!(title(&document, a), Some("a@b c"));
        for (motion, select) in [(Motion::End, false), (Motion::Left, true)] {
            assert_eq!(
                editing.input(&mut document, TextEdit::Move { motion, select }),
                Ok(())
            );
        }
        assert_eq!(typed(&mut editing, &mut document, "@"), [Refused::Tag]);
        assert_eq!(
            title(&document, a),
            Some("a@b c"),
            "nor in place of a selection after a space"
        );
        assert_eq!(editing.input(&mut document, TextEdit::SelectAll), Ok(()));
        assert_eq!(typed(&mut editing, &mut document, "x@y"), []);
        assert_eq!(
            editing.input(
                &mut document,
                TextEdit::Move {
                    motion: Motion::Start,
                    select: false,
                }
            ),
            Ok(())
        );
        assert_eq!(
            editing.input(&mut document, TextEdit::DeleteForward),
            Err(Refused::Tag)
        );
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
        let mut editing = Editing::new_item(
            &mut document,
            Item::new(ItemKind::Area, ""),
            (None, 0),
            selection.clone(),
        );
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
        let mut editing = Editing::new_item(
            &mut document,
            Item::new(ItemKind::Area, ""),
            (None, 0),
            selection.clone(),
        );
        let area = editing.id();
        typed(&mut editing, &mut document, " - x -y");
        assert_eq!(title(&document, area), Some(" -x -y"));
        assert_eq!(editing.input(&mut document, TextEdit::SelectAll), Ok(()));
        typed(&mut editing, &mut document, "x- y");
        assert_eq!(
            editing.input(
                &mut document,
                TextEdit::Move {
                    motion: Motion::Start,
                    select: false,
                }
            ),
            Ok(())
        );
        assert_eq!(
            editing.input(&mut document, TextEdit::DeleteForward),
            Err(Refused::Task)
        );
        assert_eq!(title(&document, area), Some("x- y"));
        let mut task =
            Editing::existing(&document, a, selection).ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(task.input(&mut document, TextEdit::SelectAll), Ok(()));
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
        assert_eq!(editing.input(&mut document, TextEdit::SelectAll), Ok(()));
        assert_eq!(
            editing.input(&mut document, TextEdit::DeleteBackward),
            Ok(())
        );
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
