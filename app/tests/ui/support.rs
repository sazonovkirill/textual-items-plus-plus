//! Starting the app and driving it with keys, as a user would, on a terminal of fixed size.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use textual_items_plus_plus::ui::Tui;

/// A running app with the terminal it draws on.
pub struct Harness {
    pub tui: Tui,
    pub terminal: Terminal<TestBackend>,
}

impl Harness {
    /// Starts the app on a `width` by `height` terminal and draws its first screen.
    pub fn new(width: u16, height: u16) -> Self {
        let terminal =
            Terminal::new(TestBackend::new(width, height)).expect("a test backend always sets up");
        let mut harness = Self {
            tui: Tui::default(),
            terminal,
        };
        harness.draw();
        harness
    }

    /// Hands the app `event` and draws what it shows after it.
    pub fn send(&mut self, event: &Event) {
        self.tui.handle(event);
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

    /// Types `text`, one character key at a time.
    pub fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.press(KeyCode::Char(c), KeyModifiers::NONE);
        }
    }

    /// Draws the app on the terminal.
    pub fn draw(&mut self) {
        let tui = &self.tui;
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
}
