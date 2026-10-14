//! Pinned projects: pinning and unpinning a project, which the sidebar and the Navigator then
//! list again after the views.

use std::fs;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

/// The sidebar's rows below the views on an 80-column screen, trimmed at the right, the focus
/// marker written `*`.
fn below_the_views(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .skip(9)
        .map(|row| {
            let sidebar: String = row.chars().take(25).collect();
            match sidebar.strip_prefix('▌') {
                Some(text) => format!("*{}", text.trim_end()),
                None => sidebar.trim_end().to_owned(),
            }
        })
        .collect()
}

// FR-VIEW-12, FR-VIEW-10
#[test]
fn pin_projects_from_the_sidebar_and_rename_and_unpin_one_from_its_pinned_entry() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('b');
    harness.type_text("jjjjjjjmp");
    harness.type_text("jjmp");
    assert_eq!(
        below_the_views(&harness).get(..7),
        Some(
            &[
                " # Website              4",
                " # Learn Rust           2",
                "",
                " # Learn Rust           2",
                "",
                " 󰆧 WORK                 7",
                "*# Website              4",
            ]
            .map(String::from)[..]
        ),
        "after the views, in the order of the file, set off by a blank line"
    );
    harness.type_text("kkk");
    harness.type_text("R");
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(13, 10)),
        "the title edited in the entry focused"
    );
    harness.type_text("!");
    harness.key(KeyCode::Enter);
    harness.type_text("mp");
    assert_eq!(
        below_the_views(&harness).get(..5),
        Some(
            &[
                " # Website              4",
                "",
                "*# Learn Rust!          2",
                "",
                " 󰆧 WORK                 7",
            ]
            .map(String::from)[..]
        ),
        "unpinned, the focus goes to the entry now where it was"
    );
}

/// The Navigator's entries, each row trimmed at the right.
fn navigator(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    let query = rows.iter().position(|row| row.starts_with("│›"));
    rows.iter()
        .skip(query.map_or(rows.len(), |query| query + 1))
        .take_while(|row| row.starts_with('│'))
        .map(|row| row.trim_end().to_owned())
        .collect()
}

// FR-VIEW-12, FR-VIEW-07, FR-UI-07
#[test]
fn only_a_project_is_pinned_and_the_navigator_lists_it_again_until_a_query_is_typed() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('b');
    harness.type_text("jjjjjjjjjmp");
    harness.ctrl('b');
    harness.type_text("gamp");
    let status = harness.rows().last().cloned().unwrap_or_default();
    assert!(
        status.contains("only a project is pinned"),
        "a task: {status:?}"
    );
    harness.ctrl('k');
    let listed = navigator(&harness);
    assert_eq!(
        listed.get(6..11).unwrap_or_default(),
        [
            "│ 󰩺 Trash                                                                      │",
            "│ # Website                                                                  4 │",
            "│ 󰆧 Work                                                                     7 │",
            "│ # Website                                                                  4 │",
            "│ # Blog                                                                     1 │",
        ],
        "after the views"
    );
    harness.type_text("site");
    assert_eq!(
        navigator(&harness),
        ["│ # Website  Work                                                            4 │"],
        "listed once"
    );
}

// FR-VIEW-12, D-107, FR-UI-07
#[test]
fn a_pinned_entry_moves_past_the_pinned_project_beside_it_only_when_it_is_a_sibling() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let file = "Inbox:\nA: @pinned\nB:\nC: @pinned\nWork: @area\n\tD: @pinned\n";
    fs::write(&path, file).expect("the file is written");
    let mut harness = Harness::open(path.clone(), 80, 24);
    harness.ctrl('b');
    harness.type_text("jjjjjjj");
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    assert_eq!(
        below_the_views(&harness).get(..3).unwrap_or_default(),
        [" # C", "*# A", " # D"],
        "past C, not the unpinned B between them"
    );
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    assert_eq!(
        below_the_views(&harness).get(..3).unwrap_or_default(),
        [" # C", "*# A", " # D"]
    );
    let status = harness.rows().last().cloned().unwrap_or_default();
    assert!(
        status.ends_with("from Pinned it moves only past a pinned sibling"),
        "D is in an area: {status:?}"
    );
    harness.press(KeyCode::Char('k'), KeyModifiers::ALT);
    assert_eq!(
        below_the_views(&harness).get(..3).unwrap_or_default(),
        ["*# A", " # C", " # D"]
    );
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    harness.terminate();
    assert_eq!(
        fs::read_to_string(&path).expect("the file is read"),
        "Inbox:\nB:\nC: @pinned\nA: @pinned\nWork: @area\n\tD: @pinned\n",
        "the tree changed with the group"
    );
}
