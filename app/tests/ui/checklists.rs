//! A task's checklist in the attachments panel, built and changed with the keys rows take.

use std::fs;

use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::support::Harness;

/// The rows of the panel's content, the right half of the screen between the panel's title and
/// its hint, without trailing spaces.
fn panel(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 4)
        .skip(1)
        .map(|row| {
            row.chars()
                .skip(40)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// The status line, without trailing spaces.
fn status(harness: &Harness) -> String {
    harness
        .rows()
        .last()
        .cloned()
        .unwrap_or_default()
        .trim_end()
        .to_owned()
}

// FR-CHK-01, FR-CHK-02, FR-CHK-03, FR-ATT-02, FR-ATT-05, FR-TASK-03
#[test]
fn build_a_nested_checklist() {
    let mut harness = Harness::new(80, 20);
    harness.type_text("c");
    assert_eq!(
        panel(&harness)[..5],
        [
            "│                               \u{f11d7} Note │",
            "│ No note                              │",
            "│                                      │",
            "│                          \u{f0756} Checklist │",
            "│ No checklist                         │",
        ]
    );
    harness.type_text("o");
    harness.type_text("Documents");
    harness.key(KeyCode::Enter);
    harness.type_text("ID card");
    harness.key(KeyCode::Esc);
    harness.type_text("]");
    harness.type_text("o");
    harness.type_text("Proof of address");
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Esc);
    harness.type_text("kx");
    assert_eq!(
        panel(&harness)[4..8],
        [
            "│ - Documents                          │",
            "│   ✔ ID card                          │",
            "│   - Proof of address                 │",
            "│                                      │",
        ],
        "Enter adds the next item, ] nests it, Esc on an empty one adds none, x checks"
    );
    harness.send(&Event::Paste(
        "Call\n\tAsk about fees\nThank them\n".to_owned(),
    ));
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    assert_eq!(
        panel(&harness)[4..10],
        [
            "│ - Documents                          │",
            "│   ✔ ID card                          │",
            "│   - Call                             │",
            "│     - Ask about fees                 │",
            "│   - Proof of address                 │",
            "│   - Thank them                       │",
        ],
        "a paste adds an item for each line, nested by its indentation, and Alt+j moves one down"
    );
    harness.type_text("d");
    assert_eq!(
        panel(&harness)[8],
        "│   - Proof of address                 │"
    );
    harness.key(KeyCode::Esc);
    assert!(
        harness.rows()[5].ends_with("\u{f0756} 1/5 \u{f166c} Wed "),
        "the row shows how much of its checklist is done: {}",
        harness.rows()[5]
    );
    harness.type_text("z");
    assert!(
        harness.rows()[5].contains("\u{f0756} 1/6"),
        "deleting the item was one step to undo"
    );
}

// FR-ATT-02
#[test]
fn only_a_task_has_a_checklist() {
    let mut harness = Harness::new(80, 16);
    harness.ctrl('g');
    harness.type_text("Website");
    harness.key(KeyCode::Enter);
    harness.type_text("jc");
    assert!(status(&harness).ends_with("only a task has a checklist"));
    assert!(!harness.rows()[0].contains('┌'), "no panel opens");
}

// FR-CHK-04, FR-STORE-06
#[test]
fn a_checked_item_is_saved_done_under_its_task() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- Pack\n").expect("the file is written");
    let mut harness = Harness::open(path.clone(), 80, 16);
    harness.type_text("co");
    harness.type_text("Books");
    harness.key(KeyCode::Esc);
    harness.type_text("x");
    harness.key(KeyCode::Esc);
    harness.terminate();
    let saved = fs::read_to_string(&path).expect("the file is read");
    assert!(
        saved.starts_with("Inbox:\n\t- Pack\n\t\t- Books @done("),
        "{saved}"
    );
}

// FR-CHK-01, FR-OPS-05
#[test]
fn items_copied_paste_into_a_checklist_one_for_each_line() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- Pack\n\t- Buy tape\n\t\tfor the boxes\n")
        .expect("the file is written");
    let mut harness = Harness::open(path, 80, 16);
    harness.type_text("jykc");
    harness.ctrl('v');
    assert_eq!(
        panel(&harness)[4..6],
        [
            "│ - Buy tape                           │",
            "│   - for the boxes                    │",
        ],
        "the copied task and its note line arrive as checklist items"
    );
}
