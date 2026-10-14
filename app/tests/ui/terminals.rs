//! Small terminals: every screen size down to nothing, with every window open.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::support::Harness;

/// The screen sizes the small-terminal tests draw at: 80×24 and smaller, down to nothing.
const SIZES: [(u16, u16); 12] = [
    (80, 24),
    (79, 23),
    (60, 12),
    (40, 10),
    (39, 10),
    (40, 9),
    (30, 6),
    (20, 4),
    (10, 3),
    (5, 2),
    (1, 1),
    (0, 0),
];

/// Draws the app and places the cursor as `tipp` does, failing when the cursor falls outside
/// the screen.
fn draw(app: &mut Harness, after: &str) {
    app.draw();
    let area = app.terminal.backend().buffer().area;
    if let Some(cursor) = app.tui.cursor(area) {
        assert!(
            area.contains(cursor),
            "{cursor:?} outside {area:?} after {after:?}"
        );
    }
}

// N-PORT-02: every screen down to 0×0 draws without failing, the status line at the bottom and
// the focused row above it while there is a row for each.
#[test]
fn every_size_down_to_nothing_shows_the_status_line_and_the_focused_row() {
    for (width, height) in SIZES {
        let mut app = Harness::new(width, height);
        draw(&mut app, "");
        let rows = app.rows();
        let mode: String = " 󰆿 NAVIGATE".chars().take(usize::from(width)).collect();
        if let Some(last) = rows.last() {
            assert!(last.starts_with(&mode), "{width}×{height}: {rows:#?}");
        }
        if height >= 3 {
            assert!(
                rows.iter().any(|row| row.starts_with('▌')),
                "{width}×{height}: {rows:#?}"
            );
        }
    }
}

// N-PORT-02: every window, the Navigator, the palette, Go to Symbol, Move to, the tag picker,
// search, the sidebar, the attachments panel, the date prompt, the settings, the shortcuts panel,
// every view, panes, editing and the weather open on every size without failing, the cursor
// stays on the screen, and keys go on working. `\x02`, `\x06`, `\x07`, `\x10` and `\x17` stand
// for `Ctrl` with `b`, `f`, `g`, `p` and `w`.
#[test]
fn every_window_opens_on_every_size() {
    let windows = [
        "?",
        "g,",
        "\x07",
        "\x07in",
        "\x10",
        "\x10weather\n",
        "/",
        "/bank",
        "gm",
        "mg",
        "\x06",
        "\x06the bank",
        "\x02",
        "\x02j'",
        "n",
        "N",
        "c",
        "md",
        "ms",
        "gt",
        "gU",
        "ga",
        "gL",
        "gd",
        "\x17v",
        "\x17v\x17v\x17v",
        "\n",
        "\nnew words",
        "o",
        "mn",
        "mN",
    ];
    for (width, height) in SIZES {
        for keys in windows {
            let mut app = Harness::new(width, height);
            for key in keys.chars() {
                match key {
                    '\n' => app.key(KeyCode::Enter),
                    '\x02' => app.ctrl('b'),
                    '\x06' => app.ctrl('f'),
                    '\x07' => app.ctrl('g'),
                    '\x10' => app.ctrl('p'),
                    '\x17' => app.ctrl('w'),
                    key => app.press(KeyCode::Char(key), KeyModifiers::NONE),
                }
                draw(&mut app, keys);
            }
            app.key(KeyCode::Esc);
            app.key(KeyCode::Esc);
            app.ctrl('q');
            assert!(app.tui.should_quit(), "{width}×{height} {keys:?}");
        }
    }
}
