//! Moving items up and down, from one section into the next, and indenting and outdenting them.

use std::{fs, path::PathBuf};

use crossterm::event::{KeyCode, KeyModifiers};
use tempfile::TempDir;

use crate::support::{Harness, without_chips};

/// The rows below the breadcrumbs that show a title, trimmed and without date chips, the focused
/// ones starred.
fn listed(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .skip(2)
        .filter_map(|row| {
            let focused = row.starts_with('▌');
            let text = without_chips(row.trim_start_matches('▌').trim());
            let title = text.strip_prefix("- ").unwrap_or(text);
            if title.is_empty() {
                return None;
            }
            Some(if focused {
                format!("*{title}")
            } else {
                title.to_owned()
            })
        })
        .collect()
}

/// The status line, without trailing spaces.
fn status(harness: &Harness) -> String {
    harness
        .rows()
        .last()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

fn alt(harness: &mut Harness, code: KeyCode) {
    harness.press(code, KeyModifiers::ALT);
}

/// Opens Work › Website of the demo, focused on `row`.
fn website(row: usize) -> Harness {
    let mut harness = Harness::new(70, 24);
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
    for _ in 0..row {
        harness.key(KeyCode::Char('j'));
    }
    harness
}

// FR-OPS-02, FR-OPS-10, FR-TASK-05, FR-EDIT-06
#[test]
fn move_a_task_from_one_section_into_the_next() {
    let mut harness = website(3);
    assert!(listed(&harness).contains(&"*Book the launch call @meeting".to_owned()));
    alt(&mut harness, KeyCode::Char('j'));
    let moved = listed(&harness);
    let polish = moved.iter().position(|row| row.ends_with("Polish"));
    let book = moved
        .iter()
        .position(|row| row == "*Book the launch call @meeting");
    let footer = moved.iter().position(|row| row == "Fix the footer links");
    assert!(
        polish < book && book < footer,
        "first in the next section: {moved:?}"
    );
    alt(&mut harness, KeyCode::Up);
    let back = listed(&harness);
    let polish = back.iter().position(|row| row.ends_with("Polish"));
    let book = back
        .iter()
        .position(|row| row == "*Book the launch call @meeting");
    assert!(book < polish, "last in the section above: {back:?}");
    alt(&mut harness, KeyCode::Down);
    harness.key(KeyCode::Char('z'));
    assert_eq!(listed(&harness), back, "undo takes the move back");
}

// FR-OPS-10, FR-ORG-06
#[test]
fn sections_move_among_the_sections() {
    let mut harness = website(1);
    let before = listed(&harness);
    assert!(
        before
            .iter()
            .any(|row| row.ends_with("Launch") && row.starts_with('*'))
    );
    alt(&mut harness, KeyCode::Char('k'));
    assert_eq!(
        listed(&harness),
        before,
        "never above the tasks outside sections"
    );
    alt(&mut harness, KeyCode::Char('j'));
    let after = listed(&harness);
    let polish = after.iter().position(|row| row.ends_with("Polish"));
    let launch = after.iter().position(|row| row.ends_with("Launch"));
    let blog = after.iter().position(|row| row.ends_with("Blog"));
    assert!(polish < launch && launch < blog, "{after:?}");
}

// FR-OPS-03
#[test]
fn a_task_or_a_section_is_not_indented_and_the_status_line_says_why() {
    let mut harness = website(0);
    harness.key(KeyCode::Tab);
    assert!(
        status(&harness).ends_with("only projects and checklist items nest"),
        "{}",
        status(&harness)
    );
    harness.key(KeyCode::Char('j'));
    assert!(
        status(&harness).ends_with("not saved"),
        "the next command clears it"
    );
    harness.key(KeyCode::Char('['));
    assert!(status(&harness).ends_with("only projects and checklist items nest"));
}

const PLANS: &str = "\
Inbox:
Plans: @area
\tGarden:
\tKitchen:
\t\t- Paint the walls
\tNext: @section
\t\t- Order paint
\tLater: @section
\t\t- Call the plumber
";

/// The app on a file holding [`PLANS`], opened on the area, and the file's path.
fn plans() -> (TempDir, PathBuf, Harness) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, PLANS).expect("the fixture is written");
    let mut harness = Harness::open(path.clone(), 70, 20);
    harness.ctrl('g');
    harness.type_text("plans");
    harness.key(KeyCode::Enter);
    (dir, path, harness)
}

fn saved(harness: &mut Harness, path: &PathBuf) -> String {
    harness.wait(1000);
    fs::read_to_string(path).expect("the file is there")
}

// FR-OPS-10, FR-STORE-09
#[test]
fn a_task_moved_into_the_next_section_is_saved_there() {
    let (_dir, path, mut harness) = plans();
    harness.key(KeyCode::Char('j'));
    alt(&mut harness, KeyCode::Down);
    assert_eq!(
        saved(&mut harness, &path),
        "\
Inbox:
Plans: @area
\tGarden:
\tKitchen:
\t\t- Paint the walls
\tNext: @section
\tLater: @section
\t\t- Order paint
\t\t- Call the plumber
"
    );
}

// FR-OPS-03, FR-ORG-02, FR-ORG-06
#[test]
fn a_project_indents_into_the_project_above_and_outdents_out_of_it() {
    let (_dir, path, mut harness) = plans();
    harness.key(KeyCode::Char('G'));
    harness.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert!(status(&harness).ends_with("nothing to outdent from"));
    harness.key(KeyCode::Char(']'));
    let indented = listed(&harness);
    assert!(
        indented
            .iter()
            .any(|row| row.ends_with("Garden") && row.starts_with('*'))
    );
    assert!(!indented.iter().any(|row| row.ends_with("Kitchen")));
    let file = saved(&mut harness, &path);
    assert!(
        file.contains("\tGarden:\n\t\tKitchen:\n\t\t\t- Paint the walls\n"),
        "{file}"
    );
    harness.key(KeyCode::Char('.'));
    harness.key(KeyCode::Char(']'));
    assert!(status(&harness).ends_with("nothing above to indent into"));
    harness.key(KeyCode::Char('['));
    assert!(!listed(&harness).iter().any(|row| row.ends_with("Kitchen")));
    assert_eq!(saved(&mut harness, &path), PLANS, "back where it was");
}
