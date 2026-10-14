//! Creating, renaming and deleting areas, projects and sections.

use std::fs;

use crossterm::event::KeyCode;
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

/// The screen's rows below the breadcrumbs and above the status line, without trailing spaces,
/// up to the last that is not blank.
fn pane(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    let mut pane: Vec<String> = rows
        .iter()
        .take(rows.len() - 1)
        .skip(2)
        .map(|row| row.trim_end().to_owned())
        .collect();
    while pane.last().is_some_and(String::is_empty) {
        pane.pop();
    }
    pane
}

/// The breadcrumb line, without trailing spaces.
fn crumbs(harness: &Harness) -> String {
    harness
        .rows()
        .first()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

// FR-ORG-06, FR-EDIT-05
#[test]
fn add_an_area_with_a_project_and_a_section() {
    let mut harness = Harness::new(60, 16);
    harness.type_text("oaHome");
    assert_eq!(crumbs(&harness), "  󰋜 Home");
    harness.key(KeyCode::Esc);
    harness.type_text("opGarden");
    harness.key(KeyCode::Esc);
    harness.type_text("OsChores");
    harness.key(KeyCode::Enter);
    harness.type_text("Sweep");
    harness.key(KeyCode::Esc);
    assert_eq!(
        pane(&harness),
        ["  󰆧 Home", "", "  󰚟 Chores", "▌ - Sweep", "", "  󰉖 Garden",]
    );
}

// FR-ORG-06, FR-EDIT-02
#[test]
fn the_container_shown_is_renamed_in_its_heading_and_deleted() {
    let mut harness = Harness::new(60, 16);
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
    harness.type_text("R site");
    assert!(harness.rows()[2].starts_with("  󰉖 Website site "));
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 60, 16)),
        Some(Position::new(16, 2)),
        "the cursor stands after the text typed"
    );
    harness.key(KeyCode::Esc);
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website site");
    harness.type_text("dc");
    assert_eq!(crumbs(&harness), "  󰋜 Work");
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
    harness.type_text("oaHome");
    harness.key(KeyCode::Esc);
    harness.type_text("opGarden");
    harness.key(KeyCode::Esc);
    harness.key(KeyCode::Char('.'));
    harness.type_text("osSpring");
    harness.key(KeyCode::Esc);
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\nHome: @area\n\tGarden:\n\t\tSpring: @section\n"
    );
}
