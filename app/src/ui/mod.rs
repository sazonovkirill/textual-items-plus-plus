//! The UI layer: the terminal frontend, which turns terminal events into app messages, carries out
//! the app's effects and draws its state.

mod keymap;

use crossterm::event::Event;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

pub use keymap::Keymap;

use crate::app::{self, Effect, Message, State};

/// The running app as the terminal shows it: the core's state, the keys that drive it and
/// whether the session has ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tui {
    state: State,
    keymap: Keymap,
    quit: bool,
}

impl Tui {
    /// Handles one terminal event: a bound key gives its command to the core, and the core's
    /// effects are carried out.
    pub fn handle(&mut self, event: &Event) {
        if let Some(command) = self.keymap.command(event) {
            let effects = app::update(&mut self.state, Message::Command(command));
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

impl Widget for &Tui {
    /// Draws the app; the screen stays empty until the container view arrives.
    fn render(self, _area: Rect, _buf: &mut Buffer) {}
}
