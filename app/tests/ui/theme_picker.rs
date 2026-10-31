//! The theme picker the command palette's Change Theme opens: the themes by kind, each previewed
//! as it is highlighted, the one chosen kept in `config.toml` and `Esc` going back.

use std::fs;

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Position, Rect},
    style::Color,
};

use crate::support::Harness;

/// The rows of the theme picker's window under its query, each without its icon and trailing
/// spaces.
fn window(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .skip_while(|row| !row.starts_with("┌Change Theme"))
        .skip(2)
        .take_while(|row| !row.starts_with('└'))
        .map(|row| {
            let row = row.trim_start_matches('│').trim_end().trim_end_matches('│');
            let row = row.trim();
            match row.split_once(' ') {
                Some((icon, theme)) if icon.chars().count() == 1 => theme.to_owned(),
                _ => row.to_owned(),
            }
        })
        .collect()
}

/// The background the screen is drawn on.
fn background(harness: &Harness) -> Color {
    harness.terminal.backend().buffer()[(0, 0)].bg
}

/// Opens the theme picker from the command palette, as its title reads there.
fn change_theme(harness: &mut Harness) {
    harness.ctrl('p');
    harness.type_text("change theme");
    harness.key(KeyCode::Enter);
}

// FR-UI-12, FR-NAV-06
#[test]
fn change_theme_lists_the_dark_themes_then_the_light_ones_and_previews_the_one_highlighted() {
    let mut harness = Harness::new(80, 30);
    let before = background(&harness);
    change_theme(&mut harness);
    let listed = window(&harness);
    assert_eq!(
        listed.get(..3).unwrap_or_default(),
        ["Dark Themes", "Terminal Dark", "Catppuccin Mocha"]
    );
    assert_eq!(
        background(&harness),
        before,
        "the current theme highlighted"
    );
    let query = harness
        .rows()
        .iter()
        .position(|row| row.starts_with("┌Change Theme"))
        .and_then(|row| u16::try_from(row + 1).ok());
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 30)),
        query.map(|row| Position::new(3, row)),
        "the cursor stands in the query"
    );
    harness.key(KeyCode::Down);
    assert_eq!(
        background(&harness),
        Color::Rgb(0x1e, 0x1e, 0x2e),
        "Catppuccin Mocha at once"
    );
    harness.type_text("light");
    assert_eq!(
        window(&harness).get(..2).unwrap_or_default(),
        ["Light Themes", "GitHub Light Default"]
    );
    assert_eq!(background(&harness), Color::Rgb(0xff, 0xff, 0xff));
    harness.key(KeyCode::Esc);
    assert!(!harness.screen().contains("Change Theme"), "Esc closes it");
    assert_eq!(
        background(&harness),
        before,
        "and goes back to the theme it opened on"
    );
    assert!(!harness.config().exists(), "keeping nothing");
}

// FR-UI-12, FR-SET-03
#[test]
fn enter_keeps_the_theme_chosen_which_the_picker_then_opens_on() {
    let mut harness = Harness::new(80, 30);
    change_theme(&mut harness);
    harness.type_text("flexoki light");
    harness.key(KeyCode::Enter);
    assert!(!harness.screen().contains("Change Theme"));
    let flexoki = Color::Rgb(0xff, 0xfc, 0xf0);
    assert_eq!(background(&harness), flexoki);
    assert_eq!(
        fs::read_to_string(harness.config()).ok().as_deref(),
        Some("theme = \"flexoki-light\"\n")
    );
    change_theme(&mut harness);
    harness.key(KeyCode::Up);
    harness.key(KeyCode::Down);
    assert_eq!(background(&harness), flexoki, "highlighted when it opens");
    harness.key(KeyCode::Up);
    assert_ne!(background(&harness), flexoki);
    harness.key(KeyCode::Esc);
    assert_eq!(background(&harness), flexoki);
}

// FR-UI-03, FR-UI-12, FR-SET-03
#[test]
fn star_trek_tng_previews_its_lcars_colors_and_keeps_the_choice() {
    let mut harness = Harness::new(80, 30);
    let before = background(&harness);
    change_theme(&mut harness);
    harness.type_text("star trek");
    assert_eq!(window(&harness), ["Dark Themes", "Star Trek TNG"]);
    assert_eq!(background(&harness), Color::Rgb(0, 0, 0));
    let buffer = harness.terminal.backend().buffer();
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.fg == Color::Rgb(0xcc, 0x99, 0xcc))
    );
    harness.key(KeyCode::Esc);
    assert_eq!(background(&harness), before);
    change_theme(&mut harness);
    harness.type_text("star trek");
    harness.key(KeyCode::Enter);
    assert_eq!(background(&harness), Color::Rgb(0, 0, 0));
    assert_eq!(
        fs::read_to_string(harness.config()).ok().as_deref(),
        Some("theme = \"star-trek-tng\"\n")
    );
    harness.ctrl('b');
    let buffer = harness.terminal.backend().buffer();
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.fg == Color::Rgb(0xff, 0xcc, 0x66))
    );
    fs::create_dir_all(harness.home()).expect("the home folder is created");
    let path = harness.home().join("tasks.taskpaper");
    fs::write(&path, "Inbox:\n").expect("the next launch's document is written");
    let harness = harness.relaunch(path);
    assert_eq!(
        harness.terminal.backend().buffer()[(79, 29)].bg,
        Color::Rgb(0, 0, 0)
    );
}
