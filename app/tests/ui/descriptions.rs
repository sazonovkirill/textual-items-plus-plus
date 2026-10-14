//! Editing descriptions in place, under a row and under the title of the container shown.

use std::fs;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

const SCREEN: Rect = Rect::new(0, 0, 40, 14);

/// The rows of the pane, without trailing spaces.
fn pane(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .map(|row| row.trim_end().to_owned())
        .collect()
}

/// The mode the status line names.
fn mode(harness: &Harness) -> String {
    let status = harness.rows().last().cloned().unwrap_or_default();
    status
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned()
}

// FR-ORG-10, FR-EDIT-02, FR-EDIT-03, FR-EDIT-06
#[test]
fn describe_a_task_in_place_and_undo_it() {
    let mut harness = Harness::new(40, 14);
    harness.type_text("mn");
    assert_eq!(mode(&harness), "EDIT");
    harness.type_text("Ask about the fees on the new card before Friday");
    harness.key(KeyCode::Enter);
    harness.type_text("- call before 5");
    assert_eq!(
        pane(&harness)[3..8],
        [
            "▌ - Call the bank about the card  󱙬 Wed",
            "▌   Ask about the fees on the new card",
            "▌   before Friday",
            "▌   - call before 5",
            "  - Buy coffee beans @errands",
        ],
        "the whole description shows while it is edited"
    );
    assert_eq!(harness.tui.cursor(SCREEN), Some(Position::new(19, 6)));
    harness.key(KeyCode::Esc);
    assert_eq!(mode(&harness), "NAVIGATE");
    assert_eq!(
        pane(&harness)[3..7],
        [
            "▌ - Call the bank about the card  󱙬 Wed",
            "▌   Ask about the fees on the new card",
            "▌   before Friday",
            "  - Buy coffee beans @errands",
        ],
        "a row shows its description's first line"
    );
    harness.type_text("z");
    assert_eq!(
        pane(&harness)[3..5],
        [
            "▌ - Call the bank about the card  󱙬 Wed",
            "  - Buy coffee beans @errands"
        ],
        "the whole edit is one step to undo"
    );
}

// FR-EDIT-04, FR-UI-05
#[test]
fn up_and_down_follow_the_lines_on_screen() {
    let mut harness = Harness::new(40, 14);
    harness.type_text("mn");
    harness.type_text("Ask about the fees on the new card before Friday");
    harness.key(KeyCode::Enter);
    harness.type_text("call");
    harness.key(KeyCode::Up);
    assert_eq!(harness.tui.cursor(SCREEN), Some(Position::new(8, 5)));
    harness.key(KeyCode::Up);
    assert_eq!(harness.tui.cursor(SCREEN), Some(Position::new(8, 4)));
    harness.type_text("X");
    harness.key(KeyCode::Down);
    harness.key(KeyCode::Down);
    harness.key(KeyCode::Down);
    harness.type_text("!");
    harness.key(KeyCode::Esc);
    harness.type_text("mn");
    harness.key(KeyCode::Down);
    assert_eq!(
        pane(&harness)[4..7],
        [
            "▌   Ask Xabout the fees on the new card",
            "▌   before Friday",
            "▌   call!",
        ]
    );
}

// FR-ORG-05, FR-ORG-10, FR-EDIT-03
#[test]
fn describe_the_project_shown() {
    let mut harness = Harness::new(40, 14);
    harness.ctrl('g');
    harness.type_text("Blog");
    harness.key(KeyCode::Enter);
    harness.press(KeyCode::Char('m'), KeyModifiers::NONE);
    harness.press(KeyCode::Char('N'), KeyModifiers::SHIFT);
    harness.ctrl('a');
    harness.type_text("Notes on writing");
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Enter);
    harness.type_text("Weekly.");
    assert!(pane(&harness)[0].ends_with(" Blog"));
    assert_eq!(
        pane(&harness)[1..5],
        ["  Notes on writing", "", "  Weekly.", ""],
        "a description takes the place of what was selected"
    );
    harness.key(KeyCode::Esc);
    assert_eq!(
        pane(&harness)[1..5],
        ["  Notes on writing", "", "  Weekly.", ""],
        "the container shown shows its whole description"
    );
}

// FR-STORE-04, FR-ORG-10
#[test]
fn a_description_is_saved_as_guarded_lines_and_reads_back() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- Renew the passport\n").expect("the fixture is written");
    let mut harness = Harness::open(path.clone(), 40, 14);
    harness.type_text("mn");
    harness.type_text("- bring photos");
    harness.key(KeyCode::Enter);
    harness.type_text("Goals:  ");
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Esc);
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\n\t- Renew the passport\n\t\t\u{200B}- bring photos\n\t\tGoals:\u{200B}\n",
        "space and empty lines at the end are dropped"
    );
    let mut reopened = Harness::open(path, 40, 14);
    reopened.type_text("mn");
    assert_eq!(
        pane(&reopened)[2..5],
        ["▌ - Renew the passport", "▌   - bring photos", "▌   Goals:",]
    );
}
