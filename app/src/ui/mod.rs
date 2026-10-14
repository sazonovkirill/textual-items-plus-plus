//! The UI layer: the terminal frontend, which turns terminal events into app messages, carries out
//! the app's effects and draws its state.

mod clipboard;
mod conflict;
mod container;
mod crumbs;
mod edit_keys;
mod file;
mod keymap;
mod picker;
mod status;
mod text_keys;
mod theme;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crossterm::event::Event;
use jiff::civil::Date;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Position, Rect},
    style::Style,
    text::Line,
    widgets::{StatefulWidget, Widget},
};

pub use clipboard::SystemClipboard;
pub use file::Edge;
pub use keymap::Keymap;
pub use picker::{PickerPopup, picker_input};
pub use theme::{BuiltIn, Icon, IconSet, Theme};

use crate::{
    app::{
        self, Command, Context, Destination, EditInput, Effect, Message, Opened, Part, PickerInput,
        Place, REGISTRY, Side, State, TextEdit,
    },
    domain::ItemKind,
    store::{self, OpenError, Outcome, RecoveryError},
};
use clipboard::Clipboard;
use conflict::ConflictPrompt;
use container::ContainerPane;
use crumbs::Crumbs;
use file::File;
use status::StatusLine;

/// What reaches the event loop: what the terminal, the disk worker, the watcher and the process
/// report.
#[derive(Debug)]
pub enum Input {
    /// A terminal event.
    Terminal(Event),
    /// How disk work went.
    Disk(Outcome),
    /// The watcher saw a change that may be to the file, so it is read again.
    Check,
    /// The process was told to end, as by `SIGTERM` or `SIGHUP`.
    Terminate,
}

/// The running app as the terminal shows it: the core's state, the keys that drive it, how it is
/// drawn and scrolled, the file it keeps in step, the clipboard and whether the session has
/// ended.
#[derive(Debug)]
pub struct Tui {
    state: State,
    keymap: Keymap,
    theme: Theme,
    icons: IconSet,
    /// The first line of the container pane's content on screen.
    top: usize,
    /// Where the app was last drawn.
    area: Rect,
    quit: bool,
    file: Option<File>,
    clipboard: Clipboard,
    /// The local date, as last given.
    today: Date,
}

impl Tui {
    /// The app on `state`, without a file, in the default theme and icons (D-30).
    #[must_use]
    pub fn new(state: State, today: Date) -> Self {
        Self {
            state,
            keymap: Keymap::default(),
            theme: BuiltIn::default().colors(),
            icons: IconSet::default(),
            top: 0,
            area: Rect::default(),
            quit: false,
            file: None,
            clipboard: Clipboard::default(),
            today,
        }
    }

    /// Copies to and pastes from `system` from now on, keeping only the app's own copy without
    /// it (FR-OPS-05).
    pub fn use_clipboard(&mut self, system: Box<dyn SystemClipboard>) {
        self.clipboard = Clipboard::new(system);
    }

    /// The app on the file at `path`, every change saved to it, with the document of the
    /// recovery file `recovered` when one is given, or without a file on the demo document for
    /// `today`, the local date, never saved (FR-STORE-01, FR-STORE-09, D-29). `edge` says where
    /// the app's own files go and where the disk worker and the watcher report.
    ///
    /// # Errors
    ///
    /// [`OpenError`] when the file cannot be opened.
    pub fn open(
        path: Option<PathBuf>,
        recovered: Option<&Path>,
        today: Date,
        edge: &Edge,
    ) -> Result<Self, OpenError> {
        let Some(path) = path else {
            return Ok(Self::new(
                State::new(store::demo::document(today), Opened::Demo),
                today,
            ));
        };
        let (file, state) = File::open(path, recovered, today, edge)?;
        Ok(Self {
            file: Some(file),
            ..Self::new(state, today)
        })
    }

    /// Takes in `input` on `today`, the local date.
    pub fn input(&mut self, input: Input, today: Date) {
        self.today = today;
        match input {
            Input::Terminal(event) => self.handle(&event, today),
            Input::Disk(outcome) => {
                let messages = self
                    .file
                    .as_mut()
                    .map(|file| file.outcome(outcome))
                    .unwrap_or_default();
                for message in messages {
                    self.update(message);
                }
            }
            Input::Check => {
                if let Some(file) = &self.file {
                    file.check();
                }
            }
            Input::Terminate => self.update(Message::Terminate),
        }
    }

    /// The frontend's clock reads `now`, counted from the start of the session, on `today`, the
    /// local date: a save that is due starts, and a key sequence waiting too long ends, giving
    /// its command when it is a binding of its own.
    pub fn tick(&mut self, now: Duration, today: Date) {
        self.today = today;
        self.update(Message::Tick(now));
        if let Some(command) = self.keymap.tick(now) {
            self.give(command);
        }
    }

    /// Waits until the disk work asked for so far is done and its reports are among the inputs.
    pub fn flush(&self) {
        if let Some(file) = &self.file {
            file.flush();
        }
    }

    /// When the next [`tick`](Self::tick) is due without any input, if one is.
    #[must_use]
    pub fn deadline(&self) -> Option<Duration> {
        [self.state.deadline(), self.keymap.deadline()]
            .into_iter()
            .flatten()
            .min()
    }

    /// Handles one terminal event on `today`, the local date: the keys of a bound command give
    /// it to the core, and the core's effects are carried out. While the conflict prompt is open
    /// it takes only its own keys and quitting. While the Navigator is open it takes the keys,
    /// leaving only those of global commands, such as its own, which closes it (FR-VIEW-07); the
    /// Move to picker (FR-OPS-04) and, in edit mode, the title or description being edited take
    /// them the same way (FR-EDIT-01). A key that cuts a sequence short gives the sequence's command first,
    /// then acts on its own.
    pub fn handle(&mut self, event: &Event, today: Date) {
        self.today = today;
        if let Event::Paste(text) = event {
            self.paste(text);
            return;
        }
        if let Some(command) = self.keymap.cut_short(event) {
            self.give(command);
        }
        let message = if self.state.conflict().is_some() {
            self.keymap
                .press(event)
                .filter(|&command| self.allows(command))
                .map(Message::Command)
        } else if let Some((_, part, _)) = self.state.editing() {
            match part {
                Part::Title => edit_keys::edit_input(event),
                Part::Description => {
                    edit_keys::description_input(event, self.description_columns())
                }
            }
            .map(Message::Edit)
            .or_else(|| self.global(event))
        } else if self.state.navigator().is_some() {
            picker_input(event)
                .map(Message::Navigator)
                .or_else(|| self.global(event))
        } else if self.state.move_to().is_some() {
            picker_input(event)
                .map(Message::MoveTo)
                .or_else(|| self.global(event))
        } else {
            self.keymap.press(event).map(Message::Command)
        };
        if let Some(message) = message {
            self.update(message);
        }
    }

    /// Takes in `text` pasted into the terminal (FR-OPS-05), without the line breaks that end
    /// it: a description being edited takes it whole; a title being edited takes a single line,
    /// while several lines end editing and paste as items below; the query of the open Navigator
    /// or Move to takes it with its line breaks as spaces; otherwise the items it holds paste
    /// below the focus. The conflict prompt takes none.
    fn paste(&mut self, text: &str) {
        if self.state.conflict().is_some() {
            return;
        }
        let text = text.trim_end_matches(['\r', '\n']);
        let several = text.contains(['\r', '\n']);
        let part = self.state.editing().map(|(_, part, _)| part);
        if part == Some(Part::Description) {
            for c in text.replace("\r\n", "\n").replace('\r', "\n").chars() {
                self.update(Message::Edit(EditInput::Text(TextEdit::Insert(c))));
            }
        } else if part.is_some() && !several {
            for c in text.chars() {
                self.update(Message::Edit(EditInput::Text(TextEdit::Insert(c))));
            }
        } else if self.state.navigator().is_some() || self.state.move_to().is_some() {
            let navigator = self.state.navigator().is_some();
            for c in text.chars() {
                let c = if matches!(c, '\r' | '\n') { ' ' } else { c };
                let input = PickerInput::Edit(TextEdit::Insert(c));
                self.update(if navigator {
                    Message::Navigator(input)
                } else {
                    Message::MoveTo(input)
                });
            }
        } else {
            let branches = store::taskpaper::read_branches(text);
            self.update(Message::Paste(Side::Below, branches));
        }
    }

    /// Gives `message` to the core and carries out its effects.
    fn update(&mut self, message: Message) {
        for effect in app::update(&mut self.state, message, self.today) {
            self.run(effect);
        }
    }

    /// The message of the global command whose keys `event` completes, if any.
    fn global(&mut self, event: &Event) -> Option<Message> {
        self.keymap
            .press(event)
            .filter(|&command| self.allows(command))
            .map(Message::Command)
    }

    /// Gives `command` to the core when the app takes it now.
    fn give(&mut self, command: Command) {
        if self.allows(command) {
            self.update(Message::Command(command));
        }
    }

    /// Whether the app takes `command` now: while the conflict prompt is open only its own
    /// commands and quitting, while the Navigator or Move to is open or in edit mode only global
    /// ones, and otherwise any.
    fn allows(&self, command: Command) -> bool {
        let context = REGISTRY
            .iter()
            .find(|entry| entry.command == command)
            .map(|entry| entry.context);
        if self.state.conflict().is_some() {
            command == Command::Quit || context == Some(Context::Conflict)
        } else if self.state.editing().is_some()
            || self.state.navigator().is_some()
            || self.state.move_to().is_some()
        {
            context == Some(Context::Global)
        } else {
            true
        }
    }

    /// Where the terminal's cursor goes when the app is drawn over `area`: at the caret of the
    /// query of the Navigator or Move to while one is open, at the caret of the text being edited
    /// in edit mode, else hidden.
    #[must_use]
    pub fn cursor(&self, area: Rect) -> Option<Position> {
        if self.state.conflict().is_some() {
            return None;
        }
        let [panes, _] = layout(area);
        if let Some(view) = self.state.navigator() {
            return Some(navigator_popup(&view, &self.theme, self.icons).caret(panes));
        }
        if let Some(view) = self.state.move_to() {
            return Some(move_to_popup(&view, &self.theme, self.icons).caret(panes));
        }
        ContainerPane::new(&self.state, &self.theme, self.icons)
            .caret(pane_layout(panes)[2], self.top)
    }

    /// The columns the description being edited wraps in, as last drawn.
    fn description_columns(&self) -> usize {
        let [panes, _] = layout(self.area);
        ContainerPane::new(&self.state, &self.theme, self.icons)
            .description_columns(pane_layout(panes)[2])
            .unwrap_or(usize::MAX)
    }

    /// Carries out one effect of the core.
    fn run(&mut self, effect: Effect) {
        match effect {
            Effect::Quit => self.quit = true,
            Effect::Save => {
                if let Some(file) = &self.file {
                    file.save(self.state.document(), self.today);
                }
            }
            Effect::Copy { side, ending } => {
                let report = self
                    .file
                    .as_ref()
                    .and_then(|file| file.copy(side, self.state.document(), ending));
                if let Some(report) = report {
                    self.update(report);
                }
            }
            Effect::Reloaded => {
                if let Some(file) = &mut self.file {
                    file.reloaded();
                }
            }
            Effect::SetClipboard(branches) => self.clipboard.copy(&branches),
            Effect::ReadClipboard(side) => {
                let branches = self.clipboard.branches();
                self.update(Message::Paste(side, branches));
            }
        }
    }

    /// Whether the session has ended, so the event loop should stop.
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// After a crash, writes the document to a recovery file when it holds changes its file
    /// lacks, and returns where (N-REL-01); `None` when there is nothing to keep.
    ///
    /// # Errors
    ///
    /// [`RecoveryError`] when the recovery file could not be written.
    pub fn recover(&self) -> Result<Option<PathBuf>, RecoveryError> {
        match &self.file {
            Some(file) if self.state.save_state() != app::autosave::SaveState::Saved => {
                file.recover(self.state.document()).map(Some)
            }
            _ => Ok(None),
        }
    }

    /// What to say once the terminal is restored: where the document went when the session
    /// ended with the conflict prompt open.
    #[must_use]
    pub fn farewell(&self) -> Option<String> {
        self.state
            .left_in()
            .map(|copy| format!("tipp: left theirs on disk; mine is in {}", copy.display()))
    }
}

/// The parts of the screen: where the panes go over the status line.
fn layout(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area)
}

/// The parts of a pane: its breadcrumbs, the rule under them and its rows. A terminal too small
/// for the crumbs, the rule and a row keeps the row.
fn pane_layout(pane: Rect) -> [Rect; 3] {
    let header = u16::from(pane.height > 2);
    Layout::vertical([
        Constraint::Length(header),
        Constraint::Length(header),
        Constraint::Fill(1),
    ])
    .areas(pane)
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

/// The Move to window, each destination after its kind's icon, in a tree (FR-OPS-04).
fn move_to_popup<'a>(
    view: &'a app::PickerView<'a, Destination>,
    theme: &'a Theme,
    icons: IconSet,
) -> PickerPopup<'a, Destination> {
    let icon: &dyn Fn(&Destination) -> Icon = &|destination| match destination {
        Destination::Home => Icon::Home,
        Destination::Container { kind, .. } => match kind {
            ItemKind::Inbox => Icon::Inbox,
            ItemKind::Area => Icon::Area,
            ItemKind::Section => Icon::Section,
            _ => Icon::Project,
        },
    };
    PickerPopup::new("Move to", view, icon, icons, theme).tree(&|destination| match destination {
        Destination::Home => 0,
        Destination::Container { depth, .. } => *depth,
    })
}

impl Widget for &mut Tui {
    /// Draws the pane, below its breadcrumbs and a rule, over the status line, scrolling the pane
    /// to keep the focus in view, and the Navigator, Move to or the conflict prompt over the pane
    /// while it is open.
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = area;
        let [panes, status] = layout(area);
        let [crumbs, rule, pane] = pane_layout(panes);
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
        if let Some(view) = self.state.move_to() {
            move_to_popup(&view, &self.theme, self.icons).render(panes, buf);
        }
        if let (Some(view), Some(file)) = (self.state.conflict(), &self.file) {
            let (theirs, mine) = if view.diff {
                (
                    file.theirs().unwrap_or_default(),
                    file.text(self.state.document()),
                )
            } else {
                (String::new(), String::new())
            };
            ConflictPrompt::new(view, &theirs, &mine, &self.theme).render(panes, buf);
        }
        StatusLine::new(&self.state, self.keymap.pending(), &self.theme, self.icons)
            .render(status, buf);
    }
}
