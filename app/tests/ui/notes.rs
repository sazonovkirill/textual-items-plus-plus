//! Notes in the attachments panel, over the right of the screen, rendered as Markdown and edited
//! as raw text.

use std::fs;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

const SCREEN: Rect = Rect::new(0, 0, 80, 12);

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

/// The mode the status line names.
fn mode(harness: &Harness) -> String {
    let status = harness.rows().last().cloned().unwrap_or_default();
    status
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned()
}

// FR-ATT-01, FR-ATT-02, FR-ATT-03, FR-ATT-05, FR-EDIT-03, FR-EDIT-06
#[test]
fn write_a_note_on_a_project() {
    let mut harness = Harness::new(80, 12);
    harness.ctrl('g');
    harness.type_text("Blog");
    harness.key(KeyCode::Enter);
    harness.press(KeyCode::Char('n'), KeyModifiers::ALT);
    assert_eq!(
        panel(&harness)[..3],
        [
            "┌ Blog ────────────────────────────────┐",
            "│                               󱇗 Note │",
            "│ No note                              │",
        ]
    );
    harness.type_text("e");
    assert_eq!(mode(&harness), "EDIT");
    harness.type_text("# Outline");
    harness.key(KeyCode::Enter);
    harness.type_text("- **intro** see [it](https://a.b)");
    assert_eq!(
        panel(&harness)[1..4],
        [
            "│                               󱇗 Note │",
            "│ # Outline                            │",
            "│ - **intro** see [it](https://a.b)    │",
        ],
        "the note is edited as typed"
    );
    assert_eq!(harness.tui.cursor(SCREEN), Some(Position::new(75, 3)));
    harness.key(KeyCode::Esc);
    assert_eq!(mode(&harness), "NAVIGATE");
    assert_eq!(
        panel(&harness)[1..4],
        [
            "│                               󱇗 Note │",
            "│ Outline                              │",
            "│ • intro see it                       │",
        ],
        "and shown as Markdown"
    );
    assert_eq!(harness.tui.cursor(SCREEN), None);
    harness.key(KeyCode::Esc);
    assert_eq!(panel(&harness)[0], "", "Esc closes the panel");
    harness.type_text("z");
    harness.press(KeyCode::Char('n'), KeyModifiers::ALT);
    assert_eq!(
        panel(&harness)[2],
        "│ No note                              │",
        "the whole edit is one step to undo"
    );
}

// FR-ATT-01, FR-ATT-02
#[test]
fn the_panel_takes_the_keys_until_it_closes() {
    let mut harness = Harness::new(80, 12);
    let before = harness.rows();
    harness.type_text("n");
    assert!(panel(&harness)[0].starts_with("┌ Call the bank about the card"));
    harness.type_text("jxdd");
    harness.ctrl('g');
    harness.key(KeyCode::Esc);
    assert_eq!(harness.rows(), before, "nothing behind it moved or changed");
    harness.type_text("n");
    harness.ctrl('q');
    assert!(harness.tui.should_quit(), "quitting still quits");
}

// FR-ATT-01
#[test]
fn a_long_note_scrolls() {
    let mut harness = Harness::new(80, 12);
    harness.type_text("ne");
    for n in 1..=12 {
        harness.type_text(&format!("line {n}"));
        harness.key(KeyCode::Enter);
    }
    harness.type_text("end");
    assert_eq!(
        panel(&harness)[7],
        "│ end                                  │",
        "the caret stays in view"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("g");
    assert_eq!(
        panel(&harness)[2],
        "│ line 1                               │"
    );
    harness.type_text("jj");
    assert_eq!(
        panel(&harness)[1],
        "│ line 2                               │"
    );
    harness.ctrl('d');
    assert_eq!(
        panel(&harness)[1],
        "│ line 9                               │"
    );
    harness.ctrl('d');
    assert_eq!(
        panel(&harness)[1],
        "│ line 10                              │",
        "a page down stops at the end"
    );
    harness.type_text("gG");
    assert_eq!(
        panel(&harness)[4..8],
        [
            "│ end                                  │",
            "│                                      │",
            "│                           󰆉 Comments │",
            "│ No comments                          │",
        ]
    );
    harness.key(KeyCode::Esc);
    harness.type_text("n");
    assert_eq!(
        panel(&harness)[1..3],
        [
            "│                               󱇗 Note │",
            "│ line 1                               │",
        ],
        "it opens again at the top"
    );
}

// FR-ATT-01, FR-VIEW-10
#[test]
fn an_area_gets_its_note_from_the_sidebar() {
    let mut harness = Harness::new(80, 12);
    harness.ctrl('g');
    harness.type_text("Home");
    harness.key(KeyCode::Enter);
    harness.type_text("Mn");
    let title = panel(&harness)[0].clone();
    assert!(title.starts_with("┌ Home ─"), "{title}");
    harness.type_text("eHome matters");
    harness.key(KeyCode::Esc);
    assert_eq!(
        panel(&harness)[2],
        "│ Home matters                         │"
    );
}

// FR-ATT-03, FR-OPS-05
#[test]
fn text_pasted_goes_into_the_note_edited_with_its_lines() {
    let mut harness = Harness::new(80, 12);
    let before = harness.rows();
    harness.type_text("n");
    harness.send(&crossterm::event::Event::Paste("- x\n".to_owned()));
    harness.key(KeyCode::Esc);
    assert_eq!(
        harness.rows(),
        before,
        "nothing pastes while the note is not edited"
    );
    harness.type_text("ne");
    harness.send(&crossterm::event::Event::Paste("one\ntwo\n".to_owned()));
    assert_eq!(
        panel(&harness)[2..4],
        [
            "│ one                                  │",
            "│ two                                  │",
        ]
    );
}

// FR-STORE-05, FR-ATT-03, FR-ATT-05
#[test]
fn a_note_is_saved_as_guarded_lines_and_reads_back() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- Renew the passport\n").expect("the fixture is written");
    let mut harness = Harness::open(path.clone(), 80, 12);
    harness.type_text("ne");
    harness.type_text("- bring photos");
    harness.key(KeyCode::Enter);
    harness.type_text("Steps:  ");
    harness.key(KeyCode::Enter);
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\n\t- Renew the passport\n",
        "nothing is saved while the note is edited"
    );
    harness.key(KeyCode::Esc);
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\n\t- Renew the passport\n\t\tNOTE @hidden\n\t\t\t\u{200B}- bring photos\n\t\t\tSteps:\u{200B}\n",
        "space and empty lines at the end are dropped"
    );
    let mut reopened = Harness::open(path, 80, 12);
    reopened.type_text("n");
    assert_eq!(
        panel(&reopened)[2..4],
        [
            "│ • bring photos                       │",
            "│ Steps:                               │",
        ]
    );
}
