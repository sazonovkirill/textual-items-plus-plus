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
#[expect(
    unused_variables,
    reason = "no command changes the state until the container view arrives"
)]
pub fn update(state: &mut State, message: Message) -> Vec<Effect> {
    match message {
        Message::Command(Command::Quit) => vec![Effect::Quit],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quitting_asks_the_frontend_to_end_the_session_and_leaves_the_state_alone() {
        let mut state = State::default();
        let effects = update(&mut state, Message::Command(Command::Quit));
        assert_eq!(effects, [Effect::Quit]);
        assert_eq!(state, State::default());
    }
}
