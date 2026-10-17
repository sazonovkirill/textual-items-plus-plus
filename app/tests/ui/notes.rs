//! Notes in the attachments panel, over the right of the screen, rendered as Markdown and edited
//! as raw text.

use std::fs;

use crossterm::event::{Event, KeyCode, KeyModifiers};
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
    harness.type_text("gg");
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
            "│ line 12                              │",
            "│ end                                  │",
            "│                                      │",
            "│ \u{f0756} Checklist                          │",
            "│▌No checklist                         │",
            "│                                      │",
        ],
        "and the next goes on to the task's empty checklist"
    );
    harness.type_text("ggG");
    assert_eq!(
        panel(&harness)[3..8],
        [
            "│ \u{f0756} Checklist                          │",
            "│ No checklist                         │",
            "│                                      │",
            "│ 󰆉 Comments                           │",
            "│▌No comments                          │",
        ],
        "the last is the empty comments"
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
fn backslash_goes_round_closed_half_and_full_beside_the_sidebar() {
    let mut harness = Harness::new(100, 12);
    let corner = |harness: &Harness| harness.rows()[0].chars().position(|c| c == '┌');
    harness.ctrl('b');
    harness.type_text("\\");
    assert_eq!(
        corner(&harness),
        Some(60),
        "it opens over the pane's right half, 40 columns at least"
    );
    harness.type_text("\\");
    assert_eq!(
        corner(&harness),
        Some(28),
        "over its whole pane, beside the sidebar"
    );
    let rows = harness.rows();
    assert!(rows[2].starts_with(" 󰖙 Today"), "the sidebar stays");
    assert!(rows[9].contains("│ e edit · a comment · \\ hide · Esc close  "));
    harness.type_text("eA note as wide as the panel is now, past half the screen");
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 100, 12)),
        Some(Position::new(86, 2)),
        "the note wraps in the panel's width"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("\\");
    assert_eq!(corner(&harness), None, "full, then hidden (D-129)");
    assert_eq!(mode(&harness), "NAVIGATE", "the keys back in the list");
    harness.type_text("\\");
    assert_eq!(corner(&harness), Some(60), "it opens at half again");
    harness.type_text("\\");
    harness.key(KeyCode::Esc);
    harness.type_text("\\");
    assert_eq!(corner(&harness), Some(60), "and after Esc from full too");
}

// FR-ATT-01, FR-PANE-02
#[test]
fn moving_a_full_panel_to_another_item_opens_it_at_half() {
    let mut harness = Harness::new(100, 12);
    let corner = |harness: &Harness| harness.rows()[0].chars().position(|c| c == '┌');
    harness.type_text("\\\\");
    assert_eq!(corner(&harness), Some(0), "full");
    harness.type_text("g0N");
    assert_eq!(
        corner(&harness),
        Some(60),
        "on the sidebar's entry, at half beside the sidebar"
    );
    harness.type_text("\\");
    assert_eq!(corner(&harness), Some(28), "full again");
    harness.type_text("g0");
    assert_eq!(corner(&harness), Some(28), "the keys leaving keep it full");
    harness.key(KeyCode::Tab);
    assert_eq!(corner(&harness), Some(28), "and coming back");
    assert_eq!(mode(&harness), "ATTACHMENTS", "to the panel, past its list");
}

// FR-ATT-01, FR-PANE-02, D-112
#[test]
fn the_keys_never_go_to_a_list_its_panel_covers() {
    let mut harness = Harness::new(80, 12);
    harness.type_text("\\\\");
    harness.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    harness.ctrl('w');
    harness.type_text("h");
    harness.type_text("g1");
    assert_eq!(mode(&harness), "ATTACHMENTS", "the list is out of sight");
    harness.key(KeyCode::Esc);
    harness.type_text("\\");
    harness.ctrl('w');
    harness.type_text("h");
    assert_eq!(mode(&harness), "NAVIGATE", "at half the list shows again");
    harness.ctrl('w');
    harness.type_text("v\\");
    harness.type_text("g1");
    assert_eq!(
        mode(&harness),
        "ATTACHMENTS",
        "a pane narrower than 40 columns is all panel, at half too"
    );
    harness.key(KeyCode::Esc);
    assert_eq!(mode(&harness), "NAVIGATE", "Esc gives the list the keys");
}

/// The app at 100 by 14 on a file holding a task with a checklist and a task after it, with the
/// first task's panel open at half beside the list, which has the keys.
fn beside_its_list() -> (tempfile::TempDir, Harness) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\n\t- Alpha task\n\t\t- one\n\t\t- two\n\t- Beta task\n";
    fs::write(&path, text).expect("the fixture is written");
    let mut harness = Harness::open(path, 100, 14);
    harness.type_text("Ckk");
    harness.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    (dir, harness)
}

/// Makes the terminal 38 columns wide, so that the panel covers the whole of its pane.
fn narrow(harness: &mut Harness) {
    harness.terminal.backend_mut().resize(38, 14);
    harness.send(&Event::Resize(38, 14));
}

/// What opens a window from the list.
type Opening = fn(&mut Harness);

// FR-ATT-01, FR-UI-06, D-112, D-136
#[test]
fn a_window_keeps_the_keys_when_the_panel_comes_to_cover_its_list() {
    let windows: [(&str, Opening, &str); 5] = [
        (
            "the edit dialog",
            |harness| harness.type_text("e"),
            "Alpha taskjd",
        ),
        ("the date prompt", |harness| harness.type_text("ms"), " jd"),
        ("Quick Add", |harness| harness.type_text("q"), "jd"),
        ("the context menu", |harness| harness.ctrl('\\'), "jd"),
        ("the command palette", |harness| harness.ctrl('p'), "jd"),
    ];
    for (window, opening, typed) in windows {
        let (_dir, mut harness) = beside_its_list();
        opening(&mut harness);
        narrow(&mut harness);
        harness.type_text("jd");
        assert!(
            harness.rows().iter().any(|row| row.contains(typed)),
            "{window} takes what is typed:\n{}",
            harness.screen()
        );
        assert_ne!(mode(&harness), "ATTACHMENTS", "{window} keeps the keys");
        harness.key(KeyCode::Esc);
        assert_eq!(
            mode(&harness),
            "ATTACHMENTS",
            "once {window} closes, the panel takes them"
        );
        assert!(
            harness.screen().contains("☐ one"),
            "and its checklist is as it was after {window}"
        );
    }
}

// FR-ATT-01, FR-KEY-07, FR-NAV-06, D-112, D-136
#[test]
fn a_command_chosen_from_a_menu_the_panel_comes_to_cover_acts_on_the_list() {
    let menus: [(&str, Opening); 2] = [
        ("the context menu", |harness| harness.ctrl('\\')),
        ("the command palette", |harness| harness.ctrl('p')),
    ];
    for (menu, opening) in menus {
        let (_dir, mut harness) = beside_its_list();
        opening(&mut harness);
        narrow(&mut harness);
        harness.type_text("delete");
        harness.key(KeyCode::Enter);
        let screen = harness.screen();
        assert!(
            !screen.contains("Alpha task") && screen.contains("Beta task"),
            "Delete from {menu} deletes the task it was opened on:\n{screen}"
        );
        for chosen in ["edit title", "new item below"] {
            let (_dir, mut harness) = beside_its_list();
            opening(&mut harness);
            narrow(&mut harness);
            harness.type_text(chosen);
            harness.key(KeyCode::Enter);
            assert_eq!(
                mode(&harness),
                "ATTACHMENTS",
                "{chosen} from {menu} leaves no edit under the panel"
            );
            harness.type_text("Z");
            harness.key(KeyCode::Esc);
            let screen = harness.screen();
            assert!(
                !screen.contains('Z') && screen.matches('☐').count() == 2,
                "nothing typed went into the list, nor was a task added:\n{screen}"
            );
        }
    }
}

// FR-ATT-01, FR-EDIT-01, D-105, D-112, D-136
#[test]
fn a_title_edit_ends_kept_when_the_panel_comes_to_cover_its_list() {
    let (dir, mut harness) = beside_its_list();
    harness.type_text("j");
    harness.key(KeyCode::Enter);
    harness.type_text("X");
    narrow(&mut harness);
    assert_eq!(mode(&harness), "ATTACHMENTS", "the panel takes the keys");
    harness.wait(1000);
    let saved = fs::read_to_string(dir.path().join("todo.taskpaper")).expect("the file reads");
    assert!(
        saved.contains("\t- Beta taskX\n"),
        "and the edit is saved:\n{saved}"
    );
    assert!(
        !harness.screen().contains("Beta taskX"),
        "and shows its own item, not the title edited:\n{}",
        harness.screen()
    );
    harness.type_text("Y");
    harness.key(KeyCode::Esc);
    let rows = harness.rows();
    assert!(
        rows.iter().any(|row| row.contains("Beta taskX")),
        "the edit is kept:\n{}",
        harness.screen()
    );
    assert!(
        !rows.iter().any(|row| row.contains("taskXY")),
        "and what is typed after goes nowhere near it"
    );
}

// FR-ATT-02, FR-ATT-03, D-112, D-136
#[test]
fn the_panel_that_comes_to_cover_its_list_edits_from_the_first_key() {
    let (_dir, mut harness) = beside_its_list();
    narrow(&mut harness);
    harness.type_text("eY");
    assert!(
        harness.rows()[2].contains("│▌Y"),
        "the note is edited:\n{}",
        harness.screen()
    );
}
