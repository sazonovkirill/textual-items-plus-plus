//! Getting around: the Navigator, opening containers, going up and home, the breadcrumbs, and
//! going back and forward.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use std::fs;

use crate::support::Harness;

/// The breadcrumb line, without trailing spaces.
fn crumbs(harness: &Harness) -> String {
    harness
        .rows()
        .first()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

/// The row the focus marker is on, without the marker and trailing spaces.
fn focused(harness: &Harness) -> Option<String> {
    harness
        .rows()
        .iter()
        .find_map(|row| row.strip_prefix("▌ ").map(|row| row.trim_end().to_owned()))
}

/// The rows of the Navigator's window, between its borders, without trailing spaces.
fn navigator(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .skip_while(|row| !row.starts_with("┌Navigator"))
        .skip(1)
        .take_while(|row| !row.starts_with('└'))
        .map(|row| {
            row.trim_start_matches('│')
                .trim_end()
                .trim_end_matches('│')
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// Opens Work › Website through the Navigator, as the plan's "Try it" does.
fn open_website(harness: &mut Harness) {
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
}

// FR-VIEW-07, FR-UI-09
#[test]
fn the_navigator_lists_the_inbox_areas_and_projects_as_a_tree_with_open_task_counts() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('g');
    assert_eq!(
        navigator(&harness),
        [
            "›",
            " 󰚇 Inbox                                                                    3",
            " 󰆧 Work                                                                     7",
            "   󰉖 Website                                                                5",
            "     󰉖 Blog                                                                 1",
            " 󰆧 Home                                                                     5",
            "   󰉖 Move to the new flat                                                   4",
            " 󰉖 Learn Rust                                                               2",
        ]
    );
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(3, 8)),
        "the cursor stands in the query"
    );
}

// FR-VIEW-07, FR-UI-09, FR-NAV-02, FR-NAV-03, FR-NAV-04
#[test]
fn open_work_website_from_the_navigator_and_come_back() {
    let mut harness = Harness::new(80, 24);
    assert_eq!(crumbs(&harness), "  󰋜 Inbox");
    harness.ctrl('g');
    harness.type_text("web");
    assert_eq!(
        navigator(&harness),
        [
            "› web",
            " 󰉖 Website  Work                                                            5"
        ],
        "a filtered entry names the places above it"
    );
    harness.key(KeyCode::Enter);
    assert_eq!(
        navigator(&harness),
        Vec::<String>::new(),
        "choosing closes it"
    );
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website");
    assert_eq!(
        harness.rows()[2].trim_end(),
        "  󰉖 Website                                                               󱇗 󰆉 2"
    );
    assert_eq!(
        focused(&harness).as_deref(),
        Some("- Collect logos from partners")
    );
    harness.type_text("H");
    assert_eq!(crumbs(&harness), "  󰋜 Inbox");
    assert_eq!(
        focused(&harness).as_deref(),
        Some("- Call the bank about the card")
    );
    harness.type_text("L");
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website");
    assert_eq!(harness.tui.cursor(Rect::new(0, 0, 80, 24)), None);
}

// FR-VIEW-07
#[test]
fn the_navigator_opens_on_where_the_pane_is_and_its_key_or_escape_closes_it() {
    let mut harness = Harness::new(80, 24);
    open_website(&mut harness);
    harness.type_text("jj#");
    assert_eq!(focused(&harness).as_deref(), Some("󰚟 Launch"));
    harness.type_text(".");
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website › Launch");
    harness.ctrl('g');
    let entries = navigator(&harness);
    assert!(entries.iter().any(|entry| entry.contains("Website")));
    harness.key(KeyCode::Enter);
    assert_eq!(
        crumbs(&harness),
        "  󰋜 Work › Website",
        "a section opens the Navigator on its project"
    );
    harness.ctrl('g');
    harness.type_text("j");
    assert_eq!(navigator(&harness).first().map(String::as_str), Some("› j"));
    harness.key(KeyCode::Esc);
    assert_eq!(navigator(&harness), Vec::<String>::new());
    harness.ctrl('g');
    harness.ctrl('g');
    assert_eq!(navigator(&harness), Vec::<String>::new());
    assert_eq!(
        crumbs(&harness),
        "  󰋜 Work › Website",
        "closing goes nowhere"
    );
}

// FR-VIEW-07
#[test]
fn the_navigator_keeps_the_keys_but_ctrl_q_still_quits() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('g');
    harness.type_text("gh");
    assert_eq!(
        navigator(&harness).first().map(String::as_str),
        Some("› gh")
    );
    harness.press(KeyCode::Left, KeyModifiers::ALT);
    assert_eq!(crumbs(&harness), "  󰋜 Inbox");
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
}

// FR-NAV-02, FR-NAV-03
#[test]
fn opening_going_up_and_home_with_the_keys() {
    let mut harness = Harness::new(80, 24);
    open_website(&mut harness);
    harness.type_text("G");
    assert_eq!(focused(&harness).as_deref(), Some("󰉖 Blog"));
    harness.press(KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website › Blog");
    harness.type_text(">");
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website");
    assert_eq!(
        focused(&harness).as_deref(),
        Some("󰉖 Blog"),
        "going up focuses where the pane was"
    );
    harness.type_text("gu");
    assert_eq!(crumbs(&harness), "  󰋜 Work");
    assert!(focused(&harness).is_some_and(|row| row.starts_with("󰉖 Website ")));
    harness.type_text("gu");
    assert_eq!(
        crumbs(&harness),
        "  󰋜 Work",
        "the top level has nothing above"
    );
    harness.type_text("gz");
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website");
    harness.type_text("gg.");
    assert_eq!(
        crumbs(&harness),
        "  󰋜 Work › Website",
        "a task does not open"
    );
    harness.type_text("gh");
    assert_eq!(crumbs(&harness), "  󰋜 Inbox");
    harness.press(KeyCode::Left, KeyModifiers::ALT | KeyModifiers::SHIFT);
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website");
    harness.type_text("gi");
    assert_eq!(crumbs(&harness), "  󰋜 Inbox");
}

// FR-NAV-04
#[test]
fn back_and_forward_return_with_the_focus_where_it_was() {
    let mut harness = Harness::new(80, 24);
    open_website(&mut harness);
    harness.type_text("jj#.");
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website › Launch");
    harness.type_text("H");
    assert_eq!(focused(&harness).as_deref(), Some("󰚟 Launch"));
    harness.type_text("H");
    assert_eq!(crumbs(&harness), "  󰋜 Inbox");
    harness.type_text("H");
    assert_eq!(crumbs(&harness), "  󰋜 Inbox", "nothing is further back");
    harness.press(KeyCode::Right, KeyModifiers::ALT | KeyModifiers::SHIFT);
    harness.type_text("L");
    assert_eq!(crumbs(&harness), "  󰋜 Work › Website › Launch");
    harness.type_text("L");
    assert_eq!(
        crumbs(&harness),
        "  󰋜 Work › Website › Launch",
        "nothing is further ahead"
    );
}

// FR-NAV-03
#[test]
fn breadcrumbs_too_wide_give_way_from_the_start() {
    let mut harness = Harness::new(24, 12);
    open_website(&mut harness);
    harness.type_text("G.");
    assert_eq!(crumbs(&harness), "  󰋜 … › Website › Blog");
    let mut harness = Harness::new(14, 12);
    open_website(&mut harness);
    harness.type_text("G.");
    assert_eq!(crumbs(&harness), "  󰋜 … › Blog");
    let mut harness = Harness::new(9, 12);
    open_website(&mut harness);
    assert_eq!(crumbs(&harness), "  󰋜 … › W");
}

// FR-VIEW-07
#[test]
fn a_place_without_open_tasks_shows_no_count() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- One\nSomeday:\n\t- Done @done\n").expect("written");
    let mut harness = Harness::open(path, 40, 12);
    harness.ctrl('g');
    assert_eq!(
        navigator(&harness),
        ["›", " 󰚇 Inbox                            1", " 󰉖 Someday"]
    );
}
