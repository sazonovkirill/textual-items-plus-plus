//! Quick Add: a one-line panel that adds a task, with the start and due dates its words name, as
//! the last task of the Inbox or of a place chosen in it with `/`, without leaving the place
//! shown, as taskpaper-terminal's Quick Add does (FR-EDIT-07, D-97). Hides how the text becomes
//! a task.

use jiff::civil::Date;

use super::{
    End, Notice, Picked, Picker, PickerInput, PickerView, State, TextEdit, TextField, adding,
    container, editing,
    move_to::{self, Destination},
};
use crate::domain::{
    Item, ItemId, ItemKind, Node, Op, Snapshot, Transaction,
    dates::{self, DateKind, Found},
};

/// What Quick Add opens with besides an empty text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prefill {
    /// Nothing: the task goes to the Inbox unless another place is chosen with `/`.
    Nothing,
    /// A start date of today, which a start date typed replaces.
    StartToday,
    /// The Inbox, chosen as if with `/`.
    Inbox,
}

/// What the user does in the open panel; its picker takes [`PickerInput`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickAddInput {
    /// Changes the text typed; `/` at the start of a word opens the picker of places.
    Edit(TextEdit),
    /// Adds the task, once there is text besides the dates, and closes the panel.
    Add,
    /// Closes the panel, adding nothing.
    Cancel,
}

/// The open panel as a frontend shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickAddView<'a> {
    /// The text typed.
    pub field: &'a TextField,
    /// The dates that count in the text, in order.
    pub found: Vec<Found>,
    /// The title the task gets: the text without those dates.
    pub title: String,
    /// The start date the task gets.
    pub start: Option<Date>,
    /// The due date the task gets.
    pub due: Option<Date>,
    /// The titles from the top level down to the place the task goes, the Inbox when none was
    /// chosen.
    pub place: Vec<String>,
}

/// The open panel: the text, the place chosen, the start date it opened with and the picker of
/// places while `/` has it open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QuickAdd {
    field: TextField,
    place: Option<ItemId>,
    start: Option<Date>,
    picker: Option<Picker<Destination>>,
}

impl QuickAdd {
    /// Keeps the place on the one `exact` finds for it in a reloaded document, the Inbox when it
    /// finds none, and closes the picker, whose places are the old document's.
    pub(super) fn rematch(&mut self, exact: impl Fn(ItemId) -> Option<ItemId>) {
        self.place = self.place.and_then(exact);
        self.picker = None;
    }

    /// Whether `/` typed now opens the picker: nothing is selected and the caret is at the start
    /// of the text or after a space.
    fn at_word_start(&self) -> bool {
        let before = self
            .field
            .text()
            .get(..self.field.caret())
            .unwrap_or_default();
        self.field.selection().is_none()
            && before.chars().next_back().is_none_or(char::is_whitespace)
    }

    /// Applies `edit` to the text, or returns `false` and changes nothing when it would start a
    /// word with `@`, which would read back as a tag (FR-TAG-02).
    fn edit(&mut self, edit: TextEdit) -> bool {
        let edited = self.field.edited(edit);
        if editing::word_starts(edited.text()) > editing::word_starts(self.field.text()) {
            return false;
        }
        self.field = edited;
        true
    }

    /// Types `text` as its characters would be typed; `false` when one was left out for starting
    /// a word with `@`.
    fn type_text(&mut self, text: &str) -> bool {
        let mut typed = true;
        for c in text.chars() {
            typed &= self.edit(TextEdit::Insert(c));
        }
        typed
    }
}

/// `text` without the spans of `found` and the spaces before each, trimmed.
fn without(text: &str, found: &[Found]) -> String {
    let mut out = String::new();
    let mut from = 0;
    for date in found {
        out.push_str(
            text.get(from..date.span.start)
                .unwrap_or_default()
                .trim_end(),
        );
        from = date.span.end;
    }
    out.push_str(text.get(from..).unwrap_or_default());
    out.trim().to_owned()
}

/// The `kind` date among `found`, if any.
fn date(found: &[Found], kind: DateKind) -> Option<Date> {
    found
        .iter()
        .find(|date| date.kind == kind)
        .map(|date| date.date)
}

impl State {
    /// Quick Add, while it is open.
    #[must_use]
    pub fn quick_add(&self) -> Option<QuickAddView<'_>> {
        let quick_add = self.quick_add.as_ref()?;
        let text = quick_add.field.text();
        let found = dates::find(text, self.today);
        let place = quick_add.place.unwrap_or(self.document.inbox());
        Some(QuickAddView {
            field: &quick_add.field,
            title: without(text, &found),
            start: date(&found, DateKind::Start).or(quick_add.start),
            due: date(&found, DateKind::Due),
            place: container::path(&self.document, place)
                .into_iter()
                .filter_map(|id| Some(self.document.node(id)?.item()?.title.clone()))
                .collect(),
            found,
        })
    }

    /// Quick Add's picker of places, while `/` has it open.
    #[must_use]
    pub fn quick_add_picker(&self) -> Option<PickerView<'_, Destination>> {
        self.quick_add.as_ref()?.picker.as_ref().map(Picker::view)
    }

    /// Opens Quick Add with what `prefill` names, its dates read on `today`.
    pub(super) fn open_quick_add(&mut self, prefill: Prefill, today: Date) {
        self.quick_add = Some(QuickAdd {
            field: TextField::new(String::new()),
            place: (prefill == Prefill::Inbox).then(|| self.document.inbox()),
            start: (prefill == Prefill::StartToday).then_some(today),
            picker: None,
        });
    }

    /// Applies `input` to the open panel: adding puts the task in as one step to undo, closes the
    /// panel and says where the task went. Nothing happens while it is closed.
    pub(super) fn quick_add_input(&mut self, input: QuickAddInput) {
        let Some(quick_add) = self.quick_add.as_mut() else {
            return;
        };
        match input {
            // What is pasted after a `/` that opens the picker goes on to it, as typing does.
            QuickAddInput::Edit(edit) if quick_add.picker.is_some() => {
                self.quick_add_picker_input(PickerInput::Edit(edit));
            }
            QuickAddInput::Edit(TextEdit::Insert('/')) if quick_add.at_word_start() => {
                quick_add.picker = Some(Picker::new(move_to::task_places(&self.document)));
            }
            QuickAddInput::Edit(edit) => {
                if !quick_add.edit(edit) {
                    self.notify(Notice::TagInTitle);
                }
            }
            QuickAddInput::Add => self.add_quickly(),
            QuickAddInput::Cancel => self.quick_add = None,
        }
    }

    /// Applies `input` to Quick Add's open picker: choosing makes the place focused the one the
    /// task goes to; `Esc`, or a character after which nothing matches, closes it and types `/`
    /// and the query into the text, and `Backspace` on an empty query closes it.
    pub(super) fn quick_add_picker_input(&mut self, input: PickerInput) {
        let Some(quick_add) = self.quick_add.as_mut() else {
            return;
        };
        let Some(picker) = quick_add.picker.as_mut() else {
            return;
        };
        let literal = match input {
            PickerInput::Edit(TextEdit::DeleteBackward)
                if picker.view().query.text().is_empty() =>
            {
                None
            }
            PickerInput::Dismiss => Some(format!("/{}", picker.view().query.text())),
            PickerInput::Edit(TextEdit::Insert(_)) => {
                picker.input(input);
                if !picker.view().entries.is_empty() {
                    return;
                }
                Some(format!("/{}", picker.view().query.text()))
            }
            PickerInput::Choose | PickerInput::ChooseInNewPane => {
                if let Some(Picked::Chosen(Destination::Container { id, .. })) = picker.input(input)
                {
                    quick_add.place = Some(id);
                }
                None
            }
            PickerInput::Edit(_) | PickerInput::Previous | PickerInput::Next => {
                picker.input(input);
                return;
            }
        };
        quick_add.picker = None;
        if let Some(text) = literal
            && !quick_add.type_text(&text)
        {
            self.notify(Notice::TagInTitle);
        }
    }

    /// Adds the task the open panel holds as the last task of its place, as one step to undo,
    /// and closes the panel, saying where the task went; does nothing while it has no title.
    fn add_quickly(&mut self) {
        let Some(view) = self.quick_add().filter(|view| !view.title.is_empty()) else {
            return;
        };
        let place = self
            .quick_add
            .as_ref()
            .and_then(|quick_add| quick_add.place)
            .unwrap_or(self.document.inbox());
        let (parent, index) = adding::end_place(&self.document, place, ItemKind::Task, End::Bottom)
            .expect("a task goes in the Inbox and in every place the picker lists");
        let item = Item {
            start: view.start,
            due: view.due,
            ..Item::new(ItemKind::Task, view.title)
        };
        let name = view.place.join(" › ");
        let subtree = Snapshot::new(self.document.new_id(), Node::Item(item));
        self.quick_add = None;
        self.change(Transaction::new(vec![Op::Insert {
            parent,
            index,
            subtree,
        }]));
        self.notify(Notice::QuickAdded(name));
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::{
        app::{Message, Opened, update},
        domain::{Document, PlacementError},
    };

    /// A Wednesday.
    const TODAY: Date = date(2026, 10, 14);

    /// The app on a document whose Inbox holds task `old` beside project `site`, which holds
    /// project `blog`, with `site` and `blog`.
    fn state() -> Result<(State, ItemId, ItemId), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        document.push(inbox, Node::Item(Item::new(ItemKind::Task, "old")))?;
        let site = document.push(None, Node::Item(Item::new(ItemKind::Project, "Site")))?;
        let blog = document.push(Some(site), Node::Item(Item::new(ItemKind::Project, "Blog")))?;
        Ok((State::new(document, Opened::Demo, TODAY), site, blog))
    }

    fn input(state: &mut State, input: QuickAddInput) {
        assert_eq!(update(state, Message::QuickAdd(input), TODAY), []);
    }

    fn pick(state: &mut State, input: PickerInput) {
        assert_eq!(update(state, Message::QuickAddPicker(input), TODAY), []);
    }

    fn typed(state: &mut State, text: &str) {
        for c in text.chars() {
            input(state, QuickAddInput::Edit(TextEdit::Insert(c)));
        }
    }

    fn text(state: &State) -> Option<String> {
        state.quick_add().map(|view| view.field.text().to_owned())
    }

    // FR-EDIT-07
    #[test]
    fn the_view_names_the_title_dates_and_place_the_task_gets() -> Result<(), PlacementError> {
        let (mut state, _, _) = state()?;
        state.open_quick_add(Prefill::StartToday, TODAY);
        typed(&mut state, "  Call  due fri  ");
        let view = state.quick_add().ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(view.title, "Call");
        assert_eq!(view.start, Some(TODAY), "prefilled");
        assert_eq!(view.due, Some(date(2026, 10, 16)));
        assert_eq!(view.place, ["Inbox"]);
        assert_eq!(view.found.len(), 1);
        Ok(())
    }

    // FR-EDIT-07
    #[test]
    fn the_picker_moves_its_focus_edits_its_query_and_chooses_in_a_new_pane_alike()
    -> Result<(), PlacementError> {
        let (mut state, site, blog) = state()?;
        state.open_quick_add(Prefill::Nothing, TODAY);
        pick(&mut state, PickerInput::Next);
        assert_eq!(text(&state).as_deref(), Some(""), "no picker, no change");
        typed(&mut state, "/");
        let listed = |state: &State| state.quick_add_picker().map(|view| view.entries.len());
        assert_eq!(listed(&state), Some(3), "the Inbox, Site and Blog");
        pick(&mut state, PickerInput::Next);
        pick(&mut state, PickerInput::Next);
        pick(&mut state, PickerInput::Previous);
        typed(&mut state, "q");
        assert_eq!(listed(&state), None, "nothing matches q");
        assert_eq!(text(&state).as_deref(), Some("/q"));
        typed(&mut state, " /bl");
        let caret = TextEdit::Move {
            motion: crate::app::Motion::Start,
            select: false,
        };
        input(&mut state, QuickAddInput::Edit(caret));
        assert_eq!(
            state.quick_add_picker().map(|view| view.query.caret()),
            Some(0),
            "the panel's edits go to the open picker"
        );
        typed(&mut state, "q");
        assert_eq!(
            text(&state).as_deref(),
            Some("/q /qbl"),
            "the query as it stands"
        );
        typed(&mut state, " /bl");
        pick(&mut state, PickerInput::ChooseInNewPane);
        assert_eq!(
            state
                .quick_add
                .as_ref()
                .and_then(|quick_add| quick_add.place),
            Some(blog)
        );
        typed(&mut state, "/Site");
        pick(&mut state, PickerInput::Choose);
        assert_eq!(
            state.quick_add().map(|view| view.place),
            Some(vec!["Site".to_owned()])
        );
        assert_ne!(site, blog);
        Ok(())
    }

    // FR-EDIT-07, FR-TAG-02
    #[test]
    fn a_query_typed_back_starts_no_word_with_at() -> Result<(), PlacementError> {
        let (mut state, _, _) = state()?;
        state.open_quick_add(Prefill::Nothing, TODAY);
        typed(&mut state, "/i @");
        assert!(state.quick_add_picker().is_none(), "nothing matches `i @`");
        assert_eq!(text(&state).as_deref(), Some("/i "));
        assert_eq!(state.notice(), Some(&Notice::TagInTitle));
        Ok(())
    }

    // FR-EDIT-07, FR-TAG-02
    #[test]
    fn esc_or_backspace_closes_the_picker_and_a_tag_is_refused() -> Result<(), PlacementError> {
        let (mut state, _, _) = state()?;
        state.open_quick_add(Prefill::Nothing, TODAY);
        typed(&mut state, "/");
        pick(&mut state, PickerInput::Edit(TextEdit::DeleteBackward));
        assert!(state.quick_add_picker().is_none());
        assert_eq!(
            text(&state).as_deref(),
            Some(""),
            "Backspace takes the `/` back"
        );
        typed(&mut state, "/si");
        pick(&mut state, PickerInput::Dismiss);
        assert_eq!(
            text(&state).as_deref(),
            Some("/si"),
            "Esc keeps what was typed"
        );
        typed(&mut state, " @");
        assert_eq!(text(&state).as_deref(), Some("/si "));
        assert_eq!(state.notice(), Some(&Notice::TagInTitle));
        Ok(())
    }

    // FR-EDIT-07
    #[test]
    fn enter_adds_nothing_without_a_title_and_esc_closes_the_panel() -> Result<(), PlacementError> {
        let (mut state, _, _) = state()?;
        state.open_quick_add(Prefill::Nothing, TODAY);
        typed(&mut state, "tomorrow");
        let before = state.clone();
        input(&mut state, QuickAddInput::Add);
        assert_eq!(state, before);
        input(&mut state, QuickAddInput::Cancel);
        assert!(state.quick_add().is_none());
        assert_eq!(state.document(), before.document());
        Ok(())
    }

    // FR-EDIT-07
    #[test]
    fn a_task_goes_last_into_the_place_chosen_as_a_does() -> Result<(), PlacementError> {
        let (mut state, site, _) = state()?;
        state.open_quick_add(Prefill::Nothing, TODAY);
        typed(&mut state, "/Site");
        pick(&mut state, PickerInput::Choose);
        typed(&mut state, "Ship it");
        input(&mut state, QuickAddInput::Add);
        assert!(state.quick_add().is_none());
        assert_eq!(state.notice(), Some(&Notice::QuickAdded("Site".to_owned())));
        let last = state.document().children(site).last().copied();
        let title = last
            .and_then(|id| state.document().node(id))
            .and_then(Node::item)
            .map(|item| item.title.as_str());
        assert_eq!(title, Some("Ship it"));
        Ok(())
    }

    // FR-EDIT-07
    #[test]
    fn a_reload_keeps_the_place_found_again_and_closes_the_picker() -> Result<(), PlacementError> {
        let (mut state, site, blog) = state()?;
        state.open_quick_add(Prefill::Inbox, TODAY);
        let mut quick_add = state.quick_add.take().ok_or(PlacementError::DoesNotFit)?;
        quick_add.place = Some(site);
        quick_add.picker = Some(Picker::new(Vec::new()));
        quick_add.rematch(|id| (id == site).then_some(blog));
        assert_eq!(quick_add.place, Some(blog));
        assert!(quick_add.picker.is_none());
        quick_add.rematch(|_| None);
        assert_eq!(quick_add.place, None, "the Inbox then");
        Ok(())
    }

    // FR-EDIT-07
    #[test]
    fn nothing_happens_while_it_is_closed_and_the_navigator_closes_it() -> Result<(), PlacementError>
    {
        let (mut state, _, _) = state()?;
        let before = state.clone();
        input(&mut state, QuickAddInput::Add);
        pick(&mut state, PickerInput::Choose);
        assert_eq!(state, before);
        state.open_quick_add(Prefill::Nothing, TODAY);
        state.toggle_navigator();
        assert!(state.quick_add().is_none());
        Ok(())
    }
}
