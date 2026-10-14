//! Completing tasks in the container view, showing the completed ones, and undoing and redoing.

use crossterm::event::KeyCode;

use crate::support::{Harness, without_chips};

/// The rows of the container's items between the Inbox's title and description and the status
/// line, each without its marker, date chips or trailing spaces, the focused one starred.
fn items(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .skip(5)
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
