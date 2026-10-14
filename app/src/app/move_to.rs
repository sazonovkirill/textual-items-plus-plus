//! Move to: moving the selected items into another container chosen in the fuzzy picker
//! (FR-OPS-04, FR-OPS-10, FR-ORG-06). Hides which containers can take the items and where in
//! them the items go.

use super::{Candidate, Picked, Picker, PickerInput, PickerView, TextEdit, container, navigator};
use crate::domain::{Document, ItemId, ItemKind, Node, Op, Transaction};

/// A place Move to can put the items, as the picker lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// The top level, which takes projects.
    Home,
    /// A container: the Inbox, an area, a project or a section.
    Container {
        /// The container.
        id: ItemId,
        /// What it is.
        kind: ItemKind,
        /// How deep in the tree it is: 0 for the Inbox and the top level.
        depth: usize,
    },
}

/// Move to while its picker is open: the items it moves and where it can move them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MoveTo {
    /// The projects, sections and tasks that move, none inside another, in the order taken.
    items: Vec<ItemId>,
    picker: Picker<Destination>,
}

impl MoveTo {
    /// Move to for the projects, sections and tasks among `items`, leaving out any inside another
    /// of them, which moves with it; `None` when none is left to move.
    pub(super) fn open(
        document: &Document,
        items: impl IntoIterator<Item = ItemId>,
    ) -> Option<Self> {
        let items = container::outermost_rows(document, items);
        (!items.is_empty()).then(|| Self {
            picker: Picker::new(destinations(document, &items)),
            items,
        })
    }

    /// Move to for `items` as [`open`](Self::open) makes it, with `query` typed again, as after a
    /// reload, which lists the destinations anew.
    pub(super) fn reopen(document: &Document, items: Vec<ItemId>, query: &str) -> Option<Self> {
        let mut move_to = Self::open(document, items)?;
        for c in query.chars() {
            move_to.picker.input(PickerInput::Edit(TextEdit::Insert(c)));
        }
        Some(move_to)
    }

    /// The items that move.
    pub(super) fn items(&self) -> &[ItemId] {
        &self.items
    }

    /// The picker as a frontend shows it.
    pub(super) fn view(&self) -> PickerView<'_, Destination> {
        self.picker.view()
    }

    /// Applies `input` to the picker, and says how it closed, if it did.
    pub(super) fn input(&mut self, input: PickerInput) -> Option<Picked<Destination>> {
        self.picker.input(input)
    }

    /// The transaction that moves the items, with everything inside them, to `destination`, in
    /// order, each the last of its group there, as a container lists its groups (FR-VIEW-05):
    /// after the last child of its group and the lines kept as read that follow it, which stay
    /// with it, or, in a group still empty, before the first child of a later group.
    pub(super) fn transaction(&self, document: &Document, destination: Destination) -> Transaction {
        let parent = match destination {
            Destination::Home => None,
            Destination::Container { id, .. } => Some(id),
        };
        let mut scratch = document.clone();
        let mut ops = Vec::new();
        for &id in &self.items {
            let moved = kind(&scratch, id).map(group);
            let (from, at) = scratch.position(id).expect("the items are in the document");
            let subtree = scratch.snapshot(id).expect("the items are in the document");
            let remove = Op::Remove {
                parent: from,
                index: at,
                subtree: subtree.clone(),
            };
            apply(&mut scratch, remove.clone());
            let siblings = siblings(&scratch, parent);
            let group_of = |sibling: &ItemId| kind(&scratch, *sibling).map(group);
            let index = match siblings
                .iter()
                .rposition(|sibling| group_of(sibling) == moved)
            {
                Some(last) => {
                    let kept = siblings.iter().skip(last + 1);
                    last + 1
                        + kept
                            .take_while(|sibling| group_of(sibling).is_none())
                            .count()
                }
                None => siblings
                    .iter()
                    .position(|sibling| group_of(sibling) > moved)
                    .unwrap_or(siblings.len()),
            };
            let insert = Op::Insert {
                parent,
                index,
                subtree,
            };
            apply(&mut scratch, insert.clone());
            ops.extend([remove, insert]);
        }
        Transaction::new(ops)
    }
}

/// What the item `id` is, if it is one.
fn kind(document: &Document, id: ItemId) -> Option<ItemKind> {
    document.node(id).and_then(Node::item).map(|item| item.kind)
}

/// The children of `parent`, or the top level for `None`.
fn siblings(document: &Document, parent: Option<ItemId>) -> &[ItemId] {
    parent.map_or(document.top_level(), |parent| document.children(parent))
}

/// The group a container lists items of `kind` in, in the order it lists them (FR-VIEW-05):
/// tasks, then sections, then projects.
fn group(kind: ItemKind) -> u8 {
    match kind {
        ItemKind::Section => 1,
        ItemKind::Area | ItemKind::Project => 2,
        _ => 0,
    }
}

/// Applies `op`, which was made from `document` as it is.
fn apply(document: &mut Document, op: Op) {
    document
        .apply(&Transaction::new(vec![op]))
        .expect("an operation made from the document as it is fits it");
}

/// Every place `items` can move to, in the order of the tree: the top level when it takes them
/// all, then the Inbox, then each area, project and section in file order, each followed by the
/// containers inside it, listing those that take every item. The items and what they hold are
/// left out, as are items in the Trash and completed ones with what they hold, which show
/// nowhere (FR-ORG-09), and the container every item is already in.
fn destinations(document: &Document, items: &[ItemId]) -> Vec<Candidate<Destination>> {
    let takes = |container: Option<ItemKind>, id: Option<ItemId>| {
        items
            .iter()
            .all(|&item| kind(document, item).is_some_and(|moved| moved.goes_in(container)))
            && !items
                .iter()
                .all(|&item| document.position(item).map(|(parent, _)| parent) == Some(id))
    };
    places(document, items, &takes)
}

/// Every place a new task can go, as Move to lists them (FR-EDIT-07): the Inbox, then each area,
/// project and section in file order, each followed by the containers inside it, leaving out
/// items in the Trash and completed ones with what they hold.
pub(super) fn task_places(document: &Document) -> Vec<Candidate<Destination>> {
    places(document, &[], &|container, _| {
        ItemKind::Task.goes_in(container)
    })
}

/// The places `takes` says take what goes there, in the order of the tree: the top level, then
/// the Inbox, then each area, project and section in file order, each followed by the
/// containers inside it, leaving out `items` and what they hold.
fn places(
    document: &Document,
    items: &[ItemId],
    takes: &dyn Fn(Option<ItemKind>, Option<ItemId>) -> bool,
) -> Vec<Candidate<Destination>> {
    let mut destinations = Vec::new();
    if takes(None, None) {
        destinations.push(Candidate {
            value: Destination::Home,
            text: "Home".to_owned(),
            detail: String::new(),
        });
    }
    let tops = std::iter::once(document.inbox()).chain(document.top_level().iter().copied());
    for id in tops {
        list(
            document,
            id,
            items,
            takes,
            &mut Vec::new(),
            &mut destinations,
        );
    }
    destinations
}

/// Adds the destination `id` is, if `takes` says it takes the items, and those inside it to
/// `destinations`, under the titles `above`; nothing inside `items`.
fn list<'a>(
    document: &'a Document,
    id: ItemId,
    items: &[ItemId],
    takes: &dyn Fn(Option<ItemKind>, Option<ItemId>) -> bool,
    above: &mut Vec<&'a str>,
    destinations: &mut Vec<Candidate<Destination>>,
) {
    let Some(item) = navigator::shown(document, id).filter(|_| !items.contains(&id)) else {
        return;
    };
    if !matches!(
        item.kind,
        ItemKind::Inbox | ItemKind::Area | ItemKind::Project | ItemKind::Section
    ) {
        return;
    }
    if takes(Some(item.kind), Some(id)) {
        destinations.push(Candidate {
            value: Destination::Container {
                id,
                kind: item.kind,
                depth: above.len(),
            },
            text: item.title.clone(),
            detail: above.join(" › "),
        });
    }
    above.push(&item.title);
    for &child in document.children(id) {
        list(document, child, items, takes, above, destinations);
    }
    above.pop();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Completion, Item, PlacementError};
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    /// The Inbox holds task `i`; area Work holds task `loose`, section `s_section` with task `s`
    /// and project `site`, which holds task `p`, a kept line, section `launch` and project
    /// `blog`; project `solo` stands at the top level. A project in the Trash and a completed
    /// one with a section are in Work too.
    struct Fixture {
        document: Document,
        i: ItemId,
        work: ItemId,
        loose: ItemId,
        s_section: ItemId,
        s: ItemId,
        site: ItemId,
        p: ItemId,
        launch: ItemId,
        blog: ItemId,
        solo: ItemId,
    }

    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
    ) -> Result<ItemId, PlacementError> {
        document.push(parent, Node::Item(Item::new(kind, title)))
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let i = add(&mut document, inbox, Task, "i")?;
        let work = add(&mut document, None, Area, "Work")?;
        let loose = add(&mut document, Some(work), Task, "loose")?;
        let s_section = add(&mut document, Some(work), Section, "S")?;
        let s = add(&mut document, Some(s_section), Task, "s")?;
        let site = add(&mut document, Some(work), Project, "Site")?;
        let p = add(&mut document, Some(site), Task, "p")?;
        document.push(Some(site), Node::Kept("kept".to_owned()))?;
        let launch = add(&mut document, Some(site), Section, "Launch")?;
        let blog = add(&mut document, Some(site), Project, "Blog")?;
        let mut trashed = Item::new(Project, "Trashed");
        trashed.deleted = Some(jiff::civil::date(2026, 10, 1));
        document.push(Some(work), Node::Item(trashed))?;
        let mut done = Item::new(Project, "Done");
        done.completed = Some(Completion::Undated);
        let done = document.push(Some(work), Node::Item(done))?;
        add(&mut document, Some(done), Section, "Within")?;
        let solo = add(&mut document, None, Project, "Solo")?;
        Ok(Fixture {
            document,
            i,
            work,
            loose,
            s_section,
            s,
            site,
            p,
            launch,
            blog,
            solo,
        })
    }

    /// The text of each destination Move to lists for `items`, `None` when it does not open.
    fn listed(document: &Document, items: &[ItemId]) -> Option<Vec<String>> {
        let move_to = MoveTo::open(document, items.iter().copied())?;
        Some(
            move_to
                .view()
                .entries
                .iter()
                .map(|candidate| candidate.text.clone())
                .collect(),
        )
    }

    // FR-OPS-04, FR-OPS-10, FR-ORG-09
    #[test]
    fn lists_the_containers_that_take_the_items_leaving_out_where_they_are_and_what_they_hold()
    -> Result<(), PlacementError> {
        let Fixture {
            document,
            i,
            s_section,
            site,
            solo,
            ..
        } = fixture()?;
        assert_eq!(
            listed(&document, &[i]),
            Some(
                ["Work", "S", "Site", "Launch", "Blog", "Solo"]
                    .map(String::from)
                    .into()
            ),
            "a task goes to any container, sections included, but where it is"
        );
        assert_eq!(
            listed(&document, &[s_section]),
            Some(["Site", "Blog", "Solo"].map(String::from).into()),
            "a section goes to an area or a project"
        );
        assert_eq!(
            listed(&document, &[site]),
            Some(["Home"].map(String::from).into()),
            "a project goes to the top level or an area, since v1 nests no project in another"
        );
        assert_eq!(
            listed(&document, &[solo]),
            Some(["Work"].map(String::from).into()),
            "a project at the top level is not offered the top level"
        );
        Ok(())
    }

    // FR-OPS-04
    #[test]
    fn only_projects_sections_and_tasks_move() -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            i,
            work,
            ..
        } = fixture()?;
        let check = add(&mut document, Some(i), ChecklistItem, "c")?;
        let inbox = document.inbox();
        for items in [&[][..], &[inbox], &[work], &[check], &[ItemId::new(99)]] {
            assert_eq!(listed(&document, items), None, "{items:?}");
        }
        Ok(())
    }

    // FR-OPS-04
    #[test]
    fn a_destination_names_the_places_above_it_and_its_depth() -> Result<(), PlacementError> {
        let Fixture {
            document,
            i,
            launch,
            ..
        } = fixture()?;
        let move_to = MoveTo::open(&document, [i]).ok_or(PlacementError::NoSuchNode(i))?;
        let entry = move_to
            .view()
            .entries
            .iter()
            .find(|candidate| candidate.text == "Launch")
            .cloned();
        assert_eq!(
            entry,
            Some(Candidate {
                value: Destination::Container {
                    id: launch,
                    kind: Section,
                    depth: 2,
                },
                text: "Launch".into(),
                detail: "Work › Site".into(),
            })
        );
        Ok(())
    }

    /// The titles of `parent`'s children, `"…"` for a kept line, or of the top level for `None`.
    fn titles(document: &Document, parent: Option<ItemId>) -> Vec<String> {
        siblings(document, parent)
            .iter()
            .map(|&id| match document.node(id) {
                Some(Node::Item(item)) => item.title.clone(),
                _ => "…".to_owned(),
            })
            .collect()
    }

    /// `document` after moving `items` to `destination`, checking that undo restores it.
    fn moved(
        document: &Document,
        items: &[ItemId],
        destination: Destination,
    ) -> Result<Document, PlacementError> {
        let move_to = MoveTo::open(document, items.iter().copied())
            .ok_or(PlacementError::NoSuchNode(ItemId::new(0)))?;
        let transaction = move_to.transaction(document, destination);
        let mut after = document.clone();
        after.apply(&transaction)?;
        let mut undone = after.clone();
        undone.apply(&transaction.inverse())?;
        assert_eq!(&undone, document, "undo puts everything back");
        Ok(after)
    }

    fn container(document: &Document, id: ItemId) -> Destination {
        let kind = kind(document, id).unwrap_or(Task);
        Destination::Container { id, kind, depth: 0 }
    }

    // FR-OPS-04, FR-OPS-10, FR-ORG-06, FR-VIEW-05
    #[test]
    fn a_moved_item_goes_last_in_its_group_with_everything_inside_it() -> Result<(), PlacementError>
    {
        let Fixture {
            document,
            i,
            work,
            s_section,
            site,
            launch,
            ..
        } = fixture()?;
        let after = moved(&document, &[i], container(&document, site))?;
        assert_eq!(
            titles(&after, Some(site)),
            ["p", "…", "i", "Launch", "Blog"],
            "a task goes after the tasks and kept lines, before the sections"
        );
        assert_eq!(after.children(after.inbox()), []);
        let after = moved(&document, &[s_section], container(&document, site))?;
        assert_eq!(
            titles(&after, Some(site)),
            ["p", "…", "Launch", "S", "Blog"]
        );
        assert_eq!(
            titles(&after, Some(s_section)),
            ["s"],
            "a section takes its tasks along"
        );
        let after = moved(&document, &[launch], container(&document, work))?;
        assert_eq!(
            titles(&after, Some(work)),
            ["loose", "S", "Launch", "Site", "Trashed", "Done"],
            "a section goes before the first project"
        );
        let after = moved(&document, &[site], Destination::Home)?;
        assert_eq!(titles(&after, None), ["Work", "Solo", "Site"]);
        assert_eq!(titles(&after, Some(site)), ["p", "…", "Launch", "Blog"]);
        let after = moved(&document, &[i], container(&document, launch))?;
        assert_eq!(titles(&after, Some(launch)), ["i"]);
        Ok(())
    }

    // FR-OPS-04, FR-VIEW-05
    #[test]
    fn in_a_file_that_interleaves_the_groups_a_moved_item_still_goes_last_in_its_group()
    -> Result<(), PlacementError> {
        let Fixture {
            mut document,
            i,
            site,
            ..
        } = fixture()?;
        add(&mut document, Some(site), Task, "late")?;
        document.push(Some(site), Node::Kept("kept late".to_owned()))?;
        let after = moved(&document, &[i], container(&document, site))?;
        assert_eq!(
            titles(&after, Some(site)),
            ["p", "…", "Launch", "Blog", "late", "…", "i"],
            "after the last task and the kept line after it"
        );
        let polish = add(&mut document, None, Project, "Polish")?;
        let after = moved(&document, &[polish], container(&document, site))?;
        assert_eq!(
            titles(&after, Some(site)),
            ["p", "…", "Launch", "Blog", "Polish", "late", "…"]
        );
        add(&mut document, Some(polish), Section, "Only")?;
        let after = moved(&document, &[i], container(&document, polish))?;
        assert_eq!(
            titles(&after, Some(polish)),
            ["i", "Only"],
            "the first task goes before the sections"
        );
        Ok(())
    }

    // FR-OPS-04, FR-OPS-01
    #[test]
    fn several_items_move_in_order_and_one_inside_another_moves_with_it()
    -> Result<(), PlacementError> {
        let Fixture {
            document,
            loose,
            s,
            site,
            p,
            blog,
            solo,
            ..
        } = fixture()?;
        let after = moved(&document, &[loose, s], container(&document, solo))?;
        assert_eq!(titles(&after, Some(solo)), ["loose", "s"]);
        let move_to = MoveTo::open(&document, [p, site, blog]);
        assert_eq!(move_to.as_ref().map(MoveTo::items), Some(&[site][..]));
        Ok(())
    }

    // FR-OPS-04, F-FILE-06
    #[test]
    fn reopening_types_the_query_again_and_needs_an_item() -> Result<(), PlacementError> {
        let Fixture { document, i, .. } = fixture()?;
        let move_to = MoveTo::reopen(&document, vec![i], "bl");
        let view = move_to.as_ref().map(MoveTo::view);
        assert_eq!(view.map(|view| view.query.text()), Some("bl"));
        assert_eq!(
            view.and_then(|view| view.entries.iter().next().map(|entry| entry.text.clone())),
            Some("Blog".to_owned())
        );
        assert_eq!(MoveTo::reopen(&document, Vec::new(), "bl"), None);
        Ok(())
    }
}
