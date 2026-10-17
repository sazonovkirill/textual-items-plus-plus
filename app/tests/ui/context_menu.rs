//! The context menu.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use textual_items_plus_plus::ui::{Icon, IconSet};

use crate::support::Harness;

const TITLE: &str = "┌Context Menu";

/// The screen row the context menu's top border is on, and the rows inside its borders, without
/// the space around them.
fn menu(harness: &Harness) -> Option<(usize, Vec<String>)> {
    let rows = harness.rows();
    let top = rows.iter().position(|row| row.contains(TITLE))?;
    let inside = rows
        .iter()
        .skip(top + 1)
        .take_while(|row| !row.trim_start().starts_with('└'))
        .map(|row| {
            row.trim()
                .trim_start_matches('│')
                .trim_end_matches('│')
                .trim()
                .to_owned()
        })
        .collect();
    Some((top, inside))
}

/// The screen row whose text contains `text`.
fn row_of(harness: &Harness, text: &str) -> Option<usize> {
    harness.rows().iter().position(|row| row.contains(text))
}

fn open(harness: &mut Harness) {
    harness.press(KeyCode::Char('\\'), KeyModifiers::CONTROL);
}

// FR-KEY-07
#[test]
fn ctrl_backslash_drops_the_menu_down_below_the_focused_row_grouped_with_icons_and_keys() {
    let mut harness = Harness::new(100, 30);
    let focused = row_of(&harness, "Call the bank").expect("the first task is focused");
    open(&mut harness);
    let (top, inside) = menu(&harness).expect("the menu opens");
    assert_eq!(top, focused + 1, "right below the focused row");
    assert_eq!(
        inside.first().map(String::as_str),
        Some("›"),
        "the filter first"
    );
    assert_eq!(
        inside.get(1).map(String::as_str),
        Some("Editing and Creating"),
        "a heading before its group"
    );
    assert!(
        inside
            .iter()
            .any(|row| row.starts_with("󰏫 Edit Title") && row.ends_with(" i")),
        "each command after its icon, its keys at the right: {inside:?}"
    );
    assert!(inside.contains(&"Completing, Dates and Tags".to_owned()));
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 100, 30)),
        Some(Position::new(5, u16::try_from(top).unwrap_or_default() + 1)),
        "the cursor stands in the filter"
    );
}

// FR-KEY-07
#[test]
fn typing_filters_the_menu_keeping_the_headings_and_enter_runs_the_command() {
    let mut harness = Harness::new(100, 30);
    open(&mut harness);
    harness.type_text("duplicate");
    let (_, inside) = menu(&harness).expect("the menu is open");
    assert_eq!(inside.get(1).map(String::as_str), Some("Structure"));
    assert!(
        inside
            .get(2)
            .is_some_and(|row| row.contains("Duplicate") && row.ends_with('D'))
    );
    harness.key(KeyCode::Enter);
    assert_eq!(menu(&harness), None, "choosing closes it");
    let copies = harness
        .rows()
        .iter()
        .filter(|row| row.contains("Call the bank"))
        .count();
    assert_eq!(copies, 2, "and gives the command chosen");
    open(&mut harness);
    harness.key(KeyCode::Esc);
    assert_eq!(menu(&harness), None, "Esc closes it");
}

// FR-KEY-07
#[test]
fn near_the_bottom_the_menu_opens_above_the_focused_row() {
    let mut harness = Harness::new(100, 9);
    harness.key(KeyCode::End);
    let rows = harness.rows();
    let focused = rows
        .iter()
        .rposition(|row| row.starts_with('▌'))
        .expect("a row is focused");
    open(&mut harness);
    let (top, inside) = menu(&harness).expect("the menu opens");
    assert!(top < focused, "above it");
    assert_eq!(
        top + inside.len() + 2,
        focused,
        "its bottom border right above it"
    );
}

// FR-KEY-07, FR-PANE-02, D-106
#[test]
fn ctrl_4_opens_the_menu_too_and_alt_enter_opens_a_new_pane() {
    let mut harness = Harness::new(100, 30);
    harness.press(KeyCode::Char('4'), KeyModifiers::CONTROL);
    assert!(
        menu(&harness).is_some(),
        "as a legacy terminal sends Ctrl+\\"
    );
    harness.key(KeyCode::Esc);
    harness.press(KeyCode::Enter, KeyModifiers::ALT);
    assert!(harness.screen().contains(" 2"), "two panes, numbered");
}

// FR-KEY-07, FR-VIEW-10, D-106
#[test]
fn on_the_sidebar_the_menu_drops_down_below_a_project_but_opens_on_no_view() {
    let mut harness = Harness::new(100, 40);
    harness.ctrl('b');
    open(&mut harness);
    assert_eq!(menu(&harness), None, "nothing on the Inbox");
    harness.type_text("jjjjjjj");
    let focused = row_of(&harness, "▌# Learn Rust").expect("the project is focused");
    open(&mut harness);
    let (top, inside) = menu(&harness).expect("the menu opens on a project");
    assert_eq!(top, focused + 1);
    assert!(inside.iter().any(|row| row.contains("Rename Container")));
    assert!(!inside.iter().any(|row| row.contains("Edit Title")));
    harness.type_text("rename");
    harness.key(KeyCode::Enter);
    assert!(
        harness.tui.cursor(Rect::new(0, 0, 100, 40)).is_some(),
        "the sidebar edits the project's title"
    );
}

/// The row of the open menu that shows `title` once `query` is typed into it, which it closes.
fn filtered(harness: &mut Harness, query: &str, title: &str) -> String {
    open(harness);
    harness.type_text(query);
    let row = menu(harness)
        .and_then(|(_, inside)| {
            inside.into_iter().find(|row| {
                row.split_once(' ')
                    .is_some_and(|(_, text)| text.starts_with(title))
            })
        })
        .unwrap_or_default();
    harness.key(KeyCode::Esc);
    row
}

// FR-KEY-07
#[test]
fn each_command_has_the_icon_the_app_marks_what_it_changes_with() {
    let mut harness = Harness::new(100, 30);
    let generic = Icon::Command.glyph(IconSet::Nerd);
    for (query, title, icon) in [
        ("edit item", "Edit Item", Icon::Edit),
        ("comments", "Open Comments", Icon::Comments),
        ("complete", "Complete", Icon::DoneTask),
        ("due a day later", "Due a Day Later", Icon::Due),
        ("start today", "Start Today", Icon::Today),
        ("tags", "Tags", Icon::Tag),
        ("flag", "Toggle Flag", Icon::Flag),
        ("delete", "Delete", Icon::Trash),
        ("duplicate", "Duplicate", Icon::Duplicate),
    ] {
        let row = filtered(&mut harness, query, title);
        assert!(row.starts_with(icon.glyph(IconSet::Nerd)), "{title}: {row}");
    }
    assert!(filtered(&mut harness, "copy", "Copy").starts_with(generic));
    harness.ctrl('k');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    assert!(
        harness
            .rows()
            .iter()
            .any(|row| row.starts_with("▌") && row.contains("recruiter"))
    );
    for (query, title, icon) in [
        ("convert", "Convert to Project or Section", Icon::Project),
        ("group into section", "Group into Section", Icon::Section),
        ("group into project", "Group into Project", Icon::Project),
    ] {
        let row = filtered(&mut harness, query, title);
        assert!(row.starts_with(icon.glyph(IconSet::Nerd)), "{title}: {row}");
    }
    harness.type_text("gt");
    let row = filtered(&mut harness, "evening", "Toggle Evening");
    assert!(row.starts_with(Icon::Evening.glyph(IconSet::Nerd)), "{row}");
}
