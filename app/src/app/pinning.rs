//! Pinning projects, which the sidebar and the Navigator then also list among the pinned
//! projects (FR-VIEW-12).

use super::State;
use crate::domain::{Document, Item, ItemId, ItemKind, Op, Transaction};

impl State {
    /// Pins the selected projects, or the project shown while its heading has the focus, or
    /// unpins them when all are pinned, as one step to undo; the status line says when none of
    /// them is a project.
    pub(super) fn toggle_pinned(&mut self) {
        let items = self.described();
        self.toggle_pinned_for(&items);
    }

    /// Pins the projects among `items`, or unpins them when all are pinned, as one step to undo;
    /// the status line says when none of them is a project.
    pub(super) fn toggle_pinned_for(&mut self, items: &[ItemId]) {
        match toggle(&self.document, items) {
            Some(transaction) => self.change(transaction),
            None => self.notify(super::Notice::NotAProject),
        }
    }
}

/// The transaction that pins the projects among `items`, or unpins them when all are pinned;
/// `None` when none of them is a project, since only a project is pinned.
pub(super) fn toggle(document: &Document, items: &[ItemId]) -> Option<Transaction> {
    let projects: Vec<(ItemId, &Item)> = items
        .iter()
        .filter_map(|&id| Some((id, document.node(id)?.item()?)))
        .filter(|(_, item)| item.kind == ItemKind::Project)
        .collect();
    if projects.is_empty() {
        return None;
    }
    let pin = !projects.iter().all(|(_, item)| item.pinned);
    let ops = projects
        .into_iter()
        .filter(|(_, item)| item.pinned != pin)
        .map(|(id, before)| Op::Set {
            id,
            before: before.clone(),
            after: Item {
                pinned: pin,
                ..before.clone()
            },
        })
        .collect();
    Some(Transaction::new(ops))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Node;
    use ItemKind::{Area, Project, Section, Task};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn pinned(document: &Document, id: ItemId) -> bool {
        document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| item.pinned)
    }

    // FR-VIEW-12
    #[test]
    fn pins_a_project_unpins_it_and_refuses_anything_else() -> TestResult {
        let mut document = Document::new();
        let work = document.push(None, Node::Item(Item::new(Area, "Work")))?;
        let site = document.push(Some(work), Node::Item(Item::new(Project, "Site")))?;
        let section = document.push(Some(site), Node::Item(Item::new(Section, "S")))?;
        let task = document.push(Some(section), Node::Item(Item::new(Task, "t")))?;
        let docs = document.push(Some(work), Node::Item(Item::new(Project, "Docs")))?;
        assert_eq!(toggle(&document, &[]), None);
        assert_eq!(
            toggle(&document, &[document.inbox(), work, section, task]),
            None
        );
        let pin = toggle(&document, &[task, site]).ok_or("a project is pinned")?;
        document.apply(&pin)?;
        assert!(pinned(&document, site));
        let both = toggle(&document, &[site, docs]).ok_or("projects are pinned")?;
        document.apply(&both)?;
        assert!(
            pinned(&document, site) && pinned(&document, docs),
            "not all were"
        );
        let unpin = toggle(&document, &[site, docs]).ok_or("projects are unpinned")?;
        document.apply(&unpin)?;
        assert!(!pinned(&document, site) && !pinned(&document, docs));
        Ok(())
    }

    // FR-VIEW-12
    #[test]
    fn a_pinned_project_converted_to_a_section_is_no_longer_pinned() -> TestResult {
        let mut document = Document::new();
        let site = document.push(None, Node::Item(Item::new(Project, "Site")))?;
        let blog = document.push(Some(site), Node::Item(Item::new(Project, "Blog")))?;
        document.apply(&toggle(&document, &[blog]).ok_or("a project is pinned")?)?;
        document.apply(&document.convert(blog, Section)?)?;
        assert!(!pinned(&document, blog));
        Ok(())
    }
}
