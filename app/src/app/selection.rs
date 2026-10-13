//! The selection: which items the next command acts on. Commands that act on items take the
//! selection, which holds only the focused item until multi-selection arrives (FR-OPS-01).

use crate::domain::ItemId;

/// The items the next command acts on, and the focused one among them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    focused: Option<ItemId>,
}

impl Selection {
    /// The focused item, if any item is focused.
    #[must_use]
    pub fn focused(&self) -> Option<ItemId> {
        self.focused
    }

    /// Focuses `item`, or nothing, and selects only it.
    pub fn focus(&mut self, item: Option<ItemId>) {
        self.focused = item;
    }

    /// The items a command acts on, in the order it should take them.
    pub fn items(&self) -> impl Iterator<Item = ItemId> {
        self.focused.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_selection_holds_nothing() {
        let selection = Selection::default();
        assert_eq!(selection.focused(), None);
        assert_eq!(selection.items().count(), 0);
    }

    #[test]
    fn focusing_an_item_selects_it_alone() {
        let mut selection = Selection::default();
        selection.focus(Some(ItemId::new(1)));
        selection.focus(Some(ItemId::new(2)));
        assert_eq!(selection.focused(), Some(ItemId::new(2)));
        assert_eq!(selection.items().collect::<Vec<_>>(), [ItemId::new(2)]);
    }

    #[test]
    fn focusing_nothing_empties_the_selection() {
        let mut selection = Selection::default();
        selection.focus(Some(ItemId::new(1)));
        selection.focus(None);
        assert_eq!(selection.focused(), None);
        assert_eq!(selection.items().count(), 0);
    }
}
