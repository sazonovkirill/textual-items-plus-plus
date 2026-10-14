//! The application state a frontend renders, changed only by [`update`](super::update).

use std::path::PathBuf;

use jiff::civil::Date;

use super::{
    Command, Direction, EditInput, Effect, Go, History, Notice, Part, Picked, Picker, PickerInput,
    PickerView, Selection, Side, SidebarView, TextField, Whose, adding, clipboard, completion,
    container::{self, Row},
    editing::Editing,
    focus::{self, Focus},
    move_to::{Destination, MoveTo},
    moving,
    navigator::{self, Place},
    saving::Saving,
    sidebar::{self, Sidebar},
    trash,
    visits::{Visit, Visits},
};
use crate::domain::{Branch, Document, ItemId, ItemKind, Transaction};

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
    pub(super) document: Document,
    opened: Opened,
    pub(super) container: ItemId,
    pub(super) selection: Selection,
    pub(super) history: History,
    /// Whether the pane shows completed items (FR-VIEW-11); per pane once there are several.
    show_completed: bool,
    pub(super) visits: Visits,
    pub(super) navigator: Option<Picker<Place>>,
    /// Move to, while its picker is open (FR-OPS-04).
    pub(super) move_to: Option<MoveTo>,
    /// The item whose title or description is edited, in edit mode (FR-EDIT-01), in the sidebar
    /// while it has the keys.
    editing: Option<Editing>,
    sidebar: Sidebar,
    pub(super) saving: Saving,
}

impl State {
    /// The app showing `document`, opened from `opened`, at home: the Inbox, its first row
    /// focused (FR-NAV-02). The file holds the document, and every change to it is saved; the
    /// demo document is never saved (FR-STORE-01, FR-STORE-09).
    #[must_use]
    pub fn new(document: Document, opened: Opened) -> Self {
        let saving = Saving::new(document.revision(), true, matches!(opened, Opened::File(_)));
        let mut state = Self {
            container: document.inbox(),
            document,
            opened,
            selection: Selection::default(),
            history: History::default(),
            show_completed: false,
            visits: Visits::default(),
            navigator: None,
            move_to: None,
            editing: None,
            sidebar: Sidebar::default(),
            saving,
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

    /// The containers from the top level down to the one shown, which the breadcrumbs show
    /// (FR-NAV-03).
    #[must_use]
    pub fn path(&self) -> Vec<ItemId> {
        container::path(&self.document, self.container)
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

    /// The Navigator, while it is open.
    #[must_use]
    pub fn navigator(&self) -> Option<PickerView<'_, Place>> {
        self.navigator.as_ref().map(Picker::view)
    }

    /// The Move to picker, while it is open.
    #[must_use]
    pub fn move_to(&self) -> Option<PickerView<'_, Destination>> {
        self.move_to.as_ref().map(MoveTo::view)
    }

    /// The sidebar, while it shows (FR-VIEW-10).
    #[must_use]
    pub fn sidebar(&self) -> Option<SidebarView> {
        self.sidebar.view(sidebar::entries(&self.document))
    }

    /// Whether the sidebar has the keys, so that commands and the title edited are its own.
    #[must_use]
    pub fn sidebar_has_keys(&self) -> bool {
        self.sidebar.has_keys()
    }

    /// The item edited, which of its texts and the text edited, in edit mode: an entry's item
    /// while the sidebar has the keys, else a row's item or the container shown, whose title and
    /// description head the pane.
    #[must_use]
    pub fn editing(&self) -> Option<(ItemId, Part, &TextField)> {
        self.editing
            .as_ref()
            .map(|editing| (editing.id(), editing.part(), editing.field()))
    }

    /// Moves the focus as `motion` says, keeping what is picked.
    pub(super) fn move_focus(&mut self, motion: Focus) {
        let target = focus::target(&self.rows(), self.selection.focused(), motion);
        self.selection.focus(target);
    }

    /// Moves the focus a row `direction`, picking the rows from where the run of extensions
    /// began to it (FR-OPS-01).
    pub(super) fn extend_selection(&mut self, direction: Direction) {
        let rows = self.rows();
        let motion = match direction {
            Direction::Up => Focus::Previous,
            Direction::Down => Focus::Next,
        };
        let target = focus::target(&rows, self.selection.focused(), motion);
        self.selection.extend(&rows, target);
    }

    /// Picks the focused row, or unpicks it when it is picked (FR-OPS-01).
    pub(super) fn toggle_picked(&mut self) {
        self.selection.toggle();
    }

    /// Picks every row (FR-OPS-01).
    pub(super) fn select_all(&mut self) {
        self.selection.select_all(&self.rows());
    }

    /// Picks nothing, so that commands take the focused row again (FR-OPS-01).
    pub(super) fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// The selected items without those inside another of them, which go along with it, in the
    /// order of the rows: what a command that moves or takes items acts on.
    fn targets(&self) -> Vec<ItemId> {
        container::outermost_rows(&self.document, self.selection.items(&self.rows()))
    }

    /// Completes the selected items on `today`, or uncompletes them when all are completed, as
    /// one step to undo, moving the focus on when its row hides.
    pub(super) fn toggle_completed(&mut self, today: Date) {
        let items = self.selection.items(&self.rows());
        let transaction = completion::toggle(&self.document, items, today);
        self.change(transaction);
    }

    /// Moves the selected items to the Trash on `today`, as one step to undo, moving the focus on
    /// from the rows that leave.
    pub(super) fn delete(&mut self, today: Date) {
        let transaction = trash::delete(&self.document, self.targets(), today);
        self.change(transaction);
    }

    /// Puts a copy of each selected item, with its contents, right after it, as one step to undo,
    /// focusing the copy of the last one the pane shows.
    pub(super) fn duplicate(&mut self) {
        let taken = self.targets();
        let (transaction, last) = clipboard::duplicate(&mut self.document, &taken);
        self.change_focusing(transaction, last.or(self.selection.focused()));
    }

    /// Asks for the selected items, with their contents, to go on the clipboard; nothing when no
    /// project, section or task is selected.
    pub(super) fn copy(&self) -> Vec<Effect> {
        let taken = self.targets();
        if taken.is_empty() {
            return Vec::new();
        }
        vec![Effect::SetClipboard(clipboard::branches(
            &self.document,
            &taken,
        ))]
    }

    /// Copies the selected items and takes them out of the document, not into the Trash, as one
    /// step to undo, moving the focus on from the rows that leave.
    pub(super) fn cut(&mut self) -> Vec<Effect> {
        let effects = self.copy();
        let taken = self.targets();
        let transaction = clipboard::cut(&self.document, &taken);
        self.change(transaction);
        effects
    }

    /// Pastes `branches` on `side` of the focused row, as one step to undo, focusing the last
    /// one pasted the pane shows. Nothing is pasted when one of them can go nowhere near, or
    /// while the sidebar has the keys.
    pub(super) fn paste(&mut self, side: Side, branches: &[Branch]) {
        if self.sidebar.has_keys() {
            return;
        }
        let target = clipboard::Target {
            container: self.container,
            show_completed: self.show_completed,
            focused: self.selection.focused(),
            side,
        };
        if let Some((transaction, shown)) = clipboard::paste(&mut self.document, target, branches) {
            self.change_focusing(transaction, shown.or(self.selection.focused()));
        }
    }

    /// Opens the Move to picker for the selected items, closing the Navigator. Nothing opens when
    /// no project, section or task is selected.
    pub(super) fn open_move_to(&mut self) {
        self.navigator = None;
        self.move_to = MoveTo::open(&self.document, self.selection.items(&self.rows()));
    }

    /// Applies `input` to the open Move to picker: choosing a destination closes it and moves the
    /// selected items there as one step to undo, the pane staying where it is and the focus
    /// moving on from the rows that leave, as Things 3 does. Nothing happens while it is closed.
    pub(super) fn move_to_input(&mut self, input: PickerInput) {
        let Some(picked) = self
            .move_to
            .as_mut()
            .and_then(|move_to| move_to.input(input))
        else {
            return;
        };
        let move_to = self.move_to.take();
        if let (Picked::Chosen(destination), Some(move_to)) = (picked, move_to) {
            let transaction = move_to.transaction(&self.document, destination);
            self.change(transaction);
        }
    }

    /// Moves the selected items one step `direction` among their rows, as one step to undo,
    /// keeping the focus where it is (FR-OPS-02, FR-OPS-10).
    pub(super) fn move_item(&mut self, direction: Direction) {
        let (container, show_completed) = (self.container, self.show_completed);
        let transaction =
            moving::shift_all(&self.document, &self.targets(), direction, |document| {
                container::rows(document, container, show_completed)
            });
        self.change(transaction);
    }

    /// Indents the selected items into the item of their kind above them, as one step to undo,
    /// focusing the item the focused one went into, since what it holds is not shown; the status
    /// line says why when the hierarchy refuses every one (FR-OPS-03).
    pub(super) fn indent(&mut self) {
        match moving::indent_all(&self.document, &self.targets()) {
            Ok((transaction, moved)) => {
                let focused = self.selection.focused();
                let into = moved.iter().find(|&&(id, _)| Some(id) == focused);
                self.change_focusing(transaction, into.map_or(focused, |&(_, into)| Some(into)));
            }
            Err(refusal) => self.notify(Notice::Refused(refusal)),
        }
    }

    /// Outdents the selected items out of the item of their kind they are in, as one step to
    /// undo, moving the focus on when it leaves the rows; the status line says why when the
    /// hierarchy refuses every one (FR-OPS-03).
    pub(super) fn outdent(&mut self) {
        match moving::outdent_all(&self.document, &self.targets()) {
            Ok(transaction) => self.change(transaction),
            Err(refusal) => self.notify(Notice::Refused(refusal)),
        }
    }

    /// Starts editing the focused row's title, picking nothing. Nothing happens unless a row is
    /// focused.
    pub(super) fn edit_title(&mut self) {
        let editing = self
            .selection
            .focused()
            .and_then(|id| Editing::existing(&self.document, id, self.selection.clone()));
        self.start_editing(editing);
    }

    /// Enters edit mode with `editing`, if there is one, picking nothing, since what is picked
    /// is not what is edited.
    fn start_editing(&mut self, editing: Option<Editing>) {
        if editing.is_some() {
            self.selection.clear();
        }
        self.editing = editing;
    }

    /// Starts editing the description of the focused item or of the container shown, as `whose`
    /// says, picking nothing. Nothing happens with nothing focused.
    pub(super) fn edit_description(&mut self, whose: Whose) {
        let id = match whose {
            Whose::Focused => self.selection.focused(),
            Whose::Shown => Some(self.container),
        };
        let editing =
            id.and_then(|id| Editing::description(&self.document, id, self.selection.clone()));
        self.start_editing(editing);
    }

    /// Adds an empty task on `side` of the focused row, focuses it and starts editing its title.
    /// Nothing is added to a container whose contents are hidden for being completed
    /// (FR-ORG-09), where it would not show.
    pub(super) fn new_task(&mut self, side: Side) {
        if self.document.conceals(self.container) {
            return;
        }
        let place = adding::place(
            &self.document,
            self.container,
            &self.rows(),
            self.selection.focused(),
            side,
        );
        let (parent, index) = place;
        let editing = Editing::new_item(
            &mut self.document,
            ItemKind::Task,
            (Some(parent), index),
            self.selection.clone(),
        );
        self.selection.focus(Some(editing.id()));
        self.start_editing(Some(editing));
    }

    /// Adds an empty section, project or area, as `kind` says, on `side` of the focus as near as
    /// it can go, and starts editing its title: in its row when the container shown lists it, else
    /// in the pane's heading, the new item opened. Nothing is added where the kind can go nowhere
    /// near, as a section from the Inbox, or inside a completed container (FR-ORG-09).
    pub(super) fn new_container(&mut self, kind: ItemKind, side: Side) {
        let Some((parent, index)) = adding::container_place(
            &self.document,
            self.container,
            &self.rows(),
            self.selection.focused(),
            kind,
            side,
        ) else {
            return;
        };
        if parent.is_some_and(|parent| self.document.conceals(parent)) {
            return;
        }
        let editing = Editing::new_item(
            &mut self.document,
            kind,
            (parent, index),
            self.selection.clone(),
        );
        let id = editing.id();
        if parent == Some(self.container) {
            self.selection.focus(Some(id));
        } else {
            self.open(id, None);
        }
        self.start_editing(Some(editing));
    }

    /// Starts editing the title of the container shown, in the pane's heading. The Inbox keeps
    /// its name (FR-STORE-08).
    pub(super) fn rename_container(&mut self) {
        let editing = Editing::existing(&self.document, self.container, self.selection.clone());
        self.start_editing(editing);
    }

    /// Moves the container shown to the Trash on `today`, as one step to undo; the pane then
    /// shows the nearest container above it. The Inbox stays.
    pub(super) fn delete_container(&mut self, today: Date) {
        let transaction = trash::delete(&self.document, [self.container], today);
        self.change(transaction);
    }

    /// Applies `input` to the edited title. Nothing happens outside edit mode.
    pub(super) fn edit_input(&mut self, input: EditInput) {
        let Some(editing) = self.editing.as_mut() else {
            return;
        };
        match input {
            EditInput::Text(edit) => editing.input(&mut self.document, edit),
            EditInput::Next => {
                let row = editing.id() != self.container && !self.sidebar.has_keys();
                let next = row
                    && editing.part() == Part::Title
                    && matches!(editing.kind(), ItemKind::Task | ItemKind::Section);
                if self.finish_editing() && next {
                    self.new_task(Side::Below);
                }
            }
            EditInput::Done => {
                self.finish_editing();
            }
        }
    }

    /// Ends edit mode, making the edit one step to undo, and returns whether the item edited is
    /// still there: a new item left empty is not, and when it was opened to be edited, the pane
    /// goes back to where it was. Outside edit mode nothing happens.
    pub(super) fn finish_editing(&mut self) -> bool {
        let Some(editing) = self.editing.take() else {
            return false;
        };
        let (id, started) = (editing.id(), editing.revision());
        let in_sidebar = self.sidebar.has_keys();
        let heading = !in_sidebar && id == self.container;
        let (transaction, before) = editing.finish(&mut self.document);
        if transaction.ops().is_empty() {
            self.saving.unchanged(started, self.document.revision());
        }
        let mut after = self.selection.clone();
        if !heading && !in_sidebar {
            after.focus(Some(id));
        }
        self.history
            .commit(&mut self.document, transaction, before.clone(), |_| after)
            .expect("an edit's transaction is made from the document as it was");
        let kept = self.document.node(id).is_some();
        if in_sidebar {
            self.sidebar.finish(kept);
        }
        if !kept {
            self.selection = before;
            if heading {
                self.go(Go::Back);
            }
        }
        kept
    }

    /// Applies `transaction` as one step to undo, moving the focus on when its row leaves.
    fn change(&mut self, transaction: Transaction) {
        self.change_focusing(transaction, self.selection.focused());
    }

    /// Applies `transaction` as one step to undo, then focuses `focused`, or the row nearest it
    /// when it is not a row.
    fn change_focusing(&mut self, transaction: Transaction, focused: Option<ItemId>) {
        let before = self.rows();
        let (container, show_completed) = (self.container, self.show_completed);
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
        if focused != self.selection.focused() {
            self.selection.focus(focused);
        }
        self.settle(&before);
    }

    /// Shows the sidebar with the keys, focused on the entry of the container shown or the
    /// closest one above it, or hides it when it has the keys, or gives it the keys.
    pub(super) fn toggle_sidebar(&mut self) {
        let entries = sidebar::entries(&self.document);
        let near = self
            .path()
            .into_iter()
            .rev()
            .find(|&id| entries.iter().any(|place| place.id == id));
        self.sidebar.toggle(near);
    }

    /// Gives the keys to the sidebar while it shows, for `true`, or to the pane.
    pub(super) fn focus_sidebar(&mut self, keys: bool) {
        self.sidebar.focus(keys);
    }

    /// Keeps the sidebar's focus on an entry once a message may have changed the tree.
    pub(super) fn settle_sidebar(&mut self) {
        self.sidebar.settle(&sidebar::entries(&self.document));
    }

    /// Gives `command` to the sidebar on `today` while it has the keys, and returns whether it
    /// took it. It acts on its focused entry: moving the focus, renaming, adding a project, as
    /// `o` and `O` do since it lists no tasks, or an area, deleting, moving, indenting and
    /// outdenting, each one step to undo, and opening the entry in the pane. Quitting, undo and
    /// redo, the Navigator, the sidebar's own commands, going home, back and forward and the
    /// conflict prompt's commands pass to the app; every other command does nothing.
    pub(super) fn sidebar_command(&mut self, command: Command, today: Date) -> bool {
        let passes = matches!(
            command,
            Command::Quit
                | Command::Undo
                | Command::Redo
                | Command::Navigator
                | Command::Sidebar
                | Command::FocusSidebar
                | Command::FocusPane
                | Command::Go(Go::Home | Go::Back | Go::Forward)
                | Command::KeepMine
                | Command::TakeTheirs
                | Command::ShowDiff
        );
        if passes || !self.sidebar.has_keys() {
            return false;
        }
        let entries = sidebar::entries(&self.document);
        let focused = self.sidebar.focused();
        let apply = |state: &mut Self, result: Result<Transaction, moving::Refusal>| match result {
            Ok(transaction) => state.change(transaction),
            Err(refusal) => state.notify(Notice::Refused(refusal)),
        };
        match (command, focused) {
            (Command::Focus(motion), _) => self.sidebar.move_focus(&entries, motion),
            (Command::NewTask(side) | Command::NewProject(side), _) => {
                self.sidebar_new(&entries, ItemKind::Project, side);
            }
            (Command::NewArea(side), _) => self.sidebar_new(&entries, ItemKind::Area, side),
            (Command::EditTitle, Some(id)) => {
                self.editing = Editing::existing(&self.document, id, self.selection.clone());
            }
            (Command::Delete, Some(id)) => {
                self.change(trash::delete(&self.document, [id], today));
            }
            (Command::Move(direction), Some(id)) => {
                self.change(sidebar::shift(&self.document, id, direction));
            }
            (Command::Indent, Some(id)) => apply(self, sidebar::indent(&self.document, id)),
            (Command::Outdent, Some(id)) => apply(self, sidebar::outdent(&self.document, id)),
            (Command::Go(Go::Into), Some(id)) => self.open(id, None),
            _ => {}
        }
        true
    }

    /// Adds an empty item of `kind` from the sidebar's focused entry among `entries`, on `side`
    /// of it, focuses it and starts editing its title in the sidebar.
    fn sidebar_new(&mut self, entries: &[Place], kind: ItemKind, side: Side) {
        let focused = self.sidebar.focused();
        let place = sidebar::place(&self.document, entries, focused, kind, side)
            .expect("the sidebar focuses one of its entries, beside which a project or area goes");
        let editing = Editing::new_item(&mut self.document, kind, place, self.selection.clone());
        self.sidebar.focus_new(editing.id());
        self.editing = Some(editing);
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

    /// Keeps the focus on a row now that the rows `before` have changed, and when the container
    /// shown no longer opens, having gone to the Trash or into a completed item, shows the
    /// nearest one above it that does, else the Inbox, focused where it was.
    fn settle(&mut self, before: &[Row]) {
        if !container::opens(&self.document, self.container) {
            let path = self.path();
            let document = &self.document;
            let visit = path
                .array_windows()
                .rev()
                .find(|&&[parent, _]| container::opens(document, parent))
                .map_or(
                    Visit {
                        container: document.inbox(),
                        focused: None,
                    },
                    |&[parent, child]| Visit {
                        container: parent,
                        focused: Some(child),
                    },
                );
            self.arrive(visit);
            return;
        }
        let rows = self.rows();
        let focused = focus::kept(before, &rows, self.selection.focused());
        if focused != self.selection.focused() {
            self.selection.focus(focused);
        }
        self.selection.keep(&rows, Some);
    }

    /// Shows the place `to` says.
    pub(super) fn go(&mut self, to: Go) {
        let document = &self.document;
        match to {
            Go::Into => {
                let focused = self.selection.focused();
                let row = self
                    .rows()
                    .into_iter()
                    .find(|row| Some(row.id()) == focused);
                if let Some(Row::Section(id) | Row::Project(id)) = row {
                    self.open(id, None);
                }
            }
            Go::Up => {
                let path = self.path();
                if let [.., parent, _] = path.as_slice() {
                    self.open(*parent, Some(self.container));
                }
            }
            Go::Home => self.open(document.inbox(), None),
            Go::Back => {
                let visit = self
                    .visits
                    .back(self.visit(), |id| container::opens(document, id));
                if let Some(visit) = visit {
                    self.arrive(visit);
                }
            }
            Go::Forward => {
                let visit = self
                    .visits
                    .forward(self.visit(), |id| container::opens(document, id));
                if let Some(visit) = visit {
                    self.arrive(visit);
                }
            }
        }
    }

    /// Opens the Navigator on where the pane is, closing Move to, or closes it when it is open.
    pub(super) fn toggle_navigator(&mut self) {
        self.move_to = None;
        self.navigator = match self.navigator {
            Some(_) => None,
            None => Some(navigator::open(&self.document, self.container)),
        };
    }

    /// Applies `input` to the open Navigator: choosing a place closes it and opens the place.
    /// Nothing happens while it is closed.
    pub(super) fn navigator_input(&mut self, input: PickerInput) {
        let Some(picked) = self
            .navigator
            .as_mut()
            .and_then(|picker| picker.input(input))
        else {
            return;
        };
        self.navigator = None;
        if let Picked::Chosen(place) = picked {
            self.open(place.id, None);
        }
    }

    /// Where the pane is.
    fn visit(&self) -> Visit {
        Visit {
            container: self.container,
            focused: self.selection.focused(),
        }
    }

    /// Shows `container`, focused on `focused` or its first row, so that Back returns to where
    /// the pane was. Opening the container shown changes nothing.
    fn open(&mut self, container: ItemId, focused: Option<ItemId>) {
        if container != self.container {
            self.visits.leave(self.visit());
            self.arrive(Visit { container, focused });
        }
    }

    /// Shows the container of `visit`, focused on its item when that is a row there and on the
    /// first row otherwise, with nothing picked.
    fn arrive(&mut self, visit: Visit) {
        self.selection.clear();
        self.container = visit.container;
        let rows = self.rows();
        let focused = visit
            .focused
            .filter(|&id| rows.iter().any(|row| row.id() == id))
            .or_else(|| rows.first().map(|row| row.id()));
        self.selection.focus(focused);
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::TextEdit;
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
        assert_eq!(state.selection().items(&state.rows()), [last]);
        Ok(())
    }

    /// A state on the Inbox, holding task `inbox_task`, beside area `work` holding task `loose`,
    /// section `section` with task `in_section` and project `site` with task `in_site`.
    struct Fixture {
        state: State,
        inbox_task: ItemId,
        work: ItemId,
        loose: ItemId,
        section: ItemId,
        in_section: ItemId,
        site: ItemId,
        in_site: ItemId,
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let add = |document: &mut Document, parent, kind, title| {
            document.push(Some(parent), Node::Item(Item::new(kind, title)))
        };
        let inbox = document.inbox();
        let inbox_task = add(&mut document, inbox, ItemKind::Task, "i")?;
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        let loose = add(&mut document, work, ItemKind::Task, "loose")?;
        let section = add(&mut document, work, ItemKind::Section, "S")?;
        let in_section = add(&mut document, section, ItemKind::Task, "s")?;
        let site = add(&mut document, work, ItemKind::Project, "Site")?;
        let in_site = add(&mut document, site, ItemKind::Task, "p")?;
        Ok(Fixture {
            state: State::new(document, Opened::Demo),
            inbox_task,
            work,
            loose,
            section,
            in_section,
            site,
            in_site,
        })
    }

    /// Opens `place` through the Navigator.
    fn navigate(state: &mut State, place: &str) {
        state.toggle_navigator();
        for c in place.chars() {
            state.navigator_input(PickerInput::Edit(TextEdit::Insert(c)));
        }
        state.navigator_input(PickerInput::Choose);
    }

    fn at(state: &State) -> (ItemId, Option<ItemId>) {
        (state.container(), state.selection().focused())
    }

    // FR-NAV-02, FR-NAV-03
    #[test]
    fn opening_a_section_or_project_shows_it_and_going_up_comes_back_to_it()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            loose,
            section,
            in_section,
            site,
            in_site,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        assert_eq!(at(&state), (work, Some(loose)));
        assert_eq!(state.path(), [work]);
        state.go(Go::Into);
        assert_eq!(at(&state), (work, Some(loose)), "a task does not open");
        state.move_focus(Focus::NextSection);
        state.go(Go::Into);
        assert_eq!(at(&state), (section, Some(in_section)));
        assert_eq!(state.path(), [work, section]);
        state.go(Go::Up);
        assert_eq!(at(&state), (work, Some(section)));
        state.move_focus(Focus::Last);
        state.go(Go::Into);
        assert_eq!(at(&state), (site, Some(in_site)));
        state.go(Go::Up);
        state.go(Go::Up);
        assert_eq!(at(&state), (work, Some(site)), "nothing is above an area");
        Ok(())
    }

    // FR-NAV-02
    #[test]
    fn home_is_the_inbox() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            work,
            ..
        } = fixture()?;
        let inbox = state.document().inbox();
        navigate(&mut state, "Work");
        assert_eq!(state.container(), work);
        state.go(Go::Home);
        assert_eq!(at(&state), (inbox, Some(inbox_task)));
        state.go(Go::Up);
        assert_eq!(state.container(), inbox, "nothing is above the Inbox");
        Ok(())
    }

    // FR-NAV-04
    #[test]
    fn back_and_forward_restore_the_place_and_its_focus() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            work,
            section,
            in_section,
            site,
            ..
        } = fixture()?;
        let inbox = state.document().inbox();
        navigate(&mut state, "Work");
        state.move_focus(Focus::NextSection);
        state.go(Go::Into);
        state.go(Go::Back);
        assert_eq!(at(&state), (work, Some(section)));
        state.go(Go::Back);
        assert_eq!(at(&state), (inbox, Some(inbox_task)));
        state.go(Go::Back);
        assert_eq!(state.container(), inbox, "nothing is further back");
        state.go(Go::Forward);
        state.go(Go::Forward);
        assert_eq!(at(&state), (section, Some(in_section)));
        state.go(Go::Forward);
        assert_eq!(state.container(), section, "nothing is further ahead");
        // A place gone to the Trash is passed over, and its focus, when gone, falls to the first
        // row.
        state.go(Go::Home);
        navigate(&mut state, "Site");
        state.go(Go::Home);
        let mut trashed = Item::new(ItemKind::Section, "S");
        trashed.deleted = Some(jiff::civil::date(2026, 10, 1));
        state.document.set(section, trashed)?;
        state.go(Go::Back);
        assert_eq!(state.container(), site);
        state.go(Go::Back);
        assert_eq!(state.container(), inbox);
        Ok(())
    }

    const TODAY: Date = jiff::civil::date(2026, 10, 14);

    fn title(state: &State, id: ItemId) -> Option<&str> {
        Some(state.document().node(id)?.item()?.title.as_str())
    }

    fn type_text(state: &mut State, text: &str) {
        for c in text.chars() {
            state.edit_input(EditInput::Text(TextEdit::Insert(c)));
        }
    }

    // FR-EDIT-01, FR-EDIT-02, FR-EDIT-06
    #[test]
    fn a_title_edited_in_place_is_one_step_to_undo() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            ..
        } = fixture()?;
        state.edit_title();
        assert_eq!(state.editing().map(|(id, ..)| id), Some(inbox_task));
        type_text(&mut state, "dea");
        assert_eq!(title(&state, inbox_task), Some("idea"));
        state.edit_input(EditInput::Done);
        assert!(state.editing().is_none());
        assert_eq!(title(&state, inbox_task), Some("idea"));
        state.undo();
        assert_eq!(title(&state, inbox_task), Some("i"));
        state.redo();
        assert_eq!(title(&state, inbox_task), Some("idea"));
        state.edit_input(EditInput::Done);
        assert!(
            state.editing().is_none(),
            "outside edit mode it does nothing"
        );
        assert!(!state.finish_editing());
        Ok(())
    }

    // FR-EDIT-01, FR-ORG-06
    #[test]
    fn the_focused_row_is_edited_a_section_heading_too() -> Result<(), PlacementError> {
        let Fixture {
            mut state, section, ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.move_focus(Focus::NextSection);
        state.edit_title();
        assert_eq!(state.editing().map(|(id, ..)| id), Some(section));
        type_text(&mut state, "ection");
        state.edit_input(EditInput::Done);
        assert_eq!(title(&state, section), Some("Section"));
        state.go(Go::Home);
        let mut document = state.document().clone();
        document.push(None, Node::Item(Item::new(ItemKind::Project, "Empty")))?;
        state = State::new(document, Opened::Demo);
        navigate(&mut state, "Empty");
        assert_eq!(state.selection().focused(), None);
        state.edit_title();
        assert!(
            state.editing().is_none(),
            "with nothing focused nothing is edited"
        );
        Ok(())
    }

    fn description(state: &State, id: ItemId) -> Option<&str> {
        Some(state.document().node(id)?.item()?.description.as_str())
    }

    // FR-ORG-10, FR-EDIT-03, FR-EDIT-06
    #[test]
    fn the_focused_items_or_the_shown_containers_description_is_edited_in_place()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            loose,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        let rows = state.rows();
        state.edit_description(Whose::Focused);
        assert_eq!(
            state.editing().map(|(id, part, _)| (id, part)),
            Some((loose, Part::Description))
        );
        type_text(&mut state, "a\nb");
        state.edit_input(EditInput::Next);
        assert!(state.editing().is_none(), "the next task is for titles");
        assert_eq!(state.rows(), rows, "no task was added");
        assert_eq!(description(&state, loose), Some("a\nb"));
        state.move_focus(Focus::Last);
        let last = state.selection().focused();
        state.edit_description(Whose::Shown);
        assert_eq!(state.editing().map(|(id, ..)| id), Some(work));
        type_text(&mut state, "w");
        state.edit_input(EditInput::Done);
        assert_eq!(description(&state, work), Some("w"));
        assert_eq!(state.selection().focused(), last);
        state.undo();
        assert_eq!(description(&state, work), Some(""));
        state.redo();
        assert_eq!(
            state.selection().focused(),
            last,
            "redo keeps the focus where it was"
        );
        state.undo();
        state.undo();
        assert_eq!(description(&state, loose), Some(""));
        Ok(())
    }

    // FR-ORG-10
    #[test]
    fn with_nothing_focused_no_description_is_edited() {
        let mut state = State::new(Document::new(), Opened::Demo);
        state.edit_description(Whose::Focused);
        assert!(state.editing().is_none());
    }

    // FR-EDIT-05, FR-EDIT-06
    #[test]
    fn enter_adds_tasks_one_after_another_until_one_is_left_empty() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            ..
        } = fixture()?;
        let inbox = state.document().inbox();
        state.new_task(Side::Below);
        type_text(&mut state, "second");
        state.edit_input(EditInput::Next);
        type_text(&mut state, "third");
        state.edit_input(EditInput::Next);
        state.edit_input(EditInput::Next);
        assert!(state.editing().is_none());
        let titles: Vec<_> = state
            .document()
            .children(inbox)
            .iter()
            .filter_map(|&id| title(&state, id))
            .collect();
        assert_eq!(titles, ["i", "second", "third"]);
        let third = state.document().children(inbox).last().copied();
        assert_eq!(state.selection().focused(), third);
        state.undo();
        assert_eq!(state.document().children(inbox).len(), 2);
        state.undo();
        assert_eq!(state.document().children(inbox), [inbox_task]);
        assert_eq!(state.selection().focused(), Some(inbox_task));
        Ok(())
    }

    // FR-EDIT-05
    #[test]
    fn a_new_task_left_empty_leaves_no_trace() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            ..
        } = fixture()?;
        let before = state.clone();
        state.new_task(Side::Above);
        let new = state.editing().map(|(id, ..)| id);
        assert_eq!(state.selection().focused(), new);
        assert_eq!(state.rows().first().map(|row| row.id()), new);
        state.edit_input(EditInput::Done);
        assert_eq!(
            state.document().children(state.document().inbox()),
            [inbox_task]
        );
        assert_eq!(state.selection().focused(), Some(inbox_task));
        state.undo();
        assert_eq!(title(&state, inbox_task), title(&before, inbox_task));
        assert_eq!(state.rows(), before.rows());
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-09
    #[test]
    fn nothing_is_added_where_a_completed_container_hides_it() -> Result<(), PlacementError> {
        let Fixture {
            mut state, site, ..
        } = fixture()?;
        navigate(&mut state, "Site");
        let mut done = Item::new(ItemKind::Project, "Site");
        done.completed = Some(crate::domain::Completion::Undated);
        state.document.set(site, done)?;
        let before = state.document().clone();
        state.new_task(Side::Below);
        assert!(state.editing().is_none());
        assert_eq!(state.document(), &before);
        Ok(())
    }

    // FR-OPS-05, OQ-19
    #[test]
    fn deleting_moves_the_focus_on_and_undo_brings_it_back() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            loose,
            section,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.delete(TODAY);
        let deleted = state
            .document()
            .node(loose)
            .and_then(Node::item)
            .and_then(|item| item.deleted);
        assert_eq!(deleted, Some(TODAY));
        assert_eq!(state.selection().focused(), Some(section));
        state.undo();
        assert_eq!(state.selection().focused(), Some(loose));
        Ok(())
    }

    // FR-OPS-05, FR-NAV-02
    #[test]
    fn a_shown_container_gone_to_the_trash_gives_way_to_the_nearest_one_that_opens()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            loose,
            section,
            in_section,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.move_focus(Focus::NextSection);
        state.delete(TODAY);
        state.undo();
        state.go(Go::Into);
        assert_eq!(at(&state), (section, Some(in_section)));
        state.redo();
        assert_eq!(
            at(&state),
            (work, Some(loose)),
            "the section is gone, so its area opens on its first row"
        );
        // With everything above gone too, home is left.
        state.undo();
        state.go(Go::Into);
        let mut gone = Item::new(ItemKind::Area, "Work");
        gone.deleted = Some(TODAY);
        state.document.set(work, gone)?;
        state.redo();
        assert_eq!(state.container(), state.document().inbox());
        Ok(())
    }

    /// Moves the selection through Move to to the destination `query` finds first.
    fn move_to(state: &mut State, query: &str) {
        state.open_move_to();
        for c in query.chars() {
            state.move_to_input(PickerInput::Edit(TextEdit::Insert(c)));
        }
        state.move_to_input(PickerInput::Choose);
    }

    // FR-OPS-04, FR-EDIT-06
    #[test]
    fn moving_a_task_away_keeps_the_pane_and_moves_the_focus_on() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            loose,
            section,
            site,
            in_site,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        move_to(&mut state, "Site");
        assert!(state.move_to().is_none(), "choosing closes it");
        assert_eq!(state.document().children(site), [in_site, loose]);
        assert_eq!(at(&state), (work, Some(section)));
        state.undo();
        assert_eq!(at(&state), (work, Some(loose)));
        assert_eq!(state.document().position(loose), Some((Some(work), 0)));
        // Into a section of the container shown, the task stays a row and keeps the focus.
        move_to(&mut state, "S");
        assert_eq!(state.document().position(loose), Some((Some(section), 1)));
        assert_eq!(at(&state), (work, Some(loose)));
        Ok(())
    }

    // FR-OPS-04, FR-VIEW-07
    #[test]
    fn move_to_closes_without_moving_and_gives_way_to_the_navigator() -> Result<(), PlacementError>
    {
        let Fixture { mut state, .. } = fixture()?;
        let before = state.clone();
        state.move_to_input(PickerInput::Choose);
        assert_eq!(state, before, "a closed picker takes no input");
        state.open_move_to();
        assert!(state.move_to().is_some());
        state.move_to_input(PickerInput::Next);
        state.move_to_input(PickerInput::Dismiss);
        assert!(state.move_to().is_none());
        assert_eq!(
            state.document(),
            before.document(),
            "dismissing moves nothing"
        );
        state.open_move_to();
        state.toggle_navigator();
        assert!(state.move_to().is_none() && state.navigator().is_some());
        state.open_move_to();
        assert!(state.move_to().is_some() && state.navigator().is_none());
        Ok(())
    }

    // FR-VIEW-07
    #[test]
    fn the_navigator_opens_closes_and_goes_where_it_is_told() -> Result<(), PlacementError> {
        let Fixture {
            mut state, work, ..
        } = fixture()?;
        let inbox = state.document().inbox();
        assert!(state.navigator().is_none());
        state.navigator_input(PickerInput::Choose);
        assert_eq!(
            state.container(),
            inbox,
            "a closed Navigator takes no input"
        );
        state.toggle_navigator();
        assert!(
            state
                .navigator()
                .is_some_and(|view| view.entries.len() == 3)
        );
        state.navigator_input(PickerInput::Next);
        assert!(state.navigator().is_some_and(|view| view.focused == 1));
        state.toggle_navigator();
        assert!(state.navigator().is_none());
        state.toggle_navigator();
        state.navigator_input(PickerInput::Dismiss);
        assert!(state.navigator().is_none());
        assert_eq!(state.container(), inbox, "dismissing goes nowhere");
        navigate(&mut state, "Inbox");
        state.go(Go::Back);
        assert_eq!(
            state.container(),
            inbox,
            "choosing where the pane is records no visit"
        );
        navigate(&mut state, "Work");
        assert_eq!(state.container(), work);
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn a_new_section_is_edited_in_its_row_and_enter_adds_its_first_task()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            section,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.new_container(ItemKind::Section, Side::Below);
        let (new, ..) = state.editing().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(state.container(), work);
        assert_eq!(state.selection().focused(), Some(new));
        assert_eq!(
            state.rows().get(1),
            Some(&Row::Section(new)),
            "the new section comes first among the sections"
        );
        type_text(&mut state, "Next");
        state.edit_input(EditInput::Next);
        let (task, ..) = state.editing().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(state.document().children(new), [task]);
        state.edit_input(EditInput::Done);
        assert_eq!(title(&state, new), Some("Next"));
        assert_eq!(
            state.rows().get(2),
            Some(&Row::Section(section)),
            "the section that was first follows it"
        );
        state.undo();
        assert_eq!(state.document().node(new), None);
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn enter_after_a_project_row_only_ends_editing() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            site,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.move_focus(Focus::Last);
        state.new_container(ItemKind::Project, Side::Above);
        let (new, ..) = state.editing().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(state.rows().last(), Some(&Row::Project(site)));
        type_text(&mut state, "Garden");
        state.edit_input(EditInput::Next);
        assert!(state.editing().is_none());
        assert_eq!(state.selection().focused(), Some(new));
        assert_eq!(state.document().children(work).len(), 4);
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn a_new_area_opens_with_its_title_edited_and_left_empty_the_pane_goes_back()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            work,
            ..
        } = fixture()?;
        let inbox = state.document().inbox();
        state.new_container(ItemKind::Area, Side::Below);
        let (area, ..) = state.editing().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(at(&state), (area, None));
        assert_eq!(state.document().top_level(), [area, work]);
        type_text(&mut state, " ");
        state.edit_input(EditInput::Next);
        assert_eq!(at(&state), (inbox, Some(inbox_task)));
        assert_eq!(state.document().top_level(), [work]);
        state.go(Go::Forward);
        assert_eq!(state.container(), inbox, "the area left empty is not there");
        state.new_container(ItemKind::Area, Side::Above);
        type_text(&mut state, "Home");
        state.edit_input(EditInput::Done);
        let home = state.document().top_level()[0];
        assert_eq!(at(&state), (home, None));
        assert_eq!(title(&state, home), Some("Home"));
        state.go(Go::Back);
        assert_eq!(at(&state), (inbox, Some(inbox_task)));
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn new_containers_go_only_where_they_can_show() -> Result<(), PlacementError> {
        let Fixture {
            mut state, work, ..
        } = fixture()?;
        let before = state.clone();
        state.new_container(ItemKind::Section, Side::Below);
        assert_eq!(state, before, "the Inbox holds no section");
        navigate(&mut state, "Work");
        let mut done = Item::new(ItemKind::Area, "Work");
        done.completed = Some(crate::domain::Completion::Undated);
        state.document.set(work, done)?;
        let before = state.clone();
        for kind in [ItemKind::Section, ItemKind::Project] {
            state.new_container(kind, Side::Below);
            assert_eq!(state, before, "{kind:?} in a completed area");
        }
        state.new_container(ItemKind::Area, Side::Below);
        let (area, ..) = state.editing().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(state.document().top_level(), [work, area]);
        Ok(())
    }

    // FR-ORG-06
    #[test]
    fn the_container_shown_is_renamed_in_place_and_deleted() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            site,
            in_site,
            ..
        } = fixture()?;
        state.rename_container();
        assert!(state.editing().is_none(), "the Inbox keeps its name");
        navigate(&mut state, "Site");
        state.rename_container();
        assert_eq!(state.editing().map(|(id, ..)| id), Some(site));
        type_text(&mut state, "s");
        state.edit_input(EditInput::Next);
        assert!(state.editing().is_none(), "Enter only ends editing");
        assert_eq!(at(&state), (site, Some(in_site)));
        assert_eq!(title(&state, site), Some("Sites"));
        state.delete_container(date(2026, 10, 14));
        assert_eq!(at(&state), (work, state.rows().first().map(|row| row.id())));
        assert!(!state.rows().contains(&Row::Project(site)));
        state.undo();
        assert!(state.rows().contains(&Row::Project(site)));
        Ok(())
    }

    /// A state on the Inbox holding tasks `a`, `b`, `c` and `d`.
    fn four_tasks() -> Result<(State, [ItemId; 4]), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let mut add =
            |title| document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, title)));
        let tasks = [add("a")?, add("b")?, add("c")?, add("d")?];
        Ok((State::new(document, Opened::Demo), tasks))
    }

    fn picked(state: &State) -> Vec<ItemId> {
        state.selection().items(&state.rows())
    }

    // FR-OPS-01, FR-OPS-06, FR-TASK-02, FR-EDIT-06
    #[test]
    fn three_tasks_picked_are_completed_at_once_and_undo_picks_them_again()
    -> Result<(), PlacementError> {
        let (mut state, [a, b, c, d]) = four_tasks()?;
        state.extend_selection(Direction::Down);
        state.extend_selection(Direction::Down);
        assert_eq!(picked(&state), [a, b, c]);
        state.toggle_completed(TODAY);
        assert!([a, b, c].iter().all(|&id| {
            state
                .document()
                .node(id)
                .and_then(Node::item)
                .is_some_and(|it| it.completed.is_some())
        }));
        assert_eq!(state.selection().focused(), Some(d));
        assert_eq!(picked(&state), [d], "the hidden rows are no longer picked");
        state.undo();
        assert_eq!(picked(&state), [a, b, c]);
        assert_eq!(state.selection().focused(), Some(c));
        state.extend_selection(Direction::Up);
        assert_eq!(picked(&state), [a, b]);
        Ok(())
    }

    // FR-OPS-01
    #[test]
    fn rows_are_picked_one_by_one_all_at_once_and_let_go() -> Result<(), PlacementError> {
        let (mut state, [a, b, c, d]) = four_tasks()?;
        state.toggle_picked();
        state.move_focus(Focus::Last);
        state.toggle_picked();
        assert_eq!(picked(&state), [a, d]);
        state.select_all();
        assert_eq!(picked(&state), [a, b, c, d]);
        state.clear_selection();
        assert_eq!(picked(&state), [d]);
        Ok(())
    }

    // FR-OPS-01, FR-EDIT-01
    #[test]
    fn picks_go_when_a_title_is_edited_or_the_pane_shows_another_place()
    -> Result<(), PlacementError> {
        let (mut state, [_, b, ..]) = four_tasks()?;
        state.extend_selection(Direction::Down);
        state.edit_title();
        assert_eq!(state.editing().map(|(id, _, _)| id), Some(b));
        state.finish_editing();
        assert_eq!(picked(&state), [b], "only the edited row");
        state.select_all();
        state.new_task(Side::Below);
        state.edit_input(EditInput::Done);
        assert_eq!(
            picked(&state).len(),
            4,
            "a new task left empty gives back the picks"
        );
        let Fixture {
            mut state, loose, ..
        } = fixture()?;
        state.select_all();
        navigate(&mut state, "Work");
        assert_eq!(picked(&state), [loose], "only the focused row");
        Ok(())
    }

    // FR-OPS-01, FR-OPS-05, FR-OPS-06
    #[test]
    fn a_section_deleted_with_its_task_picked_takes_the_task_along() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            section,
            in_section,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.move_focus(Focus::NextSection);
        state.extend_selection(Direction::Down);
        assert_eq!(picked(&state), [section, in_section]);
        state.delete(TODAY);
        let deleted = |id| {
            state
                .document()
                .node(id)
                .and_then(Node::item)
                .and_then(|item| item.deleted)
        };
        assert_eq!(deleted(section), Some(TODAY));
        assert_eq!(deleted(in_section), None, "it goes inside the section");
        Ok(())
    }

    // FR-OPS-01, FR-OPS-02, FR-OPS-03
    #[test]
    fn picked_items_move_and_indent_together() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            loose,
            section,
            in_section,
            site,
            in_site,
            ..
        } = fixture()?;
        let mut document = state.document().clone();
        let add = |document: &mut Document, title| {
            document.push(Some(work), Node::Item(Item::new(ItemKind::Project, title)))
        };
        let two = add(&mut document, "Two")?;
        let three = add(&mut document, "Three")?;
        state = State::new(document, Opened::Demo);
        navigate(&mut state, "Work");
        state.toggle_picked();
        state.move_focus(Focus::Next);
        state.move_focus(Focus::Next);
        state.toggle_picked();
        state.move_item(Direction::Up);
        assert_eq!(
            state.document().children(work).get(..2),
            Some(&[loose, in_section][..]),
            "the first task stays and the second comes up behind it"
        );
        state.move_item(Direction::Down);
        assert_eq!(state.document().children(section), [loose, in_section]);
        assert_eq!(state.selection().focused(), Some(in_section));
        state.clear_selection();
        state.move_focus(Focus::Last);
        state.extend_selection(Direction::Up);
        state.extend_selection(Direction::Up);
        assert_eq!(picked(&state), [site, two, three]);
        state.indent();
        assert_eq!(
            state.notice(),
            Some(&Notice::Refused(moving::Refusal::NothingAbove))
        );
        state.toggle_picked();
        state.indent();
        assert_eq!(state.document().children(site), [in_site, two, three]);
        assert_eq!(
            state.selection().focused(),
            Some(site),
            "the item the focused one went into"
        );
        state.extend_selection(Direction::Up);
        state.outdent();
        assert_eq!(
            state.notice(),
            Some(&Notice::Refused(moving::Refusal::NotNested))
        );
        state.undo();
        assert_eq!(picked(&state), [two, three]);
        state.indent();
        assert_eq!(
            state.selection().focused(),
            Some(site),
            "the focus, not on an item indented, stays"
        );
        Ok(())
    }
}
