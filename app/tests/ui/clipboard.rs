//! The clipboard: duplicating, copying, cutting and pasting items, and text pasted into the
//! terminal.

use std::{
    fs,
    sync::{Arc, Mutex},
};

use crossterm::event::{Event, KeyCode, KeyModifiers};
use textual_items_plus_plus::ui::SystemClipboard;

use crate::support::Harness;

/// The rows of the pane, without trailing spaces or blank rows.
fn pane(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .map(|row| row.trim_end().to_owned())
        .filter(|row| !row.is_empty() && !row.contains("NAVIGATE"))
        .collect()
}

/// A system clipboard the test reads and writes.
#[derive(Debug, Clone, Default)]
struct Shared(Arc<Mutex<Option<String>>>);

impl Shared {
    fn text(&self) -> Option<String> {
        self.0.lock().expect("not poisoned").clone()
    }

    fn put(&self, text: &str) {
        *self.0.lock().expect("not poisoned") = Some(text.to_owned());
    }
}

impl SystemClipboard for Shared {
    fn set(&mut self, text: &str) {
        self.put(text);
    }

    fn get(&mut self) -> Option<String> {
        self.text()
    }
}

const INBOX: [&str; 2] = ["  󰚇 Inbox", "  Everything not sorted yet lands here."];

/// The pane's rows below the Inbox's heading and description.
fn tasks(harness: &Harness) -> Vec<String> {
    let rows = pane(harness);
    assert_eq!(rows.get(..2), Some(INBOX.map(str::to_owned).as_slice()));
    rows.into_iter().skip(2).collect()
}

// FR-OPS-05, FR-EDIT-06
#[test]
fn duplicate_copy_cut_and_paste_tasks_in_the_inbox() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("D");
    assert_eq!(
        tasks(&harness),
        [
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "▌ ☐ Call the bank about the card                                          󱙬 Wed",
            "  ☐ Buy coffee beans @errands",
            "  ☐ Read the article Sam sent",
        ],
        "the copy goes right after and takes the focus"
    );
    harness.type_text("z");
    harness.type_text("yj");
    harness.press(KeyCode::Char('v'), KeyModifiers::ALT);
    assert_eq!(
        tasks(&harness),
        [
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "▌ ☐ Call the bank about the card                                          󱙬 Wed",
            "  ☐ Buy coffee beans @errands",
            "  ☐ Read the article Sam sent",
        ],
        "pasting above the next task puts the copy before it"
    );
    harness.type_text("z");
    harness.ctrl('x');
    assert_eq!(
        tasks(&harness),
        [
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "▌ ☐ Read the article Sam sent",
        ],
        "cutting takes the task out"
    );
    harness.ctrl('v');
    assert_eq!(
        tasks(&harness),
        [
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "  ☐ Read the article Sam sent",
            "▌ ☐ Buy coffee beans @errands",
        ]
    );
    harness.press(KeyCode::Char('v'), KeyModifiers::ALT);
    assert_eq!(
        tasks(&harness),
        [
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "  ☐ Read the article Sam sent",
            "▌ ☐ Buy coffee beans @errands",
            "  ☐ Buy coffee beans @errands",
        ]
    );
}

// FR-OPS-05
#[test]
fn copy_to_and_paste_a_list_from_the_system_clipboard() {
    let mut harness = Harness::new(80, 24);
    let shared = Shared::default();
    harness.tui.use_clipboard(Box::new(shared.clone()));
    harness.type_text("y");
    assert_eq!(
        shared.text().as_deref(),
        Some("- Call the bank about the card @due(2026-10-07)\n")
    );
    shared.put("Milk\nEggs @errands\n");
    harness.ctrl('v');
    assert_eq!(
        tasks(&harness),
        [
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "  ☐ Milk",
            "▌ ☐ Eggs @errands",
            "  ☐ Buy coffee beans @errands",
            "  ☐ Read the article Sam sent",
        ],
        "each line is a task"
    );
}

// FR-OPS-05, FR-EDIT-04
#[test]
fn text_pasted_into_the_terminal_becomes_tasks_or_goes_into_the_text_typed() {
    let mut harness = Harness::new(80, 24);
    let paste = |harness: &mut Harness, text: &str| harness.send(&Event::Paste(text.to_owned()));
    paste(&mut harness, "Milk\r\n\r\n- Eggs\n");
    assert_eq!(
        tasks(&harness),
        [
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "  ☐ Milk",
            "▌ ☐ Eggs",
            "  ☐ Buy coffee beans @errands",
            "  ☐ Read the article Sam sent",
        ]
    );
    harness.key(KeyCode::Enter);
    paste(&mut harness, " and bread\n");
    harness.key(KeyCode::Esc);
    assert_eq!(
        tasks(&harness)[2],
        "▌ ☐ Eggs and bread",
        "a single line goes into the title"
    );
    harness.type_text("o");
    paste(&mut harness, "Tea\nCake");
    assert_eq!(
        tasks(&harness),
        [
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "  ☐ Milk",
            "  ☐ Eggs and bread",
            "  ☐ Tea",
            "▌ ☐ Cake",
            "  ☐ Buy coffee beans @errands",
            "  ☐ Read the article Sam sent",
        ],
        "several lines end editing, the empty new task left out, and paste below"
    );
    harness.ctrl('g');
    paste(&mut harness, "Learn\nRust");
    assert!(
        harness.screen().contains("› Learn Rust"),
        "{}",
        harness.screen()
    );
    harness.key(KeyCode::Esc);
    harness.type_text("gm");
    paste(&mut harness, "Hiring");
    assert!(
        harness.screen().contains("› Hiring"),
        "{}",
        harness.screen()
    );
}

// FR-OPS-05, F-FILE-06
#[test]
fn the_conflict_prompt_takes_no_paste() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- Renew the passport\n").expect("written");
    let mut harness = Harness::open(path.clone(), 70, 20);
    harness.type_text("x");
    fs::write(&path, "Inbox:\n\t- Book trains\n").expect("written");
    harness.check();
    let screen = harness.screen();
    harness.send(&Event::Paste("Milk".to_owned()));
    assert_eq!(harness.screen(), screen);
}

// FR-OPS-05, FR-ORG-10
#[test]
fn text_pasted_into_a_description_keeps_its_lines() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("mn");
    harness.send(&Event::Paste("First line\r\nsecond line\n".to_owned()));
    let rows = tasks(&harness);
    assert_eq!(
        rows.get(..3),
        Some(
            [
                "▌ ☐ Call the bank about the card                                          󱙬 Wed",
                "▌   First line",
                "▌   second line",
            ]
            .map(str::to_owned)
            .as_slice()
        ),
        "{rows:?}"
    );
}

// FR-EDIT-04, FR-OPS-05
#[test]
fn ctrl_v_pastes_the_clipboard_text_into_the_text_edited() {
    let mut harness = Harness::new(80, 24);
    let shared = Shared::default();
    harness.tui.use_clipboard(Box::new(shared.clone()));
    shared.put(" today\n");
    harness.type_text("i");
    harness.ctrl('v');
    harness.key(KeyCode::Esc);
    assert_eq!(
        tasks(&harness)[0],
        "▌ ☐ Call the bank about the card today                                    󱙬 Wed",
        "a single line goes into the title"
    );
    shared.put("First line\nsecond line\n");
    harness.type_text("mn");
    harness.ctrl('v');
    assert_eq!(
        tasks(&harness).get(1..3),
        Some(
            ["▌   First line", "▌   second line"]
                .map(str::to_owned)
                .as_slice()
        ),
        "a description takes the text with its lines"
    );
}

// FR-OPS-05, D-68, D-103
#[test]
fn pasting_on_the_title_row_puts_the_items_first() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("y");
    harness.ctrl('k');
    harness.type_text("Work");
    harness.key(KeyCode::Enter);
    harness.type_text("gg");
    let before = pane(&harness);
    harness.press(KeyCode::Char('v'), KeyModifiers::ALT);
    assert_eq!(pane(&harness), before, "no row stands above the title row");
    harness.ctrl('v');
    let call = "▌ ☐ Call the bank about the card                                          󱙬 Wed";
    assert_eq!(pane(&harness)[2], call, "the first task");
    harness.type_text("gg");
    harness.send(&Event::Paste("Tea".to_owned()));
    assert_eq!(
        pane(&harness)[2..5],
        [
            "▌ ☐ Tea",
            "  ☐ Call the bank about the card                                          󱙬 Wed",
            "  ☐ Answer the recruiter's email                                    󰨱 yesterday",
        ]
    );
}
