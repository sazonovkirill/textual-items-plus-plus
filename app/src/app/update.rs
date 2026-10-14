//! The update function: the one place messages change the state, dispatching each command to the
//! module that owns it.

use jiff::civil::Date;

use super::{Command, PickerInput, State};

/// A user intent or a result the frontend reports back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// The user gave a command.
    Command(Command),
    /// The user did something in the open Navigator (FR-VIEW-07).
    Navigator(PickerInput),
}

/// Work the frontend carries out for the core, which never does I/O itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// End the session, after the effects before it.
    Quit,
}

/// Applies `message` to `state` on `today`, the local date by the frontend's clock, and returns
/// the effects the frontend should carry out, in order.
pub fn update(state: &mut State, message: Message, today: Date) -> Vec<Effect> {
    match message {
        Message::Command(Command::Quit) => return vec![Effect::Quit],
        Message::Command(Command::Focus(motion)) => state.move_focus(motion),
        Message::Command(Command::ToggleCompleted) => state.toggle_completed(today),
        Message::Command(Command::ShowCompleted) => state.toggle_show_completed(),
        Message::Command(Command::Undo) => state.undo(),
        Message::Command(Command::Redo) => state.redo(),
        Message::Command(Command::Go(to)) => state.go(to),
        Message::Command(Command::Navigator) => state.toggle_navigator(),
        Message::Navigator(input) => state.navigator_input(input),
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::{Focus, Go, Opened};
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
    fn quitting_asks_the_frontend_to_end_the_session_and_leaves_the_state_alone()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        let before = state.clone();
        let effects = update(&mut state, Message::Command(Command::Quit), TODAY);
        assert_eq!(effects, [Effect::Quit]);
        assert_eq!(state, before);
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
