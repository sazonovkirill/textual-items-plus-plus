//! Completing tasks in the container view, showing the completed ones, and undoing and redoing.

use std::fs;

use crossterm::event::KeyCode;

use crate::support::{Harness, without_chips};

/// The rows of the container's items between the Inbox's title and description and the status
/// line, each without its marker, date chips or trailing spaces, the focused one starred.
fn items(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .skip(3)
        .filter_map(|row| {
            let (focus, rest) = row.split_at(row.char_indices().nth(2)?.0);
            let title = without_chips(rest.split_once(' ')?.1);
            if title.is_empty() {
                return None;
            }
            Some(if focus.starts_with('▌') {
                format!("*{title}")
            } else {
                title.to_owned()
            })
        })
        .collect()
}

const CALL: &str = "Call the bank about the card";
const COFFEE: &str = "Buy coffee beans @errands";
const READ: &str = "Read the article Sam sent";

fn star(title: &str) -> String {
    format!("*{title}")
}

// FR-TASK-02, FR-VIEW-11, FR-EDIT-06
#[test]
fn x_completes_ctrl_o_shows_it_last_and_z_and_shift_z_undo_and_redo() {
    let mut harness = Harness::new(60, 12);
    assert_eq!(items(&harness), [star(CALL), COFFEE.into(), READ.into()]);
    harness.key(KeyCode::Char('x'));
    assert_eq!(items(&harness), [star(COFFEE), READ.into()]);
    harness.ctrl('o');
    assert_eq!(items(&harness), [star(COFFEE), READ.into(), CALL.into()]);
    harness.ctrl('o');
    harness.key(KeyCode::Char('z'));
    assert_eq!(items(&harness), [star(CALL), COFFEE.into(), READ.into()]);
    harness.key(KeyCode::Char('Z'));
    assert_eq!(items(&harness), [star(COFFEE), READ.into()]);
}

// FR-TASK-02, FR-VIEW-11
#[test]
fn x_on_a_shown_completed_task_uncompletes_it_in_place() {
    let mut harness = Harness::new(60, 12);
    harness.ctrl('o');
    harness.key(KeyCode::Char('x'));
    assert_eq!(items(&harness), [COFFEE.into(), READ.into(), star(CALL)]);
    harness.key(KeyCode::Char('x'));
    assert_eq!(items(&harness), [star(CALL), COFFEE.into(), READ.into()]);
}

// FR-TASK-02, FR-ORG-09, D-80
#[test]
fn dot_completes_a_task_a_section_and_a_project_as_x_does() {
    let mut harness = Harness::new(60, 24);
    harness.key(KeyCode::Char('.'));
    assert_eq!(items(&harness), [star(COFFEE), READ.into()]);
    harness.ctrl('g');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    harness.type_text("G.");
    assert!(!harness.screen().contains("# Blog"), "{}", harness.screen());
    harness.type_text("gg}.");
    assert!(
        !harness.screen().contains("Review two portfolios"),
        "{}",
        harness.screen()
    );
    harness.type_text("zz");
    let screen = harness.screen();
    assert!(screen.contains("# Blog") && screen.contains("Review two portfolios"));
}

// FR-ORG-09, FR-VIEW-03, FR-STORE-10, D-81
#[test]
fn an_area_is_not_completed_and_one_its_file_marks_done_stays_open() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\nWork: @area @done(2026-10-01)\n\t- Work out\n";
    fs::write(&path, text).expect("written");
    let mut harness = Harness::open(path.clone(), 60, 12);
    harness.type_text("gL");
    assert!(!harness.screen().contains("Work"), "{}", harness.screen());
    harness.ctrl('g');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    let screen = harness.screen();
    assert!(screen.contains("☐ Work out"), "{screen}");
    harness.ctrl('f');
    harness.type_text("work");
    harness.key(KeyCode::Esc);
    harness.key(KeyCode::Char('x'));
    let status = harness.rows().last().map(|row| row.trim_end().to_owned());
    assert!(
        status.is_some_and(|status| status.ends_with("an area can't be completed")),
        "{}",
        harness.screen()
    );
    harness.ctrl('a');
    harness.key(KeyCode::Char('.'));
    harness.wait(1000);
    let file = fs::read_to_string(&path).expect("the file is there");
    assert!(
        file.starts_with("Inbox:\nWork: @area @done(2026-10-01)\n\t- Work out @done("),
        "only the task completes and the area keeps its tag: {file}"
    );
}
