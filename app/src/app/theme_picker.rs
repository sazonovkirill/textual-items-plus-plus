//! The theme picker: every built-in theme, the dark ones and then the light ones, filtered as the
//! user types, the app drawn at once in the one highlighted and kept in the one chosen, as VS
//! Code's Color Theme picker does (FR-UI-12, D-142).

use super::{BuiltIn, Candidate, Effect, Picked, Picker, PickerInput, PickerView, Setting, State};

/// The open theme picker, and the theme the app was drawn in when it opened, which dismissing it
/// gives back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ThemePicker {
    picker: Picker<BuiltIn>,
    before: BuiltIn,
}

impl State {
    /// The theme picker, while it is open.
    #[must_use]
    pub fn theme_picker(&self) -> Option<PickerView<'_, BuiltIn>> {
        self.theme_picker.as_ref().map(|open| open.picker.view())
    }

    /// Opens the theme picker on every built-in theme, the dark ones and then the light ones,
    /// each in [`BuiltIn::ALL`]'s order, the theme the app is drawn in highlighted.
    pub(super) fn open_theme_picker(&mut self) {
        let (light, dark): (Vec<_>, Vec<_>) =
            BuiltIn::ALL.into_iter().partition(|theme| theme.is_light());
        let themes: Vec<_> = dark.into_iter().chain(light).collect();
        let groups = themes
            .iter()
            .map(|theme| usize::from(theme.is_light()))
            .collect();
        let candidates = themes
            .into_iter()
            .map(|theme| Candidate {
                value: theme,
                text: theme.name().to_owned(),
                detail: String::new(),
            })
            .collect();
        let mut picker = Picker::grouped(candidates, groups);
        let before = self.settings.theme;
        picker.focus_where(|&theme| theme == before);
        self.theme_picker = Some(ThemePicker { picker, before });
    }

    /// Applies `input` to the open theme picker: the app is drawn in the theme highlighted, while
    /// one is; choosing one closes the picker and asks the frontend to keep it, unless it is the
    /// theme the picker opened on, and dismissing it draws the app in that theme again. Nothing
    /// happens while it is closed, nor for choosing in a new pane, which no theme opens in.
    pub(super) fn theme_picker_input(&mut self, input: PickerInput) -> Vec<Effect> {
        if input == PickerInput::ChooseInNewPane {
            return Vec::new();
        }
        let Some(open) = self.theme_picker.as_mut() else {
            return Vec::new();
        };
        let Some(picked) = open.picker.input(input) else {
            let view = open.picker.view();
            if let Some(theme) = view.entries.iter().nth(view.focused) {
                self.settings.theme = theme.value;
            }
            return Vec::new();
        };
        let before = open.before;
        self.theme_picker = None;
        match picked {
            Picked::Chosen(theme) if theme != before => {
                self.settings.theme = theme;
                vec![Effect::KeepSetting(Setting::Theme)]
            }
            Picked::Chosen(_) | Picked::Dismissed => {
                self.settings.theme = before;
                Vec::new()
            }
        }
    }

    /// Closes the theme picker, while it is open, drawing the app in the theme it opened on again.
    pub(super) fn dismiss_theme_picker(&mut self) {
        if let Some(open) = self.theme_picker.take() {
            self.settings.theme = open.before;
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::{
        app::{Command, Message, Opened, Settings, TextEdit, update},
        domain::Document,
    };

    const TODAY: Date = date(2026, 10, 15);

    /// The app drawn in `theme`, as the settings file sets it.
    fn drawn_in(theme: BuiltIn) -> State {
        let settings = Settings {
            theme,
            ..Settings::default()
        };
        State::new(Document::new(), Opened::Demo, TODAY)
            .with_settings(Settings::default(), settings)
    }

    fn give(state: &mut State, message: Message) -> Vec<Effect> {
        update(state, message, TODAY)
    }

    fn input(state: &mut State, input: PickerInput) -> Vec<Effect> {
        give(state, Message::ThemePicker(input))
    }

    fn typed(state: &mut State, query: &str) {
        for c in query.chars() {
            input(state, PickerInput::Edit(TextEdit::Insert(c)));
        }
    }

    fn listed(state: &State) -> Vec<BuiltIn> {
        state
            .theme_picker()
            .map(|view| view.entries.iter().map(|entry| entry.value).collect())
            .unwrap_or_default()
    }

    fn highlighted(state: &State) -> Option<BuiltIn> {
        let view = state.theme_picker()?;
        view.entries
            .iter()
            .nth(view.focused)
            .map(|entry| entry.value)
    }

    // FR-UI-12
    #[test]
    fn the_picker_lists_the_dark_themes_then_the_light_ones_the_current_one_highlighted() {
        let mut state = drawn_in(BuiltIn::Moonfly);
        assert_eq!(state.theme_picker(), None);
        give(&mut state, Message::Command(Command::ChangeTheme));
        let listed = listed(&state);
        assert_eq!(listed.len(), BuiltIn::ALL.len());
        let first_light = listed.iter().position(|theme| theme.is_light());
        assert_eq!(first_light, Some(16), "sixteen dark, then the light ones");
        assert!(listed.iter().skip(16).all(|theme| theme.is_light()));
        assert_eq!(listed.first(), Some(&BuiltIn::TerminalDark));
        assert_eq!(listed.get(3), Some(&BuiltIn::RosePine), "in ALL's order");
        assert_eq!(listed.get(16), Some(&BuiltIn::GithubLight));
        assert_eq!(highlighted(&state), Some(BuiltIn::Moonfly));
        assert_eq!(state.settings().theme, BuiltIn::Moonfly);
    }

    // FR-UI-12
    #[test]
    fn moving_the_highlight_draws_the_app_in_that_theme_and_dismissing_restores_it() {
        let mut state = drawn_in(BuiltIn::TerminalDark);
        give(&mut state, Message::Command(Command::ChangeTheme));
        assert_eq!(input(&mut state, PickerInput::Next), []);
        assert_eq!(state.settings().theme, BuiltIn::CatppuccinMocha);
        input(&mut state, PickerInput::Next);
        assert_eq!(state.settings().theme, BuiltIn::TokyoNight);
        input(&mut state, PickerInput::Previous);
        assert_eq!(state.settings().theme, BuiltIn::CatppuccinMocha);
        assert_eq!(input(&mut state, PickerInput::Dismiss), []);
        assert_eq!(state.theme_picker(), None);
        assert_eq!(state.settings().theme, BuiltIn::TerminalDark);
    }

    // FR-UI-12
    #[test]
    fn typing_filters_the_themes_and_choosing_one_keeps_it() {
        let mut state = drawn_in(BuiltIn::TerminalDark);
        give(&mut state, Message::Command(Command::ChangeTheme));
        typed(&mut state, "gruv");
        assert_eq!(
            listed(&state),
            [BuiltIn::GruvboxDark, BuiltIn::GruvboxLight]
        );
        assert_eq!(state.settings().theme, BuiltIn::GruvboxDark, "previewed");
        typed(&mut state, " light");
        assert_eq!(state.settings().theme, BuiltIn::GruvboxLight);
        typed(&mut state, "zz");
        assert_eq!(listed(&state), []);
        assert_eq!(
            state.settings().theme,
            BuiltIn::GruvboxLight,
            "with nothing highlighted, the last one previewed stays"
        );
        assert_eq!(input(&mut state, PickerInput::Choose), []);
        assert!(state.theme_picker().is_some(), "nothing to choose");
        input(&mut state, PickerInput::Edit(TextEdit::DeleteBackward));
        input(&mut state, PickerInput::Edit(TextEdit::DeleteBackward));
        assert_eq!(
            input(&mut state, PickerInput::Choose),
            [Effect::KeepSetting(Setting::Theme)]
        );
        assert_eq!(state.theme_picker(), None);
        assert_eq!(state.settings().theme, BuiltIn::GruvboxLight);
    }

    // FR-UI-12
    #[test]
    fn choosing_the_theme_it_opened_on_keeps_nothing_and_no_theme_opens_in_a_new_pane() {
        let mut state = drawn_in(BuiltIn::Dayfox);
        let before = state.clone();
        assert_eq!(input(&mut state, PickerInput::Choose), []);
        assert_eq!(state, before, "nothing happens while it is closed");
        give(&mut state, Message::Command(Command::ChangeTheme));
        input(&mut state, PickerInput::ChooseInNewPane);
        assert!(state.theme_picker().is_some());
        assert_eq!(input(&mut state, PickerInput::Choose), []);
        assert_eq!(state.theme_picker(), None);
        assert_eq!(state.settings().theme, BuiltIn::Dayfox);
    }

    // FR-UI-12, FR-NAV-06
    #[test]
    fn the_palette_opens_the_picker_which_takes_the_keys_and_the_navigator_closes_it() {
        let mut state = drawn_in(BuiltIn::TerminalDark);
        give(&mut state, Message::Command(Command::Palette));
        for c in "change theme".chars() {
            give(
                &mut state,
                Message::Palette(PickerInput::Edit(TextEdit::Insert(c))),
            );
        }
        give(&mut state, Message::Palette(PickerInput::Choose));
        assert_eq!(state.palette(), None);
        assert_eq!(highlighted(&state), Some(BuiltIn::TerminalDark));
        assert!(!state.takes(Command::ChangeTheme));
        assert!(!state.takes(Command::Palette));
        assert!(state.takes(Command::Navigator));
        input(&mut state, PickerInput::Next);
        give(&mut state, Message::Command(Command::Navigator));
        assert!(state.theme_picker().is_none() && state.navigator().is_some());
        assert_eq!(state.settings().theme, BuiltIn::TerminalDark, "restored");
        give(&mut state, Message::Command(Command::Navigator));
        give(&mut state, Message::Command(Command::Sidebar));
        assert!(state.sidebar_has_keys());
        give(&mut state, Message::Command(Command::ChangeTheme));
        assert!(state.theme_picker().is_some(), "from the sidebar too");
    }
}
