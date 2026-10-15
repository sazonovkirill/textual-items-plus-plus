//! The Navigator: the places a user can go, the Inbox, the views of the view registry, the pinned
//! projects and every area and project as a tree, in the fuzzy picker a key opens over the panes
//! (FR-VIEW-07, FR-VIEW-12, OQ-10). Hides how the tree and its counts follow from the document.

use std::ops::Range;

use super::{
    Candidate, Picker, PickerInput, TextEdit, container,
    view::{self, Shown, View},
};
use crate::domain::{Document, Item, ItemId, ItemKind, Node};

/// An entry of the Navigator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// A container.
    Place(Place),
    /// A view.
    View(View),
}

impl Stop {
    /// What a pane shows of it.
    #[must_use]
    pub const fn shown(self) -> Shown {
        match self {
            Self::Place(place) => Shown::Container(place.id),
            Self::View(view) => Shown::View(view),
        }
    }

    /// How deep in the tree it is: 0 for a view.
    #[must_use]
    pub const fn depth(self) -> usize {
        match self {
            Self::Place(place) => place.depth,
            Self::View(_) => 0,
        }
    }

    /// Whether it is a pinned project's entry among the pinned projects, rather than in the tree.
    #[must_use]
    pub const fn in_pinned(self) -> bool {
        match self {
            Self::Place(place) => place.in_pinned,
            Self::View(_) => false,
        }
    }
}

/// A place the Navigator lists, with what it shows of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    /// The container it opens.
    pub id: ItemId,
    /// What the container is.
    pub kind: ItemKind,
    /// How deep in the tree it is: 0 for the Inbox and the top level.
    pub depth: usize,
    /// How many tasks not completed it holds, at any depth.
    pub open_tasks: usize,
    /// Whether it is a project marked Later, or inside one (D-92).
    pub later: bool,
    /// Whether it is listed among the pinned projects, which list a pinned project again, apart
    /// from the tree, at depth 0 (FR-VIEW-12).
    pub in_pinned: bool,
    /// Whether it is a flagged project (FR-ORG-12).
    pub flagged: bool,
}

/// The Navigator over `document`, focused on the entry of what the pane shows: a view's own, or
/// that of the container shown, else of the closest container above it that has one, such as a
/// section's project, as an editor's file tree follows the open file.
pub(super) fn open(document: &Document, shown: Shown) -> Picker<Stop> {
    let (stops, pinned) = listing(document);
    let mut picker = Picker::with_repeats(stops, pinned);
    match shown {
        Shown::View(view) => {
            picker.focus_where(|stop| *stop == Stop::View(view));
        }
        Shown::Container(id) => {
            for id in container::path(document, id).into_iter().rev() {
                if picker.focus_where(|stop| matches!(stop, Stop::Place(place) if place.id == id)) {
                    break;
                }
            }
        }
    }
    picker
}

/// The Navigator over `document` as [`open`] opens it, with `query` typed again, as after a
/// reload, which lists the places anew.
pub(super) fn reopen(document: &Document, shown: Shown, query: &str) -> Picker<Stop> {
    let mut picker = open(document, shown);
    for c in query.chars() {
        picker.input(PickerInput::Edit(TextEdit::Insert(c)));
    }
    picker
}

/// Every entry, in order: the Inbox, then the views as the view registry lists them, then the
/// pinned projects and then the other [`places`].
pub(super) fn stops(document: &Document) -> Vec<Candidate<Stop>> {
    listing(document).0
}

/// Every entry as [`stops`] lists them, and the positions of the pinned projects among them,
/// which repeat entries of the tree.
fn listing(document: &Document) -> (Vec<Candidate<Stop>>, Range<usize>) {
    let places = places(document);
    // A pinned project is listed again in the order of the tree, under the same titles above it.
    let pinned: Vec<Candidate<Stop>> = places
        .iter()
        .filter(|place| place.value.kind == ItemKind::Project && pinned(document, place.value.id))
        .map(|place| Candidate {
            value: Stop::Place(Place {
                depth: 0,
                in_pinned: true,
                ..place.value
            }),
            text: place.text.clone(),
            detail: place.detail.clone(),
        })
        .collect();
    let mut places = places.into_iter().map(|place| Candidate {
        value: Stop::Place(place.value),
        text: place.text,
        detail: place.detail,
    });
    let views = view::REGISTRY.iter().map(|entry| Candidate {
        value: Stop::View(entry.view),
        text: entry.title.to_owned(),
        detail: String::new(),
    });
    let mut stops: Vec<Candidate<Stop>> = places.next().into_iter().chain(views).collect();
    let start = stops.len();
    stops.extend(pinned);
    let repeats = start..stops.len();
    stops.extend(places);
    (stops, repeats)
}

/// Whether the item `id` is pinned.
fn pinned(document: &Document, id: ItemId) -> bool {
    document
        .node(id)
        .and_then(Node::item)
        .is_some_and(|item| item.pinned)
}

/// Every place, in the order of the tree: the Inbox, then each area and project at the top level
/// in file order, Later projects last, each followed by the projects inside it, Later ones last
/// (D-89). Each matches by its title and is told apart by the titles above it. Items in the Trash
/// and completed ones are left out with what they hold, as Things 3 takes completed projects out
/// of its sidebar (FR-ORG-09).
pub(super) fn places(document: &Document) -> Vec<Candidate<Place>> {
    let mut places = Vec::new();
    let tops = std::iter::once(document.inbox())
        .chain(container::later_last(document, document.top_level()));
    for id in tops {
        // Only an area or a project sits at the top level beside the Inbox (FR-ORG-01).
        if let Some(item) = shown(document, id) {
            list(document, id, item, &mut Vec::new(), &mut places);
        }
    }
    places
}

/// Adds the place `id`, shown as `item`, and those inside it to `places`, under the titles
/// `above`, and gives how many open tasks it holds; each place's count is the sum of what is
/// inside it, so the document is walked once (N-PERF-02).
fn list<'a>(
    document: &'a Document,
    id: ItemId,
    item: &'a Item,
    above: &mut Vec<&'a str>,
    places: &mut Vec<Candidate<Place>>,
) -> usize {
    let at = places.len();
    places.push(Candidate {
        value: Place {
            id,
            kind: item.kind,
            depth: above.len(),
            open_tasks: 0,
            later: document.defers(id),
            in_pinned: false,
            flagged: item.flagged,
        },
        text: item.title.clone(),
        detail: above.join(" › "),
    });
    above.push(&item.title);
    let mut open = 0;
    for child in container::later_last(document, document.children(id)) {
        let held = match shown(document, child) {
            Some(inside) if inside.kind == ItemKind::Project => {
                list(document, child, inside, above, places)
            }
            _ => open_tasks(document, id, child),
        };
        open += held;
    }
    above.pop();
    if let Some(place) = places.get_mut(at) {
        place.value.open_tasks = open;
    }
    open
}

/// How many tasks not completed `id` holds, at any depth, as its entry counts them.
pub(super) fn held_open_tasks(document: &Document, id: ItemId) -> usize {
    document
        .children(id)
        .iter()
        .map(|&child| open_tasks(document, id, child))
        .sum()
}

/// How many tasks not completed `child` of `parent` is or holds, at any depth, leaving out the
/// Trash, what completed items hold (FR-ORG-09) and, as its view does, the Inbox's scheduled
/// tasks (FR-VIEW-01).
fn open_tasks(document: &Document, parent: ItemId, child: ItemId) -> usize {
    let Some(item) = shown(document, child) else {
        return 0;
    };
    if container::scheduled_out_of_inbox(document, parent, item) {
        return 0;
    }
    match item.kind {
        ItemKind::Task => 1,
        // Only tasks hold checklist items, so below a task there are no tasks to count.
        _ => document
            .children(child)
            .iter()
            .map(|&inside| open_tasks(document, child, inside))
            .sum(),
    }
}

/// The item `id`, unless it is not one, is in the Trash or is completed.
pub(super) fn shown(document: &Document, id: ItemId) -> Option<&Item> {
    document
        .node(id)
        .and_then(Node::item)
        .filter(|item| item.deleted.is_none() && item.completion().is_none())
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::app::{Picked, PickerInput};
    use crate::domain::{Completion, PlacementError};
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    fn add(
        document: &mut Document,
        parent: Option<ItemId>,
        kind: ItemKind,
        title: &str,
    ) -> Result<ItemId, PlacementError> {
        document.push(parent, Node::Item(Item::new(kind, title)))
    }

    fn deleted(kind: ItemKind, title: &str) -> Node {
        let mut item = Item::new(kind, title);
        item.deleted = Some(date(2026, 10, 1));
        Node::Item(item)
    }

    /// The text, detail and place of each entry.
    fn listed(document: &Document) -> Vec<(String, String, Place)> {
        places(document)
            .into_iter()
            .map(|candidate| (candidate.text, candidate.detail, candidate.value))
            .collect()
    }

    // FR-VIEW-07
    #[test]
    fn lists_the_inbox_then_areas_and_projects_as_a_tree() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        let work = add(&mut document, None, Area, "Work")?;
        let site = add(&mut document, Some(work), Project, "Site")?;
        add(&mut document, Some(work), Section, "Hiring")?;
        let blog = add(&mut document, Some(site), Project, "Blog")?;
        let solo = add(&mut document, None, Project, "Solo")?;
        document.push(None, Node::Kept("a line".to_owned()))?;
        let place = |id, kind, depth| Place {
            id,
            kind,
            depth,
            open_tasks: 0,
            later: false,
            in_pinned: false,
            flagged: false,
        };
        assert_eq!(
            listed(&document),
            [
                (
                    "Inbox".into(),
                    String::new(),
                    place(inbox, ItemKind::Inbox, 0)
                ),
                ("Work".into(), String::new(), place(work, Area, 0)),
                ("Site".into(), "Work".into(), place(site, Project, 1)),
                ("Blog".into(), "Work › Site".into(), place(blog, Project, 2)),
                ("Solo".into(), String::new(), place(solo, Project, 0)),
            ]
        );
        Ok(())
    }

    // FR-VIEW-07, FR-VIEW-01, OQ-19
    #[test]
    fn counts_open_tasks_at_any_depth_leaving_out_the_trash_completed_and_inbox_scheduled_items()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = document.inbox();
        add(&mut document, Some(inbox), Task, "a")?;
        let mut scheduled = Item::new(Task, "later");
        scheduled.start = Some(date(2026, 10, 20));
        document.push(Some(inbox), Node::Item(scheduled))?;
        let work = add(&mut document, None, Area, "Work")?;
        let task = add(&mut document, Some(work), Task, "b")?;
        add(&mut document, Some(task), ChecklistItem, "not a task")?;
        let section = add(&mut document, Some(work), Section, "S")?;
        add(&mut document, Some(section), Task, "c")?;
        let mut done = Item::new(Task, "done");
        done.completed = Some(Completion::Undated);
        document.push(Some(section), Node::Item(done))?;
        document.push(Some(section), deleted(Task, "gone"))?;
        let project = add(&mut document, Some(work), Project, "P")?;
        add(&mut document, Some(project), Task, "d")?;
        let trashed = document.push(Some(work), deleted(Project, "Trashed"))?;
        add(&mut document, Some(trashed), Task, "e")?;
        add(&mut document, Some(trashed), Project, "Inner")?;
        document.push(None, deleted(Area, "Old"))?;
        let mut finished = Item::new(Project, "Finished");
        finished.completed = Some(Completion::Undated);
        let finished = document.push(Some(work), Node::Item(finished))?;
        add(&mut document, Some(finished), Task, "f")?;
        add(&mut document, Some(finished), Project, "Within")?;
        let counts: Vec<(String, usize)> = listed(&document)
            .into_iter()
            .map(|(text, _, place)| (text, place.open_tasks))
            .collect();
        assert_eq!(
            counts,
            [("Inbox".into(), 1), ("Work".into(), 3), ("P".into(), 1)]
        );
        Ok(())
    }

    // FR-VIEW-07
    #[test]
    fn opens_on_the_shown_container_or_the_closest_one_above_it_listed_or_on_the_view_shown()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = add(&mut document, None, Area, "Work")?;
        let site = add(&mut document, Some(work), Project, "Site")?;
        let section = add(&mut document, Some(site), Section, "Launch")?;
        let chosen = |shown| {
            let mut picker = open(&document, shown);
            picker.input(PickerInput::Choose)
        };
        let place = |id| {
            places(&document)
                .into_iter()
                .find(|candidate| candidate.value.id == id)
                .map(|candidate| Picked::Chosen(Stop::Place(candidate.value)))
        };
        let container = Shown::Container;
        assert_eq!(chosen(container(site)), place(site));
        assert_eq!(chosen(container(section)), place(site));
        assert_eq!(chosen(container(document.inbox())), place(document.inbox()));
        assert_eq!(
            chosen(container(ItemId::new(99))),
            place(document.inbox()),
            "with no entry it opens on the first"
        );
        assert_eq!(
            chosen(Shown::View(View::Logbook)),
            Some(Picked::Chosen(Stop::View(View::Logbook)))
        );
        Ok(())
    }

    // FR-VIEW-07
    #[test]
    fn lists_the_views_after_the_inbox() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = add(&mut document, None, Area, "Work")?;
        let listed: Vec<(String, Stop)> = stops(&document)
            .into_iter()
            .map(|candidate| (candidate.text, candidate.value))
            .collect();
        let place = |id, kind| {
            Stop::Place(Place {
                id,
                kind,
                depth: 0,
                open_tasks: 0,
                later: false,
                in_pinned: false,
                flagged: false,
            })
        };
        let mut expected = vec![("Inbox".to_owned(), place(document.inbox(), ItemKind::Inbox))];
        expected.extend(
            view::REGISTRY
                .iter()
                .map(|entry| (entry.title.to_owned(), Stop::View(entry.view))),
        );
        expected.push(("Work".to_owned(), place(work, Area)));
        assert_eq!(listed, expected);
        assert!(listed.contains(&("Logbook".to_owned(), Stop::View(View::Logbook))));
        Ok(())
    }

    // FR-VIEW-12, FR-ORG-12
    #[test]
    fn lists_the_pinned_projects_after_the_views_and_once_while_a_query_is_typed()
    -> Result<(), PlacementError> {
        let mut document = Document::new();
        let work = add(&mut document, None, Area, "Work")?;
        let mut site = Item::new(Project, "Site");
        (site.pinned, site.flagged) = (true, true);
        let site = document.push(Some(work), Node::Item(site))?;
        add(&mut document, Some(site), Task, "t")?;
        let mut done = Item::new(Project, "Done");
        (done.pinned, done.completed) = (true, Some(Completion::Undated));
        document.push(None, Node::Item(done))?;
        let listed: Vec<(String, String, Stop)> = stops(&document)
            .into_iter()
            .map(|candidate| (candidate.text, candidate.detail, candidate.value))
            .collect();
        // Each holds the one task.
        let place = |id, kind, depth, in_pinned| {
            Stop::Place(Place {
                id,
                kind,
                depth,
                open_tasks: 1,
                later: false,
                in_pinned,
                flagged: kind == Project,
            })
        };
        let after_views = listed.get(view::REGISTRY.len() + 1..);
        assert_eq!(
            after_views,
            Some(
                &[
                    ("Site".into(), "Work".into(), place(site, Project, 0, true)),
                    ("Work".into(), String::new(), place(work, Area, 0, false)),
                    ("Site".into(), "Work".into(), place(site, Project, 1, false)),
                ][..]
            ),
            "the completed project is left out"
        );
        let mut picker = open(&document, Shown::Container(site));
        assert_eq!(
            picker.input(PickerInput::Choose),
            Some(Picked::Chosen(place(site, Project, 0, true)))
        );
        let mut picker = reopen(&document, Shown::Container(site), "site");
        assert_eq!(picker.view().entries.len(), 1, "listed once");
        assert_eq!(
            picker.input(PickerInput::Choose),
            Some(Picked::Chosen(place(site, Project, 1, false)))
        );
        Ok(())
    }
}
