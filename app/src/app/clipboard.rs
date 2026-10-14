//! The clipboard commands: duplicating, copying, cutting and pasting the selected items with
//! everything inside them (FR-OPS-05). Hides which items they take and where copies go; the
//! frontend keeps the clipboard itself.

use super::{
    End, Side, adding,
    container::{self, Row},
    trash,
};
use crate::domain::{Branch, Document, ItemId, ItemKind, Op, Transaction};

/// The items `taken`, which [`container::outermost_rows`] gives, with their contents, apart from
/// the document, as copying puts them on the clipboard.
pub(super) fn branches(document: &Document, taken: &[ItemId]) -> Vec<Branch> {
    taken
        .iter()
        .map(|&id| {
            document
                .branch(id)
                .expect("the items taken are in the document")
        })
        .collect()
}

/// The transaction that takes the items `taken` out of the document, as cutting does once they
/// are on the clipboard.
pub(super) fn cut(document: &Document, taken: &[ItemId]) -> Transaction {
    let mut scratch = document.clone();
    let ops = taken
        .iter()
        .map(|&id| {
            let (parent, index) = scratch
                .position(id)
                .expect("the items taken are in the document");
            let subtree = scratch
                .snapshot(id)
                .expect("the items taken are in the document");
            let remove = Op::Remove {
                parent,
                index,
                subtree,
            };
            apply(&mut scratch, remove.clone());
            remove
        })
        .collect();
    Transaction::new(ops)
}

/// The transaction that puts a copy of each of the items `taken` right after it, and the copy of
/// the last one.
pub(super) fn duplicate(
    document: &mut Document,
    taken: &[ItemId],
) -> (Transaction, Option<ItemId>) {
    let copies: Vec<_> = taken
        .iter()
        .map(|&id| {
            let branch = document
                .branch(id)
                .expect("the items taken are in the document");
            (id, document.graft(&branch))
        })
        .collect();
    let last = copies.last().map(|(_, copy)| copy.id());
    let mut scratch = document.clone();
    let ops = copies
        .into_iter()
        .map(|(id, subtree)| {
            let (parent, index) = scratch
                .position(id)
                .expect("the items taken are in the document");
            let insert = Op::Insert {
                parent,
                index: index + 1,
                subtree,
            };
            apply(&mut scratch, insert.clone());
            insert
        })
        .collect();
    (Transaction::new(ops), last)
}

/// Where a paste goes: the container shown, whether it shows completed items, its focused row
/// and the side of it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Target {
    pub(super) container: ItemId,
    pub(super) show_completed: bool,
    pub(super) focused: Option<ItemId>,
    pub(super) side: Side,
}

/// The transaction that puts `branches` in on the target's side of its focused row, in order, and
/// the last of them the container shows, if any; `None` when one of them can go nowhere near, as a section pasted in the
/// Inbox, or into a container whose contents are hidden for being completed (FR-ORG-09), and
/// then nothing is pasted. Each goes where a new item of its kind would (FR-EDIT-05, FR-ORG-06),
/// after the one before it when that is a row, and never before an earlier one in the same
/// place; text kept as read goes as a task would. Nothing pasted goes in the Trash (D-113).
pub(super) fn paste(
    document: &mut Document,
    target: Target,
    branches: &[Branch],
) -> Option<(Transaction, Option<ItemId>)> {
    let subtrees: Vec<_> = branches
        .iter()
        .map(|branch| (kind(branch), document.graft(&trash::live(branch))))
        .collect();
    let mut scratch = document.clone();
    let mut ops = Vec::new();
    let mut pasted: Vec<ItemId> = Vec::new();
    for (kind, subtree) in subtrees {
        let previous = pasted.last().copied();
        let rows = container::rows(&scratch, target.container, target.show_completed);
        let (focused, side) = match previous {
            Some(id) if rows.iter().any(|row| row.id() == id) => (Some(id), Side::Below),
            _ => (target.focused, target.side),
        };
        let (parent, index) = place(&scratch, target.container, &rows, focused, kind, side)?;
        if parent.is_some_and(|parent| scratch.conceals(parent)) {
            return None;
        }
        // Never before an item pasted earlier into the same place, so that they keep their order.
        let index = match previous.and_then(|id| scratch.position(id)) {
            Some((earlier, at)) if earlier == parent => index.max(at + 1),
            _ => index,
        };
        pasted.push(subtree.id());
        let insert = Op::Insert {
            parent,
            index,
            subtree,
        };
        apply(&mut scratch, insert.clone());
        ops.push(insert);
    }
    let rows = container::rows(&scratch, target.container, target.show_completed);
    let shown = pasted
        .iter()
        .rev()
        .copied()
        .find(|&id| rows.iter().any(|row| row.id() == id));
    (!pasted.is_empty()).then(|| (Transaction::new(ops), shown))
}

/// What the top node of `branch` is: `None` for text kept as read.
fn kind(branch: &Branch) -> Option<ItemKind> {
    branch.node.item().map(|item| item.kind)
}

/// The parent, `None` for the top level, and index where an item of `kind` goes on `side` of
/// `focused`, as a new one would; `None` when it can go nowhere near. Below the title row of
/// `container`, its heading, it goes first of its kind there, as `o` adds its first task, and above
/// it nowhere, as `O` adds nothing (D-68, D-103).
fn place(
    document: &Document,
    container: ItemId,
    rows: &[Row],
    focused: Option<ItemId>,
    kind: Option<ItemKind>,
    side: Side,
) -> Option<(Option<ItemId>, usize)> {
    match kind {
        Some(ItemKind::Inbox | ItemKind::ChecklistItem) => None,
        _ if focused == Some(container) => match side {
            Side::Below => adding::end_place(
                document,
                container,
                kind.unwrap_or(ItemKind::Task),
                End::Top,
            ),
            Side::Above => None,
        },
        None | Some(ItemKind::Task) => {
            let (parent, index) = adding::place(document, container, rows, focused, side);
            Some((Some(parent), index))
        }
        Some(kind @ (ItemKind::Section | ItemKind::Project | ItemKind::Area)) => {
            adding::container_place(document, container, rows, focused, kind, side)
        }
    }
}

/// Applies `op`, which was made from `document` as it is.
fn apply(document: &mut Document, op: Op) {
    document
        .apply(&Transaction::new(vec![op]))
        .expect("an operation made from the document as it is fits it");
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};

    use super::*;
    use crate::app::{Command, Effect, Focus, Message, Opened, State, update};
    use crate::domain::{Item, Node, PlacementError};
    use ItemKind::{Area, ChecklistItem, Project, Section, Task};

    const TODAY: Date = date(2026, 10, 14);

    /// The Inbox holds tasks `a` and `b`; area Work holds task `loose`, section `Soon` with task
    /// `s`, which has a description and a checklist item, and project `Site` with task `p`.
    struct Fixture {
        state: State,
        work: ItemId,
        section: ItemId,
        s: ItemId,
    }

    fn fixture() -> Result<Fixture, PlacementError> {
        let mut document = Document::new();
        let add = |document: &mut Document, parent, kind, title| {
            document.push(parent, Node::Item(Item::new(kind, title)))
        };
        let inbox = Some(document.inbox());
        add(&mut document, inbox, Task, "a")?;
        add(&mut document, inbox, Task, "b")?;
        let work = add(&mut document, None, Area, "Work")?;
        add(&mut document, Some(work), Task, "loose")?;
        let section = add(&mut document, Some(work), Section, "Soon")?;
        let mut item = Item::new(Task, "s");
        item.description = "why".to_owned();
        let s = document.push(Some(section), Node::Item(item))?;
        add(&mut document, Some(s), ChecklistItem, "c")?;
        let site = add(&mut document, Some(work), Project, "Site")?;
        add(&mut document, Some(site), Task, "p")?;
        Ok(Fixture {
            state: State::new(document, Opened::Demo, TODAY),
            work,
            section,
            s,
        })
    }

    fn send(state: &mut State, message: Message) -> Vec<Effect> {
        update(state, message, TODAY)
    }

    fn give(state: &mut State, command: Command) -> Vec<Effect> {
        send(state, Message::Command(command))
    }

    fn title(state: &State, id: ItemId) -> &str {
        state
            .document()
            .node(id)
            .and_then(Node::item)
            .map_or("", |item| item.title.as_str())
    }

    /// The titles of the rows shown.
    fn titles(state: &State) -> Vec<&str> {
        state
            .rows()
            .iter()
            .map(|row| title(state, row.id()))
            .collect()
    }

    /// Every node of the document, apart from the identifiers it has handed out.
    fn tree(state: &State) -> Vec<Option<Branch>> {
        let document = state.document();
        std::iter::once(document.inbox())
            .chain(document.top_level().iter().copied())
            .map(|id| document.branch(id))
            .collect()
    }

    fn focused(state: &State) -> &str {
        state
            .selection()
            .focused()
            .map_or("", |id| title(state, id))
    }

    /// Opens `place` through the Navigator.
    fn open(state: &mut State, place: &str) {
        give(state, Command::Navigator);
        for c in place.chars() {
            send(
                state,
                Message::Navigator(crate::app::PickerInput::Edit(crate::app::TextEdit::Insert(
                    c,
                ))),
            );
        }
        send(state, Message::Navigator(crate::app::PickerInput::Choose));
    }

    // FR-OPS-05, FR-EDIT-06
    #[test]
    fn duplicating_puts_a_copy_with_its_contents_after_the_item_and_focuses_it()
    -> Result<(), PlacementError> {
        let Fixture { mut state, s, .. } = fixture()?;
        open(&mut state, "Work");
        give(&mut state, Command::Focus(Focus::Next));
        give(&mut state, Command::Focus(Focus::Next));
        assert_eq!(give(&mut state, Command::Duplicate), []);
        assert_eq!(titles(&state), ["loose", "Soon", "s", "s", "Site"]);
        let copy = state.selection().focused().filter(|&id| id != s);
        let copy = copy.expect("the copy is focused");
        assert_eq!(state.document().branch(copy), state.document().branch(s));
        give(&mut state, Command::Undo);
        assert_eq!(titles(&state), ["loose", "Soon", "s", "Site"]);
        assert_eq!(state.selection().focused(), Some(s));
        Ok(())
    }

    // FR-OPS-05
    #[test]
    fn copying_gives_the_selected_items_with_their_contents_and_changes_nothing()
    -> Result<(), PlacementError> {
        let Fixture {
            mut state, section, ..
        } = fixture()?;
        open(&mut state, "Work");
        give(&mut state, Command::Focus(Focus::Next));
        let before = state.document().clone();
        let branches = state.document().branch(section).into_iter().collect();
        assert_eq!(
            give(&mut state, Command::Copy),
            [Effect::SetClipboard(branches)]
        );
        assert_eq!(state.document(), &before);
        Ok(())
    }

    // FR-OPS-05, FR-EDIT-06
    #[test]
    fn cutting_takes_the_items_out_and_pasting_puts_them_back_elsewhere()
    -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        let a = state.selection().focused();
        let branches: Vec<_> = a
            .and_then(|a| state.document().branch(a))
            .into_iter()
            .collect();
        assert_eq!(
            give(&mut state, Command::Cut),
            [Effect::SetClipboard(branches.clone())]
        );
        assert_eq!(titles(&state), ["b"]);
        assert_eq!(focused(&state), "b");
        assert_eq!(
            give(&mut state, Command::Paste(Side::Below)),
            [Effect::ReadClipboard(Side::Below)]
        );
        send(&mut state, Message::Paste(Side::Below, branches.clone()));
        assert_eq!(titles(&state), ["b", "a"]);
        assert_eq!(focused(&state), "a");
        send(&mut state, Message::Paste(Side::Above, branches));
        assert_eq!(titles(&state), ["b", "a", "a"]);
        give(&mut state, Command::Undo);
        give(&mut state, Command::Undo);
        give(&mut state, Command::Undo);
        assert_eq!(titles(&state), ["a", "b"], "each is one step to undo");
        Ok(())
    }

    // FR-OPS-05
    #[test]
    fn nothing_is_copied_or_cut_without_a_row_selected() -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        open(&mut state, "Site");
        give(&mut state, Command::Delete);
        assert_eq!(titles(&state), Vec::<&str>::new());
        let before = state.clone();
        assert_eq!(give(&mut state, Command::Copy), []);
        assert_eq!(give(&mut state, Command::Cut), []);
        assert_eq!(give(&mut state, Command::Duplicate), []);
        assert_eq!(state.document(), before.document());
        Ok(())
    }

    fn task(title: &str) -> Branch {
        Branch {
            node: Node::Item(Item::new(Task, title)),
            children: Vec::new(),
        }
    }

    fn container(kind: ItemKind, title: &str, children: Vec<Branch>) -> Branch {
        Branch {
            node: Node::Item(Item::new(kind, title)),
            children,
        }
    }

    // FR-OPS-05
    #[test]
    fn pasted_items_go_where_new_ones_of_their_kind_would_in_order() -> Result<(), PlacementError> {
        let Fixture {
            mut state, work, ..
        } = fixture()?;
        open(&mut state, "Work");
        let kept = Branch {
            node: Node::Kept("kept".to_owned()),
            children: Vec::new(),
        };
        let pasted = vec![
            task("t1"),
            task("t2"),
            container(Section, "S2", vec![task("u")]),
            container(Project, "P2", Vec::new()),
            kept,
        ];
        send(&mut state, Message::Paste(Side::Below, pasted));
        assert_eq!(
            titles(&state),
            ["loose", "t1", "t2", "S2", "u", "Soon", "s", "P2", "Site"],
            "a section or project pasted after tasks comes first of its kind"
        );
        assert_eq!(focused(&state), "P2", "kept text is no row to focus");
        let document = state.document();
        let children: Vec<_> = document
            .children(work)
            .iter()
            .map(|&id| {
                document
                    .node(id)
                    .and_then(Node::item)
                    .map(|item| item.title.as_str())
            })
            .collect();
        assert_eq!(
            children,
            [
                Some("loose"),
                Some("t1"),
                Some("t2"),
                Some("S2"),
                Some("Soon"),
                Some("P2"),
                None,
                Some("Site")
            ],
            "kept text goes as a task would, after what was pasted before it"
        );
        Ok(())
    }

    // FR-OPS-05, D-68, D-103
    #[test]
    fn pasted_below_the_title_row_items_go_first_and_above_it_nothing() -> Result<(), PlacementError>
    {
        let Fixture {
            mut state, work, ..
        } = fixture()?;
        open(&mut state, "Work");
        give(&mut state, Command::Focus(Focus::First));
        assert_eq!(state.selection().focused(), Some(work));
        let before = tree(&state);
        send(&mut state, Message::Paste(Side::Above, vec![task("t")]));
        assert_eq!(tree(&state), before, "no row stands above the title row");
        assert_eq!(state.selection().focused(), Some(work));
        let pasted = vec![task("t1"), task("t2")];
        send(&mut state, Message::Paste(Side::Below, pasted));
        assert_eq!(titles(&state), ["t1", "t2", "loose", "Soon", "s", "Site"]);
        assert_eq!(focused(&state), "t2");
        give(&mut state, Command::Focus(Focus::First));
        let pasted = vec![container(Section, "S2", Vec::new()), task("t3")];
        send(&mut state, Message::Paste(Side::Below, pasted));
        assert_eq!(
            titles(&state),
            ["t1", "t2", "loose", "S2", "t3", "Soon", "s", "Site"],
            "a section goes first of its kind"
        );
        Ok(())
    }

    // FR-OPS-05
    #[test]
    fn a_project_pasted_in_the_inbox_goes_to_the_top_level_and_an_area_beside_it()
    -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        let pasted = vec![
            container(Project, "P2", Vec::new()),
            container(Area, "A2", Vec::new()),
        ];
        send(&mut state, Message::Paste(Side::Above, pasted));
        let top: Vec<&str> = state
            .document()
            .top_level()
            .iter()
            .map(|&id| title(&state, id))
            .collect();
        assert_eq!(top, ["P2", "A2", "Work"]);
        assert_eq!(focused(&state), "a", "the pane shows neither");
        Ok(())
    }

    // FR-OPS-05, FR-ORG-09
    #[test]
    fn nothing_is_pasted_when_an_item_can_go_nowhere_near() -> Result<(), PlacementError> {
        let Fixture { mut state, .. } = fixture()?;
        let before = tree(&state);
        let pasted = vec![task("t"), container(Section, "S2", Vec::new())];
        send(&mut state, Message::Paste(Side::Below, pasted));
        assert_eq!(tree(&state), before, "no section goes in the Inbox");
        send(&mut state, Message::Paste(Side::Below, Vec::new()));
        assert_eq!(tree(&state), before);
        let checklist = container(ChecklistItem, "c", Vec::new());
        send(&mut state, Message::Paste(Side::Below, vec![checklist]));
        assert_eq!(tree(&state), before, "a checklist item goes only in a task");
        // Inside a completed section nothing shows, so nothing goes.
        open(&mut state, "Work");
        give(&mut state, Command::ShowCompleted);
        give(&mut state, Command::Focus(Focus::Next));
        give(&mut state, Command::ToggleCompleted);
        assert_eq!(focused(&state), "Soon");
        let before = tree(&state);
        send(&mut state, Message::Paste(Side::Below, vec![task("t")]));
        assert_eq!(tree(&state), before);
        Ok(())
    }
}
