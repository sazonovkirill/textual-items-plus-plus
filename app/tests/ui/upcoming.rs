//! The Upcoming view: what is dated after today, by day for a week and then by month.

use crate::support::Harness;

/// The rows of the screen, without trailing spaces.
fn trimmed(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

// FR-VIEW-02, FR-KEY-06, OQ-8, OQ-20
#[test]
fn open_upcoming() {
    let mut harness = Harness::new(60, 25);
    harness.type_text("gU");
    let upcoming = [
        "  \u{f1135} Upcoming",
        "────────────────────────────────────────────────────────────",
        "  \u{f1135} Upcoming",
        "",
        "  Tomorrow · Tue 6 Oct",
        "▌ - Write a small CLI @rust                      \u{f166d} tomorrow",
        "",
        "  Wed 7 Oct",
        "  - Call the bank about the card                      \u{f166c} Wed",
        "",
        "  Thu 8 Oct",
        "  - Plan the weekend trip                             \u{f166d} Thu",
        "",
        "  Sat 10 Oct",
        "  - Book the launch call @meeting                     \u{f166c} Sat",
        "",
        "  Next Mon · Mon 12 Oct",
        "  - Register at the town hall                    \u{f166d} next Mon",
        "",
        "  Rest of October",
        "  - Hand back the old keys                       \u{f166c} next Sat",
        "  # Website                              \u{f11d7} \u{f0189} 2 \u{f166c} in 20 days",
        "    The new marketing site, built with the design team.",
        "",
        " \u{f01bf} NAVIGATE · Demo data · not saved",
    ];
    assert_eq!(
        trimmed(&harness),
        upcoming,
        "each item once, the week by day, then the rest of the month in date order, a project too"
    );
}

// FR-VIEW-02, FR-TASK-02, FR-VIEW-11
#[test]
fn completing_a_task_in_upcoming_takes_it_out_until_completed_items_are_shown() {
    let mut harness = Harness::new(60, 25);
    harness.type_text("gU");
    harness.type_text("x");
    let rows = trimmed(&harness);
    assert_eq!(rows[4], "  Wed 7 Oct", "{rows:#?}");
    assert!(
        rows[5].starts_with("▌ - Call the bank about the card"),
        "the focus moves on: {rows:#?}"
    );
    harness.ctrl('o');
    let rows = trimmed(&harness);
    assert_eq!(rows[4], "  Tomorrow · Tue 6 Oct", "{rows:#?}");
    assert!(
        rows[5].contains("Write a small CLI"),
        "shown again, completed: {rows:#?}"
    );
}
