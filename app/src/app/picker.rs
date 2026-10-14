//! The fuzzy picker every modal window that chooses an item shares, such as the Navigator or Move
//! to: the query typed into it, the candidates it matches in ranked order, the focused one and
//! what each input does to them (FR-UI-09). Hides how candidates are matched and ranked.

use std::{cmp::Reverse, mem};

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
};

use super::{TextEdit, TextField};

/// Something a picker can choose, as it lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate<T> {
    /// What choosing it gives.
    pub value: T,
    /// The text the query matches.
    pub text: String,
    /// What a frontend shows beside the text to tell candidates apart, such as the area a
    /// project is in; empty when nothing needs telling apart.
    pub detail: String,
}

/// What the user does to an open picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerInput {
    /// Change the query.
    Edit(TextEdit),
    /// Focus the match above.
    Previous,
    /// Focus the match below.
    Next,
    /// Choose the focused match.
    Choose,
    /// Close without choosing.
    Dismiss,
}

/// How an input closed a picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picked<T> {
    /// This value was chosen.
    Chosen(T),
    /// The picker was closed without choosing.
    Dismissed,
}

/// An open picker as a frontend shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickerView<'a, T> {
    /// The query, with its caret and selection.
    pub query: &'a TextField,
    /// The candidates the query matches, best first.
    pub entries: Entries<'a, T>,
    /// The position of the focused entry in `entries`; 0 when there are none.
    pub focused: usize,
}

/// The candidates a query matches, best first, read one at a time, since a picker over a large
/// file matches many more than show (N-PERF-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entries<'a, T> {
    candidates: &'a [Candidate<T>],
    /// The positions in `candidates` of those matched, best first.
    ranked: &'a [usize],
}

impl<'a, T> Entries<'a, T> {
    /// How many there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ranked.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranked.is_empty()
    }

    /// Every entry, best first.
    pub fn iter(&self) -> impl Iterator<Item = &'a Candidate<T>> + use<'a, T> {
        let candidates = self.candidates;
        self.ranked
            .iter()
            .filter_map(move |&index| candidates.get(index))
    }
}

/// An open picker: its candidates, the query, the matching candidates ranked and the focused one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picker<T> {
    candidates: Vec<Candidate<T>>,
    query: TextField,
    /// The positions in `candidates` of those the query matches, best first.
    ranked: Vec<usize>,
    focused: usize,
}

impl<T: Clone> Picker<T> {
    /// A picker over `candidates` with an empty query, which lists them all in the order given,
    /// the first focused.
    #[must_use]
    pub fn new(candidates: Vec<Candidate<T>>) -> Self {
        Self {
            ranked: (0..candidates.len()).collect(),
            candidates,
            query: TextField::new(String::new()),
            focused: 0,
        }
    }

    /// The picker as a frontend shows it.
    #[must_use]
    pub fn view(&self) -> PickerView<'_, T> {
        PickerView {
            query: &self.query,
            entries: Entries {
                candidates: &self.candidates,
                ranked: &self.ranked,
            },
            focused: self.focused,
        }
    }

    /// Lists `candidates` instead, matched against the query as typed, the first match focused.
    pub fn relist(&mut self, candidates: Vec<Candidate<T>>) {
        self.candidates = candidates;
        self.rank(false);
        self.focused = 0;
    }

    /// Focuses the first entry listed whose value satisfies `predicate`, and says whether there
    /// is one; without one the focus stays where it was.
    pub fn focus_where(&mut self, predicate: impl Fn(&T) -> bool) -> bool {
        let candidates = &self.candidates;
        let found = self.ranked.iter().position(|&index| {
            candidates
                .get(index)
                .is_some_and(|candidate| predicate(&candidate.value))
        });
        if let Some(position) = found {
            self.focused = position;
        }
        found.is_some()
    }

    /// Applies `input`, and returns how it closed the picker, or `None` while it stays open.
    pub fn input(&mut self, input: PickerInput) -> Option<Picked<T>> {
        match input {
            PickerInput::Edit(edit) => self.edit(edit),
            PickerInput::Previous => self.focused = self.focused.saturating_sub(1),
            PickerInput::Next => {
                self.focused = (self.focused + 1).min(self.ranked.len().saturating_sub(1));
            }
            PickerInput::Choose => {
                let index = *self.ranked.get(self.focused)?;
                let candidate = self.candidates.get(index)?;
                return Some(Picked::Chosen(candidate.value.clone()));
            }
            PickerInput::Dismiss => return Some(Picked::Dismissed),
        }
        None
    }

    /// Applies `edit` to the query and, when it changes the text, ranks the candidates again and
    /// focuses the first match. Text typed at the end can only narrow the matches, so only those
    /// are gone through (N-PERF-02), unless it follows a backslash, which with a space after it
    /// stops asking for the backslash.
    fn edit(&mut self, edit: TextEdit) {
        let edited = self.query.edited(edit);
        let before = mem::replace(&mut self.query, edited);
        let (before, after) = (before.text(), self.query.text());
        if before == after {
            return;
        }
        let narrowed = after.starts_with(before) && !before.ends_with('\\');
        self.rank(narrowed);
        self.focused = 0;
    }

    /// Ranks the candidates the query matches: by how well their text matches, then in the order
    /// given. Each word of the query must match, fuzzily, ignoring case unless it has a capital
    /// and ignoring accents unless it has one; characters such as `!` or `^` match themselves.
    /// With `narrowed`, only those matched before are gone through.
    fn rank(&mut self, narrowed: bool) {
        let pattern = Pattern::new(
            self.query.text(),
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut buffer = Vec::new();
        let candidates = &self.candidates;
        let mut score = |index: usize| {
            let text = Utf32Str::new(&candidates.get(index)?.text, &mut buffer);
            Some((Reverse(pattern.score(text, &mut matcher)?), index))
        };
        let mut scored: Vec<_> = if narrowed {
            self.ranked
                .iter()
                .filter_map(|&index| score(index))
                .collect()
        } else {
            (0..candidates.len()).filter_map(score).collect()
        };
        scored.sort_unstable();
        self.ranked = scored.into_iter().map(|(_, index)| index).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Motion;

    fn candidate(value: u8, text: &str) -> Candidate<u8> {
        Candidate {
            value,
            text: text.to_owned(),
            detail: format!("detail {value}"),
        }
    }

    fn over(texts: &[&str]) -> Picker<u8> {
        Picker::new(
            (1..)
                .zip(texts)
                .map(|(value, text)| candidate(value, text))
                .collect(),
        )
    }

    fn sample() -> Picker<u8> {
        over(&["Write the report", "Reports", "Buy milk", "Read later"])
    }

    /// The values listed, best first, and the focused position.
    fn listed(picker: &Picker<u8>) -> (Vec<u8>, usize) {
        let view = picker.view();
        let values = view.entries.iter().map(|entry| entry.value).collect();
        (values, view.focused)
    }

    fn typed(picker: &mut Picker<u8>, text: &str) {
        for c in text.chars() {
            assert_eq!(picker.input(PickerInput::Edit(TextEdit::Insert(c))), None);
        }
    }

    fn edit(picker: &mut Picker<u8>, edit: TextEdit) {
        assert_eq!(picker.input(PickerInput::Edit(edit)), None);
    }

    fn go(motion: Motion) -> TextEdit {
        TextEdit::Move {
            motion,
            select: false,
        }
    }

    // FR-UI-09
    #[test]
    fn an_empty_query_lists_every_candidate_in_order_the_first_focused() {
        let picker = sample();
        assert_eq!(listed(&picker), (vec![1, 2, 3, 4], 0));
        let view = picker.view();
        assert_eq!(view.query.text(), "");
        assert_eq!(view.entries.len(), 4);
        assert!(!view.entries.is_empty());
        assert_eq!(
            view.entries.iter().next(),
            Some(&candidate(1, "Write the report"))
        );
    }

    #[test]
    fn focusing_where_a_value_matches_focuses_the_first_listed() {
        let mut picker = sample();
        assert!(picker.focus_where(|&value| value > 2));
        assert_eq!(listed(&picker).1, 2);
        assert!(!picker.focus_where(|&value| value > 9));
        assert_eq!(listed(&picker).1, 2, "without a match the focus stays");
        typed(&mut picker, "rep");
        assert!(picker.focus_where(|&value| value == 2));
        assert_eq!(listed(&picker).1, 1, "among the matches");
    }

    #[test]
    fn a_picker_over_nothing_lists_nothing_and_chooses_nothing() {
        let mut picker = over(&[]);
        assert_eq!(listed(&picker), (vec![], 0));
        assert!(picker.view().entries.is_empty());
        assert_eq!(picker.input(PickerInput::Next), None);
        assert_eq!(listed(&picker).1, 0);
        assert_eq!(picker.input(PickerInput::Choose), None);
    }

    // FR-UI-09
    #[test]
    fn typing_keeps_the_fuzzy_matches_best_first() {
        let mut picker = sample();
        typed(&mut picker, "rep");
        assert_eq!(listed(&picker).0, [1, 2]);
        assert_eq!(picker.view().query.text(), "rep");
        let mut picker = sample();
        typed(&mut picker, "wrr");
        assert_eq!(listed(&picker).0, [1], "letters in order, not together");
        let mut picker = sample();
        typed(&mut picker, "milk buy");
        assert_eq!(listed(&picker).0, [3], "each word matches on its own");
        let mut picker = sample();
        typed(&mut picker, "zz");
        assert_eq!(listed(&picker), (vec![], 0));
        assert_eq!(picker.input(PickerInput::Choose), None, "nothing to choose");
    }

    #[test]
    fn a_better_match_ranks_first_and_equal_matches_keep_the_order_given() {
        let mut picker = over(&["a b c d e f g h", "abc"]);
        typed(&mut picker, "abc");
        assert_eq!(listed(&picker).0, [2, 1]);
        let mut picker = over(&["b one", "a one", "c one"]);
        typed(&mut picker, "one");
        assert_eq!(listed(&picker).0, [1, 2, 3]);
    }

    #[test]
    fn a_capital_in_the_query_matches_case_and_an_accent_matches_accents() {
        let mut picker = sample();
        typed(&mut picker, "r");
        assert_eq!(listed(&picker).0.len(), 3, "r is in all but Buy milk");
        let mut picker = sample();
        typed(&mut picker, "R");
        assert_eq!(listed(&picker).0, [2, 4]);
        let mut picker = picker_of_cafes();
        typed(&mut picker, "cafe");
        assert_eq!(listed(&picker).0, [1, 2]);
        let mut picker = picker_of_cafes();
        typed(&mut picker, "café");
        assert_eq!(listed(&picker).0, [2]);
    }

    fn picker_of_cafes() -> Picker<u8> {
        over(&["Cafe", "Café"])
    }

    #[test]
    fn operator_characters_and_wide_text_match_themselves() {
        let mut picker = over(&["!important", "plain", "日本語のメモ", "👋🏽 hello"]);
        typed(&mut picker, "!imp");
        assert_eq!(listed(&picker).0, [1]);
        let mut picker = over(&["!important", "plain", "日本語のメモ", "👋🏽 hello"]);
        typed(&mut picker, "本メ");
        assert_eq!(listed(&picker).0, [3]);
    }

    // FR-UI-09, N-PERF-02
    #[test]
    fn typing_on_ranks_as_the_whole_query_typed_at_once() {
        let texts = ["ab cd", "cd ab", "a\\b", "a b", "Abc", "xaybxc", "abc"];
        for query in ["abc", "ab c", "aB", "a\\ ", "a\\ b", "a\\b", "cd ab"] {
            let mut typed_on = over(&texts);
            typed(&mut typed_on, query);
            let mut at_once = over(&texts);
            at_once.query = TextField::new(query.to_owned());
            at_once.rank(false);
            assert_eq!(listed(&typed_on).0, listed(&at_once).0, "{query:?}");
        }
        let mut picker = over(&texts);
        typed(&mut picker, "a\\ b");
        assert_eq!(
            listed(&picker).0,
            [4],
            "a space after a backslash matches one"
        );
    }

    // FR-UI-09
    #[test]
    fn previous_and_next_move_within_the_matches_and_stop_at_the_ends() {
        let mut picker = sample();
        assert_eq!(picker.input(PickerInput::Previous), None);
        assert_eq!(listed(&picker).1, 0);
        for _ in 0..9 {
            assert_eq!(picker.input(PickerInput::Next), None);
        }
        assert_eq!(listed(&picker).1, 3);
        assert_eq!(picker.input(PickerInput::Previous), None);
        assert_eq!(picker.input(PickerInput::Choose), Some(Picked::Chosen(3)));
    }

    // FR-UI-09
    #[test]
    fn changing_the_query_focuses_the_first_match() {
        let mut picker = sample();
        picker.input(PickerInput::Next);
        typed(&mut picker, "e");
        assert_eq!(listed(&picker).1, 0);
        picker.input(PickerInput::Next);
        edit(&mut picker, TextEdit::DeleteBackward);
        assert_eq!(listed(&picker), (vec![1, 2, 3, 4], 0), "erasing widens");
    }

    #[test]
    fn edits_that_leave_the_text_as_it_was_keep_the_focus() {
        let mut picker = sample();
        typed(&mut picker, "e");
        picker.input(PickerInput::Next);
        for unchanged in [
            go(Motion::Left),
            go(Motion::Start),
            TextEdit::DeleteBackward,
            TextEdit::SelectAll,
            TextEdit::Insert('\n'),
        ] {
            edit(&mut picker, unchanged);
            assert_eq!(listed(&picker).1, 1, "{unchanged:?}");
        }
        assert_eq!(picker.view().query.text(), "e");
        assert_eq!(picker.view().query.selection(), Some(0..1));
    }

    #[test]
    fn typing_before_the_end_of_the_query_ranks_every_candidate_again() {
        let mut picker = over(&["ba", "ab"]);
        typed(&mut picker, "ab");
        assert_eq!(listed(&picker).0, [2]);
        edit(&mut picker, go(Motion::Left));
        typed(&mut picker, " ");
        assert_eq!(picker.view().query.text(), "a b");
        assert_eq!(listed(&picker).0.len(), 2, "two words match more than one");
    }

    // FR-UI-09
    #[test]
    fn choosing_gives_the_focused_value_and_dismissing_gives_none() {
        let mut picker = sample();
        typed(&mut picker, "rep");
        picker.input(PickerInput::Next);
        assert_eq!(picker.input(PickerInput::Choose), Some(Picked::Chosen(2)));
        assert_eq!(
            sample().input(PickerInput::Dismiss),
            Some(Picked::Dismissed)
        );
    }

    // FR-UI-09
    #[test]
    fn relisting_keeps_the_query_and_ranks_the_new_candidates() {
        let mut picker = sample();
        typed(&mut picker, "re");
        picker.input(PickerInput::Next);
        picker.relist(vec![candidate(9, "Recipes"), candidate(8, "Groceries")]);
        assert_eq!(picker.view().query.text(), "re");
        assert_eq!(listed(&picker), (vec![9, 8], 0));
    }
}
