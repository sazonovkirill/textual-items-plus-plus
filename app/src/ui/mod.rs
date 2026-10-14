//! The UI layer: the terminal frontend, which turns terminal events into app messages, carries out
//! the app's effects and draws its state.

mod attachments;
mod clipboard;
mod conflict;
mod container;
mod crumbs;
mod cut;
mod date_prompt;
mod edit_keys;
mod field;
mod file;
mod keymap;
mod markdown;
mod panes;
mod picker;
mod search;
mod settings;
mod shortcuts;
mod sidebar;
mod status;
mod text_keys;
mod theme;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crossterm::event::{Event, KeyModifiers};
use jiff::civil::{Date, DateTime};
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
        self, AttachmentsInput, Command, DateInput, Destination, Direction, EditInput, Effect,
        Focus, Message, Opened, Part, PickerInput, Place, SearchInput, Selected, Settings, Side,
        State, Stop, Symbol, TagEntry, TagMark, TagsFor, TagsView, TextEdit,
        view::{Shown, View},
    },
    domain::ItemKind,
    store::{self, ConfigFile, OpenError, Outcome, RecoveryError},
};
use attachments::{AttachmentsPanel, PanelKey, Scroll, Scrolled};
use clipboard::Clipboard;
use conflict::ConflictPrompt;
use container::ContainerPane;
use crumbs::Crumbs;
use date_prompt::DatePromptPanel;
use file::File;
use settings::SettingsWindow;
use shortcuts::{Keyboard, Listing, ShortcutsPanel, Window};
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
    /// The colors of the theme the settings choose, kept in step with them as the core changes.
    theme: Theme,
    /// The settings file, where a setting changed in the app is kept (FR-SET-04).
    config: ConfigFile,
    /// The first line of each pane's content on screen, left to right.
    tops: Vec<usize>,
    /// The first pane on screen, when they do not all fit.
    first: usize,
    /// How the attachments panel is scrolled.
    panel: Scrolled,
    /// Where the app was last drawn.
    area: Rect,
    quit: bool,
    file: Option<File>,
    clipboard: Clipboard,
    /// The local date, as last given.
    today: Date,
    /// What the terminal has shown of its keyboard, which the shortcuts panel notes.
    keyboard: Keyboard,
    /// The first line of the shortcuts panel's list on screen, and the window whose keys it
    /// lists, which a change scrolls back to the top (FR-KEY-03).
    shortcuts_scroll: (usize, Option<Window>),
    /// How many lines a page of the shortcuts panel was at the last draw, which its keys scroll.
    shortcuts_page: usize,
    /// The local time now, which stamps comments (FR-ATT-04).
    clock: fn() -> DateTime,
}

impl Tui {
    /// The app on `state`, without a file or a settings file, its local time read from `clock`.
    #[must_use]
    pub fn new(state: State, today: Date, clock: fn() -> DateTime) -> Self {
        Self {
            tops: vec![0; state.panes().count()],
            theme: state.settings().theme.colors(),
            state,
            keymap: Keymap::default(),
            config: ConfigFile::new(None),
            first: 0,
            panel: Scrolled::default(),
            area: Rect::default(),
            quit: false,
            file: None,
            clipboard: Clipboard::default(),
            today,
            keyboard: Keyboard::default(),
            shortcuts_scroll: (0, None),
            shortcuts_page: 1,
            clock,
        }
    }

    /// Notes in the shortcuts panel whether the kitty keyboard protocol is on (FR-KEY-03).
    pub fn use_kitty(&mut self, kitty: bool) {
        self.keyboard.kitty = kitty;
    }

    /// Copies to and pastes from `system` from now on, keeping only the app's own copy without
    /// it (FR-OPS-05).
    pub fn use_clipboard(&mut self, system: Box<dyn SystemClipboard>) {
        self.clipboard = Clipboard::new(system);
    }

    /// The app on the file at `path`, every change saved to it, with the document of the
    /// recovery file `recovered` when one is given, or without a file on the demo document for
    /// `today`, the local date, never saved (FR-STORE-01, FR-STORE-09, D-29). It runs with the
    /// settings of the settings file, the status line saying what of the file it ignored
    /// (FR-SET-01, FR-SET-04). `edge` says where the app's own files go and where the disk worker
    /// and the watcher report.
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
        let config = ConfigFile::new(edge.config.clone());
        let (settings, problems) = config.load::<Settings>();
        let mut tui = match path {
            None => Self::new(
                State::new(store::demo::document(today), Opened::Demo, today)
                    .with_settings(settings),
                today,
                edge.clock,
            ),
            Some(path) => {
                let (file, state) = File::open(path, recovered, today, edge)?;
                Self {
                    file: Some(file),
                    ..Self::new(state.with_settings(settings), today, edge.clock)
                }
            }
        };
        tui.config = config;
        if !problems.is_empty() {
            let problems: Vec<String> = problems.iter().map(ToString::to_string).collect();
            tui.update(Message::SettingsFailed(problems.join("; ")));
        }
        Ok(tui)
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
    /// local date: a save that is due starts, and a key sequence waiting too long ends.
    pub fn tick(&mut self, now: Duration, today: Date) {
        self.today = today;
        self.update(Message::Tick(now));
        self.keymap.tick(now);
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
    /// command palette (FR-NAV-06), the Move to picker (FR-OPS-04) and, in edit mode, the title,
    /// description or note being edited take them the same way (FR-EDIT-01), and so do the date
    /// prompt (FR-DATE-02) and the settings window (FR-SET-02). The open attachments panel takes
    /// them, scrolling itself (FR-ATT-01), leaving quitting and the commands it acts on
    /// (FR-CHK-03).
    pub fn handle(&mut self, event: &Event, today: Date) {
        self.today = today;
        if let Event::Paste(text) = event {
            self.paste(text);
            return;
        }
        if keymap::key_press(event)
            .is_some_and(|(modifiers, _)| modifiers.contains(KeyModifiers::ALT))
        {
            self.keyboard.alt_seen = true;
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
                Part::Comment(_) => edit_keys::comment_input(event),
            }
            .map(Message::Edit)
            .or_else(|| self.global(event))
        } else if self.state.attachments().is_some() {
            match attachments::panel_input(event) {
                Some(PanelKey::Input(input)) => Some(Message::Attachments(input)),
                Some(PanelKey::AddComment) => {
                    Some(Message::Attachments(AttachmentsInput::AddComment((self
                        .clock)(
                    ))))
                }
                Some(PanelKey::Scroll(scroll)) => self
                    .panel_scroll(scroll)
                    .map(|motion| Message::Attachments(AttachmentsInput::Select(motion))),
                None => self.global(event),
            }
        } else if self.state.date_prompt().is_some() {
            date_prompt::date_prompt_input(event)
                .map(Message::DatePrompt)
                .or_else(|| self.global(event))
        } else if self.state.settings_window().is_some() {
            settings::settings_input(event)
                .map(Message::Settings)
                .or_else(|| self.global(event))
        } else if self.state.search().is_some() {
            search::search_input(event)
                .map(Message::Search)
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

    /// Scrolls the open attachments panel as `scroll` says, through what is selected while it
    /// goes past the panel's edge, and returns the motion that selects another otherwise
    /// (FR-ATT-01).
    fn panel_scroll(&mut self, scroll: Scroll) -> Option<Focus> {
        let view = self.state.attachments()?;
        let [panes, ..] = self.layout(self.area);
        let area = AttachmentsPanel::area(self.strip(panes));
        let selected = AttachmentsPanel::new(view, &self.theme, self.icons()).selected_lines(area);
        let (top, motion) = scroll.from(self.panel.top, AttachmentsPanel::page(area), selected);
        self.panel.top = top;
        motion
    }

    /// Whether the open attachments panel has an entry of a checklist selected, where pastes go.
    fn in_checklist(&self) -> bool {
        self.state
            .attachments()
            .is_some_and(|view| matches!(view.selected, Selected::Checklist(_)))
    }

    /// Takes in `text` pasted into the terminal (FR-OPS-05), without the line breaks that end
    /// it: a description or a note being edited takes it whole; a title being edited takes a
    /// single line, while several lines end editing and paste as items below; a comment being
    /// edited, the query of the open Navigator, Move to or tag picker and the open date prompt
    /// take it with its line breaks as spaces; otherwise the items it holds paste below the
    /// focus, or, in the checklist of the attachments panel, one item for each line below the
    /// item selected (FR-CHK-03). The conflict prompt and the settings window take none, nor does
    /// the attachments panel while nothing in it is edited and no checklist entry is selected.
    fn paste(&mut self, text: &str) {
        let panel = self.state.attachments().is_some() && self.state.editing().is_none();
        let window = self.state.conflict().is_some() || self.state.settings_window().is_some();
        if window || (panel && !self.in_checklist()) {
            return;
        }
        let text = text.trim_end_matches(['\r', '\n']);
        let several = text.contains(['\r', '\n']);
        let part = self.state.editing().map(|(_, part, _)| part);
        if matches!(part, Some(Part::Description | Part::Note)) {
            for c in text.replace("\r\n", "\n").replace('\r', "\n").chars() {
                self.update(Message::Edit(EditInput::Text(TextEdit::Insert(c))));
            }
        } else if matches!(part, Some(Part::Comment(_))) {
            for c in text.replace("\r\n", "\n").chars() {
                let c = if matches!(c, '\r' | '\n') { ' ' } else { c };
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
        } else if self.state.search().is_some() {
            for c in text.chars() {
                let c = if matches!(c, '\r' | '\n') { ' ' } else { c };
                self.update(Message::Search(SearchInput::Edit(TextEdit::Insert(c))));
            }
        } else if let Some(to_picker) = self.open_picker() {
            for c in text.chars() {
                let c = if matches!(c, '\r' | '\n') { ' ' } else { c };
                self.update(to_picker(PickerInput::Edit(TextEdit::Insert(c))));
            }
        } else if self.in_checklist() {
            let branches = store::taskpaper::read_checklist(text);
            self.update(Message::Paste(Side::Below, branches));
        } else {
            let branches = store::taskpaper::read_branches(text);
            self.update(Message::Paste(Side::Below, branches));
        }
    }

    /// Gives `message` to the core and carries out its effects, keeping each pane's scroll with
    /// it as panes open and close.
    fn update(&mut self, message: Message) {
        let (panes, current) = (self.state.panes().count(), self.state.current_pane());
        for effect in app::update(&mut self.state, message, self.today) {
            self.run(effect);
        }
        let after = self.state.panes().count();
        if after > panes {
            let opened = self.state.current_pane().min(self.tops.len());
            self.tops.insert(opened, 0);
        } else if after < panes && current < self.tops.len() {
            self.tops.remove(current);
        }
        self.tops.resize(after, 0);
        self.theme = self.state.settings().theme.colors();
    }

    /// The icons the settings choose (FR-UI-04).
    fn icons(&self) -> IconSet {
        self.state.settings().icons
    }

    /// The width the pane with the keys keeps, `wider` or narrower by a step than its size
    /// among the panes, which the last pane on screen may draw wider, no narrower than a pane is
    /// drawn and no wider than the room beside the sidebar (FR-PANE-02).
    fn resized(&self, wider: bool) -> u16 {
        let [panes, ..] = self.layout(self.area);
        let [_, room] = split(self.strip(panes), self.state.sidebar().is_some());
        let widths: Vec<_> = self.state.panes().map(|pane| pane.width()).collect();
        let width = panes::sizes(room.width, &widths)
            .get(self.state.current_pane())
            .copied()
            .unwrap_or(room.width);
        if wider {
            width.saturating_add(panes::STEP).min(room.width)
        } else {
            width.saturating_sub(panes::STEP).max(panes::MIN_WIDTH)
        }
    }

    /// What carries an input to the open picker, the Navigator, the command palette, Go to
    /// Symbol, Move to or the tag picker, while one is open.
    fn open_picker(&self) -> Option<fn(PickerInput) -> Message> {
        if self.state.navigator().is_some() {
            Some(Message::Navigator)
        } else if self.state.palette().is_some() {
            Some(Message::Palette)
        } else if self.state.go_to_symbol().is_some() {
            Some(Message::GoToSymbol)
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

    /// Whether the app takes `command` now.
    fn allows(&self, command: Command) -> bool {
        self.state.takes(command)
    }

    /// Where the terminal's cursor goes when the app is drawn over `area`: at the caret of the
    /// note being edited in the attachments panel while it is open, else at the caret of the
    /// query of the Navigator, the command palette, Go to Symbol or Move to while one is open, at the caret of
    /// the date prompt while it is open, at the caret of the text being edited in edit mode, in
    /// the pane or the sidebar, else hidden.
    #[must_use]
    pub fn cursor(&self, area: Rect) -> Option<Position> {
        if self.state.conflict().is_some() {
            return None;
        }
        let [panes, prompt, _] = self.layout(area);
        if let Some(view) = self.state.attachments() {
            return AttachmentsPanel::new(view, &self.theme, self.icons())
                .caret(AttachmentsPanel::area(self.strip(panes)), self.panel.top);
        }
        if let Some(view) = self.state.date_prompt() {
            return Some(DatePromptPanel::new(&view, &self.theme).caret(prompt));
        }
        if let Some(view) = self.state.navigator() {
            return Some(navigator_popup(&view, &self.theme, self.icons()).caret(panes));
        }
        if let Some(view) = self.state.palette() {
            return Some(palette_popup(&view, &self.theme, self.icons()).caret(panes));
        }
        if let Some(view) = self.state.go_to_symbol() {
            return Some(go_to_symbol_popup(&view, &self.theme, self.icons()).caret(panes));
        }
        if let Some(view) = self.state.move_to() {
            return Some(move_to_popup(&view, &self.theme, self.icons()).caret(panes));
        }
        if let Some(view) = self.state.tags() {
            return Some(tags_popup(&view, &self.theme, self.icons()).caret(panes));
        }
        let sidebar = self.state.sidebar();
        let [side, room] = split(self.strip(panes), sidebar.is_some());
        if let Some(view) = sidebar.filter(|view| view.keys) {
            return SidebarPane::new(&self.state, view, &self.theme, self.icons()).caret(side);
        }
        let top = self
            .tops
            .get(self.state.current_pane())
            .copied()
            .unwrap_or_default();
        self.current_pane()
            .caret(pane_layout(self.current_area(room))[2], top)
    }

    /// The columns the description being edited wraps in, as last drawn.
    fn description_columns(&self) -> usize {
        let [panes, _, _] = self.layout(self.area);
        let [_, room] = split(self.strip(panes), self.state.sidebar().is_some());
        self.current_pane()
            .description_columns(pane_layout(self.current_area(room))[2])
            .unwrap_or(usize::MAX)
    }

    /// The pane with the keys, to draw.
    fn current_pane(&self) -> ContainerPane<'_> {
        let pane = self.state.pane_view();
        ContainerPane::new(
            &self.state,
            pane,
            true,
            self.today,
            &self.theme,
            self.icons(),
        )
    }

    /// Where each pane on screen is drawn over `room`, the panes' room beside the sidebar, with
    /// its position among the panes.
    fn pane_areas(&self, room: Rect) -> Vec<(usize, Rect)> {
        let widths: Vec<_> = self.state.panes().map(|pane| pane.width()).collect();
        panes::strip(room, &widths, self.state.current_pane(), self.first)
    }

    /// Where the pane with the keys is drawn over `room`, the panes' room beside the sidebar.
    fn current_area(&self, room: Rect) -> Rect {
        let current = self.state.current_pane();
        self.pane_areas(room)
            .into_iter()
            .find(|&(index, _)| index == current)
            .map_or(room, |(_, area)| area)
    }

    /// The columns the note being edited wraps in, as last drawn.
    fn note_columns(&self) -> usize {
        let [panes, ..] = self.layout(self.area);
        AttachmentsPanel::columns(AttachmentsPanel::area(self.strip(panes)))
    }

    /// Carries out one effect of the core.
    fn run(&mut self, effect: Effect) {
        match effect {
            Effect::Quit => {
                if let Some(file) = &self.file {
                    file.keep_layout(&self.state.layout());
                }
                self.quit = true;
            }
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
                let branches = self.clipboard.branches(self.in_checklist());
                self.update(Message::Paste(side, branches));
            }
            Effect::NarrowPane => self.update(Message::PaneWidth(self.resized(false))),
            Effect::WidenPane => self.update(Message::PaneWidth(self.resized(true))),
            Effect::KeepSetting(setting) => {
                if let Err(error) = self.config.write(setting.key(), &self.state.settings()) {
                    let key = setting.key();
                    self.update(Message::SettingsFailed(format!(
                        "`{key}` not saved: {error}"
                    )));
                }
            }
            Effect::ScrollShortcuts(direction) => {
                let (scroll, _) = &mut self.shortcuts_scroll;
                *scroll = match direction {
                    Direction::Down => scroll.saturating_add(self.shortcuts_page),
                    Direction::Up => scroll.saturating_sub(self.shortcuts_page),
                };
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

    /// Where the panes go of `panes`, the area they share with the shortcuts panel: left of it
    /// while it shows, or all of it on a screen under 80 columns, which draws the panel over the
    /// pane, as taskpaper-terminal does.
    fn strip(&self, panes: Rect) -> Rect {
        match self.shortcuts_area(panes) {
            Some(shortcuts) if panes.width >= 80 => Rect {
                width: panes.width - shortcuts.width,
                ..panes
            },
            _ => panes,
        }
    }

    /// Where the shortcuts panel goes of `panes`, at their right, while it shows and the screen
    /// has room for it.
    fn shortcuts_area(&self, panes: Rect) -> Option<Rect> {
        if !self.state.shows_shortcuts() || panes.width < 40 || panes.height < 9 {
            return None;
        }
        let [_, shortcuts] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(shortcuts::width(panes.width)),
        ])
        .areas(panes);
        Some(shortcuts)
    }

    /// The window that has the keys, when one does that binds keys of its own.
    fn window(&self) -> Option<Window> {
        if let Some((_, part, _)) = self.state.editing() {
            Some(Window::Editing(part))
        } else if self.state.attachments().is_some() {
            Some(Window::Attachments)
        } else if self.state.date_prompt().is_some() {
            Some(Window::DatePrompt)
        } else if self.state.settings_window().is_some() {
            Some(Window::Settings)
        } else if self.state.search().is_some() {
            Some(Window::Search)
        } else if self.open_picker().is_some() {
            Some(Window::Picker)
        } else {
            None
        }
    }

    /// What the shortcuts panel lists while it shows: the keys of the window that has them, if
    /// any, then each group of the commands the app takes now with how their keys read, leaving
    /// out commands no key gives (FR-KEY-03).
    fn shortcuts(&self) -> Option<Vec<Listing>> {
        let groups = self.state.shortcuts()?;
        let window = self
            .window()
            .map(shortcuts::window_keys)
            .unwrap_or_default();
        let commands = groups.into_iter().filter_map(|(group, entries)| {
            let keys: Vec<_> = entries
                .into_iter()
                .map(|entry| (entry.title.to_owned(), self.keymap.labels(entry.command)))
                .filter(|(_, keys)| !keys.is_empty())
                .collect();
            (!keys.is_empty()).then_some((group.name(), keys))
        });
        Some(window.into_iter().chain(commands).collect())
    }

    /// Draws the panes placed in `room`, each below its breadcrumbs and a rule, a dim rule apart,
    /// scrolling each to keep its focus in view.
    fn render_panes(&mut self, room: Rect, buf: &mut Buffer) {
        let icons = self.icons();
        let placed = self.pane_areas(room);
        if let Some(&(first, _)) = placed.first() {
            self.first = first;
        }
        let views: Vec<_> = self.state.panes().collect();
        let current = self.state.current_pane();
        for (index, (&pane, top)) in views.iter().zip(&mut self.tops).enumerate() {
            let Some(&(_, area)) = placed.iter().find(|&&(placed, _)| placed == index) else {
                continue;
            };
            let [crumbs, rule, rows] = pane_layout(area);
            let path = pane.path();
            let view = match pane.shown() {
                Shown::View(view) => Some(view),
                Shown::Container(_) => None,
            };
            let mut header = Crumbs::new(self.state.document(), &path, view, &self.theme, icons)
                .filter(pane.tag_filter());
            if views.len() > 1 {
                header = header.number(index + 1);
            }
            if index != current {
                header = header.dimmed();
            }
            header.render(crumbs, buf);
            Line::styled(
                "─".repeat(usize::from(rule.width)),
                Style::new().fg(self.theme.rule),
            )
            .render(rule, buf);
            ContainerPane::new(
                &self.state,
                pane,
                index == current,
                self.today,
                &self.theme,
                icons,
            )
            .render(rows, buf, top);
            if area.right() < room.right() {
                for y in area.top()..area.bottom() {
                    if let Some(cell) = buf.cell_mut((area.right(), y)) {
                        cell.set_symbol("│").set_fg(self.theme.rule);
                    }
                }
            }
        }
    }

    /// Draws the shortcuts panel in `area` while it shows, scrolled no further than its end and
    /// naming the keys that scroll it while they do; a panel that is hidden, or lists another
    /// window's keys, opens at the top.
    fn render_shortcuts(&mut self, area: Option<Rect>, buf: &mut Buffer) {
        let window = self.window();
        let (Some(groups), Some(area)) = (self.shortcuts(), area) else {
            self.shortcuts_scroll = (0, window);
            return;
        };
        if self.shortcuts_scroll.1 != window {
            self.shortcuts_scroll = (0, window);
        }
        let scrolls = [Direction::Up, Direction::Down].map(|direction| {
            let command = Command::ScrollShortcuts(direction);
            if self.allows(command) {
                self.keymap.labels(command)
            } else {
                String::new()
            }
        });
        let [up, down] = &scrolls;
        let panel = ShortcutsPanel::new(&groups, self.keyboard, &self.theme, [up, down]);
        let (last, page) = panel.scroll_range(area);
        self.shortcuts_scroll.0 = self.shortcuts_scroll.0.min(last);
        self.shortcuts_page = page;
        panel.scrolled(self.shortcuts_scroll.0).render(area, buf);
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
        .tree(&|stop| stop.depth())
        .color(|theme, &stop| stop_color(theme, stop))
        .badge(&|stop| match stop {
            Stop::Place(Place { open_tasks: 0, .. }) | Stop::View(_) => String::new(),
            Stop::Place(place) => place.open_tasks.to_string(),
        })
}

/// The command palette's window, each command after the command icon; drawn, it shows each
/// command's keys as its badge (FR-NAV-06).
fn palette_popup<'a>(
    view: &'a app::PickerView<'a, Command>,
    theme: &'a Theme,
    icons: IconSet,
) -> PickerPopup<'a, Command> {
    PickerPopup::new("Command Palette", view, &|_| Icon::Command, icons, theme)
}

/// Go to Symbol's window, each item after its kind's icon in its kind's color, the titles above
/// it dimmed after it (FR-NAV-05).
fn go_to_symbol_popup<'a>(
    view: &'a app::PickerView<'a, Symbol>,
    theme: &'a Theme,
    icons: IconSet,
) -> PickerPopup<'a, Symbol> {
    let icon: &dyn Fn(&Symbol) -> Icon = &|symbol| match symbol.kind {
        ItemKind::Project => Icon::Project,
        ItemKind::Section => Icon::Section,
        _ => Icon::Task,
    };
    PickerPopup::new("Go to Symbol", view, icon, icons, theme)
        .color(|theme, symbol| theme.kind(symbol.kind))
}

/// The icon of `stop` in the Navigator and the sidebar: a place's kind's, or a view's own.
fn stop_icon(stop: Stop) -> Icon {
    match stop {
        Stop::Place(place) => match place.kind {
            ItemKind::Inbox => Icon::Inbox,
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
        View::Search => Icon::Search,
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
    /// Draws, on the theme's background in its text color, the panes side by side, a dim rule
    /// apart, each below its breadcrumbs and a rule, beside the sidebar while it shows and the
    /// shortcuts panel at the right while it shows, over the status line, scrolling each pane to
    /// keep its focus in view, the date prompt between them while it is open, and the Navigator,
    /// the command palette, Go to Symbol, Move to, the tag picker, the settings window, the attachments panel or
    /// the conflict prompt over them while it is open.
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = area;
        let icons = self.icons();
        buf.set_style(
            area,
            Style::new().bg(self.theme.background).fg(self.theme.text),
        );
        let [panes, prompt, status] = self.layout(area);
        let sidebar = self.state.sidebar();
        let [side, room] = split(self.strip(panes), sidebar.is_some());
        if let Some(view) = sidebar {
            SidebarPane::new(&self.state, view, &self.theme, icons).render(side, buf);
        }
        self.render_panes(room, buf);
        let shortcuts = self.shortcuts_area(panes);
        self.render_shortcuts(shortcuts, buf);
        if let Some(view) = self.state.navigator() {
            navigator_popup(&view, &self.theme, icons).render(panes, buf);
        }
        if let Some(view) = self.state.palette() {
            let keys = |&command: &Command| self.keymap.labels(command);
            palette_popup(&view, &self.theme, icons)
                .badge(&keys)
                .render(panes, buf);
        }
        if let Some(view) = self.state.go_to_symbol() {
            go_to_symbol_popup(&view, &self.theme, icons).render(panes, buf);
        }
        if let Some(view) = self.state.move_to() {
            move_to_popup(&view, &self.theme, icons).render(panes, buf);
        }
        if let Some(view) = self.state.date_prompt() {
            DatePromptPanel::new(&view, &self.theme).render(prompt, buf);
        }
        if let Some(view) = self.state.tags() {
            tags_popup(&view, &self.theme, icons).render(panes, buf);
        }
        if let Some(focused) = self.state.settings_window() {
            let settings = self.state.settings();
            SettingsWindow::new(settings, focused, self.config.path(), &self.theme)
                .render(panes, buf);
        }
        match self.state.attachments() {
            Some(view) => AttachmentsPanel::new(view, &self.theme, icons).render(
                self.strip(panes),
                buf,
                &mut self.panel,
            ),
            None => self.panel = Scrolled::default(),
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
        StatusLine::new(&self.state, self.keymap.pending(), &self.theme, icons).render(status, buf);
    }
}
