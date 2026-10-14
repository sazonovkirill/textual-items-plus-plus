//! The mapping between an [`Outline`] and the domain's [`Document`] (FR-STORE-02–10, OQ-17–19):
//! which lines are which items, descriptions and attachments, and which are kept as read; and
//! writing a document back so that every node unchanged since it was read keeps its line.
//!
//! An item's line is followed by its description, its text lines (FR-STORE-04), then a
//! `NOTE @hidden` and a `COMMENTS @hidden` node holding its attachments (FR-STORE-05), then its
//! children. Description and attachment lines are guarded as taskpaper-terminal guards
//! attachment lines ([attachments.md §1.1](../../../../specs/reference/taskpaper-terminal/specs/attachments.md#11-guarding-content-lines)).

use std::collections::HashMap;

use jiff::civil::Date;

use super::{Line, LineId, LineKind, Outline, parse, serialize, split_lines, tags};
use crate::domain::{self, Branch, Completion, Document, Item, ItemId, ItemKind, Node};

/// The invisible character that guards a content line (attachments.md §1.1).
const GUARD: char = '\u{200B}';
/// The line of an item's note node.
const NOTE: &str = "NOTE @hidden";
/// The line of an item's comments node.
const COMMENTS: &str = "COMMENTS @hidden";

/// What a document was read from: the outline, which line each node came from and where the
/// Inbox was, so that [`write_document`] writes back what is unchanged as it was read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    outline: Outline,
    lines: HashMap<ItemId, LineId>,
    /// The top-level node the Inbox followed; `None` when it was first or missing.
    inbox_after: Option<ItemId>,
}

/// Where a line sits, which decides what it can be.
#[derive(Debug, Clone, Copy)]
enum Place {
    /// At the top level.
    Top,
    /// Inside an item of this kind.
    In(ItemKind),
    /// Inside kept content, where everything is kept.
    Kept,
}

/// Reads the document `outline` holds. The Inbox is the first top-level project titled `Inbox`
/// without a kind tag, and an empty one is made when there is none (FR-STORE-08). Lines the model
/// does not interpret, such as a task at the top level or a project inside a section, are kept
/// with everything under them (FR-STORE-09).
#[must_use]
pub fn read_document(outline: &Outline) -> (Document, Source) {
    let mut reader = Reader {
        outline,
        document: Document::new(),
        lines: HashMap::new(),
    };
    let mut inbox_found = false;
    let mut previous = None;
    let mut inbox_after = None;
    for &line_id in children(outline, outline.root()) {
        if !inbox_found && is_inbox(line(outline, line_id)) {
            inbox_found = true;
            inbox_after = previous;
            reader.read_inbox(line_id);
        } else {
            previous = Some(reader.read_node(None, Place::Top, line_id));
        }
    }
    let source = Source {
        outline: outline.clone(),
        lines: reader.lines,
        inbox_after,
    };
    (reader.document, source)
}

/// The outline of `document`, in the format `source` was read in. A node whose line, description
/// or attachments are unchanged since it was read gets the lines it was read from, so saving an
/// unchanged document gives back the file it was read from (F-FILE-03); the Inbox keeps its place
/// among the top-level nodes, or goes first, as it does when a project titled `Inbox` would come
/// before it and read back as the Inbox (FR-STORE-08).
#[must_use]
pub fn write_document(document: &Document, source: &Source) -> Outline {
    let mut writer = Writer {
        document,
        source,
        outline: Outline::new(source.outline.format().clone()),
    };
    let root = writer.outline.root();
    let top_level = document.top_level();
    let named_inbox = top_level.iter().position(|&id| {
        document
            .node(id)
            .and_then(Node::item)
            .is_some_and(|item| item.kind == ItemKind::Project && item.title == "Inbox")
    });
    let inbox_after = source.inbox_after.filter(|id| {
        top_level
            .iter()
            .position(|other| other == id)
            .is_some_and(|at| named_inbox.is_none_or(|named| at < named))
    });
    if inbox_after.is_none() {
        writer.write_node(root, Place::Top, document.inbox());
    }
    for &id in document.top_level() {
        writer.write_node(root, Place::Top, id);
        if inbox_after == Some(id) {
            writer.write_node(root, Place::Top, document.inbox());
        }
    }
    writer.outline
}

/// TaskPaper text holding `branches`, tab-indented from their top nodes, as a file writes new
/// items: what is copied and cut (FR-OPS-05).
#[must_use]
pub fn write_branches(branches: &[Branch]) -> String {
    let (document, source) = (Document::new(), Source::default());
    let mut writer = Writer {
        document: &document,
        source: &source,
        outline: Outline::default(),
    };
    let root = writer.outline.root();
    for branch in branches {
        writer.write_branch(root, branch);
    }
    serialize(&writer.outline)
}

/// The items TaskPaper `text` holds, as pasted (FR-OPS-05): each top-level line an area or a
/// project where the top level takes it, else an item inside a project, and a text line a task,
/// so that a pasted list becomes tasks. What is under each line reads as in a file. Blank lines at
/// the top level are left out; a top-level line tagged `@hidden` is kept as read.
///
/// # Panics
///
/// Never: the lines are read into a document of their own, where each goes where its kind fits.
#[must_use]
pub fn read_branches(text: &str) -> Vec<Branch> {
    let outline = parse(text);
    let mut reader = Reader {
        outline: &outline,
        document: Document::new(),
        lines: HashMap::new(),
    };
    let holder = reader
        .document
        .push(None, Node::Item(Item::new(ItemKind::Project, "")))
        .expect("a project fits at the top level");
    let in_project = Place::In(ItemKind::Project);
    let mut read = Vec::new();
    for &line_id in children(&outline, outline.root()) {
        let top = line(&outline, line_id);
        if top.text().trim().is_empty() && top.children().is_empty() {
            continue;
        }
        let id = if let Some(kind) = classify(top, Place::Top) {
            reader.read_as(None, Some(kind), line_id)
        } else {
            let text = top.kind() == LineKind::Note && !has_tag(top, "hidden");
            let kind = classify(top, in_project).or(text.then_some(ItemKind::Task));
            reader.read_as(Some(holder), kind, line_id)
        };
        read.push(id);
    }
    read.into_iter()
        .map(|id| {
            reader
                .document
                .branch(id)
                .expect("the lines read are in the document")
        })
        .collect()
}

struct Reader<'a> {
    outline: &'a Outline,
    document: Document,
    lines: HashMap<ItemId, LineId>,
}

impl<'a> Reader<'a> {
    fn read_inbox(&mut self, line_id: LineId) {
        let inbox = self.document.inbox();
        let (item, rest) = self.read_item(line_id, ItemKind::Inbox);
        self.document
            .set(inbox, item)
            .expect("the document's Inbox is an Inbox");
        self.lines.insert(inbox, line_id);
        self.read_children(inbox, Place::In(ItemKind::Inbox), rest);
    }

    /// Adds the line `line_id` and everything under it to `parent`, as an item where its kind
    /// fits `place` and as kept content otherwise.
    fn read_node(&mut self, parent: Option<ItemId>, place: Place, line_id: LineId) -> ItemId {
        let kind = classify(line(self.outline, line_id), place);
        self.read_as(parent, kind, line_id)
    }

    /// Adds the line `line_id` and everything under it to `parent`, as an item of `kind`, which
    /// fits there, or as kept content for `None`.
    fn read_as(
        &mut self,
        parent: Option<ItemId>,
        kind: Option<ItemKind>,
        line_id: LineId,
    ) -> ItemId {
        let read = line(self.outline, line_id);
        let (node, place, rest) = match kind {
            Some(kind) => {
                let (item, rest) = self.read_item(line_id, kind);
                (Node::Item(item), Place::In(kind), rest)
            }
            None => (
                Node::Kept(read.text().to_owned()),
                Place::Kept,
                read.children(),
            ),
        };
        let id = self
            .document
            .push(parent, node)
            .expect("lines are read as items only where their kind fits");
        self.lines.insert(id, line_id);
        self.read_children(id, place, rest);
        id
    }

    fn read_children(&mut self, parent: ItemId, place: Place, lines: &[LineId]) {
        for &child in lines {
            self.read_node(Some(parent), place, child);
        }
    }

    /// The item of kind `kind` the line `line_id` holds, and its child lines that are not its
    /// description or attachments.
    fn read_item(&self, line_id: LineId, kind: ItemKind) -> (Item, &'a [LineId]) {
        let outline = self.outline;
        let parts = Parts::of(outline, line_id);
        let mut item = fields(line(outline, line_id), kind);
        let content = |lines: &[LineId]| -> Vec<String> {
            lines
                .iter()
                .map(|&id| unguard(line(outline, id).text()))
                .collect()
        };
        item.description = content(parts.description).join("\n");
        item.note = parts
            .note
            .map(|(_, lines)| content(lines).join("\n"))
            .unwrap_or_default();
        item.comments = parts
            .comments
            .map(|(_, lines)| content(lines))
            .unwrap_or_default();
        (item, parts.rest)
    }
}

struct Writer<'a> {
    document: &'a Document,
    source: &'a Source,
    outline: Outline,
}

impl Writer<'_> {
    /// Writes `branch` under `parent` as new lines.
    fn write_branch(&mut self, parent: LineId, branch: &Branch) {
        let line_id = match &branch.node {
            Node::Kept(text) => self.push(parent, text.clone(), None),
            Node::Item(item) => {
                let line_id = self.push(parent, line_text(item), None);
                self.write_contents(line_id, item, None);
                line_id
            }
        };
        for child in &branch.children {
            self.write_branch(line_id, child);
        }
    }

    fn write_node(&mut self, parent: LineId, place: Place, id: ItemId) {
        let read_from = self.source;
        let source_id = read_from.lines.get(&id).copied();
        let source = source_id.and_then(|line_id| read_from.outline.get(line_id));
        let node = self
            .document
            .node(id)
            .expect("the document's own nodes are written");
        let children = self.document.children(id);
        let (line_id, place, children) = match node {
            Node::Kept(text) => (
                self.push(parent, text.clone(), source),
                Place::Kept,
                children.to_vec(),
            ),
            Node::Item(item) => {
                let text = match source {
                    Some(read) if reads_as(read, place, item) => read.text().to_owned(),
                    _ => line_text(item),
                };
                let line_id = self.push(parent, text, source);
                self.write_contents(line_id, item, source_id);
                (
                    line_id,
                    Place::In(item.kind),
                    self.children_order(item, source_id, children),
                )
            }
        };
        for child in children {
            self.write_node(line_id, place, child);
        }
    }

    /// The order to write `item`'s `children` in: as they are, unless the kept nodes before its
    /// first item child would read back as its description or attachments, which happens when the
    /// item that was first moved away; those nodes then go right after that child, where they
    /// were read, so that they stay kept (D-41).
    fn children_order(
        &self,
        item: &Item,
        source_id: Option<LineId>,
        children: &[ItemId],
    ) -> Vec<ItemId> {
        let document = self.document;
        let first_item = children
            .iter()
            .position(|&child| !matches!(document.node(child), Some(Node::Kept(_))));
        match first_item.and_then(|index| children.split_at_checked(index)) {
            Some((kept, [first, rest @ ..])) if self.joins_contents(item, source_id, kept) => {
                std::iter::once(first)
                    .chain(kept)
                    .chain(rest)
                    .copied()
                    .collect()
            }
            _ => children.to_vec(),
        }
    }

    /// Whether the nodes `kept`, written right after `item`'s description and attachments, would
    /// read back as part of them; worked out on a scratch outline.
    fn joins_contents(&self, item: &Item, source_id: Option<LineId>, kept: &[ItemId]) -> bool {
        if kept.is_empty() {
            return false;
        }
        let mut scratch = Writer {
            document: self.document,
            source: self.source,
            outline: Outline::new(self.outline.format().clone()),
        };
        let root = scratch.outline.root();
        let line_id = scratch.push(root, String::new(), None);
        scratch.write_contents(line_id, item, source_id);
        let contents = children(&scratch.outline, line_id).len();
        for &node in kept {
            scratch.write_node(line_id, Place::In(item.kind), node);
        }
        let parts = Parts::of(&scratch.outline, line_id);
        parts.description.len()
            + usize::from(parts.note.is_some())
            + usize::from(parts.comments.is_some())
            > contents
    }

    /// Writes `item`'s description and attachments under its line `line_id`; `source_id` is the
    /// line it was read from.
    fn write_contents(&mut self, line_id: LineId, item: &Item, source_id: Option<LineId>) {
        let source = self.source;
        let parts = source_id.map(|id| Parts::of(&source.outline, id));
        self.write_lines(
            line_id,
            &content_lines([item.description.as_str()]),
            parts.as_ref().map_or(&[], |parts| parts.description),
        );
        let note = parts.as_ref().and_then(|parts| parts.note);
        self.write_attachment(line_id, NOTE, &content_lines([item.note.as_str()]), note);
        let comments = parts.as_ref().and_then(|parts| parts.comments);
        let comment_lines = content_lines(item.comments.iter().map(String::as_str));
        self.write_attachment(line_id, COMMENTS, &comment_lines, comments);
    }

    fn write_attachment(
        &mut self,
        parent: LineId,
        header: &str,
        lines: &[&str],
        source: Option<(LineId, &[LineId])>,
    ) {
        if lines.is_empty() {
            return;
        }
        let read_from = self.source;
        let header_source = source.and_then(|(id, _)| read_from.outline.get(id));
        let node = self.push(parent, header.to_owned(), header_source);
        self.write_lines(node, lines, source.map_or(&[], |(_, lines)| lines));
    }

    /// Writes `lines` under `parent`, each from the next of the lines `originals` it was read
    /// from, if any, and guarded otherwise.
    fn write_lines(&mut self, parent: LineId, lines: &[&str], originals: &[LineId]) {
        let source = self.source;
        let outline = &source.outline;
        let mut next = 0;
        for &text in lines {
            let original = originals
                .iter()
                .enumerate()
                .skip(next)
                .find_map(|(index, &id)| {
                    let read = outline.get(id)?;
                    (unguard(read.text()) == text).then_some((index, read))
                });
            match original {
                Some((index, read)) => {
                    next = index + 1;
                    self.push(parent, read.text().to_owned(), Some(read));
                }
                None => {
                    self.push(parent, guard(text), None);
                }
            }
        }
    }

    /// Adds `text` under `parent`, at the depth and with the origin of the line `source` it was
    /// read from, if any.
    fn push(&mut self, parent: LineId, text: String, source: Option<&Line>) -> LineId {
        self.outline
            .push(
                parent,
                text,
                source.map_or(0, Line::extra_indent),
                source.and_then(Line::origin).cloned(),
            )
            .expect("the writer pushes only under lines it pushed")
    }
}

/// An item's child lines by role: its description, its note and comments nodes with their
/// content lines, and the rest.
struct Parts<'a> {
    description: &'a [LineId],
    note: Option<(LineId, &'a [LineId])>,
    comments: Option<(LineId, &'a [LineId])>,
    rest: &'a [LineId],
}

impl<'a> Parts<'a> {
    /// The parts of the line `line_id`'s children. The description is their leading run of text
    /// lines without children and not tagged `@hidden` unless guarded, less its trailing lines
    /// that are empty once unguarded, which [`write_document`] would not write; the note, then the
    /// comments, follow it directly or are not attachments.
    fn of(outline: &'a Outline, line_id: LineId) -> Self {
        let lines = children(outline, line_id);
        let mut description_len = 0;
        for (index, &child) in lines.iter().enumerate() {
            let read = line(outline, child);
            let hidden = !read.text().starts_with(GUARD) && has_tag(read, "hidden");
            if read.kind() != LineKind::Note || !read.children().is_empty() || hidden {
                break;
            }
            if !is_empty(read) {
                description_len = index + 1;
            }
        }
        let (description, rest) = lines.split_at(description_len);
        let (note, rest) = attachment(outline, rest, NOTE);
        let (comments, rest) = attachment(outline, rest, COMMENTS);
        Self {
            description,
            note,
            comments,
            rest,
        }
    }
}

/// The attachment node with the line `header` that starts `lines`, with its content lines, and
/// the lines after it. A node is an attachment only when it has content lines, none with
/// children and the last not empty once unguarded, which is how [`write_document`] writes one.
fn attachment<'a>(
    outline: &'a Outline,
    lines: &'a [LineId],
    header: &str,
) -> (Option<(LineId, &'a [LineId])>, &'a [LineId]) {
    if let Some((&first, rest)) = lines.split_first() {
        let read = line(outline, first);
        let content = read.children();
        let is_attachment = read.text() == header
            && content
                .iter()
                .all(|&id| line(outline, id).children().is_empty())
            && content
                .last()
                .is_some_and(|&id| !is_empty(line(outline, id)));
        if is_attachment {
            return (Some((first, content)), rest);
        }
    }
    (None, lines)
}

/// The kind of item `read` is in `place`, or `None` when it is content the model keeps as read:
/// a text line, a line tagged `@hidden`, a project tagged both `@area` and `@section`, or an
/// item where its kind does not fit (OQ-17, OQ-18).
fn classify(read: &Line, place: Place) -> Option<ItemKind> {
    let parent = match place {
        Place::Top => None,
        Place::In(kind) => Some(kind),
        Place::Kept => return None,
    };
    if has_tag(read, "hidden") {
        return None;
    }
    let kind = match read.kind() {
        LineKind::Task if matches!(parent, Some(ItemKind::Task | ItemKind::ChecklistItem)) => {
            ItemKind::ChecklistItem
        }
        LineKind::Task => ItemKind::Task,
        LineKind::Project => match (has_bare_tag(read, "area"), has_bare_tag(read, "section")) {
            (false, false) => ItemKind::Project,
            (true, false) => ItemKind::Area,
            (false, true) => ItemKind::Section,
            (true, true) => return None,
        },
        LineKind::Note => return None,
    };
    kind.fits_in(parent).then_some(kind)
}

/// Whether the top-level line `read` is an Inbox: a project titled `Inbox` without a kind tag.
fn is_inbox(read: &Line) -> bool {
    classify(read, Place::Top) == Some(ItemKind::Project) && read.display_text() == "Inbox"
}

/// Whether the line `read`, in `place`, holds `item`'s kind, title, dates, state and tags.
fn reads_as(read: &Line, place: Place, item: &Item) -> bool {
    let kind = if item.kind == ItemKind::Inbox {
        is_inbox(read).then_some(ItemKind::Inbox)
    } else {
        classify(read, place)
    };
    kind == Some(item.kind) && {
        let mut own = fields(read, item.kind);
        own.description.clone_from(&item.description);
        own.note.clone_from(&item.note);
        own.comments.clone_from(&item.comments);
        own == *item
    }
}

fn has_tag(read: &Line, name: &str) -> bool {
    read.tags().iter().any(|tag| tag.name == name)
}

fn has_bare_tag(read: &Line, name: &str) -> bool {
    read.tags()
        .iter()
        .any(|tag| tag.name == name && tag.value.is_none())
}

/// The item of kind `kind` that the line `read` holds, without what its children hold: its
/// title, and its tags, the dates and states among them read as fields (FR-STORE-07). A date tag
/// whose value is not a `YYYY-MM-DD` day stays a tag, and so does every other tag.
fn fields(read: &Line, kind: ItemKind) -> Item {
    let mut item = Item::new(kind, read.display_text());
    for tag in read.tags() {
        let value = tag.value.as_deref();
        let day = value.and_then(day);
        match (tag.name.as_str(), day) {
            ("start", Some(day)) => item.start = Some(day),
            ("due", Some(day)) => item.due = Some(day),
            ("done", Some(day)) => item.completed = Some(Completion::On(day)),
            ("done", None) if value.is_none() => item.completed = Some(Completion::Undated),
            ("deleted", Some(day)) => item.deleted = Some(day),
            ("area", None) if value.is_none() && kind == ItemKind::Area => {}
            ("section", None) if value.is_none() && kind == ItemKind::Section => {}
            _ => item.tags.push(domain::Tag {
                name: tag.name.clone(),
                value: tag.value.clone(),
            }),
        }
    }
    item
}

/// The day `value` names as `YYYY-MM-DD`, written exactly so.
fn day(value: &str) -> Option<Date> {
    value
        .parse::<Date>()
        .ok()
        .filter(|day| day.to_string() == value)
}

/// The line of `item`: its title as a task or a project, then its kind tag, start and due dates,
/// its other tags, and its completion and deletion (OQ-17, FR-STORE-07).
fn line_text(item: &Item) -> String {
    let mut text = match item.kind {
        ItemKind::Task | ItemKind::ChecklistItem => format!("- {}", item.title),
        ItemKind::Inbox | ItemKind::Area | ItemKind::Project | ItemKind::Section => {
            format!("{}:", item.title)
        }
    };
    let mut add = |name: &str, value: Option<String>| {
        text.push_str(" @");
        text.push_str(name);
        if let Some(value) = value {
            text.push('(');
            text.push_str(&value.replace('(', r"\(").replace(')', r"\)"));
            text.push(')');
        }
    };
    match item.kind {
        ItemKind::Area => add("area", None),
        ItemKind::Section => add("section", None),
        ItemKind::Inbox | ItemKind::Project | ItemKind::Task | ItemKind::ChecklistItem => {}
    }
    if let Some(start) = item.start {
        add("start", Some(start.to_string()));
    }
    if let Some(due) = item.due {
        add("due", Some(due.to_string()));
    }
    for tag in &item.tags {
        add(&tag.name, tag.value.clone());
    }
    match item.completed {
        Some(Completion::On(day)) => add("done", Some(day.to_string())),
        Some(Completion::Undated) => add("done", None),
        None => {}
    }
    if let Some(deleted) = item.deleted {
        add("deleted", Some(deleted.to_string()));
    }
    text
}

/// The lines of `texts`, each split at its line breaks, without the trailing empty lines, which
/// the file would read back elsewhere (file-format.md §3); guarding keeps other blank lines.
fn content_lines<'a>(texts: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut lines: Vec<&str> = texts
        .into_iter()
        .flat_map(|text| split_lines(text).0.into_iter().map(|raw| raw.text))
        .collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

/// `line` as a content line stores it, so that it reads back as text at its depth: with U+200B
/// first when it starts with whitespace, `-` or U+200B, and right after a colon that ends its
/// text before its trailing tags (attachments.md §1.1). A line tagged `@hidden` gets U+200B first
/// too, so that it is not read as a hidden node (D-33).
fn guard(line: &str) -> String {
    let mut text = String::with_capacity(line.len() + 2 * GUARD.len_utf8());
    let hidden = tags::scan(line).iter().any(|tag| tag.name == "hidden");
    if hidden || line.starts_with(|c| tags::is_whitespace(c) || c == '-' || c == GUARD) {
        text.push(GUARD);
    }
    text.push_str(line);
    let before_tags = tags::trailing_run_start(&text, &tags::scan(&text));
    let unguarded = text
        .get(..before_tags)
        .unwrap_or_default()
        .trim_end_matches(GUARD);
    if unguarded.ends_with(':') {
        text.insert(unguarded.len(), GUARD);
    }
    text
}

/// The line the user wrote, from `text` as a content line stores it: without one guard at its
/// start and, where its text before its trailing tags ends in a colon and guards, one of those.
fn unguard(text: &str) -> String {
    let mut line = text.strip_prefix(GUARD).unwrap_or(text).to_owned();
    let before_tags = tags::trailing_run_start(&line, &tags::scan(&line));
    let content = line.get(..before_tags).unwrap_or_default();
    if content.ends_with(GUARD) && content.trim_end_matches(GUARD).ends_with(':') {
        line.remove(before_tags - GUARD.len_utf8());
    }
    line
}

/// Whether the content line `read` holds nothing once unguarded.
fn is_empty(read: &Line) -> bool {
    unguard(read.text()).is_empty()
}

fn line(outline: &Outline, id: LineId) -> &Line {
    outline
        .get(id)
        .expect("ids come from the outline's own lines")
}

fn children(outline: &Outline, id: LineId) -> &[LineId] {
    line(outline, id).children()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use proptest::prelude::*;

    use super::super::{parse, parse_bytes, serialize};
    use super::*;

    /// The example file of OQ-17.
    const EXAMPLE: &str = "\
Inbox:
\t- Call the bank @due(2026-10-15)
Work: @area
\tEverything for the day job.
\tWebsite: @due(2026-11-01)
\t\tLaunch: @section
\t\t\tWhat has to happen before launch.
\t\t\t- Write the announcement @start(2026-10-20) @writing
\t\t\t\tNOTE @hidden
\t\t\t\t\tDraft in the shared doc.
\t\t\t\tCOMMENTS @hidden
\t\t\t\t\t[2026-10-13 21:30] Ask design for screenshots
\t\t\t\t- Outline
\t\t\t\t\t- Intro
\t\t\t\t- First draft @done(2026-10-13)
";

    fn load(text: &str) -> (Document, Source) {
        read_document(&parse_bytes(text.as_bytes()).unwrap())
    }

    fn save(document: &Document, source: &Source) -> String {
        serialize(&write_document(document, source))
    }

    #[track_caller]
    fn item(document: &Document, id: ItemId) -> &Item {
        document.node(id).and_then(Node::item).unwrap()
    }

    /// Whether `id` is content kept as the line `text`.
    fn is_kept(document: &Document, id: ItemId, text: &str) -> bool {
        document.node(id) == Some(&Node::Kept(text.to_owned()))
    }

    /// The only child of `id`.
    #[track_caller]
    fn only_child(document: &Document, id: ItemId) -> ItemId {
        let children = document.children(id);
        assert_eq!(children.len(), 1, "{id:?} has {children:?}");
        children[0]
    }

    /// The children of `id`, which has `N` of them.
    #[track_caller]
    fn children<const N: usize>(document: &Document, id: Option<ItemId>) -> [ItemId; N] {
        let children = id.map_or(document.top_level(), |id| document.children(id));
        children.try_into().unwrap()
    }

    fn tag(name: &str, value: Option<&str>) -> domain::Tag {
        domain::Tag {
            name: name.to_owned(),
            value: value.map(str::to_owned),
        }
    }

    /// `text` with the item `id` of the document it holds changed by `change`, as saved.
    fn edited(text: &str, id: impl Fn(&Document) -> ItemId, change: impl Fn(&mut Item)) -> String {
        let (mut document, source) = load(text);
        let id = id(&document);
        let mut changed = item(&document, id).clone();
        change(&mut changed);
        document.set(id, changed).unwrap();
        save(&document, &source)
    }

    fn first_top_level(document: &Document) -> ItemId {
        document.top_level()[0]
    }

    // FR-OPS-05
    #[test]
    fn copied_items_are_written_as_a_file_writes_them_and_read_back_alike() {
        let (document, _) = load(EXAMPLE);
        let [work] = children(&document, None);
        let branch = document.branch(work).unwrap();
        let text = write_branches(std::slice::from_ref(&branch));
        assert_eq!(text, &EXAMPLE[EXAMPLE.find("Work").unwrap()..]);
        assert_eq!(read_branches(&text), [branch]);
        assert_eq!(write_branches(&[]), "");
    }

    // FR-OPS-05
    #[test]
    fn pasted_lines_become_items_where_they_fit_and_plain_lines_become_tasks() {
        let text = "\
Milk

- Eggs @shop
Groceries:
\t- Bread
\t\tWholemeal
Later: @section
\t- x
Home: @area
Secret @hidden
\tunder it
";
        let branches = read_branches(text);
        let kinds: Vec<_> = branches
            .iter()
            .map(|branch| {
                branch
                    .node
                    .item()
                    .map(|item| (item.kind, item.title.as_str()))
            })
            .collect();
        assert_eq!(
            kinds,
            [
                Some((ItemKind::Task, "Milk")),
                Some((ItemKind::Task, "Eggs")),
                Some((ItemKind::Project, "Groceries")),
                Some((ItemKind::Section, "Later")),
                Some((ItemKind::Area, "Home")),
                None,
            ]
        );
        assert_eq!(branches[1].node.item().unwrap().tags, [tag("shop", None)]);
        let bread = branches[2].children[0].node.item().unwrap();
        assert_eq!(
            (bread.title.as_str(), bread.description.as_str()),
            ("Bread", "Wholemeal")
        );
        assert_eq!(branches[3].children.len(), 1);
        assert_eq!(branches[5].node, Node::Kept("Secret @hidden".to_owned()));
        assert_eq!(
            branches[5].children[0].node,
            Node::Kept("under it".to_owned())
        );
        assert_eq!(
            write_branches(&branches[5..]),
            "Secret @hidden\n\tunder it\n",
            "kept lines are copied as they were"
        );
        assert_eq!(read_branches("\n\n"), []);
    }

    // FR-STORE-02, FR-STORE-03, FR-STORE-04, FR-STORE-05, FR-STORE-06, FR-STORE-07,
    // FR-STORE-08, FR-STORE-10, FR-ORG-01, FR-ORG-02, FR-ORG-03, FR-ORG-04, FR-ORG-07,
    // FR-TASK-01, FR-ATT-01, FR-CHK-01
    #[test]
    fn reads_every_kind_of_item_and_what_it_holds() {
        let (document, _) = load(EXAMPLE);
        let inbox = document.inbox();
        assert_eq!(item(&document, inbox).title, "Inbox");
        let call = item(&document, only_child(&document, inbox));
        assert_eq!(
            (call.kind, call.title.as_str()),
            (ItemKind::Task, "Call the bank")
        );
        assert_eq!(call.due, Some(date(2026, 10, 15)));

        let [work] = children(&document, None);
        let work_item = item(&document, work);
        assert_eq!(work_item.kind, ItemKind::Area);
        assert_eq!(work_item.description, "Everything for the day job.");
        assert_eq!(work_item.tags, [], "the kind tag is not a user tag");
        let website = only_child(&document, work);
        assert_eq!(item(&document, website).kind, ItemKind::Project);
        assert_eq!(item(&document, website).due, Some(date(2026, 11, 1)));
        let launch = only_child(&document, website);
        assert_eq!(item(&document, launch).kind, ItemKind::Section);
        assert_eq!(
            item(&document, launch).description,
            "What has to happen before launch."
        );

        let announce_id = only_child(&document, launch);
        let announce = item(&document, announce_id);
        assert_eq!(announce.kind, ItemKind::Task);
        assert_eq!(announce.start, Some(date(2026, 10, 20)));
        assert_eq!(announce.tags, [tag("writing", None)]);
        assert_eq!(announce.note, "Draft in the shared doc.");
        assert_eq!(
            announce.comments,
            ["[2026-10-13 21:30] Ask design for screenshots"]
        );
        let [outline, draft] = children(&document, Some(announce_id));
        assert_eq!(item(&document, outline).kind, ItemKind::ChecklistItem);
        let intro = item(&document, only_child(&document, outline));
        assert_eq!(
            (intro.kind, intro.title.as_str()),
            (ItemKind::ChecklistItem, "Intro")
        );
        assert_eq!(
            item(&document, draft).completed,
            Some(Completion::On(date(2026, 10, 13)))
        );
    }

    // F-FILE-03, FR-STORE-09, N-REL-02
    #[test]
    fn an_unchanged_document_is_written_back_byte_for_byte() {
        for text in [
            EXAMPLE,
            "Inbox:",
            "Inbox:\r\n\t- a\r\n",
            "Inbox:\n    - a  @due(2026-10-15)   @x\n        text\n",
            "Inbox:\n\t- Free text at the end @future(next week)\nNotes\n- a top-level task\n\t\tindented twice\n",
            "Inbox:\nP: @area @section\n\t- x\nA:\n\tB: @area\n\tS: @section\n\t\tP2:\n\t\t\t- y\n\t\tnote after\n",
            "Inbox:\n- t\nP:\n\t- t\n\t\tLOG @hidden\n\t\t\tentry\n\t\tNOTE @hidden\n\t\tCOMMENTS @hidden\n\t\t\tc\n\t\t\t\tdeeper\n",
            "Inbox:\n\t- t\n\t\t\n\t\tafter a blank\n\t\tNOTE @hidden\n\t\t\tn\n\t\t\t\n\t\t\tm\n\n\t- u\n\n",
            "Inbox:\n\t- t\n\t\tNOTE @hidden\n\t\t\t\u{200B}- item\n\t\t\tSteps:\u{200B}\n\t\tCOMMENTS @hidden\n\t\t\tx\n\t\tNOTE @hidden\n\t\t\ty\n",
            "\u{FEFF}Inbox:\r\t- t @done\r\t- u @done(soon) @deleted(2026-01-01)\r",
            "Inbox:\n\t- t\n\t\tx\n\t\t\u{200B}\n\t\tNOTE @hidden\n\t\t\t\u{200B}\n\t- u\n\t\tNOTE @hidden\n\t\t\tn\n\t\t\t\u{200B}\n",
            "Inbox:\n\t- t\n\t\tLOG @hidden\n\t\tNOTE @hidden\n\t\t\tn\n",
        ] {
            let (document, source) = load(text);
            assert_eq!(save(&document, &source), text, "{text:?}");
        }
    }

    proptest! {
        // F-FILE-03, FR-STORE-09, N-REL-02
        #[test]
        fn any_file_with_an_inbox_first_is_written_back_byte_for_byte(
            lines in prop::collection::vec(
                (0..4_usize, prop::sample::select(vec![
                    "- task", "- done @done(2026-10-13)", "- odd @done(x)", "Project:",
                    "Area: @area", "Section: @section", "text", "", "NOTE @hidden",
                    "COMMENTS @hidden", "\u{200B}- guarded", "Goals:\u{200B}", "X: @hidden",
                    "- t @start(2026-01-02) @due(2026-01-03) @x(y)", "Inbox:", "\u{200B}",
                    "L @hidden", "\u{200B}L @hidden",
                ])),
                0..24,
            ),
            line_break in prop::sample::select(vec!["\n", "\r\n", "\r"]),
        ) {
            let mut text = format!("Inbox:{line_break}");
            for (depth, line) in lines {
                text.push_str(&"\t".repeat(depth));
                text.push_str(line);
                text.push_str(line_break);
            }
            let (document, source) = load(&text);
            prop_assert_eq!(save(&document, &source), text);
        }

        // FR-STORE-04, FR-STORE-09, N-REL-02
        #[test]
        fn removing_a_node_changes_no_other_item_that_keeps_an_item_child(
            lines in prop::collection::vec(
                (1..4_usize, prop::sample::select(vec![
                    "- task", "Project:", "Section: @section", "text", "", "NOTE @hidden",
                    "COMMENTS @hidden", "L @hidden", "\u{200B}- guarded",
                ])),
                0..24,
            ),
            pick in any::<prop::sample::Index>(),
        ) {
            let mut text = "Inbox:\nA: @area\n".to_owned();
            for (depth, line) in lines {
                text.push_str(&"\t".repeat(depth));
                text.push_str(line);
                text.push('\n');
            }
            let (mut document, source) = load(&text);
            let nodes = descendants(&document, first_top_level(&document));
            prop_assume!(!nodes.is_empty());
            remove(&mut document, *pick.get(&nodes));
            let (reloaded, _) = load(&save(&document, &source));
            let after = items(&reloaded);
            prop_assert_eq!(items(&document).len(), after.len());
            for ((id, before), after) in items(&document).into_iter().zip(after) {
                let has_item_child = document
                    .children(id)
                    .iter()
                    .any(|&child| document.node(child).and_then(Node::item).is_some());
                if has_item_child {
                    prop_assert_eq!(before, after.1);
                } else {
                    prop_assert_eq!((before.kind, &before.title), (after.1.kind, &after.1.title));
                }
            }
        }
    }

    /// `document` without the node `id` and its subtree.
    fn remove(document: &mut Document, id: ItemId) {
        let (parent, index) = document.position(id).unwrap();
        let subtree = document.snapshot(id).unwrap();
        let remove = domain::Op::Remove {
            parent,
            index,
            subtree,
        };
        document
            .apply(&domain::Transaction::new(vec![remove]))
            .unwrap();
    }

    /// `text` without the node `id` picks from the document it holds, as saved.
    fn removed(text: &str, id: impl Fn(&Document) -> ItemId) -> String {
        let (mut document, source) = load(text);
        let id = id(&document);
        remove(&mut document, id);
        save(&document, &source)
    }

    /// Every node under `id`, in file order.
    fn descendants(document: &Document, id: ItemId) -> Vec<ItemId> {
        document
            .children(id)
            .iter()
            .flat_map(|&child| std::iter::once(child).chain(descendants(document, child)))
            .collect()
    }

    /// The items of `document` in file order, without kept content.
    fn items(document: &Document) -> Vec<(ItemId, &Item)> {
        let mut ids = vec![document.inbox()];
        ids.extend(descendants(document, document.inbox()));
        for &id in document.top_level() {
            ids.push(id);
            ids.extend(descendants(document, id));
        }
        ids.into_iter()
            .filter_map(|id| Some((id, document.node(id)?.item()?)))
            .collect()
    }

    // FR-STORE-04, FR-STORE-09, N-REL-02
    #[test]
    fn kept_lines_stay_kept_when_the_first_child_moves_away() {
        let first = |document: &Document| {
            let children = document.children(first_top_level(document));
            *children
                .iter()
                .find(|&&id| document.node(id).and_then(Node::item).is_some())
                .unwrap()
        };
        for (text, expected) in [
            (
                "Inbox:\nP:\n\t- a\n\ttext\n\t- b\n",
                "Inbox:\nP:\n\t- b\n\ttext\n",
            ),
            (
                "Inbox:\nP:\n\tabout\n\t- a\n\ttext\n\t\n\tmore\n\t- b\n\t- c\n",
                "Inbox:\nP:\n\tabout\n\t- b\n\ttext\n\t\n\tmore\n\t- c\n",
            ),
            (
                "Inbox:\nP:\n\t- a\n\tNOTE @hidden\n\t\tn\n\t- b\n",
                "Inbox:\nP:\n\t- b\n\tNOTE @hidden\n\t\tn\n",
            ),
            (
                "Inbox:\nP:\n\tL @hidden\n\t- a\n\ttext\n\t- b\n",
                "Inbox:\nP:\n\tL @hidden\n\ttext\n\t- b\n",
            ),
            ("Inbox:\nP:\n\t- a\n\t\n\t- b\n", "Inbox:\nP:\n\t\n\t- b\n"),
        ] {
            let saved = removed(text, first);
            assert_eq!(saved, expected, "{text:?}");
            let (original, _) = load(text);
            let (reloaded, _) = load(&saved);
            assert_eq!(
                item(&reloaded, first_top_level(&reloaded)),
                item(&original, first_top_level(&original)),
                "{text:?}"
            );
        }
    }

    // FR-STORE-04
    #[test]
    fn kept_lines_with_no_item_left_before_them_read_as_the_description() {
        let first = |document: &Document| document.children(first_top_level(document))[0];
        let saved = removed("Inbox:\nP:\n\t- a\n\ttext\n", first);
        assert_eq!(saved, "Inbox:\nP:\n\ttext\n");
        let (document, _) = load(&saved);
        assert_eq!(
            item(&document, first_top_level(&document)).description,
            "text"
        );
    }

    // FR-STORE-08
    #[test]
    fn a_missing_inbox_is_written_first() {
        let (document, source) = load("Work: @area\n\t- t\n");
        assert_eq!(document.children(document.inbox()), []);
        assert_eq!(save(&document, &source), "Inbox:\nWork: @area\n\t- t\n");
        let (document, source) = load("");
        assert_eq!(save(&document, &source), "Inbox:");
    }

    // FR-STORE-08
    #[test]
    fn the_inbox_keeps_its_place_in_the_file() {
        let text = "A: @area\nInbox:\n\t- t\nB:\n";
        let (document, source) = load(text);
        assert_eq!(document.top_level().len(), 2);
        assert_eq!(document.children(document.inbox()).len(), 1);
        assert_eq!(save(&document, &source), text);
        assert_eq!(
            edited(text, Document::inbox, |inbox| inbox.note = "n".to_owned()),
            "A: @area\nInbox:\n\tNOTE @hidden\n\t\tn\n\t- t\nB:\n"
        );
    }

    // FR-STORE-08
    #[test]
    fn the_inbox_goes_before_a_project_titled_inbox_that_would_come_first() {
        let text = "A: @area\nB:\nInbox:\n\t- t\n";
        let (mut document, source) = load(text);
        let [_, b] = children(&document, None);
        let renamed = Item::new(ItemKind::Project, "Inbox");
        document.set(b, renamed).unwrap();
        assert_eq!(
            save(&document, &source),
            "Inbox:\n\t- t\nA: @area\nInbox:\n"
        );
        let (read, _) = load(&save(&document, &source));
        assert_eq!(read.children(read.inbox()).len(), 1);
    }

    // FR-STORE-08
    #[test]
    fn only_the_first_top_level_project_named_inbox_without_a_kind_tag_is_the_inbox() {
        let (document, _) = load("Inbox: @area\n\tInbox:\nInbox: @tag\nInbox:\n");
        let [area, second] = children(&document, None);
        assert_eq!(item(&document, area).kind, ItemKind::Area);
        assert_eq!(
            item(&document, only_child(&document, area)).kind,
            ItemKind::Project
        );
        assert_eq!(item(&document, document.inbox()).tags, [tag("tag", None)]);
        assert_eq!(item(&document, second).kind, ItemKind::Project);
    }

    // FR-STORE-02, FR-STORE-09, FR-ORG-01, FR-ORG-03, FR-ORG-04
    #[test]
    fn items_where_their_kind_does_not_fit_are_kept_with_their_contents() {
        let (document, _) = load(concat!(
            "Inbox:\n",
            "\tP:\n",
            "- top-level task\n",
            "\t- under it\n",
            "S: @section\n",
            "A: @area\n",
            "\tB: @area\n",
            "\tS: @section\n",
            "\t\tP:\n",
            "Both: @area @section\n",
        ));
        let inbox_child = only_child(&document, document.inbox());
        assert!(is_kept(&document, inbox_child, "P:"));
        let [task, section, area, both] = children(&document, None);
        assert!(is_kept(&document, task, "- top-level task"));
        assert!(is_kept(
            &document,
            only_child(&document, task),
            "- under it"
        ));
        assert!(is_kept(&document, section, "S: @section"));
        let [nested_area, inner] = children(&document, Some(area));
        assert!(is_kept(&document, nested_area, "B: @area"));
        assert_eq!(item(&document, inner).kind, ItemKind::Section);
        assert!(is_kept(&document, only_child(&document, inner), "P:"));
        assert!(is_kept(&document, both, "Both: @area @section"));
    }

    // FR-STORE-04, FR-STORE-05, FR-STORE-09
    #[test]
    fn text_after_the_leading_run_and_unknown_hidden_nodes_are_kept() {
        let (document, _) = load(concat!(
            "Inbox:\n",
            "\t- t\n",
            "\t\tdescription\n",
            "\t\t\n",
            "\t\tNOTE @hidden\n",
            "\t\t\tnot attached after a blank line\n",
            "\t\tLOG @hidden\n",
            "\t\t- check @hidden\n",
            "\t\ttext with a child\n",
            "\t\t\t- c\n",
        ));
        let task_id = only_child(&document, document.inbox());
        let task = item(&document, task_id);
        assert_eq!(task.description, "description");
        assert_eq!(task.note, "");
        let nodes: Vec<Node> = document
            .children(task_id)
            .iter()
            .filter_map(|&id| document.node(id).cloned())
            .collect();
        let kept = |texts: &[&str]| -> Vec<Node> {
            texts
                .iter()
                .map(|&text| Node::Kept(text.to_owned()))
                .collect()
        };
        assert_eq!(
            nodes,
            kept(&[
                "",
                "NOTE @hidden",
                "LOG @hidden",
                "- check @hidden",
                "text with a child"
            ])
        );
    }

    // FR-STORE-05
    #[test]
    fn attachments_follow_the_description_in_order_and_hold_flat_text() {
        let read = |text: &str| {
            let (document, _) = load(text);
            let task = only_child(&document, document.inbox());
            let item = item(&document, task).clone();
            (item.note, item.comments, document.children(task).len())
        };
        assert_eq!(
            read("Inbox:\n\t- t\n\t\tNOTE @hidden\n\t\t\ta\n\t\t\t\n\t\t\tb\n"),
            ("a\n\nb".to_owned(), vec![], 0)
        );
        assert_eq!(
            read("Inbox:\n\t- t\n\t\tCOMMENTS @hidden\n\t\t\tc\n\t\tNOTE @hidden\n\t\t\tn\n"),
            (String::new(), vec!["c".to_owned()], 1),
            "a note after the comments is not the note"
        );
        assert_eq!(
            read("Inbox:\n\t- t\n\t\tNOTE @hidden\n\t\t\ta\n\t\t\t\tb\n"),
            (String::new(), vec![], 1),
            "nested content is kept"
        );
        assert_eq!(
            read("Inbox:\n\t- t\n\t\tNOTE @hidden @x\n\t\t\ta\n"),
            (String::new(), vec![], 1),
            "only the exact line is a note"
        );
    }

    // FR-STORE-07, FR-STORE-10
    #[test]
    fn dates_and_states_are_read_from_their_tags_and_other_tags_are_kept() {
        let (document, _) = load(concat!(
            "Inbox:\n",
            "\t- a @start(2026-10-01) @due(2026-10-02) @done(2026-10-03) @deleted(2026-10-04)\n",
            "\t- b @done @start(next week) @due(2026-1-5) @deleted @home @area @section\n",
            "A: @area(x)\n",
            "\tS: @section(y)\n",
            "\t- c @due(2026-10-01) @due(2026-10-09)\n",
        ));
        let [a, b] = children(&document, Some(document.inbox()));
        let a = item(&document, a);
        assert_eq!(
            (a.start, a.due, a.completed, a.deleted),
            (
                Some(date(2026, 10, 1)),
                Some(date(2026, 10, 2)),
                Some(Completion::On(date(2026, 10, 3))),
                Some(date(2026, 10, 4))
            )
        );
        assert_eq!(a.tags, []);
        let b = item(&document, b);
        assert_eq!((b.start, b.due, b.deleted), (None, None, None));
        assert_eq!(b.completed, Some(Completion::Undated));
        assert_eq!(
            b.tags,
            [
                tag("start", Some("next week")),
                tag("due", Some("2026-1-5")),
                tag("deleted", None),
                tag("home", None),
                tag("area", None),
                tag("section", None),
            ]
        );
        let [project] = children(&document, None);
        let project_item = item(&document, project);
        assert_eq!(project_item.kind, ItemKind::Project);
        assert_eq!(project_item.tags, [tag("area", Some("x"))]);
        let [section, task] = children(&document, Some(project));
        assert_eq!(item(&document, section).kind, ItemKind::Project);
        assert_eq!(
            item(&document, task).due,
            Some(date(2026, 10, 1)),
            "the first of a tag counts"
        );
    }

    // FR-STORE-04
    #[test]
    fn guards_are_removed_on_read() {
        let (document, _) = load(concat!(
            "Inbox:\n",
            "\t- t\n",
            "\t\t\u{200B}- item\n",
            "\t\tGoals:\u{200B}\n",
            "\t\t\u{200B}  code\n",
            "\t\ta\u{200B}b\n",
        ));
        let task = item(&document, only_child(&document, document.inbox()));
        assert_eq!(task.description, "- item\nGoals:\n  code\na\u{200B}b");
    }

    // FR-STORE-04, FR-STORE-05, FR-STORE-09
    #[test]
    fn hidden_and_empty_lines_never_read_as_content() {
        let (document, _) = load(concat!(
            "Inbox:\n",
            "\t- t\n",
            "\t\tLOG @hidden\n",
            "\t- u\n",
            "\t\td\n",
            "\t\t\u{200B}\n",
            "\t\tNOTE @hidden\n",
            "\t\t\tn\n",
            "\t- v\n",
            "\t\tNOTE @hidden\n",
            "\t\t\t\u{200B}\n",
        ));
        let [t, u, v] = children(&document, Some(document.inbox()));
        assert_eq!(item(&document, t).description, "");
        assert!(is_kept(&document, only_child(&document, t), "LOG @hidden"));
        assert_eq!(item(&document, u).description, "d");
        assert_eq!(
            item(&document, u).note,
            "",
            "the note no longer follows the description"
        );
        assert_eq!(document.children(u).len(), 2);
        assert_eq!(item(&document, v).note, "");
        assert!(is_kept(&document, only_child(&document, v), "NOTE @hidden"));
    }

    // FR-STORE-04
    #[test]
    fn a_description_line_with_a_hidden_tag_reads_back_as_written() {
        let description = "Use @hidden\nLOG @hidden";
        let text = edited(
            "Inbox:\n\t- t\n",
            |document| only_child(document, document.inbox()),
            |task| task.description = description.to_owned(),
        );
        let (document, _) = load(&text);
        let task = only_child(&document, document.inbox());
        assert_eq!(item(&document, task).description, description);
        assert_eq!(document.children(task), []);
    }

    // FR-STORE-04, FR-STORE-05
    #[test]
    fn content_is_guarded_on_write_and_reads_back_as_written() {
        let description = "- item\nGoals:\n  code\n\u{200B}x\nMeeting: @at(10:30)\n\nlast";
        let note = "Steps:\n\t- a\nNOTE @hidden";
        let comments = vec!["[2026-10-13 21:30] Re: it".to_owned(), "plain".to_owned()];
        let text = edited(
            "Inbox:\n\t- t\n",
            |document| only_child(document, document.inbox()),
            |task| {
                task.description = description.to_owned();
                task.note = note.to_owned();
                task.comments.clone_from(&comments);
            },
        );
        let (document, _) = load(&text);
        let task = item(&document, only_child(&document, document.inbox()));
        assert_eq!(task.description, description);
        assert_eq!(task.note, note);
        assert_eq!(task.comments, comments);
        assert_eq!(document.children(document.inbox()).len(), 1);
    }

    // FR-STORE-04, FR-STORE-05
    #[test]
    fn content_is_written_after_the_item_line_in_order() {
        assert_eq!(
            edited(
                "Inbox:\n\t- t\n\t\t- check\n",
                |document| only_child(document, document.inbox()),
                |task| {
                    task.description = "d\r\ne\n\n".to_owned();
                    task.note = "n".to_owned();
                    task.comments = vec!["c2".to_owned(), "c1".to_owned()];
                },
            ),
            "Inbox:\n\t- t\n\t\td\n\t\te\n\t\tNOTE @hidden\n\t\t\tn\n\t\tCOMMENTS @hidden\n\t\t\tc2\n\t\t\tc1\n\t\t- check\n"
        );
    }

    // FR-STORE-04, FR-STORE-05, FR-STORE-09
    #[test]
    fn unchanged_content_lines_keep_their_bytes() {
        // A note line another app wrote unguarded is read as written and kept so.
        let text =
            "Inbox:\r\n\t- t\r\n\t\tkeep\r\n\t\tNOTE @hidden\r\n\t\t\tGoals:\r\n\t\t\told\r\n";
        let (document, _) = load(text);
        let task_id = only_child(&document, document.inbox());
        assert_eq!(item(&document, task_id).note, "Goals:\nold");
        assert_eq!(
            edited(
                text,
                |document| only_child(document, document.inbox()),
                |task| task.note = "new\nGoals:\nmore".to_owned(),
            ),
            "Inbox:\r\n\t- t\r\n\t\tkeep\r\n\t\tNOTE @hidden\r\n\t\t\tnew\r\n\t\t\tGoals:\r\n\t\t\tmore\r\n"
        );
    }

    // FR-STORE-05
    #[test]
    fn emptied_attachments_and_descriptions_are_removed() {
        assert_eq!(
            edited(
                "Inbox:\n\t- t\n\t\td\n\t\tNOTE @hidden\n\t\t\tn\n\t\tCOMMENTS @hidden\n\t\t\tc\n\t\t- check\n",
                |document| only_child(document, document.inbox()),
                |task| {
                    task.description.clear();
                    task.note = "\n".to_owned();
                    task.comments.clear();
                },
            ),
            "Inbox:\n\t- t\n\t\t- check\n"
        );
    }

    // FR-STORE-02, FR-STORE-03, FR-STORE-07, FR-STORE-10
    #[test]
    fn a_changed_item_line_is_written_from_the_item() {
        let task = |document: &Document| only_child(document, document.inbox());
        assert_eq!(
            edited("Inbox:\n\t- a  @x  @due(2026-10-01)\n", task, |task| {
                task.title = "b".to_owned();
            }),
            "Inbox:\n\t- b @due(2026-10-01) @x\n"
        );
        assert_eq!(
            edited("Inbox:\n\t- a @done(2026-10-01)\n", task, |task| {
                task.completed = None;
            }),
            "Inbox:\n\t- a\n"
        );
        assert_eq!(
            edited("Work: @area\n", first_top_level, |area| {
                area.title = "Home".to_owned();
            }),
            "Inbox:\nHome: @area\n"
        );
    }

    // FR-STORE-02, FR-STORE-07, FR-STORE-10
    #[test]
    fn item_lines_carry_kind_dates_tags_and_states_in_order() {
        let mut item = Item::new(ItemKind::Section, "Launch");
        item.start = Some(date(2026, 10, 1));
        item.due = Some(date(2026, 10, 2));
        item.tags = vec![tag("home", None), tag("note", Some("a (b) c"))];
        item.completed = Some(Completion::Undated);
        item.deleted = Some(date(2026, 10, 3));
        assert_eq!(
            line_text(&item),
            r"Launch: @section @start(2026-10-01) @due(2026-10-02) @home @note(a \(b\) c) @done @deleted(2026-10-03)"
        );
        item.kind = ItemKind::Area;
        item.completed = Some(Completion::On(date(2026, 10, 4)));
        assert!(line_text(&item).starts_with("Launch: @area @start"));
        let read = parse(&line_text(&item));
        let line = read
            .get(read.get(read.root()).unwrap().children()[0])
            .unwrap();
        assert_eq!(fields(line, ItemKind::Area), item);
        for kind in [
            ItemKind::Inbox,
            ItemKind::Project,
            ItemKind::Task,
            ItemKind::ChecklistItem,
        ] {
            let item = Item::new(kind, "x");
            let line = line_text(&item);
            let read = parse(&line);
            let read = read
                .get(read.get(read.root()).unwrap().children()[0])
                .unwrap();
            assert_eq!(fields(read, kind), item, "{line}");
        }
    }

    // FR-STORE-02, FR-STORE-03, FR-STORE-06, FR-STORE-08
    #[test]
    fn a_new_document_is_written_from_its_items() {
        let mut document = Document::new();
        let area = document
            .push(None, Node::Item(Item::new(ItemKind::Area, "Work")))
            .unwrap();
        let section = document
            .push(Some(area), Node::Item(Item::new(ItemKind::Section, "Next")))
            .unwrap();
        let task = document
            .push(Some(section), Node::Item(Item::new(ItemKind::Task, "Plan")))
            .unwrap();
        document
            .push(
                Some(task),
                Node::Item(Item::new(ItemKind::ChecklistItem, "Step")),
            )
            .unwrap();
        document
            .push(Some(area), Node::Kept("free text".to_owned()))
            .unwrap();
        let text = save(&document, &Source::default());
        assert_eq!(
            text,
            "Inbox:\nWork: @area\n\tNext: @section\n\t\t- Plan\n\t\t\t- Step\n\tfree text\n"
        );
        let (reread, _) = load(&text);
        let reread_area = reread.top_level()[0];
        assert_eq!(item(&reread, reread_area).kind, ItemKind::Area);
        assert!(is_kept(
            &reread,
            reread.children(reread_area)[1],
            "free text"
        ));
    }

    // FR-ORG-06
    #[test]
    fn an_item_changed_into_another_kind_gets_a_new_line() {
        let (document, source) = load("Inbox:\nP:\n\t- t\n");
        let mut changed = Document::new();
        let area = changed
            .push(None, Node::Item(Item::new(ItemKind::Area, "P")))
            .unwrap();
        // Numbered as the read document was, the area stands where the project's line was read,
        // as a converted item will (FR-ORG-06); its line must say it is an area.
        assert_eq!(area, document.top_level()[0]);
        assert_eq!(save(&changed, &source), "Inbox:\nP: @area\n");
    }

    #[test]
    fn guard_and_unguard_are_inverse() {
        assert_eq!(guard(""), "");
        for line in [
            "plain",
            "- item",
            " lead",
            "\tlead",
            "\u{200B}x",
            "Steps:",
            "Steps:\u{200B}",
            "Meeting: @at(10:30)",
            "- Steps:",
            ":",
            "a\u{200B}b",
            "Use @hidden",
        ] {
            assert_eq!(unguard(&guard(line)), line, "{line:?}");
            let read = parse(&format!("P:\n\t{}", guard(line)));
            let child = read.get(read.root()).unwrap().children()[0];
            let child = read.get(read.get(child).unwrap().children()[0]).unwrap();
            assert_eq!(child.kind(), LineKind::Note, "{line:?}");
        }
        assert_eq!(unguard(":\u{200B}\u{200B} @t"), ":\u{200B} @t");
        assert_eq!(unguard("a\u{200B}"), "a\u{200B}");
    }
}
