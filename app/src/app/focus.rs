//! Moving the focus through the rows of a container view with the motions of FR-NAV-01, the
//! title of the container shown standing above them as a row of its own (D-68).

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
}

/// The row `motion` focuses when `focused` has the focus among `rows`, below `heading`, the
/// title of the container shown when it takes the focus, or `focused` itself when the motion has
/// nowhere to go. The heading is above the first row and holds every row not under a section.
/// With nothing focused, every motion focuses the first row, else the heading.
pub(super) fn target(
    rows: &[Row],
    heading: Option<ItemId>,
    focused: Option<ItemId>,
    motion: Focus,
) -> Option<ItemId> {
    let Some(at) = focused.and_then(|id| rows.iter().position(|row| row.id() == id)) else {
        let first = rows.first().copied().map(Row::id);
        return match motion {
            _ if heading.is_none() || focused != heading => first.or(heading),
            Focus::Next => first.or(focused),
            Focus::Last => rows.last().copied().map(Row::id).or(focused),
            Focus::Previous | Focus::First => focused,
        };
    };
    let row = match motion {
        Focus::Next => rows.get(at + 1),
        Focus::Previous => at.checked_sub(1).and_then(|before| rows.get(before)),
        Focus::First => rows.first().filter(|_| heading.is_none()),
        Focus::Last => rows.last(),
    };
    match (row, motion) {
        (None, Focus::Previous | Focus::First) => heading.or(focused),
        (row, _) => row.copied().map(Row::id).or(focused),
    }
}

/// The row to focus once the rows `before` have become `after`, below `heading`, the title of
/// the container shown when it takes the focus: `focused` while it is still a row or the heading,
/// else the nearest row after it that remains, else the nearest before it, so that hiding the
/// focused row moves the focus on as a list does when an item leaves it. With nothing of that to
/// go to, the first row, else the heading.
pub(super) fn kept(
    before: &[Row],
    after: &[Row],
    heading: Option<ItemId>,
    focused: Option<ItemId>,
) -> Option<ItemId> {
    let remains = |id: &ItemId| Some(*id) == heading || after.iter().any(|row| row.id() == *id);
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
    .or(heading)
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
            target(&rows(), None, Some(id(from)), motion),
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

    #[test]
    fn without_a_focused_row_every_motion_focuses_the_first() {
        for motion in [Focus::Next, Focus::Previous, Focus::Last] {
            assert_eq!(
                target(&rows(), None, None, motion),
                Some(id(1)),
                "{motion:?}"
            );
            assert_eq!(
                target(&rows(), None, Some(id(99)), motion),
                Some(id(1)),
                "{motion:?} from a row not shown"
            );
        }
    }

    #[test]
    fn without_rows_nothing_is_focused() {
        assert_eq!(target(&[], None, None, Focus::Next), None);
        assert_eq!(target(&[], None, Some(id(1)), Focus::Last), None);
    }

    // FR-VIEW-11
    #[test]
    fn a_row_that_stays_keeps_the_focus() {
        let after = [task(2, None), task(1, None)];
        assert_eq!(kept(&rows(), &after, None, Some(id(1))), Some(id(1)));
    }

    // FR-VIEW-11, FR-ORG-09
    #[test]
    fn a_row_that_leaves_hands_the_focus_to_the_next_that_stays_else_the_one_before() {
        let after = [task(1, None), Row::Section(id(3)), task(5, Some(3))];
        assert_eq!(kept(&rows(), &after, None, Some(id(4))), Some(id(5)));
        assert_eq!(kept(&rows(), &after, None, Some(id(7))), Some(id(5)));
        assert_eq!(kept(&rows(), &after, None, Some(id(2))), Some(id(3)));
    }

    #[test]
    fn a_focus_that_was_no_row_goes_to_the_first() {
        let after = [task(2, None)];
        assert_eq!(kept(&rows(), &after, None, Some(id(99))), Some(id(2)));
        assert_eq!(kept(&rows(), &after, None, None), Some(id(2)));
        assert_eq!(kept(&rows(), &[], None, Some(id(1))), None);
    }

    /// The heading of the container shown, numbered 0, above [`rows`].
    const HEADING: Option<ItemId> = Some(ItemId::new(0));

    // FR-NAV-01, D-68
    #[test]
    fn the_title_of_the_container_shown_is_a_row_above_the_first() {
        let go = |from: Option<u64>, motion| target(&rows(), HEADING, from.map(id), motion);
        assert_eq!(go(Some(1), Focus::Previous), HEADING);
        assert_eq!(go(Some(4), Focus::First), HEADING);
        assert_eq!(go(Some(0), Focus::Next), Some(id(1)));
        assert_eq!(go(Some(0), Focus::Last), Some(id(7)));
        for motion in [Focus::Previous, Focus::First] {
            assert_eq!(go(Some(0), motion), HEADING, "{motion:?}");
        }
        assert_eq!(go(None, Focus::Next), Some(id(1)));
        for motion in [Focus::Next, Focus::Last] {
            assert_eq!(target(&[], HEADING, HEADING, motion), HEADING, "{motion:?}");
        }
        assert_eq!(target(&[], HEADING, None, Focus::Next), HEADING);
    }

    // FR-VIEW-11, D-68
    #[test]
    fn the_heading_keeps_the_focus_and_takes_it_when_no_row_is_left() {
        assert_eq!(kept(&rows(), &[], HEADING, HEADING), HEADING);
        assert_eq!(kept(&rows(), &[], HEADING, Some(id(1))), HEADING);
        assert_eq!(kept(&rows(), &rows(), HEADING, Some(id(4))), Some(id(4)));
    }
}
