//! The application state a frontend renders, changed only by [`update`](super::update).

use super::Selection;

/// Everything the app knows while it runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    selection: Selection,
}

impl State {
    /// The items the next command acts on.
    #[must_use]
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_state_selects_nothing() {
        assert_eq!(State::default().selection(), &Selection::default());
    }
}
