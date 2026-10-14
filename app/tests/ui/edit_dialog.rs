//! The edit dialog: every property of a task, project, section or area but its attachments,
//! edited in a modal window at the bottom of the screen and applied as one step to undo.

use crossterm::event::{Event, KeyCode};

use ratatui::layout::{Position, Rect};

use crate::support::Harness;

/// The dialog's rows, from its top border to its bottom one, trimmed at the right.
fn dialog(harness: &Harness) -> Vec<String> {
    let rows = harness.rows();
    let top = rows.iter().position(|row| row.starts_with("┌ Edit "));
    rows.iter()
        .skip(top.unwrap_or(rows.len()))
        .take_while(|row| row.starts_with(['┌', '│', '└']))
        .map(|row| row.trim_end().to_owned())
        .collect()
}

// FR-EDIT-08, FR-DATE-02, FR-TAG-02, FR-EDIT-06
#[test]
fn edit_every_field_of_a_task_and_save_them_as_one_step_to_undo() {
    let mut harness = Harness::new(80, 24);
    let before = harness.screen();
    harness.type_text("e!");
    harness.key(KeyCode::Tab);
    harness.type_text("Ask about the fee");
    harness.key(KeyCode::Enter);
    harness.type_text("and the limit");
    harness.key(KeyCode::Tab);
    harness.type_text("err");
    harness.key(KeyCode::Down);
    harness.key(KeyCode::Enter);
    harness.key(KeyCode::Tab);
    harness.type_text("tomorrow");
    harness.key(KeyCode::Tab);
    harness.type_text("fri");
    assert_eq!(
        dialog(&harness),
        [
            "┌ Edit Task ───────────────────────────────────────────────────────────────────┐",
            "│  Title        Call the bank about the card!                                  │",
            "│  Description  Ask about the fee                                              │",
            "│               and the limit                                                  │",
            "│  Tags         @errands                                                       │",
            "│  Start        󱙭 Tue 6 Oct 2026 · tomorrow                                    │",
            "│▌ Due          fri  → Fri 9 Oct 2026 · Fri                                    │",
            "│                                                                              │",
            "│ Type a date: tomorrow, fri, +3d · Enter save · Esc cancel                    │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ],
        "the tag picked from the matches, the dates typed in words"
    );
    harness.key(KeyCode::Enter);
    assert_eq!(dialog(&harness), Vec::<String>::new());
    harness.type_text("gt");
    let today = harness.screen();
    assert!(
        !today.contains("Call the bank"),
        "it starts tomorrow, out of Today and the Inbox"
    );
    harness.type_text("gUe");
    assert_eq!(
        dialog(&harness).get(1..5).unwrap_or_default(),
        [
            "│▌ Title        Call the bank about the card!                                  │",
            "│  Description  Ask about the fee                                              │",
            "│               and the limit                                                  │",
            "│  Tags         @errands                                                       │",
        ],
        "saved, and focused in Upcoming"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("gi");
    harness.type_text("z");
    assert_eq!(harness.screen(), before, "one step takes it all back");
}

// FR-EDIT-08, FR-EDIT-04
#[test]
fn m_i_opens_it_too_escape_closes_it_unchanged_and_pasting_types_into_the_field() {
    let mut harness = Harness::new(80, 24);
    let before = harness.screen();
    harness.type_text("mi");
    harness.send(&Event::Paste(" today\nplease".to_owned()));
    assert_eq!(
        dialog(&harness).get(1),
        Some(
            &"│▌ Title        Call the bank about the card today please                      │"
                .to_owned()
        ),
        "on one line"
    );
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(57, 15)),
        "after the title"
    );
    harness.key(KeyCode::Esc);
    assert_eq!(harness.screen(), before);
    harness.type_text("e");
    harness.key(KeyCode::End);
    for _ in 0.."Call the bank about the card".len() {
        harness.key(KeyCode::Backspace);
    }
    harness.key(KeyCode::Enter);
    assert_eq!(
        dialog(&harness).get(7),
        Some(
            &"│ A title can't be empty                                                       │"
                .to_owned()
        ),
        "it stays open on the title"
    );
    harness.type_text("@");
    assert!(
        (harness.rows().last())
            .is_some_and(|row| row.contains("a title has no word starting with @")),
        "{}",
        harness.screen()
    );
}

// FR-EDIT-08, FR-VIEW-12, FR-VIEW-10
#[test]
fn the_sidebar_opens_a_projects_dialog_which_pins_it_and_an_areas_without_tags_or_dates() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('b');
    harness.type_text("jjjjjjje");
    assert_eq!(
        dialog(&harness),
        [
            "┌ Edit Project ────────────────────────────────────────────────────────────────┐",
            "│▌ Title        Learn Rust                                                     │",
            "│  Description  A project outside any area.                                    │",
            "│  Tags         No tags                                                        │",
            "│  Start        None                                                           │",
            "│  Due          None                                                           │",
            "│  Pinned       ☐ Not pinned                                                   │",
            "│                                                                              │",
            "│ Tab next field · Enter save · Esc cancel                                     │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ]
    );
    for _ in 0..5 {
        harness.key(KeyCode::Tab);
    }
    harness.type_text(" ");
    harness.key(KeyCode::Enter);
    assert!(
        harness
            .rows()
            .get(9)
            .is_some_and(|row| row.starts_with(" # Learn Rust")),
        "pinned after the views"
    );
    harness.type_text("je");
    assert_eq!(
        dialog(&harness).get(..4).unwrap_or_default(),
        [
            "┌ Edit Area ───────────────────────────────────────────────────────────────────┐",
            "│▌ Title        Work                                                           │",
            "│  Description  Everything for the day job.                                    │",
            "│                                                                              │",
        ]
    );
}

// FR-EDIT-08
#[test]
fn a_section_opens_a_dialog_of_its_title_and_description() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('b');
    harness.type_text("jjjjjjjjjjjj");
    harness.key(KeyCode::Enter);
    harness.ctrl('b');
    harness.type_text("jje");
    assert_eq!(
        dialog(&harness),
        [
            "┌ Edit Section ────────────────────────────────────────────────────────────────┐",
            "│▌ Title        Paperwork                                                      │",
            "│  Description  No description                                                 │",
            "│                                                                              │",
            "│ Tab next field · Enter save · Esc cancel                                     │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ]
    );
    harness.key(KeyCode::Tab);
    harness.type_text("Forms to fill in");
    harness.key(KeyCode::Enter);
    harness.type_text("before the move");
    harness.key(KeyCode::Up);
    harness.key(KeyCode::Home);
    harness.key(KeyCode::Up);
    harness.key(KeyCode::Enter);
    assert_eq!(
        dialog(&harness),
        Vec::<String>::new(),
        "saved from the title"
    );
    assert_eq!(
        harness.rows().get(5..8).unwrap_or_default(),
        [
            "▌ / Paperwork                                                                   ",
            "▌   Forms to fill in                                                            ",
            "  ☐ Change the address at the bank                                              ",
        ],
        "its row shows the description's first line"
    );
    harness.type_text("e");
    assert_eq!(
        dialog(&harness).get(2..4).unwrap_or_default(),
        [
            "│  Description  Forms to fill in                                               │",
            "│               before the move                                                │",
        ]
    );
}

// FR-EDIT-08, FR-TAG-02
#[test]
fn pasting_on_the_tags_types_into_the_tag_picker_which_takes_the_cursor() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("e");
    harness.key(KeyCode::Tab);
    harness.key(KeyCode::Tab);
    let tags = harness.screen();
    harness.send(&Event::Paste("errands".to_owned()));
    assert_eq!(harness.screen(), tags, "the tags take no text");
    harness.type_text(" ");
    harness.send(&Event::Paste("err\nands".to_owned()));
    let rows = harness.rows();
    let query = rows.iter().position(|row| row.starts_with("│› err ands"));
    assert!(query.is_some(), "{}", harness.screen());
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        query
            .and_then(|y| u16::try_from(y).ok())
            .map(|y| Position::new(11, y)),
        "after the query"
    );
}

// FR-EDIT-08, FR-UI-07
#[test]
fn the_navigator_opens_over_the_dialog_closing_it() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("e");
    harness.ctrl('k');
    assert_eq!(dialog(&harness), Vec::<String>::new());
    assert!(harness.screen().contains("│›"), "{}", harness.screen());
}
