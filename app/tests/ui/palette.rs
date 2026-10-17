//! The command palette.

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

/// The text of each command the palette lists, its icon and its keys apart.
fn commands(harness: &Harness) -> Vec<(String, String)> {
    window(harness, "Command Palette")
        .iter()
        .skip(1)
        .filter_map(|row| {
            let (_, row) = row.trim_start().split_once(' ')?;
            let (title, keys) = row.split_once("  ").unwrap_or((row, ""));
            Some((title.to_owned(), keys.trim().to_owned()))
        })
        .collect()
}

// FR-NAV-06, FR-UI-09
#[test]
fn ctrl_p_lists_every_command_with_its_keys_and_typing_filters_them() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('p');
    let listed = commands(&harness);
    assert_eq!(
        listed.first(),
        Some(&("Quit".to_owned(), "Ctrl+Q".to_owned())),
        "every command in the registry's order, its keys at the right"
    );
    assert!(listed.contains(&("First Item".to_owned(), "gg, Home".to_owned())));
    assert_eq!(
        harness.tui.cursor(Rect::new(0, 0, 80, 24)),
        Some(Position::new(3, 3)),
        "the cursor stands in the query"
    );
    harness.type_text("move");
    assert_eq!(
        commands(&harness),
        [
            ("Move To".to_owned(), "M".to_owned()),
            ("Move Up".to_owned(), "mk, Alt+k, Alt+↑".to_owned()),
            ("Move Down".to_owned(), "mj, Alt+j, Alt+↓".to_owned()),
            ("New Item Above".to_owned(), "O".to_owned()),
        ]
    );
    harness.key(KeyCode::Enter);
    assert_eq!(
        window(&harness, "Command Palette"),
        Vec::<String>::new(),
        "choosing closes it"
    );
    assert_eq!(
        window(&harness, "Move to").first().map(String::as_str),
        Some("›"),
        "and gives the command chosen"
    );
}

// FR-NAV-06, FR-VIEW-11
#[test]
fn gc_opens_the_palette_and_ctrl_o_alone_shows_completed_items() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("x");
    harness.type_text("gc");
    assert_eq!(
        window(&harness, "Command Palette")
            .first()
            .map(String::as_str),
        Some("›")
    );
    harness.key(KeyCode::Esc);
    assert_eq!(window(&harness, "Command Palette"), Vec::<String>::new());
    assert!(
        !harness.screen().contains('✔'),
        "gc no longer shows completed items"
    );
    harness.type_text("gc");
    harness.type_text("show completed");
    harness.key(KeyCode::Enter);
    assert!(harness.screen().contains('✔'), "the palette does");
    harness.ctrl('o');
    assert!(!harness.screen().contains('✔'), "Ctrl+O hides them again");
}
