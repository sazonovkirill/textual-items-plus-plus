//! Tags: adding and removing the user's tags on the selected items from the tag picker, which
//! offers the name typed as a new tag, and the tag filter that narrows the rows a pane shows to the
//! items carrying chosen tags (FR-TAG-01, FR-TAG-02, FR-TAG-04, FR-OPS-06). Hides which tags an
//! item carries, inherits and is offered.

use super::{Candidate, Picked, Picker, PickerInput, PickerView, Row, TextEdit, container};
use crate::domain::{Document, Item, ItemId, Node, Op, Tag, Transaction, tag_name};

/// One tag the tag picker lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagEntry {
    /// Its name, without `@`.
    pub name: String,
    /// How it stands with what the picker is for.
    pub mark: TagMark,
}

/// How a tag the picker lists stands with the items it tags, or with the filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagMark {
    /// None of the items carries it, or the filter leaves it out.
    Off,
    /// Some of the items carry it.
    Some,
    /// Every item carries it, or the filter has it.
    All,
    /// No item in the document carries it yet: choosing it creates it.
    New,
}

/// What an open tag picker is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagsFor {
    /// Adding a tag to the selected items or removing it from them.
    Items,
    /// Choosing the tags the pane is filtered by.
    Filter,
}

/// The tag picker as a frontend shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagsView<'a> {
    /// What it is for.
    pub tags_for: TagsFor,
    /// Its query and the tags it lists.
    pub picker: PickerView<'a, TagEntry>,
}

/// What a tag chosen in the picker changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Choice {
    /// The selected items, as this transaction does.
    Items(Transaction),
    /// The filter, which now has this tag when it lacked it and lacks it when it had it.
    Filter(String),
}

/// The tag picker while it is open: what it is for and the tags it lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TagPicker {
    /// The items it tags, in the order of their rows; empty for the filter.
    items: Vec<ItemId>,
    tags_for: TagsFor,
    /// The tags it lists whatever is typed.
    listed: Vec<Candidate<TagEntry>>,
    picker: Picker<TagEntry>,
}

impl TagPicker {
    /// The picker that tags `items`, listing every tag of the user's in the document, each
    /// marked by how many of the items carry it; `None` without an item.
    pub(super) fn for_items(document: &Document, items: Vec<ItemId>) -> Option<Self> {
        let items: Vec<ItemId> = items
            .into_iter()
            .filter(|&id| item(document, id).is_some_and(|item| item.kind.takes_tags_and_dates()))
            .collect();
        if items.is_empty() {
            return None;
        }
        let mut names = Vec::new();
        for id in document.top_level().iter().chain([&document.inbox()]) {
            gather(document, *id, &mut names);
        }
        let listed = listed(names, |name| {
            let carrying = items
                .iter()
                .filter(|&&id| item(document, id).is_some_and(|item| carries(item, name)))
                .count();
            match carrying {
                0 => TagMark::Off,
                all if all == items.len() => TagMark::All,
                _ => TagMark::Some,
            }
        });
        Some(Self::new(items, TagsFor::Items, listed))
    }

    /// The picker that chooses the tags `filter` holds, listing the tags of the user's carried by
    /// the items in `rows` and those inside them, and those `filter` holds, which it marks.
    pub(super) fn for_filter(document: &Document, rows: &[Row], filter: &[String]) -> Self {
        let mut names = filter.to_vec();
        for row in rows {
            gather(document, row.id(), &mut names);
        }
        let listed = listed(names, |name| {
            if filter.iter().any(|chosen| chosen == name) {
                TagMark::All
            } else {
                TagMark::Off
            }
        });
        Self::new(Vec::new(), TagsFor::Filter, listed)
    }

    /// The picker `self` was, made again for `document` with `items` and `rows`, and the query
    /// typed again, as after a reload; `None` when it tagged items and none is left.
    pub(super) fn reopen(
        &self,
        document: &Document,
        items: Vec<ItemId>,
        rows: &[Row],
        filter: &[String],
    ) -> Option<Self> {
        let mut picker = match self.tags_for {
            TagsFor::Items => Self::for_items(document, items)?,
            TagsFor::Filter => Self::for_filter(document, rows, filter),
        };
        for c in self.picker.view().query.text().chars() {
            picker.input(document, PickerInput::Edit(TextEdit::Insert(c)));
        }
        Some(picker)
    }

    fn new(items: Vec<ItemId>, tags_for: TagsFor, listed: Vec<Candidate<TagEntry>>) -> Self {
        Self {
            items,
            tags_for,
            picker: Picker::new(listed.clone()),
            listed,
        }
    }

    /// The items it tags.
    pub(super) fn items(&self) -> &[ItemId] {
        &self.items
    }

    /// The picker as a frontend shows it.
    pub(super) fn view(&self) -> TagsView<'_> {
        TagsView {
            tags_for: self.tags_for,
            picker: self.picker.view(),
        }
    }

    /// Applies `input`, and returns how it closed the picker, if it did, with what the tag chosen
    /// changes in `document`. While it tags items, a name typed that no tag listed has is offered
    /// first, as a new tag (FR-TAG-02).
    pub(super) fn input(
        &mut self,
        document: &Document,
        input: PickerInput,
    ) -> Option<Picked<Choice>> {
        let before = self.picker.view().query.text().to_owned();
        let picked = self.picker.input(input);
        if let Some(offered) = self.offered(&before) {
            self.picker.relist(offered);
        }
        Some(match picked? {
            Picked::Chosen(entry) => Picked::Chosen(match self.tags_for {
                TagsFor::Items => Choice::Items(toggle(document, &self.items, &entry)),
                TagsFor::Filter => Choice::Filter(entry.name),
            }),
            Picked::Dismissed => Picked::Dismissed,
        })
    }

    /// The candidates to list now that the query changed from `before`, the name it types first
    /// when that is a new tag; `None` while the query is as it was.
    fn offered(&self, before: &str) -> Option<Vec<Candidate<TagEntry>>> {
        let query = self.picker.view().query.text();
        if query == before || self.tags_for == TagsFor::Filter {
            return None;
        }
        let new = tag_name(query)
            .filter(|name| !self.listed.iter().any(|listed| listed.value.name == *name))
            .map(|name| Candidate {
                value: TagEntry {
                    name: name.clone(),
                    mark: TagMark::New,
                },
                text: name,
                detail: String::new(),
            });
        Some(new.into_iter().chain(self.listed.iter().cloned()).collect())
    }
}

/// The candidates for `names`, without repeats, in alphabetical order whatever the case, each
/// marked as `mark` says.
fn listed(mut names: Vec<String>, mark: impl Fn(&str) -> TagMark) -> Vec<Candidate<TagEntry>> {
    names.sort_by_cached_key(|name| (name.to_lowercase(), name.clone()));
    names.dedup();
    names
        .into_iter()
        .map(|name| Candidate {
            value: TagEntry {
                mark: mark(&name),
                name: name.clone(),
            },
            text: name,
            detail: String::new(),
        })
        .collect()
}

/// The transaction that adds `entry`'s tag to each of `items` that lacks it, or, when every one
/// carries it, removes it from them all (FR-OPS-06).
fn toggle(document: &Document, items: &[ItemId], entry: &TagEntry) -> Transaction {
    let remove = entry.mark == TagMark::All;
    let ops = items
        .iter()
        .filter_map(|&id| {
            let before = item(document, id)?;
            let mut after = before.clone();
            if remove {
                after
                    .tags
                    .retain(|tag| !(tag.is_users() && tag.name == entry.name));
            } else if !carries(before, &entry.name) {
                after.tags.push(Tag::plain(entry.name.as_str()));
            }
            (after != *before).then(|| Op::Set {
                id,
                before: before.clone(),
                after,
            })
        })
        .collect();
    Transaction::new(ops)
}

/// `rows` narrowed to those carrying every tag of `filter` (FR-TAG-04), counting the tags an item
/// inherits from the containers it is in, as Things 3 does: a task's or a project's row stays when
/// its item carries them, a section's heading when it carries them or a task listed under it stays,
/// and a child project's row also when an open item inside it carries them. Without a tag in
/// `filter`, every row stays.
pub(super) fn filtered(document: &Document, rows: Vec<Row>, filter: &[String]) -> Vec<Row> {
    if filter.is_empty() {
        return rows;
    }
    let matches = |id: ItemId| inherits(document, id, filter);
    let headed: Vec<ItemId> = rows
        .iter()
        .filter_map(|&row| match row {
            Row::Task {
                id,
                section: Some(section),
            } if matches(id) => Some(section),
            _ => None,
        })
        .collect();
    rows.into_iter()
        .filter(|&row| match row {
            Row::Task { id, .. } => matches(id),
            Row::Section(id) => matches(id) || headed.contains(&id),
            Row::Project(id) => matches(id) || holds(document, id, filter),
        })
        .collect()
}

/// Whether `id` carries every tag of `filter`, itself or by being inside an item that does.
fn inherits(document: &Document, id: ItemId, filter: &[String]) -> bool {
    let path = container::path(document, id);
    filter.iter().all(|name| {
        path.iter()
            .any(|&above| item(document, above).is_some_and(|item| carries(item, name)))
    })
}

/// Whether an open item inside `id`, outside the Trash, carries every tag of `filter`.
fn holds(document: &Document, id: ItemId, filter: &[String]) -> bool {
    document
        .children(id)
        .iter()
        .any(|&child| match item(document, child) {
            Some(item) if item.deleted.is_none() => {
                (item.completed.is_none() && inherits(document, child, filter))
                    || holds(document, child, filter)
            }
            _ => false,
        })
}

/// Adds the names of the user's tags carried by `id` and the items inside it, outside the Trash,
/// to `names`.
fn gather(document: &Document, id: ItemId, names: &mut Vec<String>) {
    let Some(item) = item(document, id).filter(|item| item.deleted.is_none()) else {
        return;
    };
    names.extend(users_tags(item).map(|tag| tag.name.clone()));
    for &child in document.children(id) {
        gather(document, child, names);
    }
}

/// Whether `item` carries the user's tag `name`.
fn carries(item: &Item, name: &str) -> bool {
    users_tags(item).any(|tag| tag.name == name)
}

/// The user's tags `item` carries, as it shows them (D-75).
fn users_tags(item: &Item) -> impl Iterator<Item = &Tag> {
    item.shown_tags().iter().filter(|tag| tag.is_users())
}

/// The item `id` is, if it is one.
fn item(document: &Document, id: ItemId) -> Option<&Item> {
    document.node(id).and_then(Node::item)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::rows;
    use crate::domain::{Completion, ItemKind, PlacementError};
    use ItemKind::{Area, Project, Section, Task};

    /// The Inbox holds `a @home`, `b` and `c @home @work`, and `gone @old` in the Trash; project
    /// `work @office` holds task `loose`, section `meetings` with `call @phone`, and project
    /// `site`, whose section `launch` holds `ship @deep` and `done @deep`, completed.
    struct Fixture {
        document: Document,
        a: ItemId,
        b: ItemId,
        c: ItemId,
        work: ItemId,
        meetings: ItemId,
    }

    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
        tags: &[&str],
    ) -> Result<ItemId, PlacementError> {
        let item = Item {
            tags: tags.iter().map(|&name| Tag::plain(name)).collect(),
            ..Item::new(kind, title)
        };
        document.push(parent, Node::Item(item))
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let a = add(&mut document, inbox, Task, "a", &["home"])?;
        let b = add(&mut document, inbox, Task, "b", &[])?;
        let c = add(&mut document, inbox, Task, "c", &["home", "work"])?;
        let gone = Item {
            deleted: Some(date(2026, 10, 1)),
            tags: vec![Tag::plain("old")],
            ..Item::new(Task, "gone")
        };
        document.push(inbox, Node::Item(gone))?;
        let work = add(&mut document, None, Project, "work", &["office"])?;
        add(&mut document, Some(work), Task, "loose", &[])?;
        let meetings = add(&mut document, Some(work), Section, "meetings", &[])?;
        add(&mut document, Some(meetings), Task, "call", &["phone"])?;
        let site = add(&mut document, Some(work), Project, "site", &[])?;
        let launch = add(&mut document, Some(site), Section, "launch", &[])?;
        add(&mut document, Some(launch), Task, "ship", &["deep"])?;
        let done = Item {
            completed: Some(Completion::On(date(2026, 10, 1))),
            tags: vec![Tag::plain("later")],
            ..Item::new(Task, "done")
        };
        document.push(Some(launch), Node::Item(done))?;
        document.push(Some(site), Node::Kept("kept as read @deep".to_owned()))?;
        Ok(Fixture {
            document,
            a,
            b,
            c,
            work,
            meetings,
        })
    }

    fn listed(picker: &TagPicker) -> Vec<(String, TagMark)> {
        picker
            .view()
            .picker
            .entries
            .iter()
            .map(|entry| (entry.value.name.clone(), entry.value.mark))
            .collect()
    }

    fn typed(picker: &mut TagPicker, document: &Document, text: &str) {
        for c in text.chars() {
            let input = PickerInput::Edit(TextEdit::Insert(c));
            assert_eq!(picker.input(document, input), None);
        }
    }

    fn entry(name: &str, mark: TagMark) -> (String, TagMark) {
        (name.to_owned(), mark)
    }

    fn tags(document: &Document, id: ItemId) -> Vec<String> {
        item(document, id)
            .map(|item| item.tags.iter().map(|tag| tag.name.clone()).collect())
            .unwrap_or_default()
    }

    // FR-TAG-02, FR-OPS-06
    #[test]
    fn the_picker_lists_every_tag_outside_the_trash_marked_by_the_items_carrying_it()
    -> Result<(), PlacementError> {
        let mut f = fixture()?;
        let picker =
            TagPicker::for_items(&f.document, vec![f.a, f.c]).ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(picker.view().tags_for, TagsFor::Items);
        assert_eq!(picker.items(), [f.a, f.c]);
        assert_eq!(
            listed(&picker),
            [
                entry("deep", TagMark::Off),
                entry("home", TagMark::All),
                entry("later", TagMark::Off),
                entry("office", TagMark::Off),
                entry("phone", TagMark::Off),
                entry("work", TagMark::Some),
            ]
        );
        assert_eq!(TagPicker::for_items(&f.document, Vec::new()), None);
        let kept = f.document.new_id();
        assert_eq!(TagPicker::for_items(&f.document, vec![kept]), None);
        Ok(())
    }

    // FR-TAG-01, FR-TAG-02, FR-TAG-04
    #[test]
    fn an_area_has_no_tags_even_those_a_file_gives_it() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let area = add(&mut document, None, Area, "home", &["away"])?;
        add(&mut document, Some(area), Task, "x", &[])?;
        assert_eq!(TagPicker::for_items(&document, vec![area]), None);
        let rows = rows(&document, area, false);
        let picker = TagPicker::for_filter(&document, &rows, &[]);
        assert_eq!(listed(&picker), Vec::new(), "its tag is not offered");
        let kept = filtered(&document, rows, &filter(&["away"]));
        assert_eq!(titles(&document, &kept), Vec::<String>::new());
        Ok(())
    }

    // FR-TAG-02
    #[test]
    fn a_name_typed_that_no_tag_has_is_offered_first_as_a_new_tag() -> Result<(), PlacementError> {
        let f = fixture()?;
        let mut picker =
            TagPicker::for_items(&f.document, vec![f.b]).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut picker, &f.document, "ho");
        assert_eq!(
            listed(&picker),
            [
                entry("ho", TagMark::New),
                entry("home", TagMark::Off),
                entry("phone", TagMark::Off),
            ]
        );
        typed(&mut picker, &f.document, "me");
        assert_eq!(
            listed(&picker),
            [entry("home", TagMark::Off)],
            "home exists"
        );
        typed(&mut picker, &f.document, " office");
        assert_eq!(
            listed(&picker).first(),
            Some(&entry("home-office", TagMark::New))
        );
        let mut refused =
            TagPicker::for_items(&f.document, vec![f.b]).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut refused, &f.document, "due");
        assert_eq!(listed(&refused), [], "due is the app's own");
        assert_eq!(refused.input(&f.document, PickerInput::Next), None);
        Ok(())
    }

    // FR-TAG-02, FR-OPS-06
    #[test]
    fn choosing_a_tag_adds_it_to_the_items_lacking_it_or_takes_it_off_all()
    -> Result<(), PlacementError> {
        let f = fixture()?;
        let mut document = f.document.clone();
        let mut picker =
            TagPicker::for_items(&document, vec![f.a, f.b]).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut picker, &document, "home");
        let adding = tagged(picker.input(&document, PickerInput::Choose))
            .ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(adding.ops().len(), 1, "a has it already");
        document.apply(&adding)?;
        assert_eq!(tags(&document, f.b), ["home"]);
        let mut picker =
            TagPicker::for_items(&document, vec![f.a, f.b]).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut picker, &document, "home");
        let removing = tagged(picker.input(&document, PickerInput::Choose))
            .ok_or(PlacementError::DoesNotFit)?;
        document.apply(&removing)?;
        assert_eq!(
            (tags(&document, f.a), tags(&document, f.b)),
            (vec![], vec![])
        );
        let mut picker =
            TagPicker::for_items(&document, vec![f.c]).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut picker, &document, "fresh");
        let new = tagged(picker.input(&document, PickerInput::Choose))
            .ok_or(PlacementError::DoesNotFit)?;
        document.apply(&new)?;
        assert_eq!(tags(&document, f.c), ["home", "work", "fresh"]);
        assert_eq!(tagged(picker.input(&document, PickerInput::Dismiss)), None);
        Ok(())
    }

    // FR-TAG-02, FR-STORE-10
    #[test]
    fn a_tag_with_a_value_counts_by_its_name_and_a_reserved_one_not_at_all()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let valued = Tag {
            name: "home".to_owned(),
            value: Some("1".to_owned()),
        };
        let item = Item {
            tags: vec![valued, Tag::plain("due")],
            ..Item::new(Task, "t")
        };
        let t = document.push(inbox, Node::Item(item))?;
        let b = add(&mut document, inbox, Task, "b", &[])?;
        let mut picker =
            TagPicker::for_items(&document, vec![t, b]).ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(listed(&picker), [entry("home", TagMark::Some)]);
        typed(&mut picker, &document, "due");
        assert_eq!(
            listed(&picker),
            [],
            "a reserved name is neither listed nor offered"
        );
        let mut picker =
            TagPicker::for_items(&document, vec![t, b]).ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(
            picker.input(&document, PickerInput::Choose),
            Some(Picked::Chosen(Choice::Items(Transaction::new(vec![
                Op::Set {
                    id: b,
                    before: Item::new(Task, "b"),
                    after: Item {
                        tags: vec![Tag::plain("home")],
                        ..Item::new(Task, "b")
                    },
                }
            ])))),
            "t carries home already, with its value"
        );
        let mut picker =
            TagPicker::for_items(&document, vec![t]).ok_or(PlacementError::DoesNotFit)?;
        let removing = tagged(picker.input(&document, PickerInput::Choose))
            .ok_or(PlacementError::DoesNotFit)?;
        document.apply(&removing)?;
        assert_eq!(tags(&document, t), ["due"]);
        Ok(())
    }

    /// The transaction choosing a tag for items makes; none for anything else.
    fn tagged(picked: Option<Picked<Choice>>) -> Option<Transaction> {
        if let Some(Picked::Chosen(Choice::Items(transaction))) = picked {
            return Some(transaction);
        }
        None
    }

    fn titles(document: &Document, rows: &[Row]) -> Vec<String> {
        rows.iter()
            .filter_map(|row| Some(item(document, row.id())?.title.clone()))
            .collect()
    }

    fn filter(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    // FR-TAG-04
    #[test]
    fn a_filter_keeps_the_rows_carrying_every_tag_chosen() -> Result<(), PlacementError> {
        let f = fixture()?;
        let document = &f.document;
        let inbox = rows(document, document.inbox(), false);
        let shown =
            |names: &[&str]| titles(document, &filtered(document, inbox.clone(), &filter(names)));
        assert_eq!(shown(&[]), ["a", "b", "c"]);
        assert_eq!(shown(&["home"]), ["a", "c"]);
        assert_eq!(shown(&["home", "work"]), ["c"]);
        assert_eq!(shown(&["office"]), Vec::<String>::new());
        Ok(())
    }

    // FR-TAG-04
    #[test]
    fn items_inherit_the_tags_of_the_containers_they_are_in() -> Result<(), PlacementError> {
        let f = fixture()?;
        let document = &f.document;
        let work = rows(document, f.work, false);
        let shown =
            |names: &[&str]| titles(document, &filtered(document, work.clone(), &filter(names)));
        assert_eq!(shown(&["office"]), ["loose", "meetings", "call", "site"]);
        assert_eq!(
            shown(&["phone"]),
            ["meetings", "call"],
            "a heading over a task kept"
        );
        assert_eq!(shown(&["deep"]), ["site"], "a project holding a task kept");
        assert_eq!(
            shown(&["later"]),
            Vec::<String>::new(),
            "completed ones don't count"
        );
        assert_eq!(shown(&["office", "phone"]), ["meetings", "call"]);
        let meetings = Item {
            tags: vec![Tag::plain("talk")],
            ..Item::new(Section, "meetings")
        };
        let mut document = document.clone();
        document.set(f.meetings, meetings)?;
        let work = rows(&document, f.work, false);
        let kept = filtered(&document, work, &filter(&["talk"]));
        assert_eq!(titles(&document, &kept), ["meetings", "call"]);
        Ok(())
    }

    // FR-TAG-04
    #[test]
    fn the_filter_picker_lists_the_tags_shown_and_those_chosen() -> Result<(), PlacementError> {
        let f = fixture()?;
        let document = &f.document;
        let inbox = rows(document, document.inbox(), false);
        let mut picker = TagPicker::for_filter(document, &inbox, &filter(&["phone"]));
        assert_eq!(picker.view().tags_for, TagsFor::Filter);
        assert_eq!(picker.items(), []);
        assert_eq!(
            listed(&picker),
            [
                entry("home", TagMark::Off),
                entry("phone", TagMark::All),
                entry("work", TagMark::Off),
            ]
        );
        typed(&mut picker, document, "zzz");
        assert_eq!(listed(&picker), [], "no new tag to filter by");
        let mut picker = TagPicker::for_filter(document, &inbox, &[]);
        typed(&mut picker, document, "wo");
        assert_eq!(
            picker.input(document, PickerInput::Choose),
            Some(Picked::Chosen(Choice::Filter("work".to_owned())))
        );
        Ok(())
    }

    // FR-TAG-02, FR-TAG-04
    #[test]
    fn a_picker_reopens_with_its_query_on_the_items_left() -> Result<(), PlacementError> {
        let f = fixture()?;
        let document = &f.document;
        let inbox = rows(document, document.inbox(), false);
        let mut picker =
            TagPicker::for_items(document, vec![f.a]).ok_or(PlacementError::DoesNotFit)?;
        typed(&mut picker, document, "wo");
        let again = picker
            .reopen(document, vec![f.c], &inbox, &[])
            .ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(again.items(), [f.c]);
        assert_eq!(again.view().picker.query.text(), "wo");
        assert_eq!(picker.reopen(document, Vec::new(), &inbox, &[]), None);
        let mut filtering = TagPicker::for_filter(document, &inbox, &[]);
        typed(&mut filtering, document, "h");
        let again = filtering
            .reopen(document, Vec::new(), &inbox, &filter(&["home"]))
            .ok_or(PlacementError::DoesNotFit)?;
        assert_eq!(listed(&again), [entry("home", TagMark::All)]);
        Ok(())
    }
}
