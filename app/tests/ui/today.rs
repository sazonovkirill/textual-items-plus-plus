//! The Today view: what is due or has started, in one list, overdue first, each row saying where
//! it is.

use crossterm::event::KeyCode;

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
    harness.ctrl('\\');
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
        rows[4].starts_with(" 󰉹 All")
            && rows[5].starts_with(" 󱓳 Logbook")
            && rows[6].starts_with(" 󰩺 Trash"),
        "the Logbook between All and the Trash: {rows:#?}"
    );
}
