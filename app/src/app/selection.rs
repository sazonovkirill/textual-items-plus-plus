//! The selection: which items the next command acts on. Commands that act on items take the
//! picked rows, or the focused one when none is picked, as in taskpaper-terminal (FR-OPS-01,
//! D-18): extending with `Shift` and a motion picks a run of rows, and rows can be picked one by
//! one while plain motions move the focus and keep what is picked.

use super::container::Row;
use crate::domain::ItemId;

/// The items the next command acts on, and the focused one among them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    focused: Option<ItemId>,
    /// The rows picked, in the order they were picked; none when commands take the focused row.
    picked: Vec<ItemId>,
    /// The latest run of extensions, which the next extension continues while the focus is still
    /// where it ended.
    run: Option<Run>,
}

/// A run of extensions: the row it began at, the row it ended at and what was picked before it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Run {
    anchor: ItemId,
    end: ItemId,
    before: Vec<ItemId>,
}

impl Selection {
    /// The focused item, if any item is focused.
    #[must_use]
    pub fn focused(&self) -> Option<ItemId> {
        self.focused
    }

    /// Whether `id` is picked, which a frontend marks.
    #[must_use]
    pub fn is_picked(&self, id: ItemId) -> bool {
        self.picked.contains(&id)
    }

    /// Focuses `item`, or nothing, keeping what is picked.
    pub fn focus(&mut self, item: Option<ItemId>) {
        self.focused = item;
    }

    /// The items a command acts on, in the order of `rows`: the picked ones, or the focused one
    /// when none is picked and it is one of `rows`, not the container's heading.
    pub(super) fn items(&self, rows: &[Row]) -> Vec<ItemId> {
        if self.picked.is_empty() {
            return self
                .focused
                .filter(|&id| rows.iter().any(|row| row.id() == id))
                .into_iter()
                .collect();
        }
        rows.iter()
            .map(|row| row.id())
            .filter(|id| self.picked.contains(id))
            .collect()
    }

    /// Moves the focus to `target` among `rows` and picks every row from where the run of
    /// extensions began to it, on top of what was picked before the run, so that moving back
    /// shrinks the run. A run begins at the focus unless the focus is still where the latest run
    /// ended. A `target` that is the focus, as at the first or last row, picks the focused row.
    /// From a focus that is no row, as the container's heading, it only moves the focus.
    pub(super) fn extend(&mut self, rows: &[Row], target: Option<ItemId>) {
        let at = |id: ItemId| rows.iter().position(|row| row.id() == id);
        let (Some(focused), Some(target)) = (self.focused, target) else {
            return;
        };
        if at(focused).is_none() {
            self.focused = Some(target);
            self.run = None;
            return;
        }
        let run = match self.run.take() {
            Some(run) if run.end == focused => run,
            _ => Run {
                anchor: focused,
                end: focused,
                before: self.picked.clone(),
            },
        };
        let span = at(run.anchor)
            .zip(at(target))
            .and_then(|(from, to)| rows.get(from.min(to)..=from.max(to)))
            .unwrap_or_default()
            .iter()
            .map(|row| row.id());
        self.picked.clone_from(&run.before);
        for id in span {
            if !self.picked.contains(&id) {
                self.picked.push(id);
            }
        }
        self.focused = Some(target);
        self.run = Some(Run { end: target, ..run });
    }

    /// Picks the focused row among `rows`, or unpicks it when it is picked.
    pub(super) fn toggle(&mut self, rows: &[Row]) {
        let Some(focused) = self
            .focused
            .filter(|&id| rows.iter().any(|row| row.id() == id))
        else {
            return;
        };
        if let Some(at) = self.picked.iter().position(|&id| id == focused) {
            self.picked.remove(at);
        } else {
            self.picked.push(focused);
        }
        self.run = None;
    }

    /// Picks every row of `rows`.
    pub(super) fn select_all(&mut self, rows: &[Row]) {
        self.picked = rows.iter().map(|row| row.id()).collect();
        self.run = None;
    }

    /// Picks nothing, so that commands take the focused row again.
    pub(super) fn clear(&mut self) {
        self.picked.clear();
        self.run = None;
    }

    /// Maps the picked items and the run of extensions through `found`, as after a reload, and
    /// unpicks those it does not find and those no row of `rows` shows, as when they were
    /// deleted, hidden or moved away; the run ends when its anchor goes.
    pub(super) fn keep(&mut self, rows: &[Row], found: impl Fn(ItemId) -> Option<ItemId>) {
        let shown = |id: &ItemId| rows.iter().any(|row| row.id() == *id);
        let kept = |ids: &[ItemId]| -> Vec<ItemId> {
            ids.iter()
                .filter_map(|&id| found(id))
                .filter(shown)
                .collect()
        };
        self.picked = kept(&self.picked);
        self.run = self.run.take().and_then(|run| {
            Some(Run {
                anchor: found(run.anchor).filter(shown)?,
                end: found(run.end)?,
                before: kept(&run.before),
            })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(number: u64) -> ItemId {
        ItemId::new(number)
    }

    /// Five task rows, numbered 1 to 5.
    fn rows() -> Vec<Row> {
        (1..=5)
            .map(|number| Row::Task {
                id: id(number),
                section: None,
            })
            .collect()
    }

    fn focused_on(number: u64) -> Selection {
        let mut selection = Selection::default();
        selection.focus(Some(id(number)));
        selection
    }

    fn items(selection: &Selection) -> Vec<u64> {
        selection
            .items(&rows())
            .into_iter()
            .map(|id| {
                (1..=5)
                    .find(|&number| ItemId::new(number) == id)
                    .unwrap_or(0)
            })
            .collect()
    }

    #[test]
    fn a_new_selection_holds_nothing() {
        let selection = Selection::default();
        assert_eq!(selection.focused(), None);
        assert_eq!(selection.items(&rows()), Vec::<ItemId>::new());
    }

    // FR-OPS-01
    #[test]
    fn with_nothing_picked_commands_take_the_focused_row() {
        let mut selection = focused_on(1);
        selection.focus(Some(id(2)));
        assert_eq!(selection.focused(), Some(id(2)));
        assert_eq!(items(&selection), [2]);
        selection.focus(None);
        assert_eq!(items(&selection), Vec::<u64>::new());
    }

    // FR-OPS-01
    #[test]
    fn extending_picks_the_run_and_moving_back_shrinks_it() {
        let rows = rows();
        let mut selection = focused_on(2);
        selection.extend(&rows, Some(id(3)));
        selection.extend(&rows, Some(id(4)));
        assert_eq!(items(&selection), [2, 3, 4]);
        assert_eq!(selection.focused(), Some(id(4)));
        selection.extend(&rows, Some(id(3)));
        assert_eq!(items(&selection), [2, 3]);
        selection.extend(&rows, Some(id(2)));
        selection.extend(&rows, Some(id(1)));
        assert_eq!(
            items(&selection),
            [1, 2],
            "past the anchor it grows the other way"
        );
    }

    // FR-OPS-01
    #[test]
    fn at_the_edge_extending_picks_the_focused_row() {
        let mut selection = focused_on(5);
        selection.extend(&rows(), Some(id(5)));
        assert_eq!(items(&selection), [5]);
        assert!(selection.is_picked(id(5)));
        let mut nothing = Selection::default();
        nothing.extend(&rows(), Some(id(1)));
        assert_eq!(
            nothing,
            Selection::default(),
            "with no focus nothing is picked"
        );
    }

    // FR-OPS-01
    #[test]
    fn a_run_begun_elsewhere_adds_to_what_was_picked() {
        let rows = rows();
        let mut selection = focused_on(1);
        selection.extend(&rows, Some(id(2)));
        selection.focus(Some(id(4)));
        selection.extend(&rows, Some(id(5)));
        assert_eq!(items(&selection), [1, 2, 4, 5]);
        selection.extend(&rows, Some(id(4)));
        assert_eq!(
            items(&selection),
            [1, 2, 4],
            "the new run shrinks, the old one stays"
        );
        selection.focus(Some(id(1)));
        selection.focus(Some(id(4)));
        selection.extend(&rows, Some(id(3)));
        assert_eq!(
            items(&selection),
            [1, 2, 3, 4],
            "back where the run ended, it goes on"
        );
    }

    // FR-OPS-01
    #[test]
    fn rows_are_picked_one_by_one_and_all_at_once() {
        let rows = rows();
        let mut selection = focused_on(4);
        selection.toggle(&rows);
        selection.focus(Some(id(2)));
        selection.toggle(&rows);
        assert_eq!(items(&selection), [2, 4], "in the order of the rows");
        selection.toggle(&rows);
        assert_eq!(items(&selection), [4]);
        selection.select_all(&rows);
        assert_eq!(items(&selection), [1, 2, 3, 4, 5]);
        selection.clear();
        assert_eq!(items(&selection), [2], "the focused row again");
        Selection::default().toggle(&rows);
    }

    // FR-OPS-01, D-68
    #[test]
    fn a_focus_on_no_row_is_not_picked_nor_acted_on_and_extending_from_it_only_moves() {
        let rows = rows();
        let mut heading = focused_on(99);
        heading.toggle(&rows);
        assert_eq!(items(&heading), Vec::<u64>::new());
        heading.extend(&rows, Some(id(1)));
        assert_eq!(heading.focused(), Some(id(1)));
        assert_eq!(items(&heading), [1], "nothing picked");
        heading.extend(&rows, Some(id(2)));
        assert_eq!(items(&heading), [1, 2], "a run from the first row");
    }

    // FR-OPS-01
    #[test]
    fn an_item_no_row_shows_is_unpicked() {
        let rows = rows();
        let mut selection = focused_on(2);
        selection.extend(&rows, Some(id(3)));
        selection.extend(&rows, Some(id(4)));
        let fewer: Vec<Row> = rows.into_iter().filter(|row| row.id() != id(2)).collect();
        selection.keep(&fewer, Some);
        assert_eq!(items(&selection), [3, 4]);
        selection.extend(&fewer, Some(id(5)));
        assert_eq!(
            items(&selection),
            [3, 4, 5],
            "with its anchor gone the run starts anew"
        );
        selection.keep(&fewer, |found| (found != id(3)).then_some(found));
        assert_eq!(
            items(&selection),
            [4, 5],
            "what is not found again goes too"
        );
    }

    // FR-OPS-01
    #[test]
    fn an_item_unpicked_for_leaving_the_rows_stays_unpicked_as_the_run_goes_on() {
        let rows = rows();
        let mut selection = focused_on(1);
        selection.toggle(&rows);
        selection.focus(Some(id(3)));
        selection.extend(&rows, Some(id(4)));
        let fewer: Vec<Row> = rows.into_iter().filter(|row| row.id() != id(1)).collect();
        selection.keep(&fewer, Some);
        selection.extend(&fewer, Some(id(5)));
        assert_eq!(items(&selection), [3, 4, 5]);
        selection.keep(&fewer, |found| (found != id(5)).then_some(found));
        assert_eq!(
            items(&selection),
            [3, 4],
            "nor when its end is not found again"
        );
    }
}
