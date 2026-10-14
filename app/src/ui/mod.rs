//! The UI layer: the terminal frontend, which turns terminal events into app messages, carries out
//! the app's effects and draws its state.

mod container;
mod crumbs;
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
    layout::{Constraint, Layout, Position, Rect},
    style::Style,
    text::Line,
    widgets::{StatefulWidget, Widget},
};

pub use keymap::Keymap;
pub use picker::{PickerPopup, picker_input};
pub use theme::{BuiltIn, Icon, IconSet, Theme};

use crate::{
    app::{self, Context, Effect, Message, Opened, Place, REGISTRY, State},
    domain::ItemKind,
    store::{self, OpenError},
};
use container::ContainerPane;
use crumbs::Crumbs;
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
    /// it to the core, and the core's effects are carried out. While the Navigator is open it
    /// takes the keys, leaving only those of global commands, such as its own, which closes it
    /// (FR-VIEW-07).
    pub fn handle(&mut self, event: &Event, today: Date) {
        let message = if self.state.navigator().is_some() {
            picker_input(event).map(Message::Navigator).or_else(|| {
                self.keymap
                    .press(event)
                    .filter(|&command| {
                        REGISTRY.iter().any(|entry| {
                            entry.command == command && entry.context == Context::Global
                        })
                    })
                    .map(Message::Command)
            })
        } else {
            self.keymap.press(event).map(Message::Command)
        };
        if let Some(message) = message {
            for effect in app::update(&mut self.state, message, today) {
                self.run(effect);
            }
        }
    }

    /// Where the terminal's cursor goes when the app is drawn over `area`: at the caret of the
    /// Navigator's query while it is open, else hidden.
    #[must_use]
    pub fn cursor(&self, area: Rect) -> Option<Position> {
        let view = self.state.navigator()?;
        let [panes, _] = layout(area);
        Some(navigator_popup(&view, &self.theme, self.icons).caret(panes))
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

/// The parts of the screen: where the panes go over the status line.
fn layout(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area)
}

/// The Navigator's window, each place after its kind's icon, in a tree with its open tasks
/// counted at the right (FR-VIEW-07).
fn navigator_popup<'a>(
    view: &'a app::PickerView<'a, Place>,
    theme: &'a Theme,
    icons: IconSet,
) -> PickerPopup<'a, Place> {
    let icon: &dyn Fn(&Place) -> Icon = &|place| match place.kind {
        ItemKind::Inbox => Icon::Inbox,
        ItemKind::Area => Icon::Area,
        _ => Icon::Project,
    };
    PickerPopup::new("Navigator", view, icon, icons, theme)
        .tree(&|place| place.depth)
        .badge(&|place| match place.open_tasks {
            0 => String::new(),
            count => count.to_string(),
        })
}

impl Widget for &mut Tui {
    /// Draws the pane, below its breadcrumbs and a rule, over the status line, scrolling the pane
    /// to keep the focus in view, and the Navigator over the pane while it is open.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [panes, status] = layout(area);
        // A terminal too small for the crumbs, the rule and a row keeps the row.
        let header = u16::from(panes.height > 2);
        let [crumbs, rule, pane] = Layout::vertical([
            Constraint::Length(header),
            Constraint::Length(header),
            Constraint::Fill(1),
        ])
        .areas(panes);
        let path = self.state.path();
        Crumbs::new(self.state.document(), &path, &self.theme, self.icons).render(crumbs, buf);
        Line::styled(
            "─".repeat(usize::from(rule.width)),
            Style::new().fg(self.theme.rule),
        )
        .render(rule, buf);
        ContainerPane::new(&self.state, &self.theme, self.icons).render(pane, buf, &mut self.top);
        if let Some(view) = self.state.navigator() {
            navigator_popup(&view, &self.theme, self.icons).render(panes, buf);
        }
        StatusLine::new(
            self.state.opened(),
            self.keymap.pending(),
            &self.theme,
            self.icons,
        )
        .render(status, buf);
    }
}
