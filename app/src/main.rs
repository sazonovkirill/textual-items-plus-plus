//! `tipp [file]`: runs textual-items++ in the terminal on the file, saving every change to it, or
//! on the demo document without one, until `Ctrl+Q`.

use std::{
    env, io,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::Instant,
};

use anyhow::Context;
use crossterm::{
    event::{
        self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use signal_hook::{
    consts::{SIGHUP, SIGTERM},
    iterator::Signals,
};
use textual_items_plus_plus::{
    store::{self, Recovery},
    ui::{Edge, Input, Tui},
};

fn main() -> anyhow::Result<()> {
    let mut args = env::args_os().skip(1);
    let path = args.next().map(PathBuf::from);
    anyhow::ensure!(args.next().is_none(), "usage: tipp [file]");
    let home = env::home_dir();
    let (inputs, received) = mpsc::channel();
    let edge = Edge {
        inputs: inputs.clone(),
        backups: store::backups_folder(env::var_os("XDG_DATA_HOME"), home.clone()),
        recovery: store::recovery_folder(env::var_os("XDG_STATE_HOME"), home),
        clock: || jiff::Zoned::now().datetime(),
    };
    let recovered = match &path {
        Some(path) => Recovery::new(edge.recovery.clone())
            .offer(path, &mut io::stdin().lock(), &mut io::stderr())
            .context("cannot look for recovery files")?,
        None => None,
    };
    let mut tui = Tui::open(path, recovered.as_deref(), today(), &edge)?;
    forward_signals(inputs.clone()).context("cannot listen for signals")?;
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
    forward_terminal_events(inputs);
    // The panic hook `try_init` installed restores the terminal first; once the panic has
    // unwound to here, the document is written out as it stands (N-REL-01).
    let result = panic::catch_unwind(AssertUnwindSafe(|| run(&mut terminal, &mut tui, &received)));
    if kitty {
        // The session is over; a terminal that cannot take this has gone already.
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    if let Some(farewell) = tui.farewell() {
        eprintln!("{farewell}");
    }
    result.unwrap_or_else(|panic| {
        match tui.recover() {
            Ok(Some(path)) => eprintln!("tipp: the unsaved changes are in {}", path.display()),
            Ok(None) => {}
            Err(error) => {
                eprintln!("tipp: cannot write the unsaved changes to a recovery file: {error}");
            }
        }
        panic::resume_unwind(panic)
    })
}

/// The local date now.
fn today() -> jiff::civil::Date {
    jiff::Zoned::now().date()
}

/// Turns `SIGTERM` and `SIGHUP` into [`Input::Terminate`], so the document is saved first.
fn forward_signals(inputs: Sender<Input>) -> io::Result<()> {
    let mut signals = Signals::new([SIGTERM, SIGHUP])?;
    thread::spawn(move || {
        for _ in signals.forever() {
            if inputs.send(Input::Terminate).is_err() {
                break;
            }
        }
    });
    Ok(())
}

/// Reads terminal events on a thread of their own. When reading fails the terminal is gone, as
/// after `SIGHUP`, so the session ends the same way.
fn forward_terminal_events(inputs: Sender<Input>) {
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            if inputs.send(Input::Terminal(event)).is_err() {
                return;
            }
        }
        // A closed channel means the event loop has ended already.
        let _ = inputs.send(Input::Terminate);
    });
}

/// Draws the app and hands it each input, and the time whenever a save falls due, until it
/// quits.
#[expect(
    clippy::disallowed_methods,
    reason = "the event loop is the edge that reads the clock and passes time to the app"
)]
fn run(
    terminal: &mut ratatui::DefaultTerminal,
    tui: &mut Tui,
    received: &Receiver<Input>,
) -> anyhow::Result<()> {
    let start = Instant::now();
    while !tui.should_quit() {
        terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(&mut *tui, area);
                if let Some(cursor) = tui.cursor(area) {
                    frame.set_cursor_position(cursor);
                }
            })
            .context("cannot draw")?;
        let input = match tui.deadline() {
            None => Some(received.recv()?),
            Some(deadline) => match received.recv_timeout(deadline.saturating_sub(start.elapsed()))
            {
                Ok(input) => Some(input),
                Err(RecvTimeoutError::Timeout) => None,
                Err(error) => return Err(error.into()),
            },
        };
        tui.tick(start.elapsed(), today());
        if let Some(input) = input {
            tui.input(input, today());
        }
    }
    Ok(())
}
