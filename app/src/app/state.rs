//! The application state a frontend renders, changed only by [`update`](super::update).

use std::path::PathBuf;

use jiff::civil::Date;

use super::{
    Go, History, Picked, Picker, PickerInput, PickerView, Selection, completion,
    container::{self, Row},
    focus::{self, Focus},
    navigator::{self, Place},
    visits::{Visit, Visits},
};
use crate::domain::{Document, ItemId};

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
    document: Document,
    opened: Opened,
    container: ItemId,
    selection: Selection,
    history: History,
    /// Whether the pane shows completed items (FR-VIEW-11); per pane once there are several.
    show_completed: bool,
    visits: Visits,
    navigator: Option<Picker<Place>>,
}

impl State {
    /// The app showing `document`, opened from `opened`, at home: the Inbox, its first row
    /// focused (FR-NAV-02).
    #[must_use]
    pub fn new(document: Document, opened: Opened) -> Self {
        let mut state = Self {
            container: document.inbox(),
            document,
            opened,
            selection: Selection::default(),
            history: History::default(),
            show_completed: false,
            visits: Visits::default(),
            navigator: None,
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

    /// Moves the focus as `motion` says, selecting only the newly focused row.
    pub(super) fn move_focus(&mut self, motion: Focus) {
        let target = focus::target(&self.rows(), self.selection.focused(), motion);
        self.selection.focus(target);
    }

    /// Completes the selected items on `today`, or uncompletes them when all are completed, as
    /// one step to undo, moving the focus on when its row hides.
    pub(super) fn toggle_completed(&mut self, today: Date) {
        let transaction = completion::toggle(&self.document, self.selection.items(), today);
        let before = self.rows();
        let (container, show_completed) = (self.container, self.show_completed);
        let focused = self.selection.focused();
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
        self.settle(&before);
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

    /// Keeps the focus on a row now that the rows `before` have changed.
    fn settle(&mut self, before: &[Row]) {
        let focused = focus::kept(before, &self.rows(), self.selection.focused());
        if focused != self.selection.focused() {
            self.selection.focus(focused);
        }
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

    /// Opens the Navigator on where the pane is, or closes it when it is open.
    pub(super) fn toggle_navigator(&mut self) {
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
    /// first row otherwise.
    fn arrive(&mut self, visit: Visit) {
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
        assert_eq!(state.selection().items().collect::<Vec<_>>(), [last]);
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
}
