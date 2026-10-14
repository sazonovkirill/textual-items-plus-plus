//! The Today view: what is due or has started, in one list, overdue first, each row saying where
//! it is.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::support::Harness;

/// The rows of the screen, without trailing spaces.
fn trimmed(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

// FR-VIEW-06, FR-KEY-06, FR-DATE-04, D-51
#[test]
fn open_today() {
    let mut harness = Harness::new(60, 16);
    harness.type_text("gt");
    let today = [
        "  \u{f0599} Today",
        "",
        "▌ ☐ Answer the recruiter's email                \u{f0a31} yesterday",
        "▌   Work",
        "  ☐ Write the announcement @writing     \u{f0756} 1/5 \u{f11d7} \u{f0189} 1 \u{f166d} today",
        "    Work › Website › Launch",
        "  ☐ Review two portfolios                           \u{f166c} today",
        "    Work › Hiring",
        "  ☐ Water the plants                            \u{f166d} yesterday",
        "    Home",
        "",
        "",
        "",
        "",
        "",
        " \u{f01bf} NAVIGATE · Demo data · not saved",
    ];
    assert_eq!(
        trimmed(&harness),
        today,
        "one list, the overdue task first, each row saying where it is under its title instead of its description (D-51)"
    );
}

// FR-VIEW-06, FR-NAV-02, FR-NAV-04
#[test]
fn a_task_in_today_opens_where_it_lives_in_a_new_pane() {
    let mut harness = Harness::new(60, 16);
    harness.type_text("gtjj");
    harness.ctrl('t');
    let rows = trimmed(&harness);
    assert!(rows[0].starts_with("  \u{f01a7} Work"), "{rows:#?}");
    assert!(
        rows.iter()
            .any(|row| row.starts_with("▌ ☐ Review two portfolios")),
        "{rows:#?}"
    );
    harness.ctrl('w');
    harness.type_text("c");
    let rows = trimmed(&harness);
    assert_eq!(rows[0], "  \u{f0599} Today");
    assert!(
        rows[6].starts_with("▌ ☐ Review two portfolios"),
        "{rows:#?}"
    );
}

// FR-VIEW-06, FR-TAG-04
#[test]
fn filter_today_by_a_tag() {
    let mut harness = Harness::new(60, 16);
    harness.type_text("gtgf");
    harness.type_text("writing");
    harness.key(KeyCode::Enter);
    let mut filtered = vec![
        "  \u{f0599} Today                                        \u{f04fc} @writing",
        "",
        "▌ ☐ Write the announcement @writing     \u{f0756} 1/5 \u{f11d7} \u{f0189} 1 \u{f166d} today",
        "▌   Work › Website › Launch",
    ];
    filtered.resize(15, "");
    filtered.push(" \u{f01bf} NAVIGATE · Demo data · not saved");
    assert_eq!(
        trimmed(&harness),
        filtered,
        "the title line names the view and the filter, and only the tagged row is left"
    );
}

// FR-VIEW-06, FR-VIEW-10
#[test]
fn open_today_from_the_sidebar() {
    let mut harness = Harness::new(80, 16);
    harness.ctrl('b');
    harness.type_text("j");
    harness.key(KeyCode::Enter);
    let rows = trimmed(&harness);
    assert!(
        rows[1].starts_with("                         │"),
        "{rows:#?}"
    );
    assert!(
        rows[2].starts_with("▌󰖙 Today"),
        "the sidebar keeps the keys: {rows:#?}"
    );
    assert!(rows[0].contains("│  󰖙 Today"), "Enter shows it: {rows:#?}");
    assert!(
        rows[5].starts_with(" 󰉹 All")
            && rows[6].starts_with(" 󱓳 Logbook")
            && rows[7].starts_with(" 󰩺 Trash"),
        "the Logbook between All and the Trash: {rows:#?}"
    );
}

// FR-VIEW-13, FR-OPS-06
#[test]
fn me_puts_a_task_in_the_evening_at_the_bottom_of_today_and_takes_it_out() {
    let mut harness = Harness::new(60, 18);
    harness.type_text("gtjme");
    assert_eq!(
        trimmed(&harness)[2..12],
        [
            "  ☐ Answer the recruiter's email                \u{f0a31} yesterday",
            "    Work",
            "  ☐ Review two portfolios                           \u{f166c} today",
            "    Work › Hiring",
            "  ☐ Water the plants                            \u{f166d} yesterday",
            "    Home",
            "",
            "  \u{f0594} Evening",
            "▌ ☐ Write the announcement @writing     \u{f0756} 1/5 \u{f11d7} \u{f0189} 1 \u{f166d} today",
            "▌   Work › Website › Launch",
        ],
        "under the Evening heading after the rest, still focused"
    );
    harness.type_text("kme");
    assert_eq!(
        trimmed(&harness)[6..12],
        [
            "",
            "  \u{f0594} Evening",
            "  ☐ Write the announcement @writing     \u{f0756} 1/5 \u{f11d7} \u{f0189} 1 \u{f166d} today",
            "    Work › Website › Launch",
            "▌ ☐ Water the plants                            \u{f166d} yesterday",
            "▌   Home",
        ],
        "in file order within the Evening"
    );
    harness.type_text("Vkme");
    assert_eq!(
        trimmed(&harness)[2..10],
        [
            "  ☐ Answer the recruiter's email                \u{f0a31} yesterday",
            "    Work",
            "▌ ☐ Write the announcement @writing     \u{f0756} 1/5 \u{f11d7} \u{f0189} 1 \u{f166d} today",
            "▌   Work › Website › Launch",
            "  ☐ Review two portfolios                           \u{f166c} today",
            "    Work › Hiring",
            "▌ ☐ Water the plants                            \u{f166d} yesterday",
            "▌   Home",
        ],
        "once all selected are in the Evening, the same key takes them out"
    );
    assert!(!trimmed(&harness).iter().any(|row| row.contains("Evening")));
    harness.type_text("z");
    assert!(
        trimmed(&harness).contains(&"  \u{f0594} Evening".to_owned()),
        "one step to undo"
    );
}

// FR-VIEW-13
#[test]
fn the_evening_is_not_kept_in_the_file_and_only_today_takes_me() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    std::fs::write(&path, "Inbox:\n\t- Call mom @start(2026-10-05)\n").expect("written");
    let mut harness = Harness::open(path.clone(), 60, 12);
    harness.type_text("me");
    harness.ctrl('p');
    harness.type_text("evening");
    assert_eq!(
        trimmed(&harness)[6],
        "│ No matches                                               │",
        "Toggle Evening is offered only in Today"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("gt");
    harness.ctrl('p');
    harness.type_text("evening");
    assert_eq!(
        trimmed(&harness)[6],
        "│ \u{f07b7} Toggle Evening                                      me │"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("me");
    harness.wait(1000);
    assert_eq!(
        std::fs::read_to_string(&path).expect("saved"),
        "Inbox:\n\t- Call mom @start(2026-10-05)\n",
        "the file never keeps the Evening"
    );
    let evening = |harness: &Harness| trimmed(harness).contains(&"  \u{f0594} Evening".to_owned());
    assert!(evening(&harness));
    drop(harness);
    let mut restarted = Harness::open(path, 60, 12);
    restarted.type_text("gt");
    assert!(!evening(&restarted), "a restart empties the Evening");
}

/// The titles of Today's tasks, top to bottom.
fn titles(harness: &Harness) -> Vec<String> {
    trimmed(harness)
        .iter()
        .filter_map(|row| row.split_once("☐ "))
        .map(|(_, title)| title.split("  ").next().unwrap_or_default().to_owned())
        .collect()
}

// FR-VIEW-06, FR-OPS-02, D-108
#[test]
fn alt_arrows_reorder_today_and_undo_puts_the_order_back() {
    let mut harness = Harness::new(60, 16);
    harness.type_text("gt");
    harness.press(KeyCode::Down, KeyModifiers::ALT);
    assert_eq!(
        titles(&harness),
        [
            "Write the announcement @writing",
            "Answer the recruiter's email",
            "Review two portfolios",
            "Water the plants",
        ],
        "the overdue task goes below the next one"
    );
    assert!(
        trimmed(&harness)[4].starts_with("▌ ☐ Answer"),
        "still focused"
    );
    harness.type_text("Vj");
    harness.press(KeyCode::Char('k'), KeyModifiers::ALT);
    assert_eq!(
        titles(&harness),
        [
            "Answer the recruiter's email",
            "Review two portfolios",
            "Write the announcement @writing",
            "Water the plants",
        ],
        "selected rows move together"
    );
    harness.press(KeyCode::Up, KeyModifiers::ALT);
    assert_eq!(
        titles(&harness)[0],
        "Answer the recruiter's email",
        "the top row stays at the top"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("z");
    assert_eq!(
        titles(&harness),
        [
            "Write the announcement @writing",
            "Answer the recruiter's email",
            "Review two portfolios",
            "Water the plants",
        ],
        "one step to undo"
    );
}

// FR-VIEW-06, D-108
#[test]
fn today_s_order_is_not_kept_in_the_file() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let path = dir.path().join("todo.taskpaper");
    let text = "Inbox:\n\t- Call mom @start(2026-10-05)\n\t- Pay rent @start(2026-10-05)\n";
    std::fs::write(&path, text).expect("written");
    let mut harness = Harness::open(path.clone(), 60, 12);
    harness.type_text("gt");
    harness.press(KeyCode::Down, KeyModifiers::ALT);
    assert_eq!(titles(&harness), ["Pay rent", "Call mom"]);
    harness.wait(1000);
    assert_eq!(
        std::fs::read_to_string(&path).expect("saved"),
        text,
        "the file never keeps Today's order"
    );
    drop(harness);
    let mut restarted = Harness::open(path, 60, 12);
    restarted.type_text("gt");
    assert_eq!(
        titles(&restarted),
        ["Call mom", "Pay rent"],
        "a restart forgets the order"
    );
}
