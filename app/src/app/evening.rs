//! Putting Today's items in its Evening part, as Things 3's This Evening, out of the way until
//! later in the day (FR-VIEW-13).

use jiff::civil::Date;

use super::{
    State,
    view::{Shown, View},
};
use crate::domain::{Document, Item, ItemId, Op, Transaction};

impl State {
    /// While the pane shows Today, puts the selected items in the Evening, or takes them out of
    /// it when all are in it, as one step to undo.
    pub(super) fn toggle_evening(&mut self) {
        if self.shown() == Shown::View(View::Today) {
            let items = self.described();
            self.change(toggle(&self.document, &items, self.today));
        }
    }
}

/// The transaction that takes `items` out of the Evening when every one of them is in it on
/// `today`, and otherwise puts in it, for `today`, those that are not.
fn toggle(document: &Document, items: &[ItemId], today: Date) -> Transaction {
    let items: Vec<(ItemId, &Item)> = items
        .iter()
        .filter_map(|&id| Some((id, document.node(id)?.item()?)))
        .collect();
    let putting = items.iter().any(|(_, item)| !item.in_evening(today));
    let ops = items
        .into_iter()
        .filter(|(_, item)| item.in_evening(today) != putting)
        .map(|(id, before)| Op::Set {
            id,
            before: before.clone(),
            after: Item {
                evening: putting.then_some(today),
                ..before.clone()
            },
        })
        .collect();
    Transaction::new(ops)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{ItemKind, Node, PlacementError};

    const TODAY: Date = date(2026, 10, 14);

    fn evening(document: &Document, id: ItemId) -> Option<Date> {
        document.node(id).and_then(Node::item)?.evening
    }

    // FR-VIEW-13
    #[test]
    fn puts_in_the_evening_what_is_not_and_takes_out_once_all_are() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let inbox = Some(document.inbox());
        let mut add = |title| document.push(inbox, Node::Item(Item::new(ItemKind::Task, title)));
        let (a, b) = (add("a")?, add("b")?);
        document.apply(&toggle(&document, &[a], TODAY))?;
        assert_eq!(evening(&document, a), Some(TODAY));
        let both = toggle(&document, &[a, b], TODAY);
        assert_eq!(both.ops().len(), 1, "only the one not in it changes");
        document.apply(&both)?;
        document.apply(&toggle(&document, &[b, a], TODAY))?;
        assert_eq!((evening(&document, a), evening(&document, b)), (None, None));
        Ok(())
    }

    // FR-VIEW-13
    #[test]
    fn an_earlier_evening_counts_as_none_and_is_moved_to_today() -> Result<(), PlacementError> {
        let mut document = Document::new();
        let mut item = Item::new(ItemKind::Task, "a");
        item.evening = Some(date(2026, 10, 13));
        let a = document.push(Some(document.inbox()), Node::Item(item))?;
        document.apply(&toggle(&document, &[a, ItemId::new(99)], TODAY))?;
        assert_eq!(evening(&document, a), Some(TODAY));
        Ok(())
    }
}
