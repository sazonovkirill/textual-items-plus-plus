//! Full-text search (FR-SRCH-01–03): the words a query looks for, the items holding them all in
//! their titles, descriptions, notes, comments, tags and checklists, and the line outside the
//! title a word was found in, for a frontend to show. Hides how text is matched.

use std::ops::Range;

use super::{
    State, TextEdit, TextField,
    container::Row,
    view::{Group, View},
};
use crate::domain::{Document, Item, ItemId, ItemKind, Node, Tag, comments};

/// What the user does while typing the words to search for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchInput {
    /// Changes the words, and so the results, focusing the first.
    Edit(TextEdit),
    /// Stops typing, leaving the keys to the results.
    Done,
}

/// The words typed into a search, each found in any case anywhere in an item's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The words, lowercase.
    words: Vec<String>,
}

/// Where in an item a line outside its title comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Its description.
    Description,
    /// Its note.
    Note,
    /// One of its comments.
    Comment,
    /// An item of its checklist.
    Checklist,
}

/// A line of an item, outside its title and tags, holding a word searched for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Excerpt {
    /// Where it comes from.
    pub source: Source,
    /// The line, a comment without its timestamp.
    pub text: String,
}

impl Query {
    /// The query for `text`: the words it holds, split at whitespace.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            words: text.split_whitespace().map(folded).collect(),
        }
    }

    /// Where in `text` the words are found, in order, overlapping ones merged.
    #[must_use]
    pub fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        let mut lower = String::with_capacity(text.len());
        // The start of the character of `text` each byte of `lower` comes from.
        let mut origins = Vec::with_capacity(text.len());
        for (start, c) in text.char_indices() {
            for low in c.to_lowercase() {
                lower.push(low);
                origins.resize(lower.len(), start);
            }
        }
        let mut found: Vec<Range<usize>> = self
            .words
            .iter()
            .flat_map(|word| lower.match_indices(word.as_str()))
            .filter_map(|(start, word)| {
                let first = *origins.get(start)?;
                let last = *origins.get(start + word.len() - 1)?;
                Some(first..last + text.get(last..)?.chars().next()?.len_utf8())
            })
            .collect();
        found.sort_by_key(|range| range.start);
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(found.len());
        for range in found {
            match merged.last_mut() {
                Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
                _ => merged.push(range),
            }
        }
        merged
    }

    /// The first line of item `id`, outside its title and tags, holding a word they do not:
    /// from its description, its note, its comments, then its checklist; `None` when its title
    /// and tags hold every word.
    #[must_use]
    pub fn excerpt(&self, document: &Document, id: ItemId) -> Option<Excerpt> {
        let item = document.node(id).and_then(Node::item)?;
        let named = folded(&named(item));
        let missing: Vec<&str> = self
            .words
            .iter()
            .map(String::as_str)
            .filter(|word| !named.contains(word))
            .collect();
        lines(document, id, item).find_map(|(source, line)| {
            let lower = folded(line);
            missing
                .iter()
                .any(|word| lower.contains(word))
                .then(|| Excerpt {
                    source,
                    text: line.to_owned(),
                })
        })
    }

    /// Whether item `id` holds every word in its text. A word holds no whitespace, so it is found
    /// within the title, one tag, the description, the note, one comment or one checklist item,
    /// each looked through as it is (N-PERF-02).
    fn finds(&self, document: &Document, id: ItemId, item: &Item) -> bool {
        let mut checklist = Vec::new();
        checklist_titles(document, id, &mut checklist);
        self.words.iter().all(|word| {
            holds(&item.title, word)
                || (item.shown_tags().iter()).any(|tag| holds(&named_tag(tag), word))
                || holds(&item.description, word)
                || holds(&item.note, word)
                || (item.comments.iter()).any(|comment| holds(comments::split(comment).1, word))
                || checklist.iter().any(|title| holds(title, word))
        })
    }
}

impl State {
    /// The words searched for, while they are typed (FR-SRCH-01).
    #[must_use]
    pub fn search(&self) -> Option<&TextField> {
        self.searching.then_some(&self.pane().query)
    }

    /// Shows the pane's search results and types its words, selected so that typing replaces
    /// them, giving the pane the keys from the sidebar (FR-SRCH-01).
    pub(super) fn open_search(&mut self) {
        if self.sidebar_has_keys() {
            self.focus_right();
        }
        self.go(super::Go::View(View::Search));
        let pane = self.pane_mut();
        pane.query = pane.query.edited(TextEdit::SelectAll);
        self.searching = true;
    }

    /// Applies `input` to the words typed: the results follow each change, focused on the first.
    /// Nothing happens while they are not typed.
    pub(super) fn search_input(&mut self, input: SearchInput) {
        if !self.searching {
            return;
        }
        match input {
            SearchInput::Edit(edit) => {
                let pane = self.pane_mut();
                pane.query = pane.query.edited(edit);
                let first = self.listed().first().map(|row| row.id());
                let selection = &mut self.pane_mut().selection;
                selection.clear();
                selection.focus(first);
            }
            SearchInput::Done => self.searching = false,
        }
    }

    /// Stops typing the words searched for, as any command does.
    pub(super) fn finish_searching(&mut self) {
        self.searching = false;
    }
}

/// The one group search results show for `query` in `document`, without a heading: every task,
/// section and project holding all of its words, in file order, the completed ones, which show
/// whatever Show Completed says, last (FR-SRCH-03). Items in the Trash and the contents of a
/// completed item are left out (FR-ORG-09, OQ-14); a checklist item is found as part of its
/// task (FR-CHK-04). With no words, or nothing found, there is no group.
#[must_use]
pub(super) fn groups(document: &Document, query: &str) -> Vec<Group> {
    let query = Query::new(query);
    if query.words.is_empty() {
        return Vec::new();
    }
    let mut found = Vec::new();
    let tops = document
        .children(document.inbox())
        .iter()
        .chain(document.top_level());
    for &id in tops {
        visit(document, &query, id, &mut found);
    }
    // Stable, so the open and the completed each keep the file's order.
    found.sort_by_key(|&(completed, _)| completed);
    let rows: Vec<Row> = found.into_iter().map(|(_, row)| row).collect();
    if rows.is_empty() {
        return Vec::new();
    }
    vec![Group {
        heading: None,
        rows,
    }]
}

/// Whether every item `to` finds is one `from` finds too, as when words are typed on: each word
/// of `from` is part of a word of `to`. Then the results for `to` are those for `from` that it
/// finds, which [`refined`] picks without walking the whole document again (N-PERF-02).
pub(super) fn narrows(from: &str, to: &str) -> bool {
    let (from, to) = (Query::new(from), Query::new(to));
    !from.words.is_empty()
        && from
            .words
            .iter()
            .all(|word| to.words.iter().any(|longer| longer.contains(word.as_str())))
}

/// The rows of `rows`, results of a query that [`narrows`] to `query`, that `query` finds.
pub(super) fn refined(document: &Document, query: &str, rows: &[Row]) -> Vec<Row> {
    let query = Query::new(query);
    rows.iter()
        .copied()
        .filter(|row| {
            let id = row.id();
            document
                .node(id)
                .and_then(Node::item)
                .is_some_and(|item| query.finds(document, id, item))
        })
        .collect()
}

/// Adds `id`, when `query` finds it, and what it holds that shows to `found`, with whether each
/// is completed.
fn visit(document: &Document, query: &Query, id: ItemId, found: &mut Vec<(bool, Row)>) {
    let Some(item) = document.node(id).and_then(Node::item) else {
        return;
    };
    if item.deleted.is_some() {
        return;
    }
    let row = match item.kind {
        ItemKind::Task => Some(Row::Task { id, section: None }),
        ItemKind::Area | ItemKind::Section | ItemKind::Project => Some(Row::Project(id)),
        ItemKind::Inbox | ItemKind::ChecklistItem => None,
    };
    if let Some(row) = row
        && query.finds(document, id, item)
    {
        found.push((item.completion().is_some(), row));
    }
    if item.completion().is_none() {
        for &child in document.children(id) {
            visit(document, query, child, found);
        }
    }
}

/// `item`'s title and its tags as a row shows them.
fn named(item: &Item) -> String {
    let mut text = item.title.clone();
    for tag in item.shown_tags() {
        text.push(' ');
        text.push_str(&named_tag(tag));
    }
    text
}

/// `tag` as a row shows it: `@name`, or `@name(value)`.
fn named_tag(tag: &Tag) -> String {
    match &tag.value {
        Some(value) => format!("@{}({value})", tag.name),
        None => format!("@{}", tag.name),
    }
}

/// Whether `text` holds `word`, a lowercase one, in any case: as [`folded`] would find it, without
/// making a lowercase copy of text that is all ASCII.
fn holds(text: &str, word: &str) -> bool {
    if !text.is_ascii() {
        return folded(text).contains(word);
    }
    let text = text.as_bytes();
    word.as_bytes().split_first().is_none_or(|(&first, rest)| {
        memchr::memchr2_iter(first, first.to_ascii_uppercase(), text).any(|at| {
            text.get(at + 1..at + 1 + rest.len())
                .is_some_and(|after| after.eq_ignore_ascii_case(rest))
        })
    })
}

/// The lines of item `id` outside its title and tags, in the order an excerpt takes them: its
/// description's, its note's, its comments without their timestamps, then the titles of its
/// checklist's items, nested ones after the item they are in.
fn lines<'a>(
    document: &'a Document,
    id: ItemId,
    item: &'a Item,
) -> impl Iterator<Item = (Source, &'a str)> {
    let described = item
        .description
        .lines()
        .map(|line| (Source::Description, line));
    let noted = item.note.lines().map(|line| (Source::Note, line));
    let commented = item
        .comments
        .iter()
        .map(|comment| (Source::Comment, comments::split(comment).1));
    let mut checklist = Vec::new();
    checklist_titles(document, id, &mut checklist);
    described.chain(noted).chain(commented).chain(
        checklist
            .into_iter()
            .map(|title| (Source::Checklist, title)),
    )
}

/// Adds the titles of the checklist items inside `id` to `titles`, in file order.
fn checklist_titles<'a>(document: &'a Document, id: ItemId, titles: &mut Vec<&'a str>) {
    for &child in document.children(id) {
        if let Some(item) = document.node(child).and_then(Node::item)
            && item.kind == ItemKind::ChecklistItem
            && item.deleted.is_none()
        {
            titles.push(&item.title);
            checklist_titles(document, child, titles);
        }
    }
}

/// `text` in lowercase, character by character, as [`Query::ranges`] matches it.
fn folded(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::{
        app::{Command, Focus, Go, Opened, view::Shown},
        domain::{Completion, Tag},
    };
    use ItemKind::{ChecklistItem, Project, Section, Task};

    /// Adds an item of `kind` titled `title` to `parent`, changed by `edit`.
    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
        edit: impl FnOnce(&mut Item),
    ) -> ItemId {
        let mut item = Item::new(kind, title);
        edit(&mut item);
        document
            .push(parent, Node::Item(item))
            .expect("the test builds a valid hierarchy")
    }

    fn task(id: ItemId) -> Row {
        Row::Task { id, section: None }
    }

    /// The rows search results show of `document` for `query`.
    fn found(document: &Document, query: &str) -> Vec<Row> {
        groups(document, query)
            .into_iter()
            .flat_map(|group| {
                assert_eq!(group.heading, None);
                group.rows
            })
            .collect()
    }

    // FR-SRCH-01
    #[test]
    fn every_word_is_found_in_any_case_in_any_text_of_an_item() {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let titled = add(&mut document, inbox, Task, "Call the BANK", |_| {});
        let described = add(&mut document, inbox, Task, "a", |item| {
            item.description = "first\nthe bank's number".to_owned();
        });
        let noted = add(&mut document, inbox, Task, "b", |item| {
            item.note = "# Bank\nopen at nine".to_owned();
        });
        let commented = add(&mut document, inbox, Task, "c", |item| {
            item.comments = vec!["[2026-10-04 07:05] Banking app".to_owned()];
        });
        let tagged = add(&mut document, inbox, Task, "d", |item| {
            item.tags = vec![Tag {
                name: "where".to_owned(),
                value: Some("bank".to_owned()),
            }];
        });
        let listed = add(&mut document, inbox, Task, "e", |_| {});
        let line = add(&mut document, Some(listed), ChecklistItem, "x", |_| {});
        add(&mut document, Some(line), ChecklistItem, "the bank", |_| {});
        add(&mut document, inbox, Task, "f", |item| {
            item.comments = vec!["[2026-10-04 07:05] other".to_owned()];
        });
        assert_eq!(
            found(&document, "bank"),
            [titled, described, noted, commented, tagged, listed].map(task),
            "a checklist item, nested or not, is found as part of its task"
        );
        assert_eq!(found(&document, "  BaNk   nine "), [task(noted)]);
        assert_eq!(
            found(&document, "where bank"),
            [task(tagged)],
            "a tag's name is found as well as its value"
        );
        assert_eq!(found(&document, "07:05"), [], "not a comment's timestamp");
        assert_eq!(
            found(&document, "bank\nthe"),
            [titled, described, listed].map(task)
        );
        assert_eq!(groups(&document, " \t"), [], "no words find nothing");
        assert_eq!(groups(&document, "nowhere"), [], "nothing found, no group");
    }

    // FR-SRCH-01, N-PORT-04
    #[test]
    fn words_are_found_in_any_case_in_text_beyond_ascii() {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let cafe = add(&mut document, inbox, Task, "Café ÜBER alles", |_| {});
        add(&mut document, inbox, Task, "Cafe uber", |_| {});
        assert_eq!(found(&document, "über café"), [task(cafe)]);
        assert_eq!(found(&document, "ALLES"), [task(cafe)]);
    }

    // FR-SRCH-01, N-PERF-02
    #[test]
    fn typing_on_narrows_the_results_and_refining_them_finds_what_a_search_finds() {
        assert!(narrows("ban", "bank"));
        assert!(narrows("ban nin", "NINE bank"));
        assert!(narrows("bank", "bank nine"), "a word added narrows too");
        assert!(!narrows("bank", "ban"), "a shorter word widens");
        assert!(!narrows("bank nine", "bank"), "a word taken away widens");
        assert!(!narrows("", "bank"), "no words found nothing to narrow");
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        for title in ["Call the bank", "Bank at nine", "Bandana", "nothing"] {
            add(&mut document, inbox, Task, title, |_| {});
        }
        for (from, to) in [("ban", "bank"), ("ban", "ban nine"), ("n", "nine bank")] {
            assert_eq!(
                refined(&document, to, &found(&document, from)),
                found(&document, to),
                "{from:?} to {to:?}"
            );
        }
    }

    // FR-SRCH-03, FR-ORG-09, OQ-14
    #[test]
    fn completed_items_come_last_and_the_trash_and_completed_contents_are_left_out() {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let done = add(&mut document, inbox, Task, "plan a", |item| {
            item.completed = Some(Completion::Undated);
        });
        let open = add(&mut document, inbox, Task, "plan b", |_| {});
        add(&mut document, inbox, Task, "plan c", |item| {
            item.deleted = Some(date(2026, 10, 1));
        });
        let work = add(&mut document, None, Project, "Plans", |_| {});
        let section = add(&mut document, Some(work), Section, "plan d", |_| {});
        let inside = add(&mut document, Some(section), Task, "plan e", |_| {});
        let finished = add(&mut document, Some(work), Project, "plan f", |item| {
            item.completed = Some(Completion::On(date(2026, 10, 2)));
        });
        add(&mut document, Some(finished), Task, "plan g", |_| {});
        let gone = add(&mut document, None, Project, "Gone", |item| {
            item.deleted = Some(date(2026, 10, 1));
        });
        add(&mut document, Some(gone), Task, "plan h", |_| {});
        document
            .push(None, Node::Kept("plan i, as read".to_owned()))
            .expect("kept content goes anywhere");
        assert_eq!(
            found(&document, "plan"),
            [
                task(open),
                Row::Project(work),
                Row::Project(section),
                task(inside),
                task(done),
                Row::Project(finished),
            ]
        );
        assert_eq!(
            found(&document, "inbox"),
            [],
            "the Inbox itself is no result"
        );
    }

    // FR-SRCH-01
    #[test]
    fn the_words_are_found_where_they_are_in_any_case_overlaps_merged() {
        let query = Query::new("an BAN");
        assert_eq!(query.ranges("Banana bandana"), [0..5, 7..10, 11..13]);
        assert_eq!(query.ranges("nothing"), []);
        assert_eq!(Query::new("").ranges("anything"), []);
        // `İ` is two characters in lowercase, the first an `i`: finding that marks all of `İ`.
        assert_eq!(Query::new("i").ranges("aİb"), [Range { start: 1, end: 3 }]);
        assert_eq!(Query::new("é").ranges("CAFÉ"), [Range { start: 3, end: 5 }]);
    }

    // FR-SRCH-01
    #[test]
    fn the_excerpt_is_the_first_line_holding_a_word_the_title_and_tags_do_not() {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let id = add(&mut document, inbox, Task, "Bank call", |item| {
            item.tags = vec![Tag::plain("phone")];
            item.description = "first\nask about the card".to_owned();
            item.note = "card number\nfee".to_owned();
            item.comments = vec![
                "[2026-10-04 07:05] the fee is high".to_owned(),
                "they said yes".to_owned(),
            ];
        });
        let line = add(&mut document, Some(id), ChecklistItem, "x", |_| {});
        add(
            &mut document,
            Some(line),
            ChecklistItem,
            "bring the passport",
            |_| {},
        );
        let excerpt = |query: &str| Query::new(query).excerpt(&document, id);
        let of = |source, text: &str| {
            Some(Excerpt {
                source,
                text: text.to_owned(),
            })
        };
        assert_eq!(excerpt("bank phone"), None, "all in the title and tags");
        assert_eq!(
            excerpt("BANK card"),
            of(Source::Description, "ask about the card")
        );
        assert_eq!(excerpt("fee"), of(Source::Note, "fee"));
        assert_eq!(excerpt("high"), of(Source::Comment, "the fee is high"));
        assert_eq!(excerpt("yes"), of(Source::Comment, "they said yes"));
        assert_eq!(
            excerpt("passport"),
            of(Source::Checklist, "bring the passport")
        );
        assert_eq!(excerpt("call nowhere"), None);
        assert_eq!(Query::new("x").excerpt(&document, ItemId::new(99)), None);
    }

    const TODAY: jiff::civil::Date = date(2026, 10, 14);

    fn state() -> (State, [ItemId; 3]) {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let first = add(&mut document, inbox, Task, "plan a", |_| {});
        let second = add(&mut document, inbox, Task, "plan b", |_| {});
        let work = add(&mut document, None, Project, "Work", |_| {});
        let done = add(&mut document, Some(work), Task, "a plan done", |item| {
            item.completed = Some(Completion::Undated);
        });
        add(&mut document, Some(work), Task, "other", |_| {});
        (
            State::new(document, Opened::Demo, TODAY),
            [first, second, done],
        )
    }

    fn type_text(state: &mut State, text: &str) {
        for c in text.chars() {
            state.search_input(SearchInput::Edit(TextEdit::Insert(c)));
        }
    }

    // FR-SRCH-01
    #[test]
    fn searching_shows_the_results_as_the_words_are_typed_focusing_the_first() {
        let (mut state, [first, second, done]) = state();
        state.search_input(SearchInput::Edit(TextEdit::Insert('x')));
        assert_eq!(state.search(), None, "nothing is typed before searching");
        state.open_search();
        assert_eq!(state.shown(), Shown::View(View::Search));
        assert_eq!(state.search().map(TextField::text), Some(""));
        assert_eq!(state.rows(), []);
        assert!(state.takes(Command::Navigator) && state.takes(Command::Quit));
        assert!(
            !state.takes(Command::Focus(Focus::Next)),
            "the words take the keys"
        );
        type_text(&mut state, "plan");
        assert_eq!(state.rows(), [first, second, done].map(task));
        assert_eq!(state.selection().focused(), Some(first));
        type_text(&mut state, " b");
        assert_eq!(state.rows(), [task(second)]);
        assert_eq!(state.selection().focused(), Some(second));
        state.search_input(SearchInput::Done);
        assert_eq!(state.search(), None);
        assert!(state.takes(Command::Focus(Focus::Next)));
        assert_eq!(
            state.pane_view().query().map(TextField::text),
            Some("plan b")
        );
        state.go(Go::Inbox);
        assert_eq!(
            state.pane_view().query(),
            None,
            "only search results show the words"
        );
        state.open_search();
        assert_eq!(
            state.search().and_then(TextField::selected_text),
            Some("plan b"),
            "the words are kept, selected so that typing replaces them"
        );
        state.finish_searching();
        assert_eq!(state.search(), None);
        state.toggle_sidebar();
        assert!(state.sidebar_has_keys());
        state.open_search();
        assert!(!state.sidebar_has_keys(), "the words typed are the pane's");
    }

    // FR-SRCH-01, FR-SRCH-03, N-PERF-02
    #[test]
    fn the_results_follow_the_words_and_the_document_as_they_change() {
        let (mut state, [first, second, done]) = state();
        state.open_search();
        type_text(&mut state, "pla");
        assert_eq!(state.rows(), [first, second, done].map(task));
        type_text(&mut state, "n b");
        assert_eq!(state.rows(), [task(second)], "typing on narrows");
        state.search_input(SearchInput::Edit(TextEdit::DeleteBackward));
        assert_eq!(
            state.rows(),
            [first, second, done].map(task),
            "taking a letter away widens"
        );
        state.search_input(SearchInput::Done);
        state.toggle_completed(TODAY);
        assert_eq!(
            state.rows(),
            [second, first, done].map(task),
            "a result completed moves down among the completed"
        );
    }

    // FR-SRCH-02, FR-SRCH-03, FR-VIEW-11
    #[test]
    fn a_completed_result_opens_where_it_lives_in_a_new_pane_with_completed_items_shown() {
        let searched = || {
            let (mut state, [.., done]) = state();
            state.open_search();
            type_text(&mut state, "done");
            state.search_input(SearchInput::Done);
            assert!(!state.shows_completed());
            (state, done)
        };
        let (mut state, done) = searched();
        state.open_in_new_pane();
        assert_eq!(state.panes().count(), 2);
        assert_eq!(state.selection().focused(), Some(done));
        assert!(state.shows_completed());
    }
}
