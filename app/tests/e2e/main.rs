//! End-to-end smoke tests of the real binary in a pseudo-terminal, with termlens.

type TestResult = Result<(), Box<dyn std::error::Error>>;

// FR-STORE-01, FR-KEY-01, FR-NAV-01, N-PORT-03
#[test]
fn opens_the_demo_inbox_moves_through_it_and_quits_with_ctrl_q_restoring_the_terminal() -> TestResult
{
    let mut terminal = termlens::bin!("tipp")?;
    terminal.wait_until(|screen| screen.text().contains("Call the bank about the card"))?;
    terminal.send(termlens::Key::Char('j'))?;
    terminal.wait_until(|screen| screen.text().contains("▌ ☐ Buy coffee beans"))?;
    terminal.send(termlens::Key::Char('q'))?;
    terminal.send(termlens::Key::Ctrl('q'))?;
    assert!(terminal.wait_exit()?.success());
    assert!(!terminal.screen().alternate_screen());
    Ok(())
}
