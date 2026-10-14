//! Comments in the attachments panel, below the note: stamped one-line remarks, newest first,
//! added, edited and deleted.

use std::{fs, path::PathBuf};

use crossterm::event::{Event, KeyCode, KeyModifiers};
use tempfile::TempDir;

use crate::support::Harness;

/// The rows of the panel, the right half of the screen above the status line, without trailing
/// spaces.
fn panel(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .map(|row| {
            row.chars()
                .skip(40)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// A file holding `text` in a folder of its own, which goes when it is dropped.
fn file(text: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, text).expect("the fixture is written");
    (dir, path)
}

fn read(path: &PathBuf) -> String {
    fs::read_to_string(path).expect("the file is there")
}

// FR-ATT-02, FR-ATT-04, FR-ATT-05, FR-STORE-05
#[test]
fn comment_on_a_task() {
    let (_dir, path) = file("Inbox:\n\t- Renew the passport\n");
    let mut harness = Harness::open(path.clone(), 80, 16);
    harness.type_text("N");
    assert_eq!(
        panel(&harness)[1..9],
        [
            "│ 󱇗 Note                               │",
            "│ No note                              │",
            "│                                      │",
            "│ \u{f0756} Checklist                          │",
            "│ No checklist                         │",
            "│                                      │",
            "│ 󰆉 Comments                           │",
            "│▌No comments                          │",
        ],
        "a task's checklist comes between, and its comments, none yet, are selected"
    );
    assert_eq!(
        panel(&harness)[13],
        "│ a comment · \\ full · Esc close       │"
    );
    harness.type_text("aPhotos: done");
    assert_eq!(
        panel(&harness)[8..10],
        [
            "│▌Oct 5 09:30                          │",
            "│▌Photos: done                         │",
        ],
        "stamped with the time it was added"
    );
    assert_eq!(
        panel(&harness)[13],
        "│ Enter done · Esc cancel              │"
    );
    harness.wait(1000);
    assert_eq!(
        read(&path),
        "Inbox:\n\t- Renew the passport\n",
        "nothing is saved while it is typed"
    );
    harness.key(KeyCode::Enter);
    harness.type_text("aBooked");
    harness.key(KeyCode::Enter);
    assert_eq!(
        panel(&harness)[8..10],
        [
            "│▌Oct 5 09:30 Booked                   │",
            "│ Oct 5 09:30 Photos: done             │",
        ],
        "newest first"
    );
    harness.type_text("je for the form");
    harness.key(KeyCode::Enter);
    harness.wait(1000);
    assert_eq!(
        read(&path),
        "Inbox:\n\t- Renew the passport\n\t\tCOMMENTS @hidden\n\t\t\t[2026-10-05 09:30] Booked\n\t\t\t[2026-10-05 09:30] Photos: done for the form\n"
    );
    harness.type_text("kd");
    assert_eq!(
        panel(&harness)[8..10],
        [
            "│▌Oct 5 09:30 Photos: done for the     │",
            "│▌form                                 │",
        ]
    );
    harness.key(KeyCode::Esc);
    harness.ctrl('z');
    harness.type_text("N");
    assert_eq!(
        panel(&harness)[8],
        "│▌Oct 5 09:30 Booked                   │",
        "deleting is a step to undo, once the panel is closed"
    );
    harness.key(KeyCode::Esc);
    let mut reopened = Harness::open(path, 80, 16);
    reopened.type_text("N");
    assert_eq!(
        panel(&reopened)[8],
        "│▌Oct 5 09:30 Booked                   │"
    );
}

// FR-ATT-04
#[test]
fn escape_cancels_a_comment_and_a_paste_goes_in_on_one_line() {
    let mut harness = Harness::new(80, 16);
    harness.type_text("N");
    let before = harness.rows();
    harness.type_text("aon second thought");
    harness.key(KeyCode::Esc);
    assert_eq!(harness.rows(), before, "nothing was added");
    harness.type_text("a");
    harness.send(&Event::Paste("one\ntwo\r\nthree\n".to_owned()));
    harness.key(KeyCode::Enter);
    assert_eq!(
        panel(&harness)[8],
        "│▌Oct 5 09:30 one two three            │"
    );
}

// FR-ATT-02, FR-VIEW-10
#[test]
fn the_container_shown_and_a_sidebar_entry_open_their_comments() {
    let mut harness = Harness::new(80, 12);
    harness.press(KeyCode::Char('N'), KeyModifiers::ALT | KeyModifiers::SHIFT);
    assert!(panel(&harness)[0].starts_with("┌ Inbox ─"));
    harness.key(KeyCode::Esc);
    harness.ctrl('g');
    harness.type_text("Home");
    harness.key(KeyCode::Enter);
    harness.ctrl('b');
    harness.type_text("N");
    let title = panel(&harness)[0].clone();
    assert!(title.starts_with("┌ Home ─"), "{title}");
}
