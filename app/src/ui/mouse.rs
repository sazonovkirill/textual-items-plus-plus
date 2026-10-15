//! What the mouse points at (FR-KEY-05, D-138): which mouse events the app takes, and what the
//! last frame drew under the pointer, so that a click or a turn of the wheel becomes the messages
//! the keys send there.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

use crate::{app::Direction, domain::ItemId};

/// How many rows or lines a turn of the wheel moves, as terminals and editors commonly scroll
/// (D-74).
pub(super) const WHEEL: usize = 3;

/// A mouse event the app takes, with where it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Pointer {
    /// The left button went down.
    Click(Position),
    /// The wheel turned a notch this way.
    Wheel(Direction, Position),
}

impl Pointer {
    /// The pointer `mouse` reports, if the app takes it: a press of the left button or a turn of
    /// the wheel up or down. Moving, dragging, releasing, the other buttons and the sideways wheel
    /// do nothing.
    pub(super) fn of(mouse: MouseEvent) -> Option<Self> {
        let at = Position::new(mouse.column, mouse.row);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => Some(Self::Click(at)),
            MouseEventKind::ScrollUp => Some(Self::Wheel(Direction::Up, at)),
            MouseEventKind::ScrollDown => Some(Self::Wheel(Direction::Down, at)),
            _ => None,
        }
    }

    /// Where it happened.
    pub(super) fn at(self) -> Position {
        match self {
            Self::Click(at) | Self::Wheel(_, at) => at,
        }
    }
}

/// Something drawn that the mouse can point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    /// The pane at this position among the panes, where no row is drawn.
    Pane(usize),
    /// The row of item `id` in the pane at position `pane` among the panes.
    Row {
        /// The pane's position among the panes.
        pane: usize,
        /// The item the row shows.
        id: ItemId,
    },
    /// The checkbox of the task `id` in the pane at position `pane` among the panes.
    Checkbox {
        /// The pane's position among the panes.
        pane: usize,
        /// The task.
        id: ItemId,
    },
    /// The attachments panel of the pane at this position among the panes.
    Panel(usize),
    /// The sidebar's entry at this index, or the sidebar where no entry is drawn.
    Sidebar(Option<usize>),
    /// The shortcuts panel.
    Shortcuts,
    /// The open picker's entry at this index among the matches, or its window elsewhere.
    Picker(Option<usize>),
}

impl Target {
    /// The position among the panes of the pane holding it, for a row, a checkbox or a pane.
    pub(super) fn pane(self) -> Option<usize> {
        match self {
            Self::Pane(pane) | Self::Row { pane, .. } | Self::Checkbox { pane, .. } => Some(pane),
            Self::Panel(_) | Self::Sidebar(_) | Self::Shortcuts | Self::Picker(_) => None,
        }
    }
}

/// Where the last frame drew each target, in the order drawn.
#[derive(Debug, Default)]
pub(super) struct Targets(Vec<(Rect, Target)>);

impl Targets {
    /// Forgets every target, as a new frame starts.
    pub(super) fn clear(&mut self) {
        self.0.clear();
    }

    /// Notes that `target` was drawn over `area`, over whatever was drawn there before.
    pub(super) fn add(&mut self, area: Rect, target: Target) {
        self.0.push((area, target));
    }

    /// What was drawn last at `at`, if anything.
    pub(super) fn at(&self, at: Position) -> Option<Target> {
        self.0
            .iter()
            .rev()
            .find(|(area, _)| area.contains(at))
            .map(|&(_, target)| target)
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;

    fn event(kind: MouseEventKind) -> MouseEvent {
        MouseEvent {
            kind,
            column: 7,
            row: 3,
            modifiers: KeyModifiers::NONE,
        }
    }

    // FR-KEY-05
    #[test]
    fn the_left_button_clicks_and_the_wheel_turns_up_or_down() {
        let at = Position::new(7, 3);
        for (kind, expected) in [
            (MouseEventKind::Down(MouseButton::Left), Pointer::Click(at)),
            (MouseEventKind::ScrollUp, Pointer::Wheel(Direction::Up, at)),
            (
                MouseEventKind::ScrollDown,
                Pointer::Wheel(Direction::Down, at),
            ),
        ] {
            let pointer = Pointer::of(event(kind));
            assert_eq!(pointer, Some(expected), "{kind:?}");
            assert_eq!(pointer.map(Pointer::at), Some(at));
        }
    }

    // FR-KEY-05
    #[test]
    fn moving_dragging_releasing_other_buttons_and_the_sideways_wheel_do_nothing() {
        for kind in [
            MouseEventKind::Moved,
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Down(MouseButton::Middle),
            MouseEventKind::ScrollLeft,
            MouseEventKind::ScrollRight,
        ] {
            assert_eq!(Pointer::of(event(kind)), None, "{kind:?}");
        }
    }

    #[test]
    fn rows_checkboxes_and_panes_are_in_a_pane() {
        let id = crate::domain::Document::new().inbox();
        for (target, pane) in [
            (Target::Pane(2), Some(2)),
            (Target::Row { pane: 1, id }, Some(1)),
            (Target::Checkbox { pane: 0, id }, Some(0)),
            (Target::Panel(1), None),
            (Target::Sidebar(Some(0)), None),
            (Target::Shortcuts, None),
            (Target::Picker(None), None),
        ] {
            assert_eq!(target.pane(), pane, "{target:?}");
        }
    }

    #[test]
    fn the_target_drawn_last_at_a_position_is_the_one_pointed_at() {
        let mut targets = Targets::default();
        assert_eq!(targets.at(Position::new(0, 0)), None, "nothing drawn");
        targets.add(Rect::new(0, 0, 10, 5), Target::Pane(0));
        targets.add(Rect::new(0, 2, 10, 1), Target::Sidebar(Some(1)));
        assert_eq!(
            targets.at(Position::new(3, 2)),
            Some(Target::Sidebar(Some(1)))
        );
        assert_eq!(targets.at(Position::new(3, 1)), Some(Target::Pane(0)));
        assert_eq!(targets.at(Position::new(9, 4)), Some(Target::Pane(0)));
        assert_eq!(
            targets.at(Position::new(10, 4)),
            None,
            "past the right edge"
        );
        targets.clear();
        assert_eq!(targets.at(Position::new(3, 2)), None, "a new frame forgets");
    }
}
