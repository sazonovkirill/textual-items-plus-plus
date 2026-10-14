//! Go to Symbol.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

/// The rows of the window titled Go to Symbol, between its borders, without trailing spaces.
fn window(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .skip_while(|row| !row.starts_with("┌Go to Symbol"))
        .skip(1)
        .take_while(|row| !row.starts_with('└'))
        .map(|row| {
            row.trim_start_matches('│')
                .trim_end()
                .trim_end_matches('│')
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// The rows the focus marker is on, without the marker and trailing spaces.
fn focused(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .filter_map(|row| row.strip_prefix("▌ ").map(|row| row.trim_end().to_owned()))
        .collect()
}

// FR-NAV-05, FR-UI-09
#[test]
fn slash_lists_areas_projects_and_sections_then_tasks_where_they_are_and_typing_filters_them() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("/");
    let listed = window(&harness);
    assert_eq!(
        listed.get(..4),
        Some(
            [
                "›",
                " \u{f01a7} Work",
                " # Website  Work",
                " / Launch  Work › Website",
            ]
            .map(str::to_owned)
            .as_slice()
        )
    );
    assert!(listed.contains(&" - Buy coffee beans  Inbox".to_owned()));
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(3, 3)),
        "the cursor stands in the query"
    );
    harness.type_text("write");
    assert_eq!(
        window(&harness).get(1..3),
        Some(
            [
                " - Write the announcement  Work › Website › Launch",
                " - Write a small CLI  Learn Rust",
            ]
            .map(str::to_owned)
            .as_slice()
        )
    );
    harness.key(KeyCode::Enter);
    assert_eq!(window(&harness), Vec::<String>::new(), "choosing closes it");
    assert!(harness.rows()[0].contains("Work › Website"));
    assert!(
        focused(&harness)
            .first()
            .is_some_and(|row| row.starts_with("- Write the announcement")),
        "the task chosen has the focus where it is"
    );
}

// FR-NAV-05, FR-PANE-02
#[test]
fn alt_enter_opens_the_item_chosen_in_a_new_pane_and_esc_closes_it() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("/blog");
    harness.press(KeyCode::Enter, KeyModifiers::ALT);
    let header = harness.rows()[0].clone();
    assert!(
        header.contains("Inbox") && header.contains('│') && header.contains("Blog"),
        "{header}"
    );
    harness.type_text("/");
    harness.key(KeyCode::Esc);
    assert_eq!(window(&harness), Vec::<String>::new());
    assert!(harness.rows()[0].contains("Blog"));
}
