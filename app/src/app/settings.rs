//! The settings and the settings window: which built-in theme the app draws in, which icons it
//! draws, whether long text wraps and whether and when the weather plays, each changed in the
//! window and kept at once, as taskpaper-terminal does ([ui.md §8](../../../specs/reference/taskpaper-terminal/specs/ui.md#8-settings-window-fr-set-02-fr-set-03),
//! FR-SET-02, FR-SET-03, FR-UI-03, FR-UI-04, FR-UI-05, FR-UI-08). Names each choice; how it
//! looks, and the file the settings are kept in, are the frontend's.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{Direction, Effect, Notice, State};

/// One of the built-in themes (FR-UI-03), kept by its [`id`](Self::id).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltIn {
    /// The terminal's own colors, so it follows the user's palette; the default, because it
    /// looks right in any terminal, truecolor or not, until settings choose another (D-30).
    #[default]
    TerminalDark,
    /// Catppuccin Mocha, dark.
    CatppuccinMocha,
    /// Tokyo Night (Night), dark.
    TokyoNight,
    /// GitHub Light Default, light.
    GithubLight,
    /// Flexoki Light, light.
    FlexokiLight,
}

impl BuiltIn {
    /// Every built-in theme, in the order themes.md lists them, Terminal Dark first.
    pub const ALL: [Self; 5] = [
        Self::TerminalDark,
        Self::CatppuccinMocha,
        Self::TokyoNight,
        Self::GithubLight,
        Self::FlexokiLight,
    ];

    /// The theme's key, such as `catppuccin-mocha`, as themes.md gives it.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::TerminalDark => "terminal-dark",
            Self::CatppuccinMocha => "catppuccin-mocha",
            Self::TokyoNight => "tokyo-night",
            Self::GithubLight => "github-light",
            Self::FlexokiLight => "flexoki-light",
        }
    }

    /// The theme's name on screen, such as `Catppuccin Mocha`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::TerminalDark => "Terminal Dark",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::TokyoNight => "Tokyo Night",
            Self::GithubLight => "GitHub Light Default",
            Self::FlexokiLight => "Flexoki Light",
        }
    }
}

/// The glyphs the frontend draws for its icons (FR-UI-04), kept as `nerd` or `plain`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconSet {
    /// Nerd Font glyphs, the default.
    #[default]
    Nerd,
    /// Plain Unicode markers, for terminals without a Nerd Font.
    Plain,
}

/// The settings the app runs with, each kept under its [`Setting::key`] (FR-SET-01, FR-SET-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The theme the app draws in (FR-UI-03).
    pub theme: BuiltIn,
    /// The icons it draws (FR-UI-04).
    pub icons: IconSet,
    /// Whether long text wraps; without wrapping, a row's title takes one line and its
    /// description's first line one more, cut short (FR-UI-05).
    pub word_wrap: bool,
    /// Whether the weather starts once the app has had no input for
    /// [`weather_idle_minutes`](Self::weather_idle_minutes) (FR-UI-08).
    pub weather: bool,
    /// How many minutes without input the weather waits for (FR-UI-08).
    pub weather_idle_minutes: NonZeroU32,
}

impl Default for Settings {
    /// Terminal Dark, Nerd Font icons and wrapping, as the app drew before it had settings
    /// (D-30), and the weather after three minutes, as in taskpaper-terminal.
    fn default() -> Self {
        Self {
            theme: BuiltIn::default(),
            icons: IconSet::default(),
            word_wrap: true,
            weather: true,
            weather_idle_minutes: NonZeroU32::new(3).expect("three is not zero"),
        }
    }
}

/// One setting, as the settings window lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// The theme.
    Theme,
    /// The icons.
    Icons,
    /// Word wrap.
    WordWrap,
    /// Whether the weather plays once the app sits idle.
    Weather,
    /// How long the app sits idle before the weather starts.
    WeatherIdle,
}

impl Setting {
    /// Every setting, in the order the window lists them.
    pub const ALL: [Self; 5] = [
        Self::Theme,
        Self::Icons,
        Self::WordWrap,
        Self::Weather,
        Self::WeatherIdle,
    ];

    /// The key the settings file keeps it under, such as `word_wrap`.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Icons => "icons",
            Self::WordWrap => "word_wrap",
            Self::Weather => "weather",
            Self::WeatherIdle => "weather_idle_minutes",
        }
    }

    /// Its name in the window.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Theme => "Theme",
            Self::Icons => "Icons",
            Self::WordWrap => "Word wrap",
            Self::Weather => "Weather animation",
            Self::WeatherIdle => "Weather after idle",
        }
    }

    /// The topic the window lists it under.
    #[must_use]
    pub fn group(self) -> &'static str {
        match self {
            Self::Theme | Self::Icons | Self::WordWrap => "Appearance",
            Self::Weather | Self::WeatherIdle => "Weather",
        }
    }

    /// What it does, in a line.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::Theme => "The colors the app draws in",
            Self::Icons => "Nerd Font icons, or plain markers without the font",
            Self::WordWrap => "Long titles wrap; off, each takes one line, cut short",
            Self::Weather => "The season's weather falls over the app while it sits idle",
            Self::WeatherIdle => "How long the app sits idle before the weather starts",
        }
    }
}

impl Settings {
    /// How `setting`'s value reads in the window.
    #[must_use]
    pub fn value(&self, setting: Setting) -> String {
        let switch = |on| if on { "On" } else { "Off" };
        match setting {
            Setting::Theme => self.theme.name().to_owned(),
            Setting::Icons => match self.icons {
                IconSet::Nerd => "Nerd Font".to_owned(),
                IconSet::Plain => "Plain".to_owned(),
            },
            Setting::WordWrap => switch(self.word_wrap).to_owned(),
            Setting::Weather => switch(self.weather).to_owned(),
            Setting::WeatherIdle => match self.weather_idle_minutes.get() {
                1 => "1 minute".to_owned(),
                minutes => format!("{minutes} minutes"),
            },
        }
    }

    /// Whether `setting` has its default value.
    #[must_use]
    pub fn is_default(&self, setting: Setting) -> bool {
        let default = Self::default();
        match setting {
            Setting::Theme => self.theme == default.theme,
            Setting::Icons => self.icons == default.icons,
            Setting::WordWrap => self.word_wrap == default.word_wrap,
            Setting::Weather => self.weather == default.weather,
            Setting::WeatherIdle => self.weather_idle_minutes == default.weather_idle_minutes,
        }
    }

    /// Gives `setting` its next value, toward `Down`, or its previous one: going round, the next
    /// theme of [`BuiltIn::ALL`], the other icons, wrapping or the weather turned over; a minute
    /// more or less of idle time, no less than one.
    fn turn(&mut self, setting: Setting, direction: Direction) {
        match setting {
            Setting::Theme => {
                let count = BuiltIn::ALL.len();
                let at = BuiltIn::ALL
                    .iter()
                    .position(|&theme| theme == self.theme)
                    .unwrap_or_default();
                let next = match direction {
                    Direction::Down => at + 1,
                    Direction::Up => at + count - 1,
                } % count;
                if let Some(&theme) = BuiltIn::ALL.get(next) {
                    self.theme = theme;
                }
            }
            Setting::Icons => {
                self.icons = match self.icons {
                    IconSet::Nerd => IconSet::Plain,
                    IconSet::Plain => IconSet::Nerd,
                };
            }
            Setting::WordWrap => self.word_wrap = !self.word_wrap,
            Setting::Weather => self.weather = !self.weather,
            Setting::WeatherIdle => {
                let minutes = self.weather_idle_minutes;
                self.weather_idle_minutes = match direction {
                    Direction::Down => minutes.saturating_add(1),
                    Direction::Up => NonZeroU32::new(minutes.get() - 1).unwrap_or(minutes),
                };
            }
        }
    }

    /// Gives `setting` its default value.
    fn reset(&mut self, setting: Setting) {
        let default = Self::default();
        match setting {
            Setting::Theme => self.theme = default.theme,
            Setting::Icons => self.icons = default.icons,
            Setting::WordWrap => self.word_wrap = default.word_wrap,
            Setting::Weather => self.weather = default.weather,
            Setting::WeatherIdle => self.weather_idle_minutes = default.weather_idle_minutes,
        }
    }
}

/// What the user does in the open settings window (FR-SET-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsInput {
    /// Highlights the setting this way, stopping at the ends.
    Focus(Direction),
    /// Gives the highlighted setting its next value, toward `Down`, or its previous one.
    Turn(Direction),
    /// Gives the highlighted setting its default value.
    Reset,
    /// Closes the window.
    Close,
}

impl State {
    /// The settings the app runs with.
    #[must_use]
    pub fn settings(&self) -> Settings {
        self.settings
    }

    /// The app with `settings`, as the settings file sets them, from the start.
    #[must_use]
    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    /// The setting highlighted in the settings window, while it is open (FR-SET-02).
    #[must_use]
    pub fn settings_window(&self) -> Option<Setting> {
        self.settings_window
    }

    /// Opens the settings window on its first setting.
    pub(super) fn open_settings(&mut self) {
        self.settings_window = Some(Setting::Theme);
    }

    /// Takes `input` in the open settings window: a change applies at once and asks the
    /// frontend to keep it (FR-SET-02).
    pub(super) fn settings_input(&mut self, input: SettingsInput) -> Vec<Effect> {
        let Some(setting) = self.settings_window else {
            return Vec::new();
        };
        let before = self.settings;
        match input {
            SettingsInput::Focus(direction) => {
                let at = Setting::ALL
                    .iter()
                    .position(|&shown| shown == setting)
                    .unwrap_or_default();
                let to = match direction {
                    Direction::Down => at + 1,
                    Direction::Up => at.saturating_sub(1),
                };
                self.settings_window = Setting::ALL.get(to).copied().or(Some(setting));
            }
            SettingsInput::Turn(direction) => self.settings.turn(setting, direction),
            SettingsInput::Reset => self.settings.reset(setting),
            SettingsInput::Close => self.settings_window = None,
        }
        if self.settings == before {
            Vec::new()
        } else {
            vec![Effect::KeepSetting(setting)]
        }
    }

    /// Has the status line say what went wrong with the settings file, such as what it sets
    /// that the app ignored or that a setting could not be kept (FR-SET-04).
    pub(super) fn settings_failed(&mut self, problem: String) {
        self.notify(Notice::Settings(problem));
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::{
        app::{Command, Message, Opened, update},
        domain::Document,
    };

    const TODAY: Date = date(2026, 10, 14);

    fn give(state: &mut State, message: Message) -> Vec<Effect> {
        update(state, message, TODAY)
    }

    fn input(state: &mut State, input: SettingsInput) -> Vec<Effect> {
        give(state, Message::Settings(input))
    }

    // FR-SET-02, FR-SET-03, FR-UI-03
    #[test]
    fn the_window_turns_the_highlighted_setting_and_asks_to_keep_each_change() {
        let mut state = State::new(Document::new(), Opened::Demo, TODAY);
        assert_eq!(state.settings_window(), None);
        assert_eq!(state.settings(), Settings::default());
        give(&mut state, Message::Command(Command::Settings));
        assert_eq!(state.settings_window(), Some(Setting::Theme));
        let themes: Vec<_> = (0..5)
            .map(|_| {
                let effects = input(&mut state, SettingsInput::Turn(Direction::Down));
                assert_eq!(effects, [Effect::KeepSetting(Setting::Theme)]);
                state.settings().theme
            })
            .collect();
        assert_eq!(
            themes,
            [
                BuiltIn::CatppuccinMocha,
                BuiltIn::TokyoNight,
                BuiltIn::GithubLight,
                BuiltIn::FlexokiLight,
                BuiltIn::TerminalDark,
            ],
            "going round"
        );
        input(&mut state, SettingsInput::Turn(Direction::Up));
        assert_eq!(state.settings().theme, BuiltIn::FlexokiLight);
        assert!(!state.settings().is_default(Setting::Theme));
        assert_eq!(
            input(&mut state, SettingsInput::Reset),
            [Effect::KeepSetting(Setting::Theme)]
        );
        assert_eq!(state.settings().theme, BuiltIn::TerminalDark);
        assert_eq!(
            input(&mut state, SettingsInput::Reset),
            [],
            "nothing changed, nothing to keep"
        );
    }

    // FR-SET-02, FR-UI-04, FR-UI-05
    #[test]
    fn the_highlight_moves_through_the_settings_and_stops_at_the_ends() {
        let mut state = State::new(Document::new(), Opened::Demo, TODAY);
        give(&mut state, Message::Command(Command::Settings));
        input(&mut state, SettingsInput::Focus(Direction::Up));
        assert_eq!(state.settings_window(), Some(Setting::Theme));
        input(&mut state, SettingsInput::Focus(Direction::Down));
        assert_eq!(state.settings_window(), Some(Setting::Icons));
        input(&mut state, SettingsInput::Turn(Direction::Up));
        assert_eq!(state.settings().icons, IconSet::Plain);
        assert_eq!(state.settings().value(Setting::Icons), "Plain");
        input(&mut state, SettingsInput::Turn(Direction::Down));
        assert_eq!(state.settings().icons, IconSet::Nerd, "going round");
        input(&mut state, SettingsInput::Turn(Direction::Down));
        input(&mut state, SettingsInput::Reset);
        assert_eq!(state.settings().value(Setting::Icons), "Nerd Font");
        input(&mut state, SettingsInput::Focus(Direction::Down));
        assert_eq!(state.settings_window(), Some(Setting::WordWrap));
        assert_eq!(state.settings().value(Setting::WordWrap), "On");
        assert_eq!(
            input(&mut state, SettingsInput::Turn(Direction::Down)),
            [Effect::KeepSetting(Setting::WordWrap)]
        );
        assert_eq!(state.settings().value(Setting::WordWrap), "Off");
        input(&mut state, SettingsInput::Reset);
        assert!(state.settings().word_wrap);
        input(&mut state, SettingsInput::Turn(Direction::Up));
        input(&mut state, SettingsInput::Close);
        assert_eq!(state.settings_window(), None);
        assert!(!state.settings().word_wrap, "closing keeps the changes");
        assert_eq!(input(&mut state, SettingsInput::Reset), [], "closed");
        assert!(!state.settings().word_wrap);
    }

    // FR-SET-02
    #[test]
    fn while_the_window_is_open_only_global_commands_are_taken() {
        let mut state = State::new(Document::new(), Opened::Demo, TODAY);
        assert!(state.takes(Command::Settings));
        give(&mut state, Message::Command(Command::Settings));
        assert!(!state.takes(Command::Settings));
        assert!(!state.takes(Command::Undo));
        assert!(state.takes(Command::Quit));
    }

    // FR-SET-04
    #[test]
    fn a_problem_with_the_settings_file_shows_until_the_next_command() {
        let mut state = State::new(Document::new(), Opened::Demo, TODAY).with_settings(Settings {
            word_wrap: false,
            ..Settings::default()
        });
        assert!(!state.settings().word_wrap);
        give(
            &mut state,
            Message::SettingsFailed("unknown setting `x`".into()),
        );
        assert_eq!(
            state.notice(),
            Some(&Notice::Settings("unknown setting `x`".into()))
        );
        give(&mut state, Message::Command(Command::Undo));
        assert_eq!(state.notice(), None);
    }

    // FR-SET-01
    #[test]
    fn themes_are_kept_by_their_id_and_icons_as_nerd_or_plain() -> Result<(), serde_json::Error> {
        for theme in BuiltIn::ALL {
            assert_eq!(serde_json::to_value(theme)?, theme.id());
        }
        assert_eq!(serde_json::to_value(IconSet::Plain)?, "plain");
        assert_eq!(serde_json::to_value(IconSet::Nerd)?, "nerd");
        Ok(())
    }

    // FR-SET-01
    #[test]
    fn each_setting_is_kept_under_its_key_with_the_values_the_window_names() {
        for setting in Setting::ALL {
            assert!(!setting.name().is_empty() && !setting.description().is_empty());
        }
        let groups: Vec<_> = Setting::ALL.iter().map(|setting| setting.group()).collect();
        assert_eq!(
            groups,
            [
                "Appearance",
                "Appearance",
                "Appearance",
                "Weather",
                "Weather"
            ]
        );
        let keys: Vec<_> = Setting::ALL.iter().map(|setting| setting.key()).collect();
        assert_eq!(
            keys,
            [
                "theme",
                "icons",
                "word_wrap",
                "weather",
                "weather_idle_minutes"
            ]
        );
        let names: Vec<_> = Setting::ALL.iter().map(|setting| setting.name()).collect();
        assert_eq!(
            names,
            [
                "Theme",
                "Icons",
                "Word wrap",
                "Weather animation",
                "Weather after idle"
            ]
        );
        for theme in BuiltIn::ALL {
            let settings = Settings {
                theme,
                ..Settings::default()
            };
            assert_eq!(settings.value(Setting::Theme), theme.name());
        }
    }

    // FR-UI-08, FR-SET-02
    #[test]
    fn the_weather_turns_over_and_its_idle_time_goes_by_minutes_no_less_than_one() {
        let mut state = State::new(Document::new(), Opened::Demo, TODAY);
        give(&mut state, Message::Command(Command::Settings));
        for _ in 0..3 {
            input(&mut state, SettingsInput::Focus(Direction::Down));
        }
        assert_eq!(state.settings_window(), Some(Setting::Weather));
        assert_eq!(state.settings().value(Setting::Weather), "On");
        assert_eq!(
            input(&mut state, SettingsInput::Turn(Direction::Up)),
            [Effect::KeepSetting(Setting::Weather)]
        );
        assert!(!state.settings().weather);
        assert_eq!(state.settings().value(Setting::Weather), "Off");
        input(&mut state, SettingsInput::Reset);
        assert!(state.settings().weather);
        input(&mut state, SettingsInput::Focus(Direction::Down));
        input(&mut state, SettingsInput::Focus(Direction::Down));
        assert_eq!(
            state.settings_window(),
            Some(Setting::WeatherIdle),
            "the highlight stops at the last setting"
        );
        assert_eq!(state.settings().value(Setting::WeatherIdle), "3 minutes");
        assert_eq!(
            input(&mut state, SettingsInput::Turn(Direction::Down)),
            [Effect::KeepSetting(Setting::WeatherIdle)]
        );
        assert_eq!(state.settings().value(Setting::WeatherIdle), "4 minutes");
        assert!(!state.settings().is_default(Setting::WeatherIdle));
        for _ in 0..3 {
            input(&mut state, SettingsInput::Turn(Direction::Up));
        }
        assert_eq!(state.settings().value(Setting::WeatherIdle), "1 minute");
        assert_eq!(
            input(&mut state, SettingsInput::Turn(Direction::Up)),
            [],
            "no less than a minute"
        );
        input(&mut state, SettingsInput::Reset);
        assert!(state.settings().is_default(Setting::WeatherIdle));
    }

    // FR-UI-03
    #[test]
    fn the_four_themes_of_themes_md_and_terminal_dark_are_built_in() {
        let themes: Vec<_> = BuiltIn::ALL
            .into_iter()
            .map(|theme| (theme.id(), theme.name()))
            .collect();
        assert_eq!(
            themes,
            [
                ("terminal-dark", "Terminal Dark"),
                ("catppuccin-mocha", "Catppuccin Mocha"),
                ("tokyo-night", "Tokyo Night"),
                ("github-light", "GitHub Light Default"),
                ("flexoki-light", "Flexoki Light"),
            ]
        );
    }

    // FR-UI-03, D-30
    #[test]
    fn terminal_dark_is_the_default() {
        assert_eq!(BuiltIn::default(), BuiltIn::TerminalDark);
    }

    // FR-UI-04
    #[test]
    fn nerd_font_icons_are_the_default() {
        assert_eq!(IconSet::default(), IconSet::Nerd);
    }
}
