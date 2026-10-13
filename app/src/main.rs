//! `tipp`: runs textual-items++ in the terminal until `Ctrl+Q`.

use std::io;

use anyhow::Context;
use crossterm::{
    event::{
        self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use textual_items_plus_plus::ui::Tui;

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::try_init().context("cannot set up the terminal")?;
    // A terminal without the kitty keyboard protocol keeps legacy key reporting, which the keymap
    // reads alike (N-PORT-03). One that does not answer the query, or refuses the request, is
    // taken not to have it.
    let kitty = crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false)
        && execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )
        .is_ok();
    // A panic skips the rest: the hook `try_init` installed leaves the alternate screen, and the
    // terminal drops the keyboard flags pushed there with it.
    let result = run(&mut terminal);
    if kitty {
        // The session is over; a terminal that cannot take this has gone already.
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

/// Draws the app and hands it each terminal event until it quits.
fn run(terminal: &mut ratatui::DefaultTerminal) -> anyhow::Result<()> {
    let mut tui = Tui::default();
    while !tui.should_quit() {
        terminal
            .draw(|frame| frame.render_widget(&tui, frame.area()))
            .context("cannot draw")?;
        tui.handle(&event::read().context("cannot read the terminal")?);
    }
    Ok(())
}
