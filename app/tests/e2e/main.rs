//! End-to-end smoke tests of the real binary in a pseudo-terminal, with termlens.

type TestResult = Result<(), Box<dyn std::error::Error>>;

// FR-KEY-01, N-PORT-03
#[test]
fn opens_an_empty_screen_and_quits_with_ctrl_q_restoring_the_terminal() -> TestResult {
    let mut terminal = termlens::bin!("tipp")?;
    terminal.wait_until(termlens::Screen::alternate_screen)?;
    terminal.send(termlens::Key::Char('q'))?;
    terminal.send(termlens::Key::Ctrl('q'))?;
    assert!(terminal.wait_exit()?.success());
    assert!(!terminal.screen().alternate_screen());
    Ok(())
}
