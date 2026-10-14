//! `tipp [file]`: runs textual-items++ in the terminal on the file, saving every change to it, or
//! on the demo document without one, until `Ctrl+Q`.

use std::{
    env,
    io::{self, Write as _},
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant},
};

use anyhow::Context;
use crossterm::{
    clipboard::CopyToClipboard,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute, queue,
    terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
};
use signal_hook::{
    consts::{SIGHUP, SIGTERM},
    iterator::Signals,
};
use textual_items_plus_plus::{
    store::{self, Recovery},
    ui::{self, Edge, Input, SystemClipboard, Tui},
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
        recovery: store::recovery_folder(env::var_os("XDG_STATE_HOME"), home.clone()),
        layouts: store::layouts_file(env::var_os("XDG_STATE_HOME"), home.clone()),
        config: store::config_file(env::var_os("XDG_CONFIG_HOME"), home.clone()),
        home,
        clock: || jiff::Zoned::now().datetime(),
        animations: ui::animations_default(
            env::var_os("SSH_CONNECTION").as_deref(),
            env::var_os("SSH_TTY").as_deref(),
        ),
    };
    let recovered = match &path {
        Some(path) => Recovery::new(edge.recovery.clone())
            .offer(path, &mut io::stdin().lock(), &mut io::stderr())
            .context("cannot look for recovery files")?,
        None => None,
    };
    let mut tui = Tui::open(path, recovered.as_deref(), today(), &edge)?;
    tui.use_clipboard(Box::new(System(arboard::Clipboard::new().ok())));
    forward_signals(inputs.clone()).context("cannot listen for signals")?;
    let report = panic::take_hook();
    let mut terminal = ratatui::try_init().context("cannot set up the terminal")?;
    // In place of `try_init`'s hook, whose report of a restore that failed panics in turn when
    // the terminal is gone, aborting before the document is kept (D-96).
    panic::set_hook(Box::new(move |info| {
        let _ = ratatui::try_restore();
        report(info);
    }));
    // A terminal without the kitty keyboard protocol keeps legacy key reporting, which the keymap
    // reads alike (N-PORT-03). One that does not answer the query, or refuses the request, is
    // taken not to have it.
    let kitty = crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false)
        && execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )
        .is_ok();
    tui.use_kitty(kitty);
    // Text pasted into a terminal that does not bracket it arrives as typed keys.
    let bracketed = execute!(io::stdout(), EnableBracketedPaste).is_ok();
    forward_terminal_events(inputs);
    // The panic hook restores the terminal first; once the panic has unwound to here, or the
    // event loop has failed, as when the terminal is gone, the document is kept before anything
    // is written to the terminal, and what cannot be shown there is dropped (N-REL-01, D-96).
    let result = panic::catch_unwind(AssertUnwindSafe(|| run(&mut terminal, &mut tui, &received)));
    let recovered = (!matches!(result, Ok(Ok(())))).then(|| tui.recover());
    if kitty {
        // The session is over; a terminal that cannot take this has gone already.
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    if bracketed {
        // As for the keyboard flags above.
        let _ = execute!(io::stdout(), DisableBracketedPaste);
    }
    // The session may end, or have panicked, while the mouse is reported (D-74); a terminal
    // that does not report it ignores this.
    let _ = execute!(io::stdout(), DisableMouseCapture);
    let _ = ratatui::try_restore();
    let mut stderr = io::stderr();
    if let Some(farewell) = tui.farewell() {
        let _ = writeln!(stderr, "{farewell}");
    }
    let _ = match recovered {
        Some(Ok(Some(path))) => writeln!(
            stderr,
            "tipp: the unsaved changes are in {}",
            path.display()
        ),
        Some(Err(error)) => writeln!(
            stderr,
            "tipp: cannot write the unsaved changes to a recovery file: {error}"
        ),
        Some(Ok(None)) | None => Ok(()),
    };
    result.unwrap_or_else(|panic| panic::resume_unwind(panic))
}

/// The system clipboard: the desktop's when a display server is reachable, else the terminal's
/// through OSC 52, which copies over SSH and in tmux with `set-clipboard on` but cannot be read,
/// so pasting then takes the text the app copied last, and text pasted into the terminal comes
/// as a paste event, as in taskpaper-terminal.
struct System(Option<arboard::Clipboard>);

impl std::fmt::Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("System").field(&self.0.is_some()).finish()
    }
}

impl SystemClipboard for System {
    fn set(&mut self, text: &str) {
        let copied = self
            .0
            .as_mut()
            .is_some_and(|clipboard| clipboard.set_text(text).is_ok());
        if !copied {
            // A terminal without OSC 52 ignores it, and there is no other way left to copy.
            let _ = execute!(io::stdout(), CopyToClipboard::to_clipboard_from(text));
        }
    }

    fn get(&mut self) -> Option<String> {
        self.0.as_mut()?.get_text().ok()
    }
}

/// The local date now.
fn today() -> jiff::civil::Date {
    jiff::Zoned::now().date()
}

/// How long until the local date changes, when the screen is drawn again so that date chips name
/// dates from the new day (FR-DATE-02); a day when that cannot be told.
fn until_midnight() -> Duration {
    let now = jiff::Zoned::now();
    now.tomorrow()
        .and_then(|tomorrow| tomorrow.start_of_day())
        .ok()
        .and_then(|midnight| Duration::try_from(now.duration_until(&midnight)).ok())
        .unwrap_or(Duration::from_hours(24))
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

/// Draws the app and hands it each input, and the time whenever a save falls due or the local
/// date changes, until it quits.
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
    let mut mouse = false;
    while !tui.should_quit() {
        if tui.captures_mouse() != mouse {
            mouse = !mouse;
            // A terminal that cannot report the mouse keeps the wheel to itself, and one that is
            // gone is noticed by drawing.
            let _ = if mouse {
                execute!(terminal.backend_mut(), EnableMouseCapture)
            } else {
                execute!(terminal.backend_mut(), DisableMouseCapture)
            };
        }
        // The terminal shows each frame whole, as an effect changes many cells at once. One that
        // does not know synchronized output ignores it, and one that is gone is noticed by
        // drawing.
        let _ = queue!(terminal.backend_mut(), BeginSynchronizedUpdate);
        terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(&mut *tui, area);
                if let Some(cursor) = tui.cursor(area) {
                    frame.set_cursor_position(cursor);
                }
            })
            .context("cannot draw")?;
        let _ = execute!(terminal.backend_mut(), EndSynchronizedUpdate);
        let wait = tui
            .deadline()
            .map_or(Duration::MAX, |deadline| {
                deadline.saturating_sub(start.elapsed())
            })
            .min(until_midnight());
        let input = match received.recv_timeout(wait) {
            Ok(input) => Some(input),
            Err(RecvTimeoutError::Timeout) => None,
            Err(error) => return Err(error.into()),
        };
        tui.tick(start.elapsed(), today());
        if let Some(input) = input {
            tui.input(input, today());
        }
    }
    Ok(())
}
