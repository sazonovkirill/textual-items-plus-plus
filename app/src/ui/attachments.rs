//! The attachments panel in the terminal (FR-ATT-01–03): the keys that drive it and the panel
//! itself, over the right side of the screen from its top to the status line, as
//! taskpaper-terminal draws it
//! ([attachments.md §3, §4](../../../specs/reference/taskpaper-terminal/specs/attachments.md#3-attachments-panel-fr-att-03)):
//! a border titled with the item, the Note section under its heading, the note rendered as
//! Markdown or, while it is edited, as typed, and a hint naming the keys.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget, Wrap},
};

use super::{
    Icon, IconSet, Theme, field,
    keymap::{self, Key},
    markdown::{self, Format, NoteLine},
};
use crate::app::{AttachmentsInput, AttachmentsView};

/// The narrowest and widest the panel is drawn, screen width allowing.
const WIDTH: (u16, u16) = (40, 72);

/// What a key of the open panel does: something the core does, or scrolling, which only the
/// frontend knows how far goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelKey {
    /// Gives the core this input.
    Input(AttachmentsInput),
    /// Scrolls the content.
    Scroll(Scroll),
}

/// How far the panel's content scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scroll {
    /// This many lines down, or up when negative.
    Lines(isize),
    /// This many pages down, or up when negative.
    Pages(isize),
    /// To the top.
    Top,
    /// To the bottom.
    Bottom,
}

impl Scroll {
    /// The first line shown after scrolling from `top` with pages of `page` lines. The bottom is
    /// as far as there is; drawing keeps the content in view.
    pub(super) fn from(self, top: usize, page: usize) -> usize {
        match self {
            Self::Lines(lines) => top.saturating_add_signed(lines),
            Self::Pages(pages) => {
                top.saturating_add_signed(pages.saturating_mul(page.cast_signed()))
            }
            Self::Top => 0,
            Self::Bottom => usize::MAX,
        }
    }
}

/// The keys of the open panel while the note is not edited, as taskpaper-terminal's
/// (keybindings.md §4).
const KEYS: &[(Key, PanelKey)] = &[
    (
        (KeyModifiers::NONE, KeyCode::Esc),
        PanelKey::Input(AttachmentsInput::Close),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('e')),
        PanelKey::Input(AttachmentsInput::EditNote),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Enter),
        PanelKey::Input(AttachmentsInput::EditNote),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('j')),
        PanelKey::Scroll(Scroll::Lines(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Down),
        PanelKey::Scroll(Scroll::Lines(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('k')),
        PanelKey::Scroll(Scroll::Lines(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Up),
        PanelKey::Scroll(Scroll::Lines(-1)),
    ),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('d')),
        PanelKey::Scroll(Scroll::Pages(1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::PageDown),
        PanelKey::Scroll(Scroll::Pages(1)),
    ),
    (
        (KeyModifiers::CONTROL, KeyCode::Char('u')),
        PanelKey::Scroll(Scroll::Pages(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::PageUp),
        PanelKey::Scroll(Scroll::Pages(-1)),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('g')),
        PanelKey::Scroll(Scroll::Top),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Home),
        PanelKey::Scroll(Scroll::Top),
    ),
    (
        (KeyModifiers::NONE, KeyCode::Char('G')),
        PanelKey::Scroll(Scroll::Bottom),
    ),
    (
        (KeyModifiers::NONE, KeyCode::End),
        PanelKey::Scroll(Scroll::Bottom),
    ),
];

/// What `event` does in the open panel while the note is not edited: `Esc` closes it, `e` and
/// `Enter` edit the note, and the motion keys scroll. Only key presses count.
pub(super) fn panel_input(event: &Event) -> Option<PanelKey> {
    let key = keymap::key_press(event)?;
    KEYS.iter()
        .find(|(bound, _)| *bound == key)
        .map(|&(_, input)| input)
}

/// The open panel. Its state is the first line of the content shown, which it keeps within the
/// content, and on the caret while the note is edited.
pub(super) struct AttachmentsPanel<'a> {
    view: AttachmentsView<'a>,
    theme: &'a Theme,
    icons: IconSet,
}

impl<'a> AttachmentsPanel<'a> {
    /// The panel showing `view` in `theme` with `icons`.
    pub(super) fn new(view: AttachmentsView<'a>, theme: &'a Theme, icons: IconSet) -> Self {
        Self { view, theme, icons }
    }

    /// Where the panel goes over `screen`, the screen above the status line: at its right, half
    /// as wide within 40 to 72 columns, its whole height.
    pub(super) fn area(screen: Rect) -> Rect {
        let width = (screen.width / 2).clamp(WIDTH.0, WIDTH.1).min(screen.width);
        Rect::new(
            screen.right().saturating_sub(width),
            screen.y,
            width,
            screen.height,
        )
    }

    /// Where the content goes when the panel is drawn over `area`: inside the border, a column in
    /// from each side, above a blank line and the hint.
    fn content(area: Rect) -> Rect {
        let inner = Block::bordered().inner(area);
        Rect::new(
            inner.x.saturating_add(1),
            inner.y,
            inner.width.saturating_sub(2),
            inner.height.saturating_sub(2),
        )
    }

    /// How many lines a page of the content holds when the panel is drawn over `area`, one at
    /// least.
    pub(super) fn page(area: Rect) -> usize {
        usize::from(Self::content(area).height).max(1)
    }

    /// The columns the note being edited wraps in when the panel is drawn over `area`: all the
    /// content has but the last, kept for the caret at the end of a line.
    pub(super) fn columns(area: Rect) -> usize {
        usize::from(Self::content(area).width).saturating_sub(1)
    }

    /// Where the terminal's cursor goes when the panel is drawn over `area` scrolled to line
    /// `top`: at the caret of the note being edited, while its line is in view.
    pub(super) fn caret(&self, area: Rect, top: usize) -> Option<Position> {
        let content = Self::content(area);
        let (_, (line, column)) = self.edited(Self::columns(area))?;
        let y = (line + 1).checked_sub(top)?;
        (y < usize::from(content.height)).then(|| {
            let offset = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
            Position::new(
                content.x.saturating_add(offset(column)),
                content.y.saturating_add(offset(y)),
            )
        })
    }

    /// The note being edited wrapped in `columns`, and the caret's line and column among its
    /// lines, while it is edited.
    fn edited(&self, columns: usize) -> Option<(Vec<Line<'a>>, (usize, usize))> {
        let field = self.view.editing?;
        Some(field::wrapped(field, columns, self.theme))
    }

    /// The Note section laid out in `area`: its heading, then the note, rendered and wrapping,
    /// or a dimmed "No note"; while it is edited, its text as typed, wrapped as the field wraps
    /// it, and the content's line with the caret.
    fn section(&self, area: Rect) -> (Paragraph<'a>, Option<usize>) {
        let heading = Line::styled(
            format!("{} Note", Icon::Notes.glyph(self.icons)),
            Style::new().fg(self.theme.headers).bold(),
        )
        .right_aligned();
        let mut lines = vec![heading];
        if let Some((edited, (line, _))) = self.edited(Self::columns(area)) {
            lines.extend(edited);
            return (Paragraph::new(lines), Some(line + 1));
        }
        let note = &self.view.item.note;
        if note.is_empty() {
            lines.push(Line::styled(
                "No note",
                Style::new().fg(self.theme.dim_text),
            ));
        }
        lines.extend(
            markdown::note_lines(note)
                .into_iter()
                .map(|line| self.line(line)),
        );
        (Paragraph::new(lines).wrap(Wrap { trim: false }), None)
    }

    /// A line of the note as Markdown shows it: a heading bold, the first level also
    /// underlined; a list item after its marker, a bullet for `-`, `*` or `+`; a quote dimmed
    /// after a bar; code dimmed as typed; text after its indentation; each with its inline
    /// formatting.
    fn line(&self, line: NoteLine<'a>) -> Line<'a> {
        let dim = Style::new().add_modifier(Modifier::DIM);
        let (prefix, spans) = match line {
            NoteLine::Heading(level, text) => {
                let style = if level == 1 {
                    Style::new().bold().underlined()
                } else {
                    Style::new().bold()
                };
                (String::new(), self.formatted(text, style))
            }
            NoteLine::Item {
                depth,
                marker,
                text,
            } => {
                let marker = if matches!(marker, "-" | "*" | "+") {
                    "•"
                } else {
                    marker
                };
                (
                    format!("{}{marker} ", "  ".repeat(depth)),
                    self.formatted(text, Style::new()),
                )
            }
            NoteLine::Quote(text) => ("│ ".to_owned(), self.formatted(text, dim)),
            NoteLine::Code(text) => (String::new(), vec![Span::styled(drawable(text), dim)]),
            NoteLine::Text(text) => {
                let body = text.trim_start();
                let indent = text.get(..text.len() - body.len()).unwrap_or_default();
                (drawable(indent), self.formatted(body, Style::new()))
            }
        };
        let mut shown = Line::from(Span::styled(prefix, dim));
        shown.spans.extend(spans);
        shown
    }

    /// `text` with its inline formatting over `style`: bold, italic, code dimmed and links
    /// underlined in the link color.
    fn formatted(&self, text: &'a str, style: Style) -> Vec<Span<'a>> {
        markdown::runs(text)
            .into_iter()
            .map(|run| {
                let Format {
                    bold,
                    code,
                    italic,
                    link,
                } = run.format;
                let mut style = style;
                if bold {
                    style = style.bold();
                }
                if italic {
                    style = style.italic();
                }
                if code {
                    style = style.add_modifier(Modifier::DIM);
                }
                if link {
                    style = style.underlined().fg(self.theme.link);
                }
                Span::styled(drawable(run.text), style)
            })
            .collect()
    }

    /// The hint naming the keys of the note, or of the note being edited.
    fn hint(&self) -> Line<'static> {
        let keys = if self.view.editing.is_some() {
            "Enter new line · Esc done"
        } else {
            "e edit · Esc close"
        };
        Line::styled(keys, Style::new().fg(self.theme.dim_text))
    }
}

/// `text` as the buffer can draw it: a tab shows as a space, the one column its width counts.
fn drawable(text: &str) -> String {
    text.replace('\t', " ")
}

impl StatefulWidget for AttachmentsPanel<'_> {
    type State = usize;

    /// Draws the panel over the right side of `area`, its content from line `top`, which it keeps
    /// within the content and, while the note is edited, on the caret's line.
    fn render(self, area: Rect, buf: &mut Buffer, top: &mut usize) {
        let area = Self::area(area);
        let theme = self.theme;
        Clear.render(area, buf);
        let accent = Style::new().fg(theme.accent);
        let frame = Block::bordered()
            .style(Style::new().bg(theme.background).fg(theme.text))
            .border_style(accent)
            .title_style(accent)
            .title(format!(" {} ", self.view.item.title));
        let inner = frame.inner(area);
        frame.render(area, buf);
        let content = Self::content(area);
        let (section, caret) = self.section(area);
        let page = Self::page(area);
        let total = section.line_count(content.width);
        *top = (*top).min(total.saturating_sub(page));
        if let Some(line) = caret {
            *top = (*top).clamp((line + 1).saturating_sub(page), line);
        }
        section
            .scroll((u16::try_from(*top).unwrap_or(u16::MAX), 0))
            .render(content, buf);
        if inner.height >= 2 {
            let hint = Rect::new(content.x, inner.bottom() - 1, content.width, 1);
            self.hint().render(hint, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyEvent;

    use super::*;
    use crate::app::{Motion, TextField};
    use crate::{
        app::TextEdit,
        domain::{Item, ItemId, ItemKind},
        ui::BuiltIn,
    };

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    // FR-ATT-02, FR-ATT-03
    #[test]
    fn escape_closes_e_and_enter_edit_and_the_motion_keys_scroll() {
        let none = KeyModifiers::NONE;
        for (code, modifiers, key) in [
            (KeyCode::Esc, none, PanelKey::Input(AttachmentsInput::Close)),
            (
                KeyCode::Char('e'),
                none,
                PanelKey::Input(AttachmentsInput::EditNote),
            ),
            (
                KeyCode::Enter,
                none,
                PanelKey::Input(AttachmentsInput::EditNote),
            ),
            (KeyCode::Char('j'), none, PanelKey::Scroll(Scroll::Lines(1))),
            (KeyCode::Up, none, PanelKey::Scroll(Scroll::Lines(-1))),
            (
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
                PanelKey::Scroll(Scroll::Pages(1)),
            ),
            (KeyCode::PageUp, none, PanelKey::Scroll(Scroll::Pages(-1))),
            (KeyCode::Char('g'), none, PanelKey::Scroll(Scroll::Top)),
            (
                KeyCode::Char('G'),
                KeyModifiers::SHIFT,
                PanelKey::Scroll(Scroll::Bottom),
            ),
        ] {
            assert_eq!(
                panel_input(&press(code, modifiers)),
                Some(key),
                "{code:?} {modifiers:?}"
            );
        }
        assert_eq!(panel_input(&press(KeyCode::Char('x'), none)), None);
    }

    #[test]
    fn scrolling_moves_by_lines_and_pages_and_goes_to_either_end() {
        assert_eq!(Scroll::Lines(1).from(3, 10), 4);
        assert_eq!(Scroll::Lines(-5).from(3, 10), 0);
        assert_eq!(Scroll::Pages(2).from(3, 10), 23);
        assert_eq!(Scroll::Pages(-1).from(3, 10), 0);
        assert_eq!(Scroll::Top.from(3, 10), 0);
        assert_eq!(Scroll::Bottom.from(3, 10), usize::MAX);
    }

    /// Every row of `buf`, without trailing spaces.
    fn rows(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    /// The panel on an item titled `Milk` with note `note`, edited in `editing` if given, drawn
    /// in Plain icons over `width` × `height` cells from line `top`; the rows, the line drawn
    /// from and the caret.
    fn draw(
        note: &str,
        editing: Option<&TextField>,
        (width, height): (u16, u16),
        top: usize,
    ) -> (Buffer, usize, Option<Position>) {
        let item = Item {
            note: note.to_owned(),
            ..Item::new(ItemKind::Task, "Milk")
        };
        let theme = BuiltIn::default().colors();
        let view = AttachmentsView {
            id: ItemId::new(1),
            item: &item,
            editing,
        };
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        let mut top = top;
        AttachmentsPanel::new(view, &theme, IconSet::Plain).render(area, &mut buf, &mut top);
        let caret = AttachmentsPanel::new(view, &theme, IconSet::Plain)
            .caret(AttachmentsPanel::area(area), top);
        (buf, top, caret)
    }

    // FR-ATT-01
    #[test]
    fn the_panel_covers_the_right_half_within_40_to_72_columns() {
        assert_eq!(
            AttachmentsPanel::area(Rect::new(0, 0, 100, 20)),
            Rect::new(50, 0, 50, 20)
        );
        assert_eq!(
            AttachmentsPanel::area(Rect::new(0, 0, 60, 20)),
            Rect::new(20, 0, 40, 20)
        );
        assert_eq!(
            AttachmentsPanel::area(Rect::new(0, 0, 200, 20)),
            Rect::new(128, 0, 72, 20)
        );
        assert_eq!(
            AttachmentsPanel::area(Rect::new(0, 0, 30, 20)),
            Rect::new(0, 0, 30, 20)
        );
    }

    // FR-ATT-01, FR-ATT-03
    #[test]
    fn a_note_shows_rendered_as_markdown_under_its_heading() {
        let note = "# Plan\n## Steps\n- **buy** milk\n  * see [shop](https://a.b)\n1. one\n> said\n```\n  code\n```\n\tindented";
        let (buf, top, caret) = draw(note, None, (80, 16), 0);
        assert_eq!((top, caret), (0, None));
        assert!(
            (0..40).all(|x| (0..16).all(|y| buf[(x, y)].symbol() == " ")),
            "only the right half is drawn"
        );
        let panel: Vec<String> = rows(&buf)
            .iter()
            .map(|row| row.get(40..).unwrap_or_default().to_owned())
            .collect();
        assert_eq!(
            panel,
            [
                "┌ Milk ────────────────────────────────┐",
                "│                               ≡ Note │",
                "│ Plan                                 │",
                "│ Steps                                │",
                "│ • buy milk                           │",
                "│   • see shop                         │",
                "│ 1. one                               │",
                "│ │ said                               │",
                "│   code                               │",
                "│  indented                            │",
                "│                                      │",
                "│                                      │",
                "│                                      │",
                "│                                      │",
                "│ e edit · Esc close                   │",
                "└──────────────────────────────────────┘",
            ]
        );
        let theme = BuiltIn::default().colors();
        let cell = |x: u16, y: u16| &buf[(x, y)];
        assert!(cell(42, 2).modifier.contains(Modifier::UNDERLINED), "h1");
        assert!(cell(42, 3).modifier.contains(Modifier::BOLD), "h2");
        assert!(!cell(42, 3).modifier.contains(Modifier::UNDERLINED));
        assert!(cell(44, 4).modifier.contains(Modifier::BOLD), "bold");
        assert_eq!(cell(52, 5).fg, theme.link, "link");
        assert!(cell(42, 7).modifier.contains(Modifier::DIM), "quote");
        assert!(cell(44, 8).modifier.contains(Modifier::DIM), "code");
        assert_eq!(cell(72, 1).fg, theme.headers, "heading");
    }

    // FR-ATT-03
    #[test]
    fn inline_formats_show_in_their_styles() {
        let (buf, ..) = draw("*it* `c` **b**", None, (40, 8), 0);
        let cell = |x: u16| &buf[(x, 2)];
        assert!(cell(2).modifier.contains(Modifier::ITALIC));
        assert!(cell(5).modifier.contains(Modifier::DIM));
        assert!(cell(7).modifier.contains(Modifier::BOLD));
        assert_eq!(rows(&buf)[2], "│ it c b                               │");
    }

    // FR-ATT-01
    #[test]
    fn no_note_shows_dimmed() {
        let (buf, ..) = draw("", None, (40, 6), 0);
        assert_eq!(rows(&buf)[2], "│ No note                              │");
        let theme = BuiltIn::default().colors();
        assert_eq!(buf[(2, 2)].fg, theme.dim_text);
    }

    // FR-ATT-01
    #[test]
    fn the_content_scrolls_no_further_than_its_last_line_and_wraps() {
        let note = (1..=10)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (buf, top, _) = draw(&note, None, (40, 8), usize::MAX);
        assert_eq!(top, 7, "eleven lines, four in view");
        assert_eq!(
            rows(&buf)[1..5].to_vec(),
            [
                "│ line 7                               │",
                "│ line 8                               │",
                "│ line 9                               │",
                "│ line 10                              │",
            ]
        );
        let long = "word ".repeat(12);
        let (buf, ..) = draw(&long, None, (40, 8), 0);
        assert_eq!(rows(&buf)[2], "│ word word word word word word word   │");
        assert_eq!(rows(&buf)[3], "│ word word word word word             │");
    }

    // FR-ATT-03
    #[test]
    fn a_note_edited_shows_as_typed_with_the_caret_in_view() {
        let mut field = TextField::multi_line("# Plan\n**b**");
        let (buf, top, caret) = draw("# Plan\n**b**", Some(&field), (40, 8), 0);
        assert_eq!(top, 0);
        assert_eq!(
            rows(&buf)[1..4].to_vec(),
            [
                "│                               ≡ Note │",
                "│ # Plan                               │",
                "│ **b**                                │",
            ]
        );
        assert_eq!(rows(&buf)[6], "│ Enter new line · Esc done            │");
        assert_eq!(caret, Some(Position::new(7, 3)));
        let text = (1..=10)
            .map(|n| format!("{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        field = TextField::multi_line(&text);
        let (_, top, caret) = draw(&text, Some(&field), (40, 8), 0);
        assert_eq!((top, caret), (7, Some(Position::new(4, 4))));
        field = field.edited(TextEdit::Move {
            motion: Motion::Start,
            select: false,
        });
        let (_, top, caret) = draw(&text, Some(&field), (40, 8), 7);
        assert_eq!((top, caret), (1, Some(Position::new(2, 1))));
        assert_eq!(
            AttachmentsPanel::new(
                AttachmentsView {
                    id: ItemId::new(1),
                    item: &Item::new(ItemKind::Task, "x"),
                    editing: Some(&field),
                },
                &BuiltIn::default().colors(),
                IconSet::Plain,
            )
            .caret(Rect::new(0, 0, 40, 8), 5),
            None,
            "a caret scrolled out of view shows nowhere"
        );
        let (_, _, caret) = draw(&text, Some(&field), (40, 4), 0);
        assert_eq!(caret, None, "a panel too short for its content still draws");
    }
}
