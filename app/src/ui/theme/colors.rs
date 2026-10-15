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
    /// The flag of a flagged item, and the Flagged view's icon in the sidebar and the Navigator:
    /// a red apart from Upcoming's (FR-ORG-12).
    pub flag: Color,
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

impl Theme {
    /// The theme in its grays alone, for grayscale mode (FR-UI-11): the roles that are shades of
    /// the theme, its background, text, dim text, bands, rules and completed items, keep their
    /// lightness without a hue, and every role with a hue of its own takes the text's gray, so
    /// titles, icons, chips and tags read as text and only weight and dimming set them apart.
    #[must_use]
    pub fn grayscale(self) -> Self {
        let text = gray(self.text, self.text);
        let shade = |color| gray(color, text);
        Self {
            background: shade(self.background),
            text,
            dim_text: shade(self.dim_text),
            primary: text,
            accent: text,
            selection: shade(self.selection),
            focus: text,
            current_line: shade(self.current_line),
            band_text: shade(self.band_text),
            headers: text,
            sections: text,
            rule: shade(self.rule),
            link: text,
            start: text,
            due: text,
            overdue: text,
            error: text,
            tag: text,
            done: shade(self.done),
            inbox_icon: text,
            today_icon: text,
            upcoming_icon: text,
            flag: text,
            all_icon: text,
            logbook_icon: text,
            trash_icon: shade(self.trash_icon),
        }
    }
}

/// `color` without its hue: an RGB color becomes the gray of the same WCAG luminance, so its
/// contrast with every other color keeps, and the terminal's own colors and its greys stay;
/// a color with a hue the terminal picks, whose lightness the app cannot know, becomes `text`.
fn gray(color: Color, text: Color) -> Color {
    match color {
        Color::Rgb(red, green, blue) => {
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.040_45 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            let luminance = 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue);
            let encoded = if luminance <= 0.003_130_8 {
                luminance * 12.92
            } else {
                1.055 * luminance.powf(1.0 / 2.4) - 0.055
            };
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a luminance between 0 and 1 encodes to between 0 and 255"
            )]
            let level = (encoded * 255.0).round() as u8;
            Color::Rgb(level, level, level)
        }
        Color::Reset
        | Color::Black
        | Color::DarkGray
        | Color::Gray
        | Color::White
        | Color::Indexed(0 | 7 | 8 | 15 | 232..=255) => color,
        _ => text,
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
            Self::RosePine => ROSE_PINE,
            Self::KanagawaWave => KANAGAWA_WAVE,
            Self::KanagawaDragon => KANAGAWA_DRAGON,
            Self::GruvboxDark => GRUVBOX_DARK,
            Self::EverforestDark => EVERFOREST_DARK,
            Self::GithubDark => GITHUB_DARK,
            Self::FlexokiDark => FLEXOKI_DARK,
            Self::MelangeDark => MELANGE_DARK,
            Self::Moonfly => MOONFLY,
            Self::Nightfly => NIGHTFLY,
            Self::Cyberdream => CYBERDREAM,
            Self::Synthwave84 => SYNTHWAVE_84,
            Self::GruvboxLight => GRUVBOX_LIGHT,
            Self::MelangeLight => MELANGE_LIGHT,
            Self::Dayfox => DAYFOX,
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
    flag: Color::LightRed,
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
    flag: Color::Rgb(0xeb, 0xa0, 0xac),
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
    flag: Color::Rgb(0xdb, 0x4b, 0x4b),
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
    flag: Color::Rgb(0xa4, 0x0e, 0x26),
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
    flag: Color::Rgb(0xd1, 0x4d, 0x41),
    all_icon: Color::Rgb(0x66, 0x80, 0x0b),
    logbook_icon: Color::Rgb(0x5e, 0x40, 0x9d),
    trash_icon: Color::Rgb(0x87, 0x85, 0x80),
};

/// Rosé Pine, as themes.md lists it.
const ROSE_PINE: Theme = Theme {
    background: Color::Rgb(0x19, 0x17, 0x24),
    text: Color::Rgb(0xe0, 0xde, 0xf4),
    dim_text: Color::Rgb(0x90, 0x8c, 0xaa),
    primary: Color::Rgb(0xc4, 0xa7, 0xe7),
    accent: Color::Rgb(0xeb, 0xbc, 0xba),
    selection: Color::Rgb(0x40, 0x3d, 0x52),
    focus: Color::Rgb(0x9c, 0xcf, 0xd8),
    current_line: Color::Rgb(0x26, 0x23, 0x3a),
    band_text: Color::Rgb(0xe0, 0xde, 0xf4),
    headers: Color::Rgb(0x9c, 0xcf, 0xd8),
    sections: Color::Rgb(0xf6, 0xc1, 0x77),
    rule: Color::Rgb(0x52, 0x4f, 0x67),
    link: Color::Rgb(0x9c, 0xcf, 0xd8),
    start: Color::Rgb(0x95, 0xb1, 0xac),
    due: Color::Rgb(0xeb, 0xbc, 0xba),
    overdue: Color::Rgb(0xeb, 0x6f, 0x92),
    error: Color::Rgb(0xeb, 0x6f, 0x92),
    tag: Color::Rgb(0x9c, 0xcf, 0xd8),
    done: Color::Rgb(0x6e, 0x6a, 0x86),
    inbox_icon: Color::Rgb(0x9c, 0xcf, 0xd8),
    today_icon: Color::Rgb(0xf6, 0xc1, 0x77),
    upcoming_icon: Color::Rgb(0xeb, 0x6f, 0x92),
    flag: Color::Rgb(0xeb, 0xbc, 0xba),
    all_icon: Color::Rgb(0x95, 0xb1, 0xac),
    logbook_icon: Color::Rgb(0xc4, 0xa7, 0xe7),
    trash_icon: Color::Rgb(0x6e, 0x6a, 0x86),
};

/// Kanagawa Wave, as themes.md lists it.
const KANAGAWA_WAVE: Theme = Theme {
    background: Color::Rgb(0x1f, 0x1f, 0x28),
    text: Color::Rgb(0xdc, 0xd7, 0xba),
    dim_text: Color::Rgb(0x93, 0x8a, 0xa9),
    primary: Color::Rgb(0x7e, 0x9c, 0xd8),
    accent: Color::Rgb(0xff, 0xa0, 0x66),
    selection: Color::Rgb(0x22, 0x32, 0x49),
    focus: Color::Rgb(0xd2, 0x7e, 0x99),
    current_line: Color::Rgb(0x36, 0x36, 0x46),
    band_text: Color::Rgb(0xdc, 0xd7, 0xba),
    headers: Color::Rgb(0x7f, 0xb4, 0xca),
    sections: Color::Rgb(0xe6, 0xc3, 0x84),
    rule: Color::Rgb(0x54, 0x54, 0x6d),
    link: Color::Rgb(0x7f, 0xb4, 0xca),
    start: Color::Rgb(0x98, 0xbb, 0x6c),
    due: Color::Rgb(0xe4, 0x68, 0x76),
    overdue: Color::Rgb(0xff, 0x5d, 0x62),
    error: Color::Rgb(0xff, 0x5d, 0x62),
    tag: Color::Rgb(0x7a, 0xa8, 0x9f),
    done: Color::Rgb(0x72, 0x71, 0x69),
    inbox_icon: Color::Rgb(0x7e, 0x9c, 0xd8),
    today_icon: Color::Rgb(0xe6, 0xc3, 0x84),
    upcoming_icon: Color::Rgb(0xe4, 0x68, 0x76),
    flag: Color::Rgb(0xff, 0x5d, 0x62),
    all_icon: Color::Rgb(0x98, 0xbb, 0x6c),
    logbook_icon: Color::Rgb(0x95, 0x7f, 0xb8),
    trash_icon: Color::Rgb(0x71, 0x7c, 0x7c),
};

/// Kanagawa Dragon, as themes.md lists it.
const KANAGAWA_DRAGON: Theme = Theme {
    background: Color::Rgb(0x18, 0x16, 0x16),
    text: Color::Rgb(0xc5, 0xc9, 0xc5),
    dim_text: Color::Rgb(0xa6, 0xa6, 0x9c),
    primary: Color::Rgb(0x8b, 0xa4, 0xb0),
    accent: Color::Rgb(0xb6, 0x92, 0x7b),
    selection: Color::Rgb(0x22, 0x32, 0x49),
    focus: Color::Rgb(0xa2, 0x92, 0xa3),
    current_line: Color::Rgb(0x39, 0x38, 0x36),
    band_text: Color::Rgb(0xc5, 0xc9, 0xc5),
    headers: Color::Rgb(0x94, 0x9f, 0xb5),
    sections: Color::Rgb(0xc4, 0xb2, 0x8a),
    rule: Color::Rgb(0x62, 0x5e, 0x5a),
    link: Color::Rgb(0x8e, 0xa4, 0xa2),
    start: Color::Rgb(0x87, 0xa9, 0x87),
    due: Color::Rgb(0xc4, 0x74, 0x6e),
    overdue: Color::Rgb(0xe8, 0x24, 0x24),
    error: Color::Rgb(0xe8, 0x24, 0x24),
    tag: Color::Rgb(0x8e, 0xa4, 0xa2),
    done: Color::Rgb(0x73, 0x7c, 0x73),
    inbox_icon: Color::Rgb(0x8b, 0xa4, 0xb0),
    today_icon: Color::Rgb(0xc4, 0xb2, 0x8a),
    upcoming_icon: Color::Rgb(0xc4, 0x74, 0x6e),
    flag: Color::Rgb(0xe8, 0x24, 0x24),
    all_icon: Color::Rgb(0x87, 0xa9, 0x87),
    logbook_icon: Color::Rgb(0x89, 0x92, 0xa7),
    trash_icon: Color::Rgb(0x7a, 0x83, 0x82),
};

/// Gruvbox Dark, as themes.md lists it.
const GRUVBOX_DARK: Theme = Theme {
    background: Color::Rgb(0x28, 0x28, 0x28),
    text: Color::Rgb(0xeb, 0xdb, 0xb2),
    dim_text: Color::Rgb(0xa8, 0x99, 0x84),
    primary: Color::Rgb(0x83, 0xa5, 0x98),
    accent: Color::Rgb(0xfe, 0x80, 0x19),
    selection: Color::Rgb(0x50, 0x49, 0x45),
    focus: Color::Rgb(0xd3, 0x86, 0x9b),
    current_line: Color::Rgb(0x3c, 0x38, 0x36),
    band_text: Color::Rgb(0xeb, 0xdb, 0xb2),
    headers: Color::Rgb(0x8e, 0xc0, 0x7c),
    sections: Color::Rgb(0xfa, 0xbd, 0x2f),
    rule: Color::Rgb(0x66, 0x5c, 0x54),
    link: Color::Rgb(0x83, 0xa5, 0x98),
    start: Color::Rgb(0xb8, 0xbb, 0x26),
    due: Color::Rgb(0xfb, 0x49, 0x34),
    overdue: Color::Rgb(0xfb, 0x49, 0x34),
    error: Color::Rgb(0xfb, 0x49, 0x34),
    tag: Color::Rgb(0x8e, 0xc0, 0x7c),
    done: Color::Rgb(0x92, 0x83, 0x74),
    inbox_icon: Color::Rgb(0x83, 0xa5, 0x98),
    today_icon: Color::Rgb(0xfa, 0xbd, 0x2f),
    upcoming_icon: Color::Rgb(0xfb, 0x49, 0x34),
    flag: Color::Rgb(0xfe, 0x80, 0x19),
    all_icon: Color::Rgb(0xb8, 0xbb, 0x26),
    logbook_icon: Color::Rgb(0xd3, 0x86, 0x9b),
    trash_icon: Color::Rgb(0x92, 0x83, 0x74),
};

/// Everforest Dark, as themes.md lists it.
const EVERFOREST_DARK: Theme = Theme {
    background: Color::Rgb(0x2d, 0x35, 0x3b),
    text: Color::Rgb(0xd3, 0xc6, 0xaa),
    dim_text: Color::Rgb(0x9d, 0xa9, 0xa0),
    primary: Color::Rgb(0xa7, 0xc0, 0x80),
    accent: Color::Rgb(0xe6, 0x98, 0x75),
    selection: Color::Rgb(0x54, 0x3a, 0x48),
    focus: Color::Rgb(0xd6, 0x99, 0xb6),
    current_line: Color::Rgb(0x34, 0x3f, 0x44),
    band_text: Color::Rgb(0xd3, 0xc6, 0xaa),
    headers: Color::Rgb(0x7f, 0xbb, 0xb3),
    sections: Color::Rgb(0xdb, 0xbc, 0x7f),
    rule: Color::Rgb(0x56, 0x63, 0x5f),
    link: Color::Rgb(0x7f, 0xbb, 0xb3),
    start: Color::Rgb(0xa7, 0xc0, 0x80),
    due: Color::Rgb(0xe6, 0x7e, 0x80),
    overdue: Color::Rgb(0xe6, 0x7e, 0x80),
    error: Color::Rgb(0xe6, 0x7e, 0x80),
    tag: Color::Rgb(0x83, 0xc0, 0x92),
    done: Color::Rgb(0x7a, 0x84, 0x78),
    inbox_icon: Color::Rgb(0x7f, 0xbb, 0xb3),
    today_icon: Color::Rgb(0xdb, 0xbc, 0x7f),
    upcoming_icon: Color::Rgb(0xe6, 0x7e, 0x80),
    flag: Color::Rgb(0xe6, 0x98, 0x75),
    all_icon: Color::Rgb(0xa7, 0xc0, 0x80),
    logbook_icon: Color::Rgb(0xd6, 0x99, 0xb6),
    trash_icon: Color::Rgb(0x85, 0x92, 0x89),
};

/// GitHub Dark Default, as themes.md lists it.
const GITHUB_DARK: Theme = Theme {
    background: Color::Rgb(0x0d, 0x11, 0x17),
    text: Color::Rgb(0xe6, 0xed, 0xf3),
    dim_text: Color::Rgb(0x84, 0x8d, 0x97),
    primary: Color::Rgb(0x58, 0xa6, 0xff),
    accent: Color::Rgb(0xa3, 0x71, 0xf7),
    selection: Color::Rgb(0x0c, 0x2d, 0x6b),
    focus: Color::Rgb(0xdb, 0x6d, 0x28),
    current_line: Color::Rgb(0x16, 0x1b, 0x22),
    band_text: Color::Rgb(0xe6, 0xed, 0xf3),
    headers: Color::Rgb(0x79, 0xc0, 0xff),
    sections: Color::Rgb(0xd2, 0x99, 0x22),
    rule: Color::Rgb(0x30, 0x36, 0x3d),
    link: Color::Rgb(0x2f, 0x81, 0xf7),
    start: Color::Rgb(0x3f, 0xb9, 0x50),
    due: Color::Rgb(0xff, 0x7b, 0x72),
    overdue: Color::Rgb(0xf8, 0x51, 0x49),
    error: Color::Rgb(0xf8, 0x51, 0x49),
    tag: Color::Rgb(0xff, 0xa6, 0x57),
    done: Color::Rgb(0x6e, 0x76, 0x81),
    inbox_icon: Color::Rgb(0x58, 0xa6, 0xff),
    today_icon: Color::Rgb(0xd2, 0x99, 0x22),
    upcoming_icon: Color::Rgb(0xff, 0x7b, 0x72),
    flag: Color::Rgb(0xf8, 0x51, 0x49),
    all_icon: Color::Rgb(0x3f, 0xb9, 0x50),
    logbook_icon: Color::Rgb(0xa3, 0x71, 0xf7),
    trash_icon: Color::Rgb(0x6e, 0x76, 0x81),
};

/// Flexoki Dark, as themes.md lists it.
const FLEXOKI_DARK: Theme = Theme {
    background: Color::Rgb(0x10, 0x0f, 0x0f),
    text: Color::Rgb(0xce, 0xcd, 0xc3),
    dim_text: Color::Rgb(0x87, 0x85, 0x80),
    primary: Color::Rgb(0x43, 0x85, 0xbe),
    accent: Color::Rgb(0xda, 0x70, 0x2c),
    selection: Color::Rgb(0x34, 0x33, 0x31),
    focus: Color::Rgb(0xce, 0x5d, 0x97),
    current_line: Color::Rgb(0x1c, 0x1b, 0x1a),
    band_text: Color::Rgb(0xce, 0xcd, 0xc3),
    headers: Color::Rgb(0x8b, 0x7e, 0xc8),
    sections: Color::Rgb(0xd0, 0xa2, 0x15),
    rule: Color::Rgb(0x40, 0x3e, 0x3c),
    link: Color::Rgb(0x3a, 0xa9, 0x9f),
    start: Color::Rgb(0x87, 0x9a, 0x39),
    due: Color::Rgb(0xd1, 0x4d, 0x41),
    overdue: Color::Rgb(0xe8, 0x70, 0x5f),
    error: Color::Rgb(0xd1, 0x4d, 0x41),
    tag: Color::Rgb(0x3a, 0xa9, 0x9f),
    done: Color::Rgb(0x6f, 0x6e, 0x69),
    inbox_icon: Color::Rgb(0x43, 0x85, 0xbe),
    today_icon: Color::Rgb(0xd0, 0xa2, 0x15),
    upcoming_icon: Color::Rgb(0xd1, 0x4d, 0x41),
    flag: Color::Rgb(0xe8, 0x70, 0x5f),
    all_icon: Color::Rgb(0x87, 0x9a, 0x39),
    logbook_icon: Color::Rgb(0x8b, 0x7e, 0xc8),
    trash_icon: Color::Rgb(0x6f, 0x6e, 0x69),
};

/// Melange Dark, as themes.md lists it.
const MELANGE_DARK: Theme = Theme {
    background: Color::Rgb(0x29, 0x25, 0x22),
    text: Color::Rgb(0xec, 0xe1, 0xd7),
    dim_text: Color::Rgb(0xc1, 0xa7, 0x8e),
    primary: Color::Rgb(0xa3, 0xa9, 0xce),
    accent: Color::Rgb(0xe4, 0x9b, 0x5d),
    selection: Color::Rgb(0x40, 0x3a, 0x36),
    focus: Color::Rgb(0xcf, 0x9b, 0xc2),
    current_line: Color::Rgb(0x34, 0x30, 0x2c),
    band_text: Color::Rgb(0xec, 0xe1, 0xd7),
    headers: Color::Rgb(0x89, 0xb3, 0xb6),
    sections: Color::Rgb(0xeb, 0xc0, 0x6d),
    rule: Color::Rgb(0x86, 0x74, 0x62),
    link: Color::Rgb(0x89, 0xb3, 0xb6),
    start: Color::Rgb(0x85, 0xb6, 0x95),
    due: Color::Rgb(0xd4, 0x77, 0x66),
    overdue: Color::Rgb(0xbd, 0x81, 0x83),
    error: Color::Rgb(0xd4, 0x77, 0x66),
    tag: Color::Rgb(0x89, 0xb3, 0xb6),
    done: Color::Rgb(0x86, 0x74, 0x62),
    inbox_icon: Color::Rgb(0xa3, 0xa9, 0xce),
    today_icon: Color::Rgb(0xeb, 0xc0, 0x6d),
    upcoming_icon: Color::Rgb(0xd4, 0x77, 0x66),
    flag: Color::Rgb(0xbd, 0x81, 0x83),
    all_icon: Color::Rgb(0x85, 0xb6, 0x95),
    logbook_icon: Color::Rgb(0xcf, 0x9b, 0xc2),
    trash_icon: Color::Rgb(0x86, 0x74, 0x62),
};

/// Moonfly, as themes.md lists it.
const MOONFLY: Theme = Theme {
    background: Color::Rgb(0x08, 0x08, 0x08),
    text: Color::Rgb(0xc6, 0xc6, 0xc6),
    dim_text: Color::Rgb(0x94, 0x94, 0x94),
    primary: Color::Rgb(0x80, 0xa0, 0xff),
    accent: Color::Rgb(0xde, 0x93, 0x5f),
    selection: Color::Rgb(0x32, 0x34, 0x37),
    focus: Color::Rgb(0xae, 0x81, 0xff),
    current_line: Color::Rgb(0x1c, 0x1c, 0x1c),
    band_text: Color::Rgb(0xc6, 0xc6, 0xc6),
    headers: Color::Rgb(0x74, 0xb2, 0xff),
    sections: Color::Rgb(0xe3, 0xc7, 0x8a),
    rule: Color::Rgb(0x44, 0x44, 0x44),
    link: Color::Rgb(0x79, 0xda, 0xc8),
    start: Color::Rgb(0x8c, 0xc8, 0x5f),
    due: Color::Rgb(0xff, 0x5d, 0x5d),
    overdue: Color::Rgb(0xff, 0x51, 0x89),
    error: Color::Rgb(0xff, 0x5d, 0x5d),
    tag: Color::Rgb(0x36, 0xc6, 0x92),
    done: Color::Rgb(0x62, 0x62, 0x62),
    inbox_icon: Color::Rgb(0x80, 0xa0, 0xff),
    today_icon: Color::Rgb(0xe3, 0xc7, 0x8a),
    upcoming_icon: Color::Rgb(0xff, 0x5d, 0x5d),
    flag: Color::Rgb(0xff, 0x51, 0x89),
    all_icon: Color::Rgb(0x8c, 0xc8, 0x5f),
    logbook_icon: Color::Rgb(0xae, 0x81, 0xff),
    trash_icon: Color::Rgb(0x80, 0x80, 0x80),
};

/// Nightfly, as themes.md lists it.
const NIGHTFLY: Theme = Theme {
    background: Color::Rgb(0x01, 0x16, 0x27),
    text: Color::Rgb(0xc3, 0xcc, 0xdc),
    dim_text: Color::Rgb(0xa1, 0xaa, 0xb8),
    primary: Color::Rgb(0x82, 0xaa, 0xff),
    accent: Color::Rgb(0xf7, 0x8c, 0x6c),
    selection: Color::Rgb(0x1d, 0x3b, 0x53),
    focus: Color::Rgb(0xc7, 0x92, 0xea),
    current_line: Color::Rgb(0x09, 0x22, 0x36),
    band_text: Color::Rgb(0xc3, 0xcc, 0xdc),
    headers: Color::Rgb(0x87, 0xbc, 0xff),
    sections: Color::Rgb(0xe3, 0xd1, 0x8a),
    rule: Color::Rgb(0x2c, 0x30, 0x43),
    link: Color::Rgb(0x7f, 0xdb, 0xca),
    start: Color::Rgb(0xa1, 0xcd, 0x5e),
    due: Color::Rgb(0xfc, 0x51, 0x4e),
    overdue: Color::Rgb(0xff, 0x58, 0x74),
    error: Color::Rgb(0xfc, 0x51, 0x4e),
    tag: Color::Rgb(0x21, 0xc7, 0xa8),
    done: Color::Rgb(0x7c, 0x8f, 0x8f),
    inbox_icon: Color::Rgb(0x82, 0xaa, 0xff),
    today_icon: Color::Rgb(0xe3, 0xd1, 0x8a),
    upcoming_icon: Color::Rgb(0xfc, 0x51, 0x4e),
    flag: Color::Rgb(0xff, 0x58, 0x74),
    all_icon: Color::Rgb(0xa1, 0xcd, 0x5e),
    logbook_icon: Color::Rgb(0xae, 0x81, 0xff),
    trash_icon: Color::Rgb(0x76, 0x87, 0x99),
};

/// Cyberdream, as themes.md lists it.
const CYBERDREAM: Theme = Theme {
    background: Color::Rgb(0x16, 0x18, 0x1a),
    text: Color::Rgb(0xff, 0xff, 0xff),
    dim_text: Color::Rgb(0x7b, 0x84, 0x96),
    primary: Color::Rgb(0x5e, 0xa1, 0xff),
    accent: Color::Rgb(0xff, 0xbd, 0x5e),
    selection: Color::Rgb(0x3c, 0x40, 0x48),
    focus: Color::Rgb(0xff, 0x5e, 0xf1),
    current_line: Color::Rgb(0x1e, 0x21, 0x24),
    band_text: Color::Rgb(0xff, 0xff, 0xff),
    headers: Color::Rgb(0x5e, 0xf1, 0xff),
    sections: Color::Rgb(0xf1, 0xff, 0x5e),
    rule: Color::Rgb(0x3c, 0x40, 0x48),
    link: Color::Rgb(0x5e, 0xf1, 0xff),
    start: Color::Rgb(0x5e, 0xff, 0x6c),
    due: Color::Rgb(0xff, 0x6e, 0x5e),
    overdue: Color::Rgb(0xff, 0x5e, 0xa0),
    error: Color::Rgb(0xff, 0x6e, 0x5e),
    tag: Color::Rgb(0x5e, 0xf1, 0xff),
    done: Color::Rgb(0x7b, 0x84, 0x96),
    inbox_icon: Color::Rgb(0x5e, 0xa1, 0xff),
    today_icon: Color::Rgb(0xf1, 0xff, 0x5e),
    upcoming_icon: Color::Rgb(0xff, 0x6e, 0x5e),
    flag: Color::Rgb(0xff, 0x5e, 0xa0),
    all_icon: Color::Rgb(0x5e, 0xff, 0x6c),
    logbook_icon: Color::Rgb(0xbd, 0x5e, 0xff),
    trash_icon: Color::Rgb(0x7b, 0x84, 0x96),
};

/// Synthwave '84, as themes.md lists it.
const SYNTHWAVE_84: Theme = Theme {
    background: Color::Rgb(0x26, 0x23, 0x35),
    text: Color::Rgb(0xff, 0xff, 0xff),
    dim_text: Color::Rgb(0x84, 0x8b, 0xbd),
    primary: Color::Rgb(0x36, 0xf9, 0xf6),
    accent: Color::Rgb(0xf9, 0x7e, 0x72),
    selection: Color::Rgb(0x46, 0x34, 0x65),
    focus: Color::Rgb(0xff, 0x7e, 0xdb),
    current_line: Color::Rgb(0x34, 0x29, 0x4f),
    band_text: Color::Rgb(0xff, 0xff, 0xff),
    headers: Color::Rgb(0xff, 0x7e, 0xdb),
    sections: Color::Rgb(0xfe, 0xde, 0x5d),
    rule: Color::Rgb(0x49, 0x54, 0x95),
    link: Color::Rgb(0x2e, 0xe2, 0xfa),
    start: Color::Rgb(0x72, 0xf1, 0xb8),
    due: Color::Rgb(0xfe, 0x44, 0x50),
    overdue: Color::Rgb(0xfe, 0x44, 0x50),
    error: Color::Rgb(0xfe, 0x44, 0x50),
    tag: Color::Rgb(0x72, 0xf1, 0xb8),
    done: Color::Rgb(0x84, 0x8b, 0xbd),
    inbox_icon: Color::Rgb(0x36, 0xf9, 0xf6),
    today_icon: Color::Rgb(0xfe, 0xde, 0x5d),
    upcoming_icon: Color::Rgb(0xfe, 0x44, 0x50),
    flag: Color::Rgb(0xf9, 0x7e, 0x72),
    all_icon: Color::Rgb(0x72, 0xf1, 0xb8),
    logbook_icon: Color::Rgb(0xb8, 0x93, 0xce),
    trash_icon: Color::Rgb(0x84, 0x8b, 0xbd),
};

/// Gruvbox Light, as themes.md lists it.
const GRUVBOX_LIGHT: Theme = Theme {
    background: Color::Rgb(0xfb, 0xf1, 0xc7),
    text: Color::Rgb(0x3c, 0x38, 0x36),
    dim_text: Color::Rgb(0x66, 0x5c, 0x54),
    primary: Color::Rgb(0x07, 0x66, 0x78),
    accent: Color::Rgb(0xaf, 0x3a, 0x03),
    selection: Color::Rgb(0xd5, 0xc4, 0xa1),
    focus: Color::Rgb(0x8f, 0x3f, 0x71),
    current_line: Color::Rgb(0xeb, 0xdb, 0xb2),
    band_text: Color::Rgb(0x3c, 0x38, 0x36),
    headers: Color::Rgb(0x42, 0x7b, 0x58),
    sections: Color::Rgb(0xb5, 0x76, 0x14),
    rule: Color::Rgb(0xbd, 0xae, 0x93),
    link: Color::Rgb(0x07, 0x66, 0x78),
    start: Color::Rgb(0x79, 0x74, 0x0e),
    due: Color::Rgb(0x9d, 0x00, 0x06),
    overdue: Color::Rgb(0x9d, 0x00, 0x06),
    error: Color::Rgb(0x9d, 0x00, 0x06),
    tag: Color::Rgb(0x42, 0x7b, 0x58),
    done: Color::Rgb(0x92, 0x83, 0x74),
    inbox_icon: Color::Rgb(0x07, 0x66, 0x78),
    today_icon: Color::Rgb(0xb5, 0x76, 0x14),
    upcoming_icon: Color::Rgb(0x9d, 0x00, 0x06),
    flag: Color::Rgb(0xcc, 0x24, 0x1d),
    all_icon: Color::Rgb(0x79, 0x74, 0x0e),
    logbook_icon: Color::Rgb(0x8f, 0x3f, 0x71),
    trash_icon: Color::Rgb(0x92, 0x83, 0x74),
};

/// Melange Light, as themes.md lists it.
const MELANGE_LIGHT: Theme = Theme {
    background: Color::Rgb(0xf1, 0xf1, 0xf1),
    text: Color::Rgb(0x54, 0x43, 0x3a),
    dim_text: Color::Rgb(0x7d, 0x66, 0x58),
    primary: Color::Rgb(0x46, 0x5a, 0xa4),
    accent: Color::Rgb(0xbc, 0x5c, 0x00),
    selection: Color::Rgb(0xd9, 0xd3, 0xce),
    focus: Color::Rgb(0x90, 0x41, 0x80),
    current_line: Color::Rgb(0xe9, 0xe1, 0xdb),
    band_text: Color::Rgb(0x54, 0x43, 0x3a),
    headers: Color::Rgb(0x3d, 0x65, 0x68),
    sections: Color::Rgb(0xa0, 0x6d, 0x00),
    rule: Color::Rgb(0xa9, 0x8a, 0x78),
    link: Color::Rgb(0x3d, 0x65, 0x68),
    start: Color::Rgb(0x3a, 0x68, 0x4a),
    due: Color::Rgb(0xbf, 0x00, 0x21),
    overdue: Color::Rgb(0xbf, 0x00, 0x21),
    error: Color::Rgb(0xbf, 0x00, 0x21),
    tag: Color::Rgb(0x3d, 0x65, 0x68),
    done: Color::Rgb(0x7d, 0x66, 0x58),
    inbox_icon: Color::Rgb(0x46, 0x5a, 0xa4),
    today_icon: Color::Rgb(0xa0, 0x6d, 0x00),
    upcoming_icon: Color::Rgb(0xbf, 0x00, 0x21),
    flag: Color::Rgb(0xbc, 0x5c, 0x00),
    all_icon: Color::Rgb(0x3a, 0x68, 0x4a),
    logbook_icon: Color::Rgb(0x90, 0x41, 0x80),
    trash_icon: Color::Rgb(0x7d, 0x66, 0x58),
};

/// Dayfox, as themes.md lists it.
const DAYFOX: Theme = Theme {
    background: Color::Rgb(0xf6, 0xf2, 0xee),
    text: Color::Rgb(0x3d, 0x2b, 0x5a),
    dim_text: Color::Rgb(0x64, 0x3f, 0x61),
    primary: Color::Rgb(0x28, 0x48, 0xa9),
    accent: Color::Rgb(0x95, 0x5f, 0x61),
    selection: Color::Rgb(0xe7, 0xd2, 0xbe),
    focus: Color::Rgb(0xa4, 0x40, 0xb5),
    current_line: Color::Rgb(0xe4, 0xdc, 0xd4),
    band_text: Color::Rgb(0x3d, 0x2b, 0x5a),
    headers: Color::Rgb(0x6e, 0x33, 0xce),
    sections: Color::Rgb(0xac, 0x54, 0x02),
    rule: Color::Rgb(0xaa, 0xb0, 0xad),
    link: Color::Rgb(0x28, 0x79, 0x80),
    start: Color::Rgb(0x39, 0x68, 0x47),
    due: Color::Rgb(0xa5, 0x22, 0x2f),
    overdue: Color::Rgb(0xa5, 0x22, 0x2f),
    error: Color::Rgb(0xa5, 0x22, 0x2f),
    tag: Color::Rgb(0x28, 0x79, 0x80),
    done: Color::Rgb(0x83, 0x7a, 0x72),
    inbox_icon: Color::Rgb(0x28, 0x48, 0xa9),
    today_icon: Color::Rgb(0xac, 0x54, 0x02),
    upcoming_icon: Color::Rgb(0xa5, 0x22, 0x2f),
    flag: Color::Rgb(0x95, 0x5f, 0x61),
    all_icon: Color::Rgb(0x39, 0x68, 0x47),
    logbook_icon: Color::Rgb(0x6e, 0x33, 0xce),
    trash_icon: Color::Rgb(0x83, 0x7a, 0x72),
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every color of `theme`, each with its role.
    fn roles(theme: &Theme) -> [(&'static str, Color); 26] {
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
            ("flag", theme.flag),
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
        for (theme, background, today) in [
            (BuiltIn::RosePine, [0x19, 0x17, 0x24], [0xf6, 0xc1, 0x77]),
            (
                BuiltIn::KanagawaWave,
                [0x1f, 0x1f, 0x28],
                [0xe6, 0xc3, 0x84],
            ),
            (
                BuiltIn::KanagawaDragon,
                [0x18, 0x16, 0x16],
                [0xc4, 0xb2, 0x8a],
            ),
            (BuiltIn::GruvboxDark, [0x28, 0x28, 0x28], [0xfa, 0xbd, 0x2f]),
            (
                BuiltIn::EverforestDark,
                [0x2d, 0x35, 0x3b],
                [0xdb, 0xbc, 0x7f],
            ),
            (BuiltIn::GithubDark, [0x0d, 0x11, 0x17], [0xd2, 0x99, 0x22]),
            (BuiltIn::FlexokiDark, [0x10, 0x0f, 0x0f], [0xd0, 0xa2, 0x15]),
            (BuiltIn::MelangeDark, [0x29, 0x25, 0x22], [0xeb, 0xc0, 0x6d]),
            (BuiltIn::Moonfly, [0x08, 0x08, 0x08], [0xe3, 0xc7, 0x8a]),
            (BuiltIn::Nightfly, [0x01, 0x16, 0x27], [0xe3, 0xd1, 0x8a]),
            (BuiltIn::Cyberdream, [0x16, 0x18, 0x1a], [0xf1, 0xff, 0x5e]),
            (BuiltIn::Synthwave84, [0x26, 0x23, 0x35], [0xfe, 0xde, 0x5d]),
            (
                BuiltIn::GruvboxLight,
                [0xfb, 0xf1, 0xc7],
                [0xb5, 0x76, 0x14],
            ),
            (
                BuiltIn::MelangeLight,
                [0xf1, 0xf1, 0xf1],
                [0xa0, 0x6d, 0x00],
            ),
            (BuiltIn::Dayfox, [0xf6, 0xf2, 0xee], [0xac, 0x54, 0x02]),
        ] {
            let (colors, id) = (theme.colors(), theme.id());
            assert_eq!(
                (colors.background, colors.today_icon),
                (rgb(background), rgb(today)),
                "{id}"
            );
        }
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
                colors.flag,
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

    /// Whether `color` has no hue: a grey of RGB or of the 256 colors, or one of the terminal's
    /// own colors that is not a hue.
    fn achromatic(color: Color) -> bool {
        match color {
            Color::Rgb(red, green, blue) => red == green && green == blue,
            Color::Reset | Color::Black | Color::DarkGray | Color::Gray | Color::White => true,
            Color::Indexed(index) => matches!(index, 0 | 7 | 8 | 15 | 232..=255),
            _ => false,
        }
    }

    // FR-UI-11
    #[test]
    fn in_grayscale_every_theme_draws_in_greys_with_its_hues_as_text() {
        assert!(!achromatic(Color::Blue) && !achromatic(Color::Indexed(75)));
        for theme in BuiltIn::ALL {
            let (colors, id) = (theme.colors().grayscale(), theme.id());
            for (role, color) in roles(&colors) {
                assert!(achromatic(color), "{id} {role}: {color:?}");
            }
            for color in [colors.primary, colors.headers, colors.tag, colors.flag] {
                assert_eq!(color, colors.text, "{id}");
            }
            assert_eq!(colors.kind(ItemKind::Area), colors.text, "{id}");
            assert_ne!(colors.dim_text, colors.text, "{id}");
            assert_ne!(colors.selection, colors.current_line, "{id}");
        }
        let dark = BuiltIn::TerminalDark.colors();
        assert_eq!(
            dark.grayscale(),
            Theme {
                primary: Color::Reset,
                accent: Color::Reset,
                focus: Color::Reset,
                headers: Color::Reset,
                sections: Color::Reset,
                link: Color::Reset,
                start: Color::Reset,
                due: Color::Reset,
                overdue: Color::Reset,
                error: Color::Reset,
                tag: Color::Reset,
                inbox_icon: Color::Reset,
                today_icon: Color::Reset,
                upcoming_icon: Color::Reset,
                flag: Color::Reset,
                all_icon: Color::Reset,
                logbook_icon: Color::Reset,
                ..dark
            },
            "the terminal's own text and its greys"
        );
    }

    // FR-UI-11
    #[test]
    fn in_grayscale_a_shade_keeps_its_contrast() {
        for theme in truecolor() {
            let (colors, grey, id) = (theme.colors(), theme.colors().grayscale(), theme.id());
            for (role, color, shade) in [
                ("text", colors.text, grey.text),
                ("dim_text", colors.dim_text, grey.dim_text),
                ("done", colors.done, grey.done),
                ("selection", colors.selection, grey.selection),
            ] {
                let (was, now) = (
                    contrast(color, colors.background),
                    contrast(shade, grey.background),
                );
                let off = (was - now).abs() / was;
                assert!(
                    off < 0.03,
                    "{id} {role}: {was:.2} {now:.2}, off by rounding alone"
                );
            }
        }
        assert_eq!(gray(Color::Rgb(0, 0, 0), Color::Reset), Color::Rgb(0, 0, 0));
        assert_eq!(
            gray(Color::Rgb(255, 0, 0), Color::Reset),
            Color::Rgb(127, 127, 127)
        );
        assert_eq!(gray(Color::Indexed(244), Color::Reset), Color::Indexed(244));
        assert_eq!(
            gray(Color::Red, Color::Reset),
            Color::Reset,
            "a hue the terminal picks"
        );
        assert_eq!(gray(Color::Indexed(75), Color::Reset), Color::Reset);
    }

    // FR-UI-03, D-125
    #[test]
    fn the_light_themes_are_light_and_the_others_dark() {
        let white = Color::Rgb(255, 255, 255);
        let light: Vec<_> = truecolor()
            .filter(|theme| contrast(theme.colors().background, white) < 2.0)
            .collect();
        assert_eq!(
            light,
            [
                BuiltIn::GithubLight,
                BuiltIn::FlexokiLight,
                BuiltIn::GruvboxLight,
                BuiltIn::MelangeLight,
                BuiltIn::Dayfox,
            ]
        );
    }
}
