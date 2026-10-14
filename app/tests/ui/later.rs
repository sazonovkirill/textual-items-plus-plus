//! Later: `ml` marks tasks and projects Later, which the file keeps as `@later` and the app lists
//! dimmed after the open items of their container and before the completed ones.

use std::{fs, path::PathBuf};

use crossterm::event::KeyCode;
use ratatui::style::Color;
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

/// The file at `path` once a second has passed for the app to save it.
fn saved_after_a_second(harness: &mut Harness, path: &std::path::Path) -> String {
    harness.wait(1000);
    fs::read_to_string(path).expect("the file is there")
}

/// Rows `range` of the screen, without trailing spaces.
fn rows(harness: &Harness, range: std::ops::Range<usize>) -> Vec<String> {
    harness
        .rows()
        .get(range)
        .expect("the screen has those rows")
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

/// The color of the character at column `x` of row `y`.
fn color(harness: &Harness, x: u16, y: u16) -> Color {
    harness.terminal.backend().buffer()[(x, y)].fg
}

/// Whether the status line says `text`.
fn says(harness: &Harness, text: &str) -> bool {
    harness.rows().last().is_some_and(|row| row.contains(text))
}

/// Opens the container titled `title` from the Navigator.
fn open(harness: &mut Harness, title: &str) {
    harness.ctrl('g');
    harness.type_text(title);
    harness.key(KeyCode::Enter);
}

// FR-ORG-11, FR-STORE-07, FR-EDIT-06
#[test]
fn ml_marks_a_task_later_which_goes_dimmed_after_the_open_ones() {
    let text =
        "Inbox:\nTrip:\n\t- Renew the passport\n\t- Book flights\n\t- Pack @done(2026-10-01)\n";
    let (_dir, path, mut harness) = on_file(text);
    open(&mut harness, "trip");
    harness.ctrl('o');
    harness.type_text("ml");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nTrip:\n\t- Renew the passport @later\n\t- Book flights\n\t- Pack @done(2026-10-01)\n"
    );
    assert_eq!(
        rows(&harness, 2..5),
        ["  ☐ Book flights", "▌ ☐ Renew the passport", "  ✔ Pack"],
        "after the open task, before the completed one, and still focused"
    );
    assert_eq!(color(&harness, 4, 3), Color::Indexed(248), "dimmed");
    assert_eq!(color(&harness, 4, 2), Color::Reset, "an open task is not");
    harness.type_text("ml");
    assert_eq!(
        rows(&harness, 2..4),
        ["▌ ☐ Renew the passport", "  ☐ Book flights"],
        "the same key takes it out of Later"
    );
    harness.type_text("z");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nTrip:\n\t- Renew the passport @later\n\t- Book flights\n\t- Pack @done(2026-10-01)\n",
        "one step to undo"
    );
}

// FR-ORG-11, FR-OPS-06
#[test]
fn a_selection_is_marked_later_until_all_of_it_is() {
    let text = "Inbox:\nTrip:\n\t- Renew the passport @later\n\t- Book flights\n";
    let (_dir, path, mut harness) = on_file(text);
    open(&mut harness, "trip");
    harness.type_text("Vjml");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nTrip:\n\t- Renew the passport @later\n\t- Book flights @later\n"
    );
    harness.type_text("ml");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nTrip:\n\t- Renew the passport\n\t- Book flights\n"
    );
}

// FR-ORG-11
#[test]
fn an_area_or_a_section_is_never_later() {
    let text = "Inbox:\nHome: @area\n\tGarden: @section\n\t\t- Mow\n";
    let (_dir, path, mut harness) = on_file(text);
    harness.type_text("ml");
    assert!(
        !says(&harness, "Later"),
        "the empty Inbox has nothing to mark"
    );
    open(&mut harness, "home");
    for keys in ["kml", "jml"] {
        harness.type_text(keys);
        assert!(
            says(&harness, "only a task or a project can be Later"),
            "{keys}: {:#?}",
            harness.rows()
        );
    }
    assert_eq!(saved_after_a_second(&mut harness, &path), text);
}

// FR-ORG-11, FR-VIEW-10, FR-VIEW-05
#[test]
fn a_later_project_is_dimmed_and_listed_last_in_the_sidebar_and_its_area() {
    let text = "Inbox:\nWork: @area\n\tSite:\n\tDocs:\n";
    let (_dir, path, mut harness) = on_file(text);
    harness.ctrl('b');
    harness.type_text("jjjjjjjjml");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nWork: @area\n\tSite: @later\n\tDocs:\n"
    );
    let sidebar: Vec<String> = rows(&harness, 9..12)
        .iter()
        .map(|row| {
            row.chars()
                .take(10)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    assert_eq!(sidebar, [" 󰆧 WORK", " # Docs", "▌# Site"]);
    assert_eq!(color(&harness, 3, 11), Color::Indexed(248), "dimmed");
    harness.key(KeyCode::Tab);
    open(&mut harness, "work");
    let pane: Vec<String> = rows(&harness, 2..4)
        .iter()
        .map(|row| {
            row.chars()
                .skip(26)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    assert_eq!(pane, ["▌ # Docs", "  # Site"]);
    assert_eq!(color(&harness, 30, 3), Color::Indexed(248), "dimmed");
}

// FR-ORG-11, FR-DATE-02
#[test]
fn what_a_later_project_holds_is_dimmed_and_later_and_a_start_date_exclude_each_other() {
    let text = "Inbox:\nSite: @later\n\t- Draft @start(2026-10-05)\n";
    let (_dir, path, mut harness) = on_file(text);
    open(&mut harness, "site");
    assert!(rows(&harness, 2..3)[0].starts_with("▌ ☐ Draft "));
    assert_eq!(color(&harness, 4, 2), Color::Indexed(248), "dimmed inside");
    harness.type_text("ml");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nSite: @later\n\t- Draft @later\n",
        "marking it Later clears its start date"
    );
    harness.type_text("mt");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        "Inbox:\nSite: @later\n\t- Draft @start(2026-10-05)\n",
        "starting it takes it out of Later"
    );
}
