//! Moving the focus through the rows of a container view with the motions of FR-NAV-01.

use super::container::Row;
use crate::domain::ItemId;

/// Where a motion takes the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Focus {
    /// The next row.
    Next,
    /// The row before.
    Previous,
    /// The first row.
    First,
    /// The last row.
    Last,
    /// The heading of the section the focused task is listed under.
    Parent,
    /// The next section heading.
    NextSection,
    /// The section heading above.
    PreviousSection,
}

/// The row `motion` focuses when `focused` has the focus among `rows`, or `focused` itself when
/// the motion has nowhere to go. With no row focused, every motion focuses the first; with no
/// rows, nothing.
pub(super) fn target(rows: &[Row], focused: Option<ItemId>, motion: Focus) -> Option<ItemId> {
    let Some(at) = focused.and_then(|id| rows.iter().position(|row| row.id() == id)) else {
        return rows.first().copied().map(Row::id);
    };
    let is_section = |row: &&Row| matches!(row, Row::Section(_));
    let row = match motion {
        Focus::Next => rows.get(at + 1),
        Focus::Previous => at.checked_sub(1).and_then(|before| rows.get(before)),
        Focus::First => rows.first(),
        Focus::Last => rows.last(),
        Focus::Parent => {
            return match rows.get(at) {
                Some(&Row::Task {
                    section: Some(section),
                    ..
                }) => Some(section),
                _ => focused,
            };
        }
        Focus::NextSection => rows.iter().skip(at + 1).find(is_section),
        Focus::PreviousSection => rows.iter().take(at).rfind(is_section),
    };
    row.copied().map(Row::id).or(focused)
}

/// The row to focus once the rows `before` have become `after`: `focused` while it is still a
/// row, else the nearest row after it that remains, else the nearest before it, so that hiding
/// the focused row moves the focus on as a list does when an item leaves it. With nothing of
/// that to go to, the first row.
pub(super) fn kept(before: &[Row], after: &[Row], focused: Option<ItemId>) -> Option<ItemId> {
    let remains = |id: &ItemId| after.iter().any(|row| row.id() == *id);
    if let Some(id) = focused.filter(remains) {
        return Some(id);
    }
    let at = focused.and_then(|id| before.iter().position(|row| row.id() == id));
    at.and_then(|at| {
        let (earlier, later) = before.split_at(at);
        later
            .iter()
            .chain(earlier.iter().rev())
            .map(|row| row.id())
            .find(remains)
    })
    .or_else(|| after.first().map(|row| row.id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(number: u64) -> ItemId {
        ItemId::new(number)
    }

    fn task(number: u64, section: Option<u64>) -> Row {
        Row::Task {
            id: id(number),
            section: section.map(id),
        }
    }

    /// Two loose tasks, section 3 with tasks 4 and 5, section 6 with none, then project 7.
    fn rows() -> Vec<Row> {
        vec![
            task(1, None),
            task(2, None),
            Row::Section(id(3)),
            task(4, Some(3)),
            task(5, Some(3)),
            Row::Section(id(6)),
            Row::Project(id(7)),
        ]
    }

    #[track_caller]
    fn check(from: u64, motion: Focus, to: u64) {
        assert_eq!(
            target(&rows(), Some(id(from)), motion),
            Some(id(to)),
            "{motion:?} from {from}"
        );
    }

    // FR-NAV-01
    #[test]
    fn next_and_previous_move_one_row_and_stop_at_the_ends() {
        check(1, Focus::Next, 2);
        check(2, Focus::Next, 3);
        check(3, Focus::Next, 4);
        check(7, Focus::Next, 7);
        check(4, Focus::Previous, 3);
        check(1, Focus::Previous, 1);
    }

    // FR-NAV-01
    #[test]
    fn first_and_last_go_to_the_ends() {
        check(4, Focus::First, 1);
        check(1, Focus::First, 1);
        check(4, Focus::Last, 7);
        check(7, Focus::Last, 7);
    }

    // FR-NAV-01
    #[test]
    fn parent_goes_from_a_task_to_its_section_heading() {
        check(5, Focus::Parent, 3);
        check(1, Focus::Parent, 1);
        check(3, Focus::Parent, 3);
        check(7, Focus::Parent, 7);
    }

    // FR-NAV-01
    #[test]
    fn next_and_previous_section_go_to_the_nearest_heading_that_way() {
        check(1, Focus::NextSection, 3);
        check(3, Focus::NextSection, 6);
        check(4, Focus::NextSection, 6);
        check(6, Focus::NextSection, 6);
        check(7, Focus::PreviousSection, 6);
        check(6, Focus::PreviousSection, 3);
        check(5, Focus::PreviousSection, 3);
        check(3, Focus::PreviousSection, 3);
        check(2, Focus::PreviousSection, 2);
    }

    #[test]
    fn without_a_focused_row_every_motion_focuses_the_first() {
        for motion in [Focus::Next, Focus::Previous, Focus::Last, Focus::Parent] {
            assert_eq!(target(&rows(), None, motion), Some(id(1)), "{motion:?}");
            assert_eq!(
                target(&rows(), Some(id(99)), motion),
                Some(id(1)),
                "{motion:?} from a row not shown"
            );
        }
    }

    #[test]
    fn without_rows_nothing_is_focused() {
        assert_eq!(target(&[], None, Focus::Next), None);
        assert_eq!(target(&[], Some(id(1)), Focus::Last), None);
    }

    // FR-VIEW-11
    #[test]
    fn a_row_that_stays_keeps_the_focus() {
        let after = [task(2, None), task(1, None)];
        assert_eq!(kept(&rows(), &after, Some(id(1))), Some(id(1)));
    }

    // FR-VIEW-11, FR-ORG-09
    #[test]
    fn a_row_that_leaves_hands_the_focus_to_the_next_that_stays_else_the_one_before() {
        let after = [task(1, None), Row::Section(id(3)), task(5, Some(3))];
        assert_eq!(kept(&rows(), &after, Some(id(4))), Some(id(5)));
        assert_eq!(kept(&rows(), &after, Some(id(7))), Some(id(5)));
        assert_eq!(kept(&rows(), &after, Some(id(2))), Some(id(3)));
    }

    #[test]
    fn a_focus_that_was_no_row_goes_to_the_first() {
        let after = [task(2, None)];
        assert_eq!(kept(&rows(), &after, Some(id(99))), Some(id(2)));
        assert_eq!(kept(&rows(), &after, None), Some(id(2)));
        assert_eq!(kept(&rows(), &[], Some(id(1))), None);
    }
}
