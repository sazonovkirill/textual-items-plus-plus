//! Items: what names one node of the document, and what an area, project, section, task or
//! checklist item holds ([functional-requirements.md](../../../specs/functional-requirements.md),
//! FR-ORG, FR-TASK, FR-CHK).

use jiff::civil::Date;

/// Names one node of a document, an item, a checklist item or content kept as read, for as long
/// as the app runs, whatever is renamed or moved; never reused for another node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(u64);

impl ItemId {
    /// The identifier numbered `number`; the model numbers its items.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }
}

/// One node of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// An item the model understands.
    Item(Item),
    /// A line of the file the model does not interpret, kept as read so that saving writes it
    /// back where it was (FR-STORE-09). Its children are kept content too.
    Kept(String),
}

impl Node {
    /// The item this node is, or `None` for kept content.
    #[must_use]
    pub fn item(&self) -> Option<&Item> {
        match self {
            Self::Item(item) => Some(item),
            Self::Kept(_) => None,
        }
    }
}

/// What an item is; each kind sits only where [`ItemKind::fits_in`] allows (FR-ORG-01–04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// The built-in container for tasks not yet organized (FR-ORG-07).
    Inbox,
    /// A top-level area of responsibility.
    Area,
    /// A goal made of tasks, which can hold other projects.
    Project,
    /// A titled group of tasks inside an area or a project.
    Section,
    /// One thing to do.
    Task,
    /// One line of a task's checklist, nested to any depth (FR-CHK-01).
    ChecklistItem,
}

impl ItemKind {
    /// Whether an item of this kind may sit in an item of kind `parent`, or at the top level when
    /// `parent` is `None`. The Inbox sits nowhere: a document has exactly one, of its own.
    #[must_use]
    pub fn fits_in(self, parent: Option<Self>) -> bool {
        use ItemKind::{Area, ChecklistItem, Inbox, Project, Section, Task};
        match self {
            Inbox => false,
            Area => parent.is_none(),
            Project => matches!(parent, None | Some(Area | Project)),
            Section => matches!(parent, Some(Area | Project)),
            Task => matches!(parent, Some(Inbox | Area | Project | Section)),
            ChecklistItem => matches!(parent, Some(Task | ChecklistItem)),
        }
    }
}

/// What an item holds besides its place and its contents (FR-ORG-05, FR-TASK-01, D-27).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// What the item is.
    pub kind: ItemKind,
    /// Its title; a checklist item's text.
    pub title: String,
    /// Markdown shown inline under the title, lines separated by `\n`; empty for none
    /// (FR-ORG-10).
    pub description: String,
    /// Markdown kept in the attachments panel, lines separated by `\n`; empty for none (FR-ATT).
    pub note: String,
    /// Comment lines in the order stored, newest first as the app writes them (FR-ATT-04).
    pub comments: Vec<String>,
    /// The day it becomes relevant.
    pub start: Option<Date>,
    /// The day it must be done.
    pub due: Option<Date>,
    /// Whether, and when, it was completed; a checklist item's done state.
    pub completed: Option<Completion>,
    /// The day it was moved to the Trash (OQ-19).
    pub deleted: Option<Date>,
    /// The user's tags, and any other tag the model does not interpret, in their order.
    pub tags: Vec<Tag>,
}

impl Item {
    /// An item of kind `kind` titled `title`, with nothing else.
    #[must_use]
    pub fn new(kind: ItemKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            description: String::new(),
            note: String::new(),
            comments: Vec::new(),
            start: None,
            due: None,
            completed: None,
            deleted: None,
            tags: Vec::new(),
        }
    }
}

/// How an item was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// Completed on this day (FR-TASK-02).
    On(Date),
    /// Completed with no day recorded, as other TaskPaper apps may write it.
    Undated,
}

/// A tag on an item: a name and an optional value (FR-TAG-01, FR-STORE-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The name, without `@`.
    pub name: String,
    /// The value, if it has one.
    pub value: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ItemKind::{Area, ChecklistItem, Inbox, Project, Section, Task};

    #[test]
    fn identifiers_are_equal_only_with_the_same_number() {
        assert_eq!(ItemId::new(7), ItemId::new(7));
        assert_ne!(ItemId::new(7), ItemId::new(8));
        assert!(ItemId::new(7) < ItemId::new(8));
    }

    // FR-ORG-01, FR-ORG-02, FR-ORG-03, FR-ORG-04, FR-ORG-07, FR-CHK-01
    #[test]
    fn each_kind_fits_only_where_the_hierarchy_allows() {
        let places = [
            None,
            Some(Inbox),
            Some(Area),
            Some(Project),
            Some(Section),
            Some(Task),
            Some(ChecklistItem),
        ];
        let allowed = |kind: ItemKind| -> Vec<Option<ItemKind>> {
            places
                .into_iter()
                .filter(|&place| kind.fits_in(place))
                .collect()
        };
        assert_eq!(allowed(Inbox), []);
        assert_eq!(allowed(Area), [None]);
        assert_eq!(allowed(Project), [None, Some(Area), Some(Project)]);
        assert_eq!(allowed(Section), [Some(Area), Some(Project)]);
        assert_eq!(
            allowed(Task),
            [Some(Inbox), Some(Area), Some(Project), Some(Section)]
        );
        assert_eq!(allowed(ChecklistItem), [Some(Task), Some(ChecklistItem)]);
    }

    #[test]
    fn only_an_item_node_has_an_item() {
        let item = Item::new(Task, "t");
        assert_eq!(Node::Item(item.clone()).item(), Some(&item));
        assert_eq!(Node::Kept("t".to_owned()).item(), None);
    }

    // FR-TASK-01
    #[test]
    fn a_new_item_has_only_its_kind_and_title() {
        let item = Item::new(Task, "Call the bank");
        assert_eq!(item.kind, Task);
        assert_eq!(item.title, "Call the bank");
        assert_eq!(
            (item.description, item.note, item.comments),
            (String::new(), String::new(), Vec::<String>::new())
        );
        assert_eq!((item.start, item.due, item.deleted), (None, None, None));
        assert_eq!(item.completed, None);
        assert_eq!(item.tags, []);
    }
}
