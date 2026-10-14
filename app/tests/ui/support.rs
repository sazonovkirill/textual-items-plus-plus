//! Starting the app and driving it with keys, as a user would, on a terminal of fixed size, with
//! the time and the disk work it asks for under the test's control.

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use jiff::civil::{Date, DateTime, date};
use ratatui::{Terminal, backend::TestBackend};
use tempfile::TempDir;
use textual_items_plus_plus::{
    app::Animations,
    ui::{Edge, Input, Tui},
};

/// The local date the tests run on.
pub const TODAY: Date = date(2026, 10, 5);

/// The local time the tests run at.
pub const NOW: DateTime = TODAY.at(9, 30, 0, 0);

/// A running app with the terminal it draws on.
pub struct Harness {
    pub tui: Tui,
    pub terminal: Terminal<TestBackend>,
    /// What the disk worker and the watcher report.
    received: Receiver<Input>,
    /// The app's own folders, for backups and recovery files.
    folders: TempDir,
    /// The time on the app's clock.
    now: Duration,
}

impl Harness {
    /// Starts the app without a file, on the demo document, on a `width` by `height` terminal
    /// and draws its first screen, the sidebar hidden as every way of starting it but
    /// [`launched`](Self::launched) hides it.
    pub fn new(width: u16, height: u16) -> Self {
        Self::on(None, None, width, height).expect("the demo opens")
    }

    /// Starts the app without a file, on the demo document, on a `width` by `height` terminal
    /// and draws its first screen as a user sees it, the sidebar showing. The other ways of
    /// starting it hide the sidebar, so that what tests not about it see is the panes alone.
    pub fn launched(width: u16, height: u16) -> Self {
        let folders = tempfile::tempdir().expect("a temporary folder");
        Self::launched_in(folders, None, width, height).expect("the demo opens")
    }

    /// Starts the app on `path`, else the demo, keeping its own files in `folders`, and draws its
    /// first screen as a user sees it, the sidebar showing, before any key.
    pub fn launched_in(
        folders: TempDir,
        path: Option<PathBuf>,
        width: u16,
        height: u16,
    ) -> Result<Self, textual_items_plus_plus::store::OpenError> {
        Self::starting(folders, path, None, (width, height), Animations::Off, true)
    }

    /// Starts the app as `tipp <path>` would, on a `width` by `height` terminal, and draws its
    /// first screen.
    pub fn open(path: PathBuf, width: u16, height: u16) -> Self {
        Self::on(Some(path), None, width, height).expect("the test's file opens")
    }

    /// Starts the app on `path`, with the document of the recovery file `recovered` when one is
    /// given, keeping its own files in a folder of the test's.
    pub fn on(
        path: Option<PathBuf>,
        recovered: Option<&Path>,
        width: u16,
        height: u16,
    ) -> Result<Self, textual_items_plus_plus::store::OpenError> {
        let folders = tempfile::tempdir().expect("a temporary folder");
        Self::in_folders(folders, path, recovered, width, height)
    }

    /// Starts the app as [`on`](Self::on) does, keeping its own files in `folders`.
    pub fn in_folders(
        folders: TempDir,
        path: Option<PathBuf>,
        recovered: Option<&Path>,
        width: u16,
        height: u16,
    ) -> Result<Self, textual_items_plus_plus::store::OpenError> {
        Self::starting(
            folders,
            path,
            recovered,
            (width, height),
            Animations::Off,
            false,
        )
    }

    /// Starts the app without a file, on the demo document, on a `width` by `height` terminal,
    /// with every animation unless the settings file in `folders` says otherwise, as a session
    /// not over SSH does, and the sidebar showing, since hiding it would animate; the other ways
    /// of starting it animate nothing, so that what tests not about animation see is each frame
    /// as it ends.
    pub fn animated(folders: TempDir, width: u16, height: u16) -> Self {
        Self::starting(folders, None, None, (width, height), Animations::Full, true)
            .expect("the demo opens")
    }

    /// Starts the app on `path`, with the document of the recovery file `recovered` when one is
    /// given, keeping its own files in `folders`, on a terminal of `size`, animating as
    /// `animations` says unless the settings file does, and with the sidebar it starts with
    /// hidden unless `sidebar`.
    fn starting(
        folders: TempDir,
        path: Option<PathBuf>,
        recovered: Option<&Path>,
        (width, height): (u16, u16),
        animations: Animations,
        sidebar: bool,
    ) -> Result<Self, textual_items_plus_plus::store::OpenError> {
        let terminal =
            Terminal::new(TestBackend::new(width, height)).expect("a test backend always sets up");
        let (inputs, received) = mpsc::channel();
        let edge = Edge {
            inputs,
            backups: Some(folders.path().join("backups")),
            recovery: Some(folders.path().join("recovery")),
            layouts: Some(folders.path().join("layouts.json")),
            config: Some(folders.path().join("config.toml")),
            clock: || NOW,
            animations,
        };
        let tui = Tui::open(path, recovered, TODAY, &edge)?;
        let mut harness = Self {
            tui,
            terminal,
            received,
            folders,
            now: Duration::ZERO,
        };
        if !sidebar {
            // The first `Ctrl+B` gives the sidebar the keys, the second hides it, before the
            // first screen is drawn.
            let ctrl_b = Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
            harness.tui.handle(&ctrl_b, TODAY);
            harness.tui.handle(&ctrl_b, TODAY);
        }
        harness.settle();
        Ok(harness)
    }

    /// Quits with `Ctrl+Q` and starts the app again on `path`, keeping its own folders, as the
    /// next launch does, on a terminal of the same size.
    pub fn relaunch(mut self, path: PathBuf) -> Self {
        self.ctrl('q');
        assert!(self.tui.should_quit(), "the app quits");
        let area = self.terminal.backend().buffer().area;
        Self::in_folders(self.folders, Some(path), None, area.width, area.height)
            .expect("the file opens again")
    }

    /// Where the app keeps its backups.
    pub fn backups(&self) -> PathBuf {
        self.folders.path().join("backups")
    }

    /// Where the app keeps its settings.
    pub fn config(&self) -> PathBuf {
        self.folders.path().join("config.toml")
    }

    /// Where the app keeps its recovery files.
    pub fn recovery(&self) -> PathBuf {
        self.folders.path().join("recovery")
    }

    /// Hands the app `event` and draws what it shows after it, once the disk work it asked for
    /// is reported.
    pub fn send(&mut self, event: &Event) {
        self.tui.handle(event, TODAY);
        self.settle();
    }

    /// Lets `millis` milliseconds pass on the app's clock.
    pub fn wait(&mut self, millis: u64) {
        self.now += Duration::from_millis(millis);
        self.tui.tick(self.now, TODAY);
        self.settle();
    }

    /// Tells the app the file may have changed, as its watcher does.
    pub fn check(&mut self) {
        self.tui.input(Input::Check, TODAY);
        self.settle();
    }

    /// Hands the app what the watcher reports until `done` holds, failing when it reports nothing
    /// for ten seconds.
    pub fn watch_until(&mut self, done: impl Fn(&Self) -> bool) {
        while !done(self) {
            let input = self
                .received
                .recv_timeout(Duration::from_secs(10))
                .expect("the watcher reports");
            self.tui.input(input, TODAY);
            self.settle();
        }
    }

    /// Tells the app to end, as `SIGTERM` does.
    pub fn terminate(&mut self) {
        self.tui.input(Input::Terminate, TODAY);
        self.settle();
    }

    /// Hands the app every report of the disk work it asked for, and of what that asked for in
    /// turn, then draws it.
    pub fn settle(&mut self) {
        loop {
            self.tui.flush();
            let mut reported = false;
            while let Ok(input) = self.received.try_recv() {
                reported = true;
                self.tui.input(input, TODAY);
            }
            if !reported {
                break;
            }
        }
        self.draw();
    }

    /// Presses `code` with `modifiers`.
    pub fn press(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.send(&Event::Key(KeyEvent::new(code, modifiers)));
    }

    /// Presses the character `c` with `Ctrl`.
    pub fn ctrl(&mut self, c: char) {
        self.press(KeyCode::Char(c), KeyModifiers::CONTROL);
    }

    /// Presses `code` alone.
    pub fn key(&mut self, code: KeyCode) {
        self.press(code, KeyModifiers::NONE);
    }

    /// Types `text`, one character key at a time.
    pub fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.press(KeyCode::Char(c), KeyModifiers::NONE);
        }
    }

    /// Draws the app on the terminal.
    pub fn draw(&mut self) {
        let tui = &mut self.tui;
        self.terminal
            .draw(|frame| frame.render_widget(tui, frame.area()))
            .expect("a test backend always draws");
    }

    /// The screen as drawn last, one string per row.
    pub fn rows(&self) -> Vec<String> {
        let buffer = self.terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    /// The screen as drawn last, its rows joined by newlines.
    pub fn screen(&self) -> String {
        self.rows().join("\n")
    }
}

/// `text` up to the first date chip at the right of a row, without the spaces before it.
pub fn without_chips(text: &str) -> &str {
    let chip = ['\u{f166d}', '\u{f166c}', '\u{f0a31}'];
    text.split(chip).next().unwrap_or(text).trim_end()
}
