//! Flagging: `mf` flags tasks and projects, which the file keeps as `@flagged` and the app marks
//! with a red flag in front of their titles, and `g!` opens the Flagged view listing them.

use std::{fs, path::PathBuf};

use crossterm::event::KeyCode;
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

/// The first `count` rows of the screen, without trailing spaces.
fn top(harness: &Harness, count: usize) -> Vec<String> {
    harness
        .rows()
        .iter()
        .take(count)
        .map(|row| row.trim_end().to_owned())
        .collect()
}

/// Whether the status line says `text`.
fn says(harness: &Harness, text: &str) -> bool {
    harness.rows().last().is_some_and(|row| row.contains(text))
}

// FR-ORG-12, FR-VIEW-14, FR-STORE-07
#[test]
fn mf_flags_tasks_and_projects_which_the_flagged_view_lists_projects_first() {
    let text = concat!(
        "Inbox:\n\t- Call mom\n\t- Called @flagged @done(2026-10-01)\n",
        "Work: @area\n\tSite:\n\t\tLaunch: @section\n\t\t\t- Ship it\n\t\t- Draft\n",
    );
    let (_dir, path, mut harness) = on_file(text);
    harness.type_text("mf");
    assert_eq!(top(&harness, 3)[2], "▌ ☐ 󰈻 Call mom");
    let flag = &harness.terminal.backend().buffer()[(4, 2)];
    assert_eq!(flag.fg, ratatui::style::Color::LightRed, "a red flag");
    harness.ctrl('g');
    harness.type_text("site");
    harness.key(KeyCode::Enter);
    harness.type_text("jmf");
    assert!(
        says(&harness, "only a task or a project can be flagged"),
        "a section"
    );
    harness.type_text("kmfggmf");
    assert_eq!(
        saved_after_a_second(&mut harness, &path),
        concat!(
            "Inbox:\n\t- Call mom @flagged\n\t- Called @flagged @done(2026-10-01)\n",
            "Work: @area\n\tSite: @flagged\n\t\tLaunch: @section\n\t\t\t- Ship it\n",
            "\t\t- Draft @flagged\n",
        )
    );
    harness.type_text("g!");
    assert_eq!(
        top(&harness, 7),
        [
            "  󰈻 Flagged",
            "",
            "▌ # 󰈻 Site · Work",
            "",
            "  ☐ 󰈻 Call mom",
            "  ☐ 󰈻 Draft · Work › Site",
            "",
        ]
    );
    harness.type_text("mf");
    assert_eq!(
        top(&harness, 3)[2],
        "▌ ☐ 󰈻 Call mom",
        "unflagged, it leaves"
    );
    harness.type_text("z");
    assert_eq!(top(&harness, 3)[2], "▌ # 󰈻 Site · Work", "one step to undo");
    harness.ctrl('o');
    assert_eq!(top(&harness, 7)[6], "  ✔ 󰈻 Called", "completed, shown last");
    let flag = &harness.terminal.backend().buffer()[(4, 6)];
    assert!(
        flag.modifier.contains(ratatui::style::Modifier::DIM),
        "dimmed"
    );
}

// FR-ORG-12, FR-VIEW-10, FR-VIEW-07
#[test]
fn the_sidebar_and_the_navigator_show_a_flagged_projects_flag() {
    let text = "Inbox:\nWork: @area\n\tSite: @flagged\n\tDocs:\n";
    let (_dir, _path, mut harness) = on_file(text);
    harness.ctrl('b');
    let rows = harness.rows();
    let site = rows
        .iter()
        .position(|row| row.contains("Site"))
        .expect("listed");
    let sidebar = |row: &str| {
        row.chars()
            .take(20)
            .collect::<String>()
            .trim_end()
            .to_owned()
    };
    assert_eq!(sidebar(&rows[site]), " # 󰈻 Site");
    assert_eq!(sidebar(&rows[site + 1]), " # Docs");
    let y = u16::try_from(site).expect("on screen");
    let flag = &harness.terminal.backend().buffer()[(3, y)];
    assert_eq!(flag.fg, ratatui::style::Color::LightRed);
    harness.ctrl('g');
    harness.type_text("s");
    let screen = harness.screen();
    assert!(screen.contains("# 󰈻 Site"), "{screen}");
    assert!(!screen.contains("󰈻 Docs"), "{screen}");
}
