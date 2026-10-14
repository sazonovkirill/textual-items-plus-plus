//! Quick Add in the terminal (FR-EDIT-07): the keys that drive its panel and the panel, across the
//! screen just above the status line where the date prompt opens, as taskpaper-terminal draws it:
//! titled with the place the task goes, the text typed with the dates found in it in their chips'
//! colors and, below it, the dates `Enter` sets.

use std::ops::Range;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use jiff::civil::Date;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Widget},
};

use super::{
    Theme, field,
    keymap::{self, Key},
    text_keys,
};
use crate::{app::QuickAddInput, app::QuickAddView, domain::dates::DateKind};

/// The keys of the open panel besides the text keys.
pub(super) const KEYS: &[(Key, QuickAddInput)] = &[
    ((KeyModifiers::NONE, KeyCode::Enter), QuickAddInput::Add),
    ((KeyModifiers::NONE, KeyCode::Esc), QuickAddInput::Cancel),
];

/// What `event` does in the open panel: `Enter` adds, `Esc` cancels, and the text keys edit the
/// text. Only key presses count.
pub(super) fn quick_add_input(event: &Event) -> Option<QuickAddInput> {
    let key = keymap::key_press(event)?;
    KEYS.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
        .or_else(|| text_keys::text_edit(key).map(QuickAddInput::Edit))
}

/// The open panel: titled with the path of the place the task goes, the text typed after a space,
/// the dates found in it in the start and due colors and its selection in the selection colors,
/// and below it the dates the task gets, or what it lacks.
pub(super) struct QuickAddPanel<'a> {
    view: &'a QuickAddView<'a>,
    theme: &'a Theme,
}

impl<'a> QuickAddPanel<'a> {
    /// The panel's height, its border included.
    pub(super) const HEIGHT: u16 = 4;

    /// The panel showing `view` in `theme`.
    pub(super) fn new(view: &'a QuickAddView<'a>, theme: &'a Theme) -> Self {
        Self { view, theme }
    }

    /// Where the terminal's cursor goes when the panel is drawn over `area`, as for a prompt.
    pub(super) fn caret(&self, area: Rect) -> Option<Position> {
        field::prompt_caret(self.view.field, area)
    }

    /// The text typed, each date found in its chip's color, the selection in the selection
    /// colors.
    fn input(&self) -> Line<'a> {
        let field = self.view.field;
        let text = field.text();
        let selection = field.selection().unwrap_or(field.caret()..field.caret());
        let mut cuts = vec![0, text.len(), selection.start, selection.end];
        for found in &self.view.found {
            cuts.extend([found.span.start, found.span.end]);
        }
        cuts.sort_unstable();
        cuts.dedup();
        let spans = cuts.array_windows().filter_map(|&[start, end]| {
            Some(Span::styled(
                text.get(start..end)?,
                self.style(&(start..end), &selection),
            ))
        });
        Line::from_iter(spans)
    }

    /// How the bytes `part` of the text are drawn: in the selection colors when `selection` holds
    /// them, else in the color of the date found there, if any.
    fn style(&self, part: &Range<usize>, selection: &Range<usize>) -> Style {
        let theme = self.theme;
        if selection.start <= part.start && part.end <= selection.end {
            return Style::new().bg(theme.selection).fg(theme.band_text);
        }
        let found = self
            .view
            .found
            .iter()
            .find(|found| found.span.start <= part.start && part.end <= found.span.end);
        match found.map(|found| found.kind) {
            Some(DateKind::Start) => Style::new().fg(theme.start),
            Some(DateKind::Due) => Style::new().fg(theme.due),
            None => Style::new(),
        }
    }

    /// What `Enter` gives the task: its start and due dates in the chips' colors, or that it has
    /// none; while there is no title, how to give one and choose its place.
    fn preview(&self) -> Line<'static> {
        let theme = self.theme;
        let dim = Style::new().fg(theme.dim_text);
        if self.view.title.is_empty() {
            return Line::styled(" Type the task; / chooses where it goes", dim);
        }
        let day = |date: Date| date.strftime("%a %-d %b %Y").to_string();
        let dates: Vec<Span<'static>> = [
            self.view.start.map(|date| {
                Span::styled(format!("Start {}", day(date)), Style::new().fg(theme.start))
            }),
            self.view
                .due
                .map(|date| Span::styled(format!("Due {}", day(date)), Style::new().fg(theme.due))),
        ]
        .into_iter()
        .flatten()
        .collect();
        if dates.is_empty() {
            return Line::styled(" No dates", dim);
        }
        let mut spans = vec![Span::raw(" ")];
        for (at, date) in dates.into_iter().enumerate() {
            if at > 0 {
                spans.push(Span::styled(" · ", dim));
            }
            spans.push(date);
        }
        Line::from(spans)
    }
}

impl Widget for QuickAddPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let theme = self.theme;
        let accent = Style::new().fg(theme.accent);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(format!(" Quick Add → {} ", self.view.place.join(" › ")));
        let inner = frame.inner(area);
        frame.render(area, buf);
        let mut rows = inner.rows();
        if let Some(row) = rows.next() {
            field::draw_prompt(self.view.field, self.input(), row, buf);
        }
        if let Some(row) = rows.next() {
            self.preview().render(row, buf);
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
        app::{TextEdit, TextField},
        domain::dates,
        ui::BuiltIn,
    };

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    // FR-EDIT-07
    #[test]
    fn enter_adds_escape_cancels_and_text_keys_edit() {
        assert_eq!(
            quick_add_input(&press(KeyCode::Enter)),
            Some(QuickAddInput::Add)
        );
        assert_eq!(
            quick_add_input(&press(KeyCode::Esc)),
            Some(QuickAddInput::Cancel)
        );
        assert_eq!(
            quick_add_input(&press(KeyCode::Char('/'))),
            Some(QuickAddInput::Edit(TextEdit::Insert('/')))
        );
        let mut release = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(quick_add_input(&Event::Key(release)), None);
    }

    /// The view of `field`, its dates found on 2026-10-14, going to the Inbox.
    fn view(field: &TextField) -> QuickAddView<'_> {
        let found = dates::find(field.text(), date(2026, 10, 14));
        let dated = |kind| {
            found
                .iter()
                .find(|found| found.kind == kind)
                .map(|found| found.date)
        };
        QuickAddView {
            field,
            title: String::new(),
            start: dated(DateKind::Start),
            due: dated(DateKind::Due),
            place: vec!["Inbox".to_owned()],
            found,
        }
    }

    /// The rows inside the border of the panel showing `view`, 40 columns wide.
    fn rows(view: &QuickAddView<'_>, theme: &Theme) -> (Buffer, Vec<String>) {
        let area = Rect::new(0, 0, 40, QuickAddPanel::HEIGHT);
        let mut buf = Buffer::empty(area);
        QuickAddPanel::new(view, theme).render(area, &mut buf);
        let lines = buf
            .content()
            .chunks(40)
            .map(|line| line.iter().map(Cell::symbol).collect())
            .collect();
        (buf, lines)
    }

    // FR-EDIT-07
    #[test]
    fn the_panel_colors_the_dates_found_and_the_selection() {
        let theme = BuiltIn::default().colors();
        let field = TextField::new("Pay due fri".to_owned());
        let mut shown = view(&field);
        shown.title = "Pay".to_owned();
        let (buf, lines) = rows(&shown, &theme);
        assert_eq!(lines[1], "│ Pay due fri                          │");
        assert_eq!(lines[2], "│ Due Fri 16 Oct 2026                  │");
        assert_eq!(buf[(2, 1)].fg, theme.text);
        assert_eq!(buf[(6, 1)].fg, theme.due, "due fri");
        assert_eq!(buf[(2, 2)].fg, theme.due);
        let selected = field.edited(TextEdit::SelectAll);
        let (buf, _) = rows(&view(&selected), &theme);
        assert_eq!(buf[(6, 1)].bg, theme.selection);
    }

    // FR-EDIT-07
    #[test]
    fn the_panel_shows_both_dates_no_dates_or_how_to_start() {
        let theme = BuiltIn::default().colors();
        let field = TextField::new("Pay friday due fri".to_owned());
        let mut shown = view(&field);
        shown.title = "Pay".to_owned();
        let (buf, lines) = rows(&shown, &theme);
        assert_eq!(lines[2], "│ Start Fri 16 Oct 2026 · Due Fri 16 Oc│");
        assert_eq!(buf[(6, 1)].fg, theme.start, "friday");
        assert_eq!(buf[(2, 2)].fg, theme.start);
        let field = TextField::new("Pay".to_owned());
        let mut shown = view(&field);
        shown.title = "Pay".to_owned();
        assert_eq!(
            rows(&shown, &theme).1[2],
            "│ No dates                             │"
        );
        shown.title = String::new();
        assert_eq!(
            rows(&shown, &theme).1[2],
            "│ Type the task; / chooses where it goe│"
        );
    }

    // FR-EDIT-07
    #[test]
    fn the_caret_follows_the_text_unless_there_is_no_room() {
        let theme = BuiltIn::default().colors();
        let field = TextField::new("ab".to_owned());
        let shown = view(&field);
        let panel = QuickAddPanel::new(&shown, &theme);
        assert_eq!(
            panel.caret(Rect::new(0, 10, 20, QuickAddPanel::HEIGHT)),
            Some(Position::new(4, 11))
        );
        assert_eq!(panel.caret(Rect::new(0, 0, 1, 1)), None);
    }

    // FR-EDIT-07, FR-UI-06
    #[test]
    fn a_long_text_scrolls_sideways_to_keep_the_caret_in_view() {
        let theme = BuiltIn::default().colors();
        let field = TextField::new("Call the plumber about the kitchen sink due fri".to_owned());
        let shown = view(&field);
        let (buf, lines) = rows(&shown, &theme);
        assert_eq!(lines[1], "│ umber about the kitchen sink due fri │");
        assert_eq!(buf[(31, 1)].fg, theme.due, "due, scrolled with the text");
        assert_eq!(
            QuickAddPanel::new(&shown, &theme).caret(Rect::new(0, 10, 40, QuickAddPanel::HEIGHT)),
            Some(Position::new(38, 11))
        );
    }
}
