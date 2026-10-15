//! Getting around: the Navigator, opening containers, going up and home, and going back and
//! forward.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};

use std::fs;

use crate::support::Harness;

/// The title line at the top of the pane, without the focus marker or what it has at the right.
fn title(harness: &Harness) -> String {
    harness
        .rows()
        .first()
        .and_then(|row| {
            let row = row.trim_start_matches('▌').trim();
            row.split("  ").next().map(str::to_owned)
        })
        .unwrap_or_default()
}

/// The row the focus marker is on, without the marker and trailing spaces.
fn focused(harness: &Harness) -> Option<String> {
    harness
        .rows()
        .iter()
        .find_map(|row| row.strip_prefix("▌ ").map(|row| row.trim_end().to_owned()))
}

/// The rows of the Navigator's window, between its borders, without trailing spaces.
fn navigator(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .skip_while(|row| !row.starts_with("┌Navigator"))
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

/// Opens Work › Website through the Navigator, as the plan's "Try it" does.
fn open_website(harness: &mut Harness) {
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
}

// FR-VIEW-07, FR-UI-09
#[test]
fn the_navigator_lists_the_inbox_views_areas_and_projects_as_a_tree_with_open_task_counts() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('g');
    assert_eq!(
        navigator(&harness),
        [
            "›",
            " 󰚇 Inbox                                                                    3",
            " 󰖙 Today",
            " 󱄵 Upcoming",
            " 󰈻 Flagged",
            " 󰉹 All",
            " 󱓳 Logbook",
            " 󰩺 Trash",
            " 󰆧 Work                                                                     7",
            " # Website                                                                  4",
            " # Blog                                                                     1",
            " 󰆧 Home                                                                     5",
            " # Move to the new flat                                                     4",
            " # Learn Rust                                                               2",
        ]
    );
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(3, 5)),
        "the cursor stands in the query"
    );
}

// FR-VIEW-07, FR-UI-09, FR-NAV-02, FR-NAV-04
#[test]
fn open_work_website_from_the_navigator_and_come_back() {
    let mut harness = Harness::new(80, 24);
    assert_eq!(title(&harness), "󰚇 Inbox");
    harness.ctrl('g');
    harness.type_text("web");
    assert_eq!(
        navigator(&harness),
        [
            "› web",
            " # Website  Work                                                            4"
        ],
        "a filtered entry names the places above it"
    );
    harness.key(KeyCode::Enter);
    assert_eq!(
        navigator(&harness),
        Vec::<String>::new(),
        "choosing closes it"
    );
    assert_eq!(title(&harness), "# Website");
    assert_eq!(
        harness.rows()[0].trim_end(),
        "  # Website                                                  󱇗 󰆉 2 󱙬 in 20 days"
    );
    assert_eq!(
        focused(&harness).as_deref(),
        Some("☐ Collect logos from partners")
    );
    harness.type_text("H");
    assert_eq!(title(&harness), "󰚇 Inbox");
    assert_eq!(
        focused(&harness).as_deref(),
        Some("☐ Call the bank about the card                                          󱙬 Wed")
    );
    harness.type_text("L");
    assert_eq!(title(&harness), "# Website");
    assert_eq!(harness.tui.cursor(Rect::new(0, 0, 80, 24)), None);
}

// FR-VIEW-07
#[test]
fn the_navigator_opens_on_where_the_pane_is_and_its_key_or_escape_closes_it() {
    let mut harness = Harness::new(80, 24);
    open_website(&mut harness);
    harness.ctrl('g');
    let entries = navigator(&harness);
    assert!(entries.iter().any(|entry| entry.contains("Website")));
    harness.key(KeyCode::Enter);
    assert_eq!(title(&harness), "# Website");
    harness.ctrl('g');
    harness.type_text("j");
    assert_eq!(navigator(&harness).first().map(String::as_str), Some("› j"));
    harness.key(KeyCode::Esc);
    assert_eq!(navigator(&harness), Vec::<String>::new());
    harness.ctrl('g');
    harness.ctrl('g');
    assert_eq!(navigator(&harness), Vec::<String>::new());
    assert_eq!(title(&harness), "# Website", "closing goes nowhere");
}

// FR-VIEW-07
#[test]
fn the_navigator_keeps_the_keys_but_ctrl_q_still_quits() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('g');
    harness.type_text("gh");
    assert_eq!(
        navigator(&harness).first().map(String::as_str),
        Some("› gh")
    );
    harness.press(KeyCode::Left, KeyModifiers::ALT);
    assert_eq!(title(&harness), "󰚇 Inbox");
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
}

// FR-NAV-02, FR-EDIT-01, D-80
#[test]
fn enter_opens_a_project_row_and_gi_goes_to_the_inbox() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('g');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    assert_eq!(title(&harness), "󰆧 Work");
    harness.type_text("G");
    assert!(
        focused(&harness).is_some_and(|row| row.starts_with("# Blog")),
        "{:?}",
        focused(&harness)
    );
    harness.key(KeyCode::Enter);
    assert_eq!(title(&harness), "# Blog", "Enter opens the project");
    harness.press(KeyCode::Left, KeyModifiers::ALT);
    harness.type_text(">");
    assert_eq!(title(&harness), "# Blog", "nothing goes up any more");
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
    harness.type_text("j");
    assert_eq!(focused(&harness).as_deref(), Some("/ Launch"));
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Esc);
    assert_eq!(
        title(&harness),
        "# Website",
        "a section only has its title edited"
    );
    harness.type_text("gh");
    assert_eq!(title(&harness), "# Website", "there is no home");
    harness.type_text("gi");
    assert_eq!(title(&harness), "󰚇 Inbox");
    harness.press(KeyCode::Left, KeyModifiers::ALT | KeyModifiers::SHIFT);
    assert_eq!(title(&harness), "# Website");
}

// FR-NAV-04
#[test]
fn back_and_forward_return_with_the_focus_where_it_was() {
    let mut harness = Harness::new(80, 24);
    open_website(&mut harness);
    harness.type_text("j");
    harness.ctrl('g');
    harness.type_text("blog");
    harness.key(KeyCode::Enter);
    assert_eq!(title(&harness), "# Blog");
    harness.type_text("H");
    assert_eq!(focused(&harness).as_deref(), Some("/ Launch"));
    harness.type_text("H");
    assert_eq!(title(&harness), "󰚇 Inbox");
    harness.type_text("H");
    assert_eq!(title(&harness), "󰚇 Inbox", "nothing is further back");
    harness.press(KeyCode::Right, KeyModifiers::ALT | KeyModifiers::SHIFT);
    harness.type_text("L");
    assert_eq!(title(&harness), "# Blog");
    harness.type_text("L");
    assert_eq!(title(&harness), "# Blog", "nothing is further ahead");
}

// FR-VIEW-07
#[test]
fn a_place_without_open_tasks_shows_no_count() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    fs::write(&path, "Inbox:\n\t- One\nSomeday:\n\t- Done @done\n").expect("written");
    let mut harness = Harness::open(path, 40, 12);
    harness.ctrl('g');
    assert_eq!(
        navigator(&harness),
        [
            "›",
            " 󰚇 Inbox                            1",
            " 󰖙 Today",
            " 󱄵 Upcoming",
            " 󰈻 Flagged",
            " 󰉹 All",
            " 󱓳 Logbook",
            " 󰩺 Trash",
            " # Someday"
        ]
    );
}

// FR-VIEW-07, FR-EDIT-01, FR-ATT-01, D-137
#[test]
fn the_navigator_opened_while_editing_ends_the_edit_kept_and_takes_the_keys() {
    let edits = [
        ("a title", "\n", "Call the bank about the cardA"),
        ("a description", "mn", "   A"),
        ("the description of what is shown", "mN", "lands here.A"),
    ];
    for (edit, keys, kept) in edits {
        let mut harness = Harness::new(80, 16);
        for key in keys.chars() {
            match key {
                '\n' => harness.key(KeyCode::Enter),
                key => harness.type_text(&key.to_string()),
            }
        }
        harness.type_text("A");
        harness.ctrl('g');
        harness.type_text("in");
        assert_eq!(
            navigator(&harness).first().map(String::as_str),
            Some("› in"),
            "the Navigator takes what is typed after {edit}:\n{}",
            harness.screen()
        );
        harness.key(KeyCode::Esc);
        harness.type_text("B");
        let screen = harness.screen();
        assert!(
            screen.contains(kept) && !screen.contains("AB") && !screen.contains("Ain"),
            "{edit} ended, kept, and nothing after went into it:\n{screen}"
        );
        assert!(
            screen.contains("NAVIGATE"),
            "back in navigate mode after {edit}"
        );
    }
    let mut harness = Harness::new(80, 16);
    harness.ctrl('b');
    harness.type_text("GRA");
    harness.ctrl('g');
    harness.type_text("in");
    assert_eq!(
        navigator(&harness).first().map(String::as_str),
        Some("› in"),
        "and after a rename in the sidebar:\n{}",
        harness.screen()
    );
    harness.key(KeyCode::Esc);
    harness.type_text("B");
    let screen = harness.screen();
    assert!(
        screen.contains("new flatA") && !screen.contains("AB") && !screen.contains("Ain"),
        "which ends, kept:\n{screen}"
    );
    let mut harness = Harness::new(80, 16);
    harness.type_text("CkkeA");
    harness.ctrl('g');
    harness.type_text("in");
    let screen = harness.screen();
    assert!(
        navigator(&harness).is_empty() && screen.contains("│▌Ain"),
        "the attachments panel does not offer it while its note is edited:\n{screen}"
    );
}
