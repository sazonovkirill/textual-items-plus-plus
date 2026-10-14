//! The command registry: every command a user can give, with its title and the context it is
//! given in. A frontend binds its own keys to these commands and lists them in its palette and
//! shortcuts panel.

use super::{Focus, Side};

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
    /// Show another place in the pane (FR-NAV-02, FR-NAV-04).
    Go(Go),
    /// Open the Navigator, or close it when it is open (FR-VIEW-07).
    Navigator,
    /// Edit the focused task's title in place (FR-EDIT-01, FR-EDIT-02).
    EditTitle,
    /// Add a task beside the focused row and edit its title (FR-EDIT-05).
    NewTask(Side),
    /// Move the selected items to the Trash (FR-OPS-05, OQ-19).
    Delete,
    /// Open the picker that moves the selected items to another container (FR-OPS-04).
    MoveTo,
    /// In the conflict prompt, overwrite the file on disk with the document (F-FILE-06).
    KeepMine,
    /// In the conflict prompt, reload the file on disk, discarding the unsaved changes
    /// (F-FILE-06).
    TakeTheirs,
    /// In the conflict prompt, show the diff from the file on disk to the document, or hide it
    /// (F-FILE-06).
    ShowDiff,
}

/// Where a [`Command::Go`] takes the pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Go {
    /// Into the focused section or project, which then shows its contents (FR-NAV-02).
    Into,
    /// Up to the container holding the one shown, focused on it (FR-NAV-02).
    Up,
    /// Home, to the Inbox (FR-NAV-02).
    Home,
    /// Back to where the pane was before (FR-NAV-04).
    Back,
    /// Forward again to where Back left (FR-NAV-04).
    Forward,
}

/// Where a command can be given. Later steps add the modes and windows that take keys of their
/// own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// Everywhere in the app.
    Global,
    /// In navigate mode, moving between items (FR-EDIT-01).
    Navigate,
    /// In the conflict prompt, which takes only its own keys and quitting (F-FILE-06).
    Conflict,
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
    Entry {
        command: Command::Go(Go::Into),
        title: "Open Focused Item",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::Up),
        title: "Go Up",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::Home),
        title: "Go Home",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::Back),
        title: "Go Back",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::Forward),
        title: "Go Forward",
        context: Context::Navigate,
    },
    Entry {
        command: Command::EditTitle,
        title: "Edit Title",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewTask(Side::Below),
        title: "New Task Below",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewTask(Side::Above),
        title: "New Task Above",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Delete,
        title: "Delete",
        context: Context::Navigate,
    },
    Entry {
        command: Command::MoveTo,
        title: "Move To",
        context: Context::Navigate,
    },
    // Global, so that its key closes the Navigator too.
    Entry {
        command: Command::Navigator,
        title: "Open Navigator",
        context: Context::Global,
    },
    Entry {
        command: Command::KeepMine,
        title: "Keep Mine",
        context: Context::Conflict,
    },
    Entry {
        command: Command::TakeTheirs,
        title: "Take Theirs",
        context: Context::Conflict,
    },
    Entry {
        command: Command::ShowDiff,
        title: "Show Diff",
        context: Context::Conflict,
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

    // FR-NAV-02, FR-NAV-04, FR-VIEW-07
    #[test]
    fn going_is_navigate_and_the_navigator_key_works_everywhere() {
        let context = |command| {
            REGISTRY
                .iter()
                .find(|entry| entry.command == command)
                .map(|entry| entry.context)
        };
        for to in [Go::Into, Go::Up, Go::Home, Go::Back, Go::Forward] {
            assert_eq!(context(Command::Go(to)), Some(Context::Navigate), "{to:?}");
        }
        assert_eq!(context(Command::Navigator), Some(Context::Global));
    }

    // FR-EDIT-01, FR-EDIT-05, FR-OPS-04, FR-OPS-05
    #[test]
    fn editing_adding_and_deleting_are_navigate_commands() {
        for command in [
            Command::EditTitle,
            Command::NewTask(Side::Below),
            Command::NewTask(Side::Above),
            Command::Delete,
            Command::MoveTo,
        ] {
            let entry = REGISTRY.iter().find(|entry| entry.command == command);
            assert_eq!(
                entry.map(|entry| entry.context),
                Some(Context::Navigate),
                "{command:?}"
            );
        }
    }
}
