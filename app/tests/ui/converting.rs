//! Converting tasks, sections and projects into one another, and grouping items into a new
//! section or project.

use std::{fs, path::PathBuf};

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::support::Harness;

/// The status line, without trailing spaces.
fn status(harness: &Harness) -> String {
    harness
        .rows()
        .last()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

const PLANS: &str = "\
Inbox:
\t- Buy coffee
Plans: @area
\t- Paint the walls
\t- Order paint
\t- Call the plumber
\tMove house:
\t\t- Pack
\t\t\t- Books
\tNext: @section
\t\t- Sell the sofa
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

fn keys(harness: &mut Harness, keys: &str) {
    for c in keys.chars() {
        harness.key(KeyCode::Char(c));
    }
}

// FR-OPS-07, FR-EDIT-06, FR-STORE-09
#[test]
fn group_three_tasks_into_a_section() {
    let (_dir, path, mut harness) = plans();
    keys(&mut harness, "JJgS");
    harness.type_text("Soon");
    harness.key(KeyCode::Esc);
    let screen = harness.screen();
    let heading = screen.find("Soon").expect("the new section shows");
    let tasks = ["Paint the walls", "Order paint", "Call the plumber"]
        .map(|task| screen.find(task).expect("each task shows"));
    assert!(tasks.iter().all(|&at| at > heading), "{screen}");
    assert_eq!(
        saved(&mut harness, &path),
        "\
Inbox:
\t- Buy coffee
Plans: @area
\tSoon: @section
\t\t- Paint the walls
\t\t- Order paint
\t\t- Call the plumber
\tMove house:
\t\t- Pack
\t\t\t- Books
\tNext: @section
\t\t- Sell the sofa
"
    );
    harness.key(KeyCode::Char('z'));
    assert_eq!(saved(&mut harness, &path), PLANS, "one step to undo");
}

// FR-TASK-06, FR-ORG-06
#[test]
fn a_task_becomes_a_project_and_a_project_a_section() {
    let (_dir, path, mut harness) = plans();
    keys(&mut harness, "my");
    let file = saved(&mut harness, &path);
    assert!(
        file.contains("\tPaint the walls:\n\t- Order paint\n"),
        "in its place: {file}"
    );
    keys(&mut harness, "my");
    let file = saved(&mut harness, &path);
    assert!(
        file.contains("\tPaint the walls: @section\n\t- Order paint\n"),
        "{file}"
    );
    keys(&mut harness, "zzGmy");
    let file = saved(&mut harness, &path);
    assert!(
        file.contains("\tMove house: @section\n\t\t- Pack\n\t\t\t- Books\n"),
        "a project of tasks becomes a section: {file}"
    );
}

// FR-TASK-06, FR-OPS-07, FR-OPS-08, OQ-2
#[test]
fn where_the_new_kind_cannot_go_the_status_line_says_why() {
    let (_dir, _path, mut harness) = plans();
    keys(&mut harness, "GgS");
    assert!(
        status(&harness).ends_with("a section holds only tasks"),
        "{}",
        status(&harness)
    );
    keys(&mut harness, "gi");
    keys(&mut harness, "my");
    assert!(status(&harness).ends_with("a project goes only in an area or at the top level"));
    keys(&mut harness, "gS");
    assert!(status(&harness).ends_with("a section goes only in an area or a project"));
}

// FR-OPS-08, FR-STORE-09
#[test]
fn tasks_and_a_section_group_into_a_new_project() {
    let (_dir, path, mut harness) = plans();
    keys(&mut harness, "vjjjvgG");
    harness.type_text("Renovate");
    harness.key(KeyCode::Enter);
    assert_eq!(
        saved(&mut harness, &path),
        "\
Inbox:
\t- Buy coffee
Plans: @area
\tRenovate:
\t\t- Paint the walls
\t\tNext: @section
\t\t\t- Sell the sofa
\t- Order paint
\t- Call the plumber
\tMove house:
\t\t- Pack
\t\t\t- Books
"
    );
}

// FR-TASK-06, FR-STORE-09
#[test]
fn a_task_whose_title_starts_like_a_task_line_stays_a_task() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\nPlans: @area\n\t- - Plan\n\t\t- Pack\n";
    fs::write(&path, text).expect("the fixture is written");
    let mut harness = Harness::open(path.clone(), 70, 20);
    harness.ctrl('g');
    harness.type_text("plans");
    harness.key(KeyCode::Enter);
    keys(&mut harness, "my");
    assert!(status(&harness).ends_with(r#"only a task's title can start with "- ""#));
    assert_eq!(saved(&mut harness, &path), text);
}

// FR-TASK-06, FR-STORE-09, D-47
#[test]
fn a_task_with_a_kind_tag_stays_a_task() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\nWork: @area\n\t- Plan @area\n";
    fs::write(&path, text).expect("the fixture is written");
    let mut harness = Harness::open(path.clone(), 70, 20);
    harness.ctrl('g');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    keys(&mut harness, "my");
    assert!(status(&harness).ends_with("only a task can be tagged @area or @section"));
    assert_eq!(saved(&mut harness, &path), text);
}

// FR-TASK-06, FR-VIEW-12, FR-VIEW-14, FR-ORG-12, D-120
#[test]
fn a_bare_pinned_or_flagged_tag_the_new_kind_reads_becomes_its_mark() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\nHome: @area\n\t- Plan @pinned @home\n\t\t- Pack @flagged\n";
    fs::write(&path, text).expect("the fixture is written");
    let mut harness = Harness::open(path.clone(), 120, 20);
    harness.ctrl('g');
    harness.type_text("home");
    harness.key(KeyCode::Enter);
    keys(&mut harness, "my");
    assert_eq!(
        saved(&mut harness, &path),
        "Inbox:\nHome: @area\n\tPlan: @pinned @home\n\t\t- Pack @flagged\n",
        "the lines read the same"
    );
    keys(&mut harness, "g!");
    harness.ctrl('b');
    let converted = harness.screen();
    assert!(
        converted.contains("Pack · Home › Plan"),
        "flagged: {converted}"
    );
    assert_eq!(
        converted.matches("# Plan").count(),
        2,
        "pinned: {converted}"
    );
    let mut harness = harness.relaunch(path);
    keys(&mut harness, "g!");
    harness.ctrl('b');
    assert_eq!(harness.screen(), converted, "it reads back as it was shown");
}
