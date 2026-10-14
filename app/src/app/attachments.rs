//! The attachments panel: an item's note, a task's checklist and an item's comments, read and
//! edited in a panel of a pane, each pane having its own, which takes the keys while it has them
//! as a pane's list does, as taskpaper-terminal's attachments panel is
//! ([attachments.md §3–5](../../../specs/reference/taskpaper-terminal/specs/attachments.md#3-attachments-panel-fr-att-03),
//! FR-ATT-01–05, FR-CHK-02–03). One of them is selected, which the keys act on. The note, a
//! checklist item and a comment are edited as an item's description and title are, so a session
//! is one step to undo and is saved like any edit.

use std::mem;

use jiff::civil::{Date, DateTime};

use super::{
    Command, Focus, Notice, Refusal, Side, State, TextField, Whose,
    checklist::{self, ChecklistLine},
    container,
    editing::Editing,
    view::{Shown, View},
};
use crate::domain::{Branch, Document, Item, ItemId, ItemKind, Node, Op, Transaction, comments};

/// What the user does in the open panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentsInput {
    /// Edit what is selected: the note as raw text (FR-ATT-03), a checklist item's text
    /// (FR-CHK-03) or a comment's text (FR-ATT-04).
    Edit,
    /// Select another of the note, the checklist items and the comments, in the order they show,
    /// as a motion moves the focus among rows.
    Select(Focus),
    /// Add a comment stamped with this local time, the newest, and edit its text (FR-ATT-04).
    AddComment(DateTime),
    /// Delete the selected checklist item with the items under it (FR-CHK-03) or the selected
    /// comment (FR-ATT-04).
    Delete,
    /// Close the panel, keeping what is being edited.
    Close,
}

/// What is selected in the open panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selected {
    /// The note.
    Note,
    /// One item of a task's checklist, or `None` for a checklist with no items (FR-CHK-02).
    Checklist(Option<ItemId>),
    /// The comment at this index, newest first, or `None` for an item with no comments, so that
    /// a comment can be added from there as from any entry (FR-ATT-04).
    Comment(Option<usize>),
}

/// The open panel: the item it is on, what is selected and what was selected before a checklist
/// item was added, to go back to when it is left empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Panel {
    id: ItemId,
    selected: Selected,
    adding_from: Option<Selected>,
}

impl Panel {
    /// The panel on item `id` with `selected` selected.
    fn new(id: ItemId, selected: Selected) -> Self {
        Self {
            id,
            selected,
            adding_from: None,
        }
    }
}

/// The open panel as a frontend shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentsView<'a> {
    /// The item the panel is on.
    pub id: ItemId,
    /// That item, whose note or comment shows as it is typed while it is edited.
    pub item: &'a Item,
    /// What is selected, one of what the item has.
    pub selected: Selected,
    /// The item's checklist, items in tree order, when it is a task (FR-CHK-02).
    pub checklist: Option<Vec<ChecklistLine<'a>>>,
    /// The text of what is selected as it is edited, while it is.
    pub editing: Option<&'a TextField>,
}

impl Panel {
    /// The item the panel is on.
    pub(super) fn id(self) -> ItemId {
        self.id
    }
}

/// `panel` as a frontend shows it on `document`, with `editing` the text of what it has selected
/// as it is edited; none once its item is gone.
pub(super) fn view<'a>(
    document: &'a Document,
    panel: Panel,
    editing: Option<&'a TextField>,
) -> Option<AttachmentsView<'a>> {
    let Panel { id, selected, .. } = panel;
    let item = document.node(id).and_then(Node::item)?;
    let checklist = (item.kind == ItemKind::Task).then(|| checklist::lines(document, id));
    Some(AttachmentsView {
        id,
        item,
        selected: kept(item, selected),
        checklist,
        editing,
    })
}

/// Whether a panel on `id` stays open in a pane showing `shown` on `document`: while its item is
/// there and, unless the pane shows the Trash, not in the Trash with what holds it.
pub(super) fn stays(document: &Document, id: ItemId, shown: Shown) -> bool {
    let item = |id| document.node(id).and_then(Node::item);
    item(id).is_some()
        && (shown == Shown::View(View::Trash)
            || container::path(document, id)
                .into_iter()
                .all(|id| item(id).is_none_or(|item| item.deleted.is_none())))
}

impl State {
    /// The attachments panel that has the keys, while one does (FR-ATT-01).
    #[must_use]
    pub fn attachments(&self) -> Option<AttachmentsView<'_>> {
        let panel = self.pane.panel.filter(|_| self.panel_has_keys())?;
        // While the panel has the keys, what is edited is what it has selected.
        view(
            &self.document,
            panel,
            self.editing().map(|(_, _, field)| field),
        )
    }

    /// Whether the panel of the pane with the keys has them, rather than its list.
    pub(super) fn panel_has_keys(&self) -> bool {
        self.in_panel && self.pane.panel.is_some() && !self.sidebar.has_keys()
    }

    /// Opens the panel of the pane with the keys on the focused item or on the container shown,
    /// as `whose` says, with `selected` selected as far as it has it, and gives it the keys.
    /// Nothing opens with nothing focused.
    pub(super) fn open_attachments(&mut self, whose: Whose, selected: Selected) {
        let id = match whose {
            Whose::Focused => self.pane().selection.focused(),
            Whose::Shown => self.container(),
        };
        if let Some(id) = id {
            self.open_panel(id, selected);
        }
    }

    /// Opens the panel of the pane with the keys on item `id` with `selected` selected as far as
    /// it has it, in place of the item it was on, and gives it the keys, unless `id` is a
    /// checklist item, which has no attachments (FR-ATT-01, FR-ATT-02).
    pub(super) fn open_panel(&mut self, id: ItemId, selected: Selected) {
        let attached = self
            .document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| item.kind != ItemKind::ChecklistItem);
        if attached {
            self.pane.panel = Some(Panel::new(id, selected));
            self.in_panel = true;
        }
    }

    /// Opens the panel of the pane with the keys on the checklist of the focused task, its first
    /// item selected, and gives it the keys; for any other item the status line says it has none
    /// (FR-ATT-02). Nothing opens with nothing focused.
    pub(super) fn open_checklist(&mut self) {
        let Some(id) = self.pane().selection.focused() else {
            return;
        };
        if is_task(&self.document, id) {
            let selected = first_of_checklist(&self.document, id);
            self.open_panel(id, selected);
        } else {
            self.notify(Notice::NoChecklist);
        }
    }

    /// Applies `input` to the panel that has the keys: closing it gives them to its pane's list.
    /// Nothing happens while no panel has them, and only closing while something in it is edited.
    pub(super) fn attachments_input(&mut self, input: AttachmentsInput) {
        let Some(view) = self.attachments() else {
            return;
        };
        let (id, selected) = (view.id, view.selected);
        // Nothing is being added while this does anything but close.
        let panel = Panel::new(id, selected);
        let editing = self.editing().is_some();
        let before = self.pane().selection.clone();
        match input {
            AttachmentsInput::Close => {
                self.finish_editing();
                self.pane.panel = None;
                self.in_panel = false;
            }
            _ if editing => {}
            AttachmentsInput::Edit => {
                let editing = match selected {
                    Selected::Note => Editing::note(&self.document, id, before),
                    Selected::Checklist(item) => {
                        item.and_then(|item| Editing::existing(&self.document, item, before))
                    }
                    Selected::Comment(index) => {
                        index.and_then(|index| Editing::comment(&self.document, id, index, before))
                    }
                };
                self.start_editing(editing);
            }
            AttachmentsInput::Select(motion) => {
                self.pane.panel = Some(Panel::new(id, moved(&self.document, panel, motion)));
            }
            AttachmentsInput::AddComment(now) => {
                let editing =
                    Editing::new_comment(&mut self.document, id, comments::stamp(now), before);
                self.start_editing(editing);
                self.pane.panel = Some(Panel::new(id, Selected::Comment(Some(0))));
            }
            AttachmentsInput::Delete => {
                let transaction = match selected {
                    Selected::Checklist(Some(item)) => checklist::delete(&self.document, item),
                    Selected::Comment(Some(index)) => without_comment(&self.document, id, index),
                    Selected::Note | Selected::Checklist(None) | Selected::Comment(None) => return,
                };
                let before = entries(&self.document, id);
                self.change(transaction);
                self.pane.panel = Some(settled(&self.document, panel, &before));
            }
        }
    }

    /// Gives `command` to the panel that has the keys on `today`, and returns whether it took
    /// it. With a checklist entry selected it acts on the selected item as the same command acts
    /// on a row (FR-CHK-03): checking it, editing it, adding an item beside it, moving it among
    /// its siblings, indenting and outdenting it, each one step to undo. The motions select as
    /// in the panel. Quitting, pasting, the conflict prompt's commands and those that
    /// [move the keys](moves_keys) pass to the app; every other command does nothing while the
    /// panel has the keys.
    pub(super) fn panel_command(&mut self, command: Command, today: Date) -> bool {
        let passes = matches!(
            command,
            Command::Quit
                | Command::Paste(_)
                | Command::KeepMine
                | Command::TakeTheirs
                | Command::ShowDiff
        ) || moves_keys(command);
        if passes || !self.panel_has_keys() {
            return false;
        }
        let selected = self.attachments().map(|view| view.selected);
        match (command, selected) {
            (Command::Focus(motion), _) => {
                self.attachments_input(AttachmentsInput::Select(motion));
            }
            (Command::NewLike(side), _) => self.new_checklist_item(side),
            (Command::EditTitle, _) => self.attachments_input(AttachmentsInput::Edit),
            (Command::Delete, _) => self.attachments_input(AttachmentsInput::Delete),
            (Command::ToggleCompleted, Some(Selected::Checklist(Some(item)))) => {
                self.change(checklist::toggle(&self.document, item, today));
            }
            (Command::Move(direction), Some(Selected::Checklist(Some(item)))) => {
                self.change(checklist::shift(&self.document, item, direction));
            }
            (Command::Indent, Some(Selected::Checklist(Some(item)))) => {
                match checklist::indent(&self.document, item) {
                    Some(transaction) => self.change(transaction),
                    None => self.notify(Notice::Refused(Refusal::NothingAbove)),
                }
            }
            (Command::Outdent, Some(Selected::Checklist(Some(item)))) => {
                match checklist::outdent(&self.document, item) {
                    Some(transaction) => self.change(transaction),
                    None => self.notify(Notice::Refused(Refusal::NotNested)),
                }
            }
            _ => {}
        }
        true
    }

    /// Adds an empty checklist item on `side` of the one selected, or first or last in the task
    /// with none, selects it and starts editing its text. Nothing happens unless a checklist entry
    /// is selected.
    pub(super) fn new_checklist_item(&mut self, side: Side) {
        let Some(
            panel @ Panel {
                selected: Selected::Checklist(selected),
                ..
            },
        ) = self.pane.panel
        else {
            return;
        };
        let place = checklist::place(&self.document, panel.id, selected, side);
        let before = self.pane().selection.clone();
        let editing = Editing::new_item(
            &mut self.document,
            Item::new(ItemKind::ChecklistItem, ""),
            place,
            before,
        );
        self.pane.panel = Some(Panel {
            selected: Selected::Checklist(Some(editing.id())),
            adding_from: Some(panel.selected),
            ..panel
        });
        self.start_editing(Some(editing));
    }

    /// Ends adding a checklist item: when it was left empty, what was selected before it is
    /// selected again (FR-CHK-03).
    pub(super) fn finish_adding(&mut self, kept: bool) {
        if let Some(panel) = self.pane.panel.as_mut()
            && let Some(from) = panel.adding_from.take()
            && !kept
        {
            panel.selected = from;
        }
    }

    /// Pastes `branches` as checklist items on `side` of the selected one, one step to undo,
    /// selecting the last; nothing happens unless a checklist entry is selected (FR-CHK-03).
    pub(super) fn paste_checklist(&mut self, side: Side, branches: &[Branch]) {
        let Some(
            panel @ Panel {
                selected: Selected::Checklist(selected),
                ..
            },
        ) = self.pane.panel
        else {
            return;
        };
        if let Some((transaction, last)) =
            checklist::paste(&mut self.document, panel.id, selected, side, branches)
        {
            self.change(transaction);
            self.pane.panel = Some(Panel {
                selected: Selected::Checklist(Some(last)),
                ..panel
            });
        }
    }
}

/// `panel` once the document read again as `new` replaced `old`: on its item while `exact` finds
/// it again, with the checklist item selected while `exact` finds it, else the checklist's first
/// entry, and the comment selected found again as the same line among the comments read, or at
/// the same place when it is not (FR-ATT-01, FR-ATT-04).
pub(super) fn rematched(
    panel: Panel,
    old: &Document,
    new: &Document,
    exact: &dyn Fn(ItemId) -> Option<ItemId>,
) -> Option<Panel> {
    let Panel { id, selected, .. } = panel;
    let found = exact(id)?;
    let selected = match selected {
        Selected::Checklist(item) => match item.and_then(exact) {
            Some(again) => Selected::Checklist(Some(again)),
            None => first_of_checklist(new, found),
        },
        Selected::Comment(Some(index)) => {
            let line = old
                .node(id)
                .and_then(Node::item)
                .and_then(|item| item.comments.get(index));
            let comments = new
                .node(found)
                .and_then(Node::item)
                .map(|item| item.comments.as_slice())
                .unwrap_or_default();
            let again = comments.iter().position(|comment| Some(comment) == line);
            Selected::Comment(Some(again.unwrap_or(index)))
        }
        selected @ (Selected::Note | Selected::Comment(None)) => selected,
    };
    Some(Panel::new(found, selected))
}

/// Whether `command` moves the keys among the sidebar and the panes' lists and panels, which the
/// panel that has the keys passes to the app (FR-PANE-02).
pub(super) fn moves_keys(command: Command) -> bool {
    matches!(
        command,
        Command::NextPane
            | Command::PreviousPane
            | Command::FocusPane(_)
            | Command::FocusLeft
            | Command::FocusRight
    )
}

/// Whether the open panel acts on `command`, as [`State::panel_command`] says: the motions and
/// the commands that change a checklist item, and pasting.
pub(super) fn acts_on(command: Command) -> bool {
    matches!(
        command,
        Command::Focus(_)
            | Command::NewLike(_)
            | Command::EditTitle
            | Command::Delete
            | Command::ToggleCompleted
            | Command::Move(_)
            | Command::Indent
            | Command::Outdent
            | Command::Paste(_)
    )
}

/// `selected` as far as `item` has it: a comment past the last is the last, with no comments
/// the empty comments, and the empty comments once there are some the first.
fn kept(item: &Item, selected: Selected) -> Selected {
    match selected {
        Selected::Comment(index) => {
            let last = item.comments.len().checked_sub(1);
            Selected::Comment(
                index.map_or(last.and(Some(0)), |index| last.map(|last| index.min(last))),
            )
        }
        selected => selected,
    }
}

/// What the panel on `id` has to select, in the order it shows them: the note, then a task's
/// checklist items, or its empty checklist when it has none, then the comments, or its empty
/// comments when it has none.
fn entries(document: &Document, id: ItemId) -> Vec<Selected> {
    let mut entries = vec![Selected::Note];
    if is_task(document, id) {
        let items = checklist::lines(document, id);
        if items.is_empty() {
            entries.push(Selected::Checklist(None));
        }
        entries.extend(items.iter().map(|line| Selected::Checklist(Some(line.id))));
    }
    let comments = document
        .node(id)
        .and_then(Node::item)
        .map_or(0, |item| item.comments.len());
    if comments == 0 {
        entries.push(Selected::Comment(None));
    }
    entries.extend((0..comments).map(|index| Selected::Comment(Some(index))));
    entries
}

/// `panel` with its selection kept on what `document` has once what it had, `before`, changed
/// and what was selected is gone: what is now where it was takes its place while it is of the
/// same section, else the one before it, so that the selection stays in its section, which keeps
/// an entry, if only the one saying it is empty.
fn settled(document: &Document, panel: Panel, before: &[Selected]) -> Panel {
    let now = entries(document, panel.id);
    let at = before
        .iter()
        .position(|&entry| entry == panel.selected)
        .unwrap_or_default();
    let section = mem::discriminant(&panel.selected);
    let selected = now
        .get(at)
        .filter(|&entry| mem::discriminant(entry) == section)
        .or_else(|| now.get(at.checked_sub(1)?))
        .copied();
    Panel {
        selected: selected.unwrap_or(Selected::Note),
        ..panel
    }
}

/// What `motion` selects in `panel` on `document` from what is selected: the next or previous,
/// or the first or the last. Where there is none, what is selected.
fn moved(document: &Document, panel: Panel, motion: Focus) -> Selected {
    let entries = entries(document, panel.id);
    let at = entries
        .iter()
        .position(|&entry| entry == panel.selected)
        .unwrap_or_default();
    let moved = match motion {
        Focus::Next => entries.get(at + 1),
        Focus::Previous => at.checked_sub(1).and_then(|before| entries.get(before)),
        Focus::First => entries.first(),
        Focus::Last => entries.last(),
    };
    moved.copied().unwrap_or(panel.selected)
}

/// The first entry of the checklist of task `id`.
fn first_of_checklist(document: &Document, id: ItemId) -> Selected {
    Selected::Checklist(checklist::lines(document, id).first().map(|line| line.id))
}

pub(super) fn is_task(document: &Document, id: ItemId) -> bool {
    document
        .node(id)
        .and_then(Node::item)
        .is_some_and(|item| item.kind == ItemKind::Task)
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
    use crate::app::{
        Direction, EditInput, Effect, End, Go, Message, Motion, Opened, Part, Refusal, TextEdit,
        update, view::View,
    };
    use crate::domain::{Completion, PlacementError};

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
            "the sidebar opens it on its focused entry, the panel getting the keys"
        );
        assert!(!state.sidebar_has_keys());
        state.attachments_input(AttachmentsInput::Close);
        state.focus_pane(0);
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
        state.open_attachments(Whose::Focused, Selected::Comment(Some(0)));
        assert_eq!(
            selected(&state),
            Some(Selected::Comment(None)),
            "there is none yet"
        );
        state.attachments_input(AttachmentsInput::Edit);
        state.attachments_input(AttachmentsInput::Delete);
        assert_eq!(state.editing(), None, "nothing to edit");
        state.attachments_input(AttachmentsInput::AddComment(TODAY.at(9, 5, 0, 0)));
        assert_eq!(selected(&state), Some(Selected::Comment(Some(0))));
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
            Some(Selected::Comment(Some(0))),
            "the last left"
        );
        state.attachments_input(AttachmentsInput::Delete);
        assert_eq!(comments(&state, a), Vec::<String>::new());
        assert_eq!(
            selected(&state),
            Some(Selected::Comment(None)),
            "the last gone, there are none"
        );
        state.attachments_input(AttachmentsInput::Close);
        let before_each = [
            vec!["[2026-10-14 17:30] Again"],
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
    fn selection_goes_from_the_note_through_the_checklist_and_the_comments_newest_first()
    -> Result<(), PlacementError> {
        let (mut state, a, c) = state()?;
        let mut document = state.document().clone();
        let item = document.node(a).and_then(Node::item).cloned();
        let mut item = item.ok_or(PlacementError::DoesNotFit)?;
        item.comments = vec!["new".to_owned(), "middle".to_owned(), "old".to_owned()];
        document.set(a, item)?;
        state = State::new(document, Opened::Demo, TODAY);
        state.open_attachments(Whose::Focused, Selected::Comment(Some(0)));
        assert_eq!(selected(&state), Some(Selected::Comment(Some(0))));
        for (motion, expected) in [
            (Focus::Previous, Selected::Checklist(Some(c))),
            (Focus::Previous, Selected::Note),
            (Focus::Previous, Selected::Note),
            (Focus::Next, Selected::Checklist(Some(c))),
            (Focus::Next, Selected::Comment(Some(0))),
            (Focus::Last, Selected::Comment(Some(2))),
            (Focus::Next, Selected::Comment(Some(2))),
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

    fn give(state: &mut State, command: Command) {
        update(state, Message::Command(command), TODAY);
    }

    fn input(state: &mut State, input: AttachmentsInput) {
        update(state, Message::Attachments(input), TODAY);
    }

    fn edit(state: &mut State, input: EditInput) {
        update(state, Message::Edit(input), TODAY);
    }

    /// The entry of checklist item `id`.
    fn entry(id: ItemId) -> Selected {
        Selected::Checklist(Some(id))
    }

    /// The open panel's checklist: each item's title and depth.
    fn checklist(state: &State) -> Vec<(String, usize)> {
        state
            .attachments()
            .and_then(|view| view.checklist)
            .unwrap_or_default()
            .into_iter()
            .map(|line| (line.item.title.clone(), line.depth))
            .collect()
    }

    /// The item numbered `id`.
    fn item(state: &State, id: ItemId) -> Option<&Item> {
        state.document().node(id).and_then(Node::item)
    }

    /// A state on a file whose Inbox holds task `a` with the checklist items `titles`, focused on
    /// `a`.
    fn with_checklist<const N: usize>(
        titles: [&str; N],
    ) -> Result<(State, ItemId, [ItemId; N]), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let a = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "a")))?;
        let mut items = [a; N];
        for (id, title) in items.iter_mut().zip(titles) {
            let item = Item::new(ItemKind::ChecklistItem, title);
            *id = document.push(Some(a), Node::Item(item))?;
        }
        let state = State::new(document, Opened::File("a.taskpaper".into()), TODAY);
        Ok((state, a, items))
    }

    fn branch(title: &str) -> Branch {
        Branch {
            node: Node::Item(Item::new(ItemKind::Task, title)),
            children: Vec::new(),
        }
    }

    // FR-CHK-02, D-84
    #[test]
    fn deleting_back_in_an_empty_checklist_item_keeps_editing() -> Result<(), PlacementError> {
        let (mut state, _, [c]) = with_checklist(["c"])?;
        give(&mut state, Command::OpenChecklist);
        input(&mut state, AttachmentsInput::Edit);
        for _ in 0..2 {
            edit(&mut state, EditInput::Text(TextEdit::DeleteBackward));
        }
        assert_eq!(
            state.editing().map(|(id, .., field)| (id, field.text())),
            Some((c, ""))
        );
        Ok(())
    }

    // FR-ATT-02, FR-CHK-02
    #[test]
    fn the_checklist_opens_on_a_task_and_only_on_a_task() -> Result<(), PlacementError> {
        let (mut state, a, c) = state()?;
        give(&mut state, Command::OpenChecklist);
        let view = state.attachments().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!((view.id, Some(view.selected)), (a, Some(entry(c))));
        input(&mut state, AttachmentsInput::Close);
        let (mut state, ..) = with_checklist([])?;
        give(&mut state, Command::OpenChecklist);
        assert_eq!(selected(&state), Some(Selected::Checklist(None)));
        input(&mut state, AttachmentsInput::Edit);
        assert!(
            state.editing().is_none(),
            "an empty checklist has no item to edit"
        );
        input(&mut state, AttachmentsInput::Close);
        let inbox = state.document().inbox();
        state.open_panel(inbox, Selected::Note);
        assert_eq!(state.attachments().and_then(|view| view.checklist), None);
        input(&mut state, AttachmentsInput::Close);
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::OpenChecklist);
        assert_eq!(
            state.notice(),
            Some(&Notice::NoChecklist),
            "an entry has none"
        );
        assert!(state.attachments().is_none());
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Go(Go::View(View::Logbook)));
        give(&mut state, Command::OpenChecklist);
        assert!(state.attachments().is_none(), "nothing is focused");
        let mut document = Document::new();
        let mut project = Item::new(ItemKind::Project, "p");
        project.completed = Some(Completion::On(TODAY));
        document.push(None, Node::Item(project))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Go(Go::View(View::Logbook)));
        give(&mut state, Command::OpenChecklist);
        assert_eq!(
            state.notice(),
            Some(&Notice::NoChecklist),
            "a project has none"
        );
        Ok(())
    }

    // FR-ATT-01, FR-CHK-02
    #[test]
    fn the_motions_select_the_entries_in_turn() -> Result<(), PlacementError> {
        let (mut state, _, [b, c]) = with_checklist(["b", "c"])?;
        let mut document = state.document().clone();
        let nested = Item::new(ItemKind::ChecklistItem, "d");
        let d = document.push(Some(b), Node::Item(nested))?;
        state = State::new(document, Opened::Demo, TODAY);
        state.open_attachments(Whose::Focused, Selected::Note);
        let steps = [
            (Focus::Next, Some(entry(b))),
            (Focus::Next, Some(entry(d))),
            (Focus::Previous, Some(entry(b))),
            (Focus::Last, Some(Selected::Comment(None))),
            (Focus::Next, Some(Selected::Comment(None))),
            (Focus::Previous, Some(entry(c))),
            (Focus::First, Some(Selected::Note)),
            (Focus::Previous, Some(Selected::Note)),
        ];
        for (motion, entry) in steps {
            input(&mut state, AttachmentsInput::Select(motion));
            assert_eq!(selected(&state), entry, "{motion:?}");
        }
        give(&mut state, Command::Focus(Focus::Next));
        assert_eq!(
            selected(&state),
            Some(entry(b)),
            "a motion command selects too"
        );
        input(&mut state, AttachmentsInput::Close);
        state.attachments_input(AttachmentsInput::Select(Focus::Next));
        assert!(state.attachments().is_none(), "nothing opens");
        let inbox = state.document().inbox();
        state.open_panel(inbox, Selected::Note);
        input(&mut state, AttachmentsInput::Select(Focus::Next));
        assert_eq!(
            selected(&state),
            Some(Selected::Comment(None)),
            "what is not a task has its note and comments"
        );
        Ok(())
    }

    // FR-CHK-03, FR-ATT-05, FR-EDIT-06
    #[test]
    fn items_are_added_edited_and_checked_each_one_step_to_undo() -> Result<(), PlacementError> {
        let (mut state, a, [c]) = with_checklist(["c"])?;
        give(&mut state, Command::OpenChecklist);
        give(&mut state, Command::NewLike(Side::Above));
        type_text(&mut state, "first");
        edit(&mut state, EditInput::Next);
        type_text(&mut state, "second");
        let view = state.attachments().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(
            view.editing.map(TextField::text),
            Some("second"),
            "Enter adds the next"
        );
        let second = view.selected;
        edit(&mut state, EditInput::Next);
        edit(&mut state, EditInput::Done);
        assert_eq!(
            selected(&state),
            Some(second),
            "an item left empty is not added, and the one before is selected again"
        );
        give(&mut state, Command::ToggleCompleted);
        give(&mut state, Command::EditTitle);
        type_text(&mut state, "!");
        input(&mut state, AttachmentsInput::Edit);
        input(&mut state, AttachmentsInput::Close);
        let second = state
            .document()
            .children(a)
            .iter()
            .copied()
            .find(|&id| entry(id) == second)
            .ok_or(PlacementError::DoesNotFit)?;
        let shown = item(&state, second).map(|it| (it.title.as_str(), it.completed));
        assert_eq!(shown, Some(("second!", Some(Completion::On(TODAY)))));
        assert_eq!(state.document().children(a).len(), 3);
        state.undo();
        assert_eq!(
            item(&state, second).map(|it| it.title.as_str()),
            Some("second")
        );
        state.undo();
        assert_eq!(item(&state, second).and_then(|it| it.completed), None);
        state.undo();
        state.undo();
        assert_eq!(
            state.document().children(a),
            [c],
            "each item added is a step"
        );
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn items_are_deleted_moved_indented_and_outdented() -> Result<(), PlacementError> {
        let (mut state, _, [c, d, e]) = with_checklist(["c", "d", "e"])?;
        give(&mut state, Command::OpenChecklist);
        give(&mut state, Command::Indent);
        assert_eq!(
            state.notice(),
            Some(&Notice::Refused(Refusal::NothingAbove))
        );
        give(&mut state, Command::Outdent);
        assert_eq!(state.notice(), Some(&Notice::Refused(Refusal::NotNested)));
        give(&mut state, Command::Move(Direction::Down));
        give(&mut state, Command::Focus(Focus::Previous));
        give(&mut state, Command::ToggleCompleted);
        give(&mut state, Command::Focus(Focus::Next));
        give(&mut state, Command::Indent);
        let nested = [
            ("d".to_owned(), 0),
            ("c".to_owned(), 1),
            ("e".to_owned(), 0),
        ];
        assert_eq!(checklist(&state), nested);
        give(&mut state, Command::Outdent);
        give(&mut state, Command::Focus(Focus::Last));
        give(&mut state, Command::Focus(Focus::Previous));
        give(&mut state, Command::Delete);
        assert_eq!(
            selected(&state),
            Some(entry(c)),
            "the last goes, the one before it is selected"
        );
        give(&mut state, Command::Focus(Focus::Previous));
        give(&mut state, Command::Delete);
        assert_eq!(
            selected(&state),
            Some(entry(c)),
            "the one after it is selected"
        );
        give(&mut state, Command::Delete);
        assert_eq!(selected(&state), Some(Selected::Checklist(None)));
        give(&mut state, Command::Delete);
        assert!(item(&state, d).is_none() && item(&state, e).is_none());
        Ok(())
    }

    /// The text being edited and where its caret is.
    fn edited(state: &State) -> Option<(&str, usize)> {
        state
            .editing()
            .map(|(.., field)| (field.text(), field.caret()))
    }

    // FR-CHK-03
    #[test]
    fn an_item_added_nests_while_edited_as_one_step_to_undo() -> Result<(), PlacementError> {
        let (mut state, a, [c]) = with_checklist(["c"])?;
        give(&mut state, Command::OpenChecklist);
        input(&mut state, AttachmentsInput::Edit);
        edit(&mut state, EditInput::Indent);
        assert_eq!(
            state.notice(),
            Some(&Notice::Refused(Refusal::NothingAbove))
        );
        assert_eq!(edited(&state), Some(("c", 1)), "editing goes on");
        edit(&mut state, EditInput::Next);
        type_text(&mut state, "dx");
        let left = TextEdit::Move {
            motion: Motion::Left,
            select: false,
        };
        edit(&mut state, EditInput::Text(left));
        edit(&mut state, EditInput::Indent);
        type_text(&mut state, "!");
        assert_eq!(edited(&state), Some(("d!x", 2)), "the caret stays");
        assert_eq!(
            checklist(&state),
            [("c".to_owned(), 0), ("d!x".to_owned(), 1)]
        );
        edit(&mut state, EditInput::Outdent);
        assert_eq!(
            checklist(&state),
            [("c".to_owned(), 0), ("d!x".to_owned(), 0)]
        );
        edit(&mut state, EditInput::Outdent);
        assert_eq!(state.notice(), Some(&Notice::Refused(Refusal::NotNested)));
        edit(&mut state, EditInput::Indent);
        edit(&mut state, EditInput::Done);
        let nested = [("c".to_owned(), 0), ("d!x".to_owned(), 1)];
        assert_eq!(checklist(&state), nested);
        state.undo();
        assert_eq!(state.document().children(a), [c]);
        assert!(
            state.document().children(c).is_empty(),
            "adding it where it nests is one step"
        );
        state.redo();
        assert_eq!(checklist(&state), nested);
        give(&mut state, Command::NewLike(Side::Below));
        edit(&mut state, EditInput::Outdent);
        edit(&mut state, EditInput::Done);
        assert_eq!(checklist(&state), nested, "an item left empty is not added");
        assert_eq!(
            selected(&state),
            state.document().children(c).first().copied().map(entry)
        );
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn an_item_edited_moves_with_its_text_as_one_step_to_undo() -> Result<(), PlacementError> {
        let (mut state, a, [c, d]) = with_checklist(["c", "d"])?;
        give(&mut state, Command::OpenChecklist);
        give(&mut state, Command::Focus(Focus::Next));
        give(&mut state, Command::EditTitle);
        type_text(&mut state, "!");
        edit(&mut state, EditInput::Indent);
        type_text(&mut state, "?");
        edit(&mut state, EditInput::Outdent);
        edit(&mut state, EditInput::Indent);
        edit(&mut state, EditInput::Done);
        assert_eq!(state.document().children(c), [d]);
        assert_eq!(item(&state, d).map(|it| it.title.as_str()), Some("d!?"));
        state.undo();
        assert_eq!(state.document().children(a), [c, d]);
        assert_eq!(item(&state, d).map(|it| it.title.as_str()), Some("d"));
        give(&mut state, Command::EditTitle);
        edit(&mut state, EditInput::Text(TextEdit::DeleteBackward));
        edit(&mut state, EditInput::Indent);
        edit(&mut state, EditInput::Done);
        assert_eq!(
            (
                state.document().children(c),
                item(&state, d).map(|it| it.title.as_str())
            ),
            ([d].as_slice(), Some("d")),
            "an item emptied keeps its text and still moves"
        );
        Ok(())
    }

    // FR-CHK-03
    #[test]
    fn only_a_checklist_item_nests_while_edited() -> Result<(), PlacementError> {
        let (mut state, a, _) = with_checklist(["c"])?;
        let mut document = state.document().clone();
        let inbox = Some(document.inbox());
        document.push(inbox, Node::Item(Item::new(ItemKind::Task, "b")))?;
        state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Focus(Focus::Last));
        let before = state.document().clone();
        give(&mut state, Command::EditTitle);
        edit(&mut state, EditInput::Indent);
        assert_eq!(state.document(), &before, "a task does not move");
        edit(&mut state, EditInput::Done);
        give(&mut state, Command::Focus(Focus::First));
        state.open_attachments(Whose::Focused, Selected::Note);
        input(&mut state, AttachmentsInput::Edit);
        edit(&mut state, EditInput::Outdent);
        assert_eq!(
            state.editing().map(|(id, part, _)| (id, part)),
            Some((a, Part::Note))
        );
        assert_eq!((state.document(), state.notice()), (&before, None));
        Ok(())
    }

    // FR-ATT-01, FR-CHK-03
    #[test]
    fn the_panel_takes_every_command_but_quitting_and_pasting() -> Result<(), PlacementError> {
        let (mut state, a, _) = with_checklist(["c"])?;
        state.open_attachments(Whose::Focused, Selected::Note);
        let before = state.document().clone();
        for command in [
            Command::ToggleCompleted,
            Command::NewLike(Side::Below),
            Command::Delete,
            Command::Sidebar,
            Command::Undo,
        ] {
            give(&mut state, command);
        }
        assert_eq!(
            state.document(),
            &before,
            "the note selected, the checklist's keys do nothing"
        );
        assert!(!state.sidebar_has_keys(), "nor do the rest");
        update(
            &mut state,
            Message::Paste(Side::Below, vec![branch("r")]),
            TODAY,
        );
        assert_eq!(state.document(), &before, "nothing pastes on the note");
        input(&mut state, AttachmentsInput::Select(Focus::Next));
        give(&mut state, Command::AddProject(End::Bottom));
        assert_eq!(state.document(), &before);
        let effects = update(
            &mut state,
            Message::Command(Command::Paste(Side::Above)),
            TODAY,
        );
        assert_eq!(effects, [Effect::ReadClipboard(Side::Above)]);
        let pasted = vec![branch("p"), branch("q")];
        update(&mut state, Message::Paste(Side::Above, pasted), TODAY);
        let titles: Vec<_> = checklist(&state)
            .into_iter()
            .map(|(title, _)| title)
            .collect();
        assert_eq!(titles, ["p", "q", "c"]);
        let last = state.document().children(a).get(1).copied();
        assert_eq!(
            selected(&state),
            last.map(entry),
            "the last pasted is selected"
        );
        update(&mut state, Message::Paste(Side::Above, Vec::new()), TODAY);
        assert_eq!(checklist(&state).len(), 3);
        Ok(())
    }

    // F-FILE-06, FR-ATT-01
    #[test]
    fn a_rematched_panel_keeps_its_checklist_item_while_found() -> Result<(), PlacementError> {
        let (state, a, [b, c]) = with_checklist(["b", "c"])?;
        let old = state.document().clone();
        let rematched = |selected, exact: &dyn Fn(ItemId) -> Option<ItemId>| {
            super::rematched(Panel::new(a, selected), &old, state.document(), exact)
                .map(|panel| panel.selected)
        };
        let all = |id| Some(id);
        assert_eq!(rematched(entry(c), &all), Some(entry(c)));
        let without_c = |id| (id != c).then_some(id);
        assert_eq!(
            rematched(entry(c), &without_c),
            Some(entry(b)),
            "the first item when the one selected is gone"
        );
        assert_eq!(rematched(Selected::Checklist(None), &all), Some(entry(b)));
        Ok(())
    }

    /// The item each pane's panel is on, left to right, none where it has none.
    fn panels(state: &State) -> Vec<Option<ItemId>> {
        state
            .panes()
            .map(|pane| pane.attachments().map(|view| view.id))
            .collect()
    }

    /// Where the keys are: the sidebar as `None`, else the pane with them and whether its
    /// panel has them.
    fn keys(state: &State) -> Option<(usize, bool)> {
        (!state.sidebar_has_keys()).then(|| (state.current_pane(), state.attachments().is_some()))
    }

    /// A state on a file whose Inbox holds tasks `a` and `b`, focused on `a`, with the sidebar
    /// showing.
    fn two_tasks() -> Result<(State, ItemId, ItemId), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let a = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "a")))?;
        let b = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "b")))?;
        let state = State::new(document, Opened::File("a.taskpaper".into()), TODAY);
        Ok((state, a, b))
    }

    // FR-ATT-01, FR-PANE-01, FR-PANE-02
    #[test]
    fn each_pane_has_its_own_panel_and_the_keys_go_through_lists_and_panels()
    -> Result<(), PlacementError> {
        let (mut state, a, b) = two_tasks()?;
        give(&mut state, Command::OpenNote(Whose::Focused));
        assert_eq!(
            (panels(&state), keys(&state)),
            (vec![Some(a)], Some((0, true)))
        );
        for expected in [None, Some((0, false)), Some((0, true))] {
            give(&mut state, Command::NextPane);
            assert_eq!(keys(&state), expected, "the sidebar, the list, the panel");
        }
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::Focus(Focus::Next));
        assert_eq!(
            panels(&state),
            [Some(a)],
            "the panel stays on its item while the list moves"
        );
        give(&mut state, Command::OpenInNewPane);
        give(&mut state, Command::OpenComments(Whose::Focused));
        assert_eq!(panels(&state), [Some(a), Some(b)]);
        assert_eq!(selected(&state), Some(Selected::Comment(None)));
        assert_eq!(keys(&state), Some((1, true)));
        let stops = [
            None,
            Some((0, false)),
            Some((0, true)),
            Some((1, false)),
            Some((1, true)),
        ];
        for expected in stops {
            give(&mut state, Command::NextPane);
            assert_eq!(keys(&state), expected, "Tab wraps around");
        }
        for expected in stops.iter().rev().skip(1).chain(stops.last()) {
            give(&mut state, Command::PreviousPane);
            assert_eq!(keys(&state), *expected, "Shift+Tab goes back");
        }
        give(&mut state, Command::FocusPane(1));
        assert_eq!(keys(&state), Some((0, false)), "g1 gives the list the keys");
        for expected in [
            Some((0, true)),
            Some((1, false)),
            Some((1, true)),
            Some((1, true)),
        ] {
            give(&mut state, Command::FocusRight);
            assert_eq!(keys(&state), expected, "Ctrl+W l stops at the last");
        }
        for expected in [
            Some((1, false)),
            Some((0, true)),
            Some((0, false)),
            None,
            None,
        ] {
            give(&mut state, Command::FocusLeft);
            assert_eq!(keys(&state), expected, "Ctrl+W h stops at the sidebar");
        }
        give(&mut state, Command::FocusPane(2));
        give(&mut state, Command::FocusRight);
        give(&mut state, Command::FocusPane(0));
        give(&mut state, Command::FocusRight);
        assert_eq!(
            keys(&state),
            Some((1, true)),
            "back from the sidebar to the panel that had the keys"
        );
        assert!(state.takes(Command::NextPane) && state.takes(Command::FocusPane(1)));
        input(&mut state, AttachmentsInput::Close);
        assert_eq!(
            (panels(&state), keys(&state)),
            (vec![Some(a), None], Some((1, false))),
            "Esc closes only the panel with the keys, the list getting them"
        );
        give(&mut state, Command::FocusRight);
        assert_eq!(keys(&state), Some((1, false)), "no panel to go to");
        give(&mut state, Command::ClosePane);
        assert_eq!(
            (panels(&state), keys(&state)),
            (vec![Some(a)], Some((0, false))),
            "a pane closes with its panel, the next pane's list getting the keys"
        );
        Ok(())
    }

    // FR-ATT-01, FR-ATT-02
    #[test]
    fn opening_on_another_item_moves_the_panel_there_and_gives_it_the_keys()
    -> Result<(), PlacementError> {
        let (mut state, _, b) = two_tasks()?;
        let inbox = state.document().inbox();
        give(&mut state, Command::OpenNote(Whose::Focused));
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::Focus(Focus::Next));
        give(&mut state, Command::OpenChecklist);
        assert_eq!(
            (panels(&state), keys(&state)),
            (vec![Some(b)], Some((0, true)))
        );
        assert_eq!(selected(&state), Some(Selected::Checklist(None)));
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::OpenNote(Whose::Shown));
        assert_eq!(panels(&state), [Some(inbox)], "on the container shown");
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::Go(Go::View(View::Logbook)));
        assert_eq!(
            panels(&state),
            [Some(inbox)],
            "showing another place keeps the panel"
        );
        give(&mut state, Command::Go(Go::Back));
        give(&mut state, Command::OpenNote(Whose::Focused));
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::OpenInNewPane);
        give(&mut state, Command::FocusPane(0));
        give(&mut state, Command::OpenComments(Whose::Focused));
        assert_eq!(
            (panels(&state), keys(&state)),
            (vec![Some(inbox), None], Some((0, true))),
            "the sidebar opens it in the first pane, which it leads"
        );
        Ok(())
    }

    // FR-ATT-01, FR-VIEW-04
    #[test]
    fn a_panel_closes_once_its_item_is_gone_or_in_the_trash_unless_it_shows_the_trash()
    -> Result<(), PlacementError> {
        let (mut state, a, b) = two_tasks()?;
        give(&mut state, Command::OpenNote(Whose::Focused));
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::Delete);
        assert_eq!(panels(&state), [None], "a went to the Trash");
        give(&mut state, Command::Undo);
        assert_eq!(panels(&state), [None], "and the panel stays closed");
        give(&mut state, Command::Focus(Focus::First));
        give(&mut state, Command::OpenNote(Whose::Focused));
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::OpenInNewPane);
        give(&mut state, Command::Go(Go::View(View::Trash)));
        give(&mut state, Command::FocusPane(1));
        give(&mut state, Command::Delete);
        assert_eq!(panels(&state), [None, None]);
        give(&mut state, Command::FocusPane(2));
        give(&mut state, Command::OpenNote(Whose::Focused));
        input(
            &mut state,
            AttachmentsInput::AddComment(TODAY.at(9, 5, 0, 0)),
        );
        type_text(&mut state, "kept");
        edit(&mut state, EditInput::Done);
        assert_eq!(
            panels(&state),
            [None, Some(a)],
            "in the Trash, the panel stays on what is there"
        );
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::Go(Go::Back));
        assert_eq!(panels(&state), [None, None], "and closes leaving it");
        give(&mut state, Command::FocusPane(1));
        give(&mut state, Command::Focus(Focus::First));
        give(&mut state, Command::OpenNote(Whose::Focused));
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::Cut);
        assert_eq!(panels(&state), [None, None], "b, cut, is gone");
        assert!(item(&state, b).is_none());
        Ok(())
    }

    // FR-ATT-05, FR-EDIT-06
    #[test]
    fn what_one_panel_edits_shows_in_no_other_and_ends_as_the_keys_leave()
    -> Result<(), PlacementError> {
        let (mut state, a, _) = two_tasks()?;
        give(&mut state, Command::OpenNote(Whose::Focused));
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::OpenInNewPane);
        give(&mut state, Command::OpenNote(Whose::Focused));
        input(&mut state, AttachmentsInput::Edit);
        type_text(&mut state, "x");
        let editing: Vec<_> = state
            .panes()
            .map(|pane| {
                pane.attachments()
                    .and_then(|view| view.editing)
                    .map(TextField::text)
            })
            .collect();
        assert_eq!(editing, [None, Some("x")]);
        give(&mut state, Command::PreviousPane);
        assert_eq!(state.editing(), None, "leaving ends editing");
        assert_eq!(note(&state, a), Some("x"));
        assert_eq!(keys(&state), Some((1, false)));
        give(&mut state, Command::Undo);
        assert_eq!(note(&state, a), Some(""), "one step to undo");
        Ok(())
    }
}
