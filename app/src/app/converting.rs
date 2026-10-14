//! Converting and grouping: what kind the selected items convert to, and where the section or
//! project that groups them goes, or why the hierarchy refuses either (FR-TASK-06, FR-ORG-06,
//! FR-OPS-07, FR-OPS-08, OQ-2).

use super::{
    editing,
    moving::{self, Refusal},
};
use crate::domain::{Document, ItemId, ItemKind, Node, Transaction};

/// The transaction that converts each of `items` in its place to a project, or to a section when
/// every one is a project (D-25); items already of that kind
/// stay as they are, and so does each one the hierarchy refuses, or whose title, as a task's
/// can, starts with `-` and a space, which would read back as a task (FR-TASK-06, FR-ORG-06,
/// D-47).
///
/// # Errors
///
/// The [`Refusal`] of the first item refused when none converts.
pub(super) fn convert_all(document: &Document, items: &[ItemId]) -> Result<Transaction, Refusal> {
    let all_projects = !items.is_empty()
        && items
            .iter()
            .all(|&id| kind(document, id) == Some(ItemKind::Project));
    let to = if all_projects {
        ItemKind::Section
    } else {
        ItemKind::Project
    };
    match moving::each(document, items.iter().copied(), |document, id| {
        let title = document
            .node(id)
            .and_then(Node::item)
            .map(|item| item.title.as_str());
        if title.is_some_and(editing::reads_as_task) {
            return Err(Refusal::ReadsAsTask);
        }
        document.convert(id, to).map_err(|_| {
            if fits(document, id, to) {
                Refusal::SectionHoldsTasks
            } else {
                no_room(to)
            }
        })
    }) {
        (transaction, Some(refusal)) if transaction.ops().is_empty() => Err(refusal),
        (transaction, _) => Ok(transaction),
    }
}

/// Where a new item of `kind`, a section or a project, goes to group `items`, given in the order
/// of the rows: where the first of them is, among the children of its parent (D-25). A section
/// groups only tasks in a project (FR-OPS-07); a project groups tasks, sections and
/// projects anywhere but in a section or the Inbox (FR-OPS-08). `None` with no items.
///
/// # Errors
///
/// The [`Refusal`] when the hierarchy does not allow the new item there or the items in it.
pub(super) fn group_place(
    document: &Document,
    items: &[ItemId],
    kind: ItemKind,
) -> Result<Option<(Option<ItemId>, usize)>, Refusal> {
    let Some(&first) = items.first() else {
        return Ok(None);
    };
    let refused = if kind == ItemKind::Section {
        items.iter().find_map(|&id| {
            if self::kind(document, id) != Some(ItemKind::Task) {
                Some(Refusal::SectionHoldsTasks)
            } else if !fits(document, id, kind) {
                Some(Refusal::NoRoomForSection)
            } else {
                None
            }
        })
    } else {
        (!fits(document, first, kind)).then_some(Refusal::NoRoomForProject)
    };
    refused.map_or_else(|| Ok(document.position(first)), Err)
}

/// Why an item of `kind` can't go where it would.
fn no_room(kind: ItemKind) -> Refusal {
    if kind == ItemKind::Section {
        Refusal::NoRoomForSection
    } else {
        Refusal::NoRoomForProject
    }
}

/// Whether an item of `kind` may go beside `id`, among the children of its parent.
fn fits(document: &Document, id: ItemId, kind: ItemKind) -> bool {
    let parent = document.position(id).and_then(|(parent, _)| parent);
    kind.fits_in(parent.and_then(|parent| self::kind(document, parent)))
}

/// The kind of the item `id`, or `None` when it is no item.
fn kind(document: &Document, id: ItemId) -> Option<ItemKind> {
    document.node(id).and_then(Node::item).map(|item| item.kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Item, PlacementError};
    use ItemKind::{ChecklistItem, Project, Section, Task};

    /// The Inbox holds `inboxed`; project Work holds tasks `paint` and `order`, section `section`
    /// holding task `sell`, and projects `site` and `holder`, which holds a section; project `top`
    /// is at the top level.
    struct Fixture {
        document: Document,
        inboxed: ItemId,
        work: ItemId,
        paint: ItemId,
        order: ItemId,
        section: ItemId,
        sell: ItemId,
        site: ItemId,
        holder: ItemId,
        top: ItemId,
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let item = |kind, title| Node::Item(Item::new(kind, title));
        let inboxed = document.push(Some(document.inbox()), item(Task, "inboxed"))?;
        let work = document.push(None, item(Project, "Work"))?;
        let paint = document.push(Some(work), item(Task, "paint"))?;
        let order = document.push(Some(work), item(Task, "order"))?;
        let section = document.push(Some(work), item(Section, "Soon"))?;
        let sell = document.push(Some(section), item(Task, "sell"))?;
        let site = document.push(Some(work), item(Project, "site"))?;
        let holder = document.push(Some(work), item(Project, "holder"))?;
        document.push(Some(holder), item(Section, "inner"))?;
        let top = document.push(None, item(Project, "top"))?;
        Ok(Fixture {
            document,
            inboxed,
            work,
            paint,
            order,
            section,
            sell,
            site,
            holder,
            top,
        })
    }

    /// `document` with `items` converted, some of which the hierarchy allows.
    fn converted(document: &Document, items: &[ItemId]) -> Document {
        let mut document = document.clone();
        let transaction = convert_all(&document, items).expect("some item converts");
        document
            .apply(&transaction)
            .expect("a transaction made from the document as it is fits it");
        document
    }

    // FR-TASK-06, FR-ORG-06
    #[test]
    fn items_become_projects_or_all_projects_sections() -> Result<(), PlacementError> {
        let Fixture {
            document,
            paint,
            section,
            site,
            ..
        } = fixture()?;
        let mixed = converted(&document, &[paint, section, site]);
        for id in [paint, section, site] {
            assert_eq!(kind(&mixed, id), Some(Project), "{id:?}");
        }
        let back = converted(&mixed, &[paint, section]);
        assert_eq!(kind(&back, paint), Some(Section));
        assert_eq!(kind(&back, section), Some(Section));
        assert_eq!(convert_all(&document, &[]), Ok(Transaction::default()));
        Ok(())
    }

    // FR-TASK-06, FR-ORG-06, OQ-2
    #[test]
    fn only_refused_items_stay_and_all_refused_says_why() -> Result<(), PlacementError> {
        let Fixture {
            document,
            inboxed,
            work,
            paint,
            sell,
            holder,
            top,
            site,
            ..
        } = fixture()?;
        let some = converted(&document, &[sell, paint]);
        assert_eq!(
            kind(&some, sell),
            Some(Task),
            "a project can't go in paint section"
        );
        assert_eq!(kind(&some, paint), Some(Project));
        for (items, refusal) in [
            (vec![inboxed], Refusal::NoRoomForProject),
            (vec![sell], Refusal::NoRoomForProject),
            (vec![top], Refusal::NoRoomForSection),
            (vec![holder], Refusal::SectionHoldsTasks),
            (vec![holder, top], Refusal::SectionHoldsTasks),
        ] {
            assert_eq!(convert_all(&document, &items), Err(refusal), "{items:?}");
        }
        let mut dashed = document.clone();
        let plan = dashed.push(Some(work), Node::Item(Item::new(Task, "- Plan")))?;
        assert_eq!(
            convert_all(&dashed, &[plan]),
            Err(Refusal::ReadsAsTask),
            "a project titled so would read back as a task"
        );
        let one = converted(&document, &[top, site]);
        assert_eq!(
            (kind(&one, top), kind(&one, site)),
            (Some(Project), Some(Section))
        );
        Ok(())
    }

    // FR-OPS-07, FR-OPS-08, D-25
    #[test]
    fn a_group_goes_where_its_first_item_is() -> Result<(), PlacementError> {
        let Fixture {
            document,
            work,
            paint,
            order,
            section,
            site,
            ..
        } = fixture()?;
        assert_eq!(
            group_place(&document, &[order, paint], Section),
            Ok(Some((Some(work), 1)))
        );
        assert_eq!(
            group_place(&document, &[section, paint, site], Project),
            Ok(Some((Some(work), 2)))
        );
        assert_eq!(group_place(&document, &[], Section), Ok(None));
        Ok(())
    }

    // FR-OPS-07, FR-OPS-08, OQ-2
    #[test]
    fn a_group_where_its_kind_cannot_go_or_hold_the_items_is_refused() -> Result<(), PlacementError>
    {
        let Fixture {
            document,
            inboxed,
            paint,
            section,
            sell,
            site,
            ..
        } = fixture()?;
        for (items, kind, refusal) in [
            (vec![paint, sell], Section, Refusal::NoRoomForSection),
            (vec![inboxed], Section, Refusal::NoRoomForSection),
            (vec![paint, section], Section, Refusal::SectionHoldsTasks),
            (vec![site], Section, Refusal::SectionHoldsTasks),
            (vec![sell, paint], Project, Refusal::NoRoomForProject),
            (vec![inboxed], Project, Refusal::NoRoomForProject),
        ] {
            assert_eq!(
                group_place(&document, &items, kind),
                Err(refusal),
                "{items:?} into {kind:?}"
            );
        }
        let mut checklist = document.clone();
        let check = checklist.push(Some(paint), Node::Item(Item::new(ChecklistItem, "x")))?;
        assert_eq!(
            group_place(&checklist, &[check], Section),
            Err(Refusal::SectionHoldsTasks)
        );
        Ok(())
    }
}
