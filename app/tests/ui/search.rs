//! Full-text search: the words typed find items by any of their text, wherever they live, and
//! a result opens where it lives.

use crossterm::event::{Event, KeyCode};
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

/// The rows of the screen, without trailing spaces.
fn trimmed(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

/// The rows of the pane under its title line, up to the status line.
fn results(harness: &Harness) -> Vec<String> {
    let rows = trimmed(harness);
    let status = rows.len() - 1;
    rows.into_iter()
        .take(status)
        .skip(1)
        .filter(|row| !row.is_empty())
        .collect()
}

// FR-SRCH-01, FR-SRCH-02, FR-ATT-04, FR-KEY-01
#[test]
fn search_a_word_only_in_a_comment_and_open_the_task_where_it_lives() {
    let mut harness = Harness::new(80, 20);
    harness.ctrl('f');
    harness.type_text("Screenshots");
    let rows = trimmed(&harness);
    assert_eq!(rows[0], "  \u{f0349} Search  Screenshots");
    assert_eq!(
        results(&harness),
        [
            "▌ ☐ Write the announcement @writing · Work › Website ›      \u{f0756} 1/5 \u{f11d7} \u{f0189} 1 \u{f166d} today",
            "▌   Launch",
            "▌   \u{f0189} Ask design for screenshots",
        ],
        "where the task is follows its title, and the comment holding the word shows under it"
    );
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 20)),
        Some(Position::new(23, 0)),
        "the caret is after the words typed"
    );
    harness.key(KeyCode::Enter);
    assert_eq!(harness.tui.cursor(Rect::new(0, 0, 80, 20)), None);
    harness.type_text(".");
    let rows = trimmed(&harness);
    assert!(rows[0].starts_with("  # Website "), "{rows:#?}");
    assert!(
        rows.iter()
            .any(|row| row.starts_with("▌ ☐ Write the announcement")),
        "{rows:#?}"
    );
    harness.type_text("H");
    assert_eq!(
        trimmed(&harness)[0],
        "  \u{f0349} Search  Screenshots",
        "the results stay in the pane's history with their words"
    );
}

// FR-SRCH-03, FR-VIEW-04, OQ-14
#[test]
fn completed_items_are_found_and_marked_while_the_trash_is_left_out() {
    let mut harness = Harness::new(70, 16);
    harness.type_text("gs");
    harness.type_text("hero");
    assert_eq!(
        results(&harness),
        ["▌ ✔ Compress the hero image · Work › Website › Polish"]
    );
    harness.ctrl('u');
    harness.type_text("newspaper");
    assert_eq!(
        results(&harness),
        Vec::<String>::new(),
        "deleted, so left out"
    );
}

// FR-SRCH-02, FR-PANE-02
#[test]
fn a_result_opens_in_a_new_pane() {
    let mut harness = Harness::new(100, 16);
    harness.ctrl('f');
    harness.type_text("portfolios");
    harness.key(KeyCode::Esc);
    harness.ctrl('\\');
    let rows = trimmed(&harness);
    assert!(
        rows[0].starts_with("  \u{f0349} Search ") && rows[0].contains("│  \u{f01a7} Work "),
        "{rows:#?}"
    );
    assert!(
        rows.iter()
            .any(|row| row.contains("│▌ ☐ Review two portfolios")),
        "{rows:#?}"
    );
}

// FR-TAG-04, FR-SRCH-01
#[test]
fn search_results_are_filtered_by_tags() {
    let mut harness = Harness::new(80, 20);
    harness.ctrl('f');
    harness.type_text("the");
    harness.key(KeyCode::Enter);
    assert!(results(&harness).len() > 2, "{:#?}", results(&harness));
    assert!(
        results(&harness)
            .iter()
            .any(|row| row.contains("Call the bank"))
    );
    harness.type_text("gf");
    harness.type_text("writing");
    harness.key(KeyCode::Enter);
    let filtered = results(&harness);
    assert!(
        filtered
            .iter()
            .any(|row| row.contains("Write the announcement"))
            && !filtered.iter().any(|row| row.contains("Call the bank")),
        "{filtered:#?}"
    );
}

// FR-SRCH-01, FR-OPS-05
#[test]
fn text_pasted_goes_into_the_words_typed() {
    let mut harness = Harness::new(80, 16);
    harness.type_text("gs");
    harness.send(&Event::Paste("coffee\nbeans".to_owned()));
    assert_eq!(trimmed(&harness)[0], "  \u{f0349} Search  coffee beans");
    assert_eq!(results(&harness), ["▌ ☐ Buy coffee beans @errands · Inbox"]);
}

// FR-KEY-03, FR-NAV-06
#[test]
fn the_shortcuts_panel_lists_the_keys_of_the_words_typed_and_the_palette_searches() {
    let mut harness = Harness::new(120, 40);
    harness.type_text("?");
    harness.ctrl('f');
    let screen = harness.screen();
    assert!(screen.contains("Go to Results"), "{screen}");
    harness.key(KeyCode::Esc);
    harness.type_text("gh");
    harness.ctrl('p');
    harness.type_text("search");
    harness.key(KeyCode::Enter);
    harness.type_text("bank");
    assert_eq!(
        trimmed(&harness)[0].trim_end_matches(|c: char| c != 'k'),
        "  \u{f0349} Search  bank"
    );
}
