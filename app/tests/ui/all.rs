//! The All view: every task in every list, grouped by where it lives.

use crossterm::event::KeyCode;

use crate::support::{Harness, without_chips};

/// The rows of the screen, without trailing spaces.
fn trimmed(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .map(|row| row.trim_end().to_owned())
        .collect()
}

// FR-VIEW-08, FR-VIEW-01, FR-KEY-06, FR-NAV-03
#[test]
fn open_all() {
    let mut harness = Harness::new(60, 38);
    harness.type_text("ga");
    let all = [
        "  \u{f0279} All",
        "────────────────────────────────────────────────────────────",
        "  \u{f0279} All",
        "",
        "▌ - Call the bank about the card                      \u{f166c} Wed",
        "  - Buy coffee beans @errands",
        "  - Plan the weekend trip                             \u{f166d} Thu",
        "  - Read the article Sam sent",
        "",
        "  \u{f01a7} Work",
        "  - Answer the recruiter's email                \u{f0a31} yesterday",
        "  - Review two portfolios                           \u{f166c} today",
        "",
        "  \u{f0256} Website",
        "  - Collect logos from partners",
        "  - Write the announcement @writing     \u{f0756} 1/5 \u{f11d7} \u{f0189} 1 \u{f166d} today",
        "    Keep it under 300 words.",
        "  - Book the launch call @meeting                     \u{f166c} Sat",
        "  - Fix the footer links",
        "",
        "  \u{f0256} Blog",
        "  - Pick the first three topics",
        "",
        "  \u{f01a7} Home",
        "  - Water the plants                            \u{f166d} yesterday",
        "",
        "  \u{f0256} Move to the new flat",
        "  - Pack the books                                    \u{f0756} 0/2",
        "  - Hand back the old keys                       \u{f166c} next Sat",
        "  - Change the address at the bank",
        "  - Register at the town hall                    \u{f166d} next Mon",
        "",
        "  \u{f0256} Learn Rust",
        "  - Finish the ownership chapter @rust",
        "  - Write a small CLI @rust                      \u{f166d} tomorrow",
        "",
        "",
        " \u{f01bf} NAVIGATE · Demo data · not saved",
    ];
    assert_eq!(
        trimmed(&harness),
        all,
        "the Inbox's tasks first, its scheduled one too, then each area's and project's, \
         its sections' tasks after its own"
    );
}

/// The titles of the rows of the screen's tasks, open or completed, without date chips or
/// attachment icons, the focused one starred.
fn tasks(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .filter_map(|row| {
            let (focus, task) = row
                .split_once("- ")
                .or_else(|| row.split_once("\u{2714} "))?;
            let title = without_chips(task.trim_end())
                .split("  ")
                .next()
                .unwrap_or_default();
            Some(if focus.starts_with('▌') {
                format!("*{title}")
            } else {
                title.to_owned()
            })
        })
        .collect()
}

// FR-VIEW-08, FR-VIEW-11, FR-TASK-02
#[test]
fn a_task_completed_in_all_leaves_it_until_completed_ones_are_shown() {
    let mut harness = Harness::new(60, 50);
    harness.type_text("ga");
    harness.key(KeyCode::Char('x'));
    let shown = tasks(&harness);
    assert_eq!(
        shown[..3],
        [
            "*Buy coffee beans @errands",
            "Plan the weekend trip",
            "Read the article Sam sent"
        ]
    );
    assert!(
        !shown
            .iter()
            .any(|task| task.contains("Call the bank about"))
    );
    harness.ctrl('o');
    let shown = tasks(&harness);
    assert_eq!(
        shown[3], "Call the bank about the card",
        "last among the Inbox's"
    );
    let website = shown
        .iter()
        .position(|task| task == "Collect logos from partners")
        .unwrap_or_default();
    assert_eq!(
        shown[website..website + 6],
        [
            "Collect logos from partners",
            "Write the announcement @writing",
            "Book the launch call @meeting",
            "Fix the footer links",
            "Compress the hero image",
            "Pick the first three topics",
        ],
        "completed after the open ones of their section"
    );
}

// FR-VIEW-08, FR-TASK-02
#[test]
fn a_task_of_a_section_completes_alone_in_all() {
    let mut harness = Harness::new(60, 50);
    harness.type_text("ga");
    harness.type_text("jjjjjjj");
    assert_eq!(tasks(&harness)[7], "*Write the announcement @writing");
    harness.type_text("#x");
    let shown = tasks(&harness);
    assert!(
        !shown.contains(&"Write the announcement @writing".to_owned()),
        "{shown:#?}"
    );
    assert!(
        shown.contains(&"*Book the launch call @meeting".to_owned()),
        "# finds no section heading in All, so x completes the task, not its section: {shown:#?}"
    );
}
