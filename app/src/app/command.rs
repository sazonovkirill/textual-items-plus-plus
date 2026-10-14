//! The command registry: every command a user can give, with its title and the context it is
//! given in. A frontend binds its own keys to these commands and lists them in its palette and
//! shortcuts panel.

use super::{Direction, End, Focus, Prefill, Side, view::View};
use crate::domain::dates::DateKind;

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
    /// Start visual line mode at the focused row, in which motions pick the rows from it to the
    /// focus, or end it (FR-OPS-01).
    VisualLines,
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
    /// Open the command palette, which runs the command chosen in it (FR-NAV-06).
    Palette,
    /// Open Go to Symbol, which shows the project, section or task chosen in it where it is
    /// (FR-NAV-05).
    GoToSymbol,
    /// Show the pane's search results and type the words to search for (FR-SRCH-01).
    Search,
    /// Open Quick Add, which adds a task to the Inbox or a place chosen in it, with the dates its
    /// words name, with what this prefills (FR-EDIT-07).
    QuickAdd(Prefill),
    /// Edit the focused row's title in place (FR-EDIT-01, FR-EDIT-02).
    EditTitle,
    /// Add an item of the focused row's kind on this side of it, a task in a pane without rows,
    /// and edit its title (FR-EDIT-05, FR-ORG-06, D-40).
    NewLike(Side),
    /// Add a task at this end of the container shown and edit its title (FR-EDIT-05, D-40).
    AddTask(End),
    /// Add a section at this end of the container shown and edit its title (FR-EDIT-05,
    /// FR-ORG-06, D-40).
    AddSection(End),
    /// Add a project at this end of the container shown, or of the top level from the sidebar,
    /// and edit its title (FR-EDIT-05, FR-ORG-06, D-40, D-45).
    AddProject(End),
    /// Add a project at this end of the area the sidebar focuses or holding the project it
    /// focuses, or of the container shown in the pane, and edit its title (FR-EDIT-05,
    /// FR-ORG-06, D-75).
    AddProjectInArea(End),
    /// Edit an item's description in place (FR-EDIT-03, FR-ORG-10).
    EditDescription(Whose),
    /// Open the attachments panel on an item's note (FR-ATT-01, FR-ATT-02).
    OpenNote(Whose),
    /// Open the attachments panel on an item's newest comment (FR-ATT-02, FR-ATT-04).
    OpenComments(Whose),
    /// Open the attachments panel on the focused task's checklist (FR-ATT-02, FR-CHK-02).
    OpenChecklist,
    /// Open the date prompt for the selected items' start or due date (FR-DATE-02, FR-OPS-06).
    SetDate(DateKind),
    /// Set the selected items' start date to today (FR-KEY-06, FR-OPS-06).
    StartToday,
    /// Mark the selected tasks and projects Later, or take them out of Later when all are
    /// (FR-ORG-11, FR-OPS-06).
    ToggleLater,
    /// Move the selected items' date a day earlier, a missing one to yesterday (FR-DATE-06,
    /// FR-OPS-06).
    DayEarlier(DateKind),
    /// Move the selected items' date a day later, a missing one to tomorrow (FR-DATE-06,
    /// FR-OPS-06).
    DayLater(DateKind),
    /// Move the selected items to the Trash, or in the Trash delete them for good (FR-OPS-05,
    /// FR-VIEW-04, OQ-19).
    Delete,
    /// In the Trash, put the selected items back where they were (FR-VIEW-04).
    Restore,
    /// In the Trash, delete everything in it for good, once asked again (FR-VIEW-04).
    EmptyTrash,
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
    /// Open the tag picker, which adds a tag to the selected items or removes it from them
    /// (FR-TAG-02, FR-OPS-06).
    Tags,
    /// Open the tag picker that chooses the tags the pane is filtered by (FR-TAG-04).
    FilterTags,
    /// Show every row again, filtered by no tag (FR-TAG-04).
    ClearTagFilter,
    /// Pin the selected projects, which the sidebar and the Navigator then also list among the
    /// pinned projects, or unpin them when all are pinned (FR-VIEW-12).
    TogglePinned,
    /// Show the sidebar and give it the keys, or hide it when it has them (FR-VIEW-10).
    Sidebar,
    /// Hide the projects inside the project the sidebar focuses, or show them again (FR-VIEW-10).
    Collapse,
    /// Give the keys to the pane on the left, or from the first pane to the sidebar while it
    /// shows (FR-PANE-02, FR-VIEW-10).
    FocusLeft,
    /// Give the keys to the pane on the right, or from the sidebar back to the pane (FR-PANE-02,
    /// FR-VIEW-10).
    FocusRight,
    /// Give the keys to the pane with this number, counted from 1 on the left, or with 0 to the
    /// sidebar, shown if hidden (FR-PANE-02, FR-VIEW-10).
    FocusPane(usize),
    /// Give the keys to the next pane, from the last back to the sidebar while it shows, else to
    /// the first pane (FR-PANE-02, FR-VIEW-10).
    NextPane,
    /// Give the keys to the previous pane, from the sidebar, or the first pane while it is hidden,
    /// to the last (FR-PANE-02, FR-VIEW-10).
    PreviousPane,
    /// Focus the sidebar's entry this way and show it in the first pane, the keys staying where
    /// they are (FR-VIEW-10).
    Browse(Direction),
    /// Show what the focused row opens, else the place shown, in a new pane right of the one with
    /// the keys, which gets them (FR-PANE-02).
    OpenInNewPane,
    /// Close the pane with the keys, unless it is the only one (FR-PANE-02).
    ClosePane,
    /// Make the pane with the keys narrower (FR-PANE-02).
    Narrow,
    /// Make the pane with the keys wider, as [`Narrow`](Self::Narrow) makes it narrower
    /// (FR-PANE-02).
    Widen,
    /// Show the shortcuts panel, or hide it (FR-KEY-03).
    Shortcuts,
    /// Scroll the shortcuts panel a page this way (FR-KEY-03).
    ScrollShortcuts(Direction),
    /// Open the settings window (FR-SET-02).
    Settings,
    /// Write the document to another file, which saving follows from then on (FR-STORE-11).
    SaveAs,
    /// Start the weather at once, whatever the settings say (FR-UI-08).
    StartWeather,
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
    /// To the Inbox (FR-NAV-02).
    Inbox,
    /// Back to where the pane was before (FR-NAV-04).
    Back,
    /// Forward again to where Back left (FR-NAV-04).
    Forward,
    /// To a view, such as Today (FR-VIEW-06, FR-KEY-06).
    View(View),
}

/// Whose description a [`Command::EditDescription`] edits, or whose attachments
/// [`Command::OpenNote`] and [`Command::OpenComments`] open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Whose {
    /// The focused item's, a description under its row.
    Focused,
    /// The container's shown in the pane, a description under its title.
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
        command: Command::VisualLines,
        title: "Visual Line Mode",
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
        command: Command::Browse(Direction::Down),
        title: "Next Sidebar Entry",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Browse(Direction::Up),
        title: "Previous Sidebar Entry",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::Inbox),
        title: "Go to Inbox",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::View(View::Today)),
        title: "Go to Today",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::View(View::Upcoming)),
        title: "Go to Upcoming",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::View(View::All)),
        title: "Go to All",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::View(View::Logbook)),
        title: "Go to Logbook",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Go(Go::View(View::Trash)),
        title: "Go to Trash",
        context: Context::Navigate,
    },
    Entry {
        command: Command::QuickAdd(Prefill::Nothing),
        title: "Quick Add",
        context: Context::Navigate,
    },
    Entry {
        command: Command::QuickAdd(Prefill::StartToday),
        title: "Quick Add Starting Today",
        context: Context::Navigate,
    },
    Entry {
        command: Command::QuickAdd(Prefill::Inbox),
        title: "Quick Add to Inbox",
        context: Context::Navigate,
    },
    Entry {
        command: Command::EditTitle,
        title: "Edit Title",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewLike(Side::Below),
        title: "New Item Below",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NewLike(Side::Above),
        title: "New Item Above",
        context: Context::Navigate,
    },
    Entry {
        command: Command::AddTask(End::Bottom),
        title: "New Task at Bottom",
        context: Context::Navigate,
    },
    Entry {
        command: Command::AddTask(End::Top),
        title: "New Task at Top",
        context: Context::Navigate,
    },
    Entry {
        command: Command::AddSection(End::Bottom),
        title: "New Section at Bottom",
        context: Context::Navigate,
    },
    Entry {
        command: Command::AddSection(End::Top),
        title: "New Section at Top",
        context: Context::Navigate,
    },
    Entry {
        command: Command::AddProject(End::Bottom),
        title: "New Project at Bottom",
        context: Context::Navigate,
    },
    Entry {
        command: Command::AddProject(End::Top),
        title: "New Project at Top",
        context: Context::Navigate,
    },
    Entry {
        command: Command::AddProjectInArea(End::Bottom),
        title: "New Project in Area at Bottom",
        context: Context::Navigate,
    },
    Entry {
        command: Command::AddProjectInArea(End::Top),
        title: "New Project in Area at Top",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Delete,
        title: "Delete",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Restore,
        title: "Restore",
        context: Context::Navigate,
    },
    Entry {
        command: Command::EmptyTrash,
        title: "Empty Trash",
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
        command: Command::OpenNote(Whose::Focused),
        title: "Open Note",
        context: Context::Navigate,
    },
    Entry {
        command: Command::OpenNote(Whose::Shown),
        title: "Open Container Note",
        context: Context::Navigate,
    },
    Entry {
        command: Command::OpenComments(Whose::Focused),
        title: "Open Comments",
        context: Context::Navigate,
    },
    Entry {
        command: Command::OpenComments(Whose::Shown),
        title: "Open Container Comments",
        context: Context::Navigate,
    },
    Entry {
        command: Command::OpenChecklist,
        title: "Open Checklist",
        context: Context::Navigate,
    },
    Entry {
        command: Command::SetDate(DateKind::Start),
        title: "Set Start Date",
        context: Context::Navigate,
    },
    Entry {
        command: Command::SetDate(DateKind::Due),
        title: "Set Due Date",
        context: Context::Navigate,
    },
    Entry {
        command: Command::StartToday,
        title: "Start Today",
        context: Context::Navigate,
    },
    Entry {
        command: Command::ToggleLater,
        title: "Toggle Later",
        context: Context::Navigate,
    },
    Entry {
        command: Command::DayEarlier(DateKind::Start),
        title: "Start a Day Earlier",
        context: Context::Navigate,
    },
    Entry {
        command: Command::DayLater(DateKind::Start),
        title: "Start a Day Later",
        context: Context::Navigate,
    },
    Entry {
        command: Command::DayEarlier(DateKind::Due),
        title: "Due a Day Earlier",
        context: Context::Navigate,
    },
    Entry {
        command: Command::DayLater(DateKind::Due),
        title: "Due a Day Later",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Tags,
        title: "Tags",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FilterTags,
        title: "Filter by Tags",
        context: Context::Navigate,
    },
    Entry {
        command: Command::ClearTagFilter,
        title: "Clear Tag Filter",
        context: Context::Navigate,
    },
    Entry {
        command: Command::TogglePinned,
        title: "Pin or Unpin Project",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Sidebar,
        title: "Toggle Sidebar",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Collapse,
        title: "Collapse or Expand Project",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusLeft,
        title: "Focus Pane Left",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusRight,
        title: "Focus Pane Right",
        context: Context::Navigate,
    },
    Entry {
        command: Command::NextPane,
        title: "Focus Next Pane",
        context: Context::Navigate,
    },
    Entry {
        command: Command::PreviousPane,
        title: "Focus Previous Pane",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(0),
        title: "Focus Sidebar",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(1),
        title: "Focus Pane 1",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(2),
        title: "Focus Pane 2",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(3),
        title: "Focus Pane 3",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(4),
        title: "Focus Pane 4",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(5),
        title: "Focus Pane 5",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(6),
        title: "Focus Pane 6",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(7),
        title: "Focus Pane 7",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(8),
        title: "Focus Pane 8",
        context: Context::Navigate,
    },
    Entry {
        command: Command::FocusPane(9),
        title: "Focus Pane 9",
        context: Context::Navigate,
    },
    Entry {
        command: Command::OpenInNewPane,
        title: "Open in New Pane",
        context: Context::Navigate,
    },
    Entry {
        command: Command::ClosePane,
        title: "Close Pane",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Narrow,
        title: "Narrow Pane",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Widen,
        title: "Widen Pane",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Shortcuts,
        title: "Toggle Shortcuts",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Palette,
        title: "Command Palette",
        context: Context::Navigate,
    },
    Entry {
        command: Command::GoToSymbol,
        title: "Go to Symbol",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Search,
        title: "Search",
        context: Context::Navigate,
    },
    Entry {
        command: Command::ScrollShortcuts(Direction::Down),
        title: "Scroll Shortcuts Down",
        context: Context::Navigate,
    },
    Entry {
        command: Command::ScrollShortcuts(Direction::Up),
        title: "Scroll Shortcuts Up",
        context: Context::Navigate,
    },
    Entry {
        command: Command::Settings,
        title: "Open Settings",
        context: Context::Navigate,
    },
    Entry {
        command: Command::SaveAs,
        title: "Save As…",
        context: Context::Navigate,
    },
    Entry {
        command: Command::StartWeather,
        title: "Start Weather Animation",
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
        let motions = [Focus::Next, Focus::Previous, Focus::First, Focus::Last];
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
            Command::VisualLines,
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
        for to in [
            Go::Inbox,
            Go::Back,
            Go::Forward,
            Go::View(View::Today),
            Go::View(View::Upcoming),
            Go::View(View::All),
            Go::View(View::Logbook),
            Go::View(View::Trash),
        ] {
            assert_eq!(context(Command::Go(to)), Some(Context::Navigate), "{to:?}");
        }
        assert_eq!(context(Command::Navigator), Some(Context::Global));
    }

    // FR-VIEW-10, FR-PANE-02
    #[test]
    fn the_sidebar_and_pane_commands_are_navigate_commands() {
        for command in [
            Command::Sidebar,
            Command::Collapse,
            Command::FocusLeft,
            Command::FocusRight,
            Command::NextPane,
            Command::PreviousPane,
            Command::Browse(Direction::Down),
            Command::Browse(Direction::Up),
            Command::FocusPane(0),
            Command::FocusPane(1),
            Command::FocusPane(9),
            Command::OpenInNewPane,
            Command::ClosePane,
            Command::Narrow,
            Command::Widen,
        ] {
            let entry = REGISTRY.iter().find(|entry| entry.command == command);
            assert_eq!(
                entry.map(|entry| entry.context),
                Some(Context::Navigate),
                "{command:?}"
            );
        }
    }

    // FR-DATE-02, FR-DATE-06, FR-KEY-06
    #[test]
    fn the_date_commands_are_navigate_commands() {
        let dating = [
            Command::SetDate(DateKind::Start),
            Command::SetDate(DateKind::Due),
            Command::StartToday,
            Command::DayEarlier(DateKind::Start),
            Command::DayLater(DateKind::Start),
            Command::DayEarlier(DateKind::Due),
            Command::DayLater(DateKind::Due),
        ];
        for command in dating {
            let entry = REGISTRY.iter().find(|entry| entry.command == command);
            assert_eq!(
                entry.map(|entry| entry.context),
                Some(Context::Navigate),
                "{command:?}"
            );
        }
    }

    // FR-EDIT-01, FR-EDIT-03, FR-EDIT-05, FR-ATT-02, FR-OPS-02, FR-OPS-03, FR-OPS-04, FR-OPS-05, FR-OPS-07,
    // FR-OPS-08, FR-ORG-06, FR-TASK-06
    #[test]
    fn editing_adding_deleting_and_the_clipboard_are_navigate_commands() {
        let beside = [Command::NewLike, Command::Paste]
            .into_iter()
            .flat_map(|new| [new(Side::Below), new(Side::Above)]);
        let at_ends = [
            Command::AddTask,
            Command::AddSection,
            Command::AddProject,
            Command::AddProjectInArea,
        ]
        .into_iter()
        .flat_map(|add| [add(End::Bottom), add(End::Top)]);
        let added = beside.chain(at_ends);
        let others = [
            Command::QuickAdd(Prefill::Nothing),
            Command::QuickAdd(Prefill::StartToday),
            Command::QuickAdd(Prefill::Inbox),
            Command::EditTitle,
            Command::EditDescription(Whose::Focused),
            Command::EditDescription(Whose::Shown),
            Command::OpenNote(Whose::Focused),
            Command::OpenNote(Whose::Shown),
            Command::OpenComments(Whose::Focused),
            Command::OpenComments(Whose::Shown),
            Command::OpenChecklist,
            Command::Delete,
            Command::Restore,
            Command::EmptyTrash,
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

    // FR-TAG-02, FR-TAG-04
    #[test]
    fn tagging_and_filtering_by_tags_are_navigate_commands() {
        for command in [Command::Tags, Command::FilterTags, Command::ClearTagFilter] {
            let entry = REGISTRY.iter().find(|entry| entry.command == command);
            assert_eq!(
                entry.map(|entry| entry.context),
                Some(Context::Navigate),
                "{command:?}"
            );
        }
    }
}
