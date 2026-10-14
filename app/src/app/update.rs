//! The update function: the one place messages change the state, dispatching each command to the
//! module that owns it.

use super::{Command, State};

/// A user intent or a result the frontend reports back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// The user gave a command.
    Command(Command),
}

/// Work the frontend carries out for the core, which never does I/O itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// End the session, after the effects before it.
    Quit,
}

/// Applies `message` to `state` and returns the effects the frontend should carry out, in order.
pub fn update(state: &mut State, message: Message) -> Vec<Effect> {
    match message {
        Message::Command(Command::Quit) => vec![Effect::Quit],
        Message::Command(Command::Focus(motion)) => {
            state.move_focus(motion);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Focus, Opened};
    use crate::domain::{Document, Item, ItemKind, Node, PlacementError};

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
        let effects = update(&mut state, Message::Command(Command::Quit));
        assert_eq!(effects, [Effect::Quit]);
        assert_eq!(state, before);
        Ok(())
    }

    // FR-NAV-01
    #[test]
    fn a_motion_moves_the_focus_with_no_effect() -> Result<(), PlacementError> {
        let mut state = state()?;
        let last = state.rows().last().map(|row| row.id());
        let effects = update(&mut state, Message::Command(Command::Focus(Focus::Last)));
        assert_eq!(effects, []);
        assert_eq!(state.selection().focused(), last);
        Ok(())
    }
}
