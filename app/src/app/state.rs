//! The application state a frontend renders, changed only by [`update`](super::update).

use std::{mem, path::PathBuf, rc::Rc};

use jiff::civil::Date;

use super::{
    Command, DateInput, DatePromptView, Direction, EditInput, Effect, End, Go, History, Notice,
    Part, Picked, Picker, PickerInput, PickerView, Selected, Selection, Side, SidebarView,
    TextEdit, TextField, Whose, adding,
    attachments::Panel,
    clipboard, completion,
    container::{self, Row},
    converting,
    date_prompt::{self, DatePrompt},
    editing::{Editing, Refused},
    focus::{self, Focus},
    go_to_symbol::Symbol,
    move_to::{Destination, MoveTo},
    moving,
    navigator::{self, Stop},
    pane::{self, Pane, PaneView, listed},
    saving::Saving,
    settings::{Setting, Settings},
    sidebar::{self, Sidebar},
    tags::{Choice, TagPicker, TagsView},
    trash,
    view::{self, Group, Shown, View},
    visits::Visit,
};
use crate::domain::{
    Branch, Document, Item, ItemId, ItemKind, Node, Tag, Transaction, dates::DateKind,
};

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
    /// The local date by the frontend's clock at the latest message, which views list by.
    pub(super) today: Date,
    /// The panes left of the one with the keys, left to right (FR-PANE-01).
    pub(super) left: Vec<Pane>,
    /// The pane with the keys.
    pub(super) pane: Pane,
    /// The panes right of the one with the keys, left to right.
    pub(super) right: Vec<Pane>,
    pub(super) history: History,
    pub(super) navigator: Option<Picker<Stop>>,
    /// The command palette, while it is open (FR-NAV-06).
    pub(super) palette: Option<Picker<Command>>,
    /// Go to Symbol, while it is open (FR-NAV-05).
    pub(super) go_to_symbol: Option<Picker<Symbol>>,
    /// Move to, while its picker is open (FR-OPS-04).
    pub(super) move_to: Option<MoveTo>,
    /// The date prompt, while it is open (FR-DATE-02).
    pub(super) date_prompt: Option<DatePrompt>,
    /// The tag picker, while it is open (FR-TAG-02, FR-TAG-04).
    pub(super) tags: Option<TagPicker>,
    /// The item whose title, description or note is edited, in edit mode (FR-EDIT-01), in the
    /// sidebar while it has the keys.
    editing: Option<Editing>,
    /// The attachments panel, while it is open (FR-ATT-01).
    pub(super) panel: Option<Panel>,
    sidebar: Sidebar,
    /// Whether the shortcuts panel shows (FR-KEY-03).
    pub(super) shortcuts: bool,
    /// The settings the app runs with (FR-SET-03).
    pub(super) settings: Settings,
    /// The settings the session takes where the settings file does not set them (FR-SET-02).
    pub(super) default_settings: Settings,
    /// The setting highlighted in the settings window, while it is open (FR-SET-02).
    pub(super) settings_window: Option<Setting>,
    /// Whether the words the pane searches for are typed (FR-SRCH-01).
    pub(super) searching: bool,
    pub(super) saving: Saving,
}

impl State {
    /// The app showing `document`, opened from `opened` on `today`, at home: the Inbox, its first
    /// row focused (FR-NAV-02), beside the sidebar focused on it without the keys (FR-VIEW-10).
    /// The file holds the document, and every change to it is saved; the demo document is never
    /// saved (FR-STORE-01, FR-STORE-09).
    #[must_use]
    pub fn new(document: Document, opened: Opened, today: Date) -> Self {
        let saving = Saving::new(document.revision(), true, matches!(opened, Opened::File(_)));
        let sidebar = Sidebar::open(Shown::Container(document.inbox()));
        Self {
            left: Vec::new(),
            pane: Pane::new(
                &document,
                today,
                Visit {
                    shown: Shown::Container(document.inbox()),
                    focused: None,
                },
                false,
            ),
            right: Vec::new(),
            today,
            document,
            opened,
            history: History::default(),
            navigator: None,
            palette: None,
            go_to_symbol: None,
            move_to: None,
            date_prompt: None,
            tags: None,
            editing: None,
            panel: None,
            sidebar,
            shortcuts: false,
            settings: Settings::default(),
            default_settings: Settings::default(),
            settings_window: None,
            searching: false,
            saving,
        }
    }

    /// The pane with the keys.
    pub(super) fn pane(&self) -> &Pane {
        &self.pane
    }

    /// The pane with the keys, to change.
    pub(super) fn pane_mut(&mut self) -> &mut Pane {
        &mut self.pane
    }

    /// The panes, left to right, as a frontend draws them (FR-PANE-01).
    pub fn panes(&self) -> impl Iterator<Item = PaneView<'_>> {
        self.left
            .iter()
            .chain([&self.pane])
            .chain(&self.right)
            .map(|pane| PaneView::new(pane, &self.document, self.today))
    }

    /// The position of the pane with the keys among [`panes`](Self::panes); the commands that
    /// act on a pane act on it.
    #[must_use]
    pub fn current_pane(&self) -> usize {
        self.left.len()
    }

    /// The pane with the keys, as a frontend draws it.
    #[must_use]
    pub fn pane_view(&self) -> PaneView<'_> {
        PaneView::new(&self.pane, &self.document, self.today)
    }

    /// The rows every pane lists, left to right, to keep each pane's focus on a row once they
    /// change.
    fn listings(&self) -> Vec<Rc<[Row]>> {
        self.panes().map(|pane| pane.shared_rows()).collect()
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

    /// What the pane shows.
    #[must_use]
    pub fn shown(&self) -> Shown {
        self.pane().shown
    }

    /// The container shown, unless the pane shows a view.
    #[must_use]
    pub fn container(&self) -> Option<ItemId> {
        self.pane().container()
    }

    /// The containers from the top level down to the one shown; none for a view.
    #[must_use]
    pub fn path(&self) -> Vec<ItemId> {
        self.pane().path(&self.document)
    }

    /// The rows the pane lists, in order, those the tag filter keeps.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        self.listed().to_vec()
    }

    /// As [`rows`](Self::rows), shared with the pane rather than copied.
    pub(super) fn listed(&self) -> Rc<[Row]> {
        self.pane().rows(&self.document, self.today)
    }

    /// The groups of the view shown, each under its heading, with the rows the tag filter keeps;
    /// none for a container.
    #[must_use]
    pub fn groups(&self) -> Vec<Group> {
        self.pane().groups(&self.document, self.today).to_vec()
    }

    /// The tags the pane is filtered by, none when it is not (FR-TAG-04).
    #[must_use]
    pub fn tag_filter(&self) -> &[String] {
        &self.pane().filter
    }

    /// Keeps `today`, the local date by the frontend's clock, for the views to list by.
    pub(super) fn set_today(&mut self, today: Date) {
        if today != self.today {
            let before = self.listings();
            self.today = today;
            self.settle(&before);
        }
    }

    /// Whether completed items are shown.
    #[must_use]
    pub fn shows_completed(&self) -> bool {
        self.pane().show_completed
    }

    /// The items the next command acts on.
    #[must_use]
    pub fn selection(&self) -> &Selection {
        &self.pane().selection
    }

    /// The items the message just handled changed, put in or took out, with everything inside
    /// those put in or taken out, which a frontend shows (FR-UI-07).
    #[must_use]
    pub fn changed(&self) -> &[ItemId] {
        self.history.changed()
    }

    /// The Navigator, while it is open.
    #[must_use]
    pub fn navigator(&self) -> Option<PickerView<'_, Stop>> {
        self.navigator.as_ref().map(Picker::view)
    }

    /// The Move to picker, while it is open.
    #[must_use]
    pub fn move_to(&self) -> Option<PickerView<'_, Destination>> {
        self.move_to.as_ref().map(MoveTo::view)
    }

    /// The date prompt, while it is open (FR-DATE-02).
    #[must_use]
    pub fn date_prompt(&self) -> Option<DatePromptView<'_>> {
        self.date_prompt.as_ref().map(DatePrompt::view)
    }

    /// The tag picker, while it is open.
    #[must_use]
    pub fn tags(&self) -> Option<TagsView<'_>> {
        self.tags.as_ref().map(TagPicker::view)
    }

    /// The sidebar, while it shows (FR-VIEW-10).
    #[must_use]
    pub fn sidebar(&self) -> Option<SidebarView> {
        self.sidebar.view(&self.document)
    }

    /// Whether the sidebar has the keys, so that commands and the title edited are its own.
    #[must_use]
    pub fn sidebar_has_keys(&self) -> bool {
        self.sidebar.has_keys()
    }

    /// The item edited, which of its texts and the text edited, in edit mode: the note of the
    /// item the attachments panel is on, an entry's item while the sidebar has the keys, else a
    /// row's item or the container shown, whose title and description head the pane.
    #[must_use]
    pub fn editing(&self) -> Option<(ItemId, Part, &TextField)> {
        self.editing
            .as_ref()
            .map(|editing| (editing.id(), editing.part(), editing.field()))
    }

    /// Moves the focus as `motion` says, keeping what is picked.
    pub(super) fn move_focus(&mut self, motion: Focus) {
        let (document, today) = (&self.document, self.today);
        self.pane.move_focus(document, today, motion);
    }

    /// Moves the focus a row `direction`, picking the rows from where the run of extensions
    /// began to it (FR-OPS-01).
    pub(super) fn extend_selection(&mut self, direction: Direction) {
        let rows = self.listed();
        let motion = match direction {
            Direction::Up => Focus::Previous,
            Direction::Down => Focus::Next,
        };
        let target = focus::target(&rows, None, self.pane().selection.focused(), motion);
        self.pane_mut().selection.extend(&rows, target);
    }

    /// Picks the focused row, or unpicks it when it is picked (FR-OPS-01).
    pub(super) fn toggle_picked(&mut self) {
        let rows = self.listed();
        self.pane_mut().selection.toggle(&rows);
    }

    /// Starts visual line mode at the focused row, or ends it (FR-OPS-01, D-82).
    pub(super) fn toggle_visual(&mut self) {
        let rows = self.listed();
        self.pane_mut().selection.toggle_visual(&rows);
    }

    /// Ends visual line mode, keeping the picks for what ends it to act on (D-82).
    pub(super) fn end_visual(&mut self) {
        self.pane_mut().selection.end_visual();
    }

    /// Picks every row (FR-OPS-01).
    pub(super) fn select_all(&mut self) {
        let rows = self.listed();
        self.pane_mut().selection.select_all(&rows);
    }

    /// Picks nothing, so that commands take the focused row again (FR-OPS-01).
    pub(super) fn clear_selection(&mut self) {
        self.pane_mut().selection.clear();
    }

    /// The selected items without those inside another of them, which go along with it, in the
    /// order of the rows: what a command that moves or takes items acts on.
    fn targets(&self) -> Vec<ItemId> {
        container::outermost_rows(&self.document, self.pane().selection.items(&self.listed()))
    }

    /// Completes the selected items on `today`, or uncompletes them when all are completed, as
    /// one step to undo, moving the focus on when its row hides; on areas alone the status line
    /// says they are not completed (D-81).
    pub(super) fn toggle_completed(&mut self, today: Date) {
        let items = self.pane().selection.items(&self.listed());
        if self.refuses_areas(&items, Notice::AreaUncompleted) {
            return;
        }
        let transaction = completion::toggle(&self.document, items, today);
        self.change(transaction);
    }

    /// The selected items, or the container shown while its heading has the focus, which the
    /// commands that edit an item's own details act on (owner, D-68).
    fn described(&self) -> Vec<ItemId> {
        let selection = &self.pane().selection;
        let items = selection.items(&self.listed());
        if !items.is_empty() {
            return items;
        }
        let heading = self.pane().heading(&self.document);
        heading
            .filter(|&id| selection.focused() == Some(id))
            .into_iter()
            .collect()
    }

    /// Opens the date prompt for the `kind` date of the selected items, resolving what is typed
    /// against `today`. Nothing opens when none of them takes a date.
    pub(super) fn open_date_prompt(&mut self, kind: DateKind, today: Date) {
        let items = self.described();
        let focused = self.pane().selection.focused();
        self.open_date_prompt_for(items, kind, today, focused);
    }

    /// Opens the date prompt for the `kind` date of `items`, with the date of `focused` typed;
    /// on areas alone the status line says they have none (D-75).
    fn open_date_prompt_for(
        &mut self,
        items: Vec<ItemId>,
        kind: DateKind,
        today: Date,
        focused: Option<ItemId>,
    ) {
        if !self.refuses_areas(&items, Notice::AreaUndated) {
            self.date_prompt = DatePrompt::open(&self.document, items, kind, today, focused);
        }
    }

    /// Applies `input` to the open date prompt: applying a date, or a blank text, closes it and
    /// sets or removes the date of the items it was opened on as one step to undo. Nothing
    /// happens while it is closed.
    pub(super) fn date_prompt_input(&mut self, input: DateInput) {
        let Some(prompt) = self.date_prompt.as_mut() else {
            return;
        };
        match input {
            DateInput::Edit(edit) => prompt.edit(edit),
            DateInput::Cancel => self.date_prompt = None,
            DateInput::Apply => {
                if let Some(transaction) = prompt.transaction(&self.document) {
                    self.date_prompt = None;
                    self.change(transaction);
                }
            }
        }
    }

    /// Sets the start date of the selected items to `today`, as one step to undo.
    pub(super) fn start_today(&mut self, today: Date) {
        let items = self.described();
        self.start_today_for(items, today);
    }

    /// Sets the start date of `items` to `today`, as one step to undo; on areas alone the status
    /// line says they have none (D-75).
    fn start_today_for(&mut self, items: Vec<ItemId>, today: Date) {
        if !self.refuses_areas(&items, Notice::AreaUndated) {
            let transaction = date_prompt::set(&self.document, items, DateKind::Start, Some(today));
            self.change(transaction);
        }
    }

    /// Moves the `kind` date of the selected items `days` days later, or earlier when negative,
    /// a missing one counting from `today`, as one step to undo (D-82).
    pub(super) fn nudge_date(&mut self, kind: DateKind, days: i8, today: Date) {
        let items = self.described();
        self.nudge_date_for(items, kind, days, today);
    }

    /// Moves the `kind` date of `items` as [`nudge_date`](Self::nudge_date) says; on areas alone
    /// the status line says they have none (D-75).
    fn nudge_date_for(&mut self, items: Vec<ItemId>, kind: DateKind, days: i8, today: Date) {
        if !self.refuses_areas(&items, Notice::AreaUndated) {
            let transaction = date_prompt::nudge(&self.document, items, kind, days, today);
            self.change(transaction);
        }
    }

    /// Whether `items` are all areas, which have no tags or dates and are not completed, saying so
    /// with `notice` when they are (D-75, D-81).
    fn refuses_areas(&mut self, items: &[ItemId], notice: Notice) -> bool {
        let areas = !items.is_empty()
            && items.iter().all(|&id| {
                self.document
                    .node(id)
                    .and_then(Node::item)
                    .is_some_and(|item| item.kind == ItemKind::Area)
            });
        if areas {
            self.notify(notice);
        }
        areas
    }

    /// Moves the selected items to the Trash on `today`, or in the Trash deletes them for good,
    /// as one step to undo, moving the focus on from the rows that leave (FR-VIEW-04).
    pub(super) fn delete(&mut self, today: Date) {
        let transaction = if self.pane().shown == Shown::View(View::Trash) {
            // Taking items out of the document is what cutting does once they are copied.
            clipboard::cut(&self.document, &self.pane().selection.items(&self.listed()))
        } else {
            trash::delete(&self.document, self.targets(), today)
        };
        self.change(transaction);
    }

    /// Puts the selected items in the Trash back where they were, as one step to undo, moving the
    /// focus on from the rows that leave; nothing selected elsewhere is in the Trash (FR-VIEW-04).
    pub(super) fn restore_selected(&mut self) {
        let items = self.pane().selection.items(&self.listed());
        self.change(trash::restore(&self.document, items));
    }

    /// In the Trash, when `asked` the status line has just asked to confirm, deletes everything
    /// in it for good, as one step to undo; otherwise asks to confirm, unless the Trash is empty
    /// (FR-VIEW-04).
    pub(super) fn empty_trash(&mut self, asked: bool) {
        if self.pane().shown != Shown::View(View::Trash) {
            return;
        }
        let items = trash::items(&self.document);
        if asked {
            self.change(clipboard::cut(&self.document, &items));
        } else if !items.is_empty() {
            self.notify(Notice::EmptyTrash(items.len()));
        }
    }

    /// Puts a copy of each selected item, with its contents, right after it, as one step to undo,
    /// focusing the copy of the last one the pane shows.
    pub(super) fn duplicate(&mut self) {
        let taken = self.targets();
        let (transaction, last) = clipboard::duplicate(&mut self.document, &taken);
        self.change_focusing(transaction, last.or(self.pane().selection.focused()));
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
    /// one pasted the pane shows. Nothing is pasted when one of them can go nowhere near, while
    /// the sidebar has the keys, or in a view, which has no place to paste into.
    pub(super) fn paste(&mut self, side: Side, branches: &[Branch]) {
        if self.panel.is_some() {
            self.paste_checklist(side, branches);
            return;
        }
        let Some(container) = self.container().filter(|_| !self.sidebar.has_keys()) else {
            return;
        };
        let target = clipboard::Target {
            container,
            show_completed: self.pane().show_completed,
            focused: self.pane().selection.focused(),
            side,
        };
        if let Some((transaction, shown)) = clipboard::paste(&mut self.document, target, branches) {
            self.change_focusing(transaction, shown.or(self.pane().selection.focused()));
        }
    }

    /// Opens the Move to picker for the selected items, closing the Navigator and the tag picker.
    /// Nothing opens when no project, section or task is selected.
    pub(super) fn open_move_to(&mut self) {
        self.navigator = None;
        self.tags = None;
        self.move_to = MoveTo::open(&self.document, self.pane().selection.items(&self.listed()));
    }

    /// Applies `input` to the open Move to picker: choosing a destination closes it and moves the
    /// selected items there as one step to undo, the pane staying where it is and the focus
    /// moving on from the rows that leave, as Things 3 does. Nothing happens while it is closed,
    /// nor for choosing in a new pane, which no destination opens in.
    pub(super) fn move_to_input(&mut self, input: PickerInput) {
        if input == PickerInput::ChooseInNewPane {
            return;
        }
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

    /// Opens the tag picker for the selected items, closing the Navigator and Move to (FR-TAG-02,
    /// FR-OPS-06). Nothing opens with nothing selected.
    pub(super) fn open_tags(&mut self) {
        let items = self.described();
        self.open_tags_for(items);
    }

    fn open_tags_for(&mut self, items: Vec<ItemId>) {
        if self.refuses_areas(&items, Notice::AreaUntagged) {
            return;
        }
        self.navigator = None;
        self.move_to = None;
        self.tags = TagPicker::for_items(&self.document, items);
    }

    /// Opens the tag picker that chooses the tags the pane is filtered by, closing the Navigator
    /// and Move to (FR-TAG-04).
    pub(super) fn open_tag_filter(&mut self) {
        self.navigator = None;
        self.move_to = None;
        let rows = view::rows(
            &self.document,
            self.pane().shown,
            self.today,
            self.pane().show_completed,
            self.pane().query.text(),
        );
        self.tags = Some(TagPicker::for_filter(
            &self.document,
            &rows,
            &self.pane().filter,
        ));
    }

    /// Applies `input` to the open tag picker: choosing a tag closes it and adds the tag to the
    /// selected items, or removes it from them when each carries it, as one step to undo, or adds
    /// it to the filter, or takes it out when the filter has it. Nothing happens while it is
    /// closed, nor for choosing in a new pane, which no tag opens in.
    pub(super) fn tags_input(&mut self, input: PickerInput) {
        if input == PickerInput::ChooseInNewPane {
            return;
        }
        let Some(closed) = self
            .tags
            .as_mut()
            .and_then(|picker| picker.input(&self.document, input))
        else {
            return;
        };
        self.tags = None;
        match closed {
            Picked::Chosen(Choice::Items(transaction)) => self.change(transaction),
            Picked::Chosen(Choice::Filter(name)) => {
                let mut filter = self.pane().filter.clone();
                if filter.contains(&name) {
                    filter.retain(|chosen| *chosen != name);
                } else {
                    filter.push(name);
                }
                self.set_filter(filter);
            }
            Picked::Dismissed => {}
        }
    }

    /// Shows every row again, filtered by no tag (FR-TAG-04).
    pub(super) fn clear_tag_filter(&mut self) {
        self.set_filter(Vec::new());
    }

    /// Filters the pane by `filter`, moving the focus on when its row hides.
    fn set_filter(&mut self, filter: Vec<String>) {
        let before = self.listings();
        self.pane_mut().filter = filter;
        self.settle(&before);
    }

    /// Moves the selected items one step `direction` among their rows, as one step to undo,
    /// keeping the focus where it is (FR-OPS-02, FR-OPS-10). A view's order is not the file's,
    /// so nothing moves there.
    pub(super) fn move_item(&mut self, direction: Direction) {
        let Some(container) = self.container() else {
            return;
        };
        let (today, show_completed) = (self.today, self.pane().show_completed);
        let filter = self.pane().filter.clone();
        let transaction =
            moving::shift_all(&self.document, &self.targets(), direction, |document| {
                listed(
                    document,
                    (Shown::Container(container), today, ""),
                    show_completed,
                    &filter,
                )
            });
        self.change(transaction);
    }

    /// Converts the selected items in their place to projects, or to sections when all are
    /// projects, as one step to undo, keeping the focus on its item; the status line says why
    /// when the hierarchy refuses every one (FR-TASK-06, FR-ORG-06).
    pub(super) fn convert(&mut self) {
        match converting::convert_all(&self.document, &self.targets()) {
            Ok(transaction) => self.change(transaction),
            Err(refusal) => self.notify(Notice::Refused(refusal)),
        }
    }

    /// Adds a section or a project, as `kind` says, where the first selected item is, moves the
    /// selected items into it and starts editing its title in its row, picking nothing; the
    /// status line says why when the hierarchy refuses (FR-OPS-07, FR-OPS-08). Nothing happens
    /// in a view, where the new item would be no row.
    pub(super) fn group(&mut self, kind: ItemKind) {
        if self.container().is_none() {
            return;
        }
        let items = self.targets();
        match converting::group_place(&self.document, &items, kind) {
            Ok(Some(place)) => {
                let (before, selection) = (self.listings(), self.pane().selection.clone());
                let editing = Editing::group(
                    &mut self.document,
                    Item::new(kind, ""),
                    place,
                    &items,
                    selection,
                );
                self.settle(&before);
                self.pane_mut().selection.focus(Some(editing.id()));
                self.start_editing(Some(editing));
            }
            Ok(None) => {}
            Err(refusal) => self.notify(Notice::Refused(refusal)),
        }
    }

    /// Shows the area or project the focused row is in the pane, or else starts editing the
    /// focused row's or the heading's title, picking nothing (D-80). Nothing happens when nothing
    /// is focused.
    pub(super) fn edit_title(&mut self) {
        let focused = self.pane().selection.focused();
        let opens = focused
            .filter(|&id| Some(id) != self.container())
            .filter(|&id| {
                let item = self.document.node(id).and_then(Node::item);
                item.is_some_and(|item| matches!(item.kind, ItemKind::Area | ItemKind::Project))
                    && container::opens(&self.document, id)
            });
        if let Some(id) = opens {
            self.open(id, None);
            return;
        }
        let selection = &self.pane().selection;
        let editing = selection
            .focused()
            .and_then(|id| Editing::existing(&self.document, id, selection.clone()));
        self.start_editing(editing);
    }

    /// Enters edit mode with `editing`, if there is one, picking nothing, since what is picked
    /// is not what is edited.
    pub(super) fn start_editing(&mut self, editing: Option<Editing>) {
        if editing.is_some() {
            self.pane_mut().selection.clear();
        }
        self.editing = editing;
    }

    /// Starts editing the description of the focused item or of the container shown, as `whose`
    /// says, picking nothing. Nothing happens with nothing focused.
    pub(super) fn edit_description(&mut self, whose: Whose) {
        let id = match whose {
            Whose::Focused => self.pane().selection.focused(),
            Whose::Shown => self.container(),
        };
        let editing = id
            .and_then(|id| Editing::description(&self.document, id, self.pane().selection.clone()));
        self.start_editing(editing);
    }

    /// Adds an empty task on `side` of the focused row, focuses it and starts editing its title.
    /// Nothing is added to a container whose contents are hidden for being completed
    /// (FR-ORG-09), where it would not show, nor in a view, which has no place for it.
    pub(super) fn new_task(&mut self, side: Side) {
        let Some(container) = self.container().filter(|&id| !self.document.conceals(id)) else {
            return;
        };
        let place = adding::place(
            &self.document,
            container,
            &self.listed(),
            self.pane().selection.focused(),
            side,
        );
        self.new_task_at(place);
    }

    /// Adds an empty task at `place`, as its parent and its index there, focuses it and starts
    /// editing its title.
    fn new_task_at(&mut self, (parent, index): (ItemId, usize)) {
        let item = self.new_item(ItemKind::Task);
        let selection = self.pane().selection.clone();
        let editing = Editing::new_item(&mut self.document, item, (Some(parent), index), selection);
        self.pane_mut().selection.focus(Some(editing.id()));
        self.start_editing(Some(editing));
    }

    /// Adds an empty section, project or area, as `kind` says, on `side` of the focus as near as
    /// it can go, and starts editing its title: in its row when the container shown lists it, else
    /// in the pane's heading, the new item opened. Nothing is added where the kind can go nowhere
    /// near, as a section from the Inbox, inside a completed container (FR-ORG-09) or from a view.
    pub(super) fn new_container(&mut self, kind: ItemKind, side: Side) {
        let Some(container) = self.container() else {
            return;
        };
        let Some((parent, index)) = adding::container_place(
            &self.document,
            container,
            &self.listed(),
            self.pane().selection.focused(),
            kind,
            side,
        ) else {
            return;
        };
        self.add_at(container, kind, (parent, index));
    }

    /// Adds an item of the focused row's kind on `side` of it, or a task when no row is focused,
    /// as [`new_task`](Self::new_task) and [`new_container`](Self::new_container) add each kind
    /// (D-40). Below the heading it adds the container's first task, as below a section heading,
    /// and above it nothing, which no row of the container shown stands above (D-68).
    pub(super) fn new_like(&mut self, side: Side) {
        let focused = self.pane().selection.focused();
        if let Some(heading) = self.pane().heading(&self.document)
            && focused == Some(heading)
        {
            if side == Side::Below && !self.document.conceals(heading) {
                self.new_task_at((heading, 0));
            }
            return;
        }
        let kind = focused
            .and_then(|id| self.document.node(id))
            .and_then(Node::item)
            .map_or(ItemKind::Task, |item| item.kind);
        match kind {
            ItemKind::Task => self.new_task(side),
            kind => self.new_container(kind, side),
        }
    }

    /// Adds an empty task, section or project, as `kind` says, at `end` of the items of its kind
    /// in the container shown, or in the nearest container above that can hold it, and starts
    /// editing its title as [`new_container`](Self::new_container) does (D-40). Nothing is added
    /// from a view, where the kind can go nowhere near, or inside a completed container.
    pub(super) fn add(&mut self, kind: ItemKind, end: End) {
        let Some(container) = self.container() else {
            return;
        };
        if let Some(place) = adding::end_place(&self.document, container, kind, end) {
            self.add_at(container, kind, place);
        }
    }

    /// Adds an empty item of `kind` at `place` and starts editing its title: in its row when
    /// `container`, the container shown, holds it, else in the pane's heading, the new item
    /// opened. Nothing is added inside a completed container (FR-ORG-09).
    fn add_at(&mut self, container: ItemId, kind: ItemKind, place: (Option<ItemId>, usize)) {
        let (parent, _) = place;
        if parent.is_some_and(|parent| self.document.conceals(parent)) {
            return;
        }
        let item = self.new_item(kind);
        let selection = self.pane().selection.clone();
        let editing = Editing::new_item(&mut self.document, item, place, selection);
        let id = editing.id();
        if parent == Some(container) {
            self.pane_mut().selection.focus(Some(id));
        } else {
            self.open(id, None);
        }
        self.start_editing(Some(editing));
    }

    /// An item of `kind` to add in the pane, without a title, carrying the tags the pane is
    /// filtered by so that it shows, as in Things 3 (FR-TAG-04).
    fn new_item(&self, kind: ItemKind) -> Item {
        Item {
            tags: self
                .pane()
                .filter
                .iter()
                .map(|name| Tag::plain(name.as_str()))
                .collect(),
            ..Item::new(kind, "")
        }
    }

    /// Starts editing the title of the container shown, in the pane's heading. The Inbox keeps
    /// its name (FR-STORE-08), and a view has no container to rename.
    pub(super) fn rename_container(&mut self) {
        let editing = self
            .container()
            .and_then(|id| Editing::existing(&self.document, id, self.pane().selection.clone()));
        self.start_editing(editing);
    }

    /// Moves the container shown to the Trash on `today`, as one step to undo; the pane then
    /// shows the nearest container above it. The Inbox stays, as does a view.
    pub(super) fn delete_container(&mut self, today: Date) {
        if let Some(id) = self.container() {
            let transaction = trash::delete(&self.document, [id], today);
            self.change(transaction);
        }
    }

    /// Applies `input` to the edited title. Deleting back in an empty title ends editing, as
    /// [`EditInput::Done`] does, except in a checklist item's (D-84). Nothing happens outside edit
    /// mode.
    pub(super) fn edit_input(&mut self, input: EditInput) {
        let container = self.container();
        let Some(editing) = self.editing.as_mut() else {
            return;
        };
        match input {
            EditInput::Text(TextEdit::DeleteBackward)
                if editing.part() == Part::Title
                    && editing.kind() != ItemKind::ChecklistItem
                    && editing.field().text().is_empty() =>
            {
                self.finish_editing();
            }
            EditInput::Text(edit) => {
                if editing.input(&mut self.document, edit) == Err(Refused::Tag) {
                    self.notify(Notice::TagInTitle);
                }
            }
            EditInput::Next if self.panel.is_some() => {
                let checklist = editing.kind() == ItemKind::ChecklistItem;
                if self.finish_editing() && checklist {
                    self.new_checklist_item(Side::Below);
                }
            }
            EditInput::Next => {
                let row =
                    container.is_some_and(|id| id != editing.id()) && !self.sidebar.has_keys();
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
            EditInput::Cancel => {
                editing.revert(&mut self.document);
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
        let in_panel = self.panel.is_some();
        let heading = !in_sidebar && Some(id) == self.container();
        let listed = self.listings();
        let (transaction, before) = editing.finish(&mut self.document);
        if transaction.ops().is_empty() {
            self.saving.unchanged(started, self.document.revision());
        }
        let mut after = self.pane().selection.clone();
        if !heading && !in_sidebar && !in_panel {
            after.focus(Some(id));
        }
        self.history
            .commit(&mut self.document, transaction, before.clone(), |_| after)
            .expect("an edit's transaction is made from the document as it was");
        let kept = self.document.node(id).is_some();
        if in_sidebar {
            self.sidebar.finish(kept);
        }
        self.finish_adding(kept);
        if !kept {
            self.pane_mut().selection = before;
            if heading {
                self.go(Go::Back);
            }
        }
        self.settle(&listed);
        kept
    }

    /// Applies `transaction` as one step to undo, moving the focus on when its row leaves.
    pub(super) fn change(&mut self, transaction: Transaction) {
        self.change_focusing(transaction, self.pane().selection.focused());
    }

    /// Applies `transaction` as one step to undo, then focuses `focused`, or the row nearest it
    /// when it is not a row.
    fn change_focusing(&mut self, transaction: Transaction, focused: Option<ItemId>) {
        let before = self.listings();
        let listed_before = before.get(self.left.len()).map_or(&[][..], |rows| rows);
        let (pane, today) = (&self.pane, self.today);
        let mut after = pane.selection.clone();
        let selection = after.clone();
        self.history
            .commit(&mut self.document, transaction, selection, |document| {
                let rows = pane.rows(document, today);
                let heading = pane::heading(document, pane.shown);
                after.focus(focus::kept(listed_before, &rows, heading, focused));
                after
            })
            .expect("a transaction made from the document as it is fits it");
        if focused != self.pane().selection.focused() {
            self.pane_mut().selection.focus(focused);
        }
        self.settle(&before);
    }

    /// Shows the sidebar with the keys, focused on the entry of what the pane shows or of the
    /// closest container above it, or hides it when it has the keys, or gives it the keys.
    pub(super) fn toggle_sidebar(&mut self) {
        let entries = self.sidebar.visible(&self.document);
        let near = std::iter::once(self.pane().shown)
            .chain(self.path().into_iter().rev().map(Shown::Container))
            .find(|&shown| entries.iter().any(|stop| stop.shown() == shown));
        self.sidebar.toggle(near);
    }

    /// Keeps the sidebar's focus on an entry once a message may have changed the tree.
    pub(super) fn settle_sidebar(&mut self) {
        self.sidebar.reveal(&self.document);
        if self.sidebar.shows() {
            self.sidebar.settle(&self.sidebar.visible(&self.document));
        }
    }

    /// Gives `command` to the sidebar on `today` while it has the keys, and returns whether it
    /// took it. It acts on its focused entry: moving the focus, renaming, editing its description
    /// in the pane, adding an area or a project, collapsing it, deleting, moving, converting and
    /// setting its dates, each one step to
    /// undo, opening Move to for it or the attachments panel on it, and showing the entry in the
    /// first pane, the sidebar keeping the keys, or in a new pane, which gets them; a view's entry
    /// is only shown. Quitting, undo and redo, the Navigator, Go to Symbol, searching, the
    /// sidebar's own commands, focusing a pane, going to the Inbox, a view, back and forward and
    /// the conflict prompt's commands pass to the app; every other command does nothing.
    pub(super) fn sidebar_command(&mut self, command: Command, today: Date) -> bool {
        let passes = matches!(
            command,
            Command::Quit
                | Command::Undo
                | Command::Redo
                | Command::Navigator
                | Command::GoToSymbol
                | Command::Search
                | Command::Sidebar
                | Command::FocusLeft
                | Command::FocusRight
                | Command::FocusPane(_)
                | Command::NextPane
                | Command::PreviousPane
                | Command::Browse(_)
                | Command::Shortcuts
                | Command::ScrollShortcuts(_)
                | Command::Go(_)
                | Command::KeepMine
                | Command::TakeTheirs
                | Command::ShowDiff
        );
        if passes || !self.sidebar.has_keys() {
            return false;
        }
        let entries = self.sidebar.visible(&self.document);
        let focused = self.sidebar.focused();
        let apply = |state: &mut Self, result: Result<Transaction, moving::Refusal>| match result {
            Ok(transaction) => state.change(transaction),
            Err(refusal) => state.notify(Notice::Refused(refusal)),
        };
        match (command, focused) {
            (Command::Focus(motion), _) => self.move_sidebar_focus(motion),
            (Command::Collapse, _) => self.sidebar.toggle_collapsed(&self.document),
            (Command::NewLike(side), _) => self.sidebar_new(&entries, side),
            // The task keys add an area, `p` a project at the top level and `i` one in the area
            // focused or holding the project focused (owner, D-75).
            (Command::AddTask(end), _) => {
                self.sidebar_add(self.document.inbox(), ItemKind::Area, end);
            }
            (Command::AddProject(end), _) => {
                self.sidebar_add(self.document.inbox(), ItemKind::Project, end);
            }
            (Command::AddProjectInArea(end), Some(Shown::Container(id))) => {
                if let Some(area) = sidebar::area(&self.document, id) {
                    self.sidebar_add(area, ItemKind::Project, end);
                }
            }
            (Command::EditTitle, Some(shown)) => self.show_in_first_pane(shown),
            // The sidebar shows no descriptions, so the first pane, which the sidebar leads,
            // opens the project to edit its own under its title (owner, D-68, D-79).
            (Command::EditDescription(_), Some(shown @ Shown::Container(id)))
                if id != self.document.inbox() =>
            {
                self.focus_pane(1);
                self.show(shown, Some(id));
                self.pane_mut().selection.focus(Some(id));
                self.edit_description(Whose::Shown);
            }
            (Command::OpenInNewPane, Some(shown)) => {
                self.open_pane(Visit {
                    shown,
                    focused: None,
                });
            }
            (Command::RenameContainer, Some(Shown::Container(id))) => {
                self.editing = Editing::existing(&self.document, id, self.pane().selection.clone());
            }
            (Command::Delete, Some(Shown::Container(id))) => {
                self.change(trash::delete(&self.document, [id], today));
            }
            (Command::Move(direction), Some(Shown::Container(id))) => {
                self.change(sidebar::shift(&self.document, id, direction));
            }
            (Command::Convert, Some(Shown::Container(id))) => {
                apply(self, converting::convert_all(&self.document, &[id]));
            }
            (Command::MoveTo, Some(Shown::Container(id))) if id != self.document.inbox() => {
                self.navigator = None;
                self.move_to = MoveTo::open(&self.document, [id]);
            }
            (Command::SetDate(kind), Some(Shown::Container(id))) => {
                self.open_date_prompt_for(vec![id], kind, today, Some(id));
            }
            (Command::StartToday, Some(Shown::Container(id))) => {
                self.start_today_for(vec![id], today);
            }
            (Command::DayEarlier(kind), Some(Shown::Container(id))) => {
                self.nudge_date_for(vec![id], kind, -1, today);
            }
            (Command::DayLater(kind), Some(Shown::Container(id))) => {
                self.nudge_date_for(vec![id], kind, 1, today);
            }
            (Command::Tags, Some(Shown::Container(id))) => self.open_tags_for(vec![id]),
            (Command::OpenNote(_), Some(Shown::Container(id))) => {
                self.open_panel(id, Selected::Note);
            }
            (Command::OpenComments(_), Some(Shown::Container(id))) => {
                self.open_panel(id, Selected::Comment(0));
            }
            (Command::OpenChecklist, Some(_)) => self.notify(Notice::NoChecklist),
            _ => {}
        }
        true
    }

    /// Adds an empty area beside the sidebar's focused area, or else an empty project, among
    /// `entries` on `side` of the focused entry, focuses it and starts editing its title in the
    /// sidebar. The sidebar lists no tasks or sections, so beside a view or the Inbox it adds a
    /// project (D-45).
    fn sidebar_new(&mut self, entries: &[Stop], side: Side) {
        let focused = self.sidebar.focused();
        let area = |id| {
            let item = self.document.node(id).and_then(Node::item);
            item.is_some_and(|item| item.kind == ItemKind::Area)
        };
        let kind = match focused {
            Some(Shown::Container(id)) if area(id) => ItemKind::Area,
            _ => ItemKind::Project,
        };
        let place = sidebar::place(&self.document, entries, focused, kind, side)
            .expect("the sidebar focuses one of its entries, beside which a project or area goes");
        self.sidebar_add_at(kind, place);
    }

    /// Adds an empty project or area, as `kind` says, at `end` of its kind in `container`, or at
    /// the top level from the Inbox, focuses it and starts editing its title in the sidebar
    /// (D-45, D-75).
    fn sidebar_add(&mut self, container: ItemId, kind: ItemKind, end: End) {
        let place = adding::end_place(&self.document, container, kind, end)
            .expect("a project or an area goes at the top level when nowhere nearer");
        self.sidebar_add_at(kind, place);
    }

    /// Adds an empty item of `kind` at `place` from the sidebar, focuses it and starts editing
    /// its title there.
    fn sidebar_add_at(&mut self, kind: ItemKind, place: (Option<ItemId>, usize)) {
        let selection = self.pane().selection.clone();
        let editing = Editing::new_item(&mut self.document, Item::new(kind, ""), place, selection);
        self.sidebar.focus_new(editing.id());
        self.editing = Some(editing);
    }

    /// Shows completed items, or hides them again, moving the focus on when its row hides.
    pub(super) fn toggle_show_completed(&mut self) {
        let before = self.listings();
        self.pane_mut().show_completed = !self.pane().show_completed;
        self.settle(&before);
    }

    /// Takes back the latest change and restores the selection from before it.
    pub(super) fn undo(&mut self) {
        let before = self.listings();
        let restored = self
            .history
            .undo(&mut self.document)
            .expect("every change goes through the history, so its steps fit the document");
        self.restore(restored, &before);
    }

    /// Makes the latest change taken back again and restores the selection from after it.
    pub(super) fn redo(&mut self) {
        let before = self.listings();
        let restored = self
            .history
            .redo(&mut self.document)
            .expect("every change goes through the history, so its steps fit the document");
        self.restore(restored, &before);
    }

    /// Selects `selection` in the pane with the keys, if there is one, keeping each pane's focus
    /// on a row shown.
    fn restore(&mut self, selection: Option<Selection>, before: &[Rc<[Row]>]) {
        if let Some(selection) = selection {
            self.pane_mut().selection = selection;
            self.settle(before);
        }
    }

    /// Keeps each pane's focus on a row now that the rows `before` it listed have changed, and
    /// has a pane whose container shown no longer opens give way to the nearest one above it that
    /// does, so that an edit in one pane shows in every pane at once (FR-PANE-01).
    fn settle(&mut self, before: &[Rc<[Row]>]) {
        let (document, today) = (&self.document, self.today);
        let panes = self
            .left
            .iter_mut()
            .chain([&mut self.pane])
            .chain(&mut self.right);
        for (pane, before) in panes.zip(before) {
            pane.settle(document, today, before);
        }
    }

    /// Shows the place `to` says.
    pub(super) fn go(&mut self, to: Go) {
        let document = &self.document;
        match to {
            Go::Inbox => self.open(document.inbox(), None),
            Go::View(shown) => self.show(Shown::View(shown), None),
            Go::Back => {
                let today = self.today;
                self.pane.back(document, today);
            }
            Go::Forward => {
                let today = self.today;
                self.pane.forward(document, today);
            }
        }
    }

    /// Where the focused row leads: the area or project it is, when that opens, or for a section
    /// its container focused on it, or in a view the container listing its task, focused on it.
    fn inside_focused(&self) -> Option<Visit> {
        let document = &self.document;
        let focused = self.pane().selection.focused();
        let row = self
            .rows()
            .into_iter()
            .find(|row| Some(row.id()) == focused)?;
        match row {
            Row::Section(id) | Row::Project(id) => {
                container::place(document, id).map(|(shown, focused)| Visit {
                    shown: Shown::Container(shown),
                    focused,
                })
            }
            Row::Task { id, .. } if self.container().is_none() => container::listing(document, id)
                .map(|listing| Visit {
                    shown: Shown::Container(listing),
                    focused: Some(id),
                }),
            Row::Task { .. } => None,
        }
    }

    /// Whether `visit` focuses a completed item, as one opened from a view where it lives does,
    /// which shows there only with completed items shown (FR-VIEW-11, FR-SRCH-02).
    fn focuses_completed(&self, visit: Visit) -> bool {
        visit
            .focused
            .and_then(|id| self.document.node(id))
            .and_then(Node::item)
            .is_some_and(|item| item.completion().is_some())
    }

    /// Opens what the focused row leads into, else the place the pane shows focused where it is,
    /// in a new pane right of the one with the keys, which gets them (FR-PANE-02).
    pub(super) fn open_in_new_pane(&mut self) {
        let visit = self.inside_focused().unwrap_or_else(|| self.pane().visit());
        self.open_pane(visit);
    }

    /// Adds a pane showing what `visit` shows right of the one with the keys and gives it the
    /// keys. It shows completed items when that pane does or `visit` focuses one, and has no
    /// history and no tag filter.
    pub(super) fn open_pane(&mut self, visit: Visit) {
        let pane = Pane::new(
            &self.document,
            self.today,
            visit,
            self.pane().show_completed || self.focuses_completed(visit),
        );
        self.left.push(mem::replace(&mut self.pane, pane));
        self.sidebar.focus(false);
    }

    /// Closes the pane with the keys, giving them to the pane on its left, or on its right when it
    /// was the first. The only pane stays (FR-PANE-02).
    pub(super) fn close_pane(&mut self) {
        let next = self
            .left
            .pop()
            .or_else(|| (!self.right.is_empty()).then(|| self.right.remove(0)));
        if let Some(next) = next {
            self.pane = next;
        }
    }

    /// Gives the keys to the pane numbered `number`, counted from 1 on the left, when there is
    /// one, or for 0 to the sidebar, showing it when hidden (FR-PANE-02, FR-VIEW-10).
    pub(super) fn focus_pane(&mut self, number: usize) {
        let Some(index) = number.checked_sub(1) else {
            if self.sidebar.shows() {
                self.sidebar.focus(true);
            } else {
                self.toggle_sidebar();
            }
            return;
        };
        if index > self.left.len() + self.right.len() {
            return;
        }
        while self.left.len() > index {
            self.step_left();
        }
        while self.left.len() < index {
            self.step_right();
        }
        self.sidebar.focus(false);
    }

    /// Gives the keys to the pane on the left of the one with them, or from the first pane to the
    /// sidebar while it shows (FR-PANE-02, FR-VIEW-10).
    pub(super) fn focus_left(&mut self) {
        if self.sidebar.has_keys() {
            return;
        }
        if self.left.is_empty() {
            self.sidebar.focus(true);
        } else {
            self.step_left();
        }
    }

    /// Gives the keys from the sidebar back to the pane that had them, or to the pane on the right
    /// of the one with them (FR-PANE-02, FR-VIEW-10).
    pub(super) fn focus_right(&mut self) {
        if self.sidebar.has_keys() {
            self.sidebar.focus(false);
        } else {
            self.step_right();
        }
    }

    /// Gives the keys to the next pane in `direction`, the sidebar while it shows counting as the
    /// pane before the first, wrapping around at either end (FR-PANE-02, FR-VIEW-10).
    pub(super) fn cycle_pane(&mut self, direction: Direction) {
        let last = self.left.len() + 1 + self.right.len();
        let first = usize::from(!self.sidebar.shows());
        let at = if self.sidebar.has_keys() {
            0
        } else {
            self.left.len() + 1
        };
        let next = match direction {
            Direction::Down if at == last => first,
            Direction::Down => at + 1,
            Direction::Up if at == first => last,
            Direction::Up => at - 1,
        };
        self.focus_pane(next);
    }

    /// Focuses the sidebar's entry in `direction`, shown or hidden, and shows it in the first
    /// pane, the keys staying where they are (FR-VIEW-10).
    pub(super) fn browse(&mut self, direction: Direction) {
        self.move_sidebar_focus(match direction {
            Direction::Down => Focus::Next,
            Direction::Up => Focus::Previous,
        });
    }

    /// Moves the sidebar's focus as `motion` says, shown or hidden, and shows the entry it lands
    /// on in the first pane, which the sidebar leads (FR-VIEW-10).
    fn move_sidebar_focus(&mut self, motion: Focus) {
        self.sidebar
            .move_focus(&self.sidebar.visible(&self.document), motion);
        if let Some(shown) = self.sidebar.focused() {
            self.show_in_first_pane(shown);
        }
    }

    /// Shows `shown` in the first pane, which the sidebar leads, the keys staying where they are
    /// (FR-VIEW-10).
    fn show_in_first_pane(&mut self, shown: Shown) {
        let first = self.left.first_mut().unwrap_or(&mut self.pane);
        first.show(&self.document, self.today, shown, None);
    }

    /// Gives the keys to the pane on the left, if there is one.
    fn step_left(&mut self) {
        if let Some(left) = self.left.pop() {
            self.right.insert(0, mem::replace(&mut self.pane, left));
        }
    }

    /// Gives the keys to the pane on the right, if there is one.
    fn step_right(&mut self) {
        if !self.right.is_empty() {
            let right = self.right.remove(0);
            self.left.push(mem::replace(&mut self.pane, right));
        }
    }

    /// Has the pane with the keys keep `width`, as the frontend measures it, from now on
    /// (FR-PANE-02).
    pub(super) fn set_pane_width(&mut self, width: u16) {
        self.pane_mut().width = Some(width);
    }

    /// Opens the Navigator on where the pane is, closing the command palette, Go to Symbol, Move
    /// to, the date prompt, the tag picker and the settings window, or closes it when it is open.
    pub(super) fn toggle_navigator(&mut self) {
        self.palette = None;
        self.go_to_symbol = None;
        self.settings_window = None;
        self.move_to = None;
        self.date_prompt = None;
        self.tags = None;
        self.navigator = match self.navigator {
            Some(_) => None,
            None => Some(navigator::open(&self.document, self.pane().shown)),
        };
    }

    /// Applies `input` to the open Navigator: choosing a place closes it and opens the place in
    /// the pane, or in a new pane (FR-PANE-02). Nothing happens while it is closed.
    pub(super) fn navigator_input(&mut self, input: PickerInput) {
        let Some(picked) = self
            .navigator
            .as_mut()
            .and_then(|picker| picker.input(input))
        else {
            return;
        };
        self.navigator = None;
        if let Picked::Chosen(stop) = picked {
            let visit = Visit {
                shown: stop.shown(),
                focused: None,
            };
            if input == PickerInput::ChooseInNewPane {
                self.open_pane(visit);
            } else {
                self.show(visit.shown, visit.focused);
            }
        }
    }

    /// Shows `container` as [`show`](Self::show) does.
    fn open(&mut self, container: ItemId, focused: Option<ItemId>) {
        self.show(Shown::Container(container), focused);
    }

    /// Shows `shown`, focused on `focused` or its first row, so that Back returns to where the
    /// pane was. Showing what is shown changes nothing.
    pub(super) fn show(&mut self, shown: Shown, focused: Option<ItemId>) {
        let (document, today) = (&self.document, self.today);
        self.pane.show(document, today, shown, focused);
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::{TagMark, TextEdit, view::View};
    use crate::domain::{Item, ItemKind, Node, PlacementError};

    // FR-NAV-02, FR-STORE-01
    #[test]
    fn a_new_state_shows_the_inbox_with_its_first_task_focused() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let first = document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, "a")))?;
        document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, "b")))?;
        let state = State::new(document.clone(), Opened::Demo, TODAY);
        assert_eq!(state.container(), Some(inbox));
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
            TODAY,
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
        let mut state = State::new(document, Opened::Demo, TODAY);
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
            state: State::new(document, Opened::Demo, TODAY),
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

    fn at(state: &State) -> (Option<ItemId>, Option<ItemId>) {
        (state.container(), state.selection().focused())
    }

    // FR-NAV-02, FR-EDIT-01, D-68, D-80
    #[test]
    fn enter_opens_a_project_row_and_edits_any_other_title() -> Result<(), PlacementError> {
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
        assert_eq!(at(&state), (Some(work), Some(loose)));
        assert_eq!(state.path(), [work]);
        let edited = |state: &State| state.editing().map(|(id, ..)| id);
        state.edit_title();
        assert_eq!(edited(&state), Some(loose), "a task is edited");
        state.edit_input(EditInput::Done);
        state.move_focus(Focus::Next);
        state.edit_title();
        assert_eq!(edited(&state), Some(section), "so is a section");
        state.edit_input(EditInput::Done);
        state.open_in_new_pane();
        assert_eq!(
            at(&state),
            (Some(work), Some(section)),
            "a section opens in its area, focused on it"
        );
        state.close_pane();
        state.move_focus(Focus::Last);
        state.edit_title();
        assert_eq!(edited(&state), None);
        assert_eq!(at(&state), (Some(site), Some(in_site)), "a project opens");
        state.move_focus(Focus::First);
        state.edit_title();
        assert_eq!(edited(&state), Some(site), "its title is renamed");
        assert_ne!(state.container(), Some(in_section));
        Ok(())
    }

    // FR-NAV-02
    #[test]
    fn go_to_inbox_shows_the_inbox() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            work,
            ..
        } = fixture()?;
        let inbox = state.document().inbox();
        navigate(&mut state, "Work");
        assert_eq!(state.container(), Some(work));
        state.go(Go::Inbox);
        assert_eq!(at(&state), (Some(inbox), Some(inbox_task)));
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
            site,
            in_site,
            ..
        } = fixture()?;
        let inbox = state.document().inbox();
        navigate(&mut state, "Work");
        state.move_focus(Focus::Next);
        navigate(&mut state, "Site");
        state.go(Go::Back);
        assert_eq!(at(&state), (Some(work), Some(section)));
        state.go(Go::Back);
        assert_eq!(at(&state), (Some(inbox), Some(inbox_task)));
        state.go(Go::Back);
        assert_eq!(state.container(), Some(inbox), "nothing is further back");
        state.go(Go::Forward);
        state.go(Go::Forward);
        assert_eq!(at(&state), (Some(site), Some(in_site)));
        state.go(Go::Forward);
        assert_eq!(state.container(), Some(site), "nothing is further ahead");
        // A place gone to the Trash is passed over.
        state.go(Go::Inbox);
        let mut trashed = Item::new(ItemKind::Project, "Site");
        trashed.deleted = Some(jiff::civil::date(2026, 10, 1));
        state.document.set(site, trashed)?;
        state.go(Go::Back);
        assert_eq!(state.container(), Some(work));
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

    // FR-EDIT-04, FR-TAG-02
    #[test]
    fn a_word_typed_with_an_at_sign_first_is_refused_with_a_notice() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            ..
        } = fixture()?;
        state.edit_title();
        type_text(&mut state, " @x");
        assert_eq!(title(&state, inbox_task), Some("i x"));
        assert_eq!(state.notice(), Some(&Notice::TagInTitle));
        Ok(())
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
        state.move_focus(Focus::Next);
        state.edit_title();
        assert_eq!(state.editing().map(|(id, ..)| id), Some(section));
        type_text(&mut state, "ection");
        state.edit_input(EditInput::Done);
        assert_eq!(title(&state, section), Some("Section"));
        state.go(Go::Inbox);
        let mut document = state.document().clone();
        document.push(None, Node::Item(Item::new(ItemKind::Project, "Empty")))?;
        state = State::new(document, Opened::Demo, TODAY);
        navigate(&mut state, "Empty");
        let empty = state.container();
        assert_eq!(
            state.selection().focused(),
            empty,
            "its heading, with no row"
        );
        state.edit_title();
        assert_eq!(state.editing().map(|(id, ..)| id), empty, "D-68");
        state.edit_input(EditInput::Done);
        let mut nothing = State::new(Document::new(), Opened::Demo, TODAY);
        nothing.edit_title();
        assert!(
            nothing.editing().is_none(),
            "with nothing focused nothing is edited"
        );
        Ok(())
    }

    // FR-NAV-01, FR-EDIT-05, FR-DATE-01, FR-TAG-02, D-68
    #[test]
    fn the_heading_takes_the_focus_and_the_commands_on_its_own_item() -> Result<(), PlacementError>
    {
        let Fixture {
            mut state,
            work,
            site,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.move_focus(Focus::First);
        assert_eq!(state.selection().focused(), Some(work));
        state.toggle_picked();
        state.toggle_completed(TODAY);
        let done = state
            .document()
            .node(work)
            .and_then(Node::item)
            .map(|it| it.completed);
        assert_eq!(done, Some(None), "nothing picked, nothing completed");
        state.open_tags();
        assert_eq!(state.tags(), None);
        assert_eq!(
            state.notice(),
            Some(&Notice::AreaUntagged),
            "Work is an area"
        );
        state.start_today(TODAY);
        state.open_date_prompt(DateKind::Due, TODAY);
        assert_eq!(state.date_prompt(), None);
        assert_eq!(state.notice(), Some(&Notice::AreaUndated));
        navigate(&mut state, "Site");
        state.move_focus(Focus::First);
        state.open_tags();
        assert!(state.tags().is_some(), "Site is a project");
        state.tags = None;
        state.start_today(TODAY);
        let start = |state: &State, id| {
            state
                .document()
                .node(id)
                .and_then(Node::item)
                .map(|it| it.start)
        };
        assert_eq!(start(&state, site), Some(Some(TODAY)));
        assert_eq!(start(&state, work), Some(None), "the area takes none");
        navigate(&mut state, "Work");
        state.move_focus(Focus::First);
        state.new_like(Side::Above);
        assert!(
            state.editing().is_none(),
            "nothing stands above the heading"
        );
        state.new_like(Side::Below);
        let new = state.editing().map(|(id, ..)| id);
        assert_eq!(state.document().children(work).first().copied(), new);
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
        let mut state = State::new(Document::new(), Opened::Demo, TODAY);
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

    // FR-EDIT-01, D-84
    #[test]
    fn deleting_back_in_an_empty_title_ends_editing_as_done_does() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            work,
            ..
        } = fixture()?;
        let back = EditInput::Text(TextEdit::DeleteBackward);
        let inbox = state.document().inbox();
        state.new_task(Side::Below);
        state.edit_input(back);
        assert!(state.editing().is_none());
        assert_eq!(
            state.document().children(inbox),
            [inbox_task],
            "a new task goes"
        );
        state.edit_title();
        state.edit_input(back);
        assert_eq!(
            state.editing().map(|(.., field)| field.text()),
            Some(""),
            "a title with text loses its last character"
        );
        state.edit_input(back);
        assert!(state.editing().is_none());
        assert_eq!(
            title(&state, inbox_task),
            Some("i"),
            "an emptied title is kept as done keeps it"
        );
        navigate(&mut state, "Work");
        state.new_container(ItemKind::Area, Side::Below);
        state.edit_input(back);
        assert!(state.editing().is_none());
        assert_eq!(state.document().top_level(), [work], "a new container goes");
        assert_eq!(state.container(), Some(work));
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
            site,
            in_site,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.move_focus(Focus::Last);
        state.delete(TODAY);
        state.undo();
        state.show(Shown::Container(site), None);
        assert_eq!(at(&state), (Some(site), Some(in_site)));
        state.redo();
        assert_eq!(
            at(&state),
            (Some(work), Some(loose)),
            "the project is gone, so its area opens on its first row"
        );
        // With everything above gone too, home is left.
        state.undo();
        state.show(Shown::Container(site), None);
        let mut gone = Item::new(ItemKind::Area, "Work");
        gone.deleted = Some(TODAY);
        state.document.set(work, gone)?;
        state.redo();
        assert_eq!(state.container(), Some(state.document().inbox()));
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
        assert_eq!(at(&state), (Some(work), Some(section)));
        state.undo();
        assert_eq!(at(&state), (Some(work), Some(loose)));
        assert_eq!(state.document().position(loose), Some((Some(work), 0)));
        // Into a section of the container shown, the task stays a row and keeps the focus.
        move_to(&mut state, "S");
        assert_eq!(state.document().position(loose), Some((Some(section), 1)));
        assert_eq!(at(&state), (Some(work), Some(loose)));
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

    // FR-VIEW-03, FR-TASK-02, FR-EDIT-06
    #[test]
    fn uncompleting_in_the_logbook_leaves_the_item_where_it_is_and_moves_the_focus_on()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let mut done = |title: &str, day| {
            let mut item = Item::new(ItemKind::Task, title);
            item.completed = Some(crate::domain::Completion::On(day));
            document.push(Some(inbox), Node::Item(item))
        };
        let older = done("older", date(2026, 10, 1))?;
        let newer = done("newer", date(2026, 10, 2))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        state.go(Go::View(View::Logbook));
        let ids = |state: &State| state.rows().into_iter().map(Row::id).collect::<Vec<_>>();
        assert_eq!(ids(&state), [newer, older]);
        assert_eq!(state.selection().focused(), Some(newer));
        state.toggle_completed(TODAY);
        assert_eq!(ids(&state), [older]);
        assert_eq!(state.selection().focused(), Some(older));
        assert_eq!(
            state.document().children(inbox),
            [older, newer],
            "nothing moves"
        );
        state.undo();
        assert_eq!(ids(&state), [newer, older]);
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
            Some(inbox),
            "a closed Navigator takes no input"
        );
        state.toggle_navigator();
        assert!(
            state
                .navigator()
                .is_some_and(|view| view.entries.len() == 3 + view::REGISTRY.len())
        );
        state.navigator_input(PickerInput::Next);
        assert!(state.navigator().is_some_and(|view| view.focused == 1));
        state.toggle_navigator();
        assert!(state.navigator().is_none());
        state.toggle_navigator();
        state.navigator_input(PickerInput::Dismiss);
        assert!(state.navigator().is_none());
        assert_eq!(state.container(), Some(inbox), "dismissing goes nowhere");
        navigate(&mut state, "Inbox");
        state.go(Go::Back);
        assert_eq!(
            state.container(),
            Some(inbox),
            "choosing where the pane is records no visit"
        );
        navigate(&mut state, "Work");
        assert_eq!(state.container(), Some(work));
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
        assert_eq!(state.container(), Some(work));
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
    fn a_new_top_level_project_opens_with_its_title_edited_and_left_empty_the_pane_goes_back()
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
        assert_eq!(at(&state), (Some(area), Some(area)), "its heading focused");
        assert_eq!(state.document().top_level(), [area, work]);
        type_text(&mut state, " ");
        state.edit_input(EditInput::Next);
        assert_eq!(at(&state), (Some(inbox), Some(inbox_task)));
        assert_eq!(state.document().top_level(), [work]);
        state.go(Go::Forward);
        assert_eq!(
            state.container(),
            Some(inbox),
            "the area left empty is not there"
        );
        state.new_container(ItemKind::Area, Side::Above);
        type_text(&mut state, "Home");
        state.edit_input(EditInput::Done);
        let home = state.document().top_level()[0];
        assert_eq!(at(&state), (Some(home), Some(home)));
        assert_eq!(title(&state, home), Some("Home"));
        state.go(Go::Back);
        assert_eq!(at(&state), (Some(inbox), Some(inbox_task)));
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06
    #[test]
    fn new_containers_go_only_where_they_can_show() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            site,
            ..
        } = fixture()?;
        let before = state.clone();
        state.new_container(ItemKind::Section, Side::Below);
        assert_eq!(state, before, "the Inbox holds no section");
        navigate(&mut state, "Site");
        let mut done = Item::new(ItemKind::Project, "Site");
        done.completed = Some(crate::domain::Completion::Undated);
        state.document.set(site, done)?;
        let before = state.clone();
        state.new_container(ItemKind::Section, Side::Below);
        assert_eq!(state, before, "no section in a completed project");
        state.new_container(ItemKind::Area, Side::Below);
        let (area, ..) = state.editing().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(state.document().top_level(), [work, area]);
        Ok(())
    }

    // FR-EDIT-05, FR-ORG-06, D-40
    #[test]
    fn new_items_take_the_focused_rows_kind_or_go_at_either_end_of_their_kind()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            section,
            ..
        } = fixture()?;
        let add = |state: &mut State, kind, end, title| {
            state.add(kind, end);
            type_text(state, title);
            state.edit_input(EditInput::Done);
        };
        let titles = |state: &State| -> Vec<String> {
            state
                .document()
                .children(work)
                .iter()
                .filter_map(|&id| title(state, id).map(str::to_owned))
                .collect()
        };
        let before = state.clone();
        state.add(ItemKind::Section, End::Top);
        assert_eq!(
            state, before,
            "the Inbox holds no section, nor does the top level"
        );
        state.go(Go::View(View::Today));
        let before = state.clone();
        state.add(ItemKind::Task, End::Top);
        assert_eq!(state, before, "a view has no container to add to");
        navigate(&mut state, "Work");
        add(&mut state, ItemKind::Task, End::Top, "first");
        add(&mut state, ItemKind::Section, End::Bottom, "T");
        add(&mut state, ItemKind::Project, End::Top, "Plan");
        assert_eq!(
            titles(&state),
            ["first", "loose", "S", "T", "Plan", "Site"],
            "each first or last among its kind"
        );
        state.pane_mut().selection.focus(Some(section));
        state.new_like(Side::Below);
        type_text(&mut state, "U");
        state.edit_input(EditInput::Done);
        assert_eq!(
            titles(&state),
            ["first", "loose", "S", "U", "T", "Plan", "Site"],
            "a section beside the focused section"
        );
        navigate(&mut state, "Site");
        add(&mut state, ItemKind::Project, End::Bottom, "Next");
        assert_eq!(
            titles(&state).last().map(String::as_str),
            Some("Next"),
            "a project goes into the area holding the one shown, and opens"
        );
        assert_eq!(
            state.container().and_then(|id| title(&state, id)),
            Some("Next")
        );
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
        assert_eq!(at(&state), (Some(site), Some(in_site)));
        assert_eq!(title(&state, site), Some("Sites"));
        state.delete_container(date(2026, 10, 14));
        assert_eq!(
            at(&state),
            (Some(work), state.rows().first().map(|row| row.id()))
        );
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
        Ok((State::new(document, Opened::Demo, TODAY), tasks))
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
        state.move_focus(Focus::Next);
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

    // FR-OPS-01, FR-OPS-02
    #[test]
    fn picked_items_move_together() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            loose,
            section,
            in_section,
            site,
            ..
        } = fixture()?;
        let mut document = state.document().clone();
        let add = |document: &mut Document, title| {
            document.push(Some(work), Node::Item(Item::new(ItemKind::Project, title)))
        };
        let two = add(&mut document, "Two")?;
        let three = add(&mut document, "Three")?;
        state = State::new(document, Opened::Demo, TODAY);
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
        Ok(())
    }

    /// A state on the Inbox, holding `inbox_task @work` starting today, beside area `work`
    /// holding `overdue @home`, due yesterday, and project `site` due today, holding
    /// `in_launch`, starting today in section `launch`, and `tomorrow`, starting tomorrow.
    struct Dated {
        state: State,
        inbox_task: ItemId,
        work: ItemId,
        overdue: ItemId,
        site: ItemId,
        launch: ItemId,
        in_launch: ItemId,
        tomorrow: ItemId,
    }

    fn dated() -> Result<Dated, PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let mut add = |parent, kind, start: Option<Date>, due: Option<Date>, tags: &[&str]| {
            let item = Item {
                start,
                due,
                tags: tags.iter().map(|&name| Tag::plain(name)).collect(),
                ..Item::new(kind, "x")
            };
            document.push(parent, Node::Item(item))
        };
        let (yesterday, tomorrow) = (date(2026, 10, 13), date(2026, 10, 15));
        let inbox_task = add(Some(inbox), ItemKind::Task, Some(TODAY), None, &["work"])?;
        let work = add(None, ItemKind::Area, None, None, &[])?;
        let overdue = add(Some(work), ItemKind::Task, None, Some(yesterday), &["home"])?;
        let site = add(Some(work), ItemKind::Project, None, Some(TODAY), &[])?;
        let launch = add(Some(site), ItemKind::Section, None, None, &[])?;
        let in_launch = add(Some(launch), ItemKind::Task, Some(TODAY), None, &[])?;
        let tomorrow = add(Some(site), ItemKind::Task, Some(tomorrow), None, &[])?;
        Ok(Dated {
            state: State::new(document, Opened::Demo, TODAY),
            inbox_task,
            work,
            overdue,
            site,
            launch,
            in_launch,
            tomorrow,
        })
    }

    // FR-VIEW-06, FR-NAV-04, FR-KEY-06
    #[test]
    fn today_shows_in_the_pane_and_back_and_forward_pass_through_it() -> Result<(), PlacementError>
    {
        let Dated {
            mut state,
            inbox_task,
            overdue,
            site,
            in_launch,
            ..
        } = dated()?;
        let inbox = state.document().inbox();
        state.go(Go::View(View::Today));
        assert_eq!(state.shown(), Shown::View(View::Today));
        assert_eq!(state.container(), None);
        assert_eq!(state.path(), []);
        let ids: Vec<ItemId> = state.rows().into_iter().map(Row::id).collect();
        assert_eq!(ids, [overdue, inbox_task, site, in_launch]);
        assert_eq!(
            state
                .groups()
                .iter()
                .map(|group| group.heading)
                .collect::<Vec<_>>(),
            [None],
            "one list without headings (D-51)"
        );
        assert_eq!(state.selection().focused(), Some(overdue));
        state.move_focus(Focus::Next);
        state.go(Go::View(View::Today));
        assert_eq!(
            state.selection().focused(),
            Some(inbox_task),
            "showing it again changes nothing"
        );
        state.go(Go::Back);
        assert_eq!(
            at(&state),
            (Some(inbox), None),
            "the Inbox view leaves out its scheduled task"
        );
        assert_eq!(state.groups(), [], "a container has no groups");
        state.go(Go::Forward);
        assert_eq!(state.shown(), Shown::View(View::Today));
        assert_eq!(state.selection().focused(), Some(inbox_task));
        state.toggle_navigator();
        state.navigator_input(PickerInput::Choose);
        assert_eq!(
            state.shown(),
            Shown::View(View::Today),
            "the Navigator opens on the view shown"
        );
        navigate(&mut state, "Inbox");
        assert_eq!(at(&state), (Some(inbox), None));
        Ok(())
    }

    // FR-VIEW-06, FR-NAV-02
    #[test]
    fn a_row_of_today_opens_where_it_lives() -> Result<(), PlacementError> {
        let Dated {
            mut state,
            site,
            launch,
            in_launch,
            inbox_task,
            work,
            overdue,
            ..
        } = dated()?;
        state.go(Go::View(View::Today));
        state.pane_mut().selection.focus(Some(in_launch));
        state.open_in_new_pane();
        assert_eq!(
            at(&state),
            (Some(site), Some(in_launch)),
            "a section's task shows in its project, under the section's heading"
        );
        state.close_pane();
        state.pane_mut().selection.focus(Some(overdue));
        state.open_in_new_pane();
        assert_eq!(at(&state), (Some(work), Some(overdue)));
        state.close_pane();
        state.pane_mut().selection.focus(Some(inbox_task));
        state.open_in_new_pane();
        assert_eq!(
            state.shown(),
            Shown::View(View::Today),
            "the Inbox view does not list a scheduled task, so it opens Today again"
        );
        state.close_pane();
        state.pane_mut().selection.focus(Some(site));
        state.edit_title();
        assert_eq!(state.container(), Some(site), "Enter opens a project");
        assert_eq!(container::listing(state.document(), launch), None);
        assert_eq!(container::listing(state.document(), ItemId::new(99)), None);
        Ok(())
    }

    // FR-VIEW-06, FR-TASK-02, FR-EDIT-06
    #[test]
    fn completing_in_today_takes_the_item_out_and_undo_brings_it_back() -> Result<(), PlacementError>
    {
        let Dated {
            mut state,
            inbox_task,
            site,
            ..
        } = dated()?;
        state.go(Go::View(View::Today));
        state.pane_mut().selection.focus(Some(inbox_task));
        state.toggle_completed(TODAY);
        assert_eq!(state.selection().focused(), Some(site));
        assert!(!state.rows().iter().any(|row| row.id() == inbox_task));
        state.toggle_show_completed();
        assert_eq!(
            state.rows().last(),
            Some(&Row::Task {
                id: inbox_task,
                section: None
            }),
            "a completed item comes last"
        );
        state.toggle_show_completed();
        state.undo();
        assert_eq!(state.selection().focused(), Some(inbox_task));
        assert_eq!(state.shown(), Shown::View(View::Today));
        Ok(())
    }

    // FR-VIEW-06
    #[test]
    fn today_follows_the_date() -> Result<(), PlacementError> {
        let Dated {
            mut state,
            overdue,
            tomorrow,
            ..
        } = dated()?;
        state.go(Go::View(View::Today));
        state.pane_mut().selection.focus(Some(overdue));
        state.set_today(date(2026, 10, 15));
        assert!(state.rows().iter().any(|row| row.id() == tomorrow));
        assert_eq!(state.selection().focused(), Some(overdue));
        Ok(())
    }

    // FR-VIEW-06, FR-EDIT-01
    #[test]
    fn commands_that_need_a_container_do_nothing_in_today() -> Result<(), PlacementError> {
        let Dated {
            mut state,
            inbox_task,
            ..
        } = dated()?;
        state.go(Go::View(View::Today));
        state.pane_mut().selection.focus(Some(inbox_task));
        let before = state.document().clone();
        state.new_task(Side::Below);
        for kind in [ItemKind::Section, ItemKind::Project, ItemKind::Area] {
            state.new_container(kind, Side::Below);
        }
        let branch = Branch {
            node: Node::Item(Item::new(ItemKind::Task, "pasted")),
            children: Vec::new(),
        };
        state.paste(Side::Below, &[branch]);
        state.move_item(Direction::Down);
        state.group(ItemKind::Section);
        state.delete_container(TODAY);
        assert_eq!(state.document(), &before);
        state.rename_container();
        state.edit_description(Whose::Shown);
        assert_eq!(state.editing(), None);
        state.edit_title();
        type_text(&mut state, "y");
        state.edit_input(EditInput::Next);
        assert_eq!(state.editing(), None, "Enter only ends editing");
        assert_eq!(title(&state, inbox_task), Some("xy"));
        assert_eq!(state.shown(), Shown::View(View::Today));
        assert_eq!(state.rows().len(), 4, "no task was added");
        Ok(())
    }

    // FR-VIEW-06, FR-TAG-04
    #[test]
    fn a_tag_filter_in_today_hides_the_rows_without_the_tag() -> Result<(), PlacementError> {
        let Dated {
            mut state, overdue, ..
        } = dated()?;
        state.go(Go::View(View::Today));
        state.set_filter(vec!["home".to_owned()]);
        assert_eq!(
            state.groups(),
            [Group {
                heading: None,
                rows: vec![Row::Task {
                    id: overdue,
                    section: None
                }]
            }],
            "the rows without the tag are dropped"
        );
        let flattened: Vec<Row> = state.groups().into_iter().flat_map(|g| g.rows).collect();
        assert_eq!(state.rows(), flattened);
        assert_eq!(state.selection().focused(), Some(overdue));
        state.open_tag_filter();
        let listed: Vec<(String, TagMark)> = state
            .tags()
            .map(|tags| {
                tags.picker
                    .entries
                    .iter()
                    .map(|entry| (entry.value.name.clone(), entry.value.mark))
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(
            listed,
            [
                ("home".to_owned(), TagMark::All),
                ("work".to_owned(), TagMark::Off)
            ],
            "the picker lists the tags of every row Today lists, filtered or not"
        );
        Ok(())
    }

    // FR-VIEW-04, FR-NAV-02
    #[test]
    fn the_trash_lists_deleted_items_and_none_opens() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            inbox_task,
            section,
            site,
            in_site,
            ..
        } = fixture()?;
        state.go(Go::View(View::Trash));
        assert_eq!(state.rows(), []);
        state.empty_trash(false);
        assert_eq!(state.notice(), None, "nothing to ask about");
        state.go(Go::Inbox);
        state.delete(TODAY);
        navigate(&mut state, "Work");
        state.move_focus(Focus::Last);
        state.delete(TODAY);
        state.go(Go::View(View::Trash));
        assert_eq!(
            state.rows(),
            [
                Row::Task {
                    id: inbox_task,
                    section: None
                },
                Row::Project(site)
            ]
        );
        state.move_focus(Focus::Last);
        state.open_in_new_pane();
        assert_eq!(
            state.shown(),
            Shown::View(View::Trash),
            "a deleted project does not open"
        );
        state.restore_selected();
        assert_eq!(
            state.rows(),
            [Row::Task {
                id: inbox_task,
                section: None
            }]
        );
        navigate(&mut state, "Site");
        assert_eq!(state.selection().focused(), Some(in_site));
        // Outside the Trash, nothing is restored or emptied.
        state.restore_selected();
        state.empty_trash(true);
        state.empty_trash(false);
        assert_eq!(state.notice(), None);
        assert_eq!(
            trash::items(state.document()),
            [inbox_task],
            "only the Trash empties"
        );
        assert!(state.document().node(section).is_some());
        state.go(Go::View(View::Trash));
        state.delete(TODAY);
        assert_eq!(state.rows(), []);
        assert_eq!(state.document().node(inbox_task), None, "gone for good");
        state.undo();
        state.empty_trash(false);
        assert_eq!(state.notice(), Some(&Notice::EmptyTrash(1)));
        state.empty_trash(true);
        assert_eq!(state.document().node(inbox_task), None, "emptied");
        Ok(())
    }

    /// Where the keys are: `None` in the sidebar, else the index of the pane with them.
    fn keys(state: &State) -> Option<usize> {
        (!state.sidebar_has_keys()).then(|| state.current_pane())
    }

    // FR-PANE-02, FR-VIEW-10
    #[test]
    fn the_keys_go_round_the_panes_the_sidebar_first() -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        state.open_in_new_pane();
        state.open_in_new_pane();
        assert_eq!(keys(&state), Some(2));
        for (direction, expected) in [
            (Direction::Down, None),
            (Direction::Down, Some(0)),
            (Direction::Up, None),
            (Direction::Up, Some(2)),
            (Direction::Up, Some(1)),
        ] {
            state.cycle_pane(direction);
            assert_eq!(keys(&state), expected, "{direction:?}");
        }
        state.toggle_sidebar();
        state.toggle_sidebar();
        assert!(state.sidebar().is_none());
        state.focus_pane(1);
        state.cycle_pane(Direction::Up);
        assert_eq!(
            keys(&state),
            Some(2),
            "without the sidebar from the first to the last"
        );
        state.cycle_pane(Direction::Down);
        assert_eq!(keys(&state), Some(0), "and back");
        state.focus_pane(0);
        assert!(
            state.sidebar().is_some() && state.sidebar_has_keys(),
            "shown"
        );
        state.focus_pane(0);
        assert_eq!(keys(&state), None);
        Ok(())
    }

    // FR-VIEW-10
    #[test]
    fn browsing_the_sidebar_shows_its_entries_in_the_first_pane() -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        let inbox = Shown::Container(state.document().inbox());
        state.open_in_new_pane();
        state.browse(Direction::Down);
        let shown = |state: &State| -> Vec<Shown> {
            panes(state).into_iter().map(|(shown, _)| shown).collect()
        };
        let today = Shown::View(View::Today);
        assert_eq!(shown(&state), [today, inbox]);
        assert_eq!(keys(&state), Some(1), "the keys stay");
        state.browse(Direction::Up);
        state.browse(Direction::Up);
        assert_eq!(shown(&state), [inbox, inbox], "no further than the first");
        state.go(Go::Back);
        assert_eq!(shown(&state), [inbox, inbox], "the second pane's history");
        state.focus_pane(1);
        state.go(Go::Back);
        assert_eq!(shown(&state), [today, inbox], "the first pane's");
        Ok(())
    }

    /// What each pane shows and focuses, left to right.
    fn panes(state: &State) -> Vec<(Shown, Option<ItemId>)> {
        state
            .panes()
            .map(|pane| (pane.shown(), pane.selection().focused()))
            .collect()
    }

    // FR-PANE-01, FR-PANE-02
    #[test]
    fn the_focused_row_opens_in_a_new_pane_with_its_own_focus_and_history()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            site,
            in_site,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.toggle_show_completed();
        state.pane_mut().selection.focus(Some(site));
        state.open_in_new_pane();
        assert_eq!(state.current_pane(), 1);
        assert_eq!(
            panes(&state),
            [
                (Shown::Container(work), Some(site)),
                (Shown::Container(site), Some(in_site))
            ]
        );
        assert!(state.shows_completed(), "as the pane it opened from");
        state.go(Go::Back);
        assert_eq!(state.container(), Some(site), "a new pane has no history");
        state.open_in_new_pane();
        assert_eq!(
            panes(&state).get(1..),
            Some(&[(Shown::Container(site), Some(in_site)); 2][..]),
            "a task opens where it is"
        );
        state.toggle_show_completed();
        state.focus_pane(1);
        assert_eq!(at(&state), (Some(work), Some(site)));
        assert!(
            state.shows_completed(),
            "each pane shows completed items or not"
        );
        state.go(Go::Back);
        assert_eq!(state.container(), Some(state.document().inbox()));
        Ok(())
    }

    // FR-PANE-02, FR-VIEW-06
    #[test]
    fn a_view_shown_or_a_task_in_it_opens_in_a_new_pane() -> Result<(), PlacementError> {
        let Dated {
            mut state,
            overdue,
            work,
            ..
        } = dated()?;
        state.go(Go::View(View::Today));
        state.pane_mut().selection.focus(Some(overdue));
        state.open_in_new_pane();
        assert_eq!(at(&state), (Some(work), Some(overdue)));
        state.go(Go::View(View::Logbook));
        state.open_in_new_pane();
        assert_eq!(
            state.shown(),
            Shown::View(View::Logbook),
            "with no row focused"
        );
        assert_eq!(state.panes().count(), 3);
        Ok(())
    }

    // FR-PANE-02
    #[test]
    fn panes_are_focused_by_number_or_beside_and_closed() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            site,
            ..
        } = fixture()?;
        let inbox = state.document().inbox();
        state.open_in_new_pane();
        navigate(&mut state, "Work");
        state.open_in_new_pane();
        navigate(&mut state, "Site");
        let shown = |state: &State| -> Vec<Shown> {
            panes(state).into_iter().map(|(shown, _)| shown).collect()
        };
        let [home, work, site] = [inbox, work, site].map(Shown::Container);
        assert_eq!(shown(&state), [home, work, site]);
        for (number, current) in [(1, 0), (0, 0), (4, 0), (3, 2), (2, 1)] {
            state.focus_pane(number);
            assert_eq!(state.current_pane(), current, "pane {number}");
        }
        state.focus_right();
        state.focus_right();
        assert_eq!(state.current_pane(), 2, "the last pane stays");
        state.focus_left();
        assert_eq!(state.shown(), work);
        state.close_pane();
        assert_eq!((shown(&state), state.current_pane()), (vec![home, site], 0));
        state.close_pane();
        assert_eq!((shown(&state), state.current_pane()), (vec![site], 0));
        state.close_pane();
        assert_eq!(shown(&state), [site], "the only pane stays");
        state.focus_left();
        state.focus_right();
        assert_eq!(state.current_pane(), 0);
        state.set_pane_width(40);
        assert_eq!(state.pane_view().width(), Some(40));
        Ok(())
    }

    // FR-PANE-01
    #[test]
    fn a_change_in_one_pane_shows_in_every_pane_at_once() -> Result<(), PlacementError> {
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
        navigate(&mut state, "Site");
        state.open_in_new_pane();
        navigate(&mut state, "Work");
        state.open_in_new_pane();
        state.pane_mut().selection.focus(Some(section));
        state.delete(TODAY);
        assert_eq!(
            panes(&state),
            [
                (Shown::Container(site), Some(in_site)),
                (Shown::Container(work), Some(loose)),
                (Shown::Container(work), Some(site)),
            ],
            "the other panes keep their focus"
        );
        state.delete(TODAY);
        assert_eq!(
            panes(&state),
            [(Shown::Container(work), Some(loose)); 3],
            "a pane showing a container gone to the Trash gives way"
        );
        state.focus_pane(1);
        state.undo();
        assert_eq!(
            state.selection().focused(),
            Some(site),
            "undo restores the selection here"
        );
        state.undo();
        let in_section_shown = state
            .panes()
            .nth(2)
            .is_some_and(|pane| pane.rows().iter().any(|row| row.id() == in_section));
        assert!(in_section_shown);
        let rows = state.panes().nth(1).map(|pane| pane.rows());
        assert_eq!(rows.map(|rows| rows.len()), Some(state.rows().len()));
        Ok(())
    }

    // FR-PANE-01, FR-OPS-08
    #[test]
    fn grouping_in_one_pane_moves_the_focus_on_in_the_others() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            work,
            loose,
            section,
            ..
        } = fixture()?;
        navigate(&mut state, "Work");
        state.pane_mut().selection.focus(Some(loose));
        state.open_in_new_pane();
        state.pane_mut().selection.focus(Some(loose));
        state.group(ItemKind::Project);
        assert_eq!(
            panes(&state).first(),
            Some(&(Shown::Container(work), Some(section))),
            "the grouped task is no row here any more"
        );
        state.edit_input(EditInput::Done);
        assert_eq!(
            panes(&state).first(),
            Some(&(Shown::Container(work), Some(section))),
            "left unnamed, the group is undone and the focus stays"
        );
        state.focus_pane(1);
        state.pane_mut().selection.focus(Some(loose));
        state.focus_pane(2);
        state.group(ItemKind::Project);
        for c in "P".chars() {
            state.edit_input(EditInput::Text(TextEdit::Insert(c)));
        }
        state.edit_input(EditInput::Done);
        assert_eq!(
            panes(&state).first(),
            Some(&(Shown::Container(work), Some(section)))
        );
        Ok(())
    }

    // FR-PANE-02, FR-VIEW-07
    #[test]
    fn the_navigator_opens_a_place_in_a_new_pane_and_move_to_does_not() -> Result<(), PlacementError>
    {
        let Fixture {
            mut state,
            site,
            in_site,
            ..
        } = fixture()?;
        state.toggle_navigator();
        for c in "Site".chars() {
            state.navigator_input(PickerInput::Edit(TextEdit::Insert(c)));
        }
        state.navigator_input(PickerInput::ChooseInNewPane);
        assert_eq!(state.current_pane(), 1);
        assert_eq!(at(&state), (Some(site), Some(in_site)));
        assert!(state.navigator().is_none());
        state.open_move_to();
        state.move_to_input(PickerInput::ChooseInNewPane);
        assert!(state.move_to().is_some(), "Move to opens nothing in a pane");
        state.move_to_input(PickerInput::Dismiss);
        state.open_tags();
        state.tags_input(PickerInput::ChooseInNewPane);
        assert!(state.tags().is_some(), "nor does the tag picker");
        assert_eq!(state.panes().count(), 2);
        Ok(())
    }
}
