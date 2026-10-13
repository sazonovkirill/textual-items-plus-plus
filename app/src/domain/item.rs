//! Item identity: what names one area, project, section or task while the app runs.

/// Names one item for as long as the app runs, whatever is renamed or moved; never reused for
/// another item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(u64);

impl ItemId {
    /// The identifier numbered `number`; the model numbers its items.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_equal_only_with_the_same_number() {
        assert_eq!(ItemId::new(7), ItemId::new(7));
        assert_ne!(ItemId::new(7), ItemId::new(8));
        assert!(ItemId::new(7) < ItemId::new(8));
    }
}
