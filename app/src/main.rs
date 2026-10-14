//! `tipp [file]`: runs textual-items++ in the terminal on the file, or on the demo document
//! without one, until `Ctrl+Q`.

use std::{env, io, path::PathBuf};

use anyhow::Context;
use crossterm::{
    event::{
        self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use textual_items_plus_plus::ui::Tui;

fn main() -> anyhow::Result<()> {
    let mut args = env::args_os().skip(1);
    let path = args.next().map(PathBuf::from);
    anyhow::ensure!(args.next().is_none(), "usage: tipp [file]");
    let tui = Tui::open(path, jiff::Zoned::now().date())?;
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
    let result = run(&mut terminal, tui);
    if kitty {
        // The session is over; a terminal that cannot take this has gone already.
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

/// Draws the app and hands it each terminal event until it quits.
fn run(terminal: &mut ratatui::DefaultTerminal, mut tui: Tui) -> anyhow::Result<()> {
    while !tui.should_quit() {
        terminal
            .draw(|frame| frame.render_widget(&mut tui, frame.area()))
            .context("cannot draw")?;
        let event = event::read().context("cannot read the terminal")?;
        tui.handle(&event, jiff::Zoned::now().date());
    }
    Ok(())
}
