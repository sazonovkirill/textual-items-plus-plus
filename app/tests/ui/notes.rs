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

// FR-ATT-02, FR-ATT-03
#[test]
fn a_long_list_item_in_a_note_wraps_under_its_text() {
    let mut harness = Harness::new(80, 12);
    harness.ctrl('g');
    harness.type_text("Blog");
    harness.key(KeyCode::Enter);
    harness.press(KeyCode::Char('\\'), KeyModifiers::ALT);
    harness.type_text("e- a list item long enough to wrap onto a second line");
    assert_eq!(
        panel(&harness)[2..4],
        [
            "│▌- a list item long enough to wrap    │",
            "│▌onto a second line                   │",
        ],
        "as typed while it is edited"
    );
    harness.key(KeyCode::Esc);
    assert_eq!(
        panel(&harness)[2..4],
        [
            "│▌• a list item long enough to wrap    │",
            "│▌  onto a second line                 │",
        ]
    );
    assert_eq!(
        panel(&harness)[8..10],
        [
            "│ e edit · a comment · \\ full          │",
            "│ Esc close                            │"
        ]
    );
}

// FR-ATT-01, FR-ATT-02, FR-ATT-03, FR-ATT-05, FR-EDIT-03, FR-EDIT-06
#[test]
fn write_a_note_on_a_project() {
    let mut harness = Harness::new(80, 12);
    harness.ctrl('g');
    harness.type_text("Blog");
    harness.key(KeyCode::Enter);
    harness.press(KeyCode::Char('\\'), KeyModifiers::ALT);
    assert_eq!(
        panel(&harness)[..3],
        [
            "┌ Blog ────────────────────────────────┐",
            "│ 󱇗 Note                               │",
            "│▌No note                              │",
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
            "│ 󱇗 Note                               │",
            "│▌# Outline                            │",
            "│▌- **intro** see [it](https://a.b)    │",
        ],
        "the note is edited as typed"
    );
    assert_eq!(harness.tui.cursor(SCREEN), Some(Position::new(75, 3)));
    harness.key(KeyCode::Esc);
    assert_eq!(mode(&harness), "ATTACHMENTS", "the panel has the keys");
    assert_eq!(
        panel(&harness)[1..4],
        [
            "│ 󱇗 Note                               │",
            "│▌Outline                              │",
            "│▌• intro see it                       │",
        ],
        "and shown as Markdown"
    );
    assert_eq!(harness.tui.cursor(SCREEN), None);
    harness.key(KeyCode::Esc);
    assert!(!harness.screen().contains("┌ Blog"), "Esc closes the panel");
    harness.type_text("z");
    harness.press(KeyCode::Char('\\'), KeyModifiers::ALT);
    assert_eq!(
        panel(&harness)[2],
        "│▌No note                              │",
        "the whole edit is one step to undo"
    );
}

// FR-ATT-01, FR-ATT-02
#[test]
fn the_panel_takes_the_keys_until_it_closes() {
    let mut harness = Harness::new(80, 12);
    let before = harness.rows();
    harness.type_text("\\");
    assert!(panel(&harness)[0].starts_with("┌ Call the bank about the card"));
    harness.type_text("jxdd");
    harness.ctrl('g');
    harness.key(KeyCode::Esc);
    assert_eq!(harness.rows(), before, "nothing behind it moved or changed");
    harness.type_text("\\");
    harness.ctrl('q');
    assert!(harness.tui.should_quit(), "quitting still quits");
}

// FR-ATT-01
#[test]
fn a_long_note_scrolls() {
    let mut harness = Harness::new(80, 12);
    harness.type_text("\\e");
    for n in 1..=12 {
        harness.type_text(&format!("line {n}"));
        harness.key(KeyCode::Enter);
    }
    harness.type_text("end");
    assert_eq!(
        panel(&harness)[7],
        "│▌end                                  │",
        "the caret stays in view"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("g");
    assert_eq!(
        panel(&harness)[2],
        "│▌line 1                               │"
    );
    harness.type_text("jj");
    assert_eq!(
        panel(&harness)[1],
        "│▌line 2                               │"
    );
    harness.ctrl('d');
    assert_eq!(
        panel(&harness)[1],
        "│▌line 8                               │"
    );
    harness.ctrl('d');
    assert_eq!(
        panel(&harness)[1],
        "│▌line 9                               │",
        "a page down stops at the note's end"
    );
    harness.ctrl('d');
    assert_eq!(
        panel(&harness)[1..7],
        [
            "│                                      │",
            "│ \u{f0756} Checklist                          │",
            "│▌No checklist                         │",
            "│                                      │",
            "│ 󰆉 Comments                           │",
            "│ No comments                          │",
        ],
        "and the next goes on to the task's empty checklist"
    );
    harness.type_text("gG");
    assert_eq!(
        panel(&harness)[3..7],
        [
            "│▌No checklist                         │",
            "│                                      │",
            "│ 󰆉 Comments                           │",
            "│ No comments                          │",
        ]
    );
    harness.key(KeyCode::Esc);
    harness.type_text("\\");
    assert_eq!(
        panel(&harness)[1..3],
        [
            "│ 󱇗 Note                               │",
            "│▌line 1                               │",
        ],
        "it opens again at the top"
    );
}

// FR-ATT-01, FR-VIEW-10
#[test]
fn a_top_level_project_gets_its_note_from_the_sidebar() {
    let mut harness = Harness::new(80, 12);
    harness.ctrl('g');
    harness.type_text("Home");
    harness.key(KeyCode::Enter);
    harness.ctrl('b');
    let marked = |harness: &Harness| harness.rows().iter().any(|row| row.starts_with('▌'));
    assert!(marked(&harness), "the sidebar marks its focus");
    harness.type_text("\\");
    let title = panel(&harness)[0].clone();
    assert!(title.starts_with("┌ Home ─"), "{title}");
    assert!(!marked(&harness), "but not while the panel has the keys");
    harness.type_text("eHome matters");
    harness.key(KeyCode::Esc);
    assert_eq!(
        panel(&harness)[2],
        "│▌Home matters                         │"
    );
}

// FR-ATT-03, FR-OPS-05
#[test]
fn text_pasted_goes_into_the_note_edited_with_its_lines() {
    let mut harness = Harness::new(80, 12);
    let before = harness.rows();
    harness.type_text("\\");
    harness.send(&crossterm::event::Event::Paste("- x\n".to_owned()));
    harness.key(KeyCode::Esc);
    assert_eq!(
        harness.rows(),
        before,
        "nothing pastes while the note is not edited"
    );
    harness.type_text("\\e");
    harness.send(&crossterm::event::Event::Paste("one\ntwo\n".to_owned()));
    assert_eq!(
        panel(&harness)[2..4],
        [
            "│▌one                                  │",
            "│▌two                                  │",
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
    harness.type_text("\\e");
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
    reopened.type_text("\\");
    assert_eq!(
        panel(&reopened)[2..4],
        [
            "│▌• bring photos                       │",
            "│▌Steps:                               │",
        ]
    );
}

// FR-ATT-01, FR-ATT-02
#[test]
fn backslash_widens_the_panel_over_the_panes_beside_the_sidebar_and_back() {
    let mut harness = Harness::new(100, 12);
    let corner = |harness: &Harness| harness.rows()[0].chars().position(|c| c == '┌');
    harness.ctrl('b');
    harness.type_text("\\");
    assert_eq!(corner(&harness), Some(50), "it opens over half the screen");
    harness.type_text("\\");
    assert_eq!(
        corner(&harness),
        Some(28),
        "over the panes, beside the sidebar"
    );
    let rows = harness.rows();
    assert!(rows[2].starts_with(" 󰖙 Today"), "the sidebar stays");
    assert!(rows[9].contains("│ e edit · a comment · \\ half · Esc close  "));
    harness.type_text("eA note as wide as the panel is now, past half the screen");
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 100, 12)),
        Some(Position::new(86, 2)),
        "the note wraps in the panel's width"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("\\");
    assert_eq!(corner(&harness), Some(50), "and back to half");
    harness.type_text("\\");
    harness.key(KeyCode::Esc);
    harness.type_text("\\");
    assert_eq!(corner(&harness), Some(50), "it opens at half again");
}
