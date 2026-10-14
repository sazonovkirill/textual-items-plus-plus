//! Setting start and due dates: the date prompt that takes a date in words for the selected
//! items, the date it resolves to as it is typed, and the change that sets or removes that date
//! on them (FR-DATE-01, FR-DATE-02, FR-OPS-06), as taskpaper-terminal's date prompt does
//! ([ui.md §1](../../../specs/reference/taskpaper-terminal/specs/ui.md#1-layout)).

use jiff::{Span, civil::Date};

use super::{TextEdit, TextField};
use crate::domain::{
    Document, Item, ItemId, ItemKind, Node, Op, Transaction,
    dates::{self, DateError, DateKind},
};

/// What the user does in the open date prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateInput {
    /// Changes the text typed.
    Edit(TextEdit),
    /// Sets the date the text resolves to, or removes the date when the text is blank, and
    /// closes the prompt; a text that is not a date keeps it open.
    Apply,
    /// Closes the prompt, changing nothing.
    Cancel,
}

/// The open date prompt as a frontend shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatePromptView<'a> {
    /// The date it sets.
    pub kind: DateKind,
    /// The text typed.
    pub field: &'a TextField,
    /// What applying does: set this date, remove the date for `None`, or nothing, because the
    /// text is not a date for this reason.
    pub date: Result<Option<Date>, DateError>,
}

/// The open prompt: the items and the date it sets, the local date it was opened on, which the
/// text is resolved against, and the text typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DatePrompt {
    items: Vec<ItemId>,
    kind: DateKind,
    today: Date,
    field: TextField,
}

impl DatePrompt {
    /// The prompt for the `kind` date of the projects, sections and tasks among `items`,
    /// resolving what is typed against `today`, with the date of `focused`, or else of the first
    /// of them, typed and selected, so that typing replaces it and applying keeps it; `None`
    /// when none of them takes a date.
    pub(super) fn open(
        document: &Document,
        items: impl IntoIterator<Item = ItemId>,
        kind: DateKind,
        today: Date,
        focused: Option<ItemId>,
    ) -> Option<Self> {
        let items: Vec<ItemId> = items
            .into_iter()
            .filter(|&id| dated(document, id).is_some())
            .collect();
        let shown = focused
            .filter(|focused| items.contains(focused))
            .or(items.first().copied())?;
        let text = dated(document, shown)
            .and_then(|item| item.date(kind))
            .map(|date| date.to_string())
            .unwrap_or_default();
        Some(Self {
            items,
            kind,
            today,
            field: TextField::new(text).edited(TextEdit::SelectAll),
        })
    }

    /// The prompt as a frontend shows it.
    pub(super) fn view(&self) -> DatePromptView<'_> {
        DatePromptView {
            kind: self.kind,
            field: &self.field,
            date: self.date(),
        }
    }

    /// Changes the text typed by `edit`.
    pub(super) fn edit(&mut self, edit: TextEdit) {
        self.field = self.field.edited(edit);
    }

    /// The transaction that applying makes, or `None` while the text is not a date.
    pub(super) fn transaction(&self, document: &Document) -> Option<Transaction> {
        let date = self.date().ok()?;
        Some(set(document, self.items.iter().copied(), self.kind, date))
    }

    /// The prompt on the items of a reloaded document that `exact` finds for its own, or `None`
    /// when it finds none of them.
    pub(super) fn rematched(self, exact: impl Fn(ItemId) -> Option<ItemId>) -> Option<Self> {
        let items: Vec<ItemId> = self.items.iter().filter_map(|&id| exact(id)).collect();
        (!items.is_empty()).then_some(Self { items, ..self })
    }

    /// The date the text resolves to, or `None` for a blank text, which removes the date.
    fn date(&self) -> Result<Option<Date>, DateError> {
        resolve(self.field.text(), self.today, self.kind)
    }
}

/// The `kind` date `text` names on `today`, or `None` for a blank text, which removes the date.
pub(super) fn resolve(text: &str, today: Date, kind: DateKind) -> Result<Option<Date>, DateError> {
    match dates::parse(text, today, kind) {
        Ok(date) => Ok(Some(date)),
        Err(DateError::Empty) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The transaction that sets the `kind` date of `items` to `date`, or removes it for `None`.
/// Only projects, sections and tasks take dates (FR-DATE-01, D-75); anything else among
/// `items`, and an item whose date is that already, is left alone.
pub(super) fn set(
    document: &Document,
    items: impl IntoIterator<Item = ItemId>,
    kind: DateKind,
    date: Option<Date>,
) -> Transaction {
    redate(document, items, kind, |_| date)
}

/// The transaction that moves the `kind` date of `items` `days` days later, or earlier when
/// negative, an item without one counting from `today` (D-82). Only projects, sections and tasks
/// take dates; anything else among `items`, and a date that would leave the calendar, is left
/// alone.
pub(super) fn nudge(
    document: &Document,
    items: impl IntoIterator<Item = ItemId>,
    kind: DateKind,
    days: i8,
    today: Date,
) -> Transaction {
    redate(document, items, kind, |item| {
        let from = item.date(kind).unwrap_or(today);
        from.checked_add(Span::new().days(days))
            .ok()
            .or(item.date(kind))
    })
}

/// The transaction that gives each of `items` that takes dates the `kind` date `date` makes of
/// it, leaving alone those whose date that is already. A start date takes an item out of Later,
/// as Things 3's When is either a day or Someday (D-92).
fn redate(
    document: &Document,
    items: impl IntoIterator<Item = ItemId>,
    kind: DateKind,
    date: impl Fn(&Item) -> Option<Date>,
) -> Transaction {
    let ops = items
        .into_iter()
        .filter_map(|id| Some((id, dated(document, id)?)))
        .filter_map(|(id, item)| {
            let date = date(item);
            let mut after = item.clone();
            match kind {
                DateKind::Start => {
                    after.start = date;
                    // A file's mark on a kind that is never Later stays (D-89).
                    after.later &= date.is_none() || !item.kind.goes_later();
                }
                DateKind::Due => after.due = date,
            }
            (after != *item).then(|| Op::Set {
                id,
                before: item.clone(),
                after,
            })
        })
        .collect();
    Transaction::new(ops)
}

/// The item `id` when it is a project, section or task, which take dates; an area takes none
/// (D-75).
fn dated(document: &Document, id: ItemId) -> Option<&Item> {
    document.node(id).and_then(Node::item).filter(|item| {
        matches!(
            item.kind,
            ItemKind::Project | ItemKind::Section | ItemKind::Task
        )
    })
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::PlacementError;

    /// A Wednesday.
    const TODAY: Date = date(2026, 10, 14);

    fn item(document: &Document, id: ItemId) -> &Item {
        document.node(id).and_then(Node::item).expect("an item")
    }

    /// A task `a` starting on 2026-10-01, a task `b`, and a checklist item under `a`.
    fn document() -> Result<(Document, ItemId, ItemId, ItemId), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let mut started = Item::new(ItemKind::Task, "a");
        started.start = Some(date(2026, 10, 1));
        let a = document.push(inbox, Node::Item(started))?;
        let b = document.push(inbox, Node::Item(Item::new(ItemKind::Task, "b")))?;
        let check = document.push(Some(a), Node::Item(Item::new(ItemKind::ChecklistItem, "c")))?;
        Ok((document, a, b, check))
    }

    fn typed(prompt: &mut DatePrompt, text: &str) {
        for c in text.chars() {
            prompt.edit(TextEdit::Insert(c));
        }
    }

    // FR-DATE-01, FR-OPS-06
    #[test]
    fn setting_a_date_changes_only_the_items_that_take_it_and_lack_it() -> Result<(), PlacementError>
    {
        let (mut document, a, b, check) = document()?;
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "x")))?;
        let section = document.push(Some(area), Node::Item(Item::new(ItemKind::Section, "s")))?;
        let project = document.push(Some(area), Node::Item(Item::new(ItemKind::Project, "p")))?;
        let line = document.push(None, Node::Kept("text".to_owned()))?;
        let others = [document.inbox(), area, check, line, ItemId::new(99)];
        assert_eq!(
            set(&document, others, DateKind::Due, Some(TODAY)),
            Transaction::default(),
            "an area has no dates (D-75)"
        );
        assert!(DatePrompt::open(&document, [area], DateKind::Due, TODAY, Some(area)).is_none());
        let all = [a, b, section, project];
        let start = set(&document, all, DateKind::Start, Some(date(2026, 10, 1)));
        assert_eq!(start.ops().len(), 3, "a starts on that day already");
        document.apply(&set(&document, all, DateKind::Due, Some(TODAY)))?;
        for id in all {
            assert_eq!(item(&document, id).due, Some(TODAY));
        }
        assert_eq!(item(&document, a).start, Some(date(2026, 10, 1)));
        document.apply(&set(&document, [a], DateKind::Start, None))?;
        assert_eq!(item(&document, a).start, None);
        assert_eq!(item(&document, a).due, Some(TODAY));
        Ok(())
    }

    // FR-ORG-11, FR-DATE-02
    #[test]
    fn a_start_date_takes_an_item_out_of_later_but_a_due_date_does_not()
    -> Result<(), PlacementError> {
        let (mut document, a, b, _) = document()?;
        for id in [a, b] {
            let mut later = item(&document, id).clone();
            later.later = true;
            later.start = None;
            document.set(id, later)?;
        }
        document.apply(&set(&document, [a], DateKind::Due, Some(TODAY)))?;
        document.apply(&set(&document, [a], DateKind::Start, None))?;
        assert!(
            item(&document, a).later,
            "a due date or no start date keeps it"
        );
        document.apply(&nudge(&document, [a, b], DateKind::Start, 1, TODAY))?;
        for id in [a, b] {
            assert!(!item(&document, id).later);
        }
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "x")))?;
        let mut marked = Item::new(ItemKind::Section, "s");
        marked.later = true;
        let section = document.push(Some(area), Node::Item(marked))?;
        document.apply(&set(&document, [section], DateKind::Start, Some(TODAY)))?;
        assert!(item(&document, section).later, "a section's mark is kept");
        Ok(())
    }

    // FR-DATE-06, FR-OPS-06
    #[test]
    fn nudging_moves_each_date_a_day_counting_a_missing_one_from_today()
    -> Result<(), PlacementError> {
        let (mut document, a, b, check) = document()?;
        let area = document.push(None, Node::Item(Item::new(ItemKind::Area, "x")))?;
        assert_eq!(
            nudge(&document, [area, check], DateKind::Start, 1, TODAY),
            Transaction::default(),
            "an area and a checklist item take no dates"
        );
        document.apply(&nudge(&document, [a, b, area], DateKind::Start, 1, TODAY))?;
        assert_eq!(item(&document, a).start, Some(date(2026, 10, 2)));
        assert_eq!(item(&document, b).start, Some(date(2026, 10, 15)));
        document.apply(&nudge(&document, [a, b], DateKind::Due, -1, TODAY))?;
        assert_eq!(item(&document, a).due, Some(date(2026, 10, 13)));
        assert_eq!(item(&document, b).due, Some(date(2026, 10, 13)));
        assert_eq!(item(&document, a).start, Some(date(2026, 10, 2)), "kept");
        let mut last = Item::new(ItemKind::Task, "last");
        last.due = Some(Date::MAX);
        let last = document.push(Some(document.inbox()), Node::Item(last))?;
        assert_eq!(
            nudge(&document, [last], DateKind::Due, 1, TODAY),
            Transaction::default(),
            "no day after the calendar's last"
        );
        Ok(())
    }

    // FR-DATE-02
    #[test]
    fn the_prompt_opens_with_the_focused_items_date_selected() -> Result<(), PlacementError> {
        let (document, a, b, check) = document()?;
        let prompt = DatePrompt::open(&document, [a, b], DateKind::Start, TODAY, Some(a));
        let prompt = prompt.expect("a and b take dates");
        let view = prompt.view();
        assert_eq!(view.kind, DateKind::Start);
        assert_eq!(view.field.text(), "2026-10-01");
        assert_eq!(view.field.selection(), Some(0..10));
        assert_eq!(view.date, Ok(Some(date(2026, 10, 1))));
        let prompt = DatePrompt::open(&document, [b, a], DateKind::Start, TODAY, Some(check));
        let prompt = prompt.expect("a and b take dates");
        assert_eq!(prompt.view().field.text(), "", "b, the first, has none");
        assert_eq!(prompt.view().date, Ok(None));
        let none = DatePrompt::open(&document, [check], DateKind::Due, TODAY, Some(check));
        assert_eq!(none, None);
        Ok(())
    }

    // FR-DATE-02
    #[test]
    fn the_prompt_resolves_words_as_typed_and_applies_to_every_item() -> Result<(), PlacementError>
    {
        let (mut document, a, b, _) = document()?;
        let mut prompt = DatePrompt::open(&document, [a, b], DateKind::Due, TODAY, Some(a))
            .expect("a and b take dates");
        typed(&mut prompt, "fri");
        assert_eq!(prompt.view().date, Ok(Some(date(2026, 10, 16))));
        typed(&mut prompt, "day!");
        assert_eq!(prompt.view().date, Err(DateError::Unexpected { offset: 6 }));
        assert_eq!(prompt.transaction(&document), None);
        prompt.edit(TextEdit::DeleteBackward);
        let transaction = prompt.transaction(&document).expect("friday is a date");
        document.apply(&transaction)?;
        assert_eq!(item(&document, a).due, Some(date(2026, 10, 16)));
        assert_eq!(item(&document, b).due, Some(date(2026, 10, 16)));
        Ok(())
    }

    // FR-DATE-02
    #[test]
    fn a_blank_prompt_removes_the_date() -> Result<(), PlacementError> {
        let (mut document, a, _, _) = document()?;
        let mut prompt = DatePrompt::open(&document, [a], DateKind::Start, TODAY, Some(a))
            .expect("a takes dates");
        prompt.edit(TextEdit::DeleteBackward);
        typed(&mut prompt, "  ");
        assert_eq!(prompt.view().date, Ok(None));
        let transaction = prompt.transaction(&document).expect("blank removes");
        document.apply(&transaction)?;
        assert_eq!(item(&document, a).start, None);
        Ok(())
    }

    #[test]
    fn a_rematched_prompt_keeps_the_items_found_and_closes_without_any()
    -> Result<(), PlacementError> {
        let (document, a, b, _) = document()?;
        let prompt = DatePrompt::open(&document, [a, b], DateKind::Due, TODAY, Some(a))
            .expect("a and b take dates");
        let kept = prompt.clone().rematched(|id| (id == b).then_some(a));
        let kept = kept.expect("b is found as a");
        assert_eq!(kept.items, [a]);
        assert_eq!(kept.view(), prompt.view());
        assert_eq!(prompt.rematched(|_| None), None);
        Ok(())
    }
}
