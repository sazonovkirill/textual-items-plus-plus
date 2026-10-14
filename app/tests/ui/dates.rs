//! Start and due dates: the date prompt that `ms` and `md` open, `mt`, the bracket keys that
//! nudge them a day, and the date chips on rows.

use std::{fs, path::PathBuf};

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Position, Rect},
    style::{Color, Modifier},
};
use tempfile::TempDir;

use crate::support::Harness;

/// The app on a file in a temporary folder holding `text`, on an 80 by 24 terminal, and the
/// file's path.
fn on_file(text: &str) -> (TempDir, PathBuf, Harness) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, text).expect("the fixture is written");
    let harness = Harness::open(path.clone(), 80, 24);
    (dir, path, harness)
}

/// Rows `range` of the screen, without trailing spaces.
fn rows(harness: &Harness, range: std::ops::Range<usize>) -> Vec<String> {
    harness
        .rows()
        .get(range)
        .expect("the screen has those rows")
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

/// The focused row, without trailing spaces.
fn focused(harness: &Harness) -> Option<String> {
    harness
        .rows()
        .iter()
        .find(|row| row.starts_with('▌'))
        .map(|row| row.trim_end().to_owned())
}

/// A row of an 80-column pane: `text`, then `chips` right-aligned before the last column, which
/// stays blank.
fn row(text: &str, chips: &str) -> String {
    let pad = 79 - text.chars().count() - chips.chars().count();
    format!("{text}{}{chips}", " ".repeat(pad))
}

/// The date prompt's panel: the four rows above the status line, between the borders' first
/// column and last, without trailing spaces.
fn panel(harness: &Harness) -> Vec<String> {
    rows(harness, 19..23)
        .iter()
        .map(|row| {
            let inner: String = row.chars().skip(1).collect();
            inner
                .trim_end_matches(['│', '┐', '┘'])
                .trim_end()
                .to_owned()
        })
        .collect()
}

// FR-DATE-02, FR-KEY-06, FR-TASK-03
#[test]
fn md_then_fri_sets_the_due_date_shown_in_its_chip() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("md");
    assert_eq!(
        panel(&harness),
        [
            &format!(" Due date {}", "─".repeat(68)),
            " 2026-10-07",
            " Wed 7 Oct 2026",
            &"─".repeat(78),
        ]
    );
    harness.type_text("fri");
    assert_eq!(panel(&harness)[1..3], [" fri", " Fri 9 Oct 2026"]);
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(5, 20))
    );
    harness.key(KeyCode::Enter);
    assert_eq!(
        focused(&harness),
        Some(row("▌ ☐ Call the bank about the card", "󱙬 Fri"))
    );
    assert_eq!(rows(&harness, 19..23), ["", "", "", ""], "the panel closed");
    harness.type_text("z");
    assert_eq!(
        focused(&harness),
        Some(row("▌ ☐ Call the bank about the card", "󱙬 Wed")),
        "one step to undo"
    );
}

// FR-DATE-02, FR-KEY-06
#[test]
fn the_prompt_takes_the_keys_until_enter_or_esc() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("ms");
    assert_eq!(
        panel(&harness)[0],
        format!(" Start date {}", "─".repeat(66))
    );
    assert_eq!(panel(&harness)[2], " No date: Enter removes it");
    harness.type_text("xjdd");
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    assert_eq!(panel(&harness)[1..3], [" xjdd", " Not a date from “xjdd”"]);
    harness.key(KeyCode::Enter);
    assert_eq!(panel(&harness)[1], " xjdd", "a text that is no date stays");
    harness.key(KeyCode::Esc);
    assert_eq!(rows(&harness, 19..23), ["", "", "", ""]);
    assert_eq!(
        rows(&harness, 3..6),
        [
            row("▌ ☐ Call the bank about the card", "󱙬 Wed"),
            "  ☐ Buy coffee beans @errands".to_owned(),
            "  ☐ Read the article Sam sent".to_owned(),
        ],
        "nothing changed"
    );
    harness.type_text("ms");
    harness.send(&Event::Paste("next\nfri".to_owned()));
    assert_eq!(panel(&harness)[1..3], [" next fri", " Fri 16 Oct 2026"]);
    harness.ctrl('g');
    assert!(
        harness.screen().contains("Navigator"),
        "a global key works and closes the prompt"
    );
    assert_eq!(panel(&harness)[1], "");
}

// FR-KEY-06, FR-OPS-06
#[test]
fn mt_starts_the_selection_today_and_both_are_saved() {
    let text = "Inbox:\nTrip:\n\t- Renew the passport @due(2026-11-01)\n\t- Book flights\n";
    let (_dir, path, mut harness) = on_file(text);
    harness.ctrl('g');
    harness.type_text("trip");
    harness.key(KeyCode::Enter);
    harness.type_text("Jmt");
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\nTrip:\n\t- Renew the passport @start(2026-10-05) @due(2026-11-01)\n\
         \t- Book flights @start(2026-10-05)\n"
    );
    assert_eq!(
        rows(&harness, 2..4),
        [
            row("▌ ☐ Renew the passport", "󱙭 today 󱙬 in 27 days"),
            row("▌ ☐ Book flights", "󱙭 today"),
        ]
    );
    harness.key(KeyCode::Esc);
    harness.type_text("md");
    harness.key(KeyCode::Backspace);
    harness.key(KeyCode::Enter);
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\nTrip:\n\t- Renew the passport @start(2026-10-05) @due(2026-11-01)\n\
         \t- Book flights @start(2026-10-05)\n",
        "Book flights has no due date to remove"
    );
    harness.type_text("k");
    harness.type_text("md");
    harness.key(KeyCode::Backspace);
    harness.key(KeyCode::Enter);
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\nTrip:\n\t- Renew the passport @start(2026-10-05)\n\
         \t- Book flights @start(2026-10-05)\n"
    );
}

// FR-DATE-06, FR-OPS-06, FR-KEY-02
#[test]
fn brackets_nudge_the_start_date_and_braces_the_due_date_a_day() {
    let text = "Inbox:\nTrip:\n\t- Renew the passport @due(2026-11-01)\n\t- Book flights\n";
    let (_dir, path, mut harness) = on_file(text);
    harness.ctrl('g');
    harness.type_text("trip");
    harness.key(KeyCode::Enter);
    harness.type_text("]]");
    assert_eq!(
        focused(&harness),
        Some(row("▌ ☐ Renew the passport", "󱙭 Wed 󱙬 in 27 days")),
        "no start date counts from today"
    );
    harness.type_text("[{");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nTrip:\n\t- Renew the passport @start(2026-10-06) @due(2026-10-31)\n\
         \t- Book flights\n"
    );
    harness.type_text("J}");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nTrip:\n\t- Renew the passport @start(2026-10-06) @due(2026-11-01)\n\
         \t- Book flights @due(2026-10-06)\n",
        "every selected item, an undated one from today"
    );
    harness.type_text("z");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nTrip:\n\t- Renew the passport @start(2026-10-06) @due(2026-10-31)\n\
         \t- Book flights\n",
        "one step to undo"
    );
}

// FR-DATE-01, FR-DATE-06
#[test]
fn nudging_an_area_says_it_has_no_dates() {
    let (_dir, path, mut harness) = on_file("Inbox:\nHome: @area\n");
    harness.ctrl('g');
    harness.type_text("home");
    harness.key(KeyCode::Enter);
    harness.type_text("k}");
    let rows = harness.rows();
    assert!(
        rows.last()
            .is_some_and(|row| row.contains("an area has no start or due date")),
        "{rows:#?}"
    );
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nHome: @area\n"
    );
}

/// The file at `path` once a second has passed for the app to save it.
fn saved_after_a_second(harness: &mut Harness, path: &std::path::Path) -> String {
    harness.wait(1000);
    fs::read_to_string(path).expect("the file is there")
}

// FR-DATE-02, FR-DATE-04, FR-DATE-05
#[test]
fn overdue_dates_stand_out_until_completed_and_containers_show_their_chips() {
    let text = "Inbox:\n\
                \t- Late @due(2026-10-04)\n\
                \t- Done late @due(2026-10-01) @done(2026-10-03)\n\
                Work: @start(2026-10-06) @due(2026-10-12)\n";
    let (_dir, _path, mut harness) = on_file(text);
    harness.ctrl('o');
    assert_eq!(
        rows(&harness, 2..4),
        [
            row("▌ ☐ Late", "󰨱 yesterday"),
            row("  ✔ Done late", "󱙬 4 days ago"),
        ]
    );
    let buffer = harness.terminal.backend().buffer();
    let overdue = &buffer[(68, 2)];
    assert_eq!(overdue.symbol(), "󰨱");
    assert_eq!(overdue.fg, Color::LightRed);
    assert!(
        overdue
            .modifier
            .contains(Modifier::BOLD | Modifier::REVERSED)
    );
    let done = &buffer[(67, 3)];
    assert_eq!(done.symbol(), "󱙬");
    assert_eq!(done.fg, Color::Red);
    assert!(done.modifier.contains(Modifier::DIM));
    assert!(!done.modifier.contains(Modifier::REVERSED));
    harness.ctrl('b');
    harness.type_text("jjjjjjj.");
    let heading = harness.rows()[0].trim_end().to_owned();
    assert!(heading.contains("│  # Work "), "{heading}");
    assert!(heading.ends_with(" 󱙭 tomorrow 󱙬 next Mon"), "{heading}");
}
