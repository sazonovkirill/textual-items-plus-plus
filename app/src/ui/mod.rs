//! The UI layer: the terminal frontend, which turns terminal events into app messages, carries out
//! the app's effects and draws its state.

mod animation;
mod attachments;
mod clipboard;
mod conflict;
mod container;
mod cut;
mod date_prompt;
mod edit_dialog;
mod edit_keys;
mod field;
mod file;
mod keymap;
mod markdown;
mod panes;
mod picker;
mod quick_add;
mod save_as;
mod search;
mod settings;
mod shortcuts;
mod sidebar;
mod status;
mod text_keys;
mod theme;
mod weather;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use jiff::civil::Date;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Style},
    widgets::{StatefulWidget, Widget},
};

pub use animation::animations_default;
pub use clipboard::SystemClipboard;
pub use file::Edge;
pub use keymap::Keymap;
pub use picker::{PickerPopup, picker_input};
pub use theme::{BuiltIn, Icon, IconSet, Theme};

use crate::{
    app::{
        self, Animations, AttachmentsInput, Command, DateInput, Destination, Direction,
        EditDialogInput, EditDialogView, EditField, EditInput, Effect, Focus, Message, Opened,
        PaneView, Part, PickerInput, Place, QuickAddInput, Report, SaveAsInput, SaveAsReport,
        SearchInput, Selected, Settings, Side, State, Stop, Symbol, TagEntry, TagMark, TagsFor,
        TagsView, TextEdit, view::View,
    },
    domain::ItemKind,
    store::{self, ConfigFile, OpenError, Outcome, RecoveryError},
};
use animation::{Animator, Before, DrawnPane, Intent};
use attachments::{AttachmentsPanel, PanelKey, Scroll, Scrolled};
use clipboard::Clipboard;
use conflict::ConflictPrompt;
use container::{ContainerPane, Top};
use date_prompt::DatePromptPanel;
use edit_dialog::EditDialogPanel;
use file::File;
use quick_add::QuickAddPanel;
use save_as::SaveAsPanel;
use settings::SettingsWindow;
use shortcuts::{Keyboard, Listing, ShortcutsPanel, Window};
use sidebar::SidebarPane;
use status::StatusLine;
use weather::{Idle, Season, Weather};

/// How many lines a turn of the mouse wheel scrolls the shortcuts panel, as terminals and
/// editors commonly scroll (D-74).
const WHEEL: usize = 3;

/// The glyph marking what has the focus: the focused row, sidebar entry or attachment (D-30 of
/// taskpaper-terminal).
const FOCUS_MARKER: &str = "▌";

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
    /// Where each pane is scrolled to, left to right.
    tops: Vec<Top>,
    /// The first pane on screen, when they do not all fit.
    first: usize,
    /// How the attachments panel is scrolled.
    panel: Scrolled,
    /// Whether the attachments panel covers the whole of the panes rather than half the screen,
    /// as `\\` toggles it while it is open; it opens at half.
    panel_full: bool,
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
    /// The local time now, which stamps comments (FR-ATT-04), the home folder and where the app
    /// keeps its own files and the disk worker and the watcher report, for the file Save As
    /// opens for a document without one (FR-STORE-11).
    edge: Edge,
    /// The frontend's clock, as last given, counted from the start of the session.
    now: Duration,
    /// The weather the app shows once it sits idle (FR-UI-08).
    weather: Weather,
    /// The effects drawn over each frame (FR-UI-07).
    animator: Animator,
}

impl Tui {
    /// The app on `state`, without a file or a settings file, its local time read from `edge`,
    /// which says where the file Save As opens keeps its own files and reports (FR-STORE-11).
    #[must_use]
    pub fn new(state: State, today: Date, edge: &Edge) -> Self {
        Self {
            tops: vec![Top::default(); state.panes().count()],
            theme: state.settings().theme.colors(),
            animator: Animator::new(state.settings().animations),
            state,
            keymap: Keymap::default(),
            config: ConfigFile::new(None),
            first: 0,
            panel: Scrolled::default(),
            panel_full: false,
            area: Rect::default(),
            quit: false,
            file: None,
            clipboard: Clipboard::default(),
            today,
            keyboard: Keyboard::default(),
            shortcuts_scroll: (0, None),
            shortcuts_page: 1,
            edge: edge.clone(),
            now: Duration::ZERO,
            weather: Weather::default(),
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
        let defaults = Settings {
            animations: edge.animations,
            ..Settings::default()
        };
        let (settings, problems) = config.load(defaults);
        let mut tui = match path {
            None => Self::new(
                State::new(store::demo::document(today), Opened::Demo, today)
                    .with_settings(defaults, settings),
                today,
                edge,
            ),
            Some(path) => {
                let (file, state) = File::open(path, recovered, today, edge)?;
                Self {
                    file: Some(file),
                    ..Self::new(state.with_settings(defaults, settings), today, edge)
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

    /// Takes in `input` on `today`, the local date, and has the next frame show what it did
    /// (FR-UI-07).
    pub fn input(&mut self, input: Input, today: Date) {
        self.today = today;
        let before = Before::of(&self.state);
        match input {
            Input::Terminal(event) => self.take(&event),
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
        self.animator.settle(&before, &self.state);
    }

    /// The frontend's clock reads `now`, counted from the start of the session, on `today`, the
    /// local date: a save that is due starts, a key sequence waiting too long ends, and the
    /// weather starts once the app has sat idle long enough, or moves on while it plays.
    pub fn tick(&mut self, now: Duration, today: Date) {
        self.today = today;
        self.now = now;
        self.animator.at(now);
        self.update(Message::Tick(now));
        self.keymap.tick(now);
        self.weather.tick(now, self.idle());
    }

    /// What arms the weather's idle timer: the `weather` setting, with the season's weather to
    /// start, none in summer, after the `weather_idle_minutes` setting (FR-UI-08).
    fn idle(&self) -> Option<Idle> {
        let settings = self.state.settings();
        let minutes = u64::from(settings.weather_idle_minutes.get());
        Season::of(self.today)
            .filter(|_| settings.weather && settings.animations == Animations::Full)
            .map(|season| Idle {
                season,
                after: Duration::from_mins(minutes),
            })
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
        [
            self.state.deadline(),
            self.keymap.deadline(),
            self.weather.deadline(self.idle()),
            self.animator.deadline(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Handles one terminal event on `today`, the local date: the keys of a bound command give
    /// it to the core, and the core's effects are carried out. While the conflict prompt is open
    /// it takes only its own keys and quitting. While the Navigator is open it takes the keys,
    /// leaving only those of global commands, such as its own, which closes it (FR-VIEW-07); the
    /// command palette (FR-NAV-06), the Move to picker (FR-OPS-04) and, in edit mode, the title,
    /// description or note being edited take them the same way (FR-EDIT-01), `Ctrl+V` pasting the
    /// clipboard's text there as a terminal paste goes in (FR-EDIT-04), and so do the date
    /// prompt (FR-DATE-02), the Save As prompt (FR-STORE-11) and the settings window (FR-SET-02). The open attachments panel takes
    /// them, scrolling itself (FR-ATT-01), leaving quitting and the commands it acts on
    /// (FR-CHK-03). The mouse wheel scrolls the shortcuts panel while it shows, and the mouse
    /// does nothing else (D-74). A key, paste, click or resize while the weather plays only ends
    /// it (FR-UI-08).
    pub fn handle(&mut self, event: &Event, today: Date) {
        self.input(Input::Terminal(event.clone()), today);
    }

    /// Takes in one terminal event, as [`handle`](Self::handle) says, ending the effects running
    /// first so that what it changes shows at once (FR-UI-07).
    fn take(&mut self, event: &Event) {
        self.animator.cut();
        if weather::is_input(event) && self.weather.input(self.now) {
            return;
        }
        if let Event::Paste(text) = event {
            self.paste(text);
            return;
        }
        if let Event::Mouse(mouse) = event {
            self.wheel(*mouse);
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
            if keymap::key_press(event) == Some(edit_keys::PASTE) {
                if let Some(text) = self.clipboard.text() {
                    self.paste(&text);
                }
                return;
            }
            match part {
                // The only titles the panel edits are its checklist items'.
                Part::Title if self.state.attachments().is_some() => {
                    edit_keys::checklist_input(event)
                }
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
                Some(PanelKey::Command(command)) => Some(Message::Command(command)),
                Some(PanelKey::AddComment) => {
                    Some(Message::Attachments(AttachmentsInput::AddComment((self
                        .edge
                        .clock)(
                    ))))
                }
                Some(PanelKey::Widen) => {
                    self.panel_full = !self.panel_full;
                    None
                }
                Some(PanelKey::Scroll(scroll)) => self
                    .panel_scroll(scroll)
                    .map(|motion| Message::Attachments(AttachmentsInput::Select(motion))),
                None => self.global(event),
            }
        } else if let Some((picker, focus)) = self.dialog_keys() {
            if picker {
                picker_input(event).map(|input| Message::EditDialog(EditDialogInput::Tags(input)))
            } else {
                edit_dialog::edit_dialog_input(event, focus, self.dialog_columns())
                    .map(Message::EditDialog)
            }
            .or_else(|| self.global(event))
        } else if self.state.date_prompt().is_some() {
            date_prompt::date_prompt_input(event)
                .map(Message::DatePrompt)
                .or_else(|| self.global(event))
        } else if self.state.save_as().is_some() {
            save_as::save_as_input(event)
                .map(Message::SaveAs)
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
        } else if self.state.quick_add().is_some() {
            quick_add::quick_add_input(event)
                .map(Message::QuickAdd)
                .or_else(|| self.global(event))
        } else {
            self.keymap.press(event).map(Message::Command)
        };
        if let Some(message) = message {
            self.update(message);
        }
    }

    /// Scrolls the shortcuts panel [`WHEEL`] lines while it shows when `mouse` turns the wheel,
    /// the only thing the mouse does (D-74).
    fn wheel(&mut self, mouse: MouseEvent) {
        let direction = match mouse.kind {
            MouseEventKind::ScrollUp => Direction::Up,
            MouseEventKind::ScrollDown => Direction::Down,
            _ => return,
        };
        if self.state.shows_shortcuts() {
            self.scroll_shortcuts(direction, WHEEL);
        }
    }

    /// Scrolls the shortcuts panel `lines` in `direction`, as far as drawing lets it go.
    fn scroll_shortcuts(&mut self, direction: Direction, lines: usize) {
        let (scroll, _) = &mut self.shortcuts_scroll;
        *scroll = match direction {
            Direction::Down => scroll.saturating_add(lines),
            Direction::Up => scroll.saturating_sub(lines),
        };
    }

    /// Scrolls the open attachments panel as `scroll` says, through what is selected while it
    /// goes past the panel's edge, and returns the motion that selects another otherwise
    /// (FR-ATT-01).
    fn panel_scroll(&mut self, scroll: Scroll) -> Option<Focus> {
        let view = self.state.attachments()?;
        let [panes, ..] = self.layout(self.area);
        let area = self.panel_area(panes);
        let shown = AttachmentsPanel::new(view, &self.theme, self.icons(), self.today)
            .full(self.panel_full);
        let selected = shown.selected_lines(area);
        let (top, motion) = scroll.from(self.panel.top, shown.page(area), selected);
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
    /// edited, the query of the open Navigator, Move to or tag picker, the open date prompt, Quick
    /// Add and the open Save As prompt take it with its line breaks as spaces, and the open edit
    /// dialog as [`paste_into_dialog`](Self::paste_into_dialog) says; otherwise the items
    /// it holds paste below the focus, or, in the checklist of the attachments panel, one item for each line below the
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
        } else if let Some((picker, focus)) = self.dialog_keys() {
            self.paste_into_dialog(text, picker, focus);
        } else if let Some(to_field) = self.line_field() {
            for c in text.chars() {
                let c = if matches!(c, '\r' | '\n') { ' ' } else { c };
                self.update(to_field(TextEdit::Insert(c)));
            }
        } else if self.in_checklist() {
            let branches = store::taskpaper::read_checklist(text);
            self.update(Message::Paste(Side::Below, branches));
        } else {
            let branches = store::taskpaper::read_branches(text);
            self.update(Message::Paste(Side::Below, branches));
        }
    }

    /// Takes in `text` pasted into the open edit dialog: its tag picker's query while it is open,
    /// else the focused field's text, a description's with its line breaks and a title's or a
    /// date's with them as spaces; the tags and Pinned take none.
    fn paste_into_dialog(&mut self, text: &str, picker: bool, focus: EditField) {
        let lines = picker || focus != EditField::Description;
        if !picker && matches!(focus, EditField::Tags | EditField::Pinned) {
            return;
        }
        for c in text.replace("\r\n", "\n").replace('\r', "\n").chars() {
            let c = if lines && c == '\n' { ' ' } else { c };
            let edit = TextEdit::Insert(c);
            self.update(Message::EditDialog(if picker {
                EditDialogInput::Tags(PickerInput::Edit(edit))
            } else {
                EditDialogInput::Edit(edit)
            }));
        }
    }

    /// How an edit of the one-line text that has the keys reaches the core, if one does: the
    /// open date prompt's, Save As prompt's or Quick Add's, which passes it to its picker while
    /// that is open, the words searched for, or the open picker's query.
    fn line_field(&self) -> Option<Box<dyn Fn(TextEdit) -> Message>> {
        if self.state.quick_add().is_some() {
            Some(Box::new(|edit| {
                Message::QuickAdd(QuickAddInput::Edit(edit))
            }))
        } else if self.state.date_prompt().is_some() {
            Some(Box::new(|edit| Message::DatePrompt(DateInput::Edit(edit))))
        } else if self.state.save_as().is_some() {
            Some(Box::new(|edit| Message::SaveAs(SaveAsInput::Edit(edit))))
        } else if self.state.search().is_some() {
            Some(Box::new(|edit| Message::Search(SearchInput::Edit(edit))))
        } else {
            let to_picker = self.open_picker()?;
            Some(Box::new(move |edit| to_picker(PickerInput::Edit(edit))))
        }
    }

    /// Gives `message` to the core and carries out its effects, keeping each pane's scroll with
    /// it as panes open and close, and opening the attachments panel at half.
    fn update(&mut self, message: Message) {
        let (panes, current) = (self.state.panes().count(), self.state.current_pane());
        let panel_open = self.state.attachments().is_some();
        let intent = Intent::of(&message);
        let effects = app::update(&mut self.state, message, self.today);
        self.animator.observe(intent, &self.state);
        self.animator.set(self.state.settings().animations);
        for effect in effects {
            self.run(effect);
        }
        self.panel_full &= panel_open;
        let after = self.state.panes().count();
        if after > panes {
            let opened = self.state.current_pane().min(self.tops.len());
            self.tops.insert(opened, Top::default());
        } else if after < panes && current < self.tops.len() {
            self.tops.remove(current);
        }
        self.tops.resize(after, Top::default());
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

    /// Whether the open edit dialog has its tag picker open, and the field it focuses, while it is
    /// open.
    fn dialog_keys(&self) -> Option<(bool, EditField)> {
        let view = self.state.edit_dialog()?;
        Some((view.picker.is_some(), view.focus))
    }

    /// The columns the description of the open edit dialog wraps in, as it is drawn.
    fn dialog_columns(&self) -> usize {
        EditDialogPanel::columns(self.area.width)
    }

    /// The edit dialog's panel showing `view`.
    fn dialog_panel<'a>(&'a self, view: &'a EditDialogView<'a>) -> EditDialogPanel<'a> {
        EditDialogPanel::new(view, &self.theme, self.icons(), self.today)
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
        } else if self.state.quick_add_picker().is_some() {
            Some(Message::QuickAddPicker)
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
    /// query of the Navigator, the command palette, Go to Symbol, Move to or Quick Add's picker
    /// while one is open, at the caret of the date prompt, the Save As prompt or the Quick Add
    /// panel while one is open, at the caret of the text being edited in edit mode, in the pane or
    /// the sidebar, else hidden; the weather
    /// covers it while it plays.
    #[must_use]
    pub fn cursor(&self, area: Rect) -> Option<Position> {
        if self.state.conflict().is_some() || self.weather.playing() {
            return None;
        }
        let [panes, prompt, _] = self.layout(area);
        if let Some(view) = self.state.attachments() {
            return AttachmentsPanel::new(view, &self.theme, self.icons(), self.today)
                .full(self.panel_full)
                .caret(self.panel_area(panes), self.panel.top);
        }
        if let Some(view) = self.state.edit_dialog() {
            return match &view.picker {
                Some(picker) => tags_popup(picker, &self.theme, self.icons()).caret(panes),
                None => self.dialog_panel(&view).caret(prompt),
            };
        }
        if let Some(view) = self.state.date_prompt() {
            return DatePromptPanel::new(&view, &self.theme).caret(prompt);
        }
        if let Some(view) = self.state.save_as() {
            return SaveAsPanel::new(&view, &self.theme).caret(prompt);
        }
        if let Some(view) = self.state.navigator() {
            return navigator_popup(&view, &self.theme, self.icons()).caret(panes);
        }
        if let Some(view) = self.state.palette() {
            return palette_popup(&view, &self.theme, self.icons()).caret(panes);
        }
        if let Some(view) = self.state.go_to_symbol() {
            return go_to_symbol_popup(&view, &self.theme, self.icons()).caret(panes);
        }
        if let Some(view) = self.state.move_to() {
            return move_to_popup(&view, &self.theme, self.icons()).caret(panes);
        }
        if let Some(view) = self.state.tags() {
            return tags_popup(&view, &self.theme, self.icons()).caret(panes);
        }
        if let Some(view) = self.state.quick_add_picker() {
            return destination_popup("Add to", &view, &self.theme, self.icons()).caret(panes);
        }
        if let Some(view) = self.state.quick_add() {
            return QuickAddPanel::new(&view, &self.theme).caret(prompt);
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
        self.current_pane().caret(self.current_area(room), top)
    }

    /// The columns the description being edited wraps in, as last drawn.
    fn description_columns(&self) -> usize {
        let [panes, _, _] = self.layout(self.area);
        let [_, room] = split(self.strip(panes), self.state.sidebar().is_some());
        self.current_pane()
            .description_columns(self.current_area(room))
            .unwrap_or(usize::MAX)
    }

    /// The pane with the keys, to draw.
    fn current_pane(&self) -> ContainerPane<'_> {
        self.pane(self.state.pane_view(), self.state.current_pane())
    }

    /// `pane`, at position `index` among the panes, to draw, numbered while there are several.
    fn pane<'a>(&'a self, pane: PaneView<'a>, index: usize) -> ContainerPane<'a> {
        let drawn = ContainerPane::new(
            &self.state,
            pane,
            index == self.state.current_pane(),
            self.today,
            &self.theme,
            self.icons(),
        );
        if self.state.panes().nth(1).is_some() {
            drawn.numbered(index + 1)
        } else {
            drawn
        }
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
        AttachmentsPanel::columns(self.panel_area(panes))
    }

    /// Where the attachments panel goes of `panes`, the area they share with the shortcuts
    /// panel: over half the screen at the right of the panes' strip, or while it is full over
    /// the whole of the panes beside the sidebar.
    fn panel_area(&self, panes: Rect) -> Rect {
        let strip = self.strip(panes);
        if self.panel_full {
            let [_, room] = split(strip, self.state.sidebar().is_some());
            room
        } else {
            AttachmentsPanel::area(strip)
        }
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
            Effect::Copy { side, ending, id } => {
                let report = self
                    .file
                    .as_mut()
                    .and_then(|file| file.copy(side, self.state.document(), ending, id));
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
            Effect::SaveAs { path, replace } => self.save_as(&path, replace),
            Effect::StartWeather => match Season::of(self.today) {
                Some(season) => self.weather.start(self.now, season),
                None => self.update(Message::NoWeather),
            },
            Effect::ScrollShortcuts(direction) => {
                self.scroll_shortcuts(direction, self.shortcuts_page);
            }
        }
    }

    /// Writes the document to the file at `path`, as the user typed it, or over the file there
    /// when told to `replace` it, and has it saved there from then on (FR-STORE-11): on the disk
    /// worker after the work asked of it so far, or at once for a document without a file, which
    /// has none.
    fn save_as(&mut self, path: &str, replace: bool) {
        let edge = &self.edge;
        let path = file::expand(path, edge.home.as_deref());
        if let Some(file) = &mut self.file {
            file.save_as(path, self.state.document(), replace);
            return;
        }
        let report = match File::create(
            path.clone(),
            self.state.document(),
            replace,
            self.today,
            edge,
        ) {
            Ok(file) => {
                self.file = Some(file);
                SaveAsReport::Saved(path)
            }
            Err(report) => report,
        };
        self.update(Message::Disk(Report::SaveAs(report)));
    }

    /// The parts of the screen: where the panes go, over the panel of the date prompt or the Save
    /// As prompt while one is open, over the status line. The panes give up the panel's rows
    /// rather than lie under it, so that the focus scrolls into view above it, as in
    /// taskpaper-terminal.
    fn layout(&self, area: Rect) -> [Rect; 3] {
        let panel = if let Some(view) = self.state.edit_dialog() {
            let room = area.height.saturating_sub(2);
            self.dialog_panel(&view).height(area.width).min(room)
        } else if self.state.date_prompt().is_some() {
            DatePromptPanel::HEIGHT
        } else if self.state.save_as().is_some() {
            SaveAsPanel::HEIGHT
        } else if self.state.quick_add().is_some() {
            QuickAddPanel::HEIGHT
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
            Some(
                if part == Part::Title && self.state.attachments().is_some() {
                    Window::EditingChecklistItem
                } else {
                    Window::Editing(part)
                },
            )
        } else if self.state.attachments().is_some() {
            Some(Window::Attachments)
        } else if let Some((picker, _)) = self.dialog_keys() {
            Some(if picker {
                Window::Picker
            } else {
                Window::EditDialog
            })
        } else if self.state.date_prompt().is_some() {
            Some(Window::DatePrompt)
        } else if self.state.save_as().is_some() {
            Some(Window::SaveAs)
        } else if self.state.settings_window().is_some() {
            Some(Window::Settings)
        } else if self.state.search().is_some() {
            Some(Window::Search)
        } else if self.open_picker().is_some() {
            Some(Window::Picker)
        } else if self.state.quick_add().is_some() {
            Some(Window::QuickAdd)
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

    /// Draws the panes placed in `room`, a dim rule apart, scrolling each to keep its focus in
    /// view.
    fn render_panes(&mut self, room: Rect, buf: &mut Buffer) {
        let placed = self.pane_areas(room);
        if let Some(&(first, _)) = placed.first() {
            self.first = first;
        }
        let current = self.state.current_pane();
        for (index, pane) in self.state.panes().enumerate() {
            let Some(&(_, area)) = placed.iter().find(|&&(placed, _)| placed == index) else {
                continue;
            };
            let mut top = self.tops.get(index).copied().unwrap_or_default();
            let drawn = self.pane(pane, index).draw(area, buf, &mut top);
            if let Some(kept) = self.tops.get_mut(index) {
                *kept = top;
            }
            self.animator.drawn(DrawnPane {
                index,
                focused: index == current
                    && !self.state.sidebar_has_keys()
                    && self.state.attachments().is_none(),
                focus: pane.selection().focused(),
                body: area,
                title: (top == Top::default()).then(|| Rect {
                    height: area.height.min(1),
                    ..area
                }),
                rows: drawn,
            });
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

    /// Whether the terminal should report the mouse: only while the shortcuts panel shows, whose
    /// wheel scrolls it, so that the terminal selects text with the mouse otherwise (D-74).
    #[must_use]
    pub fn captures_mouse(&self) -> bool {
        self.state.shows_shortcuts()
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
                file.recover(self.state.document())
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
        .color(|theme, &stop| stop_title_color(theme, stop))
        .icon_color(|theme, &stop| stop_icon_color(theme, stop))
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
        ItemKind::Area => Icon::Area,
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
            ItemKind::Area => Icon::Area,
            _ => Icon::Project,
        },
        Stop::View(view) => view_icon(view),
    }
}

/// The color of the icon of `stop` in the Navigator and the sidebar: the Inbox's and each view's
/// own, or a place's kind's, dimmed for a Later project (D-78, D-89).
fn stop_icon_color(theme: &Theme, stop: Stop) -> Color {
    match stop {
        Stop::Place(place) if place.later => theme.dim_text,
        Stop::Place(place) if place.kind == ItemKind::Inbox => theme.inbox_icon,
        Stop::Place(place) => theme.kind(place.kind),
        Stop::View(View::Today) => theme.today_icon,
        Stop::View(View::Upcoming) => theme.upcoming_icon,
        Stop::View(View::Flagged) => theme.flag,
        Stop::View(View::All) => theme.all_icon,
        Stop::View(View::Logbook) => theme.logbook_icon,
        // Search is opened by searching, never listed as a stop.
        Stop::View(View::Trash | View::Search) => theme.trash_icon,
    }
}

/// The color of the title of `stop` in the Navigator and the sidebar: an area's kind's, which
/// heads what it holds, dimmed for a Later project, and the text color for the rest (D-78,
/// D-89).
fn stop_title_color(theme: &Theme, stop: Stop) -> Color {
    match stop {
        Stop::Place(place) if place.kind == ItemKind::Area => theme.headers,
        Stop::Place(place) if place.later => theme.dim_text,
        _ => theme.text,
    }
}

/// Gives the text in `area` drawn in no color of its own `color`, the text color of the band or
/// selection behind it, so that it reads there whatever the terminal's own text color.
fn band_text(buf: &mut Buffer, area: Rect, color: Color) {
    for position in area.positions() {
        if let Some(cell) = buf.cell_mut(position)
            && cell.fg == Color::Reset
        {
            cell.set_fg(color);
        }
    }
}

/// The icon standing for `view` in its pane's heading.
fn view_icon(view: View) -> Icon {
    match view {
        View::Today => Icon::Today,
        View::Upcoming => Icon::Upcoming,
        View::Flagged => Icon::Flag,
        View::All => Icon::All,
        View::Logbook => Icon::Logbook,
        View::Trash => Icon::Trash,
        View::Search => Icon::Search,
    }
}

/// The Move to window (FR-OPS-04).
fn move_to_popup<'a>(
    view: &'a app::PickerView<'a, Destination>,
    theme: &'a Theme,
    icons: IconSet,
) -> PickerPopup<'a, Destination> {
    destination_popup("Move to", view, theme, icons)
}

/// A window titled `title` choosing a place, each after its kind's icon, in a tree: Move to's
/// and Quick Add's (FR-OPS-04, FR-EDIT-07).
fn destination_popup<'a>(
    title: &'a str,
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
    PickerPopup::new(title, view, icon, icons, theme)
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
        TagsFor::Items | TagsFor::Draft => "Tags",
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
    /// apart, beside the sidebar while it shows and the
    /// shortcuts panel at the right while it shows, over the status line, scrolling each pane to
    /// keep its focus in view, the date prompt between them while it is open, and the Navigator,
    /// the command palette, Go to Symbol, Move to, the tag picker, the settings window, the attachments panel or
    /// the conflict prompt over them while it is open, and the weather over everything while it
    /// plays.
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
        if let Some(view) = self.state.edit_dialog() {
            self.dialog_panel(&view).render(prompt, buf);
            if let Some(picker) = &view.picker {
                tags_popup(picker, &self.theme, icons).render(panes, buf);
            }
        }
        if let Some(view) = self.state.date_prompt() {
            DatePromptPanel::new(&view, &self.theme).render(prompt, buf);
        }
        if let Some(view) = self.state.save_as() {
            SaveAsPanel::new(&view, &self.theme).render(prompt, buf);
        }
        if let Some(view) = self.state.quick_add() {
            QuickAddPanel::new(&view, &self.theme).render(prompt, buf);
        }
        if let Some(view) = self.state.quick_add_picker() {
            destination_popup("Add to", &view, &self.theme, icons).render(panes, buf);
        }
        if let Some(view) = self.state.tags() {
            tags_popup(&view, &self.theme, icons).render(panes, buf);
        }
        if let Some(focused) = self.state.settings_window() {
            let settings = (self.state.settings(), self.state.default_settings());
            SettingsWindow::new(settings, focused, self.config.path(), &self.theme)
                .render(panes, buf);
        }
        match self.state.attachments() {
            Some(view) => {
                let area = self.panel_area(panes);
                AttachmentsPanel::new(view, &self.theme, icons, self.today)
                    .full(self.panel_full)
                    .render(area, buf, &mut self.panel);
            }
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
        self.animator.process(buf, &self.theme);
        self.weather.draw(buf, area, icons);
    }
}
