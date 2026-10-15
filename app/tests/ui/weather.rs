//! The weather: the season's leaves, snow or rain falling over the app once it has sat idle, or
//! at once from the command palette, until the next input, which does nothing else.

use std::{fs, time::Duration};

use crossterm::event::{Event, KeyCode, KeyEvent};
use jiff::civil::date;
use ratatui::{buffer::Buffer, layout::Rect};

use crate::support::Harness;

/// The glyphs of autumn's leaves with Nerd Font icons, the default.
const LEAVES: [char; 4] = ['\u{f06c}', '\u{f032a}', '❦', '❧'];

/// The screen as drawn last, with its colors.
fn frame(harness: &Harness) -> Buffer {
    harness.terminal.backend().buffer().clone()
}

/// Whether autumn's leaves show on the screen.
fn leaves(harness: &Harness) -> bool {
    harness.screen().contains(LEAVES)
}

/// Lets `seconds` seconds pass, a second at a time, as the weather's frames do.
fn play(harness: &mut Harness, seconds: u32) {
    for _ in 0..seconds {
        harness.wait(1000);
    }
}

fn status(harness: &Harness) -> String {
    harness
        .rows()
        .last()
        .cloned()
        .unwrap_or_default()
        .trim_end()
        .to_owned()
}

// FR-UI-08, N-PERF-03
#[test]
fn after_three_idle_minutes_autumn_leaves_fall_and_pile_up_until_a_key_which_does_nothing_else() {
    let mut harness = animated(60, 18);
    harness.type_text("j");
    let before = frame(&harness);
    assert_eq!(
        harness.tui.deadline(),
        Some(Duration::from_mins(3)),
        "nothing but the idle timer"
    );
    harness.wait(179_999);
    assert_eq!(frame(&harness), before, "not a moment before");
    harness.wait(1);
    assert_eq!(
        harness.tui.deadline(),
        Some(Duration::from_micros(180_033_333)),
        "a frame at a time while it plays"
    );
    play(&mut harness, 60);
    assert!(leaves(&harness), "{}", harness.screen());
    assert_eq!(harness.tui.cursor(Rect::new(0, 0, 60, 18)), None);
    let bottom = harness.rows().last().cloned().unwrap_or_default();
    assert!(bottom.contains(LEAVES), "leaves settle on the bottom row");
    harness.type_text("x");
    assert_eq!(
        frame(&harness),
        before,
        "the key only ended the weather: nothing was completed"
    );
    assert_eq!(harness.tui.deadline(), Some(Duration::from_secs(420)));
    harness.type_text("x");
    assert_ne!(frame(&harness), before, "the next key works");
}

// FR-UI-08
#[test]
fn a_paste_or_a_resize_ends_the_weather_too_and_is_dropped() {
    let mut harness = animated(60, 18);
    let before = frame(&harness);
    harness.wait(180_000);
    play(&mut harness, 5);
    assert!(leaves(&harness));
    harness.send(&Event::Paste("Buy milk".into()));
    assert_eq!(frame(&harness), before, "the paste went nowhere");
    harness.wait(180_000);
    play(&mut harness, 5);
    harness.send(&Event::Resize(60, 18));
    assert!(!leaves(&harness));
    harness.send(&Event::FocusLost);
    assert_eq!(frame(&harness), before, "losing focus is not input");
}

// FR-UI-08
#[test]
fn the_command_palette_starts_the_weather_at_once() {
    let mut harness = animated(60, 18);
    harness.ctrl('p');
    harness.type_text("weather");
    harness.key(KeyCode::Enter);
    play(&mut harness, 5);
    assert!(leaves(&harness), "{}", harness.screen());
    harness.key(KeyCode::Esc);
    assert!(!leaves(&harness));
    assert!(!harness.screen().contains("Command Palette"));
}

// FR-UI-08
#[test]
fn summer_has_no_weather_and_the_palette_says_so() {
    let mut harness = animated(60, 18);
    let summer = date(2026, 7, 14);
    harness.tui.tick(Duration::ZERO, summer);
    assert_eq!(harness.tui.deadline(), None, "nothing to wait for");
    harness.tui.tick(Duration::from_mins(10), summer);
    harness.ctrl('p');
    harness.type_text("weather");
    harness
        .tui
        .handle(&Event::Key(KeyEvent::from(KeyCode::Enter)), summer);
    harness.draw();
    assert!(
        status(&harness).contains("no weather in summer"),
        "{}",
        status(&harness)
    );
    harness
        .tui
        .tick(Duration::from_mins(10) + Duration::from_secs(1), summer);
    harness.draw();
    assert_eq!(harness.tui.deadline(), None, "once the palette has closed");
}

// FR-UI-08, FR-SET-02, N-PERF-03
#[test]
fn turning_the_weather_off_in_the_settings_window_stops_the_idle_timer() {
    let mut harness = animated(80, 30);
    harness.type_text("g,jjjjjjl");
    harness.key(KeyCode::Esc);
    harness.wait(200);
    assert_eq!(harness.tui.deadline(), None, "once the window has closed");
    let before = frame(&harness);
    harness.wait(600_000);
    assert_eq!(frame(&harness), before);
    let config = fs::read_to_string(harness.config()).expect("the setting is kept");
    assert_eq!(config, "weather = false\n");
}

// FR-UI-08, FR-SET-01, FR-SET-04
#[test]
fn the_settings_file_sets_the_idle_minutes_and_a_zero_is_named_and_ignored() {
    let mut harness = with_config("weather_idle_minutes = 1\n");
    assert_eq!(harness.tui.deadline(), Some(Duration::from_mins(1)));
    harness.wait(60_000);
    play(&mut harness, 5);
    assert!(leaves(&harness));
    let harness = with_config("weather_idle_minutes = 0\n");
    assert!(
        status(&harness).contains("config.toml: `weather_idle_minutes`"),
        "{}",
        status(&harness)
    );
    assert_eq!(harness.tui.deadline(), Some(Duration::from_mins(3)));
}

// FR-UI-08, FR-UI-07
#[test]
fn the_idle_timer_waits_for_every_animation_but_the_palette_starts_the_weather_anyway() {
    let mut harness = with_config("animations = \"reduced\"\n");
    harness.type_text("j");
    assert_eq!(harness.tui.deadline(), None, "no idle timer");
    let mut off = Harness::new(60, 18);
    off.type_text("j");
    assert_eq!(off.tui.deadline(), None);
    off.ctrl('p');
    off.type_text("weather");
    off.key(KeyCode::Enter);
    play(&mut off, 5);
    assert!(leaves(&off));
}

/// The app on the demo document, animating fully, on a `width` by `height` terminal.
fn animated(width: u16, height: u16) -> Harness {
    Harness::animated(
        tempfile::tempdir().expect("a temporary folder"),
        width,
        height,
    )
}

/// The app on the demo document with a settings file holding `text`.
fn with_config(text: &str) -> Harness {
    let folders = tempfile::tempdir().expect("a temporary folder");
    fs::write(folders.path().join("config.toml"), text).expect("written");
    Harness::animated(folders, 60, 18)
}
