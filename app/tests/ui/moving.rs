//! Moving items: Move to.

use crossterm::event::KeyCode;
use ratatui::layout::{Position, Rect};

use crate::support::Harness;

/// The rows of the window titled `title`, between its borders, without trailing spaces.
fn window(harness: &Harness, title: &str) -> Vec<String> {
    harness
        .rows()
        .iter()
        .skip_while(|row| !row.starts_with(&format!("┌{title}")))
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

/// The rows of the pane below its breadcrumbs, without trailing spaces or blank rows.
fn pane(harness: &Harness) -> Vec<String> {
    harness
        .rows()
        .iter()
        .skip(2)
        .map(|row| row.trim_end().to_owned())
        .filter(|row| !row.is_empty() && !row.contains("NAVIGATE"))
        .collect()
}

// FR-OPS-04, FR-OPS-10, FR-UI-09, FR-EDIT-06
#[test]
fn move_a_task_to_another_project() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("gm");
    assert_eq!(
        window(&harness, "Move to"),
        [
            "›",
            " # Work",
            "   # Website",
            "     / Launch",
            "     / Polish",
            "     # Blog",
            "   / Hiring",
            " # Home",
            "   # Move to the new flat",
            "     / Paperwork",
            " # Learn Rust",
        ],
        "every container but the Inbox the task is in, sections included, as a tree"
    );
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(3, 6)),
        "the cursor stands in the query"
    );
    harness.type_text("rust");
    assert_eq!(window(&harness, "Move to"), ["› rust", " # Learn Rust"]);
    harness.key(KeyCode::Enter);
    assert_eq!(window(&harness, "Move to"), Vec::<String>::new());
    assert_eq!(
        pane(&harness),
        [
            "  󰚇 Inbox",
            "  Everything not sorted yet lands here.",
            "▌ - Buy coffee beans @errands",
            "  - Read the article Sam sent",
        ],
        "the pane stays and the focus moves on"
    );
    harness.ctrl('g');
    harness.type_text("rust");
    harness.key(KeyCode::Enter);
    assert_eq!(
        pane(&harness),
        [
            "  # Learn Rust",
            "  A project with nothing above it.",
            "▌ - Finish the ownership chapter @rust",
            "  - Write a small CLI @rust                                          󱙭 tomorrow",
            "  - Call the bank about the card                                          󱙬 Wed",
        ],
        "the task is the project's last"
    );
    harness.type_text("Ggm");
    assert_eq!(
        window(&harness, "Move to").get(1).map(String::as_str),
        Some(" 󰚇 Inbox"),
        "from elsewhere a task can go back to the Inbox"
    );
    harness.key(KeyCode::Esc);
    harness.type_text("H");
    harness.type_text("z");
    assert_eq!(
        pane(&harness).get(2).map(String::as_str),
        Some("▌ - Call the bank about the card                                          󱙬 Wed"),
        "undo puts it back, focused"
    );
}

// FR-OPS-04, FR-ORG-06, FR-UI-09
#[test]
fn a_project_moves_to_the_top_level_and_escape_closes_the_picker() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('g');
    harness.type_text("website");
    harness.key(KeyCode::Enter);
    harness.type_text("Ggm");
    assert_eq!(
        window(&harness, "Move to"),
        [
            "›",
            " 󰋜 Home",
            " # Work",
            " # Home",
            "   # Move to the new flat",
            " # Learn Rust",
        ],
        "a project goes to the top level or a project, never into itself"
    );
    harness.key(KeyCode::Esc);
    assert_eq!(window(&harness, "Move to"), Vec::<String>::new());
    harness.type_text("gm");
    harness.ctrl('g');
    assert_eq!(
        window(&harness, "Move to"),
        Vec::<String>::new(),
        "the Navigator's key works there too, and the Navigator takes its place"
    );
    assert_ne!(window(&harness, "Navigator"), Vec::<String>::new());
    harness.key(KeyCode::Esc);
    harness.type_text("gm");
    harness.key(KeyCode::Enter);
    harness.ctrl('g');
    assert_eq!(
        window(&harness, "Navigator")
            .last()
            .map(|row| row.trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ')),
        Some(" # Blog"),
        "Blog is the last at the top level"
    );
}
