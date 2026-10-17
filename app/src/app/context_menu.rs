//! The context menu: the commands that act on the selected items and apply to them, grouped as
//! the shortcuts panel groups them, filtered as the user types, running the one chosen, as
//! taskpaper-terminal's does (FR-KEY-07, D-106).

use super::{
    Candidate, Command, KeyGroup, Picked, Picker, PickerInput, PickerView, REGISTRY, State,
    attachments, converting, flagging, later, pinning, state,
    view::{Shown, View},
};
use crate::domain::{ItemId, ItemKind};

/// Which items a command acts on, as the context menu offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reach {
    /// The focused row alone, so offered only when nothing else is selected.
    Focused,
    /// The selected items, or the container shown while its heading has the focus.
    Described,
    /// The selected rows.
    Rows,
}

impl Reach {
    /// What `command` acts on, or `None` for a command that does not act on the selected items,
    /// such as a motion, or acts on the container shown.
    fn of(command: Command) -> Option<Self> {
        match command {
            Command::EditTitle
            | Command::EditItem
            | Command::New(_)
            | Command::EditDescription(super::Whose::Focused)
            | Command::OpenNote(super::Whose::Focused)
            | Command::OpenComments(super::Whose::Focused)
            | Command::OpenChecklist
            | Command::QuickComment
            | Command::OpenInNewPane => Some(Self::Focused),
            Command::SetDate(_)
            | Command::StartToday
            | Command::ToggleLater
            | Command::ToggleEvening
            | Command::ToggleFlagged
            | Command::DayEarlier(_)
            | Command::DayLater(_)
            | Command::Tags
            | Command::TogglePinned => Some(Self::Described),
            Command::ToggleCompleted
            | Command::Delete
            | Command::Restore
            | Command::MoveTo
            | Command::Duplicate
            | Command::Copy
            | Command::Cut
            | Command::Move(_)
            | Command::Convert
            | Command::GroupIntoSection
            | Command::GroupIntoProject => Some(Self::Rows),
            // The pane shows no nesting to fold (D-75), pasting puts in what the clipboard holds
            // rather than acting on the selection, opening is `Enter`'s, which the menu's own
            // `Enter` would stand for, and the rest act on the container shown or on no item.
            Command::Open
            | Command::Collapse
            | Command::Paste(_)
            | Command::EditDescription(super::Whose::Shown)
            | Command::OpenNote(super::Whose::Shown)
            | Command::OpenComments(super::Whose::Shown)
            | Command::RenameContainer
            | Command::DeleteContainer
            | Command::EmptyTrash
            | Command::Quit
            | Command::Focus(_)
            | Command::Extend(_)
            | Command::TogglePicked
            | Command::VisualLines
            | Command::SelectAll
            | Command::ClearSelection
            | Command::ShowCompleted
            | Command::Undo
            | Command::Redo
            | Command::Go(_)
            | Command::Navigator
            | Command::Palette
            | Command::ContextMenu
            | Command::GoToSymbol
            | Command::Search
            | Command::QuickAdd
            | Command::AddTask(_)
            | Command::AddSection(_)
            | Command::AddProject(_)
            | Command::AddArea(_)
            | Command::FilterTags
            | Command::ClearTagFilter
            | Command::Sidebar
            | Command::FocusLeft
            | Command::FocusRight
            | Command::FocusPane(_)
            | Command::NextPane
            | Command::PreviousPane
            | Command::Browse(_)
            | Command::ClosePane
            | Command::Narrow
            | Command::Widen
            | Command::Shortcuts
            | Command::ScrollShortcuts(_)
            | Command::Settings
            | Command::ChangeTheme
            | Command::SaveAs
            | Command::StartWeather
            | Command::KeepMine
            | Command::TakeTheirs
            | Command::ShowDiff => None,
        }
    }
}

impl State {
    /// The context menu, while it is open.
    #[must_use]
    pub fn context_menu(&self) -> Option<PickerView<'_, Command>> {
        self.context_menu.as_ref().map(Picker::view)
    }

    /// Opens the context menu on the commands that act on the selected items and apply to them,
    /// grouped as the shortcuts panel groups them, each group in the registry's order. Nothing
    /// opens when none does.
    pub(super) fn open_context_menu(&mut self) {
        let mut offered: Vec<(usize, Candidate<Command>)> = REGISTRY
            .iter()
            .filter(|entry| self.offers(entry.command))
            .filter_map(|entry| {
                let group = KeyGroup::ALL
                    .iter()
                    .position(|&group| group == KeyGroup::of(entry.command))?;
                let candidate = Candidate {
                    value: entry.command,
                    text: entry.title.to_owned(),
                    detail: String::new(),
                };
                Some((group, candidate))
            })
            .collect();
        offered.sort_by_key(|&(group, _)| group);
        let (groups, candidates): (Vec<_>, Vec<_>) = offered.into_iter().unzip();
        self.context_menu = (!candidates.is_empty()).then(|| Picker::grouped(candidates, groups));
    }

    /// Applies `input` to the open context menu, and returns the command chosen, which closes
    /// it, for the caller to give. Nothing happens while it is closed, nor for choosing in a new
    /// pane, which no command opens in.
    pub(super) fn context_menu_input(&mut self, input: PickerInput) -> Option<Command> {
        if input == PickerInput::ChooseInNewPane {
            return None;
        }
        let picked = self.context_menu.as_mut()?.input(input)?;
        self.context_menu = None;
        match picked {
            Picked::Chosen(command) => Some(command),
            Picked::Dismissed => None,
        }
    }

    /// Whether the context menu offers `command`: it acts on the selected items, there are such
    /// items, the app takes it now and it would change something for them; while the sidebar has
    /// the keys, on its focused area or project.
    fn offers(&self, command: Command) -> bool {
        if self.sidebar_has_keys() {
            return self.offers_on_entry(command);
        }
        let Some(reach) = Reach::of(command) else {
            return false;
        };
        let focused = self.pane().selection.focused();
        let reached = match reach {
            Reach::Focused => {
                focused.is_some() && self.described() == focused.into_iter().collect::<Vec<_>>()
            }
            Reach::Described => !self.described().is_empty(),
            Reach::Rows => !self.pane().selection.items(&self.listed()).is_empty(),
        };
        reached && self.takes(command) && self.applies(command, focused)
    }

    /// Whether the context menu offers `command` on the sidebar's focused entry: an area or a
    /// project, not the Inbox or a view (owner, D-106), which the sidebar takes `command` for, as
    /// on a pane's row but renaming it in place, since its title opens it, and with no checklist.
    fn offers_on_entry(&self, command: Command) -> bool {
        let Some(Shown::Container(id)) = self.sidebar.focused() else {
            return false;
        };
        let acts = match command {
            Command::RenameContainer => true,
            Command::EditTitle | Command::OpenChecklist | Command::ToggleEvening => false,
            command => Reach::of(command).is_some(),
        };
        let document = &self.document;
        let entry = [id];
        let applies = match command {
            Command::Convert => converting::convert_all(document, &entry).is_ok(),
            Command::ToggleLater => later::takes_any(document, &entry),
            Command::TogglePinned => pinning::toggle(document, &entry).is_some(),
            Command::ToggleFlagged => flagging::toggle(document, &entry).is_some(),
            Command::SetDate(_)
            | Command::StartToday
            | Command::DayEarlier(_)
            | Command::DayLater(_)
            | Command::Tags => !state::all_areas(document, &entry),
            _ => true,
        };
        id != document.inbox() && acts && applies && self.takes(command)
    }

    /// Whether `command` would change something for the selected items, `focused` the focused
    /// one: in the Trash only restoring and deleting for good do, a view but Today has no place to
    /// add or move items in, and the rest by what the items are.
    fn applies(&self, command: Command, focused: Option<ItemId>) -> bool {
        if self.shown() == Shown::View(View::Trash) {
            return matches!(command, Command::Restore | Command::Delete);
        }
        let document = &self.document;
        let group = |kind| {
            self.container().is_some()
                && matches!(
                    converting::group_place(document, &self.targets(), kind),
                    Ok(Some(_))
                )
        };
        match command {
            Command::Restore => false,
            Command::New(_) | Command::Move(_) => {
                self.container().is_some() || self.shown() == Shown::View(View::Today)
            }
            Command::GroupIntoSection => group(ItemKind::Section),
            Command::GroupIntoProject => group(ItemKind::Project),
            Command::Convert => converting::convert_all(document, &self.targets()).is_ok(),
            Command::OpenInNewPane => self.inside_focused().is_some(),
            Command::OpenChecklist => focused.is_some_and(|id| attachments::is_task(document, id)),
            Command::ToggleLater => later::takes_any(document, &self.described()),
            Command::TogglePinned => pinning::toggle(document, &self.described()).is_some(),
            Command::ToggleFlagged => flagging::toggle(document, &self.described()).is_some(),
            Command::ToggleEvening => self.shown() == Shown::View(View::Today),
            Command::SetDate(_)
            | Command::StartToday
            | Command::DayEarlier(_)
            | Command::DayLater(_)
            | Command::Tags => !state::all_areas(document, &self.described()),
            Command::ToggleCompleted => {
                !state::all_areas(document, &self.pane().selection.items(&self.listed()))
            }
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::{
        app::{Direction, Effect, Go, Message, Opened, Side, TextEdit, Whose, update},
        domain::{
            Document, Item,
            ItemKind::{Project, Section, Task},
            Node, PlacementError,
            dates::DateKind,
        },
    };

    const TODAY: Date = date(2026, 10, 14);

    /// The Inbox's tasks "a" and "b", and the project "Site" holding the section "S", whose
    /// task is "t", and the project "Blog".
    struct Fixture {
        state: State,
        site: ItemId,
        t: ItemId,
        blog: ItemId,
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        for title in ["a", "b"] {
            document.push(Some(inbox), Node::Item(Item::new(Task, title)))?;
        }
        let site = document.push(None, Node::Item(Item::new(Project, "Site")))?;
        let section = document.push(Some(site), Node::Item(Item::new(Section, "S")))?;
        let t = document.push(Some(section), Node::Item(Item::new(Task, "t")))?;
        let blog = document.push(Some(site), Node::Item(Item::new(Project, "Blog")))?;
        Ok(Fixture {
            state: State::new(document, Opened::Demo, TODAY),
            site,
            t,
            blog,
        })
    }

    fn give(state: &mut State, message: Message) -> Vec<Effect> {
        update(state, message, TODAY)
    }

    /// The commands the context menu lists, opening it first.
    fn offered(state: &mut State) -> Vec<Command> {
        give(state, Message::Command(Command::ContextMenu));
        let listed = state
            .context_menu()
            .map(|view| view.entries.iter().map(|entry| entry.value).collect())
            .unwrap_or_default();
        give(state, Message::ContextMenu(PickerInput::Dismiss));
        listed
    }

    fn titles(state: &State) -> Vec<&str> {
        state
            .rows()
            .iter()
            .filter_map(|row| Some(state.document().node(row.id())?.item()?.title.as_str()))
            .collect()
    }

    // FR-KEY-07
    #[test]
    fn the_menu_lists_the_commands_on_the_focused_task_grouped_in_the_registrys_order()
    -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        let listed = offered(&mut state);
        for command in [
            Command::EditTitle,
            Command::New(Side::Below),
            Command::OpenNote(Whose::Focused),
            Command::OpenChecklist,
            Command::ToggleCompleted,
            Command::SetDate(DateKind::Due),
            Command::ToggleLater,
            Command::Tags,
            Command::Delete,
            Command::Duplicate,
            Command::Move(Direction::Down),
        ] {
            assert!(listed.contains(&command), "{command:?} in {listed:?}");
        }
        for command in [
            Command::Restore,
            Command::TogglePinned,
            Command::OpenInNewPane,
            Command::Paste(Side::Below),
            Command::OpenNote(Whose::Shown),
            Command::Undo,
            Command::Focus(crate::app::Focus::Next),
            Command::ContextMenu,
        ] {
            assert!(!listed.contains(&command), "{command:?}");
        }
        let group = |command| {
            KeyGroup::ALL
                .iter()
                .position(|&g| g == KeyGroup::of(command))
        };
        let groups: Vec<_> = listed.iter().map(|&command| group(command)).collect();
        assert!(groups.is_sorted(), "grouped: {listed:?}");
        let registry = |command| REGISTRY.iter().position(|entry| entry.command == command);
        for pair in listed.windows(2) {
            if let [before, after] = *pair
                && group(before) == group(after)
            {
                assert!(registry(before) < registry(after), "{before:?} {after:?}");
            }
        }
        Ok(())
    }

    // FR-KEY-07, FR-OPS-01
    #[test]
    fn with_several_items_picked_the_commands_on_one_row_are_left_out() -> Result<(), PlacementError>
    {
        let Fixture { mut state, .. } = fixture()?;
        give(
            &mut state,
            Message::Command(Command::Extend(Direction::Down)),
        );
        let listed = offered(&mut state);
        for command in [
            Command::EditTitle,
            Command::New(Side::Above),
            Command::EditDescription(Whose::Focused),
            Command::OpenComments(Whose::Focused),
            Command::OpenChecklist,
        ] {
            assert!(!listed.contains(&command), "{command:?}");
        }
        for command in [Command::ToggleCompleted, Command::Tags, Command::Cut] {
            assert!(listed.contains(&command), "{command:?}");
        }
        Ok(())
    }

    // FR-KEY-07, FR-VIEW-12, FR-PANE-02, D-80
    #[test]
    fn what_a_row_is_decides_what_applies_to_it() -> Result<(), PlacementError> {
        let Fixture {
            mut state,
            site,
            t,
            blog,
        } = fixture()?;
        state.show(Shown::Container(site), Some(blog));
        let listed = offered(&mut state);
        for command in [
            Command::TogglePinned,
            Command::OpenInNewPane,
            Command::EditTitle,
        ] {
            assert!(listed.contains(&command), "{command:?} on a project");
        }
        assert!(!listed.contains(&Command::OpenChecklist));
        state.pane_mut().selection.focus(Some(t));
        let listed = offered(&mut state);
        assert!(listed.contains(&Command::OpenChecklist));
        for command in [Command::TogglePinned, Command::OpenInNewPane] {
            assert!(!listed.contains(&command), "{command:?} on a task here");
        }
        Ok(())
    }

    // FR-KEY-07, FR-VIEW-04, FR-VIEW-06
    #[test]
    fn a_view_offers_no_grouping_only_today_adding_and_moving_and_the_trash_only_restoring_and_deleting()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state, site, t, ..
        } = fixture()?;
        state.show(Shown::Container(site), Some(t));
        give(&mut state, Message::Command(Command::StartToday));
        give(
            &mut state,
            Message::Command(Command::Go(Go::View(View::Today))),
        );
        assert_eq!(titles(&state), ["t"]);
        let listed = offered(&mut state);
        assert!(!listed.contains(&Command::GroupIntoSection), "in a view");
        assert!(
            listed.contains(&Command::Move(Direction::Up)),
            "Today has an order of its own (D-108)"
        );
        assert!(
            listed.contains(&Command::New(Side::Below)),
            "Today takes a new task (D-128)"
        );
        assert!(
            listed.contains(&Command::OpenInNewPane),
            "where it is listed"
        );
        give(&mut state, Message::Command(Command::Delete));
        give(
            &mut state,
            Message::Command(Command::Go(Go::View(View::Trash))),
        );
        assert_eq!(offered(&mut state), [Command::Delete, Command::Restore]);
        Ok(())
    }

    // FR-KEY-07
    #[test]
    fn choosing_closes_the_menu_and_gives_the_command_and_nothing_opens_without_one()
    -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        give(&mut state, Message::Command(Command::ContextMenu));
        assert!(!state.takes(Command::Delete), "the menu has the keys");
        assert_eq!(
            give(
                &mut state,
                Message::ContextMenu(PickerInput::ChooseInNewPane)
            ),
            []
        );
        for c in "duplicate".chars() {
            give(
                &mut state,
                Message::ContextMenu(PickerInput::Edit(TextEdit::Insert(c))),
            );
        }
        give(&mut state, Message::ContextMenu(PickerInput::Choose));
        assert_eq!(state.context_menu(), None);
        assert_eq!(titles(&state), ["a", "a", "b"]);
        give(&mut state, Message::Command(Command::ContextMenu));
        give(&mut state, Message::Command(Command::Navigator));
        assert_eq!(state.context_menu(), None, "the Navigator closes it");
        give(&mut state, Message::Command(Command::Navigator));
        give(&mut state, Message::Command(Command::SelectAll));
        give(&mut state, Message::Command(Command::Cut));
        assert_eq!(offered(&mut state), [], "an empty Inbox");
        assert_eq!(state.context_menu(), None);
        Ok(())
    }

    // FR-KEY-07, FR-ATT-01
    #[test]
    fn nothing_opens_while_an_attachments_panel_has_the_keys() -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        give(
            &mut state,
            Message::Command(Command::OpenNote(Whose::Focused)),
        );
        assert!(state.attachments().is_some());
        give(&mut state, Message::Command(Command::ContextMenu));
        assert_eq!(state.context_menu(), None);
        Ok(())
    }

    // FR-KEY-07, FR-VIEW-10, D-106
    #[test]
    fn on_the_sidebar_the_menu_offers_what_it_takes_on_an_area_or_project_but_no_view()
    -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        give(&mut state, Message::Command(Command::Sidebar));
        assert_eq!(offered(&mut state), [], "nothing on the Inbox");
        give(
            &mut state,
            Message::Command(Command::Focus(crate::app::Focus::Next)),
        );
        assert_eq!(offered(&mut state), [], "nor on a view");
        give(
            &mut state,
            Message::Command(Command::Focus(crate::app::Focus::Last)),
        );
        assert!(matches!(state.sidebar.focused(), Some(Shown::Container(_))));
        let listed = offered(&mut state);
        for command in [
            Command::RenameContainer,
            Command::TogglePinned,
            Command::ToggleFlagged,
            Command::Delete,
            Command::OpenInNewPane,
        ] {
            assert!(listed.contains(&command), "{command:?}");
        }
        for command in [
            Command::EditTitle,
            Command::OpenChecklist,
            Command::Duplicate,
            Command::ToggleCompleted,
            Command::ToggleEvening,
        ] {
            assert!(!listed.contains(&command), "{command:?}");
        }
        Ok(())
    }
}
