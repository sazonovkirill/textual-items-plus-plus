//! Flagging projects and tasks as important, which the Flagged view then lists (FR-ORG-12).

use super::State;
use crate::domain::{Document, Item, ItemId, Op, Transaction};

impl State {
    /// Flags the selected projects and tasks, or the project shown while its heading has the
    /// focus, or unflags them when all are flagged, as one step to undo; the status line says when
    /// none of them is a project or a task.
    pub(super) fn toggle_flagged(&mut self) {
        let items = self.described();
        self.toggle_flagged_for(&items);
    }

    /// Flags the projects and tasks among `items`, or unflags them when all are flagged, as one
    /// step to undo; the status line says when none of them is a project or a task.
    pub(super) fn toggle_flagged_for(&mut self, items: &[ItemId]) {
        match toggle(&self.document, items) {
            Some(transaction) => self.change(transaction),
            None => self.notify(super::Notice::NotFlaggable),
        }
    }
}

/// The transaction that flags the projects and tasks among `items`, or unflags them when all are
/// flagged; `None` when none of them is one, since only those are flagged.
fn toggle(document: &Document, items: &[ItemId]) -> Option<Transaction> {
    let flaggable: Vec<(ItemId, &Item)> = items
        .iter()
        .filter_map(|&id| Some((id, document.node(id)?.item()?)))
        .filter(|(_, item)| item.kind.takes_flag())
        .collect();
    if flaggable.is_empty() {
        return None;
    }
    let flag = !flaggable.iter().all(|(_, item)| item.flagged);
    let ops = flaggable
        .into_iter()
        .filter(|(_, item)| item.flagged != flag)
        .map(|(id, before)| Op::Set {
            id,
            before: before.clone(),
            after: Item {
                flagged: flag,
                ..before.clone()
            },
        })
        .collect();
    Some(Transaction::new(ops))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ItemKind, Node};
    use ItemKind::{Area, Project, Section, Task};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn flagged(document: &Document, id: ItemId) -> bool {
        document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| item.flagged)
    }

    // FR-ORG-12
    #[test]
    fn flags_projects_and_tasks_unflags_them_when_all_are_and_refuses_the_rest() -> TestResult {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(Area, "Work")))?;
        let site = document.push(Some(work), Node::Item(Item::new(Project, "Site")))?;
        let section = document.push(Some(site), Node::Item(Item::new(Section, "S")))?;
        let task = document.push(Some(section), Node::Item(Item::new(Task, "t")))?;
        assert_eq!(toggle(&document, &[]), None);
        assert_eq!(toggle(&document, &[document.inbox(), work, section]), None);
        document.apply(&toggle(&document, &[section, task]).ok_or("a task is flagged")?)?;
        assert!(flagged(&document, task) && !flagged(&document, section));
        let both = toggle(&document, &[task, site]).ok_or("flagged")?;
        assert_eq!(both.ops().len(), 1, "only the project changes");
        document.apply(&both)?;
        assert!(flagged(&document, site));
        document.apply(&toggle(&document, &[site, task]).ok_or("unflagged")?)?;
        assert!(!flagged(&document, site) && !flagged(&document, task));
        Ok(())
    }

    // FR-ORG-12
    #[test]
    fn a_flagged_task_converted_to_a_section_is_no_longer_flagged() -> TestResult {
        let mut document = Document::new();
        let site = document.push(None, Node::Item(Item::new(Project, "Site")))?;
        let task = document.push(Some(site), Node::Item(Item::new(Task, "t")))?;
        document.apply(&toggle(&document, &[task]).ok_or("a task is flagged")?)?;
        document.apply(&document.convert(task, Section)?)?;
        assert!(!flagged(&document, task));
        Ok(())
    }
}
