//! The sidebar: showing and hiding it, and creating, renaming, deleting, reordering, indenting
//! and outdenting areas and projects in it.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

/// The sidebar's entries on an 80-column screen, trimmed at the right, the focus marker
/// written `*`.
fn entries(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .filter_map(|row| {
            let sidebar: String = row.chars().take(25).collect();
            let focused = sidebar.starts_with('▌');
            let text = sidebar.trim_start_matches('▌').trim_end();
            if text.trim().is_empty() {
                return None;
            }
            Some(if focused {
                format!("*{text}")
            } else {
                text.to_owned()
            })
        })
        .collect()
}

/// The status line, without trailing spaces.
fn status(harness: &Harness) -> String {
    harness
        .rows()
        .last()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

/// The demo with the sidebar shown, focused on its last entry.
fn sidebar() -> Harness {
    let mut harness = Harness::new(80, 24);
    harness.type_text("MG");
    harness
}

// FR-VIEW-10, FR-ORG-06, FR-EDIT-06
#[test]
fn reorder_projects_in_the_sidebar() {
    let mut harness = sidebar();
    assert_eq!(
        entries(&harness),
        [
            " 󰆧 Work                 7",
            "   󰉖 Website            5",
            "     󰉖 Blog             1",
            " 󰆧 Home                 5",
            "   󰉖 Move to the new fl 4",
            "*󰉖 Learn Rust           2",
        ]
    );
    harness.press(KeyCode::Char('k'), KeyModifiers::ALT);
    let moved = entries(&harness);
    assert_eq!(
        moved.get(0..2),
        Some(
            &[
                " 󰆧 Work                 7".to_owned(),
                "   󰉖 Website            5".to_owned()
            ][..]
        )
    );
    assert_eq!(
        moved.get(3).map(String::as_str),
        Some("*󰉖 Learn Rust           2"),
        "{moved:?}"
    );
    assert_eq!(
        moved.get(4).map(String::as_str),
        Some(" 󰆧 Home                 5")
    );
    harness.type_text("z");
    assert_eq!(
        entries(&harness).last().map(String::as_str),
        Some("*󰉖 Learn Rust           2")
    );
    harness.type_text("M");
    assert!(
        harness.rows()[0].starts_with("  󰋜 Inbox"),
        "hidden, the pane takes the screen: {:?}",
        harness.rows()[0]
    );
}

// FR-VIEW-10, FR-EDIT-05, FR-ORG-06
#[test]
fn add_a_project_to_an_area_and_rename_it_in_the_sidebar() {
    let mut harness = sidebar();
    harness.type_text("gg");
    harness.type_text("op");
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(5, 1)),
        "the caret in the new entry"
    );
    harness.type_text("Garden");
    harness.key(KeyCode::Enter);
    let added = entries(&harness);
    assert_eq!(
        added.get(1).map(String::as_str),
        Some("*  󰉖 Garden"),
        "{added:?}"
    );
    assert!(
        harness.rows()[0].contains("󰋜 Inbox"),
        "the pane stays on the Inbox"
    );
    harness.key(KeyCode::Enter);
    harness.type_text("s");
    harness.key(KeyCode::Enter);
    assert_eq!(
        entries(&harness).get(1).map(String::as_str),
        Some("*  󰉖 Gardens")
    );
    harness.type_text("Oa");
    harness.type_text("Errands");
    harness.key(KeyCode::Enter);
    assert_eq!(
        entries(&harness).first().map(String::as_str),
        Some("*󰆧 Errands")
    );
}

// FR-VIEW-10, FR-OPS-03
#[test]
fn indent_a_project_into_an_area_and_outdent_it_again() {
    let mut harness = sidebar();
    harness.type_text("]");
    let indented = entries(&harness);
    assert_eq!(
        indented.last().map(String::as_str),
        Some("*  󰉖 Learn Rust         2"),
        "{indented:?}"
    );
    harness.type_text("[");
    assert_eq!(
        entries(&harness).last().map(String::as_str),
        Some("*󰉖 Learn Rust           2")
    );
    harness.type_text("gg]");
    assert!(status(&harness).contains("only projects and checklist items nest"));
}

// FR-VIEW-10, FR-OPS-05, FR-EDIT-06
#[test]
fn delete_the_project_shown_from_the_sidebar() {
    let mut harness = sidebar();
    harness.type_text(".");
    assert!(
        harness.rows()[0].contains("Learn Rust"),
        "opened in the pane"
    );
    harness.type_text("dd");
    assert!(
        !entries(&harness)
            .iter()
            .any(|entry| entry.contains("Learn Rust"))
    );
    assert!(harness.rows()[0].contains("󰋜 Inbox"), "the pane goes home");
    harness.type_text("z");
    assert!(
        entries(&harness)
            .iter()
            .any(|entry| entry.contains("Learn Rust"))
    );
}

// FR-VIEW-10, FR-KEY-01
#[test]
fn the_keys_go_to_the_pane_and_back_to_the_sidebar() {
    let mut harness = sidebar();
    let pane_focus = |harness: &Harness| {
        harness
            .rows()
            .iter()
            .position(|row| row.chars().nth(26) == Some('▌'))
    };
    assert_eq!(pane_focus(&harness), None, "the pane's focus is not marked");
    harness.ctrl('w');
    harness.type_text("l");
    let first = pane_focus(&harness);
    assert!(first.is_some());
    assert!(!entries(&harness).iter().any(|entry| entry.starts_with('*')));
    harness.type_text("j");
    assert_eq!(pane_focus(&harness), first.map(|row| row + 1));
    harness.ctrl('w');
    harness.key(KeyCode::Left);
    assert_eq!(
        entries(&harness).last().map(String::as_str),
        Some("*󰉖 Learn Rust           2")
    );
    harness.type_text("k");
    assert_eq!(pane_focus(&harness), None);
    assert!(entries(&harness)[4].starts_with('*'));
}

// FR-VIEW-10, FR-EDIT-04, FR-UI-05
#[test]
fn up_and_down_in_a_description_follow_its_lines_beside_the_sidebar() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("M");
    harness.ctrl('w');
    harness.type_text("l");
    harness.type_text("mn");
    harness.type_text("alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima");
    harness.press(KeyCode::Home, KeyModifiers::CONTROL);
    let top = harness.tui.cursor(Rect::new(0, 0, 80, 24));
    harness.key(KeyCode::Down);
    let below = harness.tui.cursor(Rect::new(0, 0, 80, 24));
    assert_eq!(
        below,
        top.map(|at| Position::new(at.x, at.y + 1)),
        "the start of the second line on screen"
    );
}
