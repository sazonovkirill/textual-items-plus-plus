//! The update function: the one place messages change the state, dispatching each command to the
//! module that owns it.

use std::time::Duration;

use jiff::civil::Date;

use super::{
    Command, ConflictSide, EditInput, OnDisk, PickerInput, Report, State, autosave::Ending,
};
use crate::domain::ItemKind;

/// A user intent or a result the frontend reports back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// The user gave a command.
    Command(Command),
    /// The user did something in the open Navigator (FR-VIEW-07).
    Navigator(PickerInput),
    /// The user did something in the open Move to picker (FR-OPS-04).
    MoveTo(PickerInput),
    /// The user did something while editing a title or a description (FR-EDIT-01).
    Edit(EditInput),
    /// The frontend's clock reads this, counted from any fixed moment, such as the start.
    Tick(Duration),
    /// The process was told to end, as by `SIGTERM` or `SIGHUP`: save first, then end anyway
    /// (F-FILE-05).
    Terminate,
    /// How disk work the core asked for went.
    Disk(Report),
    /// The file on disk is no longer the version the core last read or wrote (F-FILE-06).
    OnDisk(OnDisk),
}

/// Work the frontend carries out for the core, which never does I/O itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// Applies `message` to `state` on `today`, the local date by the frontend's clock, and returns
/// the effects the frontend should carry out, in order. A command given in edit mode ends it
/// first, so that the edit is kept before anything else happens. Every message is followed by
/// what saving asks for after it.
pub fn update(state: &mut State, message: Message, today: Date) -> Vec<Effect> {
    let mut effects = dispatch(state, message, today);
    effects.extend(state.step_saving());
    effects
}

/// Applies `message` to `state` by the module that owns it.
fn dispatch(state: &mut State, message: Message, today: Date) -> Vec<Effect> {
    if let Message::Command(_) = message {
        state.finish_editing();
        state.clear_notice();
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
        Message::Command(Command::Delete) => state.delete(today),
        Message::Command(Command::MoveTo) => state.open_move_to(),
        Message::Command(Command::RenameContainer) => state.rename_container(),
        Message::Command(Command::DeleteContainer) => state.delete_container(today),
        Message::Command(Command::Move(direction)) => state.move_item(direction),
        Message::Command(Command::Indent) => state.indent(),
        Message::Command(Command::Outdent) => state.outdent(),
        Message::Command(Command::EditDescription(whose)) => state.edit_description(whose),
        Message::Navigator(input) => state.navigator_input(input),
        Message::MoveTo(input) => state.move_to_input(input),
        Message::Edit(input) => state.edit_input(input),
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::{Direction, Focus, Go, Notice, Opened, Refusal, Side, TextEdit};
    use crate::domain::{Completion, Document, Item, ItemKind, Node, PlacementError};

    const TODAY: Date = date(2026, 10, 14);

    fn state() -> Result<State, PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        for title in ["a", "b"] {
            document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, title)))?;
        }
        Ok(State::new(document, Opened::Demo))
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
        let mut empty = State::new(Document::new(), Opened::Demo);
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
        let mut state = State::new(document, Opened::Demo);
        for message in [
            Message::Command(Command::Navigator),
            Message::Navigator(PickerInput::Next),
            Message::Navigator(PickerInput::Choose),
            Message::Command(Command::Focus(Focus::Last)),
            Message::Command(Command::Indent),
        ] {
            assert_eq!(update(&mut state, message, TODAY), []);
        }
        assert_eq!(titles(&state), ["A"]);
        assert_eq!(focused(&state), Some("A"), "the project it went into");
        give(&mut state, Command::Go(Go::Into));
        assert_eq!(focused(&state), Some("B"));
        give(&mut state, Command::Outdent);
        assert_eq!(titles(&state), Vec::<&str>::new());
        give(&mut state, Command::Go(Go::Up));
        assert_eq!(titles(&state), ["A", "B"]);
        Ok(())
    }

    // FR-NAV-02, FR-VIEW-07
    #[test]
    fn going_and_the_navigator_change_the_state_with_no_effect() -> Result<(), PlacementError> {
        let mut state = state()?;
        let mut document = state.document().clone();
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "Work")))?;
        state = State::new(document, Opened::Demo);
        for message in [
            Message::Command(Command::Navigator),
            Message::Navigator(PickerInput::Next),
            Message::Navigator(PickerInput::Choose),
        ] {
            assert_eq!(update(&mut state, message, TODAY), []);
        }
        assert_eq!(state.container(), area);
        let effects = update(&mut state, Message::Command(Command::Go(Go::Home)), TODAY);
        assert_eq!(effects, []);
        assert_eq!(state.container(), state.document().inbox());
        Ok(())
    }
}
