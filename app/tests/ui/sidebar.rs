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
    harness.type_text("jjjjjjj");
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
            " 󰈻 Flagged",
            " 󰉹 All",
            " 󱓳 Logbook",
            " 󰩺 Trash",
            "*# Learn Rust           2",
            " 󰆧 WORK                 7",
            " # Website              4",
            " # Blog                 1",
            " 󰆧 HOME                 5",
            " # Move to the new flat 4",
        ]
    );
    let rows = harness.rows();
    let blank = |row: usize| {
        rows.get(row)
            .is_some_and(|row| row.chars().take(25).all(|c| c == ' '))
    };
    assert!(
        blank(1) && blank(8) && blank(10) && blank(14),
        "blank lines set the groups off"
    );
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    assert_eq!(
        entries(&harness).get(7).map(String::as_str),
        Some("*# Learn Rust           2"),
        "no other project in no area to change places with"
    );
    harness.type_text("Gk");
    harness.press(KeyCode::Char('k'), KeyModifiers::ALT);
    let moved = entries(&harness);
    assert_eq!(
        moved.get(8..11),
        Some(
            &[
                "*󰆧 HOME                 5".to_owned(),
                " # Move to the new flat 4".to_owned(),
                " 󰆧 WORK                 7".to_owned(),
            ][..]
        ),
        "{moved:?}"
    );
    harness.type_text("z");
    assert_eq!(
        entries(&harness).get(11).map(String::as_str),
        Some("*󰆧 HOME                 5")
    );
    harness.ctrl('b');
    assert!(
        harness.rows()[0].starts_with("  󰆧 Home"),
        "hidden, the pane takes the screen, showing the area last focused: {:?}",
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
        Some(Position::new(3, 9)),
        "the caret in the new entry, first of the projects in no area"
    );
    harness.type_text("Garden");
    harness.key(KeyCode::Enter);
    let added = entries(&harness);
    assert_eq!(
        added.get(7).map(String::as_str),
        Some("*# Garden"),
        "{added:?}"
    );
    assert!(
        harness.rows()[0].contains("󰚇 Inbox"),
        "the pane stays on the Inbox"
    );
    harness.type_text("R");
    harness.type_text("s");
    harness.key(KeyCode::Enter);
    assert_eq!(
        entries(&harness).get(7).map(String::as_str),
        Some("*# Gardens")
    );
    harness.type_text("A");
    harness.type_text("Errands");
    harness.key(KeyCode::Enter);
    assert_eq!(
        entries(&harness).get(9).map(String::as_str),
        Some("*󰆧 ERRANDS"),
        "first of the areas"
    );
    harness.key(KeyCode::Enter);
    let title: String = harness.rows()[0].chars().skip(28).collect();
    assert!(title.contains("Errands"), "Enter opens it: {title:?}");
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
        entries(&harness).get(7).map(String::as_str),
        Some("*# Learn Rust           2")
    );
}

// FR-VIEW-10, FR-OPS-05, FR-EDIT-06
#[test]
fn delete_the_project_shown_from_the_sidebar() {
    let mut harness = sidebar();
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
    assert!(harness.rows()[0].contains("󰚇 Inbox"), "the pane goes home");
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
        entries(&harness).get(7).map(String::as_str),
        Some("*# Learn Rust           2")
    );
    harness.type_text("k");
    assert_eq!(pane_focus(&harness), None);
    assert_eq!(entries(&harness)[6], "*󰩺 Trash");
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
        entries(&harness).get(8..11),
        Some(
            &[
                "*󰆧 WORK               ▸ 7".to_owned(),
                " 󰆧 HOME                 5".to_owned(),
                " # Move to the new flat 4".to_owned(),
            ][..]
        ),
        "the projects inside it hide"
    );
    harness.type_text("j'");
    assert_eq!(
        entries(&harness).get(9).map(String::as_str),
        Some("*󰆧 HOME               ▸ 5")
    );
    harness.type_text("k'");
    assert_eq!(
        entries(&harness).get(9).map(String::as_str),
        Some(" # Website              4")
    );
    harness.type_text("G");
    harness.type_text("iBoxes");
    harness.key(KeyCode::Enter);
    let added = entries(&harness);
    assert_eq!(
        added.get(11..14),
        Some(
            &[
                " 󰆧 HOME                 5".to_owned(),
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

/// The columns where the focus marker shows, on any row.
fn marker_columns(harness: &Harness) -> Vec<usize> {
    let mut columns: Vec<usize> = harness
        .rows()
        .iter()
        .flat_map(|row| {
            row.chars()
                .enumerate()
                .filter_map(|(column, c)| (c == '▌').then_some(column))
                .collect::<Vec<_>>()
        })
        .collect();
    columns.dedup();
    columns
}

/// The title line of the pane starting at `column`, up to the next rule.
fn header(harness: &Harness, column: usize) -> String {
    let rows = harness.rows();
    let row: String = rows
        .first()
        .map(|row| row.chars().skip(column).collect())
        .unwrap_or_default();
    row.split('│').next().unwrap_or_default().trim().to_owned()
}

// FR-VIEW-10, FR-PANE-02, D-79
#[test]
fn the_app_starts_with_the_sidebar_shown_and_the_keys_in_the_first_pane() {
    let mut harness = Harness::launched(120, 24);
    assert!(
        entries(&harness)[0].starts_with(" 󰚇 Inbox"),
        "the sidebar shows, without the keys"
    );
    assert_eq!(marker_columns(&harness), [28], "the pane has them");
    harness.type_text("j");
    assert!(
        harness
            .rows()
            .iter()
            .any(|row| row.contains("│▌ ☐ Buy coffee")),
        "the pane moves: {}",
        harness.screen()
    );
}

// FR-PANE-02, FR-VIEW-10, D-79
#[test]
fn tab_gn_gp_and_g_digits_go_round_the_panes_the_sidebar_first() {
    let mut harness = Harness::launched(120, 24);
    harness.ctrl('\\');
    let (sidebar, first, second) = (0, 28, 74);
    assert_eq!(marker_columns(&harness), [second]);
    for (keys, column) in [
        ("\t", sidebar),
        ("\t", first),
        ("\t", second),
        ("gn", sidebar),
        ("gp", second),
        ("gp", first),
        ("gp", sidebar),
        ("g2", second),
        ("g0", sidebar),
        ("g1", first),
    ] {
        if keys == "\t" {
            harness.key(KeyCode::Tab);
        } else {
            harness.type_text(keys);
        }
        assert_eq!(marker_columns(&harness), [column], "{keys:?}");
    }
    harness.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(marker_columns(&harness), [sidebar], "Shift+Tab");
    harness.ctrl('b');
    harness.key(KeyCode::Tab);
    harness.key(KeyCode::Tab);
    assert_eq!(
        marker_columns(&harness),
        [0],
        "without the sidebar Tab goes from the last pane to the first"
    );
    harness.type_text("g0");
    assert!(
        entries(&harness)[0].starts_with("*󰚇 Inbox"),
        "g0 shows the sidebar with the keys"
    );
}

// FR-VIEW-10, FR-PANE-02, D-79
#[test]
fn gj_gk_and_ctrl_arrows_browse_the_sidebar_in_the_first_pane_from_anywhere() {
    let mut harness = Harness::launched(120, 24);
    harness.ctrl('\\');
    harness.type_text("gj");
    assert!(header(&harness, 28).contains("Today"));
    assert!(
        header(&harness, 74).contains("Inbox"),
        "the other pane stays"
    );
    assert_eq!(marker_columns(&harness), [74], "and keeps the keys");
    harness.press(KeyCode::Down, KeyModifiers::CONTROL);
    assert!(header(&harness, 28).contains("Upcoming"));
    harness.type_text("gk");
    harness.press(KeyCode::Up, KeyModifiers::CONTROL);
    assert!(header(&harness, 28).contains("Inbox"));
    harness.type_text("g0G");
    assert!(
        header(&harness, 28).contains("Move to the new flat"),
        "moving in the sidebar shows its entry too: {}",
        header(&harness, 28)
    );
    assert_eq!(marker_columns(&harness), [0], "the sidebar keeps the keys");
}
