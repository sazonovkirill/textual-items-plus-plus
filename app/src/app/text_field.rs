//! A text being edited with the standard text keys, on one line or on several: a caret and a
//! selection that move by grapheme, word and line, insertion, deletion and pasting (FR-EDIT-04,
//! N-PORT-04), and the lines it takes on screen. Hides grapheme and word segmentation and wrapping.

use std::{iter, ops::Range};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A change to a [`TextField`], as a text key asks for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEdit {
    /// Type a character in place of the selection, or at the caret. A character that breaks a
    /// line starts a new line in a multi-line field and is ignored on one line.
    Insert(char),
    /// Move the caret; with `select`, extend the selection to it.
    Move {
        /// Where to.
        motion: Motion,
        /// Whether the selection extends to the new caret instead of being cleared.
        select: bool,
    },
    /// Delete the selection, or the grapheme before the caret.
    DeleteBackward,
    /// Delete the selection, or the grapheme after the caret.
    DeleteForward,
    /// Delete the selection, or back to the start of the word before the caret.
    DeleteWordBackward,
    /// Delete the selection, or up to the end of the word after the caret.
    DeleteWordForward,
    /// Select the whole text.
    SelectAll,
}

/// Where a caret motion goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// One grapheme back.
    Left,
    /// One grapheme forward.
    Right,
    /// To the start of the word before the caret.
    WordLeft,
    /// To the end of the word after the caret.
    WordRight,
    /// To the start of the caret's line.
    LineStart,
    /// To the end of the caret's line.
    LineEnd,
    /// To the line above on screen, as near the column the caret moved up or down from as that
    /// line allows, or to the start of the text from its first line; the lines are those of
    /// [`TextField::screen_lines`] in `width` columns.
    Up {
        /// The columns the text wraps in.
        width: usize,
    },
    /// To the line below on screen, as near the column the caret moved up or down from as that
    /// line allows, or to the end of the text from its last line; the lines are those of
    /// [`TextField::screen_lines`] in `width` columns.
    Down {
        /// The columns the text wraps in.
        width: usize,
    },
    /// To the start of the text.
    Start,
    /// To the end of the text.
    End,
    /// To this byte offset of the text, or the start of the grapheme it falls in, or the end
    /// when it is past it, as a click places the caret.
    To(usize),
}

/// How many lines a field holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lines {
    One,
    Many,
}

/// A text with a caret and an optional selection, both at grapheme boundaries. A multi-line
/// field separates its lines with `\n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextField {
    text: String,
    /// The caret's byte offset.
    caret: usize,
    /// Where the selection started, as a byte offset; the caret is its other end.
    anchor: Option<usize>,
    /// The column `Up` and `Down` aim for, kept while they repeat so that a short line on the
    /// way does not pull the caret to its end for good.
    goal: Option<usize>,
    lines: Lines,
}

impl TextField {
    /// A one-line field holding `text` with the caret at its end.
    #[must_use]
    pub fn new(text: String) -> Self {
        Self::with_caret(text, usize::MAX)
    }

    /// A one-line field holding `text` with the caret at byte `caret`, or the start of the
    /// grapheme it falls in, or the end when it is past it, and nothing selected.
    #[must_use]
    pub fn with_caret(text: String, caret: usize) -> Self {
        let field = Self {
            text,
            caret: 0,
            anchor: None,
            goal: None,
            lines: Lines::One,
        };
        Self {
            caret: field.boundary(caret),
            ..field
        }
    }

    /// A one-line field holding `text` with the caret `column` graphemes from its start, at the
    /// end when it is shorter, and nothing selected.
    #[must_use]
    pub fn at_column(text: String, column: usize) -> Self {
        let caret = text
            .grapheme_indices(true)
            .nth(column)
            .map_or(text.len(), |(at, _)| at);
        Self::with_caret(text, caret)
    }

    /// A multi-line field holding `text`, each of its line breaks taken as `\n`, with the
    /// caret at its end.
    #[must_use]
    pub fn multi_line(text: &str) -> Self {
        Self {
            lines: Lines::Many,
            ..Self::new(newlines(text))
        }
    }

    /// How many graphemes stand between the start of the caret's line and the caret.
    #[must_use]
    pub fn column(&self) -> usize {
        self.text
            .get(self.line_start()..self.caret)
            .map_or(0, |before| before.graphemes(true).count())
    }

    /// The text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The caret's byte offset into the text.
    #[must_use]
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The selected bytes, if any are selected.
    #[must_use]
    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor.filter(|&anchor| anchor != self.caret)?;
        Some(anchor.min(self.caret)..anchor.max(self.caret))
    }

    /// The selected text, which copying or cutting takes, if any is selected.
    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        self.text.get(self.selection()?)
    }

    /// The lines the text takes on screen in `width` columns, at least one, as byte ranges
    /// without their `\n`: each of its lines, wrapped between words, and inside a word too long
    /// for a line of its own. Spaces at the end of a line stay on it, even past `width`.
    #[must_use]
    pub fn screen_lines(&self, width: usize) -> Vec<Range<usize>> {
        let width = width.max(1);
        let mut lines = Vec::new();
        let mut start = 0;
        for line in self.text.split('\n') {
            let mut from = start;
            let mut column = 0;
            let mut after_space = None;
            for (at, grapheme) in line.grapheme_indices(true) {
                let at = start + at;
                let space = grapheme.chars().all(char::is_whitespace);
                let columns = grapheme.width();
                while column + columns > width && at > from && !space {
                    let to = after_space.unwrap_or(at);
                    lines.push(from..to);
                    column = self.text.get(to..at).map_or(0, UnicodeWidthStr::width);
                    from = to;
                    after_space = None;
                }
                column += columns;
                if space {
                    after_space = Some(at + grapheme.len());
                }
            }
            start += line.len();
            lines.push(from..start);
            start += 1;
        }
        lines
    }

    /// The field after `edit`.
    #[must_use]
    pub fn edited(&self, edit: TextEdit) -> Self {
        match edit {
            TextEdit::Insert(c) => self.pasted(c.encode_utf8(&mut [0; 4])),
            TextEdit::Move { motion, select } => self.moved(motion, select),
            TextEdit::DeleteBackward => self.replaced(self.selection_or(self.left()), ""),
            TextEdit::DeleteForward => self.replaced(self.selection_or(self.right()), ""),
            TextEdit::DeleteWordBackward => self.replaced(self.selection_or(self.word_left()), ""),
            TextEdit::DeleteWordForward => self.replaced(self.selection_or(self.word_right()), ""),
            TextEdit::SelectAll => Self {
                caret: self.text.len(),
                anchor: Some(0),
                goal: None,
                ..self.clone()
            },
        }
    }

    /// The field with `text` typed in place of the selection, or at the caret. A multi-line
    /// field takes each line break as `\n`; a one-line field leaves line breaks out, as its one
    /// line cannot hold them. Unchanged when that leaves nothing to type.
    #[must_use]
    pub fn pasted(&self, text: &str) -> Self {
        let typed = match self.lines {
            Lines::One => text.chars().filter(|&c| !breaks_line(c)).collect(),
            Lines::Many => newlines(text),
        };
        if typed.is_empty() {
            return self.clone();
        }
        self.replaced(self.selection_or(self.caret), &typed)
    }

    /// The selection, or the bytes between the caret and `to` when nothing is selected.
    fn selection_or(&self, to: usize) -> Range<usize> {
        self.selection()
            .unwrap_or(to.min(self.caret)..to.max(self.caret))
    }

    /// The field with `range` replaced by `with` and the caret after it, or after the grapheme
    /// it joins with the text around it.
    fn replaced(&self, range: Range<usize>, with: &str) -> Self {
        let mut text = self.text.clone();
        text.replace_range(range.clone(), with);
        let after = range.start + with.len();
        let caret = iter::once(0)
            .chain(
                text.grapheme_indices(true)
                    .map(|(at, grapheme)| at + grapheme.len()),
            )
            .find(|&end| end >= after)
            .unwrap_or_default();
        Self {
            text,
            caret,
            anchor: None,
            goal: None,
            lines: self.lines,
        }
    }

    fn moved(&self, motion: Motion, select: bool) -> Self {
        let selection = self.selection().filter(|_| !select);
        let mut goal = None;
        let caret = match (motion, selection) {
            (Motion::Left, Some(selection)) => selection.start,
            (Motion::Right, Some(selection)) => selection.end,
            (Motion::Left, None) => self.left(),
            (Motion::Right, None) => self.right(),
            (Motion::WordLeft, _) => self.word_left(),
            (Motion::WordRight, _) => self.word_right(),
            (Motion::LineStart, _) => self.line_start(),
            (Motion::LineEnd, _) => self.line_end(self.caret),
            (Motion::Up { width } | Motion::Down { width }, _) => {
                let lines = self.screen_lines(width);
                let index = lines
                    .iter()
                    .rposition(|line| line.start <= self.caret)
                    .unwrap_or_default();
                let column = self.goal.unwrap_or_else(|| {
                    lines
                        .get(index)
                        .and_then(|line| self.text.get(line.start..self.caret))
                        .and_then(|before| grapheme_columns(before).last())
                        .map_or(0, |(_, column)| column)
                });
                goal = Some(column);
                let target = match motion {
                    Motion::Up { .. } => index.checked_sub(1),
                    _ => Some(index + 1),
                };
                match target.and_then(|target| Some((target, lines.get(target)?))) {
                    Some((target, line)) => {
                        let wraps = lines
                            .get(target + 1)
                            .is_some_and(|next| next.start == line.end);
                        in_line(&self.text, line.clone(), wraps, column)
                    }
                    None if matches!(motion, Motion::Up { .. }) => 0,
                    None => self.text.len(),
                }
            }
            (Motion::Start, _) => 0,
            (Motion::End, _) => self.text.len(),
            (Motion::To(at), _) => self.boundary(at),
        };
        Self {
            caret,
            anchor: select.then(|| self.anchor.unwrap_or(self.caret)),
            goal,
            ..self.clone()
        }
    }

    /// The start of the grapheme byte `at` falls in, or the end of the text when it is past it.
    fn boundary(&self, at: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .map(|(start, _)| start)
            .chain([self.text.len()])
            .take_while(|&start| start <= at)
            .last()
            .unwrap_or_default()
    }

    /// The start of the grapheme before the caret, or the caret at the start.
    fn left(&self) -> usize {
        self.text
            .grapheme_indices(true)
            .map(|(at, _)| at)
            .take_while(|&at| at < self.caret)
            .last()
            .unwrap_or(self.caret)
    }

    /// The end of the grapheme after the caret, or the caret at the end.
    fn right(&self) -> usize {
        self.text
            .grapheme_indices(true)
            .find(|&(at, _)| at >= self.caret)
            .map_or(self.caret, |(at, grapheme)| at + grapheme.len())
    }

    /// The start of the word before the caret, or the start of the text.
    fn word_left(&self) -> usize {
        self.text
            .unicode_word_indices()
            .map(|(at, _)| at)
            .take_while(|&at| at < self.caret)
            .last()
            .unwrap_or(0)
    }

    /// The end of the word after the caret, or the end of the text.
    fn word_right(&self) -> usize {
        self.text
            .unicode_word_indices()
            .map(|(at, word)| at + word.len())
            .find(|&end| end > self.caret)
            .unwrap_or(self.text.len())
    }

    /// The start of the caret's line.
    fn line_start(&self) -> usize {
        self.line_start_at(self.caret)
    }

    /// The start of the line byte `at` is on.
    fn line_start_at(&self, at: usize) -> usize {
        self.text
            .get(..at)
            .and_then(|before| before.rfind('\n'))
            .map_or(0, |newline| newline + 1)
    }

    /// The end of the line byte `at` is on, before its `\n`.
    fn line_end(&self, at: usize) -> usize {
        self.text
            .get(at..)
            .and_then(|after| after.find('\n'))
            .map_or(self.text.len(), |newline| at + newline)
    }
}

/// The last grapheme boundary of `line`, a screen line of `text`, at most `column` display
/// columns into it; never its end when the text `wraps` after it, since that is where the next
/// line starts.
fn in_line(text: &str, line: Range<usize>, wraps: bool, column: usize) -> usize {
    let mut stops: Vec<_> = grapheme_columns(text.get(line.clone()).unwrap_or_default()).collect();
    if wraps {
        stops.pop();
    }
    stops
        .into_iter()
        .take_while(|&(_, before)| before <= column)
        .last()
        .map_or(line.start, |(at, _)| line.start + at)
}

/// Each grapheme boundary of `text`, from its start to its end, as its byte offset and the
/// display columns before it (N-PORT-04).
pub fn grapheme_columns(text: &str) -> impl Iterator<Item = (usize, usize)> {
    let mut column = 0;
    iter::once((0, 0)).chain(text.grapheme_indices(true).map(move |(at, grapheme)| {
        column += grapheme.width();
        (at + grapheme.len(), column)
    }))
}

/// `text` with each line break as `\n`.
fn newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace(breaks_line, "\n")
}

/// Whether `c` ends a line wherever it stands, as the file format splits lines.
fn breaks_line(c: char) -> bool {
    matches!(c, '\r' | '\n' | '\u{C}' | '\u{2029}' | '\u{2B7F}')
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Up and down by the lines of the text, which never wrap.
    const UP: Motion = Motion::Up { width: usize::MAX };
    const DOWN: Motion = Motion::Down { width: usize::MAX };

    /// A one-line field written with `|` for the caret and `[` for the selection's anchor.
    fn field(marked: &str) -> TextField {
        let caret = marked.find('|').unwrap();
        let anchor = marked.find('[');
        let text: String = marked.chars().filter(|&c| c != '|' && c != '[').collect();
        let shift =
            |at: usize, other: Option<usize>| at - usize::from(other.is_some_and(|o| o < at));
        TextField {
            caret: shift(caret, anchor),
            anchor: anchor.map(|anchor| shift(anchor, Some(caret))),
            goal: None,
            lines: Lines::One,
            text,
        }
    }

    /// A multi-line field written as in [`field`].
    fn area(marked: &str) -> TextField {
        TextField {
            lines: Lines::Many,
            ..field(marked)
        }
    }

    /// The field written as in [`field`].
    fn marked(field: &TextField) -> String {
        let mut marks = vec![(field.caret, '|')];
        marks.extend(field.anchor.map(|anchor| (anchor, '[')));
        marks.sort_by_key(|&(at, mark)| (std::cmp::Reverse(at), mark == '['));
        let mut text = field.text.clone();
        for (at, mark) in marks {
            text.insert(at, mark);
        }
        text
    }

    #[track_caller]
    fn check(before: &str, edit: TextEdit, after: &str) {
        assert_eq!(
            marked(&field(before).edited(edit)),
            after,
            "{edit:?} on {before:?}"
        );
    }

    #[track_caller]
    fn check_area(before: &str, edit: TextEdit, after: &str) {
        assert_eq!(
            marked(&area(before).edited(edit)),
            after,
            "{edit:?} on {before:?}"
        );
    }

    /// The multi-line field after `edits` in turn.
    fn edited_area(before: &str, edits: &[TextEdit]) -> String {
        marked(
            &edits
                .iter()
                .fold(area(before), |field, &edit| field.edited(edit)),
        )
    }

    fn go(motion: Motion) -> TextEdit {
        TextEdit::Move {
            motion,
            select: false,
        }
    }

    fn select(motion: Motion) -> TextEdit {
        TextEdit::Move {
            motion,
            select: true,
        }
    }

    const FAMILY: &str = "👨‍👩‍👧";

    #[test]
    fn starts_with_the_caret_at_the_end_or_where_asked() {
        assert_eq!(marked(&TextField::new("ab".into())), "ab|");
        assert_eq!(marked(&TextField::new(String::new())), "|");
        assert_eq!(marked(&TextField::with_caret("ab".into(), 1)), "a|b");
        assert_eq!(marked(&TextField::with_caret("ab".into(), 9)), "ab|");
        assert_eq!(TextField::new("ab".into()).text(), "ab");
        assert_eq!(TextField::new("ab".into()).selection(), None);
    }

    // N-PORT-04
    #[test]
    fn a_caret_asked_for_inside_a_grapheme_starts_at_the_grapheme() {
        assert_eq!(
            marked(&TextField::with_caret(format!("a{FAMILY}b"), 3)),
            format!("a|{FAMILY}b")
        );
        assert_eq!(marked(&TextField::with_caret("é".into(), 1)), "|é");
    }

    // N-PORT-04
    #[test]
    fn typing_or_deleting_that_joins_graphemes_leaves_the_caret_after_the_joined_one() {
        check("|\u{301}", TextEdit::Insert('a'), "a\u{301}|");
        assert_eq!(
            edited_area(
                "x|👧",
                &[TextEdit::Insert('👨'), TextEdit::Insert('\u{200D}')]
            ),
            "x👨\u{200D}👧|"
        );
        check_area("a\n|\u{301}", TextEdit::DeleteBackward, "a\u{301}|");
    }

    #[test]
    fn a_multi_line_field_takes_each_line_break_as_a_newline() {
        let field = TextField::multi_line("a\r\nbc\rd");
        assert_eq!(field.text(), "a\nbc\nd");
        assert_eq!(marked(&field.edited(go(UP)).edited(go(UP))), "a|\nbc\nd");
    }

    #[test]
    fn a_multi_line_field_starts_with_the_caret_at_the_end() {
        let field = TextField::multi_line("ab\ncd");
        assert_eq!(marked(&field), "ab\ncd|");
        assert_eq!(field.column(), 2);
    }

    // FR-EDIT-04
    #[test]
    fn inserts_at_the_caret_or_over_the_selection() {
        check("a|b", TextEdit::Insert('x'), "ax|b");
        check("|", TextEdit::Insert('é'), "é|");
        check("a[bc|d", TextEdit::Insert('x'), "ax|d");
        check("a|bc[d", TextEdit::Insert('x'), "ax|d");
    }

    // FR-EDIT-04
    #[test]
    fn a_line_break_starts_a_new_line_only_in_a_multi_line_field() {
        check("a|b", TextEdit::Insert('\n'), "a|b");
        check("a[b|c", TextEdit::Insert('\r'), "a[b|c");
        check_area("a|b", TextEdit::Insert('\n'), "a\n|b");
        check_area("a[b|c", TextEdit::Insert('\r'), "a\n|c");
        check_area("|", TextEdit::Insert('\u{2029}'), "\n|");
    }

    // FR-EDIT-04
    #[test]
    fn pastes_at_the_caret_or_over_the_selection_without_line_breaks() {
        let pasted = |before: &str, text: &str| marked(&field(before).pasted(text));
        assert_eq!(pasted("a|b", "xy"), "axy|b");
        assert_eq!(pasted("a[bc|d", "x\r\ny\u{2029}z\n"), "axyz|d");
        assert_eq!(pasted("|", "日本"), "日本|");
        assert_eq!(
            pasted("a[bc|d", "\n"),
            "a[bc|d",
            "nothing typed, so nothing replaced"
        );
        assert_eq!(pasted("a[bc|d", ""), "a[bc|d");
    }

    // FR-EDIT-04
    #[test]
    fn pastes_into_a_multi_line_field_with_each_line_break_as_a_newline() {
        let pasted = |before: &str, text: &str| marked(&area(before).pasted(text));
        assert_eq!(pasted("a|b", "x\r\ny\rz\u{C}w\n"), "ax\ny\nz\nw\n|b");
        assert_eq!(pasted("a[bc|d", "x\n\ny"), "ax\n\ny|d");
        assert_eq!(pasted("a[bc|d", ""), "a[bc|d");
    }

    // FR-EDIT-04, N-PORT-04
    #[test]
    fn moves_by_grapheme() {
        check(
            &format!("a{FAMILY}|b"),
            go(Motion::Left),
            &format!("a|{FAMILY}b"),
        );
        check(
            &format!("a|{FAMILY}b"),
            go(Motion::Right),
            &format!("a{FAMILY}|b"),
        );
        check("e\u{301}|", go(Motion::Left), "|e\u{301}");
        check("|ab", go(Motion::Left), "|ab");
        check("ab|", go(Motion::Right), "ab|");
        check_area("a\n|b", go(Motion::Left), "a|\nb");
        check_area("a|\nb", go(Motion::Right), "a\n|b");
    }

    // FR-EDIT-04
    #[test]
    fn moves_by_word() {
        check("one two| three", go(Motion::WordLeft), "one |two three");
        check("one two |three", go(Motion::WordLeft), "one |two three");
        check("one t|wo three", go(Motion::WordLeft), "one |two three");
        check("  |one", go(Motion::WordLeft), "|  one");
        check("one| two, three", go(Motion::WordRight), "one two|, three");
        check("one tw|o three", go(Motion::WordRight), "one two| three");
        check("one two|, ", go(Motion::WordRight), "one two, |");
        check_area("one\n|two", go(Motion::WordLeft), "|one\ntwo");
        check_area("one|\ntwo", go(Motion::WordRight), "one\ntwo|");
    }

    // FR-EDIT-04
    #[test]
    fn moves_to_either_end_of_the_text() {
        check("ab|c", go(Motion::Start), "|abc");
        check("a|bc", go(Motion::End), "abc|");
        check_area("ab\nc|d", go(Motion::Start), "|ab\ncd");
        check_area("a|b\ncd", go(Motion::End), "ab\ncd|");
    }

    // FR-EDIT-04
    #[test]
    fn moves_to_either_end_of_the_line() {
        check("ab|c", go(Motion::LineStart), "|abc");
        check("a|bc", go(Motion::LineEnd), "abc|");
        check_area("ab\nc|d\nef", go(Motion::LineStart), "ab\n|cd\nef");
        check_area("ab\nc|d\nef", go(Motion::LineEnd), "ab\ncd|\nef");
        check_area("ab\n|\nef", go(Motion::LineEnd), "ab\n|\nef");
        check_area("ab\n|", go(Motion::LineStart), "ab\n|");
    }

    // FR-EDIT-04
    #[test]
    fn moves_up_and_down_to_the_same_column() {
        check_area("abc\nd|ef", go(UP), "a|bc\ndef");
        check_area("a|bc\ndef", go(DOWN), "abc\nd|ef");
        check_area("abc\ndef|", go(UP), "abc|\ndef");
        check_area(
            &format!("{FAMILY}a\nbc|"),
            go(UP),
            &format!("{FAMILY}|a\nbc"),
        );
    }

    // FR-EDIT-04, N-PORT-04
    #[test]
    fn moves_up_and_down_to_the_same_display_column() {
        check_area("日|a\nabcd", go(DOWN), "日a\nab|cd");
        check_area("ab|cd\n日日", go(DOWN), "abcd\n日|日");
        check_area("a|bcd\n日日", go(DOWN), "abcd\n|日日");
        assert_eq!(
            edited_area("日日|\na\nabcde", &[go(DOWN), go(DOWN)]),
            "日日\na\nabcd|e",
            "the display column holds past a shorter line"
        );
        let down = Motion::Down { width: 4 };
        check_area("abcd|\n日日日", go(down), "abcd\n日|日日");
    }

    // FR-EDIT-04
    #[test]
    fn moving_up_or_down_to_a_shorter_line_stops_at_its_end() {
        check_area("a\nbc|", go(UP), "a|\nbc");
        check_area("abc|\n\nd", go(DOWN), "abc\n|\nd");
    }

    // FR-EDIT-04
    #[test]
    fn moving_up_or_down_past_a_shorter_line_keeps_the_column() {
        let (up, down) = (go(UP), go(DOWN));
        assert_eq!(edited_area("abc|\nd\nefg", &[down, down]), "abc\nd\nefg|");
        assert_eq!(edited_area("abc\nd\nef|g", &[up, up]), "ab|c\nd\nefg");
        assert_eq!(
            edited_area("abc|\nd\nefg", &[down, go(Motion::Left), down]),
            "abc\nd\n|efg",
            "another motion sets a new column"
        );
        assert_eq!(
            edited_area("abc|\nd\nefg", &[down, TextEdit::Insert('x'), down]),
            "abc\ndx\nef|g",
            "an edit sets a new column"
        );
    }

    // FR-EDIT-04
    #[test]
    fn moving_up_from_the_first_line_or_down_from_the_last_goes_to_the_end() {
        check_area("a|b\ncd", go(UP), "|ab\ncd");
        check_area("ab\nc|d", go(DOWN), "ab\ncd|");
        check("a|b", go(UP), "|ab");
        check("a|b", go(DOWN), "ab|");
        check("a|b", select(DOWN), "a[b|");
    }

    /// The screen lines of `text` in `width` columns, as text.
    fn screen(text: &str, width: usize) -> Vec<&str> {
        TextField::multi_line(text)
            .screen_lines(width)
            .into_iter()
            .map(|line| &text[line])
            .collect()
    }

    // FR-UI-05
    #[test]
    fn lines_wrap_between_words_and_inside_words_too_long_for_a_line() {
        assert_eq!(screen("one two three", 6), ["one ", "two ", "three"]);
        assert_eq!(screen("one two three", 7), ["one two ", "three"]);
        assert_eq!(screen("abcdefgh ij", 3), ["abc", "def", "gh ", "ij"]);
        assert_eq!(
            screen("a  b", 1),
            ["a  ", "b"],
            "spaces stay at the end of a line"
        );
        assert_eq!(screen("ab\n\ncd", 9), ["ab", "", "cd"]);
        assert_eq!(screen("", 9), [""]);
        assert_eq!(screen("ab", 0), ["a", "b"], "a line is at least a column");
        assert_eq!(
            screen("日本語", 4),
            ["日本", "語"],
            "by columns, not graphemes"
        );
        assert_eq!(screen(&format!("a{FAMILY}b"), 2), ["a", FAMILY, "b"]);
    }

    // FR-EDIT-04, FR-UI-05
    #[test]
    fn moves_up_and_down_by_lines_on_screen() {
        let (up, down) = (Motion::Up { width: 4 }, Motion::Down { width: 4 });
        check_area("ab c|d ef\ngh", go(up), "a|b cd ef\ngh");
        check_area("a|b cd ef\ngh", go(down), "ab c|d ef\ngh");
        check_area("ab cd e|f\ngh", go(down), "ab cd ef\ng|h");
        check_area("ab cd ef\ng|h", go(up), "ab cd e|f\ngh");
        check_area("ab|c defg", go(down), "abc de|fg");
        // The end of a line that wraps is where the next one starts, so the caret stops before
        // it.
        check_area("ab cdef|", go(up), "ab| cdef");
        check_area("a|b cd", select(down), "a[b c|d");
        assert_eq!(
            edited_area("abc|d e\nfghi", &[go(down), go(down)]),
            "abcd e\nfgh|i",
            "the column holds past a shorter line"
        );
    }

    #[test]
    fn moves_to_an_offset_at_the_start_of_its_grapheme() {
        check("ab|c", go(Motion::To(0)), "|abc");
        check("|abc", go(Motion::To(2)), "ab|c");
        check("|abc", go(Motion::To(3)), "abc|");
        check("|abc", go(Motion::To(9)), "abc|");
        check(
            &format!("|a{FAMILY}b"),
            go(Motion::To(3)),
            &format!("a|{FAMILY}b"),
        );
        check("a[b|c", go(Motion::To(3)), "abc|");
        check("a|bc", select(Motion::To(3)), "a[bc|");
    }

    // FR-EDIT-04
    #[test]
    fn shift_motions_extend_the_selection() {
        check("ab|c", select(Motion::Left), "a|b[c");
        check("a|b[c", select(Motion::Left), "|ab[c");
        check("a|b[c", select(Motion::End), "ab[c|");
        check("one two|", select(Motion::WordLeft), "one |two[");
        check_area("ab\nc|d", select(UP), "a|b\nc[d");
        check_area("ab\nc|d", select(Motion::LineStart), "ab\n|c[d");
        check("ab|", TextEdit::SelectAll, "[ab|");
        check_area("a|\nb", TextEdit::SelectAll, "[a\nb|");
        assert_eq!(field("a[bc|d").selection(), Some(1..3));
        assert_eq!(field("a|bc[d").selection(), Some(1..3));
        assert_eq!(field("ab|").selection(), None);
        assert_eq!(field("a[|b").selection(), None, "an empty selection");
    }

    // FR-EDIT-04
    #[test]
    fn plain_motions_collapse_the_selection() {
        check("a[bc|d", go(Motion::Left), "a|bcd");
        check("a|bc[d", go(Motion::Right), "abc|d");
        check("a[bc|d", go(Motion::End), "abcd|");
        check("a[bc|d", go(Motion::WordLeft), "|abcd");
        check_area("ab\nc|d[e", go(UP), "a|b\ncde");
    }

    // FR-EDIT-04
    #[test]
    fn gives_the_selected_text_for_the_clipboard() {
        assert_eq!(field("a[bc|d").selected_text(), Some("bc"));
        assert_eq!(area("a|b\nc[d").selected_text(), Some("b\nc"));
        assert_eq!(field("ab|").selected_text(), None);
    }

    // FR-EDIT-04, N-PORT-04
    #[test]
    fn deletes_a_grapheme_or_the_selection() {
        check("ae\u{301}|b", TextEdit::DeleteBackward, "a|b");
        check("a|e\u{301}b", TextEdit::DeleteForward, "a|b");
        check("|ab", TextEdit::DeleteBackward, "|ab");
        check("ab|", TextEdit::DeleteForward, "ab|");
        check("a[bc|d", TextEdit::DeleteBackward, "a|d");
        check("a|bc[d", TextEdit::DeleteForward, "a|d");
        check_area("a\n|b", TextEdit::DeleteBackward, "a|b");
        check_area("a|\nb", TextEdit::DeleteForward, "a|b");
    }

    // FR-EDIT-04
    #[test]
    fn deletes_back_to_the_start_of_a_word() {
        check("one two|", TextEdit::DeleteWordBackward, "one |");
        check("one two  |x", TextEdit::DeleteWordBackward, "one |x");
        check("|one", TextEdit::DeleteWordBackward, "|one");
        check("on[e tw|o", TextEdit::DeleteWordBackward, "on|o");
    }

    // FR-EDIT-04
    #[test]
    fn deletes_up_to_the_end_of_a_word() {
        check("|one two", TextEdit::DeleteWordForward, "| two");
        check("one|  two x", TextEdit::DeleteWordForward, "one| x");
        check("one|", TextEdit::DeleteWordForward, "one|");
        check("on[e tw|o", TextEdit::DeleteWordForward, "on|o");
    }

    #[test]
    fn counts_and_places_the_caret_by_grapheme_column() {
        assert_eq!(field(&format!("a{FAMILY}|b")).column(), 2);
        assert_eq!(field("|ab").column(), 0);
        assert_eq!(area("ab\nc|d").column(), 1);
        assert_eq!(area("ab\n|").column(), 0);
        assert_eq!(
            marked(&TextField::at_column(format!("a{FAMILY}b"), 2)),
            format!("a{FAMILY}|b")
        );
        assert_eq!(marked(&TextField::at_column("ab".into(), 0)), "|ab");
        assert_eq!(marked(&TextField::at_column("ab".into(), 5)), "ab|");
    }

    fn any_edit() -> impl Strategy<Value = TextEdit> {
        let motion = prop_oneof![
            Just(Motion::Left),
            Just(Motion::Right),
            Just(Motion::WordLeft),
            Just(Motion::WordRight),
            Just(Motion::LineStart),
            Just(Motion::LineEnd),
            Just(UP),
            Just(DOWN),
            (1..6usize).prop_map(|width| Motion::Up { width }),
            (1..6usize).prop_map(|width| Motion::Down { width }),
            Just(Motion::Start),
            Just(Motion::End),
            (0..40usize).prop_map(Motion::To),
        ];
        prop_oneof![
            prop::sample::select(vec![
                'a', ' ', 'é', '\u{301}', '\n', '\r', '日', '👨', '\u{200D}'
            ])
            .prop_map(TextEdit::Insert),
            (motion, any::<bool>()).prop_map(|(motion, select)| TextEdit::Move { motion, select }),
            Just(TextEdit::DeleteBackward),
            Just(TextEdit::DeleteForward),
            Just(TextEdit::DeleteWordBackward),
            Just(TextEdit::DeleteWordForward),
            Just(TextEdit::SelectAll),
        ]
    }

    fn is_boundary(text: &str, at: usize) -> bool {
        at == text.len() || text.grapheme_indices(true).any(|(start, _)| start == at)
    }

    proptest! {
        // N-PORT-04
        #[test]
        fn caret_and_selection_never_split_a_grapheme(
            many in any::<bool>(),
            start in "[ab\u{301}\u{200D}👨 \n\r]{0,12}",
            edits in prop::collection::vec(any_edit(), 0..40),
        ) {
            let field = if many {
                TextField::multi_line(&start)
            } else {
                TextField::new(start.replace(breaks_line, ""))
            };
            let mut field = field;
            for edit in edits {
                field = field.edited(edit);
                prop_assert!(is_boundary(field.text(), field.caret()), "{edit:?}: {field:?}");
                if let Some(selection) = field.selection() {
                    prop_assert!(is_boundary(field.text(), selection.start), "{field:?}");
                    prop_assert!(is_boundary(field.text(), selection.end), "{field:?}");
                }
                prop_assert!(many || !field.text().contains(breaks_line), "{field:?}");
            }
        }

        // FR-UI-05
        #[test]
        fn screen_lines_hold_the_text_in_order_each_within_its_width(
            text in "[ab \u{301}日\n]{0,16}",
            width in 1..6usize,
        ) {
            let mut end = 0;
            for line in TextField::multi_line(&text).screen_lines(width) {
                let gap = &text[end..line.start];
                prop_assert!(gap.is_empty() || gap == "\n", "{line:?} after {end}");
                let shown = text[line.clone()].trim_end();
                prop_assert!(
                    shown.width() <= width || shown.graphemes(true).count() == 1,
                    "{shown:?}"
                );
                end = line.end;
            }
            prop_assert_eq!(end, text.len());
        }
    }
}
