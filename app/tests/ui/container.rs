//! The container view: starting on the demo Inbox or a file's, moving through it with the keys
//! and the status line under it.

use std::{fmt::Write, fs};

use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::support::Harness;

/// The row the focus marker is on, without the marker and trailing spaces.
fn focused(harness: &Harness) -> Option<String> {
    harness
        .rows()
        .iter()
        .find_map(|row| row.strip_prefix("▌ ").map(|row| row.trim_end().to_owned()))
}

/// The status line, without trailing spaces.
fn status(harness: &Harness) -> String {
    harness
        .rows()
        .last()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

/// The app on a file in a temporary folder holding `text`.
fn on_file(text: &str, width: u16, height: u16) -> (tempfile::TempDir, Harness) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, text).expect("the fixture is written");
    let harness = Harness::open(path, width, height);
    (dir, harness)
}

// FR-STORE-01, FR-VIEW-05, FR-TASK-03, FR-TAG-03, FR-ORG-10, FR-UI-01, FR-UI-02, FR-UI-06, D-29,
// FR-DATE-02
#[test]
fn without_a_file_the_app_shows_the_demo_inbox_at_80_by_24() {
    let harness = Harness::new(80, 24);
    let mut expected = vec![
        "  󰋜 Inbox".to_owned(),
        "─".repeat(80),
        "  󰚇 Inbox".to_owned(),
        "  Everything not sorted yet lands here.".to_owned(),
        String::new(),
        "▌ - Call the bank about the card                                          󱙬 Wed"
            .to_owned(),
        "  - Buy coffee beans @errands".to_owned(),
        "  - Read the article Sam sent".to_owned(),
    ];
    expected.resize(23, String::new());
    expected.push(" 󰆿 NAVIGATE · Demo data · not saved".to_owned());
    let expected: Vec<String> = expected.iter().map(|row| format!("{row:80}")).collect();
    assert_eq!(harness.rows(), expected);
}

// FR-NAV-01
#[test]
fn j_k_and_the_arrows_move_between_rows_and_stop_at_the_ends() {
    let mut harness = Harness::new(80, 24);
    let call = "- Call the bank about the card                                          󱙬 Wed";
    let coffee = "- Buy coffee beans @errands";
    let read = "- Read the article Sam sent";
    harness.type_text("j");
    assert_eq!(focused(&harness).as_deref(), Some(coffee));
    harness.key(KeyCode::Down);
    assert_eq!(focused(&harness).as_deref(), Some(read));
    harness.type_text("j");
    assert_eq!(
        focused(&harness).as_deref(),
        Some(read),
        "j stops at the last row"
    );
    harness.type_text("k");
    assert_eq!(focused(&harness).as_deref(), Some(coffee));
    harness.key(KeyCode::Up);
    harness.key(KeyCode::Up);
    assert_eq!(
        focused(&harness).as_deref(),
        Some(call),
        "↑ stops at the first row"
    );
}

// FR-NAV-01, FR-KEY-02
#[test]
fn gg_g_home_and_end_go_to_the_first_and_last_rows() {
    let mut harness = Harness::new(80, 24);
    let first = "- Call the bank about the card                                          󱙬 Wed";
    let last = "- Read the article Sam sent";
    harness.type_text("G");
    assert_eq!(focused(&harness).as_deref(), Some(last));
    harness.type_text("g");
    assert_eq!(focused(&harness).as_deref(), Some(last), "g alone waits");
    assert!(status(&harness).ends_with(" g"), "{}", status(&harness));
    harness.type_text("g");
    assert_eq!(focused(&harness).as_deref(), Some(first));
    assert!(
        status(&harness).ends_with("not saved"),
        "{}",
        status(&harness)
    );
    harness.key(KeyCode::End);
    assert_eq!(focused(&harness).as_deref(), Some(last));
    harness.key(KeyCode::Home);
    assert_eq!(focused(&harness).as_deref(), Some(first));
}

// FR-NAV-01
#[test]
fn a_key_after_g_that_continues_nothing_cancels_it() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("gx");
    assert!(
        status(&harness).ends_with("not saved"),
        "{}",
        status(&harness)
    );
    harness.type_text("j");
    assert_eq!(
        focused(&harness).as_deref(),
        Some("- Buy coffee beans @errands"),
        "the next key acts alone"
    );
}

// FR-NAV-01, FR-VIEW-05
#[test]
fn section_motions_move_between_headings_and_to_the_section_of_a_task() {
    // The Inbox has no sections, so the motions that go to one leave the focus where it is.
    let mut harness = Harness::new(80, 24);
    harness.type_text("j");
    let coffee = focused(&harness);
    harness.type_text("#");
    harness.type_text("gj");
    harness.type_text("gk");
    harness.press(KeyCode::Down, KeyModifiers::CONTROL);
    harness.press(KeyCode::Up, KeyModifiers::CONTROL);
    assert_eq!(focused(&harness), coffee);
}

// FR-STORE-01, FR-VIEW-05
#[test]
fn a_file_opens_on_its_inbox_saved() {
    let text = "Inbox:\n\t- Renew the passport @due(2026-11-01)\n\t- Book flights @travel\nWork: @area\n\t- Not in the Inbox\n";
    let (dir, mut harness) = on_file(text, 60, 8);
    let path = dir.path().join("todo.taskpaper");
    assert_eq!(
        harness
            .rows()
            .iter()
            .map(|row| row.trim_end())
            .collect::<Vec<_>>(),
        [
            "  󰋜 Inbox",
            "─".repeat(60).as_str(),
            "  󰚇 Inbox",
            "",
            "▌ - Renew the passport                         󱙬 in 27 days",
            "  - Book flights @travel",
            "",
            format!(" 󰆿 NAVIGATE · {} · saved", path.display()).as_str(),
        ]
    );
    harness.type_text("j");
    assert_eq!(focused(&harness).as_deref(), Some("- Book flights @travel"));
    assert_eq!(fs::read_to_string(&path).expect("the file is there"), text);
}

// FR-STORE-01, F-FILE-01
#[test]
fn a_missing_file_opens_empty_and_is_not_created_until_a_change() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("new.taskpaper");
    let mut harness = Harness::open(path.clone(), 40, 4);
    harness.type_text("jkG");
    assert_eq!(harness.rows()[2].trim_end(), "  󰚇 Inbox");
    assert_eq!(focused(&harness), None);
    assert!(!path.exists());
}

// F-FILE-07
#[test]
fn a_file_that_is_not_utf8_does_not_open() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("binary.taskpaper");
    fs::write(&path, b"\xff\xfe").expect("the fixture is written");
    let error = Harness::on(Some(path), None, 40, 4)
        .err()
        .expect("not UTF-8");
    assert!(error.to_string().contains("not valid UTF-8"), "{error}");
}

// FR-UI-02, FR-UI-06
#[test]
fn moving_past_the_bottom_scrolls_the_focus_into_view() {
    let mut text = "Inbox:\n".to_owned();
    for n in 1..=30 {
        writeln!(text, "\t- Task {n}").expect("a string takes any text");
    }
    let (_dir, mut harness) = on_file(&text, 40, 10);
    harness.type_text("G");
    let rows = harness.rows();
    assert_eq!(rows[8].trim_end(), "▌ - Task 30");
    assert_eq!(rows[2].trim_end(), "  - Task 24");
    harness.type_text("gg");
    assert_eq!(harness.rows()[2].trim_end(), "  󰚇 Inbox");
}

// FR-UI-06
#[test]
fn a_small_terminal_shows_what_fits_and_keeps_working() {
    let mut harness = Harness::new(24, 6);
    assert_eq!(
        harness
            .rows()
            .iter()
            .map(|row| row.trim_end())
            .collect::<Vec<_>>(),
        [
            "  󰋜 Inbox",
            "─".repeat(24).as_str(),
            "▌ - Call the bank 󱙬 Wed",
            "▌   about the",
            "▌   card",
            " 󰆿 NAVIGATE · not saved",
        ]
    );
    harness.type_text("G");
    assert_eq!(focused(&harness).as_deref(), Some("- Read the article Sam"));
    harness.send(&Event::Resize(10, 2));
    harness.terminal.backend_mut().resize(10, 2);
    harness.draw();
    assert_eq!(harness.rows()[0], "▌ - Read  ");
}
