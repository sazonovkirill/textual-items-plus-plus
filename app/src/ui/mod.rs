//! The UI layer: the terminal frontend, which turns terminal events into app messages, carries out
//! the app's effects and draws its state.

mod container;
mod keymap;
mod picker;
mod status;
mod text_keys;
mod theme;

use std::path::PathBuf;

use crossterm::event::Event;
use jiff::civil::Date;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{StatefulWidget, Widget},
};

pub use keymap::Keymap;
pub use picker::{PickerPopup, picker_input};
pub use theme::{BuiltIn, Icon, IconSet, Theme};

use crate::{
    app::{self, Effect, Message, Opened, State},
    store::{self, OpenError},
};
use container::ContainerPane;
use status::StatusLine;

/// The running app as the terminal shows it: the core's state, the keys that drive it, how it is
/// drawn and scrolled, and whether the session has ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tui {
    state: State,
    keymap: Keymap,
    theme: Theme,
    icons: IconSet,
    /// The first line of the container pane's content on screen.
    top: usize,
    quit: bool,
}

impl Tui {
    /// The app on `state`, in the default theme and icons (D-30).
    #[must_use]
    pub fn new(state: State) -> Self {
        Self {
            state,
            keymap: Keymap::default(),
            theme: BuiltIn::default().colors(),
            icons: IconSet::default(),
            top: 0,
            quit: false,
        }
    }

    /// The app on the file at `path`, read-only until saving arrives, or without one on the
    /// demo document for `today`, the local date (FR-STORE-01, D-29).
    ///
    /// # Errors
    ///
    /// [`OpenError`] when the file cannot be read.
    pub fn open(path: Option<PathBuf>, today: Date) -> Result<Self, OpenError> {
        let state = match path {
            Some(path) => {
                // What saving needs of the outline waits for saving to arrive (M1.12).
                let (document, _) = store::open(&path)?;
                State::new(document, Opened::File(path))
            }
            None => State::new(store::demo::document(today), Opened::Demo),
        };
        Ok(Self::new(state))
    }

    /// Handles one terminal event on `today`, the local date: the keys of a bound command give
    /// it to the core, and the core's effects are carried out.
    pub fn handle(&mut self, event: &Event, today: Date) {
        if let Some(command) = self.keymap.press(event) {
            let effects = app::update(&mut self.state, Message::Command(command), today);
            for effect in effects {
                self.run(effect);
            }
        }
    }

    /// Carries out one effect of the core.
    fn run(&mut self, effect: Effect) {
        match effect {
            Effect::Quit => self.quit = true,
        }
    }

    /// Whether the session has ended, so the event loop should stop.
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.quit
    }
}

impl Widget for &mut Tui {
    /// Draws the container pane over the status line, scrolling the pane to keep the focus in
    /// view.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [pane, status] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas::<2>(area);
        ContainerPane::new(&self.state, &self.theme, self.icons).render(pane, buf, &mut self.top);
        StatusLine::new(
            self.state.opened(),
            self.keymap.pending(),
            &self.theme,
            self.icons,
        )
        .render(status, buf);
    }
}
