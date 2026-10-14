//! Items: what names one node of the document, and what an area, project, section, task or
//! checklist item holds ([functional-requirements.md](../../../specs/functional-requirements.md),
//! FR-ORG, FR-TASK, FR-CHK).

use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::dates::DateKind;

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

    /// The number of this identifier.
    #[must_use]
    pub(crate) const fn number(self) -> u64 {
        self.0
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// The built-in container for tasks not yet organized (FR-ORG-07).
    Inbox,
    /// A top-level area of responsibility, holding projects, sections and tasks, with no tags or
    /// dates (D-75).
    Area,
    /// A goal made of tasks, in an area or at the top level; a file may nest one in another,
    /// which the app keeps but never makes (D-57, D-75).
    Project,
    /// A titled group of tasks inside an area or a project.
    Section,
    /// One thing to do.
    Task,
    /// One line of a task's checklist, nested to any depth (FR-CHK-01).
    ChecklistItem,
}

impl ItemKind {
    /// Whether an item of this kind goes in an item of kind `parent`, or at the top level for
    /// `None`, when the app adds, moves, converts or pastes it: where it [fits](Self::fits_in),
    /// except that v1 nests no project in another, which a file may still hold and the app keeps
    /// (D-57).
    #[must_use]
    pub fn goes_in(self, parent: Option<Self>) -> bool {
        self.fits_in(parent) && !(self == Self::Project && parent == Some(Self::Project))
    }

    /// Whether an item of this kind has tags and start and due dates: every kind but an area;
    /// tags or dates a file gives an area are kept, unseen (D-75).
    #[must_use]
    pub fn takes_tags_and_dates(self) -> bool {
        self != Self::Area
    }

    /// Whether an item of this kind can be marked Later: a project or a task, not an area or a
    /// section (D-89).
    #[must_use]
    pub fn goes_later(self) -> bool {
        matches!(self, Self::Project | Self::Task)
    }

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
    /// Whether, and when, it was completed; a checklist item's done state. What the app goes by
    /// is its [`completion`](Item::completion).
    pub completed: Option<Completion>,
    /// The day it was moved to the Trash (OQ-19).
    pub deleted: Option<Date>,
    /// Whether it is marked Later, as Things 3's Someday. What the app goes by is
    /// [`is_later`](Item::is_later).
    pub later: bool,
    /// Whether it is a project the sidebar and the Navigator also list among the pinned ones
    /// (FR-VIEW-12).
    pub pinned: bool,
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
            later: false,
            pinned: false,
            tags: Vec::new(),
        }
    }

    /// Its `kind` date, if it has one.
    #[must_use]
    pub fn date(&self, kind: DateKind) -> Option<Date> {
        match kind {
            DateKind::Start => self.start,
            DateKind::Due => self.due,
        }
    }

    /// The tags the app shows and finds it by: none on an area, which has no tags, though a file
    /// may give it some (D-75).
    #[must_use]
    pub fn shown_tags(&self) -> &[Tag] {
        if self.kind.takes_tags_and_dates() {
            &self.tags
        } else {
            &[]
        }
    }

    /// Whether, and when, the app takes it as completed: never for an area, which is not
    /// completed, though a file may mark it done (D-81).
    #[must_use]
    pub fn completion(&self) -> Option<Completion> {
        self.completed.filter(|_| self.kind != ItemKind::Area)
    }

    /// Whether the app takes it as Later: only a project or a task is, though a file may mark
    /// another kind so (D-89).
    #[must_use]
    pub fn is_later(&self) -> bool {
        self.later && self.kind.goes_later()
    }

    /// Whether it is overdue on `today`: due before it and not completed (FR-DATE-04).
    #[must_use]
    pub fn overdue(&self, today: Date) -> bool {
        self.completion().is_none() && self.due.is_some_and(|due| due < today)
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

/// The tag names the app writes itself, and those TaskPaper reads as plain text, which no tag of
/// the user's takes (FR-STORE-10, OQ-11).
const RESERVED: &[&str] = &[
    "start", "due", "done", "deleted", "later", "hidden", "area", "section", "pinned", "id",
    "text", "type",
];

impl Tag {
    /// The tag named `name` without a value, as the user's tags are.
    #[must_use]
    pub fn plain(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: None,
        }
    }

    /// Whether it is one of the user's tags, which the user adds, removes and filters by by its
    /// name, whatever its value: one whose name the app does not use itself (FR-TAG-01, OQ-11).
    #[must_use]
    pub fn is_users(&self) -> bool {
        !RESERVED.contains(&self.name.as_str())
    }
}

/// The name of the user's tag that `typed` names: trimmed, with each run of spaces inside it
/// written `-` (OQ-11). `None` when nothing is left, when a character is one a TaskPaper tag name
/// cannot hold, or when the app uses the name itself.
#[must_use]
pub fn tag_name(typed: &str) -> Option<String> {
    let name = typed.split_whitespace().collect::<Vec<_>>().join("-");
    let tag = Tag::plain(name);
    (!tag.name.is_empty() && tag.name.chars().all(is_tag_name_char) && tag.is_users())
        .then_some(tag.name)
}

/// Whether a TaskPaper tag name can hold `c`: the XML name characters without `:`, as TaskPaper's
/// `TagWord` (BMP only, like its regex).
#[must_use]
pub fn is_tag_name_char(c: char) -> bool {
    matches!(c,
        'A'..='Z' | '_' | 'a'..='z' | '-' | '.' | '0'..='9' | '\u{B7}'
        | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}'
        | '\u{300}'..='\u{36F}' | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}' | '\u{203F}'..='\u{2040}' | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}')
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

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

    // FR-ORG-01, FR-ORG-02, D-57
    #[test]
    fn the_app_puts_an_item_where_it_fits_but_no_project_in_a_project() {
        let places = [None, Some(Inbox), Some(Area), Some(Project), Some(Section)];
        for kind in [Area, Project, Section, Task] {
            for place in places {
                let nests = kind == Project && place == Some(Project);
                assert_eq!(
                    kind.goes_in(place),
                    kind.fits_in(place) && !nests,
                    "{kind:?} in {place:?}"
                );
            }
        }
        assert!(Project.fits_in(Some(Project)), "a file may nest one");
    }

    // FR-ORG-01, FR-TAG-01, FR-DATE-01
    #[test]
    fn every_kind_but_an_area_takes_tags_and_dates() {
        let kinds = [Inbox, Area, Project, Section, Task, ChecklistItem];
        let taking: Vec<ItemKind> = kinds
            .into_iter()
            .filter(|kind| kind.takes_tags_and_dates())
            .collect();
        assert_eq!(taking, [Inbox, Project, Section, Task, ChecklistItem]);
    }

    // FR-ORG-09
    #[test]
    fn an_area_a_file_marks_done_is_not_completed() {
        let mut area = Item::new(Area, "Home");
        area.completed = Some(Completion::Undated);
        assert_eq!(area.completion(), None);
        let mut project = Item::new(Project, "Garden");
        project.completed = Some(Completion::Undated);
        assert_eq!(project.completion(), Some(Completion::Undated));
    }

    #[test]
    fn an_area_shows_no_tags_a_file_gives_it() {
        let tag = Tag {
            name: "home".to_owned(),
            value: None,
        };
        let mut area = Item::new(Area, "Home");
        area.tags.push(tag.clone());
        assert_eq!(area.shown_tags(), []);
        let mut project = Item::new(Project, "Garden");
        project.tags.push(tag.clone());
        assert_eq!(project.shown_tags(), [tag]);
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

    // FR-DATE-01
    #[test]
    fn an_item_has_a_start_and_a_due_date() {
        let mut item = Item::new(Project, "p");
        item.start = Some(date(2026, 10, 1));
        item.due = Some(date(2026, 10, 9));
        assert_eq!(item.date(DateKind::Start), Some(date(2026, 10, 1)));
        assert_eq!(item.date(DateKind::Due), Some(date(2026, 10, 9)));
    }

    // FR-DATE-04
    #[test]
    fn an_item_due_before_today_is_overdue_until_completed() {
        let today = date(2026, 10, 14);
        let mut item = Item::new(Task, "t");
        assert!(!item.overdue(today), "no due date");
        item.due = Some(today);
        assert!(!item.overdue(today), "due today");
        item.due = Some(date(2026, 10, 13));
        assert!(item.overdue(today));
        item.completed = Some(Completion::Undated);
        assert!(!item.overdue(today), "completed");
    }

    // FR-STORE-10
    #[test]
    fn name_characters_match_taskpaper() {
        for c in [
            'A', 'Z', 'a', 'z', '_', '-', '.', '0', '9', '\u{B7}', '\u{C0}', '\u{D6}', '\u{D8}',
            '\u{F6}', '\u{F8}', '\u{2FF}', '\u{300}', '\u{36F}', '\u{370}', '\u{37D}', '\u{37F}',
            '\u{1FFF}', '\u{200C}', '\u{200D}', '\u{203F}', '\u{2040}', '\u{2070}', '\u{218F}',
            '\u{2C00}', '\u{2FEF}', '\u{3001}', '\u{D7FF}', '\u{F900}', '\u{FDCF}', '\u{FDF0}',
            '\u{FFFD}',
        ] {
            assert!(is_tag_name_char(c), "{c:?}");
        }
        for c in [
            ':',
            '/',
            '@',
            '#',
            '!',
            ',',
            '(',
            ' ',
            '\u{D7}',
            '\u{F7}',
            '\u{37E}',
            '\u{2000}',
            '\u{2041}',
            '\u{2190}',
            '\u{3000}',
            '\u{FDD0}',
            '\u{FFFE}',
            '\u{10000}',
        ] {
            assert!(!is_tag_name_char(c), "{c:?}");
        }
    }

    // FR-TAG-02, FR-STORE-10
    #[test]
    fn a_typed_tag_name_is_trimmed_and_joined_with_dashes() {
        assert_eq!(tag_name("  deep   work "), Some("deep-work".to_owned()));
        assert_eq!(tag_name("Home"), Some("Home".to_owned()));
        assert_eq!(tag_name("été.2"), Some("été.2".to_owned()));
        for refused in [
            "", "   ", "a/b", "a:b", "@home", "done", "area", "type", "later",
        ] {
            assert_eq!(tag_name(refused), None, "{refused:?}");
        }
    }

    // FR-ORG-11
    #[test]
    fn only_a_project_or_a_task_is_later() {
        for kind in [Inbox, Area, Project, Section, Task, ChecklistItem] {
            let mut item = Item::new(kind, "x");
            assert!(!item.is_later());
            item.later = true;
            let goes = matches!(kind, Project | Task);
            assert_eq!(kind.goes_later(), goes, "{kind:?}");
            assert_eq!(item.is_later(), goes, "{kind:?}");
        }
    }

    // FR-TAG-01, FR-STORE-10
    #[test]
    fn the_users_tags_have_a_name_the_app_leaves_alone() {
        assert!(Tag::plain("home").is_users());
        assert!(!Tag::plain("due").is_users());
        let valued = Tag {
            name: "priority".to_owned(),
            value: Some("1".to_owned()),
        };
        assert!(valued.is_users());
        let date = Tag {
            name: "due".to_owned(),
            value: Some("soon".to_owned()),
        };
        assert!(!date.is_users());
    }
}
