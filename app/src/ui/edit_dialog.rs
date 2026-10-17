//! The edit dialog in the terminal (FR-EDIT-08): the keys that drive it and its panel, across the
//! screen just above the status line, where the date prompt goes, as taskpaper-terminal draws its
//! Edit Node dialog
//! ([editing.md §7](../../../specs/reference/taskpaper-terminal/specs/editing.md#7-edit-node-dialog-fr-mode-09)):
//! its border and title in the accent color, a row for each field, its label dimmed in a column
//! of its own and the focused one marked, and a dimmed hint naming the focused field's keys.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use jiff::civil::Date;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Widget},
};

use super::{
    FOCUS_MARKER, Icon, IconSet, Theme, date_prompt,
    edit_keys::DESCRIPTION_MOTIONS,
    field,
    keymap::{self, Key},
    picker, text_keys,
};
use crate::{
    app::{
        DatePromptView, EditDialogInput, EditDialogView, EditField, EditProblem, PickerInput,
        TextEdit, TextField,
    },
    domain::{
        ItemKind,
        dates::{self, DateKind},
    },
};

/// The keys of the open dialog besides the text keys; in the description `↑`, `↓` and `Enter`
/// are its own (D-100).
pub(super) const KEYS: &[(Key, EditDialogInput)] = &[
    ((KeyModifiers::NONE, KeyCode::Tab), EditDialogInput::Next),
    (
        (KeyModifiers::SHIFT, KeyCode::BackTab),
        EditDialogInput::Previous,
    ),
    ((KeyModifiers::NONE, KeyCode::Down), EditDialogInput::Next),
    ((KeyModifiers::NONE, KeyCode::Up), EditDialogInput::Previous),
    (
        (KeyModifiers::NONE, KeyCode::Enter),
        EditDialogInput::Confirm,
    ),
    ((KeyModifiers::NONE, KeyCode::Esc), EditDialogInput::Close),
];

/// The keys of the tag picker open over the dialog that differ from a picker's: `Space` puts the
/// focused tag on or takes it off, which a tag's name can't hold anyway, and `Enter` closes it
/// (D-148).
pub(super) const TAG_KEYS: &[(Key, PickerInput)] = &[
    (
        (KeyModifiers::NONE, KeyCode::Char(' ')),
        PickerInput::Choose,
    ),
    ((KeyModifiers::NONE, KeyCode::Enter), PickerInput::Dismiss),
];

/// What `event` does in the tag picker open over the dialog: a key of [`TAG_KEYS`], else what it
/// does in a picker, but for opening in a new pane, which the dialog has no use for. Only key
/// presses count.
pub(super) fn tags_input(event: &Event) -> Option<PickerInput> {
    let key = keymap::key_press(event)?;
    TAG_KEYS
        .iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
        .or_else(|| picker::picker_input(event))
        .filter(|&input| input != PickerInput::ChooseInNewPane)
}

/// What `event` does in the open dialog with `focus` focused, the description wrapping in
/// `width` columns: in the description the keys that move by lines move its caret and `Enter`
/// breaks the line; otherwise the dialog's key acts, and the text keys edit the field. Only key
/// presses count.
pub(super) fn edit_dialog_input(
    event: &Event,
    focus: EditField,
    width: usize,
) -> Option<EditDialogInput> {
    let (modifiers, code) = keymap::key_press(event)?;
    if focus == EditField::Description {
        let moved = (modifiers.difference(KeyModifiers::SHIFT), code);
        if let Some((_, motion)) = DESCRIPTION_MOTIONS.iter().find(|(key, _)| *key == moved) {
            return Some(EditDialogInput::Edit(TextEdit::Move {
                motion: motion(width),
                select: modifiers.contains(KeyModifiers::SHIFT),
            }));
        }
        if (modifiers, code) == (KeyModifiers::NONE, KeyCode::Enter) {
            return Some(EditDialogInput::Edit(TextEdit::Insert('\n')));
        }
    }
    KEYS.iter()
        .find(|(key, _)| *key == (modifiers, code))
        .map(|&(_, input)| input)
        .or_else(|| text_keys::text_edit((modifiers, code)).map(EditDialogInput::Edit))
}

/// The columns before a field's value: the focus marker, a space, the label and two spaces.
const LABEL: u16 = 15;

/// The most lines the description takes; a longer one scrolls to keep its caret in view.
const DESCRIPTION_LINES: usize = 6;

/// The open dialog's panel.
pub(super) struct EditDialogPanel<'a> {
    view: &'a EditDialogView<'a>,
    theme: &'a Theme,
    icons: IconSet,
    today: Date,
}

impl<'a> EditDialogPanel<'a> {
    /// The panel showing `view` in `theme` with `icons`, naming dates relative to `today`.
    pub(super) fn new(
        view: &'a EditDialogView<'a>,
        theme: &'a Theme,
        icons: IconSet,
        today: Date,
    ) -> Self {
        Self {
            view,
            theme,
            icons,
            today,
        }
    }

    /// The columns a field's value takes in a panel `width` wide, which the description wraps in.
    pub(super) fn columns(width: u16) -> usize {
        usize::from(width.saturating_sub(2 + LABEL)).max(1)
    }

    /// The panel's height when it is `width` wide, its border included: a row for each field,
    /// the description's lines as it wraps, up to [`DESCRIPTION_LINES`], since every kind has
    /// one, a blank row and the hint.
    pub(super) fn height(&self, width: u16) -> u16 {
        let extra = self.description_lines(Self::columns(width)) - 1;
        u16::try_from(self.view.fields.len() + extra + 4).unwrap_or(u16::MAX)
    }

    /// How many lines the description takes in `columns`.
    fn description_lines(&self, columns: usize) -> usize {
        let lines = self.view.description.screen_lines(columns).len();
        lines.clamp(1, DESCRIPTION_LINES)
    }

    /// The first of the description's lines on screen in `columns`: the one that keeps its caret
    /// in the last line shown while it is focused, else the first.
    fn description_top(&self, columns: usize) -> usize {
        if self.view.focus != EditField::Description {
            return 0;
        }
        let (_, (line, _)) = field::wrapped(self.view.description, columns, self.theme);
        (line + 1).saturating_sub(self.description_lines(columns))
    }

    /// The row of each field inside the border at `inner`, in order, with how many lines it
    /// takes.
    fn rows(&self, inner: Rect) -> Vec<(EditField, Rect)> {
        let columns = usize::from(inner.width.saturating_sub(LABEL)).max(1);
        let mut y = inner.y;
        let mut rows = Vec::new();
        for &field in self.view.fields {
            let lines = if field == EditField::Description {
                self.description_lines(columns)
            } else {
                1
            };
            let height = u16::try_from(lines).unwrap_or(u16::MAX);
            let row = Rect::new(inner.x, y, inner.width, height).intersection(inner);
            rows.push((field, row));
            y = y.saturating_add(height);
        }
        rows
    }

    /// The columns the title is scrolled sideways in `columns` to keep its caret in view.
    fn title_shift(&self, columns: usize) -> usize {
        let field = self.view.title;
        let caret = Span::raw(field.text().get(..field.caret()).unwrap_or_default()).width();
        field::shift(field.text(), caret, columns)
    }

    /// Where the terminal's cursor goes when the panel is drawn over `area`: at the caret of the
    /// focused title, description or date; nowhere on the tags, Pinned or Flagged.
    pub(super) fn caret(&self, area: Rect) -> Option<Position> {
        let inner = Block::bordered().inner(area);
        let (_, row) = self
            .rows(inner)
            .into_iter()
            .find(|&(field, _)| field == self.view.focus)?;
        let x = row.x.saturating_add(LABEL);
        let columns = usize::from(row.right().saturating_sub(x)).max(1);
        let column = |field: &TextField| {
            Span::raw(field.text().get(..field.caret()).unwrap_or_default()).width()
        };
        let (dx, dy) = match self.view.focus {
            EditField::Title => (column(self.view.title) - self.title_shift(columns), 0),
            EditField::Description => {
                let (_, (line, column)) =
                    field::wrapped(self.view.description, columns, self.theme);
                (column, line - self.description_top(columns))
            }
            EditField::Start => (column(self.view.start.field), 0),
            EditField::Due => (column(self.view.due.field), 0),
            EditField::Tags | EditField::Pinned | EditField::Flagged => return None,
        };
        let at = |base: u16, offset: usize| {
            base.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX))
        };
        let last = row.right().saturating_sub(1);
        (!row.is_empty()).then(|| Position::new(at(x, dx).min(last), at(row.y, dy)))
    }

    /// A field's label.
    fn label(field: EditField) -> &'static str {
        match field {
            EditField::Title => "Title",
            EditField::Description => "Description",
            EditField::Tags => "Tags",
            EditField::Start => "Start",
            EditField::Due => "Due",
            EditField::Pinned => "Pinned",
            EditField::Flagged => "Flagged",
        }
    }

    /// The dimmed placeholder of a field without a value.
    fn none(&self, text: &'static str) -> Line<'static> {
        Line::styled(text, Style::new().fg(self.theme.dim_text))
    }

    /// The lines of `field`'s value in `columns`.
    fn value(&self, field: EditField, columns: usize) -> Vec<Line<'a>> {
        let focused = field == self.view.focus;
        match field {
            EditField::Title if focused => {
                vec![field::selected(
                    self.view.title,
                    0..self.view.title.text().len(),
                    self.theme,
                )]
            }
            EditField::Title => vec![Line::raw(self.view.title.text())],
            EditField::Description => self.description(columns),
            EditField::Tags if self.view.tags.is_empty() => vec![self.none("No tags")],
            EditField::Tags => {
                let style = Style::new().fg(self.theme.tag);
                let tags = self.view.tags.iter();
                let spans =
                    tags.flat_map(|name| [Span::styled(format!("@{name}"), style), Span::raw(" ")]);
                vec![Line::from(spans.collect::<Vec<_>>())]
            }
            EditField::Start => vec![self.date(&self.view.start, focused)],
            EditField::Due => vec![self.date(&self.view.due, focused)],
            EditField::Pinned if self.view.pinned => vec![Line::styled(
                format!("{} Pinned", Icon::DoneTask.glyph(self.icons)),
                Style::new().fg(self.theme.accent),
            )],
            EditField::Pinned => vec![Line::styled(
                format!("{} Not pinned", Icon::Task.glyph(self.icons)),
                Style::new().fg(self.theme.dim_text),
            )],
            EditField::Flagged if self.view.flagged => vec![Line::styled(
                format!("{} Flagged", Icon::Flag.glyph(self.icons)),
                Style::new().fg(self.theme.flag),
            )],
            EditField::Flagged => vec![Line::styled(
                format!("{} Not flagged", Icon::Task.glyph(self.icons)),
                Style::new().fg(self.theme.dim_text),
            )],
        }
    }

    /// The description's lines on screen in `columns`: the text being typed while it is focused,
    /// else its text, or a placeholder without one.
    fn description(&self, columns: usize) -> Vec<Line<'a>> {
        let field = self.view.description;
        let lines: Vec<Line<'a>> = if self.view.focus == EditField::Description {
            field::wrapped(field, columns, self.theme).0
        } else if field.text().is_empty() {
            return vec![self.none("No description")];
        } else {
            let text = field.text();
            (field.screen_lines(columns).into_iter())
                .map(|line| Line::raw(text.get(line).unwrap_or_default()))
                .collect()
        };
        let top = self.description_top(columns);
        lines
            .into_iter()
            .skip(top)
            .take(DESCRIPTION_LINES)
            .collect()
    }

    /// The line of a date field: the text typed and, after it, the date it resolves to or why it
    /// is not a date, while it is focused; else the date named in full and relative to today,
    /// with its chip's icon and color, a placeholder without one, or the text and why it is not a
    /// date.
    fn date(&self, view: &DatePromptView<'a>, focused: bool) -> Line<'a> {
        let theme = self.theme;
        let dim = Style::new().fg(theme.dim_text);
        let (icon, color) = match view.kind {
            DateKind::Start => (Icon::Start, theme.start),
            DateKind::Due => (Icon::Due, theme.due),
        };
        let named = |date: Date| {
            let day = date.strftime("%a %-d %b %Y");
            format!("{day} · {}", dates::relative(date, self.today))
        };
        let mut line = if focused {
            let text = view.field.text();
            let mut line = field::selected(view.field, 0..text.len(), theme);
            line.spans.push(Span::raw("  "));
            line
        } else {
            Line::default()
        };
        let preview = match view.date {
            Ok(Some(date)) if focused => Span::styled(format!("→ {}", named(date)), dim),
            Ok(Some(date)) => Span::styled(
                format!("{} {}", icon.glyph(self.icons), named(date)),
                Style::new().fg(color),
            ),
            Ok(None) => Span::styled("None", dim),
            Err(error) => {
                let text = view.field.text();
                if !focused {
                    line.spans.push(Span::raw(format!("{text}  ")));
                }
                Span::styled(
                    date_prompt::not_a_date(text, error),
                    Style::new().fg(theme.error),
                )
            }
        };
        line.spans.push(preview);
        line
    }

    /// The hint naming the focused field's keys, or why the dialog did not apply.
    fn hint(&self) -> Line<'static> {
        let error = Style::new().fg(self.theme.error);
        match self.view.problem {
            Some(EditProblem::EmptyTitle) => {
                return Line::styled(" A title can't be empty", error);
            }
            Some(EditProblem::NotADate(kind)) => {
                let view = match kind {
                    DateKind::Start => &self.view.start,
                    DateKind::Due => &self.view.due,
                };
                let why = view.date.err();
                let why = why.map(|why| date_prompt::not_a_date(view.field.text(), why));
                return Line::styled(format!(" {}", why.unwrap_or_default()), error);
            }
            None => {}
        }
        let keys = match self.view.focus {
            EditField::Title => " Tab next field · Enter confirm · Esc close",
            EditField::Description => " Enter new line · Tab next field · Esc close",
            EditField::Tags if self.view.picker.is_some() => {
                " Type to filter · Space toggles a tag · Enter close"
            }
            EditField::Tags => " Type or Space to pick tags · Backspace removes the last",
            EditField::Start | EditField::Due => {
                " Type a date: tomorrow, fri, +3d · Enter set · Esc close"
            }
            EditField::Pinned => " Space pins or unpins · Esc close",
            EditField::Flagged => " Space flags or unflags · Esc close",
        };
        Line::styled(keys, Style::new().fg(self.theme.dim_text))
    }
}

impl Widget for EditDialogPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let theme = self.theme;
        let kind = match self.view.kind {
            ItemKind::Area => "Area",
            ItemKind::Project => "Project",
            ItemKind::Section => "Section",
            ItemKind::Task | ItemKind::Inbox | ItemKind::ChecklistItem => "Task",
        };
        let accent = Style::new().fg(theme.accent);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(format!(" Edit {kind} "));
        let inner = frame.inner(area);
        frame.render(area, buf);
        let label_style = Style::new().fg(theme.dim_text);
        for (field, row) in self.rows(inner) {
            let focused = field == self.view.focus;
            let marker = if focused { FOCUS_MARKER } else { " " };
            let label = Line::from(vec![
                Span::styled(marker, accent),
                Span::raw(" "),
                Span::styled(
                    Self::label(field),
                    if focused {
                        accent.add_modifier(Modifier::BOLD)
                    } else {
                        label_style
                    },
                ),
            ]);
            label.render(Rect { height: 1, ..row }, buf);
            let x = row.x.saturating_add(LABEL);
            let value = Rect::new(x, row.y, row.right().saturating_sub(x), row.height);
            let columns = usize::from(value.width).max(1);
            let shift = if field == EditField::Title && focused {
                self.title_shift(columns)
            } else {
                0
            };
            Paragraph::new(self.value(field, columns))
                .scroll((0, u16::try_from(shift).unwrap_or(u16::MAX)))
                .render(value, buf);
        }
        if let Some(row) = inner.rows().next_back() {
            self.hint().render(row, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyEventKind};
    use jiff::civil::date;
    use ratatui::buffer::Cell;

    use super::*;
    use crate::{
        app::{Command, Message, Motion, Opened, State, TextField, update},
        domain::{Document, Item, Node, dates::DateError},
        ui::BuiltIn,
    };

    /// A Monday.
    const TODAY: Date = date(2026, 10, 5);

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    /// The fields of a project's dialog, as the core lists them.
    const PROJECT: &[EditField] = &[
        EditField::Title,
        EditField::Description,
        EditField::Tags,
        EditField::Start,
        EditField::Due,
        EditField::Pinned,
        EditField::Flagged,
    ];

    /// The fields of an area's dialog.
    const AREA: &[EditField] = &[EditField::Title, EditField::Description];

    /// The text fields a view borrows.
    struct Fields {
        title: TextField,
        description: TextField,
        start: TextField,
        due: TextField,
    }

    impl Fields {
        fn new(title: &str, description: &str, start: &str, due: &str) -> Self {
            Self {
                title: TextField::new(title.to_owned()),
                description: TextField::multi_line(description),
                start: TextField::new(start.to_owned()),
                due: TextField::new(due.to_owned()),
            }
        }

        /// A project's view on these fields with `focus` focused, its dates resolving to
        /// `start` and `due`.
        fn view(
            &self,
            focus: EditField,
            start: Result<Option<Date>, DateError>,
            due: Result<Option<Date>, DateError>,
        ) -> EditDialogView<'_> {
            EditDialogView {
                kind: ItemKind::Project,
                fields: PROJECT,
                focus,
                title: &self.title,
                description: &self.description,
                tags: Vec::new(),
                start: DatePromptView {
                    kind: DateKind::Start,
                    field: &self.start,
                    date: start,
                },
                due: DatePromptView {
                    kind: DateKind::Due,
                    field: &self.due,
                    date: due,
                },
                pinned: false,
                flagged: false,
                picker: None,
                problem: None,
            }
        }
    }

    fn panel<'a>(view: &'a EditDialogView<'a>, theme: &'a Theme) -> EditDialogPanel<'a> {
        EditDialogPanel::new(view, theme, IconSet::Plain, TODAY)
    }

    /// `view` drawn `width` columns wide at the height it asks for, as text lines.
    fn drawn(view: &EditDialogView, width: u16) -> Vec<String> {
        let theme = BuiltIn::default().colors();
        let height = panel(view, &theme).height(width);
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        panel(view, &theme).render(area, &mut buf);
        buf.content()
            .chunks(usize::from(width))
            .map(|line| line.iter().map(Cell::symbol).collect())
            .collect()
    }

    fn caret(view: &EditDialogView, area: Rect) -> Option<Position> {
        let theme = BuiltIn::default().colors();
        panel(view, &theme).caret(area)
    }

    // FR-EDIT-08
    #[test]
    fn tab_and_the_arrows_move_between_fields_but_in_the_description_they_edit() {
        let none = KeyModifiers::NONE;
        let input = |code, modifiers, focus| edit_dialog_input(&press(code, modifiers), focus, 20);
        assert_eq!(
            input(KeyCode::Tab, none, EditField::Title),
            Some(EditDialogInput::Next)
        );
        assert_eq!(
            input(KeyCode::BackTab, KeyModifiers::SHIFT, EditField::Due),
            Some(EditDialogInput::Previous)
        );
        assert_eq!(
            input(KeyCode::Down, none, EditField::Tags),
            Some(EditDialogInput::Next)
        );
        assert_eq!(
            input(KeyCode::Enter, none, EditField::Start),
            Some(EditDialogInput::Confirm)
        );
        assert_eq!(
            input(KeyCode::Esc, none, EditField::Description),
            Some(EditDialogInput::Close)
        );
        assert_eq!(
            input(KeyCode::Char('x'), none, EditField::Title),
            Some(EditDialogInput::Edit(TextEdit::Insert('x')))
        );
        assert_eq!(
            input(KeyCode::Enter, none, EditField::Description),
            Some(EditDialogInput::Edit(TextEdit::Insert('\n')))
        );
        assert_eq!(
            input(KeyCode::Up, KeyModifiers::SHIFT, EditField::Description),
            Some(EditDialogInput::Edit(TextEdit::Move {
                motion: Motion::Up { width: 20 },
                select: true,
            }))
        );
        assert_eq!(
            input(KeyCode::Down, none, EditField::Description),
            Some(EditDialogInput::Edit(TextEdit::Move {
                motion: Motion::Down { width: 20 },
                select: false,
            }))
        );
        let mut release = KeyEvent::new(KeyCode::Enter, none);
        release.kind = KeyEventKind::Release;
        assert_eq!(
            edit_dialog_input(&Event::Key(release), EditField::Title, 20),
            None
        );
    }

    // FR-EDIT-08, FR-TAG-02, D-148
    #[test]
    fn in_the_tag_picker_space_toggles_and_enter_closes() {
        let none = KeyModifiers::NONE;
        let input = |code, modifiers| tags_input(&press(code, modifiers));
        assert_eq!(input(KeyCode::Char(' '), none), Some(PickerInput::Choose));
        assert_eq!(input(KeyCode::Enter, none), Some(PickerInput::Dismiss));
        assert_eq!(input(KeyCode::Esc, none), Some(PickerInput::Dismiss));
        assert_eq!(input(KeyCode::Down, none), Some(PickerInput::Next));
        assert_eq!(
            input(KeyCode::Char('a'), none),
            Some(PickerInput::Edit(TextEdit::Insert('a')))
        );
        assert_eq!(input(KeyCode::Enter, KeyModifiers::ALT), None);
        let mut release = KeyEvent::new(KeyCode::Char(' '), none);
        release.kind = KeyEventKind::Release;
        assert_eq!(tags_input(&Event::Key(release)), None);
    }

    // FR-EDIT-08, FR-DATE-02, FR-VIEW-12
    #[test]
    fn the_panel_labels_each_field_marks_the_focused_one_and_previews_its_date() {
        let fields = Fields::new("Site", "", "tomorrow", "soon");
        let not_a_date = Err(DateError::Unexpected { offset: 0 });
        let mut view = fields.view(EditField::Start, Ok(Some(date(2026, 10, 6))), not_a_date);
        view.tags = vec!["web", "q4"];
        view.pinned = true;
        view.flagged = true;
        assert_eq!(
            drawn(&view, 60),
            [
                "┌ Edit Project ────────────────────────────────────────────┐",
                "│  Title        Site                                       │",
                "│  Description  No description                             │",
                "│  Tags         @web @q4                                   │",
                "│▌ Start        tomorrow  → Tue 6 Oct 2026 · tomorrow      │",
                "│  Due          soon  Not a date from “soon”               │",
                "│  Pinned       ✔ Pinned                                   │",
                "│  Flagged      ⚑ Flagged                                  │",
                "│                                                          │",
                "│ Type a date: tomorrow, fri, +3d · Enter set · Esc close  │",
                "└──────────────────────────────────────────────────────────┘",
            ]
        );
        view.focus = EditField::Title;
        let lines = drawn(&view, 60);
        assert_eq!(
            lines.get(4).map(String::as_str),
            Some("│  Start        ↦ Tue 6 Oct 2026 · tomorrow                │")
        );
        view.start.date = Ok(None);
        view.problem = Some(EditProblem::NotADate(DateKind::Due));
        let lines = drawn(&view, 60);
        assert_eq!(
            lines.get(4).map(String::as_str),
            Some("│  Start        None                                       │")
        );
        assert_eq!(
            lines.get(9).map(String::as_str),
            Some("│ Not a date from “soon”                                   │")
        );
        view.start.date = Err(DateError::Empty);
        view.problem = Some(EditProblem::NotADate(DateKind::Start));
        let lines = drawn(&view, 60);
        assert_eq!(
            lines.get(9).map(String::as_str),
            Some("│ Not a date: something is missing                         │")
        );
        view.problem = Some(EditProblem::EmptyTitle);
        let lines = drawn(&view, 60);
        assert_eq!(
            lines.get(9).map(String::as_str),
            Some("│ A title can't be empty                                   │")
        );
    }

    // FR-EDIT-08
    #[test]
    fn each_field_focused_names_its_keys() {
        let fields = Fields::new("Site", "", "", "");
        let hints: Vec<String> = PROJECT
            .iter()
            .map(|&focus| {
                let view = fields.view(focus, Ok(None), Ok(None));
                let lines = drawn(&view, 64);
                lines
                    .get(9)
                    .map(|line| line.trim_matches('│').trim().to_owned())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(
            hints,
            [
                "Tab next field · Enter confirm · Esc close",
                "Enter new line · Tab next field · Esc close",
                "Type or Space to pick tags · Backspace removes the last",
                "Type a date: tomorrow, fri, +3d · Enter set · Esc close",
                "Type a date: tomorrow, fri, +3d · Enter set · Esc close",
                "Space pins or unpins · Esc close",
                "Space flags or unflags · Esc close",
            ]
        );
        let mut document = Document::new();
        let task = Node::Item(Item::new(ItemKind::Task, "Call"));
        document
            .push(Some(document.inbox()), task)
            .expect("the Inbox takes a task");
        let mut state = State::new(document, Opened::Demo, TODAY);
        for message in [
            Message::Command(Command::EditItem),
            Message::EditDialog(EditDialogInput::Next),
            Message::EditDialog(EditDialogInput::Next),
            Message::EditDialog(EditDialogInput::Edit(TextEdit::Insert(' '))),
        ] {
            update(&mut state, message, TODAY);
        }
        let view = state.edit_dialog().expect("the dialog is open");
        let lines = drawn(&view, 64);
        assert_eq!(
            lines
                .iter()
                .rev()
                .nth(1)
                .map(|line| line.trim_matches('│').trim()),
            Some("Type to filter · Space toggles a tag · Enter close"),
            "with its tag picker open"
        );
    }

    // FR-EDIT-08, FR-ORG-10
    #[test]
    fn the_description_grows_to_six_lines_and_scrolls_to_its_caret() {
        let text = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight";
        let fields = Fields::new("Work", text, "", "");
        let mut view = fields.view(EditField::Title, Ok(None), Ok(None));
        (view.kind, view.fields) = (ItemKind::Area, AREA);
        assert_eq!(
            drawn(&view, 30),
            [
                "┌ Edit Area ─────────────────┐",
                "│▌ Title        Work         │",
                "│  Description  one          │",
                "│               two          │",
                "│               three        │",
                "│               four         │",
                "│               five         │",
                "│               six          │",
                "│                            │",
                "│ Tab next field · Enter conf│",
                "└────────────────────────────┘",
            ]
        );
        view.focus = EditField::Description;
        let lines = drawn(&view, 30);
        assert_eq!(
            lines.get(2).map(String::as_str),
            Some("│▌ Description  three        │")
        );
        assert_eq!(
            lines.get(7).map(String::as_str),
            Some("│               eight        │")
        );
        let area = Rect::new(0, 0, 30, 11);
        assert_eq!(
            caret(&view, area),
            Some(Position::new(21, 7)),
            "after `eight`"
        );
    }

    // FR-EDIT-08
    #[test]
    fn the_caret_goes_where_the_focused_text_is_typed_and_a_long_title_scrolls() {
        let fields = Fields {
            title: TextField::new("A title too long to fit".to_owned()),
            ..Fields::new("", "", "fri", "")
        };
        let area = Rect::new(2, 10, 30, 10);
        let at = |focus| caret(&fields.view(focus, Ok(None), Ok(None)), area);
        assert_eq!(
            at(EditField::Title),
            Some(Position::new(30, 11)),
            "kept in view"
        );
        assert_eq!(at(EditField::Description), Some(Position::new(18, 12)));
        assert_eq!(at(EditField::Start), Some(Position::new(21, 14)));
        assert_eq!(at(EditField::Due), Some(Position::new(18, 15)));
        assert_eq!(at(EditField::Tags), None);
        assert_eq!(at(EditField::Pinned), None);
        assert_eq!(at(EditField::Flagged), None);
        let view = fields.view(EditField::Title, Ok(None), Ok(None));
        let lines = drawn(&view, 30);
        assert_eq!(
            lines.get(1).map(String::as_str),
            Some("│▌ Title         long to fit │")
        );
        assert_eq!(
            caret(&view, Rect::new(0, 0, 30, 2)),
            None,
            "no row to show it in"
        );
    }
}
