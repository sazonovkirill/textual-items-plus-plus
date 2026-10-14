//! The places a pane has shown, to go back and forward through like a browser's history
//! (FR-NAV-04, D-10). Hides how going back and forward passes over places that cannot be shown.

use crate::domain::ItemId;

/// A container a pane showed, with the item focused there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Visit {
    /// The container shown.
    pub(super) container: ItemId,
    /// The item focused in it.
    pub(super) focused: Option<ItemId>,
}

/// The visits to go back to, latest last, and those to go forward to, next last.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Visits {
    back: Vec<Visit>,
    forward: Vec<Visit>,
}

impl Visits {
    /// Records leaving `from` for another place: Back returns to it, and nothing is ahead any
    /// more.
    pub(super) fn leave(&mut self, from: Visit) {
        self.back.push(from);
        self.forward.clear();
    }

    /// Goes back from `current` to the latest visit of another container that `opens`, and
    /// returns it, leaving `current` and the visits passed over to go forward to; `None`, with
    /// nothing changed, when there is none.
    pub(super) fn back(&mut self, current: Visit, opens: impl Fn(ItemId) -> bool) -> Option<Visit> {
        step(&mut self.back, &mut self.forward, current, opens)
    }

    /// Goes forward from `current` as [`back`](Self::back) goes back.
    pub(super) fn forward(
        &mut self,
        current: Visit,
        opens: impl Fn(ItemId) -> bool,
    ) -> Option<Visit> {
        step(&mut self.forward, &mut self.back, current, opens)
    }

    /// Moves every visit to where `container` finds its container now, with its focus where
    /// `focused` finds it, dropping the visits whose container is not found, as after a reload.
    pub(super) fn rematch(
        &mut self,
        container: impl Fn(ItemId) -> Option<ItemId>,
        focused: impl Fn(ItemId) -> Option<ItemId>,
    ) {
        let rematch = |visit: Visit| {
            Some(Visit {
                container: container(visit.container)?,
                focused: visit.focused.and_then(&focused),
            })
        };
        for visits in [&mut self.back, &mut self.forward] {
            *visits = visits.iter().copied().filter_map(rematch).collect();
        }
    }
}

/// Takes the latest visit of `from` to another container than `current`'s that `opens`, moving
/// `current` and the visits after it onto `to` so that the way back passes them in order.
fn step(
    from: &mut Vec<Visit>,
    to: &mut Vec<Visit>,
    current: Visit,
    opens: impl Fn(ItemId) -> bool,
) -> Option<Visit> {
    let at = from
        .iter()
        .rposition(|visit| visit.container != current.container && opens(visit.container))?;
    to.push(current);
    to.extend(from.drain(at + 1..).rev());
    from.pop()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visit(container: u64) -> Visit {
        Visit {
            container: ItemId::new(container),
            focused: Some(ItemId::new(container + 100)),
        }
    }

    fn any(_: ItemId) -> bool {
        true
    }

    // FR-NAV-04
    #[test]
    fn back_and_forward_walk_the_visits_in_order() {
        let mut visits = Visits::default();
        visits.leave(visit(1));
        visits.leave(visit(2));
        assert_eq!(visits.back(visit(3), any), Some(visit(2)));
        assert_eq!(visits.back(visit(2), any), Some(visit(1)));
        assert_eq!(visits.back(visit(1), any), None);
        assert_eq!(visits.forward(visit(1), any), Some(visit(2)));
        assert_eq!(visits.forward(visit(2), any), Some(visit(3)));
        assert_eq!(visits.forward(visit(3), any), None);
    }

    // FR-NAV-04
    #[test]
    fn leaving_for_a_new_place_drops_what_was_ahead() {
        let mut visits = Visits::default();
        visits.leave(visit(1));
        assert_eq!(visits.back(visit(2), any), Some(visit(1)));
        visits.leave(visit(1));
        assert_eq!(visits.forward(visit(3), any), None);
        assert_eq!(visits.back(visit(3), any), Some(visit(1)));
    }

    // FR-NAV-04
    #[test]
    fn places_that_cannot_open_and_the_place_shown_are_passed_over_and_kept() {
        let mut visits = Visits::default();
        for container in [1, 2, 3] {
            visits.leave(visit(container));
        }
        let gone = ItemId::new(3);
        let opens = |id: ItemId| id != gone;
        // Back from 2 passes over 3, which cannot open, and 2, which is shown.
        assert_eq!(visits.back(visit(2), opens), Some(visit(1)));
        assert_eq!(visits.forward(visit(1), opens), Some(visit(2)));
        assert_eq!(visits.forward(visit(2), opens), None);
        // Once 3 opens again, it is there to go forward to.
        assert_eq!(visits.forward(visit(2), any), Some(visit(3)));
    }

    // F-FILE-06
    #[test]
    fn rematching_moves_each_visit_and_drops_those_not_found() {
        let mut visits = Visits::default();
        for container in [1, 2, 3] {
            visits.leave(visit(container));
        }
        assert_eq!(visits.back(visit(4), any), Some(visit(3)));
        let renumbered =
            [(1, 11), (3, 13), (4, 14)].map(|(from, to)| (ItemId::new(from), ItemId::new(to)));
        visits.rematch(
            |id| {
                renumbered
                    .iter()
                    .find(|&&(from, _)| from == id)
                    .map(|&(_, to)| to)
            },
            |id| (id != ItemId::new(101)).then_some(id),
        );
        let moved = |container, focused: Option<u64>| Visit {
            container: ItemId::new(container),
            focused: focused.map(ItemId::new),
        };
        assert_eq!(
            visits.forward(moved(13, Some(103)), any),
            Some(moved(14, Some(104)))
        );
        assert_eq!(
            visits.back(moved(14, None), any),
            Some(moved(13, Some(103)))
        );
        assert_eq!(visits.back(moved(13, None), any), Some(moved(11, None)));
        assert_eq!(visits.back(moved(11, None), any), None);
    }

    #[test]
    fn with_nowhere_to_go_nothing_changes() {
        let mut visits = Visits::default();
        visits.leave(visit(1));
        let before = visits.clone();
        assert_eq!(visits.back(visit(2), |_| false), None);
        assert_eq!(visits.back(visit(1), any), None);
        assert_eq!(visits, before);
    }
}
