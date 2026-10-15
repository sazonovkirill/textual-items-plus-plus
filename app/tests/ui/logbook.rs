//! The Logbook: every completed item, by the day it was completed, each with where it is.

use std::fs;

use crate::support::Harness;

/// The rows of the screen, without trailing spaces.
fn trimmed(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

// FR-VIEW-03, FR-KEY-06, FR-TASK-02
#[test]
fn uncomplete_a_task_from_the_logbook() {
    let mut harness = Harness::new(60, 30);
    harness.type_text("gl");
    let mut logbook = vec![
        "  \u{f14f3} Logbook",
        "",
        "  2 days ago · Sat 3 Oct",
        "▌ ✔ Compress the hero image · Work › Website › Polish",
    ];
    logbook.resize(29, "");
    logbook.push(" \u{f01bf} NAVIGATE · Demo data · not saved");
    assert_eq!(trimmed(&harness), logbook);
    harness.ctrl('o');
    assert_eq!(
        trimmed(&harness),
        logbook,
        "the Logbook shows completed items whatever Show Completed says"
    );
    harness.ctrl('o');
    harness.type_text("x");
    let rows = trimmed(&harness);
    assert_eq!(rows[..3], ["  \u{f14f3} Logbook", "", ""], "{rows:#?}");
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(crossterm::event::KeyCode::Enter);
    assert!(
        trimmed(&harness)
            .iter()
            .any(|row| row.ends_with("☐ Compress the hero image")),
        "the task is open again where it was: {:#?}",
        trimmed(&harness)
    );
}

// FR-VIEW-03, FR-ORG-09
#[test]
fn the_logbook_groups_completed_items_by_day_newest_first_with_where_they_are() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "\
Inbox:
\t- Call the bank @done(2026-10-04)
\t- Old note @done
Work:
\t- Write the report @done(2026-10-05)
\tSite: @done(2026-10-03)
\t\t- Hidden inside @done(2026-10-05)
\t- Plan @done(2025-12-31)
";
    fs::write(&path, text).expect("written");
    let mut harness = Harness::open(path.clone(), 50, 20);
    harness.type_text("gL");
    assert_eq!(
        trimmed(&harness)[..16],
        [
            "  \u{f14f3} Logbook",
            "",
            "  Today · Mon 5 Oct",
            "▌ ✔ Write the report · Work",
            "",
            "  Yesterday · Sun 4 Oct",
            "  ✔ Call the bank",
            "",
            "  2 days ago · Sat 3 Oct",
            "  # Site · Work",
            "",
            "  278 days ago · Wed 31 Dec 2025",
            "  ✔ Plan · Work",
            "",
            "  No date",
            "  ✔ Old note",
        ]
    );
    assert_eq!(
        fs::read_to_string(&path).expect("read"),
        text,
        "the Logbook moves nothing"
    );
}
