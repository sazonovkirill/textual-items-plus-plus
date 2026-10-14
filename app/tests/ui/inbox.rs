//! The Inbox view: the Inbox's tasks without the scheduled ones, which stay under the Inbox in the
//! file.

use std::fs;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::support::Harness;

/// The rows below the breadcrumbs that show a task, trimmed.
fn tasks(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .filter_map(|row| {
            let row = row.trim_start_matches('▌').trim();
            row.strip_prefix("- ").map(str::to_owned)
        })
        .collect()
}

// FR-VIEW-01
#[test]
fn the_demos_scheduled_inbox_task_is_not_in_the_inbox_view() {
    let harness = Harness::new(80, 24);
    assert_eq!(
        tasks(&harness),
        [
            "Call the bank about the card",
            "Buy coffee beans @errands",
            "Read the article Sam sent"
        ]
    );
}

// FR-VIEW-01, FR-KEY-06, FR-OPS-02
#[test]
fn moving_past_a_scheduled_task_keeps_it_under_the_inbox_and_gi_comes_back() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\n\t- Renew the passport\n\t- Plan the trip @start(2026-10-20)\n\t- Book flights\nWork:\n\t- Report\n";
    fs::write(&path, text).expect("the fixture is written");
    let mut harness = Harness::open(path.clone(), 70, 20);
    assert_eq!(tasks(&harness), ["Renew the passport", "Book flights"]);
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    assert_eq!(tasks(&harness), ["Book flights", "Renew the passport"]);
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\n\t- Plan the trip @start(2026-10-20)\n\t- Book flights\n\t- Renew the passport\nWork:\n\t- Report\n"
    );
    harness.ctrl('g');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    assert_eq!(tasks(&harness), ["Report"]);
    harness.type_text("gi");
    assert_eq!(tasks(&harness), ["Book flights", "Renew the passport"]);
}
