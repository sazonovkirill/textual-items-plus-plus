//! The UI layer: the terminal frontend, which turns terminal events into app messages, carries out
//! the app's effects and draws its state.

mod attachments;
mod clipboard;
mod conflict;
mod container;
mod crumbs;
mod date_prompt;
mod edit_keys;
mod field;
mod file;
mod keymap;
mod markdown;
mod picker;
mod sidebar;
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
    style::{Color, Style},
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
        self, Command, Context, DateInput, Destination, EditInput, Effect, Message, Opened, Part,
        PickerInput, Place, REGISTRY, Side, State, Stop, TagEntry, TagMark, TagsFor, TagsView,
        TextEdit,
        view::{Shown, View},
    },
    domain::ItemKind,
    store::{self, OpenError, Outcome, RecoveryError},
};
use attachments::{AttachmentsPanel, PanelKey};
use clipboard::Clipboard;
use conflict::ConflictPrompt;
use container::ContainerPane;
use crumbs::Crumbs;
use date_prompt::DatePromptPanel;
use file::File;
use sidebar::SidebarPane;
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
    /// The first line of the attachments panel's content on screen.
    panel_top: usize,
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
            panel_top: 0,
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
                State::new(store::demo::document(today), Opened::Demo, today),
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
    /// Move to picker (FR-OPS-04) and, in edit mode, the title, description or note being edited
    /// take them the same way (FR-EDIT-01), and so does the date prompt (FR-DATE-02). The open
    /// attachments panel takes them leaving only quitting, scrolling itself (FR-ATT-01). A key
    /// that cuts a sequence short gives the sequence's command first, then acts on its own.
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
                Part::Note => edit_keys::description_input(event, self.note_columns()),
            }
            .map(Message::Edit)
            .or_else(|| self.global(event))
        } else if self.state.attachments().is_some() {
            match attachments::panel_input(event) {
                Some(PanelKey::Input(input)) => Some(Message::Attachments(input)),
                Some(PanelKey::Scroll(scroll)) => {
                    let [panes, ..] = self.layout(self.area);
                    let page = AttachmentsPanel::page(AttachmentsPanel::area(panes));
                    self.panel_top = scroll.from(self.panel_top, page);
                    None
                }
                None => self.global(event),
            }
        } else if self.state.date_prompt().is_some() {
            date_prompt::date_prompt_input(event)
                .map(Message::DatePrompt)
                .or_else(|| self.global(event))
        } else if let Some(to_picker) = self.open_picker() {
            picker_input(event)
                .map(to_picker)
                .or_else(|| self.global(event))
        } else {
            self.keymap.press(event).map(Message::Command)
        };
        if let Some(message) = message {
            self.update(message);
        }
    }

    /// Takes in `text` pasted into the terminal (FR-OPS-05), without the line breaks that end
    /// it: a description or a note being edited takes it whole; a title being edited takes a
    /// single line, while several lines end editing and paste as items below; the query of the
    /// open Navigator, Move to or tag picker and the open date prompt take it with its line breaks
    /// as spaces; otherwise the items it holds paste below the focus. The conflict prompt takes
    /// none, nor does the attachments panel while its note is not edited.
    fn paste(&mut self, text: &str) {
        let panel = self.state.attachments().is_some() && self.state.editing().is_none();
        if self.state.conflict().is_some() || panel {
            return;
        }
        let text = text.trim_end_matches(['\r', '\n']);
        let several = text.contains(['\r', '\n']);
        let part = self.state.editing().map(|(_, part, _)| part);
        if matches!(part, Some(Part::Description | Part::Note)) {
            for c in text.replace("\r\n", "\n").replace('\r', "\n").chars() {
                self.update(Message::Edit(EditInput::Text(TextEdit::Insert(c))));
            }
        } else if part.is_some() && !several {
            for c in text.chars() {
                self.update(Message::Edit(EditInput::Text(TextEdit::Insert(c))));
            }
        } else if self.state.date_prompt().is_some() {
            for c in text.chars() {
                let c = if matches!(c, '\r' | '\n') { ' ' } else { c };
                self.update(Message::DatePrompt(DateInput::Edit(TextEdit::Insert(c))));
            }
        } else if let Some(to_picker) = self.open_picker() {
            for c in text.chars() {
                let c = if matches!(c, '\r' | '\n') { ' ' } else { c };
                self.update(to_picker(PickerInput::Edit(TextEdit::Insert(c))));
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

    /// What carries an input to the open picker, the Navigator, Move to or the tag picker, while
    /// one is open.
    fn open_picker(&self) -> Option<fn(PickerInput) -> Message> {
        if self.state.navigator().is_some() {
            Some(Message::Navigator)
        } else if self.state.move_to().is_some() {
            Some(Message::MoveTo)
        } else if self.state.tags().is_some() {
            Some(Message::Tags)
        } else {
            None
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
    /// commands and quitting, while the attachments panel is open only quitting, while the
    /// Navigator, Move to or the date prompt is open or in edit mode only global ones, and
    /// otherwise any.
    fn allows(&self, command: Command) -> bool {
        let context = REGISTRY
            .iter()
            .find(|entry| entry.command == command)
            .map(|entry| entry.context);
        if self.state.conflict().is_some() {
            command == Command::Quit || context == Some(Context::Conflict)
        } else if self.state.attachments().is_some() {
            command == Command::Quit
        } else if self.state.editing().is_some()
            || self.open_picker().is_some()
            || self.state.date_prompt().is_some()
        {
            context == Some(Context::Global)
        } else {
            true
        }
    }

    /// Where the terminal's cursor goes when the app is drawn over `area`: at the caret of the
    /// note being edited in the attachments panel while it is open, else at the caret of the
    /// query of the Navigator or Move to while one is open, at the caret of the date prompt while
    /// it is open, at the caret of the text being edited in edit mode, in the pane or the sidebar,
    /// else hidden.
    #[must_use]
    pub fn cursor(&self, area: Rect) -> Option<Position> {
        if self.state.conflict().is_some() {
            return None;
        }
        let [panes, prompt, _] = self.layout(area);
        if let Some(view) = self.state.attachments() {
            return AttachmentsPanel::new(view, &self.theme, self.icons)
                .caret(AttachmentsPanel::area(panes), self.panel_top);
        }
        if let Some(view) = self.state.date_prompt() {
            return Some(DatePromptPanel::new(&view, &self.theme).caret(prompt));
        }
        if let Some(view) = self.state.navigator() {
            return Some(navigator_popup(&view, &self.theme, self.icons).caret(panes));
        }
        if let Some(view) = self.state.move_to() {
            return Some(move_to_popup(&view, &self.theme, self.icons).caret(panes));
        }
        if let Some(view) = self.state.tags() {
            return Some(tags_popup(&view, &self.theme, self.icons).caret(panes));
        }
        let sidebar = self.state.sidebar();
        let [side, pane] = split(panes, sidebar.is_some());
        if let Some(view) = sidebar.filter(|view| view.keys) {
            return SidebarPane::new(&self.state, view, &self.theme, self.icons).caret(side);
        }
        ContainerPane::new(&self.state, self.today, &self.theme, self.icons)
            .caret(pane_layout(pane)[2], self.top)
    }

    /// The columns the description being edited wraps in, as last drawn.
    fn description_columns(&self) -> usize {
        let [panes, _, _] = self.layout(self.area);
        let [_, pane] = split(panes, self.state.sidebar().is_some());
        ContainerPane::new(&self.state, self.today, &self.theme, self.icons)
            .description_columns(pane_layout(pane)[2])
            .unwrap_or(usize::MAX)
    }

    /// The columns the note being edited wraps in, as last drawn.
    fn note_columns(&self) -> usize {
        let [panes, ..] = self.layout(self.area);
        AttachmentsPanel::columns(AttachmentsPanel::area(panes))
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

    /// The parts of the screen: where the panes go, over the date prompt's panel while it is open,
    /// over the status line. The panes give up the panel's rows rather than lie under it, so that
    /// the focus scrolls into view above it, as in taskpaper-terminal.
    fn layout(&self, area: Rect) -> [Rect; 3] {
        let panel = if self.state.date_prompt().is_some() {
            DatePromptPanel::HEIGHT
        } else {
            0
        };
        Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(panel),
            Constraint::Length(1),
        ])
        .areas(area)
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

/// The parts of the panes' area: the sidebar, empty when it does not show, and the pane.
fn split(panes: Rect, sidebar: bool) -> [Rect; 2] {
    let width = if sidebar {
        sidebar::width(panes.width)
    } else {
        0
    };
    Layout::horizontal([Constraint::Length(width), Constraint::Fill(1)]).areas(panes)
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

/// The Navigator's window, each place after its kind's icon and each view after its own, in a
/// tree with its open tasks counted at the right (FR-VIEW-07).
fn navigator_popup<'a>(
    view: &'a app::PickerView<'a, Stop>,
    theme: &'a Theme,
    icons: IconSet,
) -> PickerPopup<'a, Stop> {
    let icon: &dyn Fn(&Stop) -> Icon = &|&stop| stop_icon(stop);
    PickerPopup::new("Navigator", view, icon, icons, theme)
        // v1 nests no project in another (D-57), so no entry is indented.
        .tree(&|_| 0)
        .color(|theme, &stop| stop_color(theme, stop))
        .badge(&|stop| match stop {
            Stop::Place(Place { open_tasks: 0, .. }) | Stop::View(_) => String::new(),
            Stop::Place(place) => place.open_tasks.to_string(),
        })
}

/// The icon of `stop` in the Navigator and the sidebar: a place's kind's, or a view's own.
fn stop_icon(stop: Stop) -> Icon {
    match stop {
        Stop::Place(place) => match place.kind {
            ItemKind::Inbox => Icon::Inbox,
            ItemKind::Area => Icon::Area,
            _ => Icon::Project,
        },
        Stop::View(view) => view_icon(view),
    }
}

/// The color of `stop` in the Navigator and the sidebar: a place's kind's, or for a view the
/// color its pane's heading has.
fn stop_color(theme: &Theme, stop: Stop) -> Color {
    match stop {
        Stop::Place(place) => theme.kind(place.kind),
        Stop::View(_) => theme.primary,
    }
}

/// The icon standing for `view` in its pane's heading and crumbs.
fn view_icon(view: View) -> Icon {
    match view {
        View::Today => Icon::Today,
        View::Upcoming => Icon::Upcoming,
        View::All => Icon::All,
        View::Logbook => Icon::Logbook,
        View::Trash => Icon::Trash,
    }
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
    PickerPopup::new("Move to", view, icon, icons, theme)
        .tree(&|destination| match destination {
            Destination::Home => 0,
            Destination::Container { depth, .. } => *depth,
        })
        .color(|theme, destination| match destination {
            Destination::Home => theme.text,
            Destination::Container { kind, .. } => theme.kind(*kind),
        })
}

/// The tag picker's window, each tag after the tag icon, marked at the right when every item it
/// tags carries it or the filter has it, when some do, or when it is new (FR-TAG-02, FR-TAG-04).
fn tags_popup<'a>(
    view: &'a TagsView<'a>,
    theme: &'a Theme,
    icons: IconSet,
) -> PickerPopup<'a, TagEntry> {
    let title = match view.tags_for {
        TagsFor::Items => "Tags",
        TagsFor::Filter => "Filter by tags",
    };
    PickerPopup::new(title, &view.picker, &|_| Icon::Tag, icons, theme).badge(&|entry| {
        match entry.mark {
            TagMark::Off => "",
            TagMark::Some => "–",
            TagMark::All => "✓",
            TagMark::New => "new tag",
        }
        .to_owned()
    })
}

impl Widget for &mut Tui {
    /// Draws the pane, below its breadcrumbs and a rule, beside the sidebar while it shows, over
    /// the status line, scrolling the pane to keep the focus in view, the date prompt between
    /// them while it is open, and the Navigator, Move to, the attachments panel or the conflict
    /// prompt over them while it is open.
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = area;
        let [panes, prompt, status] = self.layout(area);
        let sidebar = self.state.sidebar();
        let [side, pane] = split(panes, sidebar.is_some());
        if let Some(view) = sidebar {
            SidebarPane::new(&self.state, view, &self.theme, self.icons).render(side, buf);
        }
        let [crumbs, rule, pane] = pane_layout(pane);
        let path = self.state.path();
        let view = match self.state.shown() {
            Shown::View(view) => Some(view),
            Shown::Container(_) => None,
        };
        Crumbs::new(self.state.document(), &path, view, &self.theme, self.icons)
            .filter(self.state.tag_filter())
            .render(crumbs, buf);
        Line::styled(
            "─".repeat(usize::from(rule.width)),
            Style::new().fg(self.theme.rule),
        )
        .render(rule, buf);
        ContainerPane::new(&self.state, self.today, &self.theme, self.icons).render(
            pane,
            buf,
            &mut self.top,
        );
        if let Some(view) = self.state.navigator() {
            navigator_popup(&view, &self.theme, self.icons).render(panes, buf);
        }
        if let Some(view) = self.state.move_to() {
            move_to_popup(&view, &self.theme, self.icons).render(panes, buf);
        }
        if let Some(view) = self.state.date_prompt() {
            DatePromptPanel::new(&view, &self.theme).render(prompt, buf);
        }
        if let Some(view) = self.state.tags() {
            tags_popup(&view, &self.theme, self.icons).render(panes, buf);
        }
        match self.state.attachments() {
            Some(view) => AttachmentsPanel::new(view, &self.theme, self.icons).render(
                panes,
                buf,
                &mut self.panel_top,
            ),
            None => self.panel_top = 0,
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
