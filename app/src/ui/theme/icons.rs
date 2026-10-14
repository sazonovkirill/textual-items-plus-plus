//! The icons the UI draws, each next to its plain Unicode marker (FR-UI-04,
//! [ui.md §7](../../../../specs/reference/taskpaper-terminal/specs/ui.md#7-icons-fr-ui-09-fr-ui-10)).
//! Hides which glyph stands for what, so the two sets cannot drift.

/// Something the UI marks with an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// An area.
    Area,
    /// A project.
    Project,
    /// A section.
    Section,
    /// The marker of a task that is not completed.
    Task,
    /// The marker of a completed task.
    DoneTask,
    /// The Inbox.
    Inbox,
    /// The Today view.
    Today,
    /// The Upcoming view.
    Upcoming,
    /// The All view.
    All,
    /// The Logbook view.
    Logbook,
    /// The Trash view.
    Trash,
    /// A start date chip.
    Start,
    /// A due date chip that is not overdue.
    Due,
    /// An overdue due date chip.
    Overdue,
    /// An item's notes.
    Notes,
    /// An item's comments, beside their count.
    Comments,
    /// A task's checklist, beside its progress.
    Checklist,
    /// The breadcrumbs' first crumb, the home container.
    Home,
    /// A Navigator entry whose children are shown.
    Expanded,
    /// A Navigator entry whose children are hidden.
    Collapsed,
    /// The status line's navigate mode.
    Navigate,
    /// The status line's edit mode.
    Edit,
    /// The status line's mark of changes not saved yet.
    Unsaved,
    /// A tag, in the tag picker and the tag filter.
    Tag,
    /// A command, in the command palette.
    Command,
}

/// The glyphs the UI draws for its icons.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IconSet {
    /// Nerd Font glyphs, the default.
    #[default]
    Nerd,
    /// Plain Unicode markers, for terminals without a Nerd Font.
    Plain,
}

impl Icon {
    /// The glyph drawn for the icon in `set`.
    #[must_use]
    pub fn glyph(self, set: IconSet) -> &'static str {
        let (nerd, plain) = self.glyphs();
        match set {
            IconSet::Nerd => nerd,
            IconSet::Plain => plain,
        }
    }

    /// The table: each Nerd Fonts v3 code point beside its plain marker, every one a single cell.
    const fn glyphs(self) -> (&'static str, &'static str) {
        match self {
            // nf-md-cube_outline
            Self::Area => ("\u{f01a7}", "◆"),
            // nf-md-folder_outline
            Self::Project => ("\u{f0256}", "○"),
            // nf-md-format_section
            Self::Section => ("\u{f069f}", "§"),
            // The task markers and the fold markers stay plain with Nerd Font icons too, as in
            // taskpaper-terminal (its D-59, D-60).
            Self::Task => ("-", "-"),
            Self::DoneTask => ("✔", "✔"),
            Self::Expanded => ("▾", "▾"),
            Self::Collapsed => ("▸", "▸"),
            // nf-md-inbox
            Self::Inbox => ("\u{f0687}", "⊔"),
            // nf-md-weather_sunny
            Self::Today => ("\u{f0599}", "☼"),
            // nf-md-calendar_arrow_right
            Self::Upcoming => ("\u{f1135}", "»"),
            // nf-md-format_list_bulleted
            Self::All => ("\u{f0279}", "∷"),
            // nf-md-book_check
            Self::Logbook => ("\u{f14f3}", "▤"),
            // nf-md-trash_can_outline
            Self::Trash => ("\u{f0a7a}", "✕"),
            // nf-md-calendar_start
            Self::Start => ("\u{f166d}", "↦"),
            // nf-md-calendar_end
            Self::Due => ("\u{f166c}", "⇥"),
            // nf-md-calendar_alert
            Self::Overdue => ("\u{f0a31}", "!"),
            // nf-md-note_text_outline
            Self::Notes => ("\u{f11d7}", "≡"),
            // nf-md-comment_text_outline
            Self::Comments => ("\u{f0189}", "❞"),
            // nf-md-format_list_checks
            Self::Checklist => ("\u{f0756}", "☑"),
            // nf-md-home
            Self::Home => ("\u{f02dc}", "⌂"),
            // nf-md-cursor_default_outline
            Self::Navigate => ("\u{f01bf}", "↕"),
            // nf-md-pencil
            Self::Edit => ("\u{f03eb}", "✎"),
            // nf-md-circle_medium
            Self::Unsaved => ("\u{f09de}", "●"),
            // nf-md-tag_outline
            Self::Tag => ("\u{f04fc}", "@"),
            // nf-md-console_line
            Self::Command => ("\u{f07b7}", "❯"),
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::text::Span;

    use super::*;

    const ICONS: [Icon; 25] = [
        Icon::Area,
        Icon::Project,
        Icon::Section,
        Icon::Task,
        Icon::DoneTask,
        Icon::Inbox,
        Icon::Today,
        Icon::Upcoming,
        Icon::All,
        Icon::Logbook,
        Icon::Trash,
        Icon::Start,
        Icon::Due,
        Icon::Overdue,
        Icon::Notes,
        Icon::Comments,
        Icon::Checklist,
        Icon::Home,
        Icon::Expanded,
        Icon::Collapsed,
        Icon::Navigate,
        Icon::Edit,
        Icon::Unsaved,
        Icon::Tag,
        Icon::Command,
    ];

    /// The task and fold markers, drawn the same with Nerd Font icons as without.
    const MARKERS: [Icon; 4] = [Icon::Task, Icon::DoneTask, Icon::Expanded, Icon::Collapsed];

    fn icons() -> impl Iterator<Item = Icon> {
        ICONS.into_iter().filter(|icon| !MARKERS.contains(icon))
    }

    // FR-UI-04, ui.md §7
    #[test]
    fn every_icon_is_one_cell_wide_in_both_sets() {
        for icon in ICONS {
            for set in [IconSet::Nerd, IconSet::Plain] {
                assert_eq!(Span::raw(icon.glyph(set)).width(), 1, "{icon:?} {set:?}");
            }
        }
    }

    // FR-UI-04
    #[test]
    fn nerd_font_icons_are_the_default() {
        assert_eq!(IconSet::default(), IconSet::Nerd);
    }

    // ui.md §7
    #[test]
    fn the_task_and_fold_markers_are_plain_with_nerd_font_icons_too() {
        for icon in MARKERS {
            assert_eq!(icon.glyph(IconSet::Nerd), icon.glyph(IconSet::Plain));
        }
        assert_eq!(Icon::Task.glyph(IconSet::Plain), "-");
        assert_eq!(Icon::DoneTask.glyph(IconSet::Plain), "✔");
    }

    // ui.md §7
    #[test]
    fn nerd_glyphs_come_from_the_nerd_fonts_v3_material_design_range() {
        for icon in icons() {
            let code = icon.glyph(IconSet::Nerd).chars().next().map(u32::from);
            assert!(
                code.is_some_and(|code| (0xF0001..=0xF1AF0).contains(&code)),
                "{icon:?}"
            );
        }
    }

    // FR-UI-04
    #[test]
    fn every_icon_has_its_own_glyph_in_both_sets() {
        for set in [IconSet::Nerd, IconSet::Plain] {
            let glyphs: Vec<&str> = ICONS.into_iter().map(|icon| icon.glyph(set)).collect();
            for (at, glyph) in glyphs.iter().enumerate() {
                assert!(
                    !glyphs[at + 1..].contains(glyph),
                    "{glyph} twice in {set:?}"
                );
            }
        }
    }
}
