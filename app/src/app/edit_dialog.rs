//! The edit dialog: every property of one area, project, section or task but its attachments,
//! changed in a draft in a modal window and applied as one step to undo, as taskpaper-terminal's
//! Edit Node dialog and Things 3's task window do
//! ([editing.md §7](../../../specs/reference/taskpaper-terminal/specs/editing.md#7-edit-node-dialog-fr-mode-09),
//! FR-EDIT-08). Hides the draft and which fields each kind of item has.

use jiff::civil::Date;

use super::{
    DatePromptView, Motion, Notice, Picked, PickerInput, State, TagsView, TextEdit, TextField,
    date_prompt,
    editing::{self, Refused},
    tags::{Choice, TagPicker},
};
use crate::domain::{
    Document, Item, ItemId, ItemKind, Node, Op, Tag, Transaction,
    dates::{DateError, DateKind},
};

/// A field of the dialog, which the focus can be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    /// The title, on one line.
    Title,
    /// The description, on any number of lines (FR-ORG-10).
    Description,
    /// The user's tags (FR-TAG-01).
    Tags,
    /// The start date (FR-DATE-01).
    Start,
    /// The due date (FR-DATE-01).
    Due,
    /// Whether a project is pinned (FR-VIEW-12).
    Pinned,
}

/// What the user does in the open dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditDialogInput {
    /// Focus the next field, stopping at the last.
    Next,
    /// Focus the previous field, stopping at the first.
    Previous,
    /// A text key on the focused field: it edits the title, the description or a date; moving up
    /// from the description's first line or down from its last focuses the field before or
    /// after it; on the tags a space opens the tag picker, another character opens it with the
    /// character typed and deleting removes the last tag; on Pinned a space pins or unpins.
    Edit(TextEdit),
    /// Apply the changes and close, unless the title is empty or a date field holds no date.
    Accept,
    /// Close, changing nothing.
    Cancel,
    /// An input to the tag picker open over the dialog.
    Tags(PickerInput),
}

/// Why the dialog did not apply its changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditProblem {
    /// The title is empty.
    EmptyTitle,
    /// This date field holds text that is no date.
    NotADate(DateKind),
}

/// The open dialog as a frontend shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDialogView<'a> {
    /// What the item edited is.
    pub kind: ItemKind,
    /// The fields it has, in order.
    pub fields: &'static [EditField],
    /// The focused one.
    pub focus: EditField,
    /// The title typed.
    pub title: &'a TextField,
    /// The description typed.
    pub description: &'a TextField,
    /// The user's tags in the draft, by name, in order.
    pub tags: Vec<&'a str>,
    /// The start date typed, with what it resolves to.
    pub start: DatePromptView<'a>,
    /// The due date typed, with what it resolves to.
    pub due: DatePromptView<'a>,
    /// Whether the draft is pinned.
    pub pinned: bool,
    /// The tag picker open over the dialog, if any.
    pub picker: Option<TagsView<'a>>,
    /// Why the last try to apply did not, until the next input.
    pub problem: Option<EditProblem>,
}

/// The open dialog: the item it edits, its draft, the focused field, the local date the dates
/// typed resolve against, the tag picker open over it and why applying last failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditDialog {
    id: ItemId,
    kind: ItemKind,
    focus: EditField,
    title: TextField,
    description: TextField,
    tags: Vec<Tag>,
    start: TextField,
    due: TextField,
    pinned: bool,
    today: Date,
    picker: Option<TagPicker>,
    problem: Option<EditProblem>,
}

/// The fields an item of kind `kind` has in the dialog (FR-EDIT-08), in the order of Things 3's
/// task window: none for the Inbox and a checklist item, which it does not edit.
fn fields(kind: ItemKind) -> &'static [EditField] {
    use EditField::{Description, Due, Pinned, Start, Tags, Title};
    match kind {
        ItemKind::Area | ItemKind::Section => &[Title, Description],
        ItemKind::Project => &[Title, Description, Tags, Start, Due, Pinned],
        ItemKind::Task => &[Title, Description, Tags, Start, Due],
        ItemKind::Inbox | ItemKind::ChecklistItem => &[],
    }
}

/// The text a date field opens with: `date` as the file writes it, or nothing.
fn date_field(date: Option<Date>) -> TextField {
    TextField::new(date.map(|date| date.to_string()).unwrap_or_default())
}

impl EditDialog {
    /// The dialog on item `id` of `document`, its title focused with the caret at its end, the
    /// dates typed resolving against `today`; `None` unless it is an area, project, section or
    /// task.
    pub(super) fn open(document: &Document, id: ItemId, today: Date) -> Option<Self> {
        let item = document.node(id).and_then(Node::item)?;
        let &[focus, ..] = fields(item.kind) else {
            return None;
        };
        Some(Self {
            id,
            kind: item.kind,
            focus,
            title: TextField::new(item.title.clone()),
            description: TextField::multi_line(&item.description),
            tags: item.tags.clone(),
            start: date_field(item.start),
            due: date_field(item.due),
            pinned: item.pinned,
            today,
            picker: None,
            problem: None,
        })
    }

    /// The dialog as a frontend shows it.
    pub(super) fn view(&self) -> EditDialogView<'_> {
        let mut tags: Vec<&str> = Vec::new();
        for tag in self.tags.iter().filter(|tag| tag.is_users()) {
            if !tags.contains(&tag.name.as_str()) {
                tags.push(&tag.name);
            }
        }
        EditDialogView {
            kind: self.kind,
            fields: fields(self.kind),
            focus: self.focus,
            title: &self.title,
            description: &self.description,
            tags,
            start: self.date_view(DateKind::Start),
            due: self.date_view(DateKind::Due),
            pinned: self.pinned,
            picker: self.picker.as_ref().map(TagPicker::view),
            problem: self.problem,
        }
    }

    /// The `kind` date field as a frontend shows it.
    fn date_view(&self, kind: DateKind) -> DatePromptView<'_> {
        let field = match kind {
            DateKind::Start => &self.start,
            DateKind::Due => &self.due,
        };
        DatePromptView {
            kind,
            field,
            date: date_prompt::resolve(field.text(), self.today, kind),
        }
    }

    /// Focuses the field `step` fields after the focused one, or before it when negative,
    /// stopping at the first and the last; a date field focused has its text selected, so that
    /// typing replaces it.
    fn step(&mut self, step: isize) {
        let fields = fields(self.kind);
        let at = fields.iter().position(|&field| field == self.focus);
        let to = at.unwrap_or_default().saturating_add_signed(step);
        if let Some(&focus) = fields.get(to.min(fields.len().saturating_sub(1)))
            && focus != self.focus
        {
            self.focus = focus;
            for (field, date) in [
                (EditField::Start, &mut self.start),
                (EditField::Due, &mut self.due),
            ] {
                if focus == field {
                    *date = date.edited(TextEdit::SelectAll);
                }
            }
        }
    }

    /// Applies the text key `edit` to the focused field, as [`EditDialogInput::Edit`] says, with
    /// the tag picker listing the tags of `document`.
    ///
    /// # Errors
    ///
    /// Why an edit to the title was refused, when it was.
    fn edit(&mut self, document: &Document, edit: TextEdit) -> Result<(), Refused> {
        match self.focus {
            EditField::Title => {
                let edited = self.title.edited(edit);
                editing::title_edit(self.kind, self.title.text(), edited.text())?;
                self.title = edited;
            }
            EditField::Description => self.edit_description(edit),
            EditField::Tags => match edit {
                TextEdit::Insert(c) => {
                    let mut picker = TagPicker::for_draft(document, &self.tags);
                    if c != ' ' {
                        picker.input(document, PickerInput::Edit(edit));
                    }
                    self.picker = Some(picker);
                }
                TextEdit::DeleteBackward | TextEdit::DeleteForward => {
                    if let Some(last) = self.tags.iter().rfind(|tag| tag.is_users()) {
                        let name = last.name.clone();
                        self.toggle_tag(&name);
                    }
                }
                _ => {}
            },
            EditField::Start => self.start = self.start.edited(edit),
            EditField::Due => self.due = self.due.edited(edit),
            EditField::Pinned => {
                if edit == TextEdit::Insert(' ') {
                    self.pinned = !self.pinned;
                }
            }
        }
        Ok(())
    }

    /// Applies `edit` to the description, or focuses the field before it for moving up from its
    /// first line on screen, or the one after it for moving down from its last.
    fn edit_description(&mut self, edit: TextEdit) {
        let leaves = |width: usize, first: bool| {
            let lines = self.description.screen_lines(width);
            let caret = self.description.caret();
            let line = if first { lines.first() } else { lines.last() };
            line.is_some_and(|line| line.contains(&caret) || line.end == caret)
        };
        match edit {
            TextEdit::Move {
                motion: Motion::Up { width },
                select: false,
            } if leaves(width, true) => self.step(-1),
            TextEdit::Move {
                motion: Motion::Down { width },
                select: false,
            } if leaves(width, false) => self.step(1),
            _ => self.description = self.description.edited(edit),
        }
    }

    /// Takes the user's tag `name` off the draft when it carries it, or puts it on, bare, after
    /// the others when it does not.
    fn toggle_tag(&mut self, name: &str) {
        let carried = |tag: &Tag| tag.is_users() && tag.name == name;
        if self.tags.iter().any(carried) {
            self.tags.retain(|tag| !carried(tag));
        } else {
            self.tags.push(Tag::plain(name));
        }
    }

    /// Applies `input` to the tag picker open over the dialog: choosing a tag closes it and puts
    /// the tag on the draft or takes it off; closing it changes nothing.
    fn tags_input(&mut self, document: &Document, input: PickerInput) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        match picker.input(document, input) {
            Some(Picked::Chosen(Choice::Draft(name))) => {
                self.picker = None;
                self.toggle_tag(&name);
            }
            Some(_) => self.picker = None,
            None => {}
        }
    }

    /// The item as the draft makes `item`, its title trimmed and its description without the
    /// space and empty lines at its end, as editing them in place writes them (D-43).
    ///
    /// # Errors
    ///
    /// Why the draft cannot apply: the title is empty or a date field holds no date.
    fn applied(&self, item: &Item) -> Result<Item, EditProblem> {
        let title = self.title.text().trim();
        if title.is_empty() {
            return Err(EditProblem::EmptyTitle);
        }
        let date = |kind| {
            let view = self.date_view(kind);
            view.date
                .map_err(|_: DateError| EditProblem::NotADate(kind))
        };
        let has = |field| fields(self.kind).contains(&field);
        let mut after = Item {
            title: title.to_owned(),
            description: self.description.text().trim_end().to_owned(),
            ..item.clone()
        };
        if has(EditField::Tags) {
            after.tags.clone_from(&self.tags);
        }
        if has(EditField::Start) {
            after.start = date(DateKind::Start)?;
        }
        if has(EditField::Due) {
            after.due = date(DateKind::Due)?;
        }
        if has(EditField::Pinned) {
            after.pinned = self.pinned;
        }
        Ok(after)
    }

    /// The dialog on the item of a reloaded document that `exact` finds for its own, with its
    /// draft and the tag picker made again for `document`, or `None` when it finds none.
    pub(super) fn rematched(
        self,
        document: &Document,
        exact: impl Fn(ItemId) -> Option<ItemId>,
    ) -> Option<Self> {
        let id = exact(self.id)?;
        let picker =
            (self.picker.as_ref()).and_then(|picker| picker.reopen(document, Vec::new(), &[], &[]));
        Some(Self { id, picker, ..self })
    }
}

impl State {
    /// The edit dialog, while it is open (FR-EDIT-08).
    #[must_use]
    pub fn edit_dialog(&self) -> Option<EditDialogView<'_>> {
        self.edit_dialog.as_ref().map(EditDialog::view)
    }

    /// Opens the edit dialog on the focused row's item, or on the container shown while its
    /// heading has the focus (FR-EDIT-08). Nothing opens on the Inbox or without a focus.
    pub(super) fn open_edit_dialog(&mut self) {
        if let Some(id) = self.pane().selection.focused() {
            self.open_edit_dialog_on(id);
        }
    }

    /// Opens the edit dialog on `id`, closing the Navigator, Move to and the tag picker, when it
    /// is an area, project, section or task.
    pub(super) fn open_edit_dialog_on(&mut self, id: ItemId) {
        if let Some(dialog) = EditDialog::open(&self.document, id, self.today) {
            self.navigator = None;
            self.move_to = None;
            self.tags = None;
            self.edit_dialog = Some(dialog);
        }
    }

    /// Applies `input` to the open edit dialog: accepting applies the draft as one step to undo
    /// and closes it, unless the draft cannot apply, which keeps it open on the field to fix,
    /// saying why; a draft that changes nothing makes no step. Nothing happens while it is
    /// closed.
    pub(super) fn edit_dialog_input(&mut self, input: EditDialogInput) {
        let Some(dialog) = self.edit_dialog.as_mut() else {
            return;
        };
        dialog.problem = None;
        match input {
            EditDialogInput::Next => dialog.step(1),
            EditDialogInput::Previous => dialog.step(-1),
            EditDialogInput::Edit(edit) => {
                if dialog.edit(&self.document, edit) == Err(Refused::Tag) {
                    self.notify(Notice::TagInTitle);
                }
            }
            EditDialogInput::Tags(input) => dialog.tags_input(&self.document, input),
            EditDialogInput::Cancel => self.edit_dialog = None,
            EditDialogInput::Accept => {
                let before = (self.document.node(dialog.id))
                    .and_then(Node::item)
                    .expect("a reload closes the dialog unless it finds its item");
                match dialog.applied(before) {
                    Ok(after) => {
                        let id = dialog.id;
                        let transaction = if after == *before {
                            Transaction::default()
                        } else {
                            Transaction::new(vec![Op::Set {
                                id,
                                before: before.clone(),
                                after,
                            }])
                        };
                        self.edit_dialog = None;
                        self.change(transaction);
                    }
                    Err(problem) => {
                        dialog.focus = match problem {
                            EditProblem::EmptyTitle => EditField::Title,
                            EditProblem::NotADate(DateKind::Start) => EditField::Start,
                            EditProblem::NotADate(DateKind::Due) => EditField::Due,
                        };
                        dialog.problem = Some(problem);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::{Command, Focus, Message, Opened, TagMark, update};
    use crate::domain::PlacementError;
    use ItemKind::{Area, Project, Section, Task};

    /// A Monday.
    const TODAY: Date = date(2026, 10, 5);

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn item(document: &Document, id: ItemId) -> &Item {
        document.node(id).and_then(Node::item).expect("an item")
    }

    /// The Inbox holding task `call`, tagged `@phone` and `@home(1)` and starting today, so
    /// shown in Today, and task `buy`, tagged `@errands`, which the Inbox focuses; area `work` holding project `site`, which holds section
    /// `launch`.
    struct Sample {
        document: Document,
        call: ItemId,
        buy: ItemId,
        work: ItemId,
        site: ItemId,
        launch: ItemId,
    }

    fn sample() -> Result<Sample, PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let mut call = Item::new(Task, "Call the bank");
        call.tags = vec![
            Tag::plain("phone"),
            Tag {
                name: "home".to_owned(),
                value: Some("1".to_owned()),
            },
            Tag::plain("start"),
        ];
        call.start = Some(TODAY);
        let call = document.push(inbox, Node::Item(call))?;
        let mut buy = Item::new(Task, "Buy beans");
        buy.tags.push(Tag::plain("errands"));
        let buy = document.push(inbox, Node::Item(buy))?;
        let work = document.push(None, Node::Item(Item::new(Area, "Work")))?;
        let site = document.push(Some(work), Node::Item(Item::new(Project, "Site")))?;
        let launch = document.push(Some(site), Node::Item(Item::new(Section, "Launch")))?;
        Ok(Sample {
            document,
            call,
            buy,
            work,
            site,
            launch,
        })
    }

    fn dialog(document: &Document, id: ItemId) -> Result<EditDialog, &'static str> {
        EditDialog::open(document, id, TODAY).ok_or("the dialog opens")
    }

    /// Types `text` into `dialog`, into its tag picker while that is open, as a frontend does.
    fn typed(dialog: &mut EditDialog, document: &Document, text: &str) {
        for c in text.chars() {
            let edit = TextEdit::Insert(c);
            if dialog.picker.is_some() {
                dialog.tags_input(document, PickerInput::Edit(edit));
            } else {
                assert_eq!(dialog.edit(document, edit), Ok(()));
            }
        }
    }

    // FR-EDIT-08
    #[test]
    fn each_kind_has_the_owners_fields_and_the_inbox_and_checklists_none() -> TestResult {
        use EditField::{Description, Due, Pinned, Start, Tags, Title};
        let mut sample = sample()?;
        let check = Node::Item(Item::new(ItemKind::ChecklistItem, "c"));
        let check = sample.document.push(Some(sample.call), check)?;
        for (id, fields) in [
            (sample.work, &[Title, Description][..]),
            (sample.site, &[Title, Description, Tags, Start, Due, Pinned]),
            (sample.launch, &[Title, Description]),
            (sample.call, &[Title, Description, Tags, Start, Due]),
        ] {
            let open = dialog(&sample.document, id)?;
            let view = open.view();
            assert_eq!(view.fields, fields);
            assert_eq!(view.focus, Title);
        }
        for id in [sample.document.inbox(), check, ItemId::new(99)] {
            assert_eq!(EditDialog::open(&sample.document, id, TODAY), None);
        }
        Ok(())
    }

    // FR-EDIT-08, FR-TAG-01
    #[test]
    fn the_dialog_shows_the_items_properties_and_the_users_tags_once_each() -> TestResult {
        let mut sample = sample()?;
        let mut tagged = item(&sample.document, sample.call).clone();
        tagged.tags.push(Tag::plain("phone"));
        tagged.due = Some(date(2026, 10, 9));
        tagged.description = "Before noon".to_owned();
        sample.document.set(sample.call, tagged)?;
        let open = dialog(&sample.document, sample.call)?;
        let view = open.view();
        assert_eq!(view.kind, Task);
        assert_eq!(view.title.text(), "Call the bank");
        assert_eq!(view.description.text(), "Before noon");
        assert_eq!(view.tags, ["phone", "home"], "@start is not the user's");
        assert_eq!(view.start.field.text(), "2026-10-05");
        assert_eq!(view.start.date, Ok(Some(TODAY)));
        assert_eq!(view.due.date, Ok(Some(date(2026, 10, 9))));
        assert_eq!(
            (view.pinned, view.picker, view.problem),
            (false, None, None)
        );
        Ok(())
    }

    // FR-EDIT-08
    #[test]
    fn the_focus_steps_through_the_fields_selecting_a_date_and_stops_at_either_end() -> TestResult {
        let sample = sample()?;
        let mut open = dialog(&sample.document, sample.call)?;
        open.step(-1);
        assert_eq!(open.focus, EditField::Title, "the first stays");
        open.step(1);
        open.step(1);
        open.step(1);
        assert_eq!(open.focus, EditField::Start);
        assert_eq!(open.start.selection(), Some(0..10), "typing replaces it");
        open.step(1);
        assert_eq!(open.due.selection(), None, "an empty one selects nothing");
        open.step(1);
        assert_eq!(open.focus, EditField::Due, "the last stays");
        Ok(())
    }

    // FR-EDIT-08, FR-EDIT-04
    #[test]
    fn the_title_takes_text_keys_but_no_tag_and_a_containers_no_dash() -> TestResult {
        let sample = sample()?;
        let mut open = dialog(&sample.document, sample.call)?;
        typed(&mut open, &sample.document, " now@ ");
        assert_eq!(
            open.edit(&sample.document, TextEdit::Insert('@')),
            Err(Refused::Tag)
        );
        assert_eq!(open.view().title.text(), "Call the bank now@ ");
        let mut site = dialog(&sample.document, sample.site)?;
        site.edit(
            &sample.document,
            TextEdit::Move {
                motion: Motion::Start,
                select: false,
            },
        )
        .map_err(|_| "refused")?;
        typed(&mut site, &sample.document, "-");
        assert_eq!(
            site.edit(&sample.document, TextEdit::Insert(' ')),
            Err(Refused::Task)
        );
        Ok(())
    }

    // FR-EDIT-08, FR-ORG-10
    #[test]
    fn the_description_breaks_lines_and_moving_past_its_ends_leaves_it() -> TestResult {
        let sample = sample()?;
        let mut open = dialog(&sample.document, sample.call)?;
        open.step(1);
        typed(&mut open, &sample.document, "one\ntwo words");
        assert_eq!(open.view().description.text(), "one\ntwo words");
        let go = |motion| TextEdit::Move {
            motion,
            select: false,
        };
        open.edit(&sample.document, go(Motion::Up { width: 4 }))
            .map_err(|_| "refused")?;
        assert_eq!(
            open.focus,
            EditField::Description,
            "`two ` wraps from `words`"
        );
        open.edit(&sample.document, go(Motion::Up { width: 4 }))
            .map_err(|_| "refused")?;
        assert_eq!(open.focus, EditField::Description);
        let select = TextEdit::Move {
            motion: Motion::Up { width: 4 },
            select: true,
        };
        open.edit(&sample.document, select).map_err(|_| "refused")?;
        assert_eq!(open.focus, EditField::Description, "selecting stays");
        open.edit(&sample.document, go(Motion::Start))
            .map_err(|_| "refused")?;
        open.edit(&sample.document, go(Motion::Up { width: 4 }))
            .map_err(|_| "refused")?;
        assert_eq!(open.focus, EditField::Title);
        open.step(1);
        open.edit(&sample.document, go(Motion::End))
            .map_err(|_| "refused")?;
        open.edit(&sample.document, go(Motion::Down { width: 4 }))
            .map_err(|_| "refused")?;
        assert_eq!(open.focus, EditField::Tags);
        Ok(())
    }

    // FR-EDIT-08, FR-TAG-02
    #[test]
    fn a_character_on_the_tags_opens_the_tag_picker_which_toggles_a_tag() -> TestResult {
        let sample = sample()?;
        let document = &sample.document;
        let mut open = dialog(document, sample.call)?;
        open.step(2);
        open.edit(document, TextEdit::Insert(' '))
            .map_err(|_| "refused")?;
        let picker = open.view().picker.ok_or("Space opens the picker")?;
        let marks: Vec<_> = (picker.picker.entries.iter())
            .map(|entry| (entry.value.name.as_str(), entry.value.mark))
            .collect();
        assert_eq!(
            marks,
            [
                ("errands", TagMark::Off),
                ("home", TagMark::All),
                ("phone", TagMark::All)
            ]
        );
        open.tags_input(document, PickerInput::Choose);
        assert_eq!(open.view().tags, ["phone", "home", "errands"]);
        assert!(open.picker.is_none(), "choosing closes it");
        typed(&mut open, document, "ho");
        open.tags_input(document, PickerInput::Next);
        open.tags_input(document, PickerInput::Choose);
        assert_eq!(
            open.view().tags,
            ["phone", "errands"],
            "valued, gone by name"
        );
        typed(&mut open, document, "new one");
        assert_eq!(
            open.view().picker.map(|view| view.picker.query.text()),
            Some("new one")
        );
        open.tags_input(document, PickerInput::Choose);
        assert_eq!(open.view().tags, ["phone", "errands", "new-one"]);
        open.edit(document, TextEdit::Insert('x'))
            .map_err(|_| "refused")?;
        open.tags_input(document, PickerInput::Dismiss);
        assert!(open.picker.is_none());
        open.tags_input(document, PickerInput::Choose);
        assert_eq!(open.view().tags.len(), 3, "nothing is open to choose in");
        open.edit(document, TextEdit::DeleteBackward)
            .map_err(|_| "refused")?;
        open.edit(document, TextEdit::DeleteForward)
            .map_err(|_| "refused")?;
        assert_eq!(open.view().tags, ["phone"]);
        open.edit(document, TextEdit::SelectAll)
            .map_err(|_| "refused")?;
        open.edit(document, TextEdit::DeleteBackward)
            .map_err(|_| "refused")?;
        open.edit(document, TextEdit::DeleteBackward)
            .map_err(|_| "refused")?;
        assert_eq!(open.view().tags, [] as [&str; 0]);
        assert_eq!(open.tags, [Tag::plain("start")], "the app's own tags stay");
        Ok(())
    }

    // FR-EDIT-08, FR-VIEW-12
    #[test]
    fn space_pins_a_project_and_other_keys_do_nothing_there() -> TestResult {
        let sample = sample()?;
        let mut open = dialog(&sample.document, sample.site)?;
        open.step(5);
        assert_eq!(open.focus, EditField::Pinned);
        open.edit(&sample.document, TextEdit::Insert('x'))
            .map_err(|_| "refused")?;
        assert!(!open.view().pinned);
        open.edit(&sample.document, TextEdit::Insert(' '))
            .map_err(|_| "refused")?;
        assert!(open.view().pinned);
        Ok(())
    }

    /// Gives `input` to the edit dialog of `state`.
    fn input(state: &mut State, input: EditDialogInput) {
        assert_eq!(update(state, Message::EditDialog(input), TODAY), []);
    }

    fn type_into(state: &mut State, text: &str) {
        for c in text.chars() {
            input(state, EditDialogInput::Edit(TextEdit::Insert(c)));
        }
    }

    // FR-EDIT-08, FR-EDIT-06, FR-DATE-02
    #[test]
    fn accepting_applies_every_change_as_one_step_to_undo() -> TestResult {
        let sample = sample()?;
        let (buy, before) = (sample.buy, item(&sample.document, sample.buy).clone());
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        assert_eq!(
            update(&mut state, Message::Command(Command::EditItem), TODAY),
            []
        );
        type_into(&mut state, "  ");
        input(&mut state, EditDialogInput::Next);
        type_into(&mut state, "Ask about fees\n\n");
        input(&mut state, EditDialogInput::Next);
        input(&mut state, EditDialogInput::Edit(TextEdit::DeleteBackward));
        type_into(&mut state, " ");
        input(&mut state, EditDialogInput::Tags(PickerInput::Dismiss));
        input(&mut state, EditDialogInput::Next);
        type_into(&mut state, "tomorrow");
        input(&mut state, EditDialogInput::Next);
        type_into(&mut state, "fri");
        input(&mut state, EditDialogInput::Accept);
        assert_eq!(state.edit_dialog(), None);
        let after = item(state.document(), buy);
        assert_eq!(after.title, "Buy beans", "trimmed");
        assert_eq!(
            after.description, "Ask about fees",
            "without its empty lines"
        );
        assert_eq!(after.tags, []);
        assert_eq!(after.start, Some(date(2026, 10, 6)));
        assert_eq!(after.due, Some(date(2026, 10, 9)));
        state.undo();
        assert_eq!(item(state.document(), buy), &before, "one step");
        Ok(())
    }

    // FR-EDIT-08
    #[test]
    fn a_draft_with_no_title_or_no_date_stays_open_on_the_field_to_fix() -> TestResult {
        let sample = sample()?;
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        let revision = state.document().revision();
        state.open_edit_dialog();
        input(&mut state, EditDialogInput::Edit(TextEdit::SelectAll));
        input(&mut state, EditDialogInput::Edit(TextEdit::DeleteBackward));
        input(&mut state, EditDialogInput::Next);
        input(&mut state, EditDialogInput::Accept);
        let view = state.edit_dialog().ok_or("still open")?;
        assert_eq!(
            (view.focus, view.problem),
            (EditField::Title, Some(EditProblem::EmptyTitle))
        );
        type_into(&mut state, "Call");
        assert_eq!(
            state.edit_dialog().ok_or("open")?.problem,
            None,
            "until the next input"
        );
        for (steps, kind, field) in [
            (3, DateKind::Start, EditField::Start),
            (4, DateKind::Due, EditField::Due),
        ] {
            for _ in 0..steps {
                input(&mut state, EditDialogInput::Next);
            }
            type_into(&mut state, "soon");
            input(&mut state, EditDialogInput::Previous);
            input(&mut state, EditDialogInput::Accept);
            let view = state.edit_dialog().ok_or("still open")?;
            assert_eq!(
                (view.focus, view.problem),
                (field, Some(EditProblem::NotADate(kind)))
            );
            input(&mut state, EditDialogInput::Edit(TextEdit::SelectAll));
            input(&mut state, EditDialogInput::Edit(TextEdit::DeleteBackward));
            for _ in 0..4 {
                input(&mut state, EditDialogInput::Previous);
            }
        }
        input(&mut state, EditDialogInput::Cancel);
        assert_eq!(state.edit_dialog(), None);
        assert_eq!(
            state.document().revision(),
            revision,
            "cancelling changes nothing"
        );
        Ok(())
    }

    // FR-EDIT-08
    #[test]
    fn a_draft_left_as_it_was_makes_no_step_and_the_title_refuses_a_tag_aloud() -> TestResult {
        let sample = sample()?;
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        let revision = state.document().revision();
        state.open_edit_dialog();
        type_into(&mut state, " @");
        assert_eq!(state.notice(), Some(&Notice::TagInTitle));
        input(&mut state, EditDialogInput::Edit(TextEdit::DeleteBackward));
        input(&mut state, EditDialogInput::Accept);
        assert_eq!(state.edit_dialog(), None);
        state.undo();
        assert_eq!(state.document().revision(), revision, "nothing to undo");
        input(&mut state, EditDialogInput::Accept);
        assert_eq!(
            state.edit_dialog(),
            None,
            "nothing happens while it is closed"
        );
        Ok(())
    }

    // FR-EDIT-08, FR-VIEW-10
    #[test]
    fn the_dialog_opens_on_the_focus_the_heading_or_the_sidebars_entry() -> TestResult {
        let sample = sample()?;
        let site = sample.site;
        let mut state = State::new(sample.document, Opened::Demo, TODAY);
        let editing = |state: &State| state.edit_dialog().map(|view| view.title.text().to_owned());
        state.show(crate::app::view::Shown::Container(site), None);
        state.move_focus(Focus::First);
        state.open_edit_dialog();
        assert_eq!(editing(&state).as_deref(), Some("Site"), "its heading");
        state.toggle_navigator();
        assert_eq!(state.edit_dialog(), None, "the Navigator closes it");
        state.toggle_navigator();
        assert_eq!(
            update(&mut state, Message::Command(Command::Sidebar), TODAY),
            []
        );
        assert_eq!(
            update(
                &mut state,
                Message::Command(Command::Focus(Focus::Last)),
                TODAY
            ),
            []
        );
        assert_eq!(
            update(&mut state, Message::Command(Command::EditItem), TODAY),
            []
        );
        assert_eq!(editing(&state).as_deref(), Some("Site"));
        input(&mut state, EditDialogInput::Cancel);
        assert_eq!(
            update(
                &mut state,
                Message::Command(Command::Focus(Focus::Previous)),
                TODAY
            ),
            []
        );
        assert_eq!(
            update(&mut state, Message::Command(Command::EditItem), TODAY),
            []
        );
        assert_eq!(editing(&state).as_deref(), Some("Work"));
        assert_eq!(state.edit_dialog().map(|view| view.kind), Some(Area));
        let mut empty = State::new(Document::new(), Opened::Demo, TODAY);
        empty.open_edit_dialog();
        assert_eq!(empty.edit_dialog(), None, "nothing is focused");
        Ok(())
    }

    // FR-EDIT-08, FR-STORE-07
    #[test]
    fn a_rematched_dialog_keeps_its_draft_and_picker_while_its_item_is_found() -> TestResult {
        let sample = sample()?;
        let document = &sample.document;
        let mut open = dialog(document, sample.call)?;
        typed(&mut open, document, "!");
        open.step(2);
        typed(&mut open, document, "ph");
        let kept = open.clone().rematched(document, Some);
        let kept = kept.ok_or("found")?;
        assert_eq!(kept.view().title.text(), "Call the bank!");
        let query = kept
            .view()
            .picker
            .map(|view| view.picker.query.text().to_owned());
        assert_eq!(query.as_deref(), Some("ph"));
        let marks = kept.view().picker.map(|view| {
            let entries = view.picker.entries.iter();
            entries
                .filter(|entry| entry.value.mark == TagMark::All)
                .count()
        });
        assert_eq!(marks, Some(1), "phone, which the query keeps");
        assert_eq!(open.clone().rematched(document, |_| None), None);
        open.picker = None;
        let closed = open.rematched(document, Some).ok_or("found")?;
        assert!(closed.picker.is_none());
        Ok(())
    }
}
