//! Animation: the effects drawn over each frame to show where focus landed, what changed, what
//! was refused and what opened or closed, which never hold back what an input does and end as
//! the frame drawn without them (FR-UI-07).

use std::fs;

use crossterm::event::KeyCode;
use ratatui::{buffer::Buffer, layout::Position, style::Color};

use crate::support::Harness;

/// An RGB theme, so that colors fade, and no weather, so that only effects ask for frames.
const RGB: &str = "theme = \"catppuccin-mocha\"\nweather = false\n";

/// The screen as drawn last, with its colors.
fn frame(harness: &Harness) -> Buffer {
    harness.terminal.backend().buffer().clone()
}

/// A temporary folder holding a settings file of `config`.
fn folders(config: &str) -> tempfile::TempDir {
    let folders = tempfile::tempdir().expect("a temporary folder");
    fs::write(folders.path().join("config.toml"), config).expect("written");
    folders
}

/// The demo on a 60 by 16 terminal with the settings of `config`, animating as a session not
/// over SSH does unless they say otherwise, and the same in the colors of [`RGB`] without
/// animations, to compare it with.
fn pair(config: &str) -> (Harness, Harness) {
    let animated = Harness::animated(folders(config), 60, 16);
    let still = Harness::launched_in(folders(RGB), None, 60, 16).expect("the demo opens");
    (animated, still)
}

/// Gives both `harnesses` `keys`.
fn type_both((animated, still): &mut (Harness, Harness), keys: &str) {
    animated.type_text(keys);
    still.type_text(keys);
}

/// Where on screen the focus marker is.
fn marker(harness: &Harness) -> Option<Position> {
    let buffer = harness.terminal.backend().buffer();
    buffer
        .area
        .positions()
        .find(|&position| buffer[position].symbol() == "▌")
}

/// The background of the cell right of the focus marker, on the focused row.
fn focused_bg(harness: &Harness) -> Option<Color> {
    let at = marker(harness)?;
    let buffer = harness.terminal.backend().buffer();
    Some(buffer[(at.x + 3, at.y)].bg)
}

/// Lets `millis` pass on both clocks.
fn wait_both((animated, still): &mut (Harness, Harness), millis: u64) {
    animated.wait(millis);
    still.wait(millis);
}

// FR-UI-07, N-PERF-03
#[test]
fn a_jump_flashes_where_focus_landed_then_ends_as_the_frame_without_it_asking_no_more_frames() {
    let mut both = pair(RGB);
    assert_eq!(both.0.tui.deadline(), None, "nothing moves at the start");
    type_both(&mut both, "G");
    let (animated, still) = &both;
    assert_eq!(animated.screen(), still.screen(), "the jump shows at once");
    assert_ne!(
        focused_bg(animated),
        focused_bg(still),
        "tinted where focus landed"
    );
    assert!(animated.tui.deadline().is_some(), "frames while it runs");
    wait_both(&mut both, 100);
    let (animated, still) = &both;
    assert_ne!(focused_bg(animated), focused_bg(still), "fading");
    wait_both(&mut both, 200);
    assert_eq!(frame(&both.0), frame(&both.1));
    assert_eq!(both.0.tui.deadline(), None, "no frames once it ends");
    type_both(&mut both, "k");
    assert_eq!(
        frame(&both.0),
        frame(&both.1),
        "a step to the next row is not a jump"
    );
}

// FR-UI-07
#[test]
fn an_input_during_an_effect_ends_it_and_takes_effect_in_the_same_frame() {
    let mut both = pair(RGB);
    type_both(&mut both, "Gk");
    assert_eq!(frame(&both.0), frame(&both.1));
}

// FR-UI-07
#[test]
fn completing_a_task_fades_what_it_changed_from_how_it_was_drawn() {
    let mut both = pair(RGB);
    type_both(&mut both, "j");
    type_both(&mut both, "x");
    let (animated, still) = &both;
    assert_eq!(
        animated.screen(),
        still.screen(),
        "the task is gone at once"
    );
    assert_ne!(
        frame(animated),
        frame(still),
        "what took its place fades in"
    );
    wait_both(&mut both, 250);
    assert_eq!(frame(&both.0), frame(&both.1));
    type_both(&mut both, "z");
    let (animated, still) = &both;
    assert_eq!(animated.screen(), still.screen());
    assert_ne!(
        frame(animated),
        frame(still),
        "undoing brings it back highlighted"
    );
}

// FR-UI-07
#[test]
fn a_refused_conversion_flashes_the_focused_row_in_the_error_color() {
    let mut both = pair(RGB);
    type_both(&mut both, "j");
    wait_both(&mut both, 300);
    type_both(&mut both, "my");
    let (animated, still) = &both;
    let red = |harness| match focused_bg(harness) {
        Some(Color::Rgb(red, ..)) => red,
        _ => 0,
    };
    assert!(
        red(animated) > red(still),
        "tinted toward the theme's error red"
    );
    wait_both(&mut both, 200);
    assert_eq!(frame(&both.0), frame(&both.1));
}

/// What a test does to the app.
type Step = fn(&mut Harness);

// FR-UI-07
#[test]
fn windows_and_panes_fade_in_and_out() {
    let mut both = pair(RGB);
    let steps: [(&str, Step); 4] = [
        ("the Navigator opening", |harness| harness.ctrl('k')),
        ("and closing", |harness| harness.key(KeyCode::Esc)),
        ("a pane opening", |harness| harness.ctrl('t')),
        ("and closing", |harness| {
            harness.ctrl('w');
            harness.type_text("c");
        }),
    ];
    for (what, step) in steps {
        step(&mut both.0);
        step(&mut both.1);
        let (animated, still) = &both;
        assert_eq!(animated.screen(), still.screen(), "{what} shows at once");
        assert_ne!(frame(animated), frame(still), "{what} fades");
        wait_both(&mut both, 300);
        assert_eq!(frame(&both.0), frame(&both.1), "{what} ends as it is");
    }
}

// FR-UI-07, FR-SET-02
#[test]
fn reduced_animations_flash_in_one_step_without_fades_and_off_shows_nothing() {
    let mut both = pair(&format!("{RGB}animations = \"reduced\"\n"));
    type_both(&mut both, "G");
    let flashed = focused_bg(&both.0);
    assert_ne!(flashed, focused_bg(&both.1));
    wait_both(&mut both, 100);
    assert_eq!(focused_bg(&both.0), flashed, "the same tint all along");
    wait_both(&mut both, 200);
    type_both(&mut both, "kx");
    assert_eq!(frame(&both.0), frame(&both.1), "no fades");
    let mut both = pair(&format!("{RGB}animations = \"off\"\n"));
    type_both(&mut both, "Gx");
    assert_eq!(frame(&both.0), frame(&both.1));
    assert_eq!(both.0.tui.deadline(), None);
}

// FR-UI-07, FR-SET-02
#[test]
fn turning_animations_off_in_the_settings_window_ends_them_at_once() {
    let mut harness = Harness::animated(folders(RGB), 80, 30);
    harness.type_text("g,jjjl");
    harness.type_text("l");
    assert_eq!(harness.tui.deadline(), None, "off, nothing runs");
    let config = fs::read_to_string(harness.config()).expect("the setting is kept");
    assert!(config.ends_with("animations = \"off\"\n"), "{config}");
    harness.wait(1000);
    harness.type_text("G");
    assert_eq!(harness.tui.deadline(), None);
}
