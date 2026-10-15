//! Selecting several items and acting on all of them at once.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::support::{Harness, without_chips};

/// The titles of the Inbox's items between its title and description and the status line,
/// without their date chips, a row marked as focused or picked starred.
fn items(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .skip(3)
        .filter_map(|row| {
            let (marker, rest) = row.split_at(row.char_indices().nth(2)?.0);
            let title = without_chips(rest.split_once(' ')?.1);
            if title.is_empty() {
                return None;
            }
            Some(if marker.starts_with('▌') {
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

// FR-OPS-01, FR-OPS-06, FR-TASK-02, FR-EDIT-06
#[test]
fn shift_and_a_motion_pick_three_tasks_that_x_completes_at_once() {
    let mut harness = Harness::new(60, 12);
    harness.press(KeyCode::Down, KeyModifiers::SHIFT);
    harness.press(KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(items(&harness), [star(CALL), star(COFFEE), star(READ)]);
    harness.key(KeyCode::Char('K'));
    assert_eq!(items(&harness), [star(CALL), star(COFFEE), READ.into()]);
    harness.key(KeyCode::Char('J'));
    harness.key(KeyCode::Char('x'));
    assert_eq!(items(&harness), Vec::<String>::new());
    harness.ctrl('o');
    assert_eq!(
        items(&harness),
        ["Items".into(), star(CALL), COFFEE.into(), READ.into()],
        "under the Completed Items heading, in file order, completed the same day"
    );
    harness.ctrl('o');
    harness.key(KeyCode::Char('z'));
    assert_eq!(
        items(&harness),
        [star(CALL), star(COFFEE), star(READ)],
        "undo picks them again"
    );
}

// FR-OPS-01, FR-OPS-06, FR-TASK-02, D-82
#[test]
fn shift_v_picks_lines_as_j_and_k_move_until_a_command_acts_on_them() {
    let mut harness = Harness::new(60, 12);
    let status = |harness: &Harness| harness.rows().last().cloned().unwrap_or_default();
    harness.type_text("Vj");
    assert_eq!(items(&harness), [star(CALL), star(COFFEE), READ.into()]);
    assert!(
        status(&harness).contains("VISUAL LINE"),
        "{}",
        status(&harness)
    );
    harness.type_text("jk");
    assert_eq!(
        items(&harness),
        [star(CALL), star(COFFEE), READ.into()],
        "moving back shrinks the lines"
    );
    harness.type_text("jx");
    assert_eq!(
        items(&harness),
        Vec::<String>::new(),
        "x completes them all"
    );
    assert!(
        status(&harness).contains("NAVIGATE"),
        "{}",
        status(&harness)
    );
    harness.key(KeyCode::Char('z'));
    harness.type_text("VkV");
    assert!(status(&harness).contains("NAVIGATE"), "V again leaves it");
    harness.type_text("Vk");
    harness.key(KeyCode::Esc);
    assert!(status(&harness).contains("NAVIGATE"), "so does Esc");
    assert_eq!(
        items(&harness)
            .iter()
            .filter(|item| item.starts_with('*'))
            .count(),
        1,
        "Esc picks nothing: {:?}",
        items(&harness)
    );
}

// FR-OPS-01, FR-OPS-02
#[test]
fn v_picks_rows_one_by_one_alt_j_moves_them_and_esc_lets_go() {
    let mut harness = Harness::new(60, 12);
    harness.key(KeyCode::Char('v'));
    harness.type_text("jj");
    assert_eq!(
        items(&harness),
        [star(CALL), COFFEE.into(), star(READ)],
        "plain motions keep the picks"
    );
    harness.key(KeyCode::Char('v'));
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    assert_eq!(
        items(&harness),
        [COFFEE.into(), star(CALL), star(READ)],
        "the last stays and the first moves"
    );
    harness.key(KeyCode::Esc);
    harness.key(KeyCode::Char('x'));
    assert_eq!(
        items(&harness),
        [COFFEE.into(), star(CALL)],
        "only the focused row"
    );
}

// FR-OPS-01, FR-OPS-05, FR-OPS-06
#[test]
fn ctrl_a_picks_every_row_and_dd_deletes_them_all() {
    let mut harness = Harness::new(60, 12);
    harness.ctrl('a');
    assert_eq!(items(&harness), [star(CALL), star(COFFEE), star(READ)]);
    harness.type_text("dd");
    assert_eq!(items(&harness), Vec::<String>::new());
    harness.key(KeyCode::Char('z'));
    assert_eq!(items(&harness), [star(CALL), star(COFFEE), star(READ)]);
}
