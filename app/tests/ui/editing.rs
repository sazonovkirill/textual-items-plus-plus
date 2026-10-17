//! Editing titles in place, adding tasks below and above, and deleting to the Trash.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use crate::support::{Harness, without_chips};

const SCREEN: Rect = Rect::new(0, 0, 60, 12);

/// The rows of the Inbox's tasks, each without its marker, date chips or trailing spaces, the
/// focused one starred.
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

/// The mode the status line names.
fn mode(harness: &Harness) -> String {
    let status = harness.rows().last().cloned().unwrap_or_default();
    status
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned()
}

const CALL: &str = "Call the bank about the card";
const COFFEE: &str = "Buy coffee beans @errands";
const READ: &str = "Read the article Sam sent";

// FR-EDIT-01, FR-EDIT-02, FR-EDIT-05, FR-TASK-01
#[test]
fn add_and_rename_a_task() {
    let mut harness = Harness::new(60, 12);
    harness.key(KeyCode::Char('o'));
    assert_eq!(
        mode(&harness),
        "EDIT",
        "o adds a task beside the focused one at once"
    );
    harness.type_text("Pay rent");
    assert_eq!(
        items(&harness),
        [CALL, "*Pay rent", COFFEE, READ].map(String::from)
    );
    assert_eq!(
        harness.tui.cursor(SCREEN),
        Some(Position::new(12, 4)),
        "the cursor stands after the text typed"
    );
    harness.key(KeyCode::Enter);
    assert_eq!(
        mode(&harness),
        "NAVIGATE",
        "Enter only ends editing (D-123)"
    );
    assert_eq!(harness.tui.cursor(SCREEN), None);
    harness.key(KeyCode::Char('o'));
    harness.type_text("Water plants");
    harness.key(KeyCode::Esc);
    harness.key(KeyCode::Char('o'));
    harness.key(KeyCode::Enter);
    assert_eq!(mode(&harness), "NAVIGATE");
    assert_eq!(
        items(&harness),
        [CALL, "Pay rent", "*Water plants", COFFEE, READ].map(String::from),
        "the task left empty is not added"
    );
    harness.key(KeyCode::Char('k'));
    harness.key(KeyCode::Enter);
    harness.ctrl('w');
    harness.type_text("the bills");
    harness.key(KeyCode::Esc);
    assert_eq!(
        items(&harness),
        [CALL, "*Pay the bills", "Water plants", COFFEE, READ].map(String::from)
    );
    harness.type_text("z");
    assert_eq!(
        items(&harness),
        [CALL, "*Pay rent", "Water plants", COFFEE, READ].map(String::from),
        "renaming is one step to undo"
    );
}

// FR-EDIT-01, D-84
#[test]
fn backspace_in_an_empty_title_leaves_editing_as_escape_does() {
    let mut harness = Harness::new(60, 12);
    let inbox = [format!("*{CALL}"), COFFEE.into(), READ.into()];
    harness.key(KeyCode::Char('o'));
    harness.key(KeyCode::Backspace);
    assert_eq!(mode(&harness), "NAVIGATE");
    assert_eq!(items(&harness), inbox, "the new task is not added");
    harness.type_text("i");
    harness.type_text("x");
    harness.key(KeyCode::Backspace);
    harness.ctrl('u');
    assert_eq!(mode(&harness), "EDIT", "a title with text loses it");
    harness.key(KeyCode::Backspace);
    assert_eq!(mode(&harness), "NAVIGATE");
    assert_eq!(items(&harness), inbox, "the title emptied is kept");
}

// FR-EDIT-04, D-93
#[test]
fn the_keys_macos_terminals_send_for_option_and_cmd_edit_a_title() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("i");
    harness.ctrl('a');
    harness.press(KeyCode::Char('d'), KeyModifiers::ALT);
    harness.type_text("Ring");
    harness.ctrl('e');
    harness.press(KeyCode::Left, KeyModifiers::ALT);
    harness.press(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::ALT);
    harness.type_text("the debit ");
    harness.press(KeyCode::Right, KeyModifiers::ALT);
    harness.type_text(" today");
    assert_eq!(
        items(&harness),
        ["*Ring the bank about the debit card today", COFFEE, READ].map(String::from)
    );
    harness.ctrl('u');
    harness.type_text("Done");
    harness.key(KeyCode::Esc);
    assert_eq!(
        items(&harness),
        ["*Done", COFFEE, READ].map(String::from),
        "Ctrl+U deletes back to the start of the line"
    );
}

// FR-EDIT-05
#[test]
fn shift_o_adds_above_and_keys_type_while_editing() {
    let mut harness = Harness::new(60, 12);
    harness.key(KeyCode::Char('j'));
    harness.key(KeyCode::Char('O'));
    harness.type_text("jxdd zo");
    harness.key(KeyCode::Esc);
    assert_eq!(
        items(&harness),
        [CALL, "*jxdd zo", COFFEE, READ].map(String::from)
    );
}

// FR-EDIT-01, D-40, D-127
#[test]
fn space_adds_a_task_below_as_o_does_and_a_adds_none() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("nanA");
    assert_eq!(
        mode(&harness),
        "NAVIGATE",
        "the pane leaves areas to the sidebar"
    );
    harness.key(KeyCode::Char(' '));
    harness.type_text("Next");
    harness.key(KeyCode::Esc);
    assert_eq!(
        items(&harness),
        [CALL, "*Next", COFFEE, READ].map(String::from)
    );
    harness.ctrl('p');
    harness.type_text("new task at top");
    harness.key(KeyCode::Enter);
    harness.type_text("First");
    harness.key(KeyCode::Esc);
    assert_eq!(
        items(&harness),
        ["*First", CALL, "Next", COFFEE, READ].map(String::from),
        "the palette still adds a task at either end"
    );
}

// FR-OPS-05, OQ-19
#[test]
fn dd_and_delete_move_tasks_to_the_trash_and_undo_brings_them_back() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("dd");
    assert_eq!(items(&harness), [format!("*{COFFEE}"), READ.into()]);
    harness.key(KeyCode::Delete);
    assert_eq!(items(&harness), [format!("*{READ}")]);
    harness.type_text("zz");
    assert_eq!(
        items(&harness),
        [format!("*{CALL}"), COFFEE.into(), READ.into()]
    );
}

// FR-EDIT-01
#[test]
fn ctrl_q_quits_while_editing() {
    let mut harness = Harness::new(60, 12);
    harness.key(KeyCode::Enter);
    harness.press(KeyCode::Char('q'), KeyModifiers::NONE);
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
    assert_eq!(mode(&harness), "NAVIGATE");
    assert_eq!(items(&harness).first(), Some(&format!("*{CALL}q")));
}

// FR-UI-06
#[test]
fn a_terminal_too_narrow_for_the_title_hides_the_cursor() {
    let mut harness = Harness::new(4, 12);
    harness.type_text("o");
    assert_eq!(harness.tui.cursor(Rect::new(0, 0, 4, 12)), None);
    harness.terminal.backend_mut().resize(6, 12);
    harness.draw();
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 6, 12)),
        Some(Position::new(4, 5))
    );
}
