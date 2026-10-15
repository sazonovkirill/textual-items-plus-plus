//! The mouse: clicking rows, checkboxes, sidebar entries and picker entries, and turning the
//! wheel over panes, the sidebar, the attachments panel and pickers (FR-KEY-05, D-138).

use std::{fs, path::PathBuf};

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tempfile::TempDir;

use crate::support::Harness;

/// The mouse doing `kind` at `column` and `row`.
fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// A click at `column` and `row`.
fn click(harness: &mut Harness, column: u16, row: u16) {
    harness.send(&mouse(MouseEventKind::Down(MouseButton::Left), column, row));
}

/// A notch of the wheel down, or up when `up`, at `column` and `row`.
fn wheel(harness: &mut Harness, up: bool, column: u16, row: u16) {
    let kind = if up {
        MouseEventKind::ScrollUp
    } else {
        MouseEventKind::ScrollDown
    };
    harness.send(&mouse(kind, column, row));
}

/// The app on a file of three tasks in the Inbox, on a terminal 40 by 8, where the tasks are on
/// rows 2 to 4.
fn three_tasks() -> (TempDir, PathBuf, Harness) {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- Call\n\t- Buy\n\t- Read\n").expect("written");
    let harness = Harness::open(path.clone(), 40, 8);
    (dir, path, harness)
}

/// The rows 2 to 4 of the screen, trimmed.
fn tasks(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    rows.get(2..5)
        .unwrap_or_default()
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

// FR-KEY-05, D-138
#[test]
fn a_click_on_a_row_focuses_it_picking_nothing() {
    let (_dir, _path, mut harness) = three_tasks();
    harness.key(KeyCode::Char('J'));
    click(&mut harness, 20, 4);
    assert_eq!(tasks(&harness), ["  ☐ Call", "  ☐ Buy", "▌ ☐ Read"]);
    click(&mut harness, 30, 2);
    assert_eq!(
        tasks(&harness),
        ["▌ ☐ Call", "  ☐ Buy", "  ☐ Read"],
        "back up, past the end of the title"
    );
    harness.key(KeyCode::Char('x'));
    assert_eq!(
        tasks(&harness),
        ["▌ ☐ Buy", "  ☐ Read", ""],
        "the keys act on the row clicked alone"
    );
}

// FR-KEY-05, FR-TASK-02, D-138
#[test]
fn a_click_on_a_tasks_checkbox_completes_it_and_another_uncompletes_it() {
    let (_dir, path, mut harness) = three_tasks();
    click(&mut harness, 2, 3);
    assert_eq!(tasks(&harness), ["  ☐ Call", "▌ ☐ Read", ""]);
    harness.ctrl('o');
    assert_eq!(
        tasks(&harness),
        ["  ☐ Call", "▌ ☐ Read", "  ✔ Buy"],
        "completed, shown last"
    );
    click(&mut harness, 2, 4);
    assert_eq!(tasks(&harness), ["  ☐ Call", "▌ ☐ Buy", "  ☐ Read"]);
    click(&mut harness, 3, 2);
    assert_eq!(
        tasks(&harness),
        ["▌ ☐ Call", "  ☐ Buy", "  ☐ Read"],
        "beside the checkbox only focuses"
    );
    click(&mut harness, 2, 0);
    assert!(
        harness.rows()[0].contains("Inbox"),
        "the heading has no checkbox"
    );
    click(&mut harness, 2, 4);
    harness.terminate();
    let saved = fs::read_to_string(&path).expect("saved");
    assert!(
        saved.starts_with("Inbox:\n\t- Call\n\t- Buy\n\t- Read @done("),
        "{saved}"
    );
}

// FR-KEY-05, D-138
#[test]
fn the_wheel_over_a_pane_moves_its_focus_three_rows() {
    let mut harness = Harness::new(60, 12);
    harness.type_text("gt");
    let focused = |harness: &Harness| -> String {
        let rows = harness.rows();
        let row = rows.iter().find(|row| row.starts_with('▌'));
        row.map(|row| row.trim_end().to_owned()).unwrap_or_default()
    };
    let first = focused(&harness);
    wheel(&mut harness, false, 20, 6);
    let down = focused(&harness);
    assert_ne!(down, first);
    for _ in 0..3 {
        harness.key(KeyCode::Char('k'));
    }
    assert_eq!(focused(&harness), first, "three rows down, as `j` thrice");
    wheel(&mut harness, false, 20, 6);
    wheel(&mut harness, true, 20, 6);
    assert_eq!(focused(&harness), first, "and three up");
}

// FR-KEY-05, FR-PANE-02, FR-VIEW-10, D-138
#[test]
fn a_click_in_a_pane_gives_it_the_keys_from_the_sidebar_or_another_pane() {
    let mut harness = Harness::launched(100, 20);
    harness.key(KeyCode::Char('|'));
    assert!(
        harness.rows()[0].starts_with('▌'),
        "the sidebar has the keys"
    );
    click(&mut harness, 60, 10);
    let rows = harness.rows();
    assert!(!rows[0].starts_with('▌'), "{}", harness.screen());
    assert!(
        rows[3].contains("│▌ ☐ Call the bank"),
        "{}",
        harness.screen()
    );
    harness.ctrl('t');
    let screen = harness.screen();
    assert!(screen.contains("│▌ ☐ Call the bank"), "{screen}");
    click(&mut harness, 40, 6);
    let rows = harness.rows();
    assert!(
        rows[6].contains("│▌ ☐ Buy coffee beans"),
        "the first pane has the keys again, on the row clicked: {}",
        harness.screen()
    );
}

// FR-KEY-05, FR-VIEW-10, D-138
#[test]
fn a_click_on_a_sidebar_entry_shows_it_and_the_wheel_browses() {
    let mut harness = Harness::launched(100, 20);
    click(&mut harness, 5, 12);
    let rows = harness.rows();
    assert!(rows[0].contains("Website"), "{}", harness.screen());
    assert!(!rows[12].starts_with('▌'), "the keys stay in the pane");
    click(&mut harness, 5, 0);
    assert!(
        harness.rows()[0].contains("󰚇 Inbox"),
        "{}",
        harness.screen()
    );
    wheel(&mut harness, false, 5, 18);
    assert!(
        harness.rows()[0].contains("Flagged"),
        "three entries down: {}",
        harness.screen()
    );
    wheel(&mut harness, true, 5, 18);
    assert!(
        harness.rows()[0].contains("󰚇 Inbox"),
        "{}",
        harness.screen()
    );
}

// FR-KEY-05, FR-VIEW-07, D-138
#[test]
fn a_click_on_a_navigator_entry_goes_there_and_one_outside_closes_it() {
    let mut harness = Harness::launched(100, 20);
    harness.ctrl('k');
    click(&mut harness, 40, 3);
    assert!(
        harness.screen().contains("Navigator"),
        "the query line does nothing"
    );
    wheel(&mut harness, false, 40, 10);
    wheel(&mut harness, false, 40, 10);
    wheel(&mut harness, false, 40, 10);
    wheel(&mut harness, false, 40, 10);
    click(&mut harness, 40, 13);
    let screen = harness.screen();
    assert!(!screen.contains("Navigator"), "{screen}");
    assert!(
        harness.rows()[0].contains("│  # Blog"),
        "back up to an entry above the highlight: {screen}"
    );
    harness.ctrl('k');
    click(&mut harness, 40, 12);
    assert!(
        harness.rows()[0].contains("│  # Website"),
        "{}",
        harness.screen()
    );
    harness.ctrl('k');
    click(&mut harness, 95, 18);
    let screen = harness.screen();
    assert!(!screen.contains("Navigator"), "{screen}");
    assert!(harness.rows()[0].contains("│  # Website"), "nothing chosen");
}

// FR-KEY-05, FR-UI-09, D-138
#[test]
fn the_wheel_moves_the_highlight_of_an_open_picker_three_entries() {
    let mut harness = Harness::launched(100, 20);
    harness.ctrl('k');
    wheel(&mut harness, false, 2, 2);
    wheel(&mut harness, false, 2, 2);
    wheel(&mut harness, true, 2, 2);
    harness.key(KeyCode::Enter);
    let screen = harness.screen();
    assert!(harness.rows()[0].contains("│  󰈻 Flagged"), "{screen}");
}

// FR-KEY-05, FR-TAG-02, D-138
#[test]
fn a_click_on_an_entry_of_the_edit_dialogs_tag_picker_chooses_it() {
    let mut harness = Harness::new(80, 16);
    harness.key(KeyCode::Char('e'));
    harness.key(KeyCode::Tab);
    harness.key(KeyCode::Tab);
    harness.key(KeyCode::Char(' '));
    let rows = harness.rows();
    let (row, line) = rows
        .iter()
        .enumerate()
        .find(|(_, line)| line.contains("errands"))
        .expect("the picker lists errands");
    let column = line.find("errands").expect("there");
    let column = u16::try_from(line[..column].chars().count()).expect("on screen");
    let row = u16::try_from(row).expect("on screen");
    click(&mut harness, column, row);
    let screen = harness.screen();
    assert!(screen.contains("Tags         @errands"), "{screen}");
}

// FR-KEY-05, FR-ATT-01, D-138
#[test]
fn the_wheel_over_the_attachments_panel_with_the_keys_scrolls_it() {
    let mut harness = Harness::new(80, 16);
    harness.key(KeyCode::Char('c'));
    let selected = |harness: &Harness| -> String {
        let rows = harness.rows();
        rows.iter()
            .find_map(|row| row.split_once("│▌").map(|(_, rest)| rest.trim().to_owned()))
            .unwrap_or_default()
    };
    assert!(selected(&harness).starts_with("No checklist"));
    wheel(&mut harness, false, 2, 4);
    assert!(
        selected(&harness).starts_with("No checklist"),
        "not over the panel"
    );
    wheel(&mut harness, false, 60, 4);
    assert!(
        selected(&harness).starts_with("No comments"),
        "{}",
        harness.screen()
    );
    wheel(&mut harness, true, 60, 4);
    wheel(&mut harness, true, 60, 4);
    assert!(
        selected(&harness).starts_with("No note"),
        "{}",
        harness.screen()
    );
    harness.key(KeyCode::Tab);
    let screen = harness.screen();
    assert!(
        screen.contains("▌ ☐ Call the bank"),
        "the list has the keys: {screen}"
    );
    wheel(&mut harness, false, 60, 4);
    assert_eq!(harness.screen(), screen, "the panel without the keys stays");
    wheel(&mut harness, false, 2, 4);
    assert_ne!(harness.screen(), screen, "the list takes the wheel");
}

// FR-KEY-05, D-138
#[test]
fn while_a_prompt_has_the_keys_the_mouse_does_nothing() {
    let (_dir, _path, mut harness) = three_tasks();
    harness.type_text("md");
    let screen = harness.screen();
    click(&mut harness, 2, 3);
    click(&mut harness, 20, 4);
    wheel(&mut harness, false, 20, 4);
    assert_eq!(harness.screen(), screen);
}

// FR-KEY-05, D-138
#[test]
fn moving_dragging_releasing_and_other_buttons_do_nothing() {
    let (_dir, _path, mut harness) = three_tasks();
    let screen = harness.screen();
    for kind in [
        MouseEventKind::Moved,
        MouseEventKind::Drag(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::ScrollLeft,
    ] {
        harness.send(&mouse(kind, 20, 4));
    }
    assert_eq!(harness.screen(), screen);
}

// FR-KEY-05, D-138
#[test]
fn a_click_where_a_row_scrolled_partly_away_shows_no_checkbox_only_focuses_it() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\n\t- Call\n\t\tc\n\t- Buy\n\t\tb\n\t- Read\n\t\tr\n\t- Pay\n\t\tp\n";
    fs::write(&path, text).expect("written");
    let mut harness = Harness::open(path.clone(), 40, 8);
    harness.key(KeyCode::Char('G'));
    assert_eq!(
        harness.rows()[0].trim_end(),
        "    c",
        "Call's first line went"
    );
    click(&mut harness, 2, 0);
    assert_eq!(
        harness.rows()[2].trim_end(),
        "▌ ☐ Call",
        "focused, scrolled into view"
    );
    harness.terminate();
    let saved = fs::read_to_string(&path).expect("saved");
    assert_eq!(saved, text, "nothing completed");
}
