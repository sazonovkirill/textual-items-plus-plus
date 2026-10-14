//! The shortcuts panel that `?` shows: the commands the app takes now, grouped, as
//! taskpaper-terminal lists them ([ui.md §4](../../../specs/reference/taskpaper-terminal/specs/ui.md#4-shortcuts-panel-fr-key-04),
//! FR-KEY-03). Every command belongs to a group, so a command a later step registers is listed
//! with the rest.

use super::{Command, Context, Entry, REGISTRY, State};

/// A heading the shortcuts panel lists commands under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyGroup {
    /// Moving the focus.
    Moving,
    /// Selecting rows.
    Selection,
    /// Going to other places and their history.
    Going,
    /// Editing titles and descriptions and adding items.
    Creating,
    /// Completing items, their dates and their tags.
    Items,
    /// Moving, converting, grouping, deleting and copying items, and undo.
    Structure,
    /// What the pane shows, the sidebar, the panel itself and quitting.
    View,
    /// The conflict prompt's own commands.
    Conflict,
}

impl KeyGroup {
    /// Every group, in the order the panel lists them.
    const ALL: [Self; 8] = [
        Self::Moving,
        Self::Selection,
        Self::Going,
        Self::Creating,
        Self::Items,
        Self::Structure,
        Self::View,
        Self::Conflict,
    ];

    /// The group's name, which heads its commands in the panel.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Moving => "Moving",
            Self::Selection => "Selection",
            Self::Going => "Going",
            Self::Creating => "Editing and Creating",
            Self::Items => "Completing, Dates and Tags",
            Self::Structure => "Structure",
            Self::View => "View and Panels",
            Self::Conflict => "Conflict",
        }
    }

    /// The group `command` is listed in.
    fn of(command: Command) -> Self {
        match command {
            Command::Focus(_) => Self::Moving,
            Command::Extend(_)
            | Command::TogglePicked
            | Command::SelectAll
            | Command::ClearSelection => Self::Selection,
            Command::Go(_) | Command::Navigator => Self::Going,
            Command::EditTitle
            | Command::NewLike(_)
            | Command::AddTask(_)
            | Command::AddSection(_)
            | Command::AddProject(_)
            | Command::EditDescription(_)
            | Command::OpenNote(_)
            | Command::OpenComments(_)
            | Command::RenameContainer => Self::Creating,
            Command::ToggleCompleted
            | Command::SetDate(_)
            | Command::StartToday
            | Command::Tags => Self::Items,
            Command::Undo
            | Command::Redo
            | Command::Delete
            | Command::Restore
            | Command::EmptyTrash
            | Command::DeleteContainer
            | Command::MoveTo
            | Command::Move(_)
            | Command::Indent
            | Command::Outdent
            | Command::Convert
            | Command::GroupIntoSection
            | Command::GroupIntoProject
            | Command::Duplicate
            | Command::Copy
            | Command::Cut
            | Command::Paste(_) => Self::Structure,
            Command::Quit
            | Command::ShowCompleted
            | Command::FilterTags
            | Command::ClearTagFilter
            | Command::Sidebar
            | Command::FocusLeft
            | Command::FocusRight
            | Command::FocusPane(_)
            | Command::OpenInNewPane
            | Command::ClosePane
            | Command::Narrow
            | Command::Widen
            | Command::Shortcuts
            | Command::ScrollShortcuts(_) => Self::View,
            Command::KeepMine | Command::TakeTheirs | Command::ShowDiff => Self::Conflict,
        }
    }
}

impl State {
    /// Whether the app takes `command` now: while the conflict prompt is open only its own
    /// commands and quitting, while the attachments panel is open only quitting, while a picker or the date prompt is open or in edit mode only
    /// global ones, and otherwise any.
    #[must_use]
    pub fn takes(&self, command: Command) -> bool {
        let context = REGISTRY
            .iter()
            .find(|entry| entry.command == command)
            .map(|entry| entry.context);
        if self.conflict().is_some() {
            command == Command::Quit || context == Some(Context::Conflict)
        } else if self.attachments().is_some() {
            command == Command::Quit
        } else if self.editing().is_some()
            || self.navigator.is_some()
            || self.move_to.is_some()
            || self.tags.is_some()
            || self.date_prompt.is_some()
        {
            context == Some(Context::Global)
        } else {
            context != Some(Context::Conflict)
        }
    }

    /// Whether the shortcuts panel shows.
    #[must_use]
    pub fn shows_shortcuts(&self) -> bool {
        self.shortcuts
    }

    /// While the shortcuts panel shows, the commands the app takes now, each group with its
    /// registry entries in the registry's order, leaving out groups with none (FR-KEY-03).
    #[must_use]
    pub fn shortcuts(&self) -> Option<Vec<(KeyGroup, Vec<&'static Entry>)>> {
        self.shortcuts.then(|| {
            KeyGroup::ALL
                .into_iter()
                .map(|group| {
                    let entries = REGISTRY
                        .iter()
                        .filter(|entry| {
                            KeyGroup::of(entry.command) == group && self.takes(entry.command)
                        })
                        .collect::<Vec<_>>();
                    (group, entries)
                })
                .filter(|(_, entries)| !entries.is_empty())
                .collect()
        })
    }

    /// Shows the shortcuts panel, or hides it.
    pub(super) fn toggle_shortcuts(&mut self) {
        self.shortcuts = !self.shortcuts;
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::{
        app::{Message, Opened, update},
        domain::{Document, Item, ItemKind, Node, PlacementError, dates::DateKind},
    };

    const TODAY: Date = date(2026, 10, 14);

    fn state() -> Result<State, PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        document.push(Some(inbox), Node::Item(Item::new(ItemKind::Task, "a")))?;
        Ok(State::new(document, Opened::Demo, TODAY))
    }

    fn give(state: &mut State, command: Command) {
        update(state, Message::Command(command), TODAY);
    }

    /// The names of the groups the panel lists and how many commands each holds.
    fn listed(state: &State) -> Option<Vec<(&'static str, usize)>> {
        state.shortcuts().map(|groups| {
            groups
                .iter()
                .map(|(group, entries)| (group.name(), entries.len()))
                .collect()
        })
    }

    // FR-KEY-03
    #[test]
    fn the_panel_shows_and_hides_and_groups_every_command_of_navigate_mode()
    -> Result<(), PlacementError> {
        let mut state = state()?;
        assert_eq!(state.shortcuts(), None, "hidden at the start");
        give(&mut state, Command::Shortcuts);
        let groups = state.shortcuts().unwrap_or_default();
        let names: Vec<_> = groups.iter().map(|(group, _)| group.name()).collect();
        assert_eq!(
            names,
            [
                "Moving",
                "Selection",
                "Going",
                "Editing and Creating",
                "Completing, Dates and Tags",
                "Structure",
                "View and Panels",
            ]
        );
        let listed: Vec<_> = groups.iter().flat_map(|(_, entries)| entries).collect();
        let navigate = REGISTRY
            .iter()
            .filter(|entry| entry.context != Context::Conflict)
            .count();
        assert_eq!(listed.len(), navigate, "every command but the conflict's");
        assert_eq!(
            groups
                .first()
                .and_then(|(_, entries)| entries.first().copied()),
            REGISTRY.get(1),
            "in the registry's order"
        );
        give(&mut state, Command::Shortcuts);
        assert_eq!(state.shortcuts(), None);
        Ok(())
    }

    // FR-KEY-03
    #[test]
    fn a_window_that_takes_the_keys_leaves_only_the_global_commands() -> Result<(), PlacementError>
    {
        let mut state = state()?;
        give(&mut state, Command::Shortcuts);
        give(&mut state, Command::SetDate(DateKind::Due));
        assert_eq!(
            listed(&state),
            Some(vec![("Going", 1), ("View and Panels", 1)]),
            "the Navigator and quitting"
        );
        assert!(!state.takes(Command::Shortcuts));
        assert!(state.takes(Command::Navigator));
        Ok(())
    }

    // FR-KEY-03
    #[test]
    fn every_group_names_itself() {
        let names: Vec<_> = KeyGroup::ALL.into_iter().map(KeyGroup::name).collect();
        assert_eq!(names.last(), Some(&"Conflict"));
        assert!(names.iter().all(|name| !name.is_empty()));
    }

    // FR-KEY-03
    #[test]
    fn scrolling_asks_the_frontend_to_scroll_the_panel() -> Result<(), PlacementError> {
        let mut state = state()?;
        let effects = update(
            &mut state,
            Message::Command(Command::ScrollShortcuts(super::super::Direction::Up)),
            TODAY,
        );
        assert_eq!(
            effects,
            [crate::app::Effect::ScrollShortcuts(
                super::super::Direction::Up
            )]
        );
        Ok(())
    }
}
