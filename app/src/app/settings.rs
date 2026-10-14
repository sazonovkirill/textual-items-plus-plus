//! The settings: which built-in theme the app draws in and which icons it draws (FR-SET-03,
//! FR-UI-03, FR-UI-04). Names each choice; how it looks is the frontend's.

/// One of the built-in themes (FR-UI-03).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

/// The glyphs the frontend draws for its icons (FR-UI-04).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IconSet {
    /// Nerd Font glyphs, the default.
    #[default]
    Nerd,
    /// Plain Unicode markers, for terminals without a Nerd Font.
    Plain,
}

#[cfg(test)]
mod tests {
    use super::*;

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
