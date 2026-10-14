//! The command registry: every command a user can give, with its title and the context it is
//! given in. A frontend binds its own keys to these commands and lists them in its palette and
//! shortcuts panel.

use super::Focus;

/// A command: one user intent, whatever key, menu or palette gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Leave the app.
    Quit,
    /// Move the focus (FR-NAV-01).
    Focus(Focus),
    /// Complete the selected items, or uncomplete them when all are completed (FR-TASK-02,
    /// FR-ORG-09).
    ToggleCompleted,
    /// Show completed items, or hide them again (FR-VIEW-11).
    ShowCompleted,
    /// Take back the latest change (FR-EDIT-06).
    Undo,
    /// Make the latest change taken back again (FR-EDIT-06).
    Redo,
}

/// Where a command can be given. Later steps add the modes and windows that take keys of their
/// own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// Everywhere in the app.
    Global,
    /// In navigate mode, moving between items (FR-EDIT-01).
    Navigate,
}

/// A command with what the palette and the shortcuts panel show of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The command.
    pub command: Command,
    /// Its name as a user reads it.
    pub title: &'static str,
    /// Where it can be given.
    pub context: Context,
}

/// Every command, one entry each; a feature registers its commands here.
pub const REGISTRY: &[Entry] = &[
    Entry {
        command: Command::Quit,
        title: "Quit",
        context: Context::Global,
    },
    Entry {
        command: Command::Focus(Focus::Next),
        title: "Next Item",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Focus(Focus::Previous),
        title: "Previous Item",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Focus(Focus::First),
        title: "First Item",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Focus(Focus::Last),
        title: "Last Item",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Focus(Focus::Parent),
        title: "Section Heading",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Focus(Focus::NextSection),
        title: "Next Section",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Focus(Focus::PreviousSection),
        title: "Previous Section",
        context: Context::Navigate,
    },
    Entry {
        command: Command::ToggleCompleted,
        title: "Complete",
        context: Context::Navigate,
    },
    Entry {
        command: Command::ShowCompleted,
        title: "Show Completed Items",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Undo,
        title: "Undo",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Redo,
        title: "Redo",
        context: Context::Navigate,
    },
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_command_is_registered_once_with_a_distinct_title() {
        let commands: HashSet<_> = REGISTRY.iter().map(|entry| entry.command).collect();
        let titles: HashSet<_> = REGISTRY.iter().map(|entry| entry.title).collect();
        assert_eq!(commands.len(), REGISTRY.len());
        assert_eq!(titles.len(), REGISTRY.len());
        assert!(REGISTRY.iter().all(|entry| !entry.title.is_empty()));
    }

    // FR-NAV-01
    #[test]
    fn every_motion_is_a_navigate_command() {
        let motions = [
            Focus::Next,
            Focus::Previous,
            Focus::First,
            Focus::Last,
            Focus::Parent,
            Focus::NextSection,
            Focus::PreviousSection,
        ];
        for motion in motions {
            let entry = REGISTRY
                .iter()
                .find(|entry| entry.command == Command::Focus(motion));
            assert_eq!(
                entry.map(|entry| entry.context),
                Some(Context::Navigate),
                "{motion:?}"
            );
        }
    }
}
