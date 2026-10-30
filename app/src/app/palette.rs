//! The command palette: every command the app takes, by its title, filtered as the user types,
//! running the one chosen, as VS Code's does (FR-NAV-06, D-4).

use super::{Candidate, Command, Picked, Picker, PickerInput, PickerView, REGISTRY, State};

impl State {
    /// The command palette, while it is open.
    #[must_use]
    pub fn palette(&self) -> Option<PickerView<'_, Command>> {
        self.palette.as_ref().map(Picker::view)
    }

    /// Opens the command palette on the commands the app takes now, but itself, in the
    /// registry's order. No other picker is open then, since none takes this command.
    pub(super) fn open_palette(&mut self) {
        let candidates = REGISTRY
            .iter()
            .filter(|entry| entry.command != Command::Palette && self.takes(entry.command))
            .map(|entry| Candidate {
                value: entry.command,
                text: entry.title.to_owned(),
                detail: String::new(),
            })
            .collect();
        self.palette = Some(Picker::new(candidates));
    }

    /// Applies `input` to the open command palette, and returns the command chosen, which closes
    /// it, for the caller to give. Nothing happens while it is closed, nor for choosing in a new
    /// pane, which no command opens in.
    pub(super) fn palette_input(&mut self, input: PickerInput) -> Option<Command> {
        if input == PickerInput::ChooseInNewPane {
            return None;
        }
        let picked = self.palette.as_mut()?.input(input)?;
        self.palette = None;
        match picked {
            Picked::Chosen(command) => Some(command),
            Picked::Dismissed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::{
        app::{Context, Direction, Effect, Message, Opened, TextEdit, update},
        domain::{Document, Item, ItemKind, Node, PlacementError},
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

    fn give(state: &mut State, message: Message) -> Vec<Effect> {
        update(state, message, TODAY)
    }

    fn typed(state: &mut State, query: &str) {
        for c in query.chars() {
            give(
                state,
                Message::Palette(PickerInput::Edit(TextEdit::Insert(c))),
            );
        }
    }

    fn listed(state: &State) -> Vec<Command> {
        state
            .palette()
            .map(|view| view.entries.iter().map(|entry| entry.value).collect())
            .unwrap_or_default()
    }

    fn titles(state: &State) -> Vec<&str> {
        state
            .rows()
            .iter()
            .filter_map(|row| Some(state.document().node(row.id())?.item()?.title.as_str()))
            .collect()
    }

    // FR-NAV-06
    #[test]
    fn the_palette_lists_every_command_of_navigate_mode_but_itself_in_the_registrys_order()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        assert_eq!(state.palette(), None);
        give(&mut state, Message::Command(Command::Palette));
        let expected: Vec<_> = REGISTRY
            .iter()
            .filter(|entry| entry.context != Context::Conflict)
            .map(|entry| entry.command)
            .filter(|&command| {
                !matches!(
                    command,
                    Command::Palette
                        | Command::ToggleEvening
                        | Command::AddArea(_)
                        | Command::Restore
                )
            })
            .collect();
        assert_eq!(
            listed(&state),
            expected,
            "Toggle Evening only in Today, the area keys only in the sidebar, Restore only in the \
             Trash"
        );
        let first = state
            .palette()
            .and_then(|view| view.entries.iter().next().map(|entry| entry.text.clone()));
        assert_eq!(first.as_deref(), Some("Quit"), "by title");
        Ok(())
    }

    // FR-NAV-06
    #[test]
    fn typing_filters_the_commands_and_choosing_one_closes_the_palette_and_gives_it()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Message::Command(Command::Palette));
        typed(&mut state, "move down");
        assert_eq!(
            listed(&state).first(),
            Some(&Command::Move(Direction::Down))
        );
        give(&mut state, Message::Palette(PickerInput::Choose));
        assert_eq!(state.palette(), None);
        assert_eq!(titles(&state), ["b", "a"]);
        Ok(())
    }

    // FR-NAV-06
    #[test]
    fn a_command_chosen_returns_its_effects() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Message::Command(Command::Palette));
        typed(&mut state, "narrow pane");
        assert_eq!(
            give(&mut state, Message::Palette(PickerInput::Choose)),
            [Effect::NarrowPane]
        );
        Ok(())
    }

    // FR-NAV-06
    #[test]
    fn dismissing_or_choosing_nothing_gives_no_command() -> Result<(), PlacementError> {
        let mut state = state()?;
        let before = state.clone();
        give(&mut state, Message::Palette(PickerInput::Choose));
        assert_eq!(state, before, "nothing happens while it is closed");
        give(&mut state, Message::Command(Command::Palette));
        give(&mut state, Message::Palette(PickerInput::ChooseInNewPane));
        assert!(state.palette().is_some(), "no command opens in a new pane");
        typed(&mut state, "zzzz");
        give(&mut state, Message::Palette(PickerInput::Choose));
        assert!(
            state.palette().is_some(),
            "nothing matches, so nothing is chosen"
        );
        give(&mut state, Message::Palette(PickerInput::Dismiss));
        assert_eq!(state.palette(), None);
        assert_eq!(titles(&state), ["a", "b"]);
        Ok(())
    }

    // FR-NAV-06, FR-VIEW-10
    #[test]
    fn from_the_sidebar_the_palette_lists_what_it_takes_and_gives_it_the_choice()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        let mut document = state.document().clone();
        let work = document.push(None, Node::Item(Item::new(ItemKind::Project, "Work")))?;
        state = State::new(document, Opened::Demo, TODAY);
        give(&mut state, Message::Command(Command::Sidebar));
        give(
            &mut state,
            Message::Command(Command::Focus(crate::app::Focus::Last)),
        );
        give(&mut state, Message::Command(Command::Palette));
        let listed = listed(&state);
        for command in [
            Command::RenameContainer,
            Command::ToggleCompleted,
            Command::Navigator,
            Command::Settings,
        ] {
            assert!(listed.contains(&command), "{command:?} is listed");
        }
        for command in [Command::Narrow, Command::ClosePane] {
            assert!(!listed.contains(&command), "{command:?} is not");
        }
        typed(&mut state, "rename container");
        give(&mut state, Message::Palette(PickerInput::Choose));
        assert_eq!(state.editing().map(|(id, ..)| id), Some(work));
        assert!(state.sidebar_has_keys(), "the sidebar renames its entry");
        Ok(())
    }

    // FR-NAV-06, FR-VIEW-07
    #[test]
    fn the_palette_and_the_navigator_take_each_others_place() -> Result<(), PlacementError> {
        let mut state = state()?;
        give(&mut state, Message::Command(Command::Palette));
        assert!(state.takes(Command::Navigator));
        assert!(!state.takes(Command::Palette));
        give(&mut state, Message::Command(Command::Navigator));
        assert!(state.palette().is_none() && state.navigator().is_some());
        give(&mut state, Message::Command(Command::Navigator));
        give(&mut state, Message::Command(Command::Palette));
        typed(&mut state, "open navigator");
        give(&mut state, Message::Palette(PickerInput::Choose));
        assert!(state.palette().is_none() && state.navigator().is_some());
        Ok(())
    }
}
