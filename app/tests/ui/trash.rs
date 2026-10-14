//! The Trash view: deleted items, latest first, restored, deleted for good and emptied.

use crate::support::Harness;

/// The rows of the screen, without trailing spaces.
fn trimmed(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

/// The screen of the Trash listing `rows` in a 60 by 12 terminal, with `status` at the end of
/// the status line.
fn trash(rows: &[&str], status: &str) -> Vec<String> {
    let mut screen: Vec<String> = [
        "  \u{f0a7a} Trash",
        "────────────────────────────────────────────────────────────",
        "  \u{f0a7a} Trash",
    ]
    .into_iter()
    .chain((!rows.is_empty()).then_some(""))
    .chain(rows.iter().copied())
    .map(str::to_owned)
    .collect();
    screen.resize(11, String::new());
    screen.push(format!(" \u{f01bf} NAVIGATE · {status}"));
    screen
}

const SAVED: &str = "Demo data · not saved";

// FR-VIEW-04, FR-KEY-06
#[test]
fn open_the_trash() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("gd");
    assert_eq!(
        trimmed(&harness),
        trash(
            &[
                "▌ - Cancel the newspaper · Inbox",
                "  - Paint the hallway · Home › Move to the new flat",
            ],
            SAVED
        ),
        "the latest deleted first, each with where it was"
    );
}

// FR-VIEW-04
#[test]
fn restore_a_deleted_task() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("gdu");
    assert_eq!(
        trimmed(&harness),
        trash(
            &["▌ - Paint the hallway · Home › Move to the new flat"],
            SAVED
        )
    );
    harness.type_text("gi");
    assert!(
        harness.screen().contains("- Cancel the newspaper"),
        "back in the Inbox:\n{}",
        harness.screen()
    );
}

// FR-VIEW-04, FR-EDIT-06
#[test]
fn deleting_in_the_trash_deletes_for_good_and_undo_brings_it_back() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("gddd");
    assert_eq!(
        trimmed(&harness),
        trash(
            &["▌ - Paint the hallway · Home › Move to the new flat"],
            SAVED
        )
    );
    harness.type_text("gi");
    assert!(!harness.screen().contains("Cancel the newspaper"));
    harness.type_text("gdz");
    assert!(harness.screen().contains("▌ - Cancel the newspaper"));
}

// FR-VIEW-04, FR-EDIT-06
#[test]
fn empty_the_trash_after_confirming() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("gdde");
    let asked = "… · press d e again to delete 2 items for good";
    assert_eq!(
        trimmed(&harness),
        trash(
            &[
                "▌ - Cancel the newspaper · Inbox",
                "  - Paint the hallway · Home › Move to the new flat",
            ],
            asked
        ),
        "the first press only asks"
    );
    harness.type_text("jde");
    assert!(
        harness.screen().contains("Paint the hallway"),
        "another key in between asks again"
    );
    harness.type_text("de");
    assert_eq!(trimmed(&harness), trash(&[], SAVED));
    harness.type_text("de");
    assert_eq!(trimmed(&harness), trash(&[], SAVED), "nothing to ask about");
    harness.type_text("z");
    assert!(
        harness.screen().contains("Paint the hallway"),
        "undo puts all back"
    );
}
