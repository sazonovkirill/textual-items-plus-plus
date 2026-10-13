//! The outline a `.taskpaper` file holds: one line per item under a virtual root
//! ([data-model.md §1–2](../../../../specs/reference/taskpaper-terminal/specs/data-model.md)).

use super::{Origin, read, tags::Tag};

/// Names one line of an [`Outline`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineId(usize);

/// A `.taskpaper` file as a tree of lines, with how it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outline {
    /// Every line, the root first; a [`LineId`] is an index into it.
    lines: Vec<Line>,
    format: FileFormat,
}

/// One line of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    children: Vec<LineId>,
    text: String,
    kind: LineKind,
    tags: Vec<Tag>,
    display_text: String,
    extra_indent: usize,
    origin: Option<Origin>,
}

/// TaskPaper's line type ([file-format.md §4](../../../../specs/reference/taskpaper-terminal/specs/file-format.md#4-item-type)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// A line ending in `:`.
    Project,
    /// A line starting with `-` and whitespace.
    Task,
    /// Anything else, including blank lines.
    Note,
}

/// How the file was written, remembered so it can be written back the same way (F-FILE-08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFormat {
    /// The indentation of one level.
    pub indent: IndentUnit,
    /// The dominant line ending.
    pub line_ending: LineEnding,
    /// Whether the file started with a UTF-8 byte order mark.
    pub bom: bool,
    /// Whether the text ended with a line break.
    pub trailing_newline: bool,
}

/// The indentation of one outline level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentUnit {
    /// One tab per level.
    Tab,
    /// The given number of spaces per level.
    Spaces(usize),
}

/// A line ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// `\n`
    Lf,
    /// `\r\n`
    CrLf,
    /// `\r`
    Cr,
}

impl Default for FileFormat {
    /// The format of a new file: tabs, `\n`, no BOM and a trailing newline.
    fn default() -> Self {
        Self {
            indent: IndentUnit::Tab,
            line_ending: LineEnding::Lf,
            bom: false,
            trailing_newline: true,
        }
    }
}

impl Default for Outline {
    fn default() -> Self {
        Self::new(FileFormat::default())
    }
}

impl Outline {
    /// An empty outline written in `format`.
    #[must_use]
    pub fn new(format: FileFormat) -> Self {
        Self {
            lines: vec![Line::new(String::new(), 0, None)],
            format,
        }
    }

    /// The virtual root, parent of the top-level lines; it is never written to the file.
    #[must_use]
    pub fn root(&self) -> LineId {
        LineId(0)
    }

    /// The line with this id, or `None` if this outline has no such line.
    #[must_use]
    pub fn get(&self, id: LineId) -> Option<&Line> {
        self.lines.get(id.0)
    }

    /// How the file was written.
    #[must_use]
    pub fn format(&self) -> &FileFormat {
        &self.format
    }

    /// Appends `text` as the last child of `parent`, `extra_indent` levels deeper than a child
    /// would be. A line built from one that was read keeps its `origin`, so it is written back as
    /// read while its text and depth are unchanged (F-FILE-03). The text is one line: a line
    /// break in it would be read back as several lines. `None` when `parent` is not a line of
    /// this outline.
    pub fn push(
        &mut self,
        parent: LineId,
        text: String,
        extra_indent: usize,
        origin: Option<Origin>,
    ) -> Option<LineId> {
        let id = LineId(self.lines.len());
        self.lines.get_mut(parent.0)?.children.push(id);
        self.lines.push(Line::new(text, extra_indent, origin));
        Some(id)
    }

    /// Every line below `id` in document order, with its depth relative to `id`'s children (0).
    pub fn descendants(&self, id: LineId) -> impl Iterator<Item = (LineId, usize)> + '_ {
        let mut stack: Vec<(LineId, usize)> = self
            .children_of(id)
            .iter()
            .rev()
            .map(|&child| (child, 0))
            .collect();
        std::iter::from_fn(move || {
            let (id, depth) = stack.pop()?;
            stack.extend(
                self.children_of(id)
                    .iter()
                    .rev()
                    .map(|&child| (child, depth + 1)),
            );
            Some((id, depth))
        })
    }

    fn children_of(&self, id: LineId) -> &[LineId] {
        self.get(id).map_or(&[], |line| &line.children)
    }
}

impl Line {
    fn new(text: String, extra_indent: usize, origin: Option<Origin>) -> Self {
        let (kind, tags, display_text) = read(&text);
        Self {
            children: Vec::new(),
            text,
            kind,
            tags,
            display_text,
            extra_indent,
            origin,
        }
    }

    /// The child lines in document order.
    #[must_use]
    pub fn children(&self) -> &[LineId] {
        &self.children
    }

    /// The line without its indentation; everything else about it is derived from it.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The TaskPaper type of this line.
    #[must_use]
    pub fn kind(&self) -> LineKind {
        self.kind
    }

    /// The tags on this line, first occurrence per name.
    #[must_use]
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// The text without a task's `- `, a project's `:` and every tag.
    #[must_use]
    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    /// Indent levels beyond the expected depth, kept so indent jumps are written back unchanged.
    #[must_use]
    pub fn extra_indent(&self) -> usize {
        self.extra_indent
    }

    /// What this line was read from, if it was read from a file.
    #[must_use]
    pub fn origin(&self) -> Option<&Origin> {
        self.origin.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_outline_has_only_a_root() {
        let outline = Outline::default();
        let root = outline.get(outline.root()).unwrap();
        assert_eq!(root.children(), []);
        assert_eq!(outline.descendants(outline.root()).count(), 0);
    }

    #[test]
    fn default_format_is_the_new_file_format() {
        assert_eq!(
            FileFormat::default(),
            FileFormat {
                indent: IndentUnit::Tab,
                line_ending: LineEnding::Lf,
                bom: false,
                trailing_newline: true,
            }
        );
        assert_eq!(Outline::default().format(), &FileFormat::default());
    }

    #[test]
    fn push_links_parent_and_child() {
        let mut outline = Outline::default();
        let root = outline.root();
        let a = outline.push(root, "A:".into(), 0, None).unwrap();
        let b = outline.push(a, "- b".into(), 2, None).unwrap();
        assert_eq!(outline.get(root).unwrap().children(), [a]);
        assert_eq!(outline.get(a).unwrap().children(), [b]);
        assert_eq!(outline.get(b).unwrap().text(), "- b");
        assert_eq!(outline.get(b).unwrap().extra_indent(), 2);
        assert_eq!(outline.get(b).unwrap().origin(), None);
    }

    #[test]
    fn push_under_a_line_of_no_outline_adds_nothing() {
        let mut other = Outline::default();
        let other_root = other.root();
        other.push(other_root, "x".into(), 0, None);
        let stranger = other.push(other_root, "y".into(), 0, None).unwrap();
        let mut outline = Outline::default();
        assert_eq!(outline.push(stranger, "z".into(), 0, None), None);
        assert_eq!(outline, Outline::default());
        assert!(outline.get(stranger).is_none());
        assert_eq!(outline.descendants(stranger).count(), 0);
    }

    #[test]
    fn descendants_walk_depth_first_with_relative_depth() {
        let mut outline = Outline::default();
        let root = outline.root();
        let a = outline.push(root, "a".into(), 0, None).unwrap();
        let b = outline.push(a, "b".into(), 0, None).unwrap();
        let c = outline.push(b, "c".into(), 0, None).unwrap();
        let d = outline.push(root, "d".into(), 0, None).unwrap();
        assert_eq!(
            outline.descendants(root).collect::<Vec<_>>(),
            [(a, 0), (b, 1), (c, 2), (d, 0)]
        );
        assert_eq!(outline.descendants(a).collect::<Vec<_>>(), [(b, 0), (c, 1)]);
    }

    #[test]
    fn kind_tags_and_display_text_follow_the_text() {
        let mut outline = Outline::default();
        let root = outline.root();
        let project = outline.push(root, "Inbox: @a".into(), 0, None).unwrap();
        let task = outline
            .push(root, "- Milk @done @done(2026-10-01)".into(), 0, None)
            .unwrap();
        let project = outline.get(project).unwrap();
        assert_eq!(project.kind(), LineKind::Project);
        assert_eq!(project.display_text(), "Inbox");
        let task = outline.get(task).unwrap();
        assert_eq!(task.kind(), LineKind::Task);
        assert_eq!(task.display_text(), "Milk");
        assert_eq!(
            task.tags(),
            [Tag {
                span: 6..12,
                name: "done".to_owned(),
                value: None,
            }]
        );
    }
}
