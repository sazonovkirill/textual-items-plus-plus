//! The update function: the one place messages change the state, dispatching each command to the
//! module that owns it.

use std::time::Duration;

use jiff::civil::Date;

use super::{
    AttachmentsInput, Command, ConflictSide, CopyId, DateInput, Direction, EditInput, Notice,
    OnDisk, PickerInput, QuickAddInput, Report, SaveAsInput, SearchInput, Selected, Setting,
    SettingsInput, Side, State, autosave::Ending,
};
use crate::domain::{Branch, ItemKind};

/// A user intent or a result the frontend reports back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// The user gave a command.
    Command(Command),
    /// The user did something in the open Navigator (FR-VIEW-07).
    Navigator(PickerInput),
    /// The user did something in the open command palette (FR-NAV-06).
    Palette(PickerInput),
    /// The user did something in the open Go to Symbol (FR-NAV-05).
    GoToSymbol(PickerInput),
    /// The user did something in the open Move to picker (FR-OPS-04).
    MoveTo(PickerInput),
    /// The user did something in the open tag picker (FR-TAG-02, FR-TAG-04).
    Tags(PickerInput),
    /// The user did something while editing a title or a description (FR-EDIT-01).
    Edit(EditInput),
    /// The user did something in the open date prompt (FR-DATE-02).
    DatePrompt(DateInput),
    /// The user did something in the open Quick Add panel (FR-EDIT-07).
    QuickAdd(QuickAddInput),
    /// The user did something in Quick Add's open picker of places (FR-EDIT-07).
    QuickAddPicker(PickerInput),
    /// The user did something while typing the words to search for (FR-SRCH-01).
    Search(SearchInput),
    /// The user did something in the open attachments panel (FR-ATT-01).
    Attachments(AttachmentsInput),
    /// The user did something in the open settings window (FR-SET-02).
    Settings(SettingsInput),
    /// The user did something in the open Save As prompt (FR-STORE-11).
    SaveAs(SaveAsInput),
    /// Something went wrong with the settings file, as the frontend says it: what the file sets
    /// that the app ignored, or that a setting could not be kept (FR-SET-04).
    SettingsFailed(String),
    /// The pane with the keys keeps this width from now on, as the frontend measures it, for
    /// [`Command::Narrow`] and [`Command::Widen`] (FR-PANE-02).
    PaneWidth(u16),
    /// The frontend has no weather to start this season, the answer to [`Effect::StartWeather`]
    /// in summer (FR-UI-08).
    NoWeather,
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
    /// Write `side` to a conflict copy of the file and report where as [`Message::Disk`], naming
    /// it `id`. When the session is `ending`, a copy that fails goes to a recovery file instead,
    /// and the report comes before the next effect.
    Copy {
        /// The version to keep.
        side: ConflictSide,
        /// Whether the session ends once it is kept.
        ending: bool,
        /// Which copy the report tells of.
        id: CopyId,
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
    /// Make the pane with the keys narrower; how wide it is drawn only the frontend knows, so it
    /// answers with the width it keeps from now on as [`Message::PaneWidth`] (FR-PANE-02).
    NarrowPane,
    /// Make the pane with the keys wider, as [`NarrowPane`](Self::NarrowPane) makes it narrower
    /// (FR-PANE-02).
    WidenPane,
    /// Keep this setting as it is now in the settings file, and report a failure as
    /// [`Message::SettingsFailed`] (FR-SET-02, FR-SET-04).
    KeepSetting(Setting),
    /// Write the document to the file at `path`, as the user typed it, where there is none, or
    /// over the file there when told to `replace` it, after keeping it in a conflict copy; from
    /// then on save to that file. Report how it went as [`Message::Disk`], naming the file
    /// written as the frontend resolves `path` (FR-STORE-11).
    SaveAs {
        /// The path typed.
        path: String,
        /// Whether a file there is replaced.
        replace: bool,
    },
    /// Start the weather at once; the season it falls in is the frontend's to tell, and when it
    /// has none it answers [`Message::NoWeather`] (FR-UI-08).
    StartWeather,
}

/// Applies `message` to `state` on `today`, the local date by the frontend's clock, and returns
/// the effects the frontend should carry out, in order. A command or a paste given in edit mode
/// ends it first, so that the edit is kept before anything else happens; while the sidebar has the
/// keys, it takes the commands it acts on. Every message is followed by what saving asks for after
/// it.
pub fn update(state: &mut State, message: Message, today: Date) -> Vec<Effect> {
    state.set_today(today);
    state.history.forget_changed();
    state.forget_refusal();
    let mut effects = dispatch(state, message, today);
    state.settle_sidebar();
    effects.extend(state.step_saving());
    effects
}

/// Applies `message` to `state` by the module that owns it. A command or a paste ends visual
/// line mode, as in vim, unless it moves the focus, extends or toggles the mode, or shows or
/// scrolls the shortcuts panel (D-82).
fn dispatch(state: &mut State, message: Message, today: Date) -> Vec<Effect> {
    let emptying = matches!(state.notice(), Some(Notice::EmptyTrash(_)));
    if let Message::Command(_) | Message::Paste(..) = message {
        state.finish_editing();
        state.finish_searching();
        state.clear_notice();
        let keeps_visual = matches!(
            message,
            Message::Command(
                Command::Focus(_)
                    | Command::Extend(_)
                    | Command::VisualLines
                    | Command::Shortcuts
                    | Command::ScrollShortcuts(_)
            )
        );
        if !keeps_visual {
            state.end_visual();
        }
    }
    if let Message::Command(command) = message
        && (state.panel_command(command, today) || state.sidebar_command(command, today))
    {
        return Vec::new();
    }
    match message {
        Message::Tick(now) => state.tick(now),
        Message::Terminate => return state.end(Ending::Terminate),
        Message::Disk(report) => return state.disk_report(report),
        Message::OnDisk(change) => return state.on_disk(change),
        Message::Paste(side, branches) => state.paste(side, &branches),
        Message::PaneWidth(width) => state.set_pane_width(width),
        Message::Navigator(input) => state.navigator_input(input),
        Message::GoToSymbol(input) => state.go_to_symbol_input(input),
        Message::Palette(input) => {
            if let Some(command) = state.palette_input(input) {
                return dispatch(state, Message::Command(command), today);
            }
        }
        Message::MoveTo(input) => state.move_to_input(input),
        Message::Tags(input) => state.tags_input(input),
        Message::Edit(input) => state.edit_input(input),
        Message::DatePrompt(input) => state.date_prompt_input(input),
        Message::QuickAdd(input) => state.quick_add_input(input),
        Message::QuickAddPicker(input) => state.quick_add_picker_input(input),
        Message::Search(input) => state.search_input(input),
        Message::Attachments(input) => state.attachments_input(input),
        Message::Settings(input) => return state.settings_input(input),
        Message::SaveAs(input) => return state.save_as_input(input),
        Message::SettingsFailed(problem) => state.settings_failed(problem),
        Message::NoWeather => state.notify(Notice::NoWeather),
        Message::Command(command) => return run(state, command, today, emptying),
    }
    Vec::new()
}

/// Carries out `command` on `today` by the module that owns it; `emptying` says whether the Trash
/// was asked to be emptied by the command before.
fn run(state: &mut State, command: Command, today: Date, emptying: bool) -> Vec<Effect> {
    match command {
        Command::Quit => return state.end(Ending::Quit),
        Command::KeepMine => return state.keep_mine(),
        Command::TakeTheirs => return state.take_theirs(),
        Command::ShowDiff => state.toggle_diff(),
        Command::Focus(motion) => state.move_focus(motion),
        Command::Extend(direction) => state.extend_selection(direction),
        Command::TogglePicked => state.toggle_picked(),
        Command::VisualLines => state.toggle_visual(),
        Command::SelectAll => state.select_all(),
        Command::ClearSelection => state.clear_selection(),
        Command::ToggleCompleted => state.toggle_completed(today),
        Command::ShowCompleted => state.toggle_show_completed(),
        Command::Undo => state.undo(),
        Command::Redo => state.redo(),
        Command::Go(to) => state.go(to),
        Command::Navigator => state.toggle_navigator(),
        Command::Palette => state.open_palette(),
        Command::GoToSymbol => state.open_go_to_symbol(),
        Command::Search => state.open_search(),
        Command::QuickAdd(prefill) => state.open_quick_add(prefill, today),
        Command::EditTitle => state.edit_title(),
        Command::NewLike(side) => state.new_like(side),
        Command::AddTask(end) => state.add(ItemKind::Task, end),
        Command::AddSection(end) => state.add(ItemKind::Section, end),
        Command::AddProject(end) | Command::AddProjectInArea(end) => {
            state.add(ItemKind::Project, end);
        }
        Command::SetDate(kind) => state.open_date_prompt(kind, today),
        Command::StartToday => state.start_today(today),
        Command::ToggleLater => state.toggle_later(),
        Command::DayEarlier(kind) => state.nudge_date(kind, -1, today),
        Command::DayLater(kind) => state.nudge_date(kind, 1, today),
        Command::Delete => state.delete(today),
        Command::Restore => state.restore_selected(),
        Command::EmptyTrash => state.empty_trash(emptying),
        Command::MoveTo => state.open_move_to(),
        Command::Duplicate => state.duplicate(),
        Command::Copy => return state.copy(),
        Command::Cut => return state.cut(),
        Command::Paste(side) => return vec![Effect::ReadClipboard(side)],
        Command::RenameContainer => state.rename_container(),
        Command::DeleteContainer => state.delete_container(today),
        Command::Move(direction) => state.move_item(direction),
        Command::Convert => state.convert(),
        Command::GroupIntoSection => state.group(ItemKind::Section),
        Command::GroupIntoProject => state.group(ItemKind::Project),
        Command::EditDescription(whose) => state.edit_description(whose),
        Command::OpenNote(whose) => {
            state.open_attachments(whose, Selected::Note);
        }
        Command::OpenComments(whose) => {
            state.open_attachments(whose, Selected::Comment(Some(0)));
        }
        Command::OpenChecklist => state.open_checklist(),
        Command::Tags => state.open_tags(),
        Command::TogglePinned => state.toggle_pinned(),
        Command::FilterTags => state.open_tag_filter(),
        Command::ClearTagFilter => state.clear_tag_filter(),
        Command::Sidebar => state.toggle_sidebar(),
        // The pane shows a child project as one row, with nothing to fold, and only checklist
        // items nest, in the attachments panel, which takes these first (D-75).
        Command::Collapse | Command::Indent | Command::Outdent => {}
        Command::FocusLeft => state.focus_left(),
        Command::FocusRight => state.focus_right(),
        Command::FocusPane(number) => state.focus_pane(number),
        Command::NextPane => state.cycle_pane(Direction::Down),
        Command::PreviousPane => state.cycle_pane(Direction::Up),
        Command::Browse(direction) => state.browse(direction),
        Command::OpenInNewPane => state.open_in_new_pane(),
        Command::ClosePane => state.close_pane(),
        Command::Narrow => return vec![Effect::NarrowPane],
        Command::Widen => return vec![Effect::WidenPane],
        Command::Shortcuts => state.toggle_shortcuts(),
        Command::ScrollShortcuts(direction) => {
            return vec![Effect::ScrollShortcuts(direction)];
        }
        Command::Settings => state.open_settings(),
        Command::SaveAs => state.open_save_as(),
        Command::StartWeather => return vec![Effect::StartWeather],
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::{
        Direction, End, Focus, Go, Notice, Opened, Part, Refusal, Side, Stop, TextEdit, Whose,
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

    // FR-SRCH-01
    #[test]
    fn searching_takes_the_words_typed_until_a_command_is_given() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::Search);
        assert_eq!(state.shown(), Shown::View(View::Search));
        let effects = update(
            &mut state,
            Message::Search(SearchInput::Edit(TextEdit::Insert('b'))),
            TODAY,
        );
        assert_eq!(effects, []);
        assert_eq!(titles(&state), ["b"]);
        give(&mut state, Command::Navigator);
        assert_eq!(state.search(), None, "a command ends typing");
        assert!(state.navigator().is_some());
        give(&mut state, Command::Navigator);
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Search);
        assert!(
            state.search().is_some(),
            "the sidebar passes it to the pane"
        );
        assert!(!state.sidebar_has_keys());
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
        give(&mut state, Command::NewLike(Side::Above));
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

    // FR-EDIT-01
    #[test]
    fn the_adding_keys_add_at_either_end_of_the_items_kind() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::AddTask(End::Top));
        type_into(&mut state, "t");
        edit(&mut state, EditInput::Done);
        give(&mut state, Command::AddTask(End::Bottom));
        type_into(&mut state, "u");
        edit(&mut state, EditInput::Done);
        assert_eq!(titles(&state), ["t", "a", "b", "u"]);
        give(&mut state, Command::AddSection(End::Bottom));
        assert!(state.editing().is_none(), "the Inbox holds no sections");
        give(&mut state, Command::AddProject(End::Top));
        assert!(state.editing().is_some());
        Ok(())
    }

    // FR-UI-07
    #[test]
    fn the_state_tells_what_each_message_changed_and_whether_it_was_refused()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        let a = state.selection().focused();
        give(&mut state, Command::Move(Direction::Down));
        assert_eq!(
            state
                .changed()
                .iter()
                .copied()
                .map(Some)
                .collect::<Vec<_>>(),
            [a; 2]
        );
        assert!(!state.refused());
        give(&mut state, Command::Convert);
        assert!(state.refused(), "the Inbox holds no project");
        assert_eq!(state.changed(), []);
        update(&mut state, Message::Tick(Duration::from_secs(1)), TODAY);
        assert!(!state.refused(), "the next message forgets the refusal");
        assert!(
            state.notice().is_some(),
            "though the status line still says it"
        );
        Ok(())
    }

    // FR-OPS-02, FR-OPS-03
    #[test]
    fn moving_changes_the_state_with_no_effect_and_indenting_does_nothing_in_the_pane()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::Move(Direction::Down));
        assert_eq!(titles(&state), ["b", "a"]);
        assert_eq!(focused(&state), Some("a"));
        give(&mut state, Command::Move(Direction::Up));
        assert_eq!(titles(&state), ["a", "b"]);
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
        ] {
            assert_eq!(update(&mut state, message, TODAY), []);
        }
        let before = state.clone();
        for command in [Command::Indent, Command::Outdent] {
            assert_eq!(update(&mut state, Message::Command(command), TODAY), []);
        }
        assert_eq!(state, before, "only checklist items nest (D-75)");
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

    // FR-VIEW-10, FR-OPS-02, FR-EDIT-06
    #[test]
    fn the_sidebar_moves_its_focused_entry() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        for title in ["A", "B"] {
            document.push(Some(work), Node::Item(Item::new(ItemKind::Project, title)))?;
        }
        let mut state = State::new(document, Opened::Demo, TODAY);
        assert!(
            state.sidebar().is_some() && !state.sidebar_has_keys(),
            "the sidebar shows at start, the pane having the keys"
        );
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
        give(&mut state, Command::MoveTo);
        assert!(
            state.move_to().is_some(),
            "Move to opens for the focused entry"
        );
        update(&mut state, Message::MoveTo(PickerInput::Dismiss), TODAY);
        give(&mut state, Command::Focus(Focus::First));
        past_the_views(&mut state);
        let before = state.document().clone();
        for command in [
            Command::Indent,
            Command::Outdent,
            Command::AddSection(End::Bottom),
            Command::ShowCompleted,
            Command::ToggleCompleted,
        ] {
            give(&mut state, command);
        }
        assert_eq!(state.document(), &before, "other commands do nothing");
        assert!(!state.shows_completed());
        give(&mut state, Command::Go(Go::Inbox));
        assert_eq!(
            state.container(),
            Some(state.document().inbox()),
            "going to the Inbox passes"
        );
        give(&mut state, Command::EditTitle);
        assert_eq!(state.container(), Some(work), "Enter shows it in the pane");
        assert!(state.sidebar_has_keys(), "the sidebar keeps the keys");
        give(&mut state, Command::NextPane);
        assert!(
            !state.sidebar_has_keys() && state.sidebar().is_some(),
            "Tab gives the keys to the pane"
        );
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::Navigator);
        assert!(state.navigator().is_some(), "the Navigator passes");
        give(&mut state, Command::Navigator);
        give(&mut state, Command::GoToSymbol);
        assert!(state.go_to_symbol().is_some(), "so does Go to Symbol");
        Ok(())
    }

    // FR-NAV-06, FR-SET-02, FR-VIEW-10
    #[test]
    fn the_sidebar_passes_the_palette_settings_and_the_weather_on() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Palette);
        assert!(state.palette().is_some(), "the palette opens");
        update(&mut state, Message::Palette(PickerInput::Dismiss), TODAY);
        give(&mut state, Command::Settings);
        assert!(
            state.settings_window().is_some(),
            "so does the settings window"
        );
        let effects = update(&mut state, Message::Command(Command::StartWeather), TODAY);
        assert_eq!(effects, [Effect::StartWeather]);
        assert!(state.sidebar_has_keys());
        Ok(())
    }

    // FR-TAG-01, FR-TAG-02, FR-VIEW-10
    #[test]
    fn the_sidebar_tags_its_focused_entry_but_no_area() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Project, "Work")))?;
        document.push(None, Node::Item(Item::new(ItemKind::Area, "Home")))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Focus(Focus::Last));
        assert_eq!(sidebar(&state).0.as_deref(), Some("Home"));
        give(&mut state, Command::Tags);
        assert_eq!(state.tags(), None);
        assert_eq!(state.notice(), Some(&Notice::AreaUntagged));
        give(&mut state, Command::Focus(Focus::Previous));
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

    // FR-VIEW-12
    #[test]
    fn pinning_acts_on_projects_in_the_pane_and_the_sidebar() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        let site = document.push(Some(work), Node::Item(Item::new(ItemKind::Project, "Site")))?;
        document.push(Some(site), Node::Item(Item::new(ItemKind::Task, "t")))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        let pinned = |state: &State| {
            (state.document().node(site))
                .and_then(Node::item)
                .is_some_and(|item| item.pinned)
        };
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Focus(Focus::Last));
        give(&mut state, Command::TogglePinned);
        assert!(pinned(&state));
        assert_eq!(
            sidebar(&state),
            (
                Some("Site".into()),
                vec!["Site".into(), "Work".into(), "  Site".into()]
            ),
            "listed again, the focus staying on the project in its area"
        );
        give(&mut state, Command::Undo);
        assert!(!pinned(&state), "one step to undo");
        give(&mut state, Command::Redo);
        give(&mut state, Command::Focus(Focus::First));
        for _ in 0..=crate::app::view::REGISTRY.len() {
            give(&mut state, Command::TogglePinned);
            assert_eq!(state.notice(), Some(&Notice::NotAProject));
            give(&mut state, Command::Focus(Focus::Next));
        }
        assert_eq!(sidebar(&state).0.as_deref(), Some("Site"));
        give(&mut state, Command::TogglePinned);
        assert!(!pinned(&state), "unpinned from its entry among the pinned");
        assert_eq!(
            sidebar(&state).0.as_deref(),
            Some("Work"),
            "now where the entry was"
        );
        give(&mut state, Command::EditTitle);
        give(&mut state, Command::NextPane);
        give(&mut state, Command::Focus(Focus::First));
        give(&mut state, Command::TogglePinned);
        assert_eq!(state.notice(), Some(&Notice::NotAProject), "the area shown");
        give(&mut state, Command::Focus(Focus::Last));
        give(&mut state, Command::TogglePinned);
        assert!(pinned(&state), "the project's row in the pane");
        Ok(())
    }

    // FR-VIEW-10, D-67
    #[test]
    fn collapsing_in_the_pane_does_nothing() -> Result<(), PlacementError> {
        let mut state = state()?;
        let before = state.clone();
        give(&mut state, Command::Collapse);
        assert_eq!(state, before);
        Ok(())
    }

    // FR-ORG-10, FR-VIEW-10, D-68
    #[test]
    fn the_sidebar_opens_its_project_to_edit_its_description() -> Result<(), PlacementError> {
        let mut document = state()?.document().clone();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Project, "Work")))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::EditDescription(Whose::Focused));
        assert!(state.editing().is_none(), "not the Inbox's");
        give(&mut state, Command::Focus(Focus::Last));
        give(&mut state, Command::EditDescription(Whose::Focused));
        assert_eq!(
            state.editing().map(|(id, part, _)| (id, part)),
            Some((work, Part::Description))
        );
        assert_eq!(state.container(), Some(work));
        assert!(!state.sidebar_has_keys());
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
        give(&mut state, Command::Sidebar);
        past_the_views(&mut state);
        let pane = state.selection().clone();
        give(&mut state, Command::RenameContainer);
        type_into(&mut state, "s");
        edit(&mut state, EditInput::Next);
        assert!(state.editing().is_none(), "Enter only ends editing");
        assert_eq!(sidebar(&state).1, ["Works"]);
        assert_eq!(state.selection(), &pane, "the pane keeps its focus");
        give(&mut state, Command::AddProjectInArea(End::Bottom));
        assert_eq!(
            sidebar(&state),
            (Some(String::new()), vec!["Works".into(), "  ".into()]),
            "`i` adds a project in the focused area"
        );
        edit(&mut state, EditInput::Done);
        assert_eq!(
            sidebar(&state),
            (Some("Works".into()), vec!["Works".into()])
        );
        give(&mut state, Command::AddProject(End::Bottom));
        assert_eq!(
            sidebar(&state),
            (Some(String::new()), vec![String::new(), "Works".into()]),
            "`p` adds one at the top level, listed before the areas"
        );
        edit(&mut state, EditInput::Done);
        give(&mut state, Command::AddTask(End::Top));
        type_into(&mut state, "Home");
        give(&mut state, Command::Focus(Focus::Next));
        assert_eq!(
            sidebar(&state),
            (Some("Works".into()), vec!["Home".into(), "Works".into()])
        );
        assert_eq!(state.selection(), &pane);
        give(&mut state, Command::NewLike(Side::Below));
        assert_eq!(
            sidebar(&state),
            (
                Some(String::new()),
                vec!["Home".into(), "Works".into(), String::new()]
            ),
            "`o` beside an area adds an area"
        );
        edit(&mut state, EditInput::Done);
        give(&mut state, Command::EditTitle);
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
        give(&mut state, Command::FocusRight);
        assert!(!state.sidebar_has_keys() && state.sidebar().is_some());
        give(&mut state, Command::Focus(Focus::Last));
        assert_eq!(focused(&state), Some("b"), "the pane has the keys");
        give(&mut state, Command::FocusLeft);
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
            Command::EditTitle,
            Command::Tags,
            Command::Focus(Focus::Next),
        ] {
            give(&mut state, command);
        }
        assert_eq!(state.document(), &before, "the Inbox stays as it is");
        assert!(state.editing().is_none() && state.move_to().is_none());
        give(&mut state, Command::AddProject(End::Top));
        type_into(&mut state, "P");
        edit(&mut state, EditInput::Done);
        assert_eq!(sidebar(&state), (Some("P".into()), vec!["P".into()]));
        let inbox = state.document().inbox();
        give(&mut state, Command::EditTitle);
        assert_ne!(state.container(), Some(inbox));
        give(&mut state, Command::Focus(Focus::First));
        assert_eq!(
            state.container(),
            Some(inbox),
            "moving to it shows the Inbox"
        );
        give(&mut state, Command::Go(Go::Back));
        assert_ne!(state.container(), Some(inbox));
        give(&mut state, Command::EditTitle);
        assert_eq!(state.container(), Some(inbox), "and so does Enter");
        assert!(state.sidebar_has_keys(), "the sidebar keeps the keys");
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

    // FR-OPS-01, D-82
    #[test]
    fn visual_line_mode_extends_with_every_motion_until_another_command()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        for title in ["a", "b", "c", "d"] {
            document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, title)))?;
        }
        let mut state = State::new(document, Opened::Demo, TODAY);
        let picked = |state: &State| {
            let rows = state.rows();
            let titles = rows
                .iter()
                .filter(|row| state.selection().is_picked(row.id()));
            titles
                .filter_map(|row| Some(state.document().node(row.id())?.item()?.title.clone()))
                .collect::<Vec<_>>()
        };
        give(&mut state, Command::Focus(Focus::Next));
        give(&mut state, Command::VisualLines);
        assert!(state.selection().visual());
        assert_eq!(picked(&state), ["b"]);
        give(&mut state, Command::Focus(Focus::Last));
        assert_eq!(picked(&state), ["b", "c", "d"]);
        give(&mut state, Command::Shortcuts);
        give(&mut state, Command::Focus(Focus::First));
        assert_eq!(
            picked(&state),
            ["a", "b"],
            "the title row is no row to pick"
        );
        give(&mut state, Command::ToggleCompleted);
        assert!(!state.selection().visual(), "a command ends it");
        assert_eq!(titles(&state), ["c", "d"], "acting on the picks");
        give(&mut state, Command::Undo);
        give(&mut state, Command::Focus(Focus::Last));
        assert_eq!(picked(&state), ["a", "b"], "motions only move again");
        give(&mut state, Command::VisualLines);
        give(&mut state, Command::VisualLines);
        give(&mut state, Command::Focus(Focus::Previous));
        assert_eq!(
            picked(&state),
            ["a", "b", "d"],
            "V again ends it, keeping the picks"
        );
        give(&mut state, Command::VisualLines);
        give(&mut state, Command::ClearSelection);
        assert!(!state.selection().visual());
        assert_eq!(picked(&state), Vec::<String>::new(), "Esc picks nothing");
        give(&mut state, Command::VisualLines);
        let branch = crate::domain::Branch {
            node: Node::Item(Item::new(ItemKind::Task, "pasted")),
            children: Vec::new(),
        };
        update(&mut state, Message::Paste(Side::Below, vec![branch]), TODAY);
        assert!(!state.selection().visual(), "a terminal paste ends it too");
        give(&mut state, Command::Undo);
        assert!(
            !state.selection().visual(),
            "and undoing it leaves it ended"
        );
        Ok(())
    }

    // FR-DATE-06, FR-OPS-06
    #[test]
    fn nudging_moves_the_selections_dates_a_day_as_one_step() -> Result<(), PlacementError> {
        let mut state = state()?;
        let (yesterday, tomorrow) = (date(2026, 10, 13), date(2026, 10, 15));
        give(&mut state, Command::DayLater(DateKind::Due));
        assert_eq!(dates(&state, "a"), (None, Some(tomorrow)));
        assert_eq!(dates(&state, "b"), (None, None));
        give(&mut state, Command::SelectAll);
        give(&mut state, Command::DayEarlier(DateKind::Due));
        assert_eq!(dates(&state, "a"), (None, Some(TODAY)));
        assert_eq!(dates(&state, "b"), (None, Some(yesterday)));
        give(&mut state, Command::DayEarlier(DateKind::Start));
        assert_eq!(dates(&state, "a"), (Some(yesterday), Some(TODAY)));
        assert_eq!(dates(&state, "b"), (Some(yesterday), Some(yesterday)));
        give(&mut state, Command::Undo);
        assert_eq!(dates(&state, "a"), (None, Some(TODAY)));
        assert_eq!(dates(&state, "b"), (None, Some(yesterday)));
        Ok(())
    }

    // FR-ORG-11, FR-VIEW-10
    #[test]
    fn toggle_later_marks_the_focused_project_or_task_but_no_area() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Project, "Work")))?;
        let task = document.push(Some(work), Node::Item(Item::new(ItemKind::Task, "t")))?;
        let home = document.push(None, Node::Item(Item::new(ItemKind::Area, "Home")))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        let later = |state: &State, id| {
            let item = state.document().node(id).and_then(Node::item);
            item.is_some_and(|item| item.later)
        };
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Focus(Focus::Last));
        give(&mut state, Command::ToggleLater);
        assert_eq!(state.notice(), Some(&Notice::NotLater));
        assert!(!later(&state, home));
        give(&mut state, Command::Focus(Focus::Previous));
        give(&mut state, Command::ToggleLater);
        assert!(later(&state, work), "the sidebar marks its focused project");
        give(&mut state, Command::EditTitle);
        give(&mut state, Command::NextPane);
        give(&mut state, Command::Focus(Focus::Last));
        give(&mut state, Command::ToggleLater);
        assert!(later(&state, task), "the pane marks its focused task");
        Ok(())
    }

    // FR-DATE-01, FR-DATE-05, FR-DATE-06, FR-VIEW-10
    #[test]
    fn the_sidebar_dates_its_focused_entry_but_no_area() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Project, "Work")))?;
        let home = document.push(None, Node::Item(Item::new(ItemKind::Area, "Home")))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        let dated = |state: &State, id| {
            let item = state.document().node(id).and_then(Node::item);
            item.map(|item| (item.start, item.due))
        };
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Focus(Focus::Last));
        for command in [
            Command::StartToday,
            Command::SetDate(DateKind::Due),
            Command::DayLater(DateKind::Start),
            Command::DayEarlier(DateKind::Due),
        ] {
            give(&mut state, command);
            assert_eq!(state.notice(), Some(&Notice::AreaUndated), "{command:?}");
        }
        assert_eq!(state.date_prompt(), None);
        assert_eq!(dated(&state, home), Some((None, None)));
        give(&mut state, Command::Focus(Focus::Previous));
        give(&mut state, Command::StartToday);
        assert_eq!(dated(&state, work), Some((Some(TODAY), None)));
        give(&mut state, Command::SetDate(DateKind::Due));
        type_date(&mut state, "tomorrow");
        prompt(&mut state, DateInput::Apply);
        assert_eq!(
            dated(&state, work),
            Some((Some(TODAY), Some(date(2026, 10, 15))))
        );
        give(&mut state, Command::DayLater(DateKind::Start));
        give(&mut state, Command::DayEarlier(DateKind::Due));
        assert_eq!(
            dated(&state, work),
            Some((Some(date(2026, 10, 15)), Some(TODAY)))
        );
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
        give(&mut state, Command::EditTitle);
        assert_eq!(state.shown(), Shown::View(View::Today));
        assert!(state.sidebar_has_keys(), "the keys stay in the sidebar");
        give(&mut state, Command::Go(Go::Inbox));
        give(&mut state, Command::EditTitle);
        assert_eq!(state.shown(), Shown::View(View::Today));
        assert_eq!(focused(&state), Some("due"));
        assert!(
            state.sidebar_has_keys(),
            "Enter leaves the keys in the sidebar"
        );
        give(&mut state, Command::NewLike(Side::Below));
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
        let effects = update(&mut state, Message::Command(Command::Go(Go::Inbox)), TODAY);
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
            Command::EditTitle,
            Command::FocusRight,
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
        assert_eq!(
            titles(&state),
            ["a", "b", "c"],
            "the heading does not convert"
        );
        give(&mut state, Command::Focus(Focus::Next));
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
        give(&mut state, Command::FocusLeft);
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

    // FR-PANE-02, FR-VIEW-10
    #[test]
    fn panes_open_from_the_sidebar_and_take_the_keys_beside_it() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        let mut state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Command::Sidebar);
        give(&mut state, Command::Focus(Focus::Last));
        give(&mut state, Command::OpenInNewPane);
        assert_eq!((state.current_pane(), state.container()), (1, Some(work)));
        assert!(!state.sidebar_has_keys(), "the new pane has the keys");
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::FocusLeft);
        assert!(
            state.sidebar_has_keys(),
            "the sidebar is left of the first pane"
        );
        give(&mut state, Command::FocusLeft);
        assert!(state.sidebar_has_keys());
        give(&mut state, Command::FocusPane(2));
        assert_eq!((state.current_pane(), state.sidebar_has_keys()), (1, false));
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::FocusLeft);
        give(&mut state, Command::FocusRight);
        assert_eq!((state.current_pane(), state.sidebar_has_keys()), (0, false));
        let before = state.clone();
        assert_eq!(
            update(&mut state, Message::Command(Command::Narrow), TODAY),
            [Effect::NarrowPane]
        );
        assert_eq!(
            update(&mut state, Message::Command(Command::Widen), TODAY),
            [Effect::WidenPane]
        );
        assert_eq!(state, before, "the frontend answers with a width");
        assert_eq!(update(&mut state, Message::PaneWidth(42), TODAY), []);
        assert_eq!(state.pane_view().width(), Some(42));
        give(&mut state, Command::ClosePane);
        assert_eq!((state.panes().count(), state.container()), (1, Some(work)));
        Ok(())
    }
}
