//! The settings window and the settings file: opening the window with `g,`, changing each setting
//! with its live preview, keeping changes in `config.toml`, and starting with what the file sets.

use std::fs;

use crossterm::event::{Event, KeyCode};
use ratatui::style::Color;

use crate::support::{Harness, shows_path};

/// The window's rows on an 80-column screen, trimmed, without its border.
fn window(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .filter_map(|row| {
            let inner = row.split('│').nth(1)?;
            Some(inner.trim_end().to_owned())
        })
        .collect()
}

// FR-SET-02, FR-SET-03
#[test]
fn g_comma_opens_the_settings_window_and_esc_closes_it() {
    // Tall enough that the window leaves a task in view.
    let mut harness = Harness::new(80, 30);
    harness.type_text("g,");
    let lines = window(&harness);
    assert_eq!(
        lines.get(..7).unwrap_or_default(),
        [
            " Appearance",
            "   Theme                                        Terminal Dark",
            "   The colors the app draws in",
            "   Grayscale                                              Off",
            "   The theme in its grays alone: no color in text or icons",
            "   Icons                                            Nerd Font",
            "   Nerd Font icons, or plain markers without the font",
        ]
    );
    let foot = lines.last().map(|line| line.trim()).unwrap_or_default();
    assert!(shows_path(foot, &harness.config()), "{foot}");
    assert!(harness.screen().contains("┌ Settings "));
    harness.type_text("x");
    assert!(
        harness.screen().contains("☐ Call the bank"),
        "x completes nothing"
    );
    harness.key(KeyCode::Esc);
    assert!(!harness.screen().contains("Settings"));
    harness.type_text("x");
    assert!(!harness.screen().contains("☐ Call the bank"));
    harness.type_text("g,");
    harness.ctrl('g');
    assert!(
        !harness.screen().contains("┌ Settings "),
        "the Navigator's key works there too, and the Navigator takes its place"
    );
    assert!(harness.screen().contains("┌Navigator"));
}

// FR-SET-02, FR-UI-03, FR-SET-04
#[test]
fn switching_to_flexoki_light_repaints_the_app_at_once_and_keeps_it() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("g,");
    for _ in 0..4 {
        harness.type_text("l");
    }
    let lines = window(&harness);
    assert_eq!(
        lines.get(1).map(String::as_str),
        Some(" * Theme                                        Flexoki Light")
    );
    let corner = harness.terminal.backend().buffer()[(0, 0)].clone();
    assert_eq!(corner.bg, Color::Rgb(0xff, 0xfc, 0xf0), "the whole screen");
    assert_eq!(
        fs::read_to_string(harness.config()).ok().as_deref(),
        Some("theme = \"flexoki-light\"\n")
    );
    harness.type_text("h");
    assert_eq!(
        fs::read_to_string(harness.config()).ok().as_deref(),
        Some("theme = \"github-light\"\n"),
        "h goes back a theme"
    );
    harness.type_text("r");
    assert_eq!(
        fs::read_to_string(harness.config()).ok().as_deref(),
        Some("theme = \"terminal-dark\"\n"),
        "resetting keeps the default"
    );
    let corner = harness.terminal.backend().buffer()[(0, 0)].clone();
    assert_eq!(corner.bg, Color::Reset);
}

// FR-UI-11, FR-SET-02
#[test]
fn grayscale_repaints_the_app_in_the_themes_greys_and_keeps_it() {
    let hued = |harness: &Harness| {
        let buffer = harness.terminal.backend().buffer();
        buffer.content.iter().any(|cell| {
            [cell.fg, cell.bg].into_iter().any(|color| match color {
                Color::Rgb(red, green, blue) => red != green || green != blue,
                Color::Indexed(index) => !matches!(index, 0 | 7 | 8 | 15 | 232..=255),
                Color::Reset | Color::Black | Color::DarkGray | Color::Gray | Color::White => false,
                _ => true,
            })
        })
    };
    for theme in ["terminal-dark", "tokyo-night"] {
        let folders = tempfile::tempdir().expect("a temporary folder");
        let text = format!("theme = \"{theme}\"\n");
        fs::write(folders.path().join("config.toml"), &text).expect("the file is written");
        let mut harness = Harness::launched_in(folders, None, 100, 30).expect("the demo opens");
        harness.type_text("gt");
        assert!(hued(&harness), "{theme}: colors before");
        harness.type_text("g,jl");
        harness.key(KeyCode::Esc);
        assert!(!hued(&harness), "{theme}: {}", harness.screen());
        harness.type_text("gi");
        assert!(!hued(&harness), "{theme}: the Inbox too");
        assert_eq!(
            fs::read_to_string(harness.config()).ok(),
            Some(format!("{text}grayscale = true\n"))
        );
    }
}

// FR-SET-02, FR-UI-04, FR-UI-05
#[test]
fn icons_and_word_wrap_change_what_the_panes_draw() {
    let mut harness = Harness::new(30, 24);
    let wrapped = harness.screen();
    assert!(wrapped.contains("▌   the card"), "{wrapped}");
    harness.type_text("g,jj");
    harness.key(KeyCode::Enter);
    harness.type_text("j ");
    harness.key(KeyCode::Esc);
    let screen = harness.screen();
    assert!(screen.contains("⊔ Inbox"), "plain icons: {screen}");
    assert!(screen.contains("▌ ☐ Call the bank abou…"), "{screen}");
    assert!(!screen.contains("bank about the card"), "{screen}");
    assert_eq!(
        fs::read_to_string(harness.config()).ok().as_deref(),
        Some("icons = \"plain\"\nword_wrap = false\n")
    );
}

// FR-SET-01, FR-SET-04
#[test]
fn the_app_starts_with_what_the_file_sets_and_says_what_it_ignored() {
    let folders = tempfile::tempdir().expect("a temporary folder");
    let text = "# mine\nicons = \"plain\"\ntheme = \"nord\"\nfont = 1\n";
    fs::write(folders.path().join("config.toml"), text).expect("the file is written");
    // Wide enough for the whole status line, which lists every theme's key.
    let mut harness = Harness::launched_in(folders, None, 600, 24).expect("the demo opens");
    let screen = harness.screen();
    assert!(screen.contains("⊔ Inbox"), "{screen}");
    let status = harness.rows().last().cloned().unwrap_or_default();
    assert!(
        status.contains("config.toml: `theme`: unknown variant `nord`"),
        "{status}"
    );
    assert!(status.contains("; unknown setting `font`"), "{status}");
    harness.type_text("j");
    let status = harness.rows().last().cloned().unwrap_or_default();
    assert!(!status.contains("config.toml"), "{status}");
    harness.type_text("g,jjl");
    assert_eq!(
        fs::read_to_string(harness.config()).ok().as_deref(),
        Some("# mine\nicons = \"nerd\"\ntheme = \"nord\"\nfont = 1\n"),
        "the rest of the file is kept"
    );
}

// FR-SET-04
#[test]
fn a_setting_that_cannot_be_kept_still_applies_and_the_status_line_says_why() {
    let mut harness = Harness::new(120, 24);
    fs::create_dir(harness.config()).expect("a folder where the file goes");
    harness.type_text("g,l");
    let lines = window(&harness);
    assert!(
        lines
            .get(1)
            .is_some_and(|line| line.ends_with("Catppuccin Mocha")),
        "{lines:#?}"
    );
    let status = harness.rows().last().cloned().unwrap_or_default();
    assert!(
        status.contains("config.toml: `theme` not saved: "),
        "{status}"
    );
}

// FR-SET-02, FR-KEY-03
#[test]
fn the_shortcuts_panel_lists_the_windows_keys_and_pastes_go_nowhere() {
    let mut harness = Harness::new(160, 30);
    harness.type_text("?g,");
    let screen = harness.screen();
    for line in [
        "Previous Setting",
        "Next Setting",
        "Next Value",
        "Previous Value",
        "Reset to Default",
    ] {
        assert!(screen.contains(line), "{line}: {screen}");
    }
    harness.send(&Event::Paste("New task".to_owned()));
    harness.key(KeyCode::Esc);
    assert!(!harness.screen().contains("New task"));
}

/// The sidebar's rows on an 80-column screen, trimmed, without the status line.
fn sidebar(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .map(|row| {
            let row: String = row.chars().take(25).collect();
            row.trim_matches(['▌', ' ']).to_owned()
        })
        .filter(|row| !row.is_empty())
        .collect()
}

/// The Navigator's rows, trimmed, without its border.
fn navigator(harness: &mut Harness) -> Vec<String> {
    harness.ctrl('g');
    let rows = window(harness);
    harness.key(KeyCode::Esc);
    rows.into_iter().map(|row| row.trim().to_owned()).collect()
}

// FR-VIEW-10, FR-VIEW-07, FR-SET-01, FR-SET-02
#[test]
fn turning_task_counts_off_hides_the_sidebars_and_the_navigators_counts_and_keeps_it() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('b');
    let counted = sidebar(&harness);
    assert_eq!(
        counted.get(..8).unwrap_or_default(),
        [
            "󰚇 Inbox                3",
            "Views",
            "󰖙 Today                4",
            "󱄵 Upcoming             6",
            "󰈻 Flagged",
            "󰉹 All                 18",
            "󱓳 Logbook              1",
            "󰩺 Trash                2",
        ]
    );
    assert!(navigator(&mut harness).contains(
        &"# Learn Rust                                                               2".to_owned()
    ));
    harness.type_text("g,jjjjjl");
    harness.key(KeyCode::Esc);
    assert_eq!(
        fs::read_to_string(harness.config()).ok().as_deref(),
        Some("task_counts = false\n")
    );
    let uncounted = sidebar(&harness);
    assert_eq!(
        uncounted.get(..8).unwrap_or_default(),
        [
            "󰚇 Inbox",
            "Views",
            "󰖙 Today",
            "󱄵 Upcoming",
            "󰈻 Flagged",
            "󰉹 All",
            "󱓳 Logbook",
            "󰩺 Trash",
        ]
    );
    assert!(
        uncounted.iter().all(|row| !row.ends_with(char::is_numeric)),
        "{uncounted:#?}"
    );
    let places = navigator(&mut harness);
    assert!(places.contains(&"# Learn Rust".to_owned()), "{places:#?}");
}

// FR-VIEW-10, FR-SET-01
#[test]
fn the_app_starts_without_task_counts_when_the_file_turns_them_off() {
    let folders = tempfile::tempdir().expect("a temporary folder");
    fs::write(folders.path().join("config.toml"), "task_counts = false\n")
        .expect("the file is written");
    let mut harness = Harness::launched_in(folders, None, 80, 24).expect("the demo opens");
    harness.ctrl('b');
    let rows = sidebar(&harness);
    assert_eq!(
        rows.get(..3).unwrap_or_default(),
        ["󰚇 Inbox", "Views", "󰖙 Today"]
    );
}
