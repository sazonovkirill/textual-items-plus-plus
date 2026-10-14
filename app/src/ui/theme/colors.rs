//! The color themes: the color of each part of the screen in each built-in theme
//! ([themes.md](../../../../specs/reference/themes.md), FR-UI-03). Hides the palettes.

use ratatui::style::Color;

use crate::{app::BuiltIn, domain::ItemKind};

/// The colors of a theme, one per role of themes.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The screen behind everything.
    pub background: Color,
    /// Item titles and body text.
    pub text: Color,
    /// Descriptions, secondary metadata, hints and help text.
    pub dim_text: Color,
    /// The app title, the active pane's border, key hints, and the titles of projects and the
    /// Inbox.
    pub primary: Color,
    /// Secondary emphasis: counts, search matches and the mode indicator.
    pub accent: Color,
    /// The background of selected rows.
    pub selection: Color,
    /// The glyph marking the focused row or pane.
    pub focus: Color,
    /// The background of the cursor row when it is not selected.
    pub current_line: Color,
    /// Text in no color of its own on the current-line band and on selected rows, so it reads
    /// on them whatever the terminal's own text color.
    pub band_text: Color,
    /// The titles of areas, date-group headings, checklist progress and the attachment icons.
    pub headers: Color,
    /// The titles of sections, a color neither areas nor projects take.
    pub sections: Color,
    /// Separators and box borders.
    pub rule: Color,
    /// Links, which are also underlined.
    pub link: Color,
    /// Start date chips.
    pub start: Color,
    /// Due date chips.
    pub due: Color,
    /// Overdue date chips and the overdue counter.
    pub overdue: Color,
    /// Errors in the status line.
    pub error: Color,
    /// Tags.
    pub tag: Color,
    /// Completed items and checked checklist items, faded on purpose.
    pub done: Color,
    /// The Inbox's icon in the sidebar and the Navigator: a blue.
    pub inbox_icon: Color,
    /// Today's icon there: a yellow.
    pub today_icon: Color,
    /// Upcoming's icon there: a red.
    pub upcoming_icon: Color,
    /// All's icon there: a green.
    pub all_icon: Color,
    /// The Logbook's icon there: a purple.
    pub logbook_icon: Color,
    /// The Trash's icon there: a grey.
    pub trash_icon: Color,
}

impl Theme {
    /// The color of the titles of `kind`: each kind of container has its own, which no other
    /// takes, and tasks the text color.
    #[must_use]
    pub fn kind(&self, kind: ItemKind) -> Color {
        match kind {
            ItemKind::Inbox | ItemKind::Project => self.primary,
            ItemKind::Area => self.headers,
            ItemKind::Section => self.sections,
            ItemKind::Task | ItemKind::ChecklistItem => self.text,
        }
    }
}

impl BuiltIn {
    /// The theme's colors.
    #[must_use]
    pub fn colors(self) -> Theme {
        match self {
            Self::TerminalDark => TERMINAL_DARK,
            Self::CatppuccinMocha => CATPPUCCIN_MOCHA,
            Self::TokyoNight => TOKYO_NIGHT,
            Self::GithubLight => GITHUB_LIGHT,
            Self::FlexokiLight => FLEXOKI_LIGHT,
        }
    }
}

/// Terminal Dark: the terminal's 16 ANSI colors, with four greys of the 256 colors where 16 have
/// none that fits: grey 248 for dim text, between the default foreground and dark gray; grey
/// 244 for completed items, fainter than that; grey 237 for the selection, a step brighter than
/// the black current-line band; and grey 255 for text on both, which the terminal's own text
/// color, dark on a light palette, would not be ([themes.md](../../../../specs/reference/themes.md#terminal-dark)).
const TERMINAL_DARK: Theme = Theme {
    background: Color::Reset,
    text: Color::Reset,
    dim_text: Color::Indexed(248),
    primary: Color::LightBlue,
    accent: Color::Yellow,
    selection: Color::Indexed(237),
    focus: Color::LightBlue,
    current_line: Color::Black,
    band_text: Color::Indexed(255),
    headers: Color::LightMagenta,
    sections: Color::Yellow,
    rule: Color::DarkGray,
    link: Color::Cyan,
    start: Color::Green,
    due: Color::Red,
    overdue: Color::LightRed,
    error: Color::Red,
    tag: Color::Cyan,
    done: Color::Indexed(244),
    inbox_icon: Color::LightBlue,
    today_icon: Color::Yellow,
    upcoming_icon: Color::Red,
    all_icon: Color::Green,
    logbook_icon: Color::Magenta,
    trash_icon: Color::Indexed(244),
};

/// Catppuccin Mocha, as themes.md lists it.
const CATPPUCCIN_MOCHA: Theme = Theme {
    background: Color::Rgb(0x1e, 0x1e, 0x2e),
    text: Color::Rgb(0xcd, 0xd6, 0xf4),
    dim_text: Color::Rgb(0xa6, 0xad, 0xc8),
    primary: Color::Rgb(0xcb, 0xa6, 0xf7),
    accent: Color::Rgb(0xfa, 0xb3, 0x87),
    selection: Color::Rgb(0x45, 0x47, 0x5a),
    focus: Color::Rgb(0xf5, 0xc2, 0xe7),
    current_line: Color::Rgb(0x31, 0x32, 0x44),
    band_text: Color::Rgb(0xcd, 0xd6, 0xf4),
    headers: Color::Rgb(0x89, 0xb4, 0xfa),
    sections: Color::Rgb(0xf9, 0xe2, 0xaf),
    rule: Color::Rgb(0x58, 0x5b, 0x70),
    link: Color::Rgb(0x74, 0xc7, 0xec),
    start: Color::Rgb(0xa6, 0xe3, 0xa1),
    due: Color::Rgb(0xeb, 0xa0, 0xac),
    overdue: Color::Rgb(0xf3, 0x8b, 0xa8),
    error: Color::Rgb(0xf3, 0x8b, 0xa8),
    tag: Color::Rgb(0x94, 0xe2, 0xd5),
    done: Color::Rgb(0x7f, 0x84, 0x9c),
    inbox_icon: Color::Rgb(0x89, 0xb4, 0xfa),
    today_icon: Color::Rgb(0xf9, 0xe2, 0xaf),
    upcoming_icon: Color::Rgb(0xf3, 0x8b, 0xa8),
    all_icon: Color::Rgb(0xa6, 0xe3, 0xa1),
    logbook_icon: Color::Rgb(0xcb, 0xa6, 0xf7),
    trash_icon: Color::Rgb(0x93, 0x99, 0xb2),
};

/// Tokyo Night (Night), as themes.md lists it.
const TOKYO_NIGHT: Theme = Theme {
    background: Color::Rgb(0x1a, 0x1b, 0x26),
    text: Color::Rgb(0xc0, 0xca, 0xf5),
    dim_text: Color::Rgb(0xa9, 0xb1, 0xd6),
    primary: Color::Rgb(0x7a, 0xa2, 0xf7),
    accent: Color::Rgb(0xff, 0x9e, 0x64),
    selection: Color::Rgb(0x28, 0x34, 0x57),
    focus: Color::Rgb(0xbb, 0x9a, 0xf7),
    current_line: Color::Rgb(0x29, 0x2e, 0x42),
    band_text: Color::Rgb(0xc0, 0xca, 0xf5),
    headers: Color::Rgb(0x7d, 0xcf, 0xff),
    sections: Color::Rgb(0xe0, 0xaf, 0x68),
    rule: Color::Rgb(0x3b, 0x42, 0x61),
    link: Color::Rgb(0x2a, 0xc3, 0xde),
    start: Color::Rgb(0x9e, 0xce, 0x6a),
    due: Color::Rgb(0xf7, 0x76, 0x8e),
    overdue: Color::Rgb(0xff, 0x00, 0x7c),
    error: Color::Rgb(0xf7, 0x76, 0x8e),
    tag: Color::Rgb(0x73, 0xda, 0xca),
    done: Color::Rgb(0x73, 0x7a, 0xa2),
    inbox_icon: Color::Rgb(0x7a, 0xa2, 0xf7),
    today_icon: Color::Rgb(0xe0, 0xaf, 0x68),
    upcoming_icon: Color::Rgb(0xf7, 0x76, 0x8e),
    all_icon: Color::Rgb(0x9e, 0xce, 0x6a),
    logbook_icon: Color::Rgb(0x9d, 0x7c, 0xd8),
    trash_icon: Color::Rgb(0x73, 0x7a, 0xa2),
};

/// GitHub Light Default, as themes.md lists it.
const GITHUB_LIGHT: Theme = Theme {
    background: Color::Rgb(0xff, 0xff, 0xff),
    text: Color::Rgb(0x1f, 0x23, 0x28),
    dim_text: Color::Rgb(0x65, 0x6d, 0x76),
    primary: Color::Rgb(0x09, 0x69, 0xda),
    accent: Color::Rgb(0x82, 0x50, 0xdf),
    selection: Color::Rgb(0xdd, 0xf4, 0xff),
    focus: Color::Rgb(0xbc, 0x4c, 0x00),
    current_line: Color::Rgb(0xf6, 0xf8, 0xfa),
    band_text: Color::Rgb(0x1f, 0x23, 0x28),
    headers: Color::Rgb(0x05, 0x50, 0xae),
    sections: Color::Rgb(0x9a, 0x67, 0x00),
    rule: Color::Rgb(0xd0, 0xd7, 0xde),
    link: Color::Rgb(0x09, 0x69, 0xda),
    start: Color::Rgb(0x1a, 0x7f, 0x37),
    due: Color::Rgb(0xcf, 0x22, 0x2e),
    overdue: Color::Rgb(0xa4, 0x0e, 0x26),
    error: Color::Rgb(0xd1, 0x24, 0x2f),
    tag: Color::Rgb(0x95, 0x38, 0x00),
    done: Color::Rgb(0x6e, 0x77, 0x81),
    inbox_icon: Color::Rgb(0x09, 0x69, 0xda),
    today_icon: Color::Rgb(0x9a, 0x67, 0x00),
    upcoming_icon: Color::Rgb(0xcf, 0x22, 0x2e),
    all_icon: Color::Rgb(0x1a, 0x7f, 0x37),
    logbook_icon: Color::Rgb(0x82, 0x50, 0xdf),
    trash_icon: Color::Rgb(0x6e, 0x77, 0x81),
};

/// Flexoki Light, as themes.md lists it.
const FLEXOKI_LIGHT: Theme = Theme {
    background: Color::Rgb(0xff, 0xfc, 0xf0),
    text: Color::Rgb(0x10, 0x0f, 0x0f),
    dim_text: Color::Rgb(0x6f, 0x6e, 0x69),
    primary: Color::Rgb(0x20, 0x5e, 0xa6),
    accent: Color::Rgb(0xbc, 0x52, 0x15),
    selection: Color::Rgb(0xda, 0xd8, 0xce),
    focus: Color::Rgb(0xa0, 0x2f, 0x6f),
    current_line: Color::Rgb(0xf2, 0xf0, 0xe5),
    band_text: Color::Rgb(0x10, 0x0f, 0x0f),
    headers: Color::Rgb(0x5e, 0x40, 0x9d),
    sections: Color::Rgb(0x8e, 0x6b, 0x01),
    rule: Color::Rgb(0xce, 0xcd, 0xc3),
    link: Color::Rgb(0x24, 0x83, 0x7b),
    start: Color::Rgb(0x66, 0x80, 0x0b),
    due: Color::Rgb(0xaf, 0x30, 0x29),
    overdue: Color::Rgb(0x94, 0x28, 0x22),
    error: Color::Rgb(0xaf, 0x30, 0x29),
    tag: Color::Rgb(0x24, 0x83, 0x7b),
    done: Color::Rgb(0x87, 0x85, 0x80),
    inbox_icon: Color::Rgb(0x20, 0x5e, 0xa6),
    today_icon: Color::Rgb(0x8e, 0x6b, 0x01),
    upcoming_icon: Color::Rgb(0xaf, 0x30, 0x29),
    all_icon: Color::Rgb(0x66, 0x80, 0x0b),
    logbook_icon: Color::Rgb(0x5e, 0x40, 0x9d),
    trash_icon: Color::Rgb(0x87, 0x85, 0x80),
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every color of `theme`, each with its role.
    fn roles(theme: &Theme) -> [(&'static str, Color); 25] {
        [
            ("background", theme.background),
            ("text", theme.text),
            ("dim_text", theme.dim_text),
            ("primary", theme.primary),
            ("accent", theme.accent),
            ("selection", theme.selection),
            ("focus", theme.focus),
            ("current_line", theme.current_line),
            ("band_text", theme.band_text),
            ("headers", theme.headers),
            ("sections", theme.sections),
            ("rule", theme.rule),
            ("link", theme.link),
            ("start", theme.start),
            ("due", theme.due),
            ("overdue", theme.overdue),
            ("error", theme.error),
            ("tag", theme.tag),
            ("done", theme.done),
            ("inbox_icon", theme.inbox_icon),
            ("today_icon", theme.today_icon),
            ("upcoming_icon", theme.upcoming_icon),
            ("all_icon", theme.all_icon),
            ("logbook_icon", theme.logbook_icon),
            ("trash_icon", theme.trash_icon),
        ]
    }

    /// The truecolor themes: all but Terminal Dark.
    fn truecolor() -> impl Iterator<Item = BuiltIn> {
        BuiltIn::ALL.into_iter().skip(1)
    }

    /// The red, green and blue of `color`, if it is an RGB color.
    fn channels(color: Color) -> Option<[u8; 3]> {
        match color {
            Color::Rgb(r, g, b) => Some([r, g, b]),
            _ => None,
        }
    }

    /// The WCAG 2.x contrast ratio between two RGB colors.
    fn contrast(a: Color, b: Color) -> f64 {
        let luminance = |color| {
            let [red, green, blue] =
                channels(color).expect("contrast is measured between RGB colors");
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.039_28 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
        };
        let (one, other) = (luminance(a), luminance(b));
        (one.max(other) + 0.05) / (one.min(other) + 0.05)
    }

    // FR-UI-03
    #[test]
    fn terminal_dark_draws_in_the_terminals_colors_and_four_greys() {
        let greys = [237, 244, 248, 255].map(Color::Indexed);
        for (role, color) in roles(&BuiltIn::TerminalDark.colors()) {
            let ansi = channels(color).is_none() && !matches!(color, Color::Indexed(_));
            assert!(ansi || greys.contains(&color), "{role}: {color:?}");
        }
        let dark = BuiltIn::TerminalDark.colors();
        assert_eq!(
            (dark.background, dark.text),
            (Color::Reset, Color::Reset),
            "the terminal's own background and text"
        );
        assert_ne!(dark.dim_text, dark.text);
        assert_ne!(dark.selection, dark.current_line);
        assert_ne!(dark.overdue, dark.due);
    }

    // FR-UI-02
    #[test]
    fn areas_projects_and_sections_each_have_a_color_of_their_own() {
        for theme in BuiltIn::ALL {
            let colors = theme.colors();
            let [area, project, section] = [ItemKind::Area, ItemKind::Project, ItemKind::Section]
                .map(|kind| colors.kind(kind));
            let id = theme.id();
            assert!(
                area != project && project != section && section != area,
                "{id}"
            );
            assert_eq!(colors.kind(ItemKind::Inbox), project);
            assert_eq!(colors.kind(ItemKind::Task), colors.text);
        }
    }

    // FR-UI-03, themes.md
    #[test]
    fn a_truecolor_theme_has_the_colors_themes_md_gives_it() {
        let rgb = |[r, g, b]: [u8; 3]| Color::Rgb(r, g, b);
        let mocha = BuiltIn::CatppuccinMocha.colors();
        assert_eq!(mocha.background, rgb([0x1e, 0x1e, 0x2e]));
        assert_eq!(mocha.selection, rgb([0x45, 0x47, 0x5a]));
        assert_eq!(mocha.done, rgb([0x7f, 0x84, 0x9c]));
        let tokyo = BuiltIn::TokyoNight.colors();
        assert_eq!(tokyo.selection, rgb([0x28, 0x34, 0x57]));
        assert_eq!(tokyo.overdue, rgb([0xff, 0x00, 0x7c]));
        let github = BuiltIn::GithubLight.colors();
        assert_eq!(github.background, rgb([0xff, 0xff, 0xff]));
        assert_eq!(github.error, rgb([0xd1, 0x24, 0x2f]));
        let flexoki = BuiltIn::FlexokiLight.colors();
        assert_eq!(flexoki.text, rgb([0x10, 0x0f, 0x0f]));
        assert_eq!(flexoki.tag, rgb([0x24, 0x83, 0x7b]));
        for theme in truecolor() {
            let (colors, id) = (theme.colors(), theme.id());
            assert_eq!(colors.band_text, colors.text, "{id}");
            for (role, color) in roles(&colors) {
                assert!(channels(color).is_some(), "{id} {role}: {color:?}");
            }
        }
    }

    #[test]
    fn contrast_is_measured_as_wcag_defines_it() {
        let (black, white) = (Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255));
        assert!((contrast(black, white) - 21.0).abs() < 1e-9);
        assert!((contrast(white, black) - 21.0).abs() < 1e-9);
        assert!((contrast(white, white) - 1.0).abs() < 1e-9);
    }

    // FR-UI-03, themes.md Contrast summary
    #[test]
    fn every_truecolor_theme_keeps_text_readable() {
        for theme in truecolor() {
            let colors = theme.colors();
            let id = theme.id();
            for (role, color) in [("text", colors.text), ("dim_text", colors.dim_text)] {
                let ratio = contrast(color, colors.background);
                assert!(ratio >= 4.5, "{id} {role}: {ratio:.2}");
            }
            for (band, fill) in [
                ("selection", colors.selection),
                ("current_line", colors.current_line),
            ] {
                let ratio = contrast(colors.band_text, fill);
                assert!(ratio >= 4.5, "{id} text on {band}: {ratio:.2}");
                assert_ne!(fill, colors.background, "{id} {band}");
            }
            let done = contrast(colors.done, colors.background);
            assert!(done >= 3.0, "{id} done: {done:.2}");
        }
    }

    // FR-VIEW-10, D-78
    #[test]
    fn each_view_icon_has_a_color_of_its_own_that_stands_out() {
        for theme in BuiltIn::ALL {
            let (colors, id) = (theme.colors(), theme.id());
            let icons = [
                colors.inbox_icon,
                colors.today_icon,
                colors.upcoming_icon,
                colors.all_icon,
                colors.logbook_icon,
                colors.trash_icon,
            ];
            for (at, color) in icons.iter().enumerate() {
                assert!(!icons[at + 1..].contains(color), "{id}: {color:?} twice");
                if channels(colors.background).is_some() {
                    let ratio = contrast(*color, colors.background);
                    assert!(ratio >= 3.0, "{id} {color:?}: {ratio:.2}");
                }
            }
        }
    }

    // FR-UI-03
    #[test]
    fn the_two_dark_and_two_light_themes_are_dark_and_light() {
        let white = Color::Rgb(255, 255, 255);
        let light: Vec<bool> = truecolor()
            .map(|theme| contrast(theme.colors().background, white) < 2.0)
            .collect();
        assert_eq!(light, [false, false, true, true]);
    }
}
