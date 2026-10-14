//! Quick Add: the panel `qq`, `qt` and `qi` open, the dates found in its text, the place chosen
//! with `/` and the task it adds (FR-EDIT-07).

use std::{fs, path::PathBuf};

use crossterm::event::{Event, KeyCode};
use ratatui::layout::{Position, Rect};
use tempfile::TempDir;

use crate::support::Harness;

/// The app on a file in a temporary folder holding `text`, on an 80 by 24 terminal, and the
/// file's path.
fn on_file(text: &str) -> (TempDir, PathBuf, Harness) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, text).expect("the fixture is written");
    let harness = Harness::open(path.clone(), 80, 24);
    (dir, path, harness)
}

/// The panel: the four rows above the status line, between the borders' first column and last,
/// without trailing spaces.
fn panel(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .get(19..23)
        .expect("the screen has those rows")
        .iter()
        .map(|row| {
            let inner: String = row.chars().skip(1).collect();
            inner
                .trim_end_matches(['│', '┐', '┘'])
                .trim_end()
                .to_owned()
        })
        .collect()
}

const FILE: &str =
    "Inbox:\n\t- Old\nWebsite:\n\t- Fix the footer\n\tRedesign:\n\t\t- Draw\nBlog:\n";

/// The file at `path` once autosave has written it.
fn saved(harness: &mut Harness, path: &PathBuf) -> String {
    harness.wait(1000);
    fs::read_to_string(path).expect("the file is there")
}

// FR-EDIT-07
#[test]
fn qq_adds_a_task_with_its_dates_to_the_project_chosen_with_a_slash() {
    let (_dir, path, mut harness) = on_file(FILE);
    harness.type_text("qq");
    assert_eq!(
        panel(&harness),
        [
            &format!(" Quick Add → Inbox {}", "─".repeat(59)),
            "",
            " Type the task; / chooses where it goes",
            &"─".repeat(78),
        ]
    );
    harness.type_text("Do something tomorrow /Web");
    assert!(harness.rows()[9].contains("› Web"), "the picker's query");
    assert!(harness.rows()[10].contains("# Website"));
    harness.key(KeyCode::Enter);
    assert_eq!(
        panel(&harness)[..3],
        [
            format!(" Quick Add → Website {}", "─".repeat(57)),
            " Do something tomorrow".to_owned(),
            " Start Tue 6 Oct 2026".to_owned(),
        ]
    );
    harness.key(KeyCode::Enter);
    assert!(harness.rows()[23].contains("added to Website"));
    assert_eq!(panel(&harness), ["", "", "", ""], "the panel closed");
    assert_eq!(
        saved(&mut harness, &path),
        "Inbox:\n\t- Old\nWebsite:\n\t- Fix the footer\n\t- Do something @start(2026-10-06)\n\
         \tRedesign:\n\t\t- Draw\nBlog:\n"
    );
    harness.type_text("z");
    assert_eq!(saved(&mut harness, &path), FILE, "one step to undo");
}

// FR-EDIT-07
#[test]
fn next_week_is_the_start_and_due_in_a_month_the_due_date() {
    let (_dir, path, mut harness) = on_file(FILE);
    harness.type_text("qqDo something next week due in a month /Blog");
    harness.key(KeyCode::Enter);
    assert_eq!(
        panel(&harness)[..3],
        [
            format!(" Quick Add → Blog {}", "─".repeat(60)),
            " Do something next week due in a month".to_owned(),
            " Start Mon 12 Oct 2026 · Due Thu 5 Nov 2026".to_owned(),
        ]
    );
    harness.key(KeyCode::Enter);
    assert!(
        saved(&mut harness, &path)
            .ends_with("Blog:\n\t- Do something @start(2026-10-12) @due(2026-11-05)\n")
    );
}

// FR-EDIT-07
#[test]
fn a_date_of_more_than_four_words_is_read_whole() {
    let (_dir, path, mut harness) = on_file(FILE);
    harness.type_text("qqPlan in a month +1 day");
    assert_eq!(panel(&harness)[2], " Start Fri 6 Nov 2026");
    harness.key(KeyCode::Enter);
    assert!(
        saved(&mut harness, &path).starts_with("Inbox:\n\t- Old\n\t- Plan @start(2026-11-06)\n")
    );
}

// FR-EDIT-07
#[test]
fn qt_starts_the_task_today_unless_another_start_is_typed_and_qi_adds_to_the_inbox() {
    let (_dir, path, mut harness) = on_file(FILE);
    harness.type_text("qtCall mom");
    assert_eq!(panel(&harness)[2], " Start Mon 5 Oct 2026");
    harness.key(KeyCode::Enter);
    harness.type_text("qtCall dad friday due sat");
    assert_eq!(
        panel(&harness)[2],
        " Start Fri 9 Oct 2026 · Due Sat 10 Oct 2026"
    );
    harness.key(KeyCode::Enter);
    harness.type_text("qiPay rent");
    assert_eq!(
        panel(&harness)[..3],
        [
            format!(" Quick Add → Inbox {}", "─".repeat(59)),
            " Pay rent".to_owned(),
            " No dates".to_owned(),
        ]
    );
    harness.key(KeyCode::Enter);
    assert!(saved(&mut harness, &path).starts_with(
        "Inbox:\n\t- Old\n\t- Call mom @start(2026-10-05)\n\
         \t- Call dad @start(2026-10-09) @due(2026-10-10)\n\t- Pay rent\nWebsite:\n"
    ));
}

// FR-EDIT-07
#[test]
fn a_slash_that_finds_nothing_or_is_dismissed_stays_text() {
    let (_dir, path, mut harness) = on_file(FILE);
    harness.type_text("qqRead a/b /zz");
    assert_eq!(panel(&harness)[1], " Read a/b /zz", "nothing matches zz");
    harness.type_text(" /bl");
    harness.key(KeyCode::Esc);
    assert_eq!(panel(&harness)[1], " Read a/b /zz /bl", "Esc types it back");
    harness.type_text(" /");
    harness.key(KeyCode::Backspace);
    assert_eq!(
        panel(&harness)[1],
        " Read a/b /zz /bl",
        "Backspace on an empty query closes the picker"
    );
    harness.key(KeyCode::Backspace);
    harness.send(&Event::Paste(" one\ntwo /Red".to_owned()));
    harness.key(KeyCode::Enter);
    assert_eq!(
        panel(&harness)[..2],
        [
            format!(" Quick Add → Website › Redesign {}", "─".repeat(46)),
            " Read a/b /zz /bl one two".to_owned(),
        ],
        "a paste breaks no line and its slash opens the picker"
    );
    harness.key(KeyCode::Enter);
    assert!(saved(&mut harness, &path).contains("\t\t- Draw\n\t\t- Read a/b /zz /bl one two\n"));
}

// FR-EDIT-07, FR-TAG-02
#[test]
fn the_panel_types_no_tag_adds_nothing_without_a_title_and_esc_closes_it() {
    let (_dir, path, mut harness) = on_file(FILE);
    harness.type_text("qq@tomorrow");
    assert_eq!(
        panel(&harness)[1],
        " tomorrow",
        "@ starting a word is not typed"
    );
    assert!(harness.rows()[23].contains("a title has no word starting with @"));
    harness.key(KeyCode::Enter);
    assert_eq!(
        panel(&harness)[1],
        " tomorrow",
        "only a date: nothing to add"
    );
    harness.key(KeyCode::Esc);
    assert_eq!(panel(&harness), ["", "", "", ""]);
    assert_eq!(saved(&mut harness, &path), FILE);
}

// FR-EDIT-07, FR-KEY-03
#[test]
fn the_shortcuts_panel_lists_the_panel_keys_and_quick_add_works_from_the_sidebar_and_closes_for_the_navigator()
 {
    let mut harness = Harness::launched(120, 30);
    harness.type_text("?qq");
    let screen = harness.screen();
    assert!(screen.contains("Quick Add"));
    assert!(screen.contains("Add Task"));
    harness.key(KeyCode::Esc);
    harness.type_text("?");
    harness.ctrl('b');
    harness.type_text("qi");
    assert!(harness.screen().contains("Quick Add → Inbox"));
    harness.ctrl('k');
    let screen = harness.screen();
    assert!(
        !screen.contains("Quick Add → Inbox"),
        "a global key still works"
    );
    assert!(screen.contains("Navigator"));
}

// FR-EDIT-07, F-FILE-06
#[test]
fn the_caret_is_in_the_panel_or_the_picker_and_a_reload_keeps_the_place() {
    let (_dir, path, mut harness) = on_file(FILE);
    harness.type_text("qqab");
    let screen = Rect::new(0, 0, 80, 24);
    assert_eq!(harness.tui.cursor(screen), Some(Position::new(4, 20)));
    harness.type_text(" /bl");
    assert!(
        harness.tui.cursor(screen).is_some_and(|at| at.y < 19),
        "in the picker over the panes"
    );
    harness.key(KeyCode::Enter);
    fs::write(&path, format!("{FILE}\t- Draft\n")).expect("written");
    harness.check();
    assert_eq!(
        panel(&harness)[..2],
        [
            format!(" Quick Add → Blog {}", "─".repeat(60)),
            " ab".to_owned(),
        ]
    );
}
