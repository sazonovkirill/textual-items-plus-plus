//! The strip of panes across the screen, as taskpaper-terminal lays it out
//! ([navigation.md §5](../../../specs/reference/taskpaper-terminal/specs/navigation.md#5-panes-fr-ui-02--fr-ui-04-d-13)):
//! how wide each pane is drawn, and which panes show when they do not all fit (FR-PANE-01,
//! FR-PANE-02).

use ratatui::layout::Rect;

/// The fewest columns a pane is drawn in while there is room for them.
pub(super) const MIN_WIDTH: u16 = 30;

/// The columns narrowing or widening a pane takes off or adds.
pub(super) const STEP: u16 = 4;

/// Where the panes are drawn over `area`, left to right, each with its position among the panes,
/// a column apart for the rule between two. `widths` holds the width each pane keeps, if any;
/// `current` is the pane with the keys and `first` the first pane drawn the last time. The panes
/// keeping no width share what the others leave, none narrower than [`MIN_WIDTH`]. When they do
/// not all fit, the strip scrolls sideways by whole panes, only as far as keeps `current` in view,
/// and shows as many as fit; the last one drawn takes the columns left over.
pub(super) fn strip(
    area: Rect,
    widths: &[Option<u16>],
    current: usize,
    first: usize,
) -> Vec<(usize, Rect)> {
    let sizes = sizes(area.width, widths);
    let fits = |from: usize, to: usize| span(&sizes, from, to) <= u32::from(area.width);
    let mut first = first.min(current);
    while first < current && !fits(first, current) {
        first += 1;
    }
    let mut last = current;
    while last + 1 < sizes.len() && fits(first, last + 1) {
        last += 1;
    }
    while first > 0 && fits(first - 1, last) {
        first -= 1;
    }
    let mut x = area.x;
    let mut placed = Vec::with_capacity(last + 1 - first);
    for (index, &size) in sizes.iter().enumerate().take(last + 1).skip(first) {
        let width = if index == last {
            area.right().saturating_sub(x)
        } else {
            size.min(area.right().saturating_sub(x))
        };
        placed.push((index, Rect { x, width, ..area }));
        x = x.saturating_add(width).saturating_add(1).min(area.right());
    }
    placed
}

/// How wide each pane would be in `width` columns: a pane keeping a width keeps it, and the others
/// share what is left after the rules between the panes, none narrower than [`MIN_WIDTH`].
pub(super) fn sizes(width: u16, widths: &[Option<u16>]) -> Vec<u16> {
    let rules = u16::try_from(widths.len().saturating_sub(1)).unwrap_or(u16::MAX);
    let room = width.saturating_sub(rules);
    let kept: u16 = widths
        .iter()
        .flatten()
        .fold(0, |sum, &width| sum.saturating_add(width.max(MIN_WIDTH)));
    let sharing = widths.iter().filter(|width| width.is_none()).count();
    let share = u16::try_from(sharing)
        .ok()
        .filter(|&sharing| sharing > 0)
        .map_or(0, |sharing| room.saturating_sub(kept) / sharing);
    widths
        .iter()
        .map(|width| width.unwrap_or(share).max(MIN_WIDTH))
        .collect()
}

/// The columns the panes `from` to `to` take, both included, with the rules between them.
fn span(sizes: &[u16], from: usize, to: usize) -> u32 {
    let panes = to + 1 - from;
    let rules = u32::try_from(panes - 1).unwrap_or(u32::MAX);
    sizes
        .iter()
        .skip(from)
        .take(panes)
        .map(|&size| u32::from(size))
        .sum::<u32>()
        + rules
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(width: u16) -> Rect {
        Rect::new(2, 1, width, 10)
    }

    /// Each pane drawn as its position, its first column and its width.
    fn drawn(width: u16, widths: &[Option<u16>], current: usize, first: usize) -> Vec<[u16; 3]> {
        strip(area(width), widths, current, first)
            .into_iter()
            .map(|(index, rect)| {
                assert_eq!((rect.y, rect.height), (1, 10));
                [u16::try_from(index).unwrap_or(u16::MAX), rect.x, rect.width]
            })
            .collect()
    }

    // FR-PANE-01
    #[test]
    fn one_pane_takes_the_whole_width() {
        assert_eq!(drawn(80, &[None], 0, 0), [[0, 2, 80]]);
        assert_eq!(drawn(80, &[Some(40)], 0, 0), [[0, 2, 80]]);
        assert_eq!(drawn(20, &[None], 0, 0), [[0, 2, 20]]);
    }

    // FR-PANE-01
    #[test]
    fn panes_share_the_width_a_rule_apart_the_last_taking_what_is_left() {
        assert_eq!(
            drawn(100, &[None, None, None], 1, 0),
            [[0, 2, 32], [1, 35, 32], [2, 68, 34]]
        );
    }

    // FR-PANE-02
    #[test]
    fn a_pane_keeping_a_width_keeps_it_and_the_others_share_the_rest() {
        assert_eq!(
            drawn(100, &[None, Some(40), None], 1, 0),
            [[0, 2, 30], [1, 33, 69]],
            "shares no narrower than the least, so the last does not fit"
        );
        assert_eq!(
            drawn(110, &[None, Some(40), None], 1, 0),
            [[0, 2, 34], [1, 37, 40], [2, 78, 34]]
        );
        assert_eq!(
            drawn(90, &[Some(30), Some(30)], 0, 0),
            [[0, 2, 30], [1, 33, 59]],
            "with no pane to share it, the last takes what is left"
        );
    }

    // FR-PANE-02
    #[test]
    fn panes_that_do_not_fit_scroll_sideways_to_keep_the_one_with_the_keys_in_view() {
        let four = [None; 4];
        assert_eq!(drawn(70, &four, 0, 0), [[0, 2, 30], [1, 33, 39]]);
        assert_eq!(drawn(70, &four, 1, 0), [[0, 2, 30], [1, 33, 39]]);
        assert_eq!(
            drawn(70, &four, 3, 0),
            [[2, 2, 30], [3, 33, 39]],
            "scrolled right as far as it takes"
        );
        assert_eq!(
            drawn(70, &four, 2, 2),
            [[2, 2, 30], [3, 33, 39]],
            "kept where it was"
        );
        assert_eq!(
            drawn(70, &four, 1, 2),
            [[1, 2, 30], [2, 33, 39]],
            "scrolled left as far as it takes"
        );
        assert_eq!(
            drawn(70, &[None, None], 1, 1),
            [[0, 2, 34], [1, 37, 35]],
            "scrolled back once they fit"
        );
        assert_eq!(
            drawn(20, &four, 2, 0),
            [[2, 2, 20]],
            "a screen narrower than a pane shows the one with the keys"
        );
        assert_eq!(drawn(80, &[None, Some(200)], 1, 0), [[1, 2, 80]]);
    }
}
