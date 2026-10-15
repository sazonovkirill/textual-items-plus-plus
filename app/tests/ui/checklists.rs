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
    harness.type_text("C");
    assert_eq!(
        panel(&harness)[..5],
        [
            "│ \u{f11d7} Note                               │",
            "│ No note                              │",
            "│                                      │",
            "│ \u{f0756} Checklist                          │",
            "│▌No checklist                         │",
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
            "│ ☐ Documents                          │",
            "│▌  ✔ ID card                          │",
            "│   ☐ Proof of address                 │",
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
            "│ ☐ Documents                          │",
            "│   ✔ ID card                          │",
            "│   ☐ Call                             │",
            "│     ☐ Ask about fees                 │",
            "│   ☐ Proof of address                 │",
            "│▌  ☐ Thank them                       │",
        ],
        "a paste adds an item for each line, nested by its indentation, and Alt+j moves one down"
    );
    harness.type_text("d");
    assert_eq!(
        panel(&harness)[8],
        "│▌  ☐ Proof of address                 │"
    );
    harness.key(KeyCode::Esc);
    assert!(
        harness.rows()[3].ends_with("\u{f0756} 1/5 \u{f166c} Wed "),
        "the row shows how much of its checklist is done: {}",
        harness.rows()[3]
    );
    harness.type_text("z");
    assert!(
        harness.rows()[3].contains("\u{f0756} 1/6"),
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
    harness.type_text("jC");
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
    harness.type_text("Co");
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
    harness.type_text("jykC");
    harness.ctrl('v');
    assert_eq!(
        panel(&harness)[4..6],
        [
            "│▌☐ Buy tape                           │",
            "│   ☐ for the boxes                    │",
        ],
        "the copied task and its note line arrive as checklist items"
    );
}

// FR-CHK-03
#[test]
fn a_list_pasted_with_carriage_returns_or_a_copied_task_shows_every_line_as_an_item() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(
        &path,
        "Inbox:\n\t- Groceries\n\t- Write @due(2026-10-20)\n\t\tNOTE @hidden\n\t\t\tDraft\n\t\tCOMMENTS @hidden\n\t\t\t[2026-10-13 21:30] Ask\n",
    )
    .expect("the file is written");
    // The panel's hint takes two lines at this width.
    let mut harness = Harness::open(path.clone(), 80, 17);
    harness.type_text("C");
    harness.send(&Event::Paste(
        "Fruit\r\tApples\r\tPears\rBread\r".to_owned(),
    ));
    assert_eq!(
        panel(&harness)[4..8],
        [
            "│ ☐ Fruit                              │",
            "│   ☐ Apples                           │",
            "│   ☐ Pears                            │",
            "│▌☐ Bread                              │",
        ],
        "each line a terminal sends is an item, nested by its indentation"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("jyk");
    harness.type_text("C");
    harness.ctrl('v');
    harness.key(KeyCode::Esc);
    harness.terminate();
    let saved = fs::read_to_string(&path).expect("the file is read");
    assert!(
        saved.starts_with(
            "Inbox:\n\t- Groceries\n\t\t- Fruit\n\t\t\t- Apples\n\t\t\t- Pears\n\t\t- Write\n\t\t- Bread\n\t- Write @due"
        ),
        "the copied task arrives below the first item without its note, comments and date: {saved}"
    );
}

// FR-UI-02, FR-ATT-01
#[test]
fn only_the_panel_marks_what_is_selected_while_it_has_the_keys() {
    let mut harness = Harness::new(80, 16);
    let row = |harness: &Harness| harness.rows()[3].chars().take(32).collect::<String>();
    assert_eq!(row(&harness), "▌ ☐ Call the bank about the card");
    let focused_bg = harness.terminal.backend().buffer()[(4, 3)].bg;
    harness.type_text("Co");
    harness.type_text("Fiction");
    harness.key(KeyCode::Enter);
    harness.type_text("Cookbooks");
    harness.key(KeyCode::Esc);
    assert_eq!(
        panel(&harness)[3..7],
        [
            "│ \u{f0756} Checklist                          │",
            "│ ☐ Fiction                            │",
            "│▌☐ Cookbooks                          │",
            "│                                      │",
        ]
    );
    let buffer = harness.terminal.backend().buffer();
    assert_eq!(
        buffer[(45, 6)].bg,
        focused_bg,
        "the selected entry on the band"
    );
    assert_ne!(
        buffer[(45, 7)].bg,
        focused_bg,
        "not the blank line after it"
    );
    assert_eq!(
        row(&harness),
        "  ☐ Call the bank about the card",
        "the pane's focus unmarked"
    );
    assert_ne!(buffer[(4, 3)].bg, focused_bg, "and off the band");
    assert_eq!(
        status(&harness).split(" · ").next(),
        Some(" \u{f01bf} ATTACHMENTS")
    );
    harness.key(KeyCode::Esc);
    assert_eq!(row(&harness), "▌ ☐ Call the bank about the card");
}

// FR-CHK-03, D-104
#[test]
fn tab_and_shift_tab_nest_an_item_selected_or_edited_and_leave_the_panes_be() {
    let folders = tempfile::tempdir().expect("a temporary folder");
    let path = folders.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- Pack\n\t\t- Plates\n\t\t- Cups\n").expect("the file is written");
    let mut harness =
        Harness::launched_in(folders, Some(path.clone()), 100, 16).expect("the file opens");
    let checklist = |harness: &Harness| -> Vec<String> {
        harness.rows()[5..8]
            .iter()
            .map(|row| {
                row.chars()
                    .skip(61)
                    .collect::<String>()
                    .trim_end_matches(['│', ' '])
                    .to_owned()
            })
            .collect()
    };
    harness.type_text("Cj");
    harness.key(KeyCode::Tab);
    assert_eq!(
        checklist(&harness),
        [" ☐ Plates", "▌  ☐ Cups", ""],
        "Tab nests the item selected"
    );
    harness.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(checklist(&harness), [" ☐ Plates", "▌☐ Cups", ""]);
    harness.type_text("o");
    harness.type_text("Bowls");
    harness.key(KeyCode::Left);
    harness.key(KeyCode::Tab);
    harness.type_text("!");
    assert_eq!(
        checklist(&harness),
        [" ☐ Plates", " ☐ Cups", "▌  ☐ Bowl!s"],
        "Tab nests the item edited, the caret where it was"
    );
    harness.key(KeyCode::Esc);
    harness.key(KeyCode::Esc);
    assert!(
        harness.rows()[2].starts_with(" \u{f0599} Today                   │▌"),
        "the pane keeps the keys: {}",
        harness.rows()[2]
    );
    harness.terminate();
    let saved = fs::read_to_string(&path).expect("the file is read");
    assert_eq!(
        saved,
        "Inbox:\n\t- Pack\n\t\t- Plates\n\t\t- Cups\n\t\t\t- Bowl!s\n"
    );
}

// FR-CHK-03, D-124
#[test]
fn mj_and_mk_move_the_item_selected_and_leave_nothing_pending() {
    let folders = tempfile::tempdir().expect("a temporary folder");
    let path = folders.path().join("todo.taskpaper");
    fs::write(
        &path,
        "Inbox:\n\t- Pack\n\t\t- Plates\n\t\t- Cups\n\t\t- Bowls\n",
    )
    .expect("the file is written");
    let mut harness =
        Harness::launched_in(folders, Some(path.clone()), 100, 16).expect("the file opens");
    let checklist = |harness: &Harness| -> Vec<String> {
        harness.rows()[5..8]
            .iter()
            .map(|row| {
                row.chars()
                    .skip(61)
                    .collect::<String>()
                    .trim_end_matches(['│', ' '])
                    .to_owned()
            })
            .collect()
    };
    harness.type_text("Cmjmj");
    assert_eq!(
        checklist(&harness),
        [" ☐ Cups", " ☐ Bowls", "▌☐ Plates"],
        "mj moves the item selected down"
    );
    harness.type_text("mk");
    assert_eq!(checklist(&harness), [" ☐ Cups", "▌☐ Plates", " ☐ Bowls"]);
    harness.type_text("x");
    assert_eq!(
        checklist(&harness),
        [" ☐ Cups", "▌✔ Plates", " ☐ Bowls"],
        "the next key is the panel's own, no m left pending"
    );
    harness.terminate();
    let saved = fs::read_to_string(&path).expect("the file is read");
    assert!(
        saved.starts_with("Inbox:\n\t- Pack\n\t\t- Cups\n\t\t- Plates @done("),
        "{saved}"
    );
}
