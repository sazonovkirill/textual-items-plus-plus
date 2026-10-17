//! The keymap window: every key binding, grouped, as a cheat sheet over most of the screen
//! (FR-KEY-08).

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEvent, MouseEventKind};

use crate::support::Harness;

/// The row of the window's first line inside its frame, on a terminal 160 by 50.
const FIRST: usize = 4;

/// Whether the window is drawn.
fn open(harness: &Harness) -> bool {
    harness.screen().contains("┌ Keymap ")
}

// FR-KEY-08, FR-KEY-02
#[test]
fn g_question_mark_lists_every_key_in_columns_and_esc_closes_it() {
    let mut harness = Harness::new(160, 50);
    harness.type_text("g?");
    let rows = harness.rows();
    assert!(
        rows[FIRST - 1].starts_with("▌ ☐ Call┌ Keymap ─") && rows[47].contains("Esc close ┘"),
        "nine tenths of the screen: {}",
        harness.screen()
    );
    let columns = rows[FIRST]
        .split('│')
        .nth(1)
        .map(|inside| inside.split("  ").filter(|text| !text.trim().is_empty()))
        .map(Iterator::count);
    assert_eq!(
        columns,
        Some(4),
        "four columns, each headed by a group: {rows:#?}"
    );
    let mut sheet = Vec::new();
    loop {
        let page = harness.rows();
        if sheet.last() == Some(&page) {
            break;
        }
        sheet.push(page);
        harness.ctrl('d');
    }
    let lines: Vec<&String> = sheet.iter().flatten().collect();
    for (name, keys) in [
        ("Open or Edit", "Enter"),
        ("Edit Title", "i"),
        ("Go to Symbol", "/"),
        ("Paste Above", "Alt+v, gP"),
        ("Save As…", "Ctrl+S"),
    ] {
        assert!(
            lines.iter().any(|row| row
                .split_once(&format!(" {name} "))
                .is_some_and(|(_, rest)| rest.trim_start().starts_with(&format!("{keys} ")))),
            "{name} with {keys}: {sheet:#?}"
        );
    }
    for window in [
        "Edit Dialog",
        "Date Prompt",
        "Editing Checklist Item",
        "Text",
    ] {
        assert!(
            lines
                .iter()
                .any(|row| row.contains(&format!(" {window} "))
                    || row.contains(&format!("│{window} "))),
            "the windows' own keys too, {window}: {sheet:#?}"
        );
    }
    harness.key(KeyCode::Esc);
    assert!(!open(&harness));
}

// FR-KEY-08
#[test]
fn its_keys_scroll_it_and_no_other_command_gets_through() {
    let mut harness = Harness::new(160, 50);
    let before = harness.screen();
    harness.type_text("g?");
    let top = harness.rows()[FIRST].clone();
    harness.type_text("jx");
    assert_ne!(harness.rows()[FIRST], top, "j scrolls the columns a line");
    harness.type_text("k");
    assert_eq!(harness.rows()[FIRST], top);
    harness.type_text("G");
    let screen = harness.screen();
    assert!(
        screen.contains("Save As…") && !screen.contains("│Moving "),
        "G goes to the end: {screen}"
    );
    harness.send(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 80,
        row: 20,
        modifiers: KeyModifiers::NONE,
    }));
    harness.press(KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(harness.rows()[FIRST], top, "Home goes back to the top");
    harness.ctrl('d');
    let paged = harness.rows()[FIRST].clone();
    harness.ctrl('u');
    assert_eq!(harness.rows()[FIRST], top);
    harness.type_text(&"j".repeat(43));
    assert_eq!(
        harness.rows()[FIRST],
        paged,
        "Ctrl+D scrolls the 43 lines shown"
    );
    harness.press(KeyCode::Home, KeyModifiers::NONE);
    harness.key(KeyCode::Esc);
    assert_eq!(
        harness.screen(),
        before,
        "x completed nothing, j moved no focus"
    );
    harness.type_text("g?");
    assert_eq!(harness.rows()[FIRST], top, "it opens at the top again");
    harness.ctrl('g');
    assert!(!open(&harness), "the Navigator takes its place");
}

// FR-KEY-08
#[test]
fn the_wheel_scrolls_it_wherever_the_pointer_is() {
    let mut harness = Harness::new(160, 50);
    harness.type_text("?g?");
    let top = harness.rows()[FIRST].clone();
    for column in [80, 158] {
        harness.send(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column,
            row: 20,
            modifiers: KeyModifiers::NONE,
        }));
        assert_ne!(harness.rows()[FIRST], top, "over column {column}");
        harness.press(KeyCode::Home, KeyModifiers::NONE);
    }
    assert!(open(&harness), "it stays open");
}
