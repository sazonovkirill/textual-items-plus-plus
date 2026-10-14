//! The command registry: every command a user can give, with its title and the context it is
//! given in. A frontend binds its own keys to these commands and lists them in its palette and
//! shortcuts panel.

use super::{Direction, Focus, Side};

/// A command: one user intent, whatever key, menu or palette gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Leave the app.
    Quit,
    /// Move the focus (FR-NAV-01).
    Focus(Focus),
    /// Move the focus a row, picking the rows from where the run of extensions began to it
    /// (FR-OPS-01).
    Extend(Direction),
    /// Pick the focused row, or unpick it when it is picked (FR-OPS-01).
    TogglePicked,
    /// Pick every row (FR-OPS-01).
    SelectAll,
    /// Pick nothing, so that commands take the focused row again (FR-OPS-01).
    ClearSelection,
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
    /// Edit the focused row's title in place (FR-EDIT-01, FR-EDIT-02).
    EditTitle,
    /// Add a task beside the focused row and edit its title (FR-EDIT-05).
    NewTask(Side),
    /// Add a section on this side of the focus and edit its title (FR-EDIT-05, FR-ORG-06).
    NewSection(Side),
    /// Add a project on this side of the focus and edit its title (FR-EDIT-05, FR-ORG-06).
    NewProject(Side),
    /// Add an area on this side of the one holding the container shown and edit its title
    /// (FR-EDIT-05, FR-ORG-06).
    NewArea(Side),
    /// Edit an item's description in place (FR-EDIT-03, FR-ORG-10).
    EditDescription(Whose),
    /// Move the selected items to the Trash (FR-OPS-05, OQ-19).
    Delete,
    /// Open the picker that moves the selected items to another container (FR-OPS-04).
    MoveTo,
    /// Put a copy of each selected item, with its contents, right after it (FR-OPS-05).
    Duplicate,
    /// Put the selected items, with their contents, on the clipboard (FR-OPS-05).
    Copy,
    /// Put the selected items, with their contents, on the clipboard and take them out
    /// (FR-OPS-05).
    Cut,
    /// Put the items on the clipboard in on this side of the focus (FR-OPS-05).
    Paste(Side),
    /// Edit the title of the container shown, in place (FR-ORG-06).
    RenameContainer,
    /// Move the container shown to the Trash (FR-ORG-06, OQ-19).
    DeleteContainer,
    /// Move the selected items up or down among the items they are shown with (FR-OPS-02,
    /// FR-OPS-10).
    Move(Direction),
    /// Indent the selected items into the item of their kind above them (FR-OPS-03).
    Indent,
    /// Outdent the selected items out of the item of their kind they are in (FR-OPS-03).
    Outdent,
    /// Convert the selected items to projects, or to sections when all are projects (FR-TASK-06,
    /// FR-ORG-06).
    Convert,
    /// Make the selected tasks the tasks of a new section and edit its title (FR-OPS-07).
    GroupIntoSection,
    /// Make the selected items the contents of a new project and edit its title (FR-OPS-08).
    GroupIntoProject,
    /// Show the sidebar and give it the keys, or hide it when it has them (FR-VIEW-10).
    Sidebar,
    /// Give the keys to the sidebar while it shows (FR-VIEW-10).
    FocusSidebar,
    /// Give the keys back to the pane (FR-VIEW-10).
    FocusPane,
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

/// Whose description a [`Command::EditDescription`] edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Whose {
    /// The focused item's, under its row.
    Focused,
    /// The container's shown in the pane, under its title.
    Shown,
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
        command: Command::Extend(Direction::Down),
        title: "Extend Selection Down",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Extend(Direction::Up),
        title: "Extend Selection Up",
        context: Context::Navigate,
    },
    Entry {
        command: Command::TogglePicked,
        title: "Toggle Selected",
        context: Context::Navigate,
    },
    Entry {
        command: Command::SelectAll,
        title: "Select All",
        context: Context::Navigate,
    },
    Entry {
        command: Command::ClearSelection,
        title: "Clear Selection",
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
        command: Command::NewSection(Side::Below),
        title: "New Section Below",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewSection(Side::Above),
        title: "New Section Above",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewProject(Side::Below),
        title: "New Project Below",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewProject(Side::Above),
        title: "New Project Above",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewArea(Side::Below),
        title: "New Area Below",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewArea(Side::Above),
        title: "New Area Above",
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
    Entry {
        command: Command::Duplicate,
        title: "Duplicate",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Copy,
        title: "Copy",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Cut,
        title: "Cut",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Paste(Side::Below),
        title: "Paste Below",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Paste(Side::Above),
        title: "Paste Above",
        context: Context::Navigate,
    },
    Entry {
        command: Command::RenameContainer,
        title: "Rename Container",
        context: Context::Navigate,
    },
    Entry {
        command: Command::DeleteContainer,
        title: "Delete Container",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Move(Direction::Up),
        title: "Move Up",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Move(Direction::Down),
        title: "Move Down",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Indent,
        title: "Indent",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Outdent,
        title: "Outdent",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Convert,
        title: "Convert to Project or Section",
        context: Context::Navigate,
    },
    Entry {
        command: Command::GroupIntoSection,
        title: "Group into Section",
        context: Context::Navigate,
    },
    Entry {
        command: Command::GroupIntoProject,
        title: "Group into Project",
        context: Context::Navigate,
    },
    Entry {
        command: Command::EditDescription(Whose::Focused),
        title: "Edit Description",
        context: Context::Navigate,
    },
    Entry {
        command: Command::EditDescription(Whose::Shown),
        title: "Edit Container Description",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Sidebar,
        title: "Toggle Sidebar",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusSidebar,
        title: "Focus Sidebar",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane,
        title: "Focus Pane",
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

    // FR-OPS-01
    #[test]
    fn selecting_is_navigate() {
        let selecting = [
            Command::Extend(Direction::Down),
            Command::Extend(Direction::Up),
            Command::TogglePicked,
            Command::SelectAll,
            Command::ClearSelection,
        ];
        for command in selecting {
            let entry = REGISTRY.iter().find(|entry| entry.command == command);
            assert_eq!(
                entry.map(|entry| entry.context),
                Some(Context::Navigate),
                "{command:?}"
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

    // FR-VIEW-10
    #[test]
    fn the_sidebar_commands_are_navigate_commands() {
        for command in [Command::Sidebar, Command::FocusSidebar, Command::FocusPane] {
            let entry = REGISTRY.iter().find(|entry| entry.command == command);
            assert_eq!(
                entry.map(|entry| entry.context),
                Some(Context::Navigate),
                "{command:?}"
            );
        }
    }

    // FR-EDIT-01, FR-EDIT-03, FR-EDIT-05, FR-OPS-02, FR-OPS-03, FR-OPS-04, FR-OPS-05, FR-OPS-07,
    // FR-OPS-08, FR-ORG-06, FR-TASK-06
    #[test]
    fn editing_adding_deleting_and_the_clipboard_are_navigate_commands() {
        let added = [
            Command::NewTask,
            Command::NewSection,
            Command::NewProject,
            Command::NewArea,
            Command::Paste,
        ]
        .into_iter()
        .flat_map(|new| [new(Side::Below), new(Side::Above)]);
        let others = [
            Command::EditTitle,
            Command::EditDescription(Whose::Focused),
            Command::EditDescription(Whose::Shown),
            Command::Delete,
            Command::MoveTo,
            Command::RenameContainer,
            Command::DeleteContainer,
            Command::Move(Direction::Up),
            Command::Move(Direction::Down),
            Command::Indent,
            Command::Outdent,
            Command::Duplicate,
            Command::Copy,
            Command::Cut,
            Command::Convert,
            Command::GroupIntoSection,
            Command::GroupIntoProject,
        ];
        for command in added.chain(others) {
            let entry = REGISTRY.iter().find(|entry| entry.command == command);
            assert_eq!(
                entry.map(|entry| entry.context),
                Some(Context::Navigate),
                "{command:?}"
            );
        }
    }
}
