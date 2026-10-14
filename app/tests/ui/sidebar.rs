//! The sidebar: showing and hiding it, its groups, and creating, renaming, opening, deleting,
//! reordering, moving and collapsing areas and projects in it.

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

/// The demo with the sidebar shown, focused on its project in no area, Learn Rust, below the
/// Inbox and the views.
fn sidebar() -> Harness {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('b');
    harness.type_text("jjjjj");
    harness
}

// FR-VIEW-10, FR-ORG-06, FR-EDIT-06
#[test]
fn reorder_areas_in_the_sidebar() {
    let mut harness = sidebar();
    assert_eq!(
        entries(&harness),
        [
            " 󰚇 Inbox                3",
            " 󰖙 Today",
            " 󱄵 Upcoming",
            " 󰉹 All",
            " 󰩺 Trash",
            "*# Learn Rust           2",
            " 󰆧 Work                 7",
            " # Website              4",
            " # Blog                 1",
            " 󰆧 Home                 5",
            " # Move to the new flat 4",
        ]
    );
    let rows = harness.rows();
    let blank = |row: usize| {
        rows.get(row)
            .is_some_and(|row| row.chars().take(25).all(|c| c == ' '))
    };
    assert!(
        blank(1) && blank(6) && blank(8) && blank(12),
        "blank lines set the groups off"
    );
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    assert_eq!(
        entries(&harness).get(5).map(String::as_str),
        Some("*# Learn Rust           2"),
        "no other project in no area to change places with"
    );
    harness.type_text("G");
    harness.type_text("#");
    harness.press(KeyCode::Char('k'), KeyModifiers::ALT);
    let moved = entries(&harness);
    assert_eq!(
        moved.get(6..9),
        Some(
            &[
                "*󰆧 Home                 5".to_owned(),
                " # Move to the new flat 4".to_owned(),
                " 󰆧 Work                 7".to_owned(),
            ][..]
        ),
        "{moved:?}"
    );
    harness.type_text("z");
    assert_eq!(
        entries(&harness).get(9).map(String::as_str),
        Some("*󰆧 Home                 5")
    );
    harness.ctrl('b');
    assert!(
        harness.rows()[0].starts_with("  󰋜 Inbox"),
        "hidden, the pane takes the screen: {:?}",
        harness.rows()[0]
    );
}

// FR-VIEW-10, FR-EDIT-05, FR-ORG-06
#[test]
fn add_rename_and_open_entries_in_the_sidebar() {
    let mut harness = sidebar();
    harness.type_text("gg");
    harness.type_text("o");
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(3, 7)),
        "the caret in the new entry, first of the projects in no area"
    );
    harness.type_text("Garden");
    harness.key(KeyCode::Enter);
    let added = entries(&harness);
    assert_eq!(
        added.get(5).map(String::as_str),
        Some("*# Garden"),
        "{added:?}"
    );
    assert!(
        harness.rows()[0].contains("󰋜 Inbox"),
        "the pane stays on the Inbox"
    );
    harness.type_text("R");
    harness.type_text("s");
    harness.key(KeyCode::Enter);
    assert_eq!(
        entries(&harness).get(5).map(String::as_str),
        Some("*# Gardens")
    );
    harness.type_text("A");
    harness.type_text("Errands");
    harness.key(KeyCode::Enter);
    assert_eq!(
        entries(&harness).get(7).map(String::as_str),
        Some("*󰆧 Errands"),
        "first of the areas"
    );
    harness.key(KeyCode::Enter);
    let crumbs: String = harness.rows()[0].chars().skip(28).collect();
    assert!(crumbs.contains("Errands"), "Enter opens it: {crumbs:?}");
}

// FR-VIEW-10, FR-OPS-03, FR-OPS-04, D-75
#[test]
fn move_a_project_into_an_area_and_back_to_the_top_level() {
    let mut harness = sidebar();
    let before = entries(&harness);
    harness.type_text("][");
    assert_eq!(entries(&harness), before, "a project does not indent");
    harness.type_text("gm");
    harness.type_text("Home");
    harness.key(KeyCode::Enter);
    let moved = entries(&harness);
    assert_eq!(
        moved.last().map(String::as_str),
        Some("*# Learn Rust           2"),
        "{moved:?}"
    );
    harness.type_text("gm");
    harness.key(KeyCode::Enter);
    assert_eq!(
        entries(&harness).get(5).map(String::as_str),
        Some("*# Learn Rust           2")
    );
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
        entries(&harness).get(5).map(String::as_str),
        Some("*# Learn Rust           2")
    );
    harness.type_text("k");
    assert_eq!(pane_focus(&harness), None);
    assert_eq!(entries(&harness)[4], "*󰩺 Trash");
}

// FR-VIEW-10, FR-EDIT-04, FR-UI-05
#[test]
fn up_and_down_in_a_description_follow_its_lines_beside_the_sidebar() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('b');
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

// FR-VIEW-10, D-67, D-75
#[test]
fn collapse_an_area_and_expand_it_again() {
    let mut harness = sidebar();
    harness.type_text("j'");
    assert_eq!(
        entries(&harness).get(6..9),
        Some(
            &[
                "*󰆧 Work               ▸ 7".to_owned(),
                " 󰆧 Home                 5".to_owned(),
                " # Move to the new flat 4".to_owned(),
            ][..]
        ),
        "the projects inside it hide"
    );
    harness.type_text("j'");
    assert_eq!(
        entries(&harness).get(7).map(String::as_str),
        Some("*󰆧 Home               ▸ 5")
    );
    harness.type_text("k'");
    assert_eq!(
        entries(&harness).get(7).map(String::as_str),
        Some(" # Website              4")
    );
    harness.type_text("G");
    harness.type_text("iBoxes");
    harness.key(KeyCode::Enter);
    let added = entries(&harness);
    assert_eq!(
        added.get(9..12),
        Some(
            &[
                " 󰆧 Home                 5".to_owned(),
                " # Move to the new flat 4".to_owned(),
                "*# Boxes".to_owned(),
            ][..]
        ),
        "a project added in a collapsed area shows it again: {added:?}"
    );
    harness.ctrl('w');
    harness.type_text("l");
    let before = harness.rows();
    harness.type_text("'");
    assert_eq!(harness.rows(), before, "the pane has nothing to collapse");
}
