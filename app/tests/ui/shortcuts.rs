//! The shortcuts panel: showing and hiding it with `?`, what it lists in each context, scrolling
//! it and where it goes on screens of each size.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::support::Harness;

/// The panel's lines on a `width`-column screen, from its last `columns` columns, trimmed, with
/// the status line left out.
fn panel(harness: &Harness, columns: usize) -> Vec<String> {
    let rows = harness.rows();
    rows.iter()
        .take(rows.len() - 1)
        .map(|row| {
            let skip = row.chars().count() - columns;
            row.chars().skip(skip).collect::<String>().trim().to_owned()
        })
        .collect()
}

// FR-KEY-03, FR-KEY-02
#[test]
fn question_mark_shows_the_keys_of_every_command_grouped_at_the_right() {
    let mut harness = Harness::new(160, 30);
    let before = harness.screen();
    harness.type_text("?");
    let lines = panel(&harness, 40);
    assert_eq!(
        lines.get(..10).unwrap_or_default(),
        [
            "│Shortcuts",
            "│ Moving",
            "│  Next Item                       j, ↓",
            "│  Previous Item                   k, ↑",
            "│  First Item                  gg, Home",
            "│  Last Item                     G, End",
            "│  Section Heading                    #",
            "│  Next Section              gj, Ctrl+↓",
            "│  Previous Section          gk, Ctrl+↑",
            "│ Selection",
        ]
    );
    assert!(
        lines.contains(&"│  Go Home               Ctrl+', gh, gi".to_owned()),
        "{lines:#?}"
    );
    assert_eq!(
        lines.get(lines.len() - 5..).unwrap_or_default(),
        [
            "│ ↓ more                              )",
            "│",
            "│ No Alt key seen yet: on macOS",
            "│ set Option as Meta or Esc+",
            "│ Kitty keyboard protocol: off",
        ]
    );
    let screen = harness.screen();
    assert!(
        screen.contains("- Call the bank about the card"),
        "{screen}"
    );
    harness.type_text("?");
    assert_eq!(harness.screen(), before, "hidden again");
}

// FR-KEY-03
#[test]
fn parentheses_scroll_the_panel_a_page_as_far_as_it_goes() {
    let mut harness = Harness::new(160, 30);
    harness.type_text("?)");
    let lines = panel(&harness, 40);
    assert_eq!(
        lines.get(1..3).unwrap_or_default(),
        [
            "│ ↑ more                              (",
            "│  Go to All                         ga"
        ]
    );
    harness.type_text("))))))))");
    let lines = panel(&harness, 40);
    assert!(
        lines.contains(&"│  Toggle Shortcuts                   ?".to_owned()),
        "the end: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("↓ more")),
        "{lines:#?}"
    );
    harness.type_text("((((((((((");
    assert_eq!(
        panel(&harness, 40).get(1),
        Some(&"│ Moving".to_owned()),
        "back at the top"
    );
    harness.type_text("))??");
    assert_eq!(
        panel(&harness, 40).get(1),
        Some(&"│ Moving".to_owned()),
        "a panel shown again opens at the top"
    );
    harness.type_text("?");
    let rows = harness.rows();
    assert!(rows.iter().all(|row| !row.contains("Shortcuts")));
    harness.type_text(")");
    assert!(
        harness.rows().iter().all(|row| !row.contains("Shortcuts")),
        "scrolling shows nothing"
    );
}

// FR-KEY-03, FR-DATE-02
#[test]
fn the_date_prompt_lists_its_own_keys_and_the_text_keys_without_scroll_keys() {
    let mut harness = Harness::new(160, 40);
    harness.type_text("?md");
    let lines = panel(&harness, 40);
    assert_eq!(
        lines.get(1..6).unwrap_or_default(),
        [
            "│ Date Prompt",
            "│  Set Date                       Enter",
            "│  Cancel                           Esc",
            "│ Text",
            "│  Left                               ←",
        ]
    );
    assert!(lines.contains(&"│ Going".to_owned()), "{lines:#?}");
    assert!(
        lines.contains(&"│  Open Navigator    Ctrl+G, Ctrl+K, gn".to_owned()),
        "{lines:#?}"
    );
    assert!(!lines.contains(&"│ Moving".to_owned()), "{lines:#?}");
    harness.key(KeyCode::Esc);
    assert_eq!(panel(&harness, 40).get(1), Some(&"│ Moving".to_owned()));
}

// FR-KEY-03, FR-EDIT-01, FR-EDIT-03
#[test]
fn edit_mode_lists_the_keys_of_the_title_or_the_description_edited() {
    let mut harness = Harness::new(160, 40);
    harness.type_text("?");
    harness.key(KeyCode::Enter);
    let lines = panel(&harness, 40);
    assert_eq!(
        lines.get(1..4).unwrap_or_default(),
        [
            "│ Editing",
            "│  Done, Then Next Task           Enter",
            "│  Done                             Esc",
        ]
    );
    assert!(
        lines.contains(&"│  Text Start                      Home".to_owned()),
        "{lines:#?}"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("mn");
    let lines = panel(&harness, 40);
    assert_eq!(
        lines.get(1..5).unwrap_or_default(),
        [
            "│ Editing Description",
            "│  Line Up                            ↑",
            "│  Select Line Up               Shift+↑",
            "│  Line Down                          ↓",
        ]
    );
    assert!(
        lines.contains(&"│  New Line                       Enter".to_owned()),
        "{lines:#?}"
    );
    let keys = |name: &str| {
        lines
            .iter()
            .filter(|line| line.starts_with(&format!("│  {name}  ")))
            .map(|line| line.rsplit("  ").next().unwrap_or_default().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(keys("Line Start"), ["Home"]);
    assert_eq!(
        keys("Text Start"),
        ["Ctrl+Home"],
        "not Home, the line's start here"
    );
}

// FR-KEY-03, FR-VIEW-07
#[test]
fn a_picker_lists_its_own_keys() {
    let mut harness = Harness::new(160, 40);
    harness.type_text("?");
    harness.ctrl('g');
    let lines = panel(&harness, 40);
    let screen = harness.screen();
    assert!(screen.contains("│ Picker"), "{screen}");
    assert!(screen.contains("Previous Match"), "{screen}");
    assert!(
        lines.iter().any(|line| line.contains("Next Match")),
        "{lines:#?}"
    );
}

// FR-KEY-03
#[test]
fn an_alt_key_ends_the_note_on_how_to_send_one() {
    let mut harness = Harness::new(160, 30);
    harness.type_text("?");
    harness.press(KeyCode::Char('j'), KeyModifiers::ALT);
    let screen = harness.screen();
    assert!(!screen.contains("No Alt key seen yet"), "{screen}");
    assert!(screen.contains("Kitty keyboard protocol: off"), "{screen}");
    harness.tui.use_kitty(true);
    harness.draw();
    assert!(harness.screen().contains("Kitty keyboard protocol: on"));
}

// FR-KEY-03
#[test]
fn a_narrow_screen_draws_the_panel_over_the_pane_and_a_tiny_one_has_no_room() {
    let mut harness = Harness::new(60, 24);
    harness.type_text("?");
    let rows = harness.rows();
    assert!(
        rows.first()
            .is_some_and(|row| row.contains("Inbox") && row.contains("│Shortcuts")),
        "{rows:#?}"
    );
    assert!(
        rows.iter().any(|row| row.starts_with("▌ - Call the bank")),
        "the pane keeps its width under the panel: {rows:#?}"
    );
    let mut harness = Harness::new(39, 24);
    harness.type_text("?");
    assert!(!harness.screen().contains("Shortcuts"));
}

// FR-KEY-03, FR-VIEW-10
#[test]
fn the_panel_shows_beside_the_sidebar_and_its_key_works_there() {
    let mut harness = Harness::new(160, 30);
    harness.ctrl('b');
    harness.type_text("?");
    let rows = harness.rows();
    assert!(
        rows.first().is_some_and(|row| row.contains("│Shortcuts")),
        "{rows:#?}"
    );
    assert!(harness.screen().contains("Inbox"));
}

// FR-KEY-03, FR-ATT-01, FR-ATT-03, FR-ATT-04
#[test]
fn the_attachments_panel_lists_its_keys_and_opens_beside_the_shortcuts() {
    let mut harness = Harness::new(160, 40);
    harness.type_text("?n");
    let lines = panel(&harness, 40);
    assert_eq!(
        lines.get(1..4).unwrap_or_default(),
        [
            "│ Attachments Panel",
            "│  Close                            Esc",
            "│  Edit                        e, Enter",
        ]
    );
    assert!(
        lines.contains(&"│  Add Comment                        a".to_owned()),
        "{lines:#?}"
    );
    assert!(
        lines.contains(&"│  Page Down               Ctrl+D, PgDn".to_owned()),
        "{lines:#?}"
    );
    assert!(!lines.contains(&"│ Text".to_owned()), "{lines:#?}");
    let rows = harness.rows();
    assert!(
        rows.iter()
            .any(|row| row.contains("Note") && row.contains("│ Attachments Panel")),
        "the attachments panel stands left of the shortcuts: {rows:#?}"
    );
    harness.key(KeyCode::Enter);
    let lines = panel(&harness, 40);
    assert_eq!(
        lines.get(1),
        Some(&"│ Editing Note".to_owned()),
        "{lines:#?}"
    );
}
