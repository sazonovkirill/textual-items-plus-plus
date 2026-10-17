//! Creating, renaming and deleting areas, projects and sections.

use std::fs;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

/// The screen's rows above the status line, without trailing spaces,
/// up to the last that is not blank.
fn pane(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    let mut pane: Vec<String> = rows
        .iter()
        .take(rows.len() - 1)
        .map(|row| row.trim_end().to_owned())
        .collect();
    while pane.last().is_some_and(String::is_empty) {
        pane.pop();
    }
    pane
}

/// The title line at the top of the pane, without the focus marker or what it has at the right.
fn title(harness: &Harness) -> String {
    harness
        .rows()
        .first()
        .and_then(|row| {
            let row = row.trim_start_matches('▌').trim();
            row.split("  ").next().map(str::to_owned)
        })
        .unwrap_or_default()
}

// FR-ORG-06, FR-EDIT-05, D-40
#[test]
fn add_an_area_with_a_project_and_a_section() {
    let mut harness = Harness::new(60, 16);
    harness.ctrl('b');
    harness.type_text("aHome");
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Enter);
    harness.ctrl('b');
    assert_eq!(title(&harness), "󰆧 Home");
    harness.type_text("pGarden");
    harness.key(KeyCode::Esc);
    harness.type_text("SChores");
    harness.key(KeyCode::Enter);
    harness.type_text("oSweep");
    harness.key(KeyCode::Esc);
    assert_eq!(
        pane(&harness),
        ["  󰆧 Home", "", "  / Chores", "▌ ☐ Sweep", "", "  # Garden",],
        "o on a section adds a task in it (D-127)"
    );
}

// FR-ORG-06, FR-EDIT-02, D-83
#[test]
fn the_container_shown_is_renamed_in_its_heading_and_deleted() {
    let mut harness = Harness::new(60, 16);
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
    harness.type_text("R site");
    assert!(harness.rows()[0].starts_with("  # Website site "));
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 60, 16)),
        Some(Position::new(16, 0)),
        "the cursor stands after the text typed"
    );
    harness.key(KeyCode::Esc);
    assert_eq!(title(&harness), "# Website site");
    harness.type_text("dc");
    assert_eq!(title(&harness), "# Website site", "no key deletes it");
    harness.ctrl('p');
    harness.type_text("delete container");
    assert!(
        harness.screen().contains("Delete Container"),
        "a command with no key"
    );
    harness.key(KeyCode::Enter);
    assert_eq!(title(&harness), "󰆧 Work");
    assert!(!harness.screen().contains("Website"));
    harness.type_text("z");
    assert!(harness.screen().contains("Website site"));
}

// FR-ORG-06, FR-STORE-02
#[test]
fn new_containers_are_saved_with_their_kind_tags() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n").expect("written");
    let mut harness = Harness::open(path.clone(), 60, 16);
    harness.ctrl('b');
    harness.type_text("aHome");
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Tab);
    harness.type_text("pGarden");
    harness.key(KeyCode::Esc);
    harness.key(KeyCode::Enter);
    harness.type_text("sSpring");
    harness.key(KeyCode::Esc);
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\nHome: @area\n\tGarden:\n\t\tSpring: @section\n"
    );
}

// FR-NAV-01, FR-EDIT-02, FR-EDIT-03, FR-DATE-01, FR-TAG-02, FR-ATT-02, D-68
#[test]
fn the_title_of_the_project_shown_is_a_row_its_own_commands_act_on() {
    let mut harness = Harness::new(60, 16);
    harness.ctrl('g');
    harness.type_text("blog");
    harness.key(KeyCode::Enter);
    harness.type_text("k");
    assert_eq!(
        pane(&harness)[..2],
        ["▌ # Blog", "▌ A project in the Work area."],
        "the title and the description are one row"
    );
    harness.type_text("i");
    harness.type_text(" posts");
    harness.key(KeyCode::Enter);
    harness.type_text("mn");
    harness.press(KeyCode::Home, KeyModifiers::CONTROL);
    harness.press(KeyCode::End, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    harness.type_text("Weekly.");
    harness.key(KeyCode::Esc);
    harness.type_text("md");
    harness.type_text("tomorrow");
    harness.key(KeyCode::Enter);
    harness.type_text("mg");
    harness.type_text("web");
    harness.key(KeyCode::Enter);
    let heading = pane(&harness);
    assert!(
        heading[0].starts_with("▌ # Blog posts @web") && heading[0].ends_with("tomorrow"),
        "{heading:?}"
    );
    assert_eq!(heading[1], "▌ Weekly.");
    harness.type_text("o");
    harness.type_text("Draft");
    harness.key(KeyCode::Esc);
    assert_eq!(
        pane(&harness)[3..5],
        ["▌ ☐ Draft", "  ☐ Pick the first three topics"],
        "below the title, the first task"
    );
    harness.type_text("k\\");
    assert!(harness.screen().contains("┌ Blog posts ─"), "its notes");
}

// FR-ORG-10, FR-VIEW-10, D-68
#[test]
fn describe_a_project_from_the_sidebar() {
    let mut harness = Harness::new(60, 16);
    harness.ctrl('b');
    harness.type_text("G");
    harness.type_text("mn");
    harness.press(KeyCode::Home, KeyModifiers::CONTROL);
    harness.press(KeyCode::End, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    harness.type_text("Pack first.");
    harness.key(KeyCode::Esc);
    assert!(
        harness.rows()[0].contains("Move to the new flat"),
        "the pane opens the project"
    );
    assert!(harness.screen().contains("▌ Pack first."));
}
