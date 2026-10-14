//! Panes side by side: opening a place or the focused item in a new pane, moving between panes,
//! closing, narrowing and widening them, and scrolling sideways when they do not fit.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::support::Harness;

/// The column of each rule between panes, read from the rule under the breadcrumbs.
fn rules(harness: &Harness) -> Vec<usize> {
    let rows = harness.rows();
    rows.get(1)
        .map(|rule| {
            rule.chars()
                .enumerate()
                .filter_map(|(column, c)| (c == '│').then_some(column))
                .collect()
        })
        .unwrap_or_default()
}

/// The text left and right of the first rule on row `row`, trimmed.
fn sides(harness: &Harness, row: usize) -> (String, String) {
    let rows = harness.rows();
    let line = rows.get(row).map(String::as_str).unwrap_or_default();
    let (left, right) = line.split_once('│').unwrap_or((line, ""));
    (left.trim().to_owned(), right.trim().to_owned())
}

/// Whether the focus marker shows left of the first rule, right of it, on any row.
fn marked(harness: &Harness) -> (bool, bool) {
    let rows = harness.rows();
    let sides: Vec<_> = rows
        .iter()
        .map(|row| row.split_once('│').unwrap_or((row, "")))
        .collect();
    (
        sides.iter().any(|(left, _)| left.contains('▌')),
        sides.iter().any(|(_, right)| right.contains('▌')),
    )
}

/// The demo with Website shown and Today opened beside it from the Navigator.
fn today_beside_website() -> Harness {
    let mut harness = Harness::new(100, 16);
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
    harness.ctrl('g');
    harness.type_text("today");
    harness.press(KeyCode::Enter, KeyModifiers::ALT);
    harness
}

// FR-PANE-01, FR-PANE-02, FR-NAV-03
#[test]
fn open_today_beside_a_project() {
    let mut harness = today_beside_website();
    let (left, right) = sides(&harness, 0);
    assert!(
        left.contains("Work › Website") && left.ends_with(" 1"),
        "{left}"
    );
    assert!(right.contains("Today") && right.ends_with(" 2"), "{right}");
    assert_eq!(rules(&harness), [49]);
    assert_eq!(marked(&harness), (false, true), "the new pane has the keys");
    harness.ctrl('w');
    harness.type_text("h");
    assert_eq!(marked(&harness), (true, false));
    harness.type_text("gr");
    assert_eq!(marked(&harness), (false, true));
    harness.press(KeyCode::Char('1'), KeyModifiers::ALT);
    assert_eq!(marked(&harness), (true, false));
    harness.ctrl('w');
    harness.type_text("c");
    let header = harness.rows().first().cloned().unwrap_or_default();
    assert!(
        header.contains("Today") && !header.contains('│') && !header.trim_end().ends_with(" 1"),
        "the other pane takes the screen, unnumbered: {header}"
    );
}

// FR-PANE-02
#[test]
fn open_the_focused_item_in_a_new_pane() {
    let mut harness = Harness::new(100, 16);
    harness.ctrl('g');
    harness.type_text("work");
    harness.key(KeyCode::Enter);
    harness.type_text("G");
    harness.ctrl('\\');
    let (left, right) = sides(&harness, 0);
    assert!(left.contains("Work") && left.ends_with(" 1"), "{left}");
    assert!(
        right.contains("Work › ") && right.ends_with(" 2"),
        "{right}"
    );
    harness.ctrl('w');
    harness.type_text("c");
    assert_eq!(rules(&harness), Vec::<usize>::new());
    for picker in ["gm", "gf"] {
        harness.type_text(picker);
        let open = harness.rows();
        harness.press(KeyCode::Enter, KeyModifiers::ALT);
        assert_eq!(
            harness.rows(),
            open,
            "Move to and the tag picker open nothing in a new pane"
        );
        harness.key(KeyCode::Esc);
    }
}

// FR-PANE-02
#[test]
fn narrowing_a_pane_drawn_wider_than_its_width_brings_back_the_pane_beyond_it() {
    let mut harness = today_beside_website();
    harness.ctrl('\\');
    harness.ctrl('w');
    harness.type_text("h");
    assert_eq!(rules(&harness).len(), 2, "three panes fit");
    for _ in 0..2 {
        harness.ctrl('w');
        harness.type_text(">");
    }
    assert_eq!(
        rules(&harness).len(),
        1,
        "the second pane, drawn to the edge, pushes out the third"
    );
    harness.ctrl('w');
    harness.type_text("<");
    assert_eq!(
        rules(&harness).len(),
        2,
        "four columns narrower, all three fit"
    );
}

// FR-PANE-02
#[test]
fn narrow_and_widen_a_pane_and_scroll_sideways_when_panes_do_not_fit() {
    let mut harness = today_beside_website();
    harness.ctrl('w');
    harness.type_text("<");
    assert_eq!(
        rules(&harness),
        [54],
        "four columns narrower than its share"
    );
    for _ in 0..2 {
        harness.press(KeyCode::Char('L'), KeyModifiers::ALT | KeyModifiers::SHIFT);
    }
    assert_eq!(rules(&harness), [46], "eight columns wider again");
    harness.ctrl('\\');
    harness.ctrl('\\');
    let (_, right) = sides(&harness, 0);
    assert!(right.ends_with(" 4"), "{right}");
    assert_eq!(
        rules(&harness).len(),
        1,
        "two of four panes fit, the one with the keys on screen"
    );
    harness.type_text("g1");
    let (left, _) = sides(&harness, 0);
    assert!(left.ends_with(" 1"), "scrolled back to the first: {left}");
}
