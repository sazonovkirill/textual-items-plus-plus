//! The settings window and the settings file: opening the window with `g,`, changing each setting
//! with its live preview, keeping changes in `config.toml`, and starting with what the file sets.

use std::fs;

use crossterm::event::{Event, KeyCode};
use ratatui::style::Color;

use crate::support::Harness;

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
            "   Icons                                            Nerd Font",
            "   Nerd Font icons, or plain markers without the font",
            "   Word wrap                                               On",
            "   Long titles wrap; off, each takes one line, cut short",
        ]
    );
    let config = harness.config().display().to_string();
    assert_eq!(lines.last().map(|line| line.trim()), Some(config.as_str()));
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

// FR-SET-02, FR-UI-04, FR-UI-05
#[test]
fn icons_and_word_wrap_change_what_the_panes_draw() {
    let mut harness = Harness::new(30, 24);
    let wrapped = harness.screen();
    assert!(wrapped.contains("▌   the card"), "{wrapped}");
    harness.type_text("g,j");
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
    let mut harness = Harness::in_folders(folders, None, None, 300, 24).expect("the demo opens");
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
    harness.type_text("g,jl");
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
