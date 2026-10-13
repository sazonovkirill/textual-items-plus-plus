//! Starting on an empty screen and quitting with `Ctrl+Q`.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::support::Harness;

#[test]
fn the_app_starts_on_an_empty_screen() {
    let harness = Harness::new(80, 24);
    harness
        .terminal
        .backend()
        .assert_buffer_lines(vec![" ".repeat(80); 24]);
    assert!(!harness.tui.should_quit());
}

// FR-KEY-01
#[test]
fn ctrl_q_quits() {
    let mut harness = Harness::new(80, 24);
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
}

#[test]
fn other_keys_do_not_quit() {
    let mut harness = Harness::new(80, 24);
    harness.type_text("qQ");
    harness.press(KeyCode::Esc, KeyModifiers::NONE);
    harness.press(KeyCode::Char('q'), KeyModifiers::ALT);
    harness.ctrl('w');
    harness.send(&Event::Resize(40, 10));
    assert!(!harness.tui.should_quit());
    assert!(harness.rows().iter().all(|row| row.trim().is_empty()));
}

// N-PORT-03
#[test]
fn releasing_ctrl_q_does_not_quit() {
    let mut harness = Harness::new(80, 24);
    let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
    release.kind = KeyEventKind::Release;
    harness.send(&Event::Key(release));
    assert!(!harness.tui.should_quit());
}

#[test]
fn a_tiny_terminal_draws_without_failing() {
    let mut harness = Harness::new(1, 1);
    harness.terminal.backend_mut().resize(0, 0);
    harness.draw();
    harness.ctrl('q');
    assert!(harness.tui.should_quit());
}
