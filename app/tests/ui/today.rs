//! The Today view: what is due or has started, grouped by area and project, overdue first.

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

// FR-VIEW-06, FR-KEY-06, FR-DATE-04, FR-NAV-03
#[test]
fn open_today() {
    let mut harness = Harness::new(60, 16);
    harness.type_text("gt");
    let today = [
        "  \u{f0599} Today",
        "────────────────────────────────────────────────────────────",
        "  \u{f0599} Today",
        "",
        "  \u{f01a7} Work",
        "▌ - Answer the recruiter's email                \u{f0a31} yesterday",
        "  - Review two portfolios                           \u{f166c} today",
        "",
        "  \u{f0256} Website",
        "  - Write the announcement @writing           \u{f11d7} \u{f0189} 1 \u{f166d} today",
        "    Keep it under 300 words.",
        "",
        "  \u{f01a7} Home",
        "  - Water the plants                            \u{f166d} yesterday",
        "",
        " \u{f01bf} NAVIGATE · Demo data · not saved",
    ];
    assert_eq!(
        trimmed(&harness),
        today,
        "the overdue task first in Work, the section's task under its project, headings no rows"
    );
}

// FR-VIEW-06, FR-NAV-02, FR-NAV-04
#[test]
fn going_into_a_task_in_today_shows_it_where_it_lives_and_back_returns() {
    let mut harness = Harness::new(60, 16);
    harness.type_text("gtj.");
    let rows = trimmed(&harness);
    assert_eq!(rows[0], "  \u{f02dc} Work");
    assert!(
        rows.iter()
            .any(|row| row.starts_with("▌ - Review two portfolios")),
        "{rows:#?}"
    );
    harness.type_text("H");
    let rows = trimmed(&harness);
    assert_eq!(rows[0], "  \u{f0599} Today");
    assert!(
        rows[6].starts_with("▌ - Review two portfolios"),
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
        "────────────────────────────────────────────────────────────",
        "  \u{f0599} Today",
        "",
        "  \u{f0256} Website",
        "▌ - Write the announcement @writing           \u{f11d7} \u{f0189} 1 \u{f166d} today",
        "▌   Keep it under 300 words.",
    ];
    filtered.resize(15, "");
    filtered.push(" \u{f01bf} NAVIGATE · Demo data · not saved");
    assert_eq!(
        trimmed(&harness),
        filtered,
        "the crumbs name the view and the filter, and only the group with a tagged row is left"
    );
}
