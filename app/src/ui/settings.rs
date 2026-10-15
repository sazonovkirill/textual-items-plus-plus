//! The settings window in the terminal (FR-SET-02), as taskpaper-terminal draws it
//! ([ui.md §8](../../../specs/reference/taskpaper-terminal/specs/ui.md#8-settings-window-fr-set-02-fr-set-03)):
//! the keys that drive it and a centered box in the accent color listing each group of settings
//! under its name, a setting on two lines, its name and value, then what it does, with the path
//! of the settings file at the foot.

use std::path::Path;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Widget},
};

use super::{
    Theme, cut,
    keymap::{self, Key},
    status,
};
use crate::app::{Direction, Setting, Settings, SettingsInput};

/// The keys of the open window.
pub(super) const KEYS: &[(Key, SettingsInput)] = &[
    (
        (KeyModifiers::NONE, KeyCode::Char('k')),
        SettingsInput::Focus(Direction::Up),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Up),
        SettingsInput::Focus(Direction::Up),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('j')),
        SettingsInput::Focus(Direction::Down),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Down),
        SettingsInput::Focus(Direction::Down),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('l')),
        SettingsInput::Turn(Direction::Down),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Right),
        SettingsInput::Turn(Direction::Down),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Enter),
        SettingsInput::Turn(Direction::Down),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char(' ')),
        SettingsInput::Turn(Direction::Down),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('h')),
        SettingsInput::Turn(Direction::Up),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Left),
        SettingsInput::Turn(Direction::Up),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('r')),
        SettingsInput::Reset,
    ),
    ((KeyModifiers::NONE, KeyCode::Esc), SettingsInput::Close),
];

/// What `event` does in the open window. Only key presses count.
pub(super) fn settings_input(event: &Event) -> Option<SettingsInput> {
    let key = keymap::key_press(event)?;
    KEYS.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
}

/// The largest the window gets across.
const WIDTH: u16 = 64;

/// The window: the settings with their values, the highlighted one on the selection colors and
/// in view, a `*` and the value in bold where it is not the default, and the settings file's
/// path, if there is one.
pub(super) struct SettingsWindow<'a> {
    settings: Settings,
    defaults: Settings,
    focused: Setting,
    file: Option<&'a Path>,
    theme: &'a Theme,
}

impl<'a> SettingsWindow<'a> {
    /// The window showing `settings`, marking where they differ from `defaults`, with `focused`
    /// highlighted and the path of `file`, drawn in `theme`.
    pub(super) fn new(
        (settings, defaults): (Settings, Settings),
        focused: Setting,
        file: Option<&'a Path>,
        theme: &'a Theme,
    ) -> Self {
        Self {
            settings,
            defaults,
            focused,
            file,
            theme,
        }
    }

    /// Its lines in `width` columns, each with whether it is the highlighted setting's.
    fn lines(&self, width: usize) -> Vec<(Line<'static>, bool)> {
        let dim = Style::new().fg(self.theme.dim_text);
        let mut lines = Vec::new();
        let mut group = None;
        for setting in Setting::ALL {
            if group != Some(setting.group()) {
                group = Some(setting.group());
                let heading = Span::styled(setting.group(), Style::new().fg(self.theme.primary));
                lines.push((Line::from(vec![Span::raw(" "), heading.bold()]), false));
            }
            let focused = setting == self.focused;
            lines.push((self.title(setting, width), focused));
            let description = Line::from(format!("   {}", setting.description())).style(dim);
            lines.push((cut::truncate(&description, width), focused));
        }
        lines.push((Line::default(), false));
        let path = self.file.map_or_else(
            || "no settings file: no $XDG_CONFIG_HOME or home folder".to_owned(),
            |file| file.display().to_string(),
        );
        let path = status::tail(&path, width.saturating_sub(2));
        lines.push((Line::styled(format!(" {path}"), dim), false));
        lines
    }

    /// The line naming `setting` in `width` columns: a `*` when it differs from its default, its
    /// name, cut short of its value a space before it, and its value at the right.
    fn title(&self, setting: Setting, width: usize) -> Line<'static> {
        let changed = !self.settings.is_default(setting, &self.defaults);
        let lead = Span::raw(if changed { " * " } else { "   " });
        let value = Span::raw(self.settings.value(setting));
        let value = if changed { value.bold() } else { value };
        let room = width.saturating_sub(lead.width() + value.width() + 2);
        let name = cut::truncate(&Line::from(setting.name()), room);
        let gap = Span::raw(" ".repeat(room.saturating_sub(name.width()) + 1));
        let mut spans = vec![lead];
        spans.extend(name.spans);
        spans.extend([gap, value, Span::raw(" ")]);
        Line::from(spans)
    }
}

impl Widget for SettingsWindow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = usize::from(WIDTH.min(area.width).saturating_sub(2));
        let lines = self.lines(width);
        let height = u16::try_from(lines.len() + 2).unwrap_or(u16::MAX);
        let area = area.centered(Constraint::Length(WIDTH), Constraint::Length(height));
        let theme = self.theme;
        Clear.render(area, buf);
        let accent = Style::new().fg(theme.accent);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(" Settings ");
        let inner = frame.inner(area);
        frame.render(area, buf);
        // A window too short for its lines scrolls only as far as shows the highlighted setting.
        let end = lines
            .iter()
            .rposition(|&(_, focused)| focused)
            .map_or(0, |last| last + 1);
        let offset = end.saturating_sub(usize::from(inner.height));
        for ((line, focused), row) in lines.into_iter().skip(offset).zip(inner.rows()) {
            if focused {
                buf.set_style(row, Style::new().bg(theme.selection).fg(theme.band_text));
            }
            line.render(row, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Modifier;

    use super::*;
    use crate::{app::IconSet, ui::BuiltIn};

    /// The window's rows drawn over a `width` by 22 area with `file`, trimmed.
    fn drawn(settings: Settings, file: Option<&Path>, width: u16) -> Vec<String> {
        let theme = BuiltIn::default().colors();
        let area = Rect::new(0, 0, width, 22);
        let mut buf = Buffer::empty(area);
        SettingsWindow::new(
            (settings, Settings::default()),
            Setting::Icons,
            file,
            &theme,
        )
        .render(area, &mut buf);
        (0..area.height)
            .map(|y| {
                let row: String = (0..area.width).map(|x| buf[(x, y)].symbol()).collect();
                row.trim_end().to_owned()
            })
            .collect()
    }

    // FR-SET-02
    #[test]
    fn a_narrow_window_cuts_names_descriptions_and_the_path_and_marks_changed_values() {
        let settings = Settings {
            icons: IconSet::Plain,
            ..Settings::default()
        };
        let path = Path::new("/home/someone/.config/textual-items-plus-plus/config.toml");
        assert_eq!(
            drawn(settings, Some(path), 24),
            [
                "",
                "┌ Settings ────────────┐",
                "│ Appearance           │",
                "│   The… Terminal Dark │",
                "│   The colors the app…│",
                "│   Grayscale      Off │",
                "│   The theme in its g…│",
                "│ * Icons        Plain │",
                "│   Nerd Font icons, o…│",
                "│   Word wrap       On │",
                "│   Long titles wrap; …│",
                "│   Animations    Full │",
                "│   Full, reduced to f…│",
                "│ Weather              │",
                "│   Weather animat… On │",
                "│   The season's weath…│",
                "│   Weather… 3 minutes │",
                "│   How long the app s…│",
                "│                      │",
                "│ …us-plus/config.toml │",
                "└──────────────────────┘",
                "",
            ]
        );
        let theme = BuiltIn::default().colors();
        let area = Rect::new(0, 0, 24, 22);
        let mut buf = Buffer::empty(area);
        SettingsWindow::new(
            (settings, Settings::default()),
            Setting::Icons,
            Some(path),
            &theme,
        )
        .render(area, &mut buf);
        assert_eq!(buf[(4, 7)].bg, theme.selection, "the highlighted setting");
        assert_eq!(buf[(4, 8)].bg, theme.selection, "and its description");
        assert_eq!(buf[(4, 4)].bg, theme.background);
        assert!(
            buf[(18, 7)].modifier.contains(Modifier::BOLD),
            "a changed value"
        );
        assert!(!buf[(18, 3)].modifier.contains(Modifier::BOLD));
    }

    // FR-SET-02, FR-UI-06
    #[test]
    fn a_short_window_scrolls_to_keep_the_highlighted_setting_in_view() {
        let theme = BuiltIn::default().colors();
        let area = Rect::new(0, 0, 40, 9);
        let mut buf = Buffer::empty(area);
        let settings = (Settings::default(), Settings::default());
        SettingsWindow::new(settings, Setting::WeatherIdle, None, &theme).render(area, &mut buf);
        let rows: Vec<String> = (0..area.height)
            .map(|y| (0..area.width).map(|x| buf[(x, y)].symbol()).collect())
            .collect();
        assert_eq!(rows[6], "│   Weather after idle       3 minutes │");
        assert_eq!(rows[7], "│   How long the app sits idle before …│");
        assert_eq!(buf[(4, 6)].bg, theme.selection);
        let mut buf = Buffer::empty(area);
        SettingsWindow::new(settings, Setting::Theme, None, &theme).render(area, &mut buf);
        assert_eq!(
            buf[(4, 2)].bg,
            theme.selection,
            "from the top while it fits"
        );
    }

    // FR-SET-01
    #[test]
    fn without_a_settings_file_the_foot_says_why() {
        let lines = drawn(Settings::default(), None, 80);
        assert!(
            lines.contains(
                &"        │ no settings file: no $XDG_CONFIG_HOME or home folder         │"
                    .to_owned()
            ),
            "{lines:#?}"
        );
    }
}
