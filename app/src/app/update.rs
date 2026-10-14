//! The update function: the one place messages change the state, dispatching each command to the
//! module that owns it.

use std::time::Duration;

use jiff::civil::Date;

use super::{
    AttachmentsInput, Command, ConflictSide, DateInput, Direction, EditInput, Notice, OnDisk,
    PickerInput, Report, Side, State, autosave::Ending,
};
use crate::domain::{Branch, ItemKind};

/// A user intent or a result the frontend reports back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// The user gave a command.
    Command(Command),
    /// The user did something in the open Navigator (FR-VIEW-07).
    Navigator(PickerInput),
    /// The user did something in the open Move to picker (FR-OPS-04).
    MoveTo(PickerInput),
    /// The user did something in the open tag picker (FR-TAG-02, FR-TAG-04).
    Tags(PickerInput),
    /// The user did something while editing a title or a description (FR-EDIT-01).
    Edit(EditInput),
    /// The user did something in the open date prompt (FR-DATE-02).
    DatePrompt(DateInput),
    /// The user did something in the open attachments panel (FR-ATT-01).
    Attachments(AttachmentsInput),
    /// The frontend's clock reads this, counted from any fixed moment, such as the start.
    Tick(Duration),
    /// The process was told to end, as by `SIGTERM` or `SIGHUP`: save first, then end anyway
    /// (F-FILE-05).
    Terminate,
    /// How disk work the core asked for went.
    Disk(Report),
    /// The file on disk is no longer the version the core last read or wrote (F-FILE-06).
    OnDisk(OnDisk),
    /// These items, as the clipboard holds them, are pasted on this side of the focus
    /// (FR-OPS-05); the answer to [`Effect::ReadClipboard`], or text pasted into the terminal.
    Paste(Side, Vec<Branch>),
}

/// Work the frontend carries out for the core, which never does I/O itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// End the session, after the effects before it.
    Quit,
    /// Write the document as it is now to its file, unless the file on disk is no longer the
    /// version last read or written, after the file's daily backup; report how it went as
    /// [`Message::Disk`], after what is on disk as [`Message::OnDisk`] when it is refused.
    Save,
    /// Write `side` to a conflict copy of the file and report where as [`Message::Disk`]. When
    /// the session is `ending`, a copy that fails goes to a recovery file instead, and the report
    /// comes before the next effect.
    Copy {
        /// The version to keep.
        side: ConflictSide,
        /// Whether the session ends once it is kept.
        ending: bool,
    },
    /// The document is now the one the file on disk was last reported to hold; write it the way
    /// that file was read from now on.
    Reloaded,
    /// Put these items on the clipboard (FR-OPS-05).
    SetClipboard(Vec<Branch>),
    /// Read the items on the clipboard and paste them on this side of the focus with
    /// [`Message::Paste`] (FR-OPS-05).
    ReadClipboard(Side),
    /// Scroll the shortcuts panel a page this way, as far as it goes, while it shows; how much a
    /// page is depends on how the frontend draws it (FR-KEY-03).
    ScrollShortcuts(Direction),
}

/// Applies `message` to `state` on `today`, the local date by the frontend's clock, and returns
/// the effects the frontend should carry out, in order. A command or a paste given in edit mode
/// ends it first, so that the edit is kept before anything else happens; while the sidebar has the
/// keys, it takes the commands it acts on. Every message is followed by what saving asks for after
/// it.
pub fn update(state: &mut State, message: Message, today: Date) -> Vec<Effect> {
    state.set_today(today);
    let mut effects = dispatch(state, message, today);
    state.settle_sidebar();
    effects.extend(state.step_saving());
    effects
}

/// Applies `message` to `state` by the module that owns it.
fn dispatch(state: &mut State, message: Message, today: Date) -> Vec<Effect> {
    let emptying = matches!(state.notice(), Some(Notice::EmptyTrash(_)));
    if let Message::Command(_) | Message::Paste(..) = message {
        state.finish_editing();
        state.clear_notice();
    }
    if let Message::Command(command) = message
        && state.sidebar_command(command, today)
    {
        return Vec::new();
    }
    match message {
        Message::Command(Command::Quit) => return state.end(Ending::Quit),
        Message::Command(Command::KeepMine) => return state.keep_mine(),
        Message::Command(Command::TakeTheirs) => return state.take_theirs(),
        Message::Command(Command::ShowDiff) => state.toggle_diff(),
        Message::Tick(now) => state.tick(now),
        Message::Terminate => return state.end(Ending::Terminate),
        Message::Disk(report) => return state.disk_report(report),
        Message::OnDisk(change) => return state.on_disk(change),
        Message::Command(Command::Focus(motion)) => state.move_focus(motion),
        Message::Command(Command::Extend(direction)) => state.extend_selection(direction),
        Message::Command(Command::TogglePicked) => state.toggle_picked(),
        Message::Command(Command::SelectAll) => state.select_all(),
        Message::Command(Command::ClearSelection) => state.clear_selection(),
        Message::Command(Command::ToggleCompleted) => state.toggle_completed(today),
        Message::Command(Command::ShowCompleted) => state.toggle_show_completed(),
        Message::Command(Command::Undo) => state.undo(),
        Message::Command(Command::Redo) => state.redo(),
        Message::Command(Command::Go(to)) => state.go(to),
        Message::Command(Command::Navigator) => state.toggle_navigator(),
        Message::Command(Command::EditTitle) => state.edit_title(),
        Message::Command(Command::NewTask(side)) => state.new_task(side),
        Message::Command(Command::NewSection(side)) => {
            state.new_container(ItemKind::Section, side);
        }
        Message::Command(Command::NewProject(side)) => {
            state.new_container(ItemKind::Project, side);
        }
        Message::Command(Command::NewArea(side)) => state.new_container(ItemKind::Area, side),
        Message::Command(Command::SetDate(kind)) => state.open_date_prompt(kind, today),
        Message::Command(Command::StartToday) => state.start_today(today),
        Message::Command(Command::Delete) => state.delete(today),
        Message::Command(Command::Restore) => state.restore_selected(),
        Message::Command(Command::EmptyTrash) => state.empty_trash(emptying),
        Message::Command(Command::MoveTo) => state.open_move_to(),
        Message::Command(Command::Duplicate) => state.duplicate(),
        Message::Command(Command::Copy) => return state.copy(),
        Message::Command(Command::Cut) => return state.cut(),
        Message::Command(Command::Paste(side)) => return vec![Effect::ReadClipboard(side)],
        Message::Paste(side, branches) => state.paste(side, &branches),
        Message::Command(Command::RenameContainer) => state.rename_container(),
        Message::Command(Command::DeleteContainer) => state.delete_container(today),
        Message::Command(Command::Move(direction)) => state.move_item(direction),
        Message::Command(Command::Indent) => state.indent(),
        Message::Command(Command::Outdent) => state.outdent(),
        Message::Command(Command::Convert) => state.convert(),
        Message::Command(Command::GroupIntoSection) => state.group(ItemKind::Section),
        Message::Command(Command::GroupIntoProject) => state.group(ItemKind::Project),
        Message::Command(Command::EditDescription(whose)) => state.edit_description(whose),
        Message::Command(Command::OpenNote(whose)) => state.open_note(whose),
        Message::Command(Command::Tags) => state.open_tags(),
        Message::Command(Command::FilterTags) => state.open_tag_filter(),
        Message::Command(Command::ClearTagFilter) => state.clear_tag_filter(),
        Message::Command(Command::Sidebar) => state.toggle_sidebar(),
        Message::Command(Command::FocusSidebar) => state.focus_sidebar(true),
        Message::Command(Command::FocusPane) => state.focus_sidebar(false),
        Message::Command(Command::Shortcuts) => state.toggle_shortcuts(),
        Message::Command(Command::ScrollShortcuts(direction)) => {
            return vec![Effect::ScrollShortcuts(direction)];
        }
        Message::Navigator(input) => state.navigator_input(input),
        Message::MoveTo(input) => state.move_to_input(input),
        Message::Tags(input) => state.tags_input(input),
        Message::Edit(input) => state.edit_input(input),
        Message::DatePrompt(input) => state.date_prompt_input(input),
        Message::Attachments(input) => state.attachments_input(input),
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::{
        Direction, Focus, Go, Notice, Opened, Refusal, Side, Stop, TextEdit,
        view::{Shown, View},
    };
    use crate::domain::{
        Completion, Document, Item, ItemKind, Node, PlacementError, dates::DateKind,
    };

    const TODAY: Date = date(2026, 10, 14);

    fn state() -> Result<State, PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        for title in ["a", "b"] {
            document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, title)))?;
        }
        Ok(State::new(document, Opened::Demo, TODAY))
    }

    #[test]
    fn quitting_the_demo_asks_the_frontend_to_end_the_session_and_leaves_the_document_alone()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        let before = state.clone();
        let effects = update(&mut state, Message::Command(Command::Quit), TODAY);
        assert_eq!(effects, [Effect::Quit]);
        assert_eq!(state.document(), before.document());
        assert_eq!(state.rows(), before.rows());
        Ok(())
    }

    // FR-NAV-01
    #[test]
    fn a_motion_moves_the_focus_with_no_effect() -> Result<(), PlacementError> {
        let mut state = state()?;
        let last = state.rows().last().map(|row| row.id());
        let effects = update(
            &mut state,
            Message::Command(Command::Focus(Focus::Last)),
            TODAY,
        );
        assert_eq!(effects, []);
        assert_eq!(state.selection().focused(), last);
        Ok(())
    }

    fn give(state: &mut State, command: Command) {
        assert_eq!(update(state, Message::Command(command), TODAY), []);
    }

    fn titles(state: &State) -> Vec<&str> {
        state
            .rows()
            .iter()
            .filter_map(|row| Some(state.document().node(row.id())?.item()?.title.as_str()))
            .collect()
    }

    fn focused(state: &State) -> Option<&str> {
        let id = state.selection().focused()?;
        Some(state.document().node(id)?.item()?.title.as_str())
    }

    // FR-TASK-02, FR-VIEW-11, FR-EDIT-06
    #[test]
    fn completing_hides_the_task_and_undo_and_redo_take_it_back_and_again()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        let a = state.selection().focused();
        give(&mut state, Command::ToggleCompleted);
        assert_eq!(titles(&state), ["b"]);
        assert_eq!(focused(&state), Some("b"));
        let completion = a
            .and_then(|id| state.document().node(id))
            .and_then(Node::item)
            .and_then(|item| item.completed);
        assert_eq!(completion, Some(Completion::On(TODAY)));
        give(&mut state, Command::Undo);
        assert_eq!(titles(&state), ["a", "b"]);
        assert_eq!(focused(&state), Some("a"));
        give(&mut state, Command::Redo);
        assert_eq!(titles(&state), ["b"]);
        assert_eq!(focused(&state), Some("b"));
        give(&mut state, Command::Redo);
        assert_eq!(titles(&state), ["b"]);
        Ok(())
    }

    // FR-VIEW-11
    #[test]
    fn shown_completed_items_come_last_and_hiding_them_moves_the_focus_on()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::ShowCompleted);
        assert!(state.shows_completed());
        give(&mut state, Command::ToggleCompleted);
        assert_eq!(titles(&state), ["b", "a"]);
        assert_eq!(focused(&state), Some("a"));
        give(&mut state, Command::ShowCompleted);
        assert!(!state.shows_completed());
        assert_eq!(titles(&state), ["b"]);
        assert_eq!(focused(&state), Some("b"));
        give(&mut state, Command::ShowCompleted);
        give(&mut state, Command::Focus(Focus::Last));
        give(&mut state, Command::ToggleCompleted);
        assert_eq!(titles(&state), ["a", "b"]);
        assert_eq!(focused(&state), Some("a"));
        Ok(())
    }

    // FR-EDIT-06
    #[test]
    fn undo_keeps_the_focus_on_a_row_shown() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::Undo);
        assert_eq!(focused(&state), Some("a"));
        give(&mut state, Command::ShowCompleted);
        give(&mut state, Command::ToggleCompleted);
        give(&mut state, Command::ToggleCompleted);
        give(&mut state, Command::ShowCompleted);
        give(&mut state, Command::Undo);
        assert_eq!(titles(&state), ["b"]);
        assert_eq!(focused(&state), Some("b"));
        Ok(())
    }

    // FR-EDIT-01, FR-EDIT-05, FR-OPS-05
    #[test]
    fn editing_adding_and_deleting_change_the_state_and_a_command_ends_editing_first()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::NewTask(Side::Above));
        for message in [
            Message::Edit(EditInput::Text(TextEdit::Insert('n'))),
            Message::Command(Command::Focus(Focus::Last)),
        ] {
            assert_eq!(update(&mut state, message, TODAY), []);
        }
        assert!(state.editing().is_none());
        assert_eq!(titles(&state), ["n", "a", "b"]);
        assert_eq!(focused(&state), Some("b"));
        give(&mut state, Command::Delete);
        assert_eq!(titles(&state), ["n", "a"]);
        give(&mut state, Command::EditTitle);
        assert!(state.editing().is_some());
        let effects = update(&mut state, Message::Command(Command::Quit), TODAY);
        assert_eq!(effects, [Effect::Quit]);
        assert!(state.editing().is_none(), "quitting keeps the edit");
        Ok(())
    }

    // FR-OPS-02, FR-OPS-03
    #[test]
    fn moving_and_indenting_change_the_state_with_no_effect() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::Move(Direction::Down));
        assert_eq!(titles(&state), ["b", "a"]);
        assert_eq!(focused(&state), Some("a"));
        give(&mut state, Command::Indent);
        assert_eq!(state.notice(), Some(&Notice::Refused(Refusal::DoesNotNest)));
        give(&mut state, Command::Outdent);
        assert_eq!(state.notice(), Some(&Notice::Refused(Refusal::DoesNotNest)));
        give(&mut state, Command::Move(Direction::Up));
        assert_eq!(state.notice(), None);
        assert_eq!(titles(&state), ["a", "b"]);
        let mut empty = State::new(Document::new(), Opened::Demo, TODAY);
        let before = empty.clone();
        for command in [
            Command::Move(Direction::Up),
            Command::Indent,
            Command::Outdent,
        ] {
            give(&mut empty, command);
        }
        assert_eq!(empty, before, "with nothing focused nothing happens");
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        for title in ["A", "B"] {
            document.push(Some(area), Node::Item(Item::new(ItemKind::Project, title)))?;
        }
        let mut state = State::new(document, Opened::Demo, TODAY);
        for message in [
            Message::Command(Command::Navigator),
            Message::Navigator(PickerInput::Edit(TextEdit::Insert('W'))),
            Message::Navigator(PickerInput::Choose),
            Message::Command(Command::Focus(Focus::Last)),
            Message::Command(Command::Indent),
        ] {
            assert_eq!(update(&mut state, message, TODAY), []);
        }
        assert_eq!(titles(&state), ["A", "B"], "v1 nests no project in another");
        assert_eq!(state.notice(), Some(&Notice::Refused(Refusal::DoesNotNest)));
        Ok(())
    }

    /// The title of the sidebar's focused entry, and of every container's entry after the Inbox,
    /// which every sidebar lists first, indented by depth.
    fn sidebar(state: &State) -> (Option<String>, Vec<String>) {
        let view = state.sidebar().expect("the sidebar shows");
        let title = |stop: &Stop| match *stop {
            Stop::Place(place) => state
                .document()
                .node(place.id)
                .and_then(Node::item)
                .map_or(String::new(), |item| item.title.clone()),
            Stop::View(view) => view.title().to_owned(),
        };
        let focused = view
            .focused
            .and_then(|index| view.entries.get(index))
            .map(title);
        let entries = view
            .entries
            .iter()
            .skip(1)
            .filter(|stop| matches!(stop, Stop::Place(_)))
            .map(|stop| format!("{}{}", "  ".repeat(stop.depth()), title(stop)))
            .collect();
        (focused, entries)
    }

    /// Moves the sidebar's focus from the Inbox past the views listed after it.
    fn past_the_views(state: &mut State) {
        let views = state.sidebar().map_or(0, |view| {
            let views = view.entries.iter();
            views.filter(|stop| matches!(stop, Stop::View(_))).count()
        });
        for _ in 0..=views {
            give(state, Command::Focus(Focus::Next));
        }
    }

    fn type_into(state: &mut State, text: &str) {
        for c in text.chars() {
            let message = Message::Edit(EditInput::Text(TextEdit::Insert(c)));
            assert_eq!(update(state, message, TODAY), []);
        }
    }

    fn edit(state: &mut State, input: EditInput) {
        assert_eq!(update(state, Message::Edit(input), TODAY), []);
    }

    // FR-VIEW-10, FR-OPS-02, FR-OPS-03, FR-EDIT-06
    #[test]
    fn the_sidebar_moves_indents_and_outdents_its_focused_entry() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        for title in ["A", "B"] {
            document.push(Some(work), Node::Item(Item::new(ItemKind::Project, title)))?;
        }
        let mut state = State::new(document, Opened::Demo, TODAY);
        assert_eq!(state.sidebar(), None);
        give(&mut state, Command::Sidebar);
        assert!(state.sidebar_has_keys());
        past_the_views(&mut state);
        assert_eq!(
            sidebar(&state),
            (
                Some("Work".into()),
                vec!["Work".into(), "  A".into(), "  B".into()]
            )
        );
        give(&mut state, Command::Focus(Focus::Next));
        give(&mut state, Command::Move(Direction::Down));
        assert_eq!(sidebar(&state).1, ["Work", "  B", "  A"]);
        give(&mut state, Command::Undo);
        assert_eq!(
            sidebar(&state),
            (
                Some("A".into()),
                vec!["Work".into(), "  A".into(), "  B".into()]
            )
        );
        give(&mut state, Command::Indent);
        assert_eq!(
            state.notice(),
            Some(&Notice::Refused(Refusal::DoesNotNest)),
            "v1 nests no project in another"
        );
        give(&mut state, Command::Outdent);
        assert_eq!(sidebar(&state).1, ["A", "Work", "  B"], "out of its area");
        give(&mut state, Command::Undo);
        give(&mut state, Command::MoveTo);
        assert!(
            state.move_to().is_some(),
            "Move to opens for the focused entry"
        );
        update(&mut state, Message::MoveTo(PickerInput::Dismiss), TODAY);
        give(&mut state, Command::Focus(Focus::First));
        past_the_views(&mut state);
        give(&mut state, Command::Outdent);
        assert_eq!(state.notice(), Some(&Notice::Refused(Refusal::DoesNotNest)));
        let before = state.document().clone();
        for command in [
            Command::NewSection(Side::Below),
            Command::ShowCompleted,
            Command::ToggleCompleted,
            Command::Go(Go::Up),
        ] {
            give(&mut state, command);
        }
        assert_eq!(state.document(), &before, "other commands do nothing");
        assert!(!state.shows_completed());
        give(&mut state, Command::Go(Go::Into));
        assert_eq!(state.container(), Some(work), "opened in the pane");
        assert!(state.sidebar_has_keys(), "the sidebar keeps the keys");
        give(&mut state, Command::Go(Go::Home));
        assert_eq!(
            state.container(),
            Some(state.document().inbox()),
            "going home passes"
        );
        give(&mut state, Command::EditTitle);
        assert_eq!(state.container(), Some(work), "Enter opens it");
        assert!(
            !state.sidebar_has_keys() && state.sidebar().is_some(),
            "Enter gives the keys to the pane"
        );
        give(&mut state, Command::FocusSidebar);
        give(&mut state, Command::Navigator);
        assert!(state.navigator().is_some(), "the Navigator passes");
        Ok(())
    }

    // FR-TAG-01, FR-TAG-02, FR-VIEW-10
    #[test]
    fn the_sidebar_tags_its_focused_entry() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Focus(Focus::Last));
        assert_eq!(sidebar(&state).0.as_deref(), Some("Work"));
        give(&mut state, Command::Tags);
        let typed = Message::Tags(PickerInput::Edit(TextEdit::Insert('x')));
        assert_eq!(update(&mut state, typed, TODAY), []);
        let chosen = Message::Tags(PickerInput::Choose);
        assert_eq!(update(&mut state, chosen, TODAY), []);
        let tags = state
            .document()
            .node(work)
            .and_then(Node::item)
            .map(|area| area.tags.clone());
        assert_eq!(tags, Some(vec![crate::domain::Tag::plain("x")]));
        Ok(())
    }

    // FR-VIEW-10, FR-EDIT-05, FR-ORG-06, FR-OPS-05
    #[test]
    fn the_sidebar_adds_renames_and_deletes_without_moving_the_pane() -> Result<(), PlacementError>
    {
        let mut state = state()?;
        let mut document = state.document().clone();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        state = State::new(document, Opened::Demo, TODAY);
        let pane = state.selection().clone();
        give(&mut state, Command::Sidebar);
        past_the_views(&mut state);
        give(&mut state, Command::RenameContainer);
        type_into(&mut state, "s");
        edit(&mut state, EditInput::Next);
        assert!(state.editing().is_none(), "Enter only ends editing");
        assert_eq!(sidebar(&state).1, ["Works"]);
        assert_eq!(state.selection(), &pane, "the pane keeps its focus");
        give(&mut state, Command::NewTask(Side::Below));
        assert_eq!(
            sidebar(&state),
            (Some(String::new()), vec!["Works".into(), "  ".into()])
        );
        edit(&mut state, EditInput::Done);
        assert_eq!(
            sidebar(&state),
            (Some("Works".into()), vec!["Works".into()])
        );
        give(&mut state, Command::NewArea(Side::Above));
        type_into(&mut state, "Home");
        give(&mut state, Command::Focus(Focus::Next));
        assert_eq!(
            sidebar(&state),
            (Some("Works".into()), vec!["Home".into(), "Works".into()])
        );
        assert_eq!(state.selection(), &pane);
        give(&mut state, Command::Go(Go::Into));
        give(&mut state, Command::Delete);
        assert_eq!(sidebar(&state), (Some("Home".into()), vec!["Home".into()]));
        assert_eq!(
            state.container(),
            Some(state.document().inbox()),
            "the pane leaves it"
        );
        give(&mut state, Command::Undo);
        assert_eq!(state.document().position(work), Some((None, 1)));
        let before = state.document().clone();
        give(&mut state, Command::Paste(Side::Below));
        let branch = crate::domain::Branch {
            node: Node::Item(Item::new(ItemKind::Task, "pasted")),
            children: Vec::new(),
        };
        let pasted = Message::Paste(Side::Below, vec![branch]);
        assert_eq!(update(&mut state, pasted, TODAY), []);
        assert_eq!(
            state.document(),
            &before,
            "nothing is pasted from the sidebar"
        );
        give(&mut state, Command::FocusPane);
        assert!(!state.sidebar_has_keys() && state.sidebar().is_some());
        give(&mut state, Command::Focus(Focus::Last));
        assert_eq!(focused(&state), Some("b"), "the pane has the keys");
        give(&mut state, Command::FocusSidebar);
        assert!(state.sidebar_has_keys());
        give(&mut state, Command::Sidebar);
        assert_eq!(state.sidebar(), None);
        Ok(())
    }

    // FR-VIEW-10, FR-EDIT-05
    #[test]
    fn the_sidebar_lists_the_inbox_first_and_only_opens_it() {
        let mut state = State::new(Document::new(), Opened::Demo, TODAY);
        give(&mut state, Command::Sidebar);
        assert_eq!(sidebar(&state), (Some("Inbox".into()), vec![]));
        let before = state.document().clone();
        for command in [
            Command::RenameContainer,
            Command::Delete,
            Command::Move(Direction::Up),
            Command::Indent,
            Command::Outdent,
            Command::Convert,
            Command::MoveTo,
            Command::StartToday,
            Command::Go(Go::Into),
            Command::Tags,
            Command::Focus(Focus::Next),
        ] {
            give(&mut state, command);
        }
        assert_eq!(state.document(), &before, "the Inbox stays as it is");
        assert!(state.editing().is_none() && state.move_to().is_none());
        give(&mut state, Command::NewProject(Side::Above));
        type_into(&mut state, "P");
        edit(&mut state, EditInput::Done);
        assert_eq!(sidebar(&state), (Some("P".into()), vec!["P".into()]));
        let inbox = state.document().inbox();
        give(&mut state, Command::Go(Go::Into));
        assert_ne!(state.container(), Some(inbox));
        give(&mut state, Command::Focus(Focus::First));
        give(&mut state, Command::Go(Go::Into));
        assert_eq!(state.container(), Some(inbox), "Go Into opens the Inbox");
        give(&mut state, Command::Go(Go::Back));
        give(&mut state, Command::EditTitle);
        assert_eq!(state.container(), Some(inbox), "and so does Enter");
        assert!(!state.sidebar_has_keys(), "Enter gives the pane the keys");
    }

    /// The dates of the Inbox task titled `title`, which the Inbox view leaves out once it has a
    /// start date (FR-VIEW-01).
    fn dates(state: &State, title: &str) -> (Option<Date>, Option<Date>) {
        let document = state.document();
        let item = document
            .children(document.inbox())
            .iter()
            .filter_map(|&id| document.node(id)?.item())
            .find(|item| item.title == title)
            .cloned()
            .expect("a row titled so");
        (item.start, item.due)
    }

    fn prompt(state: &mut State, input: DateInput) {
        assert_eq!(update(state, Message::DatePrompt(input), TODAY), []);
    }

    fn type_date(state: &mut State, text: &str) {
        for c in text.chars() {
            prompt(state, DateInput::Edit(TextEdit::Insert(c)));
        }
    }

    // FR-DATE-01, FR-DATE-02, FR-KEY-06, FR-OPS-06, FR-EDIT-06
    #[test]
    fn the_date_prompt_sets_and_removes_dates_of_the_selection_one_step_each()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::SelectAll);
        give(&mut state, Command::SetDate(DateKind::Due));
        type_date(&mut state, "fri");
        assert_eq!(
            state.date_prompt().map(|view| view.date),
            Some(Ok(Some(date(2026, 10, 16))))
        );
        prompt(&mut state, DateInput::Apply);
        assert!(state.date_prompt().is_none());
        assert_eq!(dates(&state, "a"), (None, Some(date(2026, 10, 16))));
        assert_eq!(dates(&state, "b"), (None, Some(date(2026, 10, 16))));
        give(&mut state, Command::Undo);
        assert_eq!(dates(&state, "a"), (None, None));
        assert_eq!(dates(&state, "b"), (None, None));
        give(&mut state, Command::Redo);
        give(&mut state, Command::ClearSelection);
        give(&mut state, Command::SetDate(DateKind::Due));
        let text = state.date_prompt().map(|view| view.field.text().to_owned());
        assert_eq!(text.as_deref(), Some("2026-10-16"));
        prompt(&mut state, DateInput::Edit(TextEdit::DeleteBackward));
        prompt(&mut state, DateInput::Apply);
        assert_eq!(dates(&state, "a"), (None, None), "blank removes it");
        assert_eq!(dates(&state, "b"), (None, Some(date(2026, 10, 16))));
        Ok(())
    }

    // FR-DATE-02
    #[test]
    fn the_date_prompt_stays_open_on_a_text_that_is_no_date_and_closes_on_cancel()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::SetDate(DateKind::Start));
        type_date(&mut state, "soon");
        let before = state.document().clone();
        prompt(&mut state, DateInput::Apply);
        assert!(state.date_prompt().is_some());
        prompt(&mut state, DateInput::Cancel);
        assert!(state.date_prompt().is_none());
        assert_eq!(state.document(), &before);
        prompt(&mut state, DateInput::Apply);
        assert_eq!(
            state.document(),
            &before,
            "nothing happens while it is closed"
        );
        give(&mut state, Command::SetDate(DateKind::Start));
        give(&mut state, Command::Navigator);
        assert!(state.date_prompt().is_none(), "the Navigator closes it");
        assert!(state.navigator().is_some());
        let mut empty = State::new(Document::new(), Opened::Demo, TODAY);
        give(&mut empty, Command::SetDate(DateKind::Start));
        assert!(empty.date_prompt().is_none(), "nothing to date");
        Ok(())
    }

    // FR-KEY-06, FR-OPS-06
    #[test]
    fn start_today_starts_the_selection_today() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::StartToday);
        assert_eq!(dates(&state, "a"), (Some(TODAY), None));
        assert_eq!(dates(&state, "b"), (None, None));
        give(&mut state, Command::SelectAll);
        give(&mut state, Command::StartToday);
        assert_eq!(dates(&state, "b"), (Some(TODAY), None));
        give(&mut state, Command::Undo);
        assert_eq!(dates(&state, "a"), (Some(TODAY), None));
        assert_eq!(dates(&state, "b"), (None, None));
        Ok(())
    }

    // FR-DATE-01, FR-DATE-05, FR-VIEW-10
    #[test]
    fn the_sidebar_dates_its_focused_entry() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        let dated = |state: &State| {
            let item = state.document().node(work).and_then(Node::item);
            item.map(|item| (item.start, item.due))
        };
        give(&mut state, Command::Sidebar);
        past_the_views(&mut state);
        give(&mut state, Command::StartToday);
        assert_eq!(dated(&state), Some((Some(TODAY), None)));
        give(&mut state, Command::SetDate(DateKind::Due));
        type_date(&mut state, "tomorrow");
        prompt(&mut state, DateInput::Apply);
        assert_eq!(dated(&state), Some((Some(TODAY), Some(date(2026, 10, 15)))));
        let mut empty = State::new(Document::new(), Opened::Demo, TODAY);
        give(&mut empty, Command::Sidebar);
        let before = empty.clone();
        give(&mut empty, Command::SetDate(DateKind::Due));
        give(&mut empty, Command::StartToday);
        assert_eq!(empty, before, "an empty sidebar dates nothing");
        Ok(())
    }

    // FR-VIEW-10, FR-VIEW-06
    #[test]
    fn the_sidebar_lists_the_views_after_the_inbox_and_only_opens_them()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        let mut task = Item::new(ItemKind::Task, "due");
        task.due = Some(TODAY);
        document.push(Some(work), Node::Item(task))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Focus(Focus::Next));
        assert_eq!(sidebar(&state).0.as_deref(), Some("Today"));
        let before = state.clone();
        for command in [
            Command::RenameContainer,
            Command::Delete,
            Command::Move(Direction::Down),
            Command::Indent,
            Command::Outdent,
            Command::Convert,
            Command::MoveTo,
            Command::SetDate(DateKind::Due),
            Command::StartToday,
            Command::Tags,
        ] {
            give(&mut state, command);
        }
        assert_eq!(state, before, "a view is only opened");
        give(&mut state, Command::Go(Go::Into));
        assert_eq!(state.shown(), Shown::View(View::Today));
        assert!(state.sidebar_has_keys(), "the keys stay in the sidebar");
        give(&mut state, Command::Go(Go::Home));
        give(&mut state, Command::EditTitle);
        assert_eq!(state.shown(), Shown::View(View::Today));
        assert_eq!(focused(&state), Some("due"));
        assert!(!state.sidebar_has_keys(), "Enter gives the pane the keys");
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::NewProject(Side::Below));
        type_into(&mut state, "P");
        edit(&mut state, EditInput::Done);
        assert_eq!(
            sidebar(&state).1,
            ["P", "Work"],
            "from a view a project goes first among those in no area, as from the Inbox"
        );
        Ok(())
    }

    // FR-NAV-02, FR-VIEW-07
    #[test]
    fn going_and_the_navigator_change_the_state_with_no_effect() -> Result<(), PlacementError> {
        let mut state = state()?;
        let mut document = state.document().clone();
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        state = State::new(document, Opened::Demo, TODAY);
        for message in [
            Message::Command(Command::Navigator),
            Message::Navigator(PickerInput::Edit(TextEdit::Insert('W'))),
            Message::Navigator(PickerInput::Choose),
        ] {
            assert_eq!(update(&mut state, message, TODAY), []);
        }
        assert_eq!(state.container(), Some(area));
        let effects = update(&mut state, Message::Command(Command::Go(Go::Home)), TODAY);
        assert_eq!(effects, []);
        assert_eq!(state.container(), Some(state.document().inbox()));
        Ok(())
    }

    // FR-TASK-06, FR-ORG-06, FR-OPS-07, FR-OPS-08, FR-EDIT-06
    #[test]
    fn converting_and_grouping_change_the_state_with_no_effect() -> Result<(), PlacementError> {
        let mut state = state()?;
        for (command, refusal) in [
            (Command::Convert, Refusal::NoRoomForProject),
            (Command::GroupIntoSection, Refusal::NoRoomForSection),
            (Command::GroupIntoProject, Refusal::NoRoomForProject),
        ] {
            give(&mut state, command);
            assert_eq!(
                state.notice(),
                Some(&Notice::Refused(refusal)),
                "{command:?}"
            );
        }
        let mut empty = State::new(Document::new(), Opened::Demo, TODAY);
        let before = empty.clone();
        for command in [
            Command::Convert,
            Command::GroupIntoSection,
            Command::GroupIntoProject,
        ] {
            give(&mut empty, command);
        }
        assert_eq!(empty, before, "with nothing selected nothing happens");
        let mut document = Document::new();
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        for title in ["a", "b", "c"] {
            document.push(Some(area), Node::Item(Item::new(ItemKind::Task, title)))?;
        }
        let mut state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Sidebar);
        past_the_views(&mut state);
        for command in [
            Command::Go(Go::Into),
            Command::FocusPane,
            Command::Extend(Direction::Down),
            Command::GroupIntoSection,
        ] {
            give(&mut state, command);
        }
        assert_eq!(titles(&state), ["c", "", "a", "b"]);
        type_into(&mut state, "S");
        edit(&mut state, EditInput::Done);
        assert_eq!(titles(&state), ["c", "S", "a", "b"]);
        assert_eq!(focused(&state), Some("S"));
        give(&mut state, Command::Undo);
        assert_eq!(titles(&state), ["a", "b", "c"], "one step to undo");
        give(&mut state, Command::ClearSelection);
        give(&mut state, Command::Focus(Focus::First));
        give(&mut state, Command::Convert);
        assert_eq!(titles(&state), ["b", "c", "a"]);
        assert_eq!(focused(&state), Some("a"));
        give(&mut state, Command::Convert);
        assert_eq!(titles(&state), ["b", "c", "a"]);
        assert_eq!(
            state.rows().last().map(|row| row.id()),
            state.selection().focused()
        );
        give(&mut state, Command::GroupIntoProject);
        assert_eq!(titles(&state), ["b", "c", ""]);
        edit(&mut state, EditInput::Done);
        assert_eq!(
            titles(&state),
            ["b", "c", "a"],
            "left empty, nothing is grouped"
        );
        give(&mut state, Command::FocusSidebar);
        give(&mut state, Command::Convert);
        assert_eq!(
            state
                .document()
                .node(area)
                .and_then(Node::item)
                .map(|it| it.kind),
            Some(ItemKind::Project)
        );
        give(&mut state, Command::Convert);
        assert_eq!(
            state.notice(),
            Some(&Notice::Refused(Refusal::NoRoomForSection)),
            "a top-level project can't be a section"
        );
        Ok(())
    }
}
