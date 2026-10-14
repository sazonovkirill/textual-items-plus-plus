//! Tagging the selected items from the tag picker and filtering the pane by tags.

use std::fs;

use crossterm::event::KeyCode;
use ratatui::layout::{Position, Rect};

use crate::support::{Harness, without_chips};

/// The task rows, without their date chips, the focused ones starred.
fn tasks(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .filter_map(|row| {
            let focused = row.starts_with('▌');
            let title = without_chips(row.trim_start_matches('▌').trim()).strip_prefix("☐ ")?;
            Some(if focused {
                format!("*{title}")
            } else {
                title.to_owned()
            })
        })
        .collect()
}

/// The title line at the top of the pane, without trailing spaces.
fn title(harness: &Harness) -> String {
    harness
        .rows()
        .first()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

/// The status line, without trailing spaces.
fn status(harness: &Harness) -> String {
    harness
        .rows()
        .last()
        .map(|row| row.trim_end().to_owned())
        .unwrap_or_default()
}

/// Chooses the tag `name` in the tag picker `keys` open.
fn choose(harness: &mut Harness, keys: &str, name: &str) {
    harness.type_text(keys);
    harness.type_text(name);
    harness.key(KeyCode::Enter);
}

// FR-TAG-01, FR-TAG-02, FR-TAG-04, FR-OPS-06, FR-UI-09
#[test]
fn tag_two_tasks_and_filter_by_the_tag() {
    let mut harness = Harness::new(70, 16);
    harness.type_text("J");
    harness.type_text("mg");
    let screen = harness.screen();
    assert!(screen.contains("┌Tags"), "{screen}");
    assert!(
        screen.contains("errands") && screen.contains('–'),
        "one of two: {screen}"
    );
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 70, 16)),
        Some(Position::new(3, 5)),
        "at the query"
    );
    harness.type_text("deep work");
    assert!(harness.screen().contains("deep-work") && harness.screen().contains("new tag"));
    harness.key(KeyCode::Enter);
    assert_eq!(
        tasks(&harness),
        [
            "*Call the bank about the card @deep-work",
            "*Buy coffee beans @errands @deep-work",
            "Read the article Sam sent",
        ]
    );
    harness.key(KeyCode::Esc);
    choose(&mut harness, "gf", "deep");
    assert!(
        title(&harness).ends_with("@deep-work"),
        "{}",
        title(&harness)
    );
    assert_eq!(
        tasks(&harness),
        [
            "Call the bank about the card @deep-work",
            "*Buy coffee beans @errands @deep-work",
        ]
    );
    choose(&mut harness, "gf", "err");
    assert!(title(&harness).ends_with("@deep-work @errands"));
    assert_eq!(tasks(&harness), ["*Buy coffee beans @errands @deep-work"]);
    choose(&mut harness, "gf", "deep");
    assert!(
        title(&harness).ends_with(" @errands"),
        "chosen again, it goes"
    );
    harness.type_text("gF");
    assert_eq!(tasks(&harness).len(), 3, "the filter is off");
    assert!(!title(&harness).contains('@'));
}

// FR-TAG-02, FR-OPS-06, FR-EDIT-06
#[test]
fn a_tag_every_selected_task_carries_comes_off_them_all() {
    let mut harness = Harness::new(70, 16);
    harness.key(KeyCode::Char('j'));
    choose(&mut harness, "mg", "errands");
    assert_eq!(
        tasks(&harness).get(1).map(String::as_str),
        Some("*Buy coffee beans")
    );
    harness.key(KeyCode::Char('z'));
    assert_eq!(
        tasks(&harness).get(1).map(String::as_str),
        Some("*Buy coffee beans @errands"),
        "undo puts it back"
    );
    harness.type_text("mg");
    harness.key(KeyCode::Esc);
    assert!(!harness.screen().contains("┌Tags"), "Esc closes the picker");
    assert_eq!(
        tasks(&harness).get(1).map(String::as_str),
        Some("*Buy coffee beans @errands")
    );
}

// FR-TAG-04, FR-EDIT-05
#[test]
fn a_task_added_in_a_filtered_pane_carries_the_filters_tags() {
    let mut harness = Harness::new(70, 16);
    choose(&mut harness, "gf", "errands");
    harness.key(KeyCode::Char('o'));
    harness.type_text("Grind the beans");
    harness.key(KeyCode::Esc);
    assert_eq!(
        tasks(&harness),
        ["Buy coffee beans @errands", "*Grind the beans @errands"]
    );
}

// FR-TAG-04, FR-NAV-02
#[test]
fn the_filter_goes_when_the_pane_shows_another_place() {
    let mut harness = Harness::new(70, 16);
    choose(&mut harness, "gf", "errands");
    assert_eq!(tasks(&harness).len(), 1);
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
    assert!(!title(&harness).contains('@'));
    harness.key(KeyCode::Char('H'));
    assert_eq!(tasks(&harness).len(), 3);
}

// FR-EDIT-04, FR-TAG-02
#[test]
fn an_at_sign_typed_in_a_title_points_to_the_tag_picker() {
    let mut harness = Harness::new(90, 16);
    harness.key(KeyCode::Enter);
    harness.type_text(" @home");
    assert_eq!(
        tasks(&harness).first().map(String::as_str),
        Some("*Call the bank about the card home")
    );
    assert!(
        status(&harness).ends_with("a title has no word starting with @; add tags with m g"),
        "{}",
        status(&harness)
    );
}

// FR-TAG-01, FR-STORE-10
#[test]
fn tags_on_any_item_are_saved_as_taskpaper_tags_and_the_sidebar_tags_projects() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\nWork: @area\n\tSite:\n\t\t- Ship it\n").expect("written");
    let mut harness = Harness::open(path.clone(), 70, 16);
    harness.ctrl('g');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    choose(&mut harness, "mg", "web");
    harness.key(KeyCode::Enter);
    choose(&mut harness, "mg", "launch");
    harness.ctrl('b');
    choose(&mut harness, "mg", "home");
    harness.wait(1000);
    assert_eq!(
        fs::read_to_string(&path).expect("the file is there"),
        "Inbox:\nWork: @area\n\tSite: @web @home\n\t\t- Ship it @launch\n"
    );
}

// FR-TAG-01, FR-DATE-01, FR-STORE-10, D-75
#[test]
fn an_area_takes_no_tags_or_dates_and_keeps_those_its_file_gives_it() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\nWork: @area @old @due(2026-10-12)\n\t- Ship it\n";
    fs::write(&path, text).expect("written");
    let mut harness = Harness::open(path.clone(), 70, 16);
    harness.ctrl('g');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    harness.type_text("k");
    let heading = harness.rows()[0].trim_end().to_owned();
    assert_eq!(heading, "▌ 󰆧 Work", "the file's tag and date are not shown");
    for (keys, why) in [
        ("mg", "an area has no tags"),
        ("md", "an area has no start or due date"),
        ("ms", "an area has no start or due date"),
        ("mt", "an area has no start or due date"),
    ] {
        harness.type_text(keys);
        assert!(
            status(&harness).ends_with(why),
            "{keys}: {}",
            status(&harness)
        );
        assert!(!harness.screen().contains("┌Tags"), "{keys} opens nothing");
    }
    harness.ctrl('b');
    harness.type_text("md");
    assert!(status(&harness).ends_with("an area has no start or due date"));
    harness.ctrl('b');
    harness.type_text("jx");
    harness.wait(1000);
    let file = fs::read_to_string(&path).expect("the file is there");
    assert!(
        file.starts_with("Inbox:\nWork: @area @old @due(2026-10-12)\n\t- Ship it @done("),
        "{file}"
    );
}
