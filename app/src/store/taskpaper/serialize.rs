//! Writing an [`Outline`] as `.taskpaper` text ([file-format.md §6](../../../../specs/reference/taskpaper-terminal/specs/file-format.md#6-serialization)):
//! decides which lines can be written back from their original bytes and how the others are indented and ended.

use std::borrow::Cow;

use super::{BOM, FileFormat, IndentUnit, Line, LineEnding, Origin, Outline, is_whitespace, parse};

/// The file contents for `outline`, BOM included: an unedited outline gives back the exact text
/// it was parsed from (F-FILE-03), and new or edited lines use the remembered indent unit and line
/// ending (F-FILE-08). An indent jump that would nest a line under an earlier sibling is dropped.
/// When the text would not read back at the outline's depths, its lines are
/// re-indented with the remembered unit, or else with tabs, which read back for every canonical
/// outline (D-31).
#[must_use]
pub fn serialize(outline: &Outline) -> String {
    let format = outline.format();
    let lines = lines_with_depth(outline);
    let first = body(&lines, format, format.indent, true);
    let body = if reads_back(&first, &lines) {
        first
    } else {
        [format.indent, IndentUnit::Tab]
            .into_iter()
            .map(|unit| body(&lines, format, unit, false))
            .find(|body| reads_back(body, &lines))
            .unwrap_or(first)
    };
    if format.bom {
        format!("{BOM}{body}")
    } else {
        body
    }
}

/// The text without a BOM: each line indented with `unit`, or as read where `reuse_originals`
/// and it still reads back the same, and ended by its line break.
fn body(
    lines: &[(&Line, usize)],
    format: &FileFormat,
    unit: IndentUnit,
    reuse_originals: bool,
) -> String {
    let contents = line_contents(lines, unit, reuse_originals);
    let mut text = String::new();
    let last = lines.len().checked_sub(1);
    let mut previous_break = "";
    for (index, (content, &(line, _))) in contents.iter().zip(lines).enumerate() {
        text.push_str(content);
        // An empty last line is only read back when a line break ends it (file-format.md §3.1).
        if Some(index) != last || format.trailing_newline || content.is_empty() {
            let mut this_break = line_break(line, format.line_ending);
            // A `\r` directly followed by `\n` reads back as one break, losing this empty line.
            if content.is_empty() && previous_break == "\r" && this_break.starts_with('\n') {
                this_break = "\r";
            }
            text.push_str(this_break);
            previous_break = this_break;
        }
    }
    text
}

/// Whether `body` parses back to `lines` at their depths. The depths of lines written as read were
/// read with the file's smallest run of leading spaces (file-format.md §2), so an edit that
/// removes that run, or leaves no line one level deep, would change them.
fn reads_back(body: &str, lines: &[(&Line, usize)]) -> bool {
    let reread = parse(body);
    lines_with_depth(&reread)
        .iter()
        .map(|&(_, depth)| depth)
        .eq(lines.iter().map(|&(_, depth)| depth))
}

/// Each line without its break: the original line where `reuse_originals` and it still reads
/// back the same, otherwise indent and text.
fn line_contents<'a>(
    lines: &[(&'a Line, usize)],
    unit: IndentUnit,
    reuse_originals: bool,
) -> Vec<Cow<'a, str>> {
    lines
        .iter()
        .zip(next_text_depths(lines))
        .map(|(&(line, depth), next_text_depth)| {
            match line.origin().filter(|origin| {
                reuse_originals && rereads_as(origin, line, depth, next_text_depth)
            }) {
                Some(origin) => Cow::Borrowed(origin.line.as_str()),
                None => Cow::Owned(indent(unit, depth) + line.text()),
            }
        })
        .collect()
}

/// The break that followed the line as read, so an edited or moved line keeps it; the
/// remembered line ending for new lines and for a former last line without one.
fn line_break(line: &Line, line_ending: LineEnding) -> &'static str {
    line.origin()
        .map(|origin| origin.line_break)
        .filter(|line_break| !line_break.is_empty())
        .unwrap_or(match line_ending {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        })
}

/// Whether the line as read, written where `line` now is, parses back to its text and depth.
fn rereads_as(origin: &Origin, line: &Line, depth: usize, next_text_depth: Option<usize>) -> bool {
    if origin.line.get(origin.text_start..) != Some(line.text()) {
        return false;
    }
    let reread_depth = if is_blank(line) {
        next_text_depth.map_or(0, |next| origin.depth.max(next))
    } else {
        origin.depth
    };
    reread_depth == depth
}

/// Every line in document order with its absolute depth ([data-model.md §2](../../../../specs/reference/taskpaper-terminal/specs/data-model.md#2-derived-line-attributes)),
/// without its extra indent where that would make it deeper than an earlier sibling, which would
/// then read back as its parent ([file-format.md §3.1](../../../../specs/reference/taskpaper-terminal/specs/file-format.md#31-canonical-outlines), rule 1).
fn lines_with_depth(outline: &Outline) -> Vec<(&Line, usize)> {
    let mut path_depths: Vec<usize> = Vec::new();
    outline
        .descendants(outline.root())
        .filter_map(|(id, level)| outline.get(id).map(|line| (line, level)))
        .map(|(line, level)| {
            let previous_sibling = path_depths.get(level).copied();
            path_depths.truncate(level);
            let child_depth = path_depths.last().map_or(0, |parent| parent + 1);
            let depth = Some(child_depth + line.extra_indent())
                .filter(|&depth| previous_sibling.is_none_or(|sibling| depth <= sibling))
                .unwrap_or(child_depth);
            path_depths.push(depth);
            (line, depth)
        })
        .collect()
}

/// For each line, the depth of the next line that is not blank, which a blank line takes when deeper.
fn next_text_depths(lines: &[(&Line, usize)]) -> Vec<Option<usize>> {
    let mut next = None;
    let mut depths: Vec<_> = lines
        .iter()
        .rev()
        .map(|&(line, depth)| {
            let after = next;
            if !is_blank(line) {
                next = Some(depth);
            }
            after
        })
        .collect();
    depths.reverse();
    depths
}

fn is_blank(line: &Line) -> bool {
    line.text().chars().all(is_whitespace)
}

fn indent(unit: IndentUnit, depth: usize) -> String {
    match unit {
        IndentUnit::Tab => "\t".repeat(depth),
        IndentUnit::Spaces(per_level) => " ".repeat(per_level.saturating_mul(depth)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use proptest::prelude::*;

    use super::super::{LineId, parse_bytes, tests::shape};
    use super::*;

    #[track_caller]
    fn check_round_trip(text: &str) {
        let outline = parse_bytes(text.as_bytes()).unwrap();
        assert_eq!(serialize(&outline), text, "round trip of {text:?}");
    }

    fn origin(line: &str, text_start: usize, depth: usize, line_break: &'static str) -> Origin {
        Origin {
            line: line.to_owned(),
            text_start,
            depth,
            line_break,
        }
    }

    /// Appends a line that was read as `origin`.
    fn push_read(
        outline: &mut Outline,
        parent: LineId,
        text: &str,
        extra_indent: usize,
        origin: Origin,
    ) -> LineId {
        outline
            .push(parent, text.to_owned(), extra_indent, Some(origin))
            .unwrap()
    }

    /// Appends a new line.
    fn push_new(outline: &mut Outline, parent: LineId, text: &str, extra_indent: usize) -> LineId {
        outline
            .push(parent, text.to_owned(), extra_indent, None)
            .unwrap()
    }

    /// A copy of `source` keeping every line's origin, without the lines `skip` picks.
    fn rebuilt_without(source: &Outline, skip: impl Fn(&Line) -> bool) -> Outline {
        fn copy(
            source: &Outline,
            from: LineId,
            target: &mut Outline,
            to: LineId,
            skip: &impl Fn(&Line) -> bool,
        ) {
            let line = source.get(from).unwrap();
            if skip(line) {
                return;
            }
            let id = target
                .push(
                    to,
                    line.text().to_owned(),
                    line.extra_indent(),
                    line.origin().cloned(),
                )
                .unwrap();
            for &child in line.children() {
                copy(source, child, target, id, skip);
            }
        }
        let mut target = Outline::new(source.format().clone());
        let root = target.root();
        for &top in source.get(source.root()).unwrap().children() {
            copy(source, top, &mut target, root, &skip);
        }
        target
    }

    /// A copy of `source` built as a caller saving its model would build it: every line keeps
    /// its origin, `edit` gives each line's new text and the top-level lines come in `top_order`.
    fn rebuilt(source: &Outline, edit: impl Fn(&str) -> String, top_order: &[usize]) -> Outline {
        fn copy(
            source: &Outline,
            from: LineId,
            target: &mut Outline,
            to: LineId,
            edit: &impl Fn(&str) -> String,
        ) {
            let line = source.get(from).unwrap();
            let id = target
                .push(
                    to,
                    edit(line.text()),
                    line.extra_indent(),
                    line.origin().cloned(),
                )
                .unwrap();
            for &child in line.children() {
                copy(source, child, target, id, edit);
            }
        }
        let mut target = Outline::new(source.format().clone());
        let tops = source.get(source.root()).unwrap().children();
        let root = target.root();
        for &index in top_order {
            copy(source, tops[index], &mut target, root, &edit);
        }
        target
    }

    // F-FILE-03, F-FILE-08
    #[test]
    fn unedited_text_is_written_back_byte_for_byte() {
        let cases = [
            "",
            "A",
            "A\n",
            "\n",
            "\n\n",
            "\r\n",
            "a\r",
            "A:\n\t- b\n\t\tnote\n\t- c\nB:\n",
            "A:\r\n\t- b\r\n",
            "A:\r\t- b\r",
            "a\r\nb\nc\rd\r\n",
            "a\u{C}b\u{2029}c\u{2B7F}d\u{2B7F}",
            "a\u{2028}b\n",
            "\u{FEFF}A:\n\t- b\n",
            "\u{FEFF}\u{FEFF}A",
            "A\n    B\n        C\n    D\n",
            "A\n    B\n  C\n",
            "A\n  B\n   C",
            "A\n  B\n\t   \tC\n",
            "A\n \tB\n    C\n",
            "A\n\t\t\tB\n\tC\n",
            "\t\tA\nB\n",
            "A\n\n\tB\n",
            "A\n\t\t\nB\n",
            "A\n\t \u{A0}\n\tB\n",
            "A\n\tB\n\t\n\t\t\n",
            "\t\n\t\t",
            "Inbox:  \n- task  \t\n",
            "  - Milk",
            "\t- Milk\n",
        ];
        for text in cases {
            check_round_trip(text);
        }
    }

    /// Each line as `depth extra_indent "text" kind [tags]`, one per line.
    fn line_tree(outline: &Outline) -> String {
        let lines: Vec<_> = shape(outline)
            .into_iter()
            .zip(outline.descendants(outline.root()))
            .map(|(shown, (id, _))| {
                let line = outline.get(id).unwrap();
                let tags: Vec<_> = line
                    .tags()
                    .iter()
                    .map(|tag| match &tag.value {
                        Some(value) => format!("@{}({value})", tag.name),
                        None => format!("@{}", tag.name),
                    })
                    .collect();
                format!("{shown} {:?} [{}]", line.kind(), tags.join(" "))
            })
            .collect();
        lines.join("\n")
    }

    // N-REL-02
    #[test]
    fn corpus_files_round_trip_and_parse_to_their_snapshot() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus");
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        paths.sort();
        assert!(!paths.is_empty(), "the corpus has files");
        for path in paths {
            let bytes = std::fs::read(&path).unwrap();
            let outline = parse_bytes(&bytes).unwrap();
            assert_eq!(serialize(&outline).as_bytes(), bytes, "{}", path.display());
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            insta::assert_snapshot!(name, line_tree(&outline));
        }
    }

    #[test]
    fn new_outlines_are_empty() {
        assert_eq!(serialize(&Outline::default()), "");
        let with_bom = Outline::new(FileFormat {
            bom: true,
            ..FileFormat::default()
        });
        assert_eq!(serialize(&with_bom), "\u{FEFF}");
    }

    /// An outline of new lines `A`, `B` under it, `C` under `B` and `D` two levels under `C`.
    fn new_lines(format: FileFormat) -> Outline {
        let mut outline = Outline::new(format);
        let root = outline.root();
        let a = push_new(&mut outline, root, "A:", 0);
        let b = push_new(&mut outline, a, "- B", 0);
        let c = push_new(&mut outline, b, "C", 0);
        push_new(&mut outline, c, "D", 1);
        outline
    }

    // F-FILE-08
    #[test]
    fn new_lines_use_the_remembered_format() {
        let cases = [
            (FileFormat::default(), "A:\n\t- B\n\t\tC\n\t\t\t\tD\n"),
            (
                FileFormat {
                    indent: IndentUnit::Spaces(4),
                    ..FileFormat::default()
                },
                "A:\n    - B\n        C\n                D\n",
            ),
            (
                FileFormat {
                    line_ending: LineEnding::CrLf,
                    trailing_newline: false,
                    ..FileFormat::default()
                },
                "A:\r\n\t- B\r\n\t\tC\r\n\t\t\t\tD",
            ),
            (
                FileFormat {
                    line_ending: LineEnding::Cr,
                    bom: true,
                    ..FileFormat::default()
                },
                "\u{FEFF}A:\r\t- B\r\t\tC\r\t\t\t\tD\r",
            ),
        ];
        for (format, expected) in cases {
            assert_eq!(
                serialize(&new_lines(format.clone())),
                expected,
                "{format:?}"
            );
        }
    }

    #[test]
    fn top_level_items_keep_their_extra_indent() {
        let mut outline = Outline::default();
        let root = outline.root();
        push_new(&mut outline, root, "A", 2);
        push_new(&mut outline, root, "", 1);
        assert_eq!(serialize(&outline), "\t\tA\n\t\n");
    }

    #[test]
    fn edited_text_is_written_with_its_indent_and_keeps_its_line_break() {
        let mut outline = Outline::new(FileFormat {
            indent: IndentUnit::Spaces(2),
            ..FileFormat::default()
        });
        let root = outline.root();
        let a = push_read(&mut outline, root, "A", 0, origin("A", 0, 0, "\r\n"));
        push_read(&mut outline, a, "- new", 0, origin("   - old", 3, 1, "\r"));
        assert_eq!(serialize(&outline), "A\r\n  - new\r");
    }

    #[test]
    fn lines_are_reindented_when_reusing_them_would_change_the_space_unit() {
        // "P:\n   Q:\n        a\n         b\n  s\n" with the 2-space line `s` deleted.
        let mut outline = Outline::new(FileFormat {
            indent: IndentUnit::Spaces(2),
            ..FileFormat::default()
        });
        let root = outline.root();
        let p = push_read(&mut outline, root, "P:", 0, origin("P:", 0, 0, "\n"));
        let q = push_read(&mut outline, p, "Q:", 0, origin("   Q:", 3, 1, "\r\n"));
        push_read(&mut outline, q, "a", 2, origin("        a", 8, 4, "\n"));
        push_read(&mut outline, q, "b", 2, origin("         b", 9, 4, "\n"));
        let text = serialize(&outline);
        assert_eq!(text, "P:\n  Q:\r\n        a\n        b\n");
        assert_eq!(shape(&parse(&text)), shape(&outline));
    }

    // F-FILE-03, F-FILE-08
    #[test]
    fn lines_go_back_to_tabs_when_no_space_run_would_give_their_depths() {
        let mut outline = Outline::new(FileFormat {
            indent: IndentUnit::Spaces(2),
            ..FileFormat::default()
        });
        let root = outline.root();
        let a = push_new(&mut outline, root, "A", 0);
        let b = push_new(&mut outline, a, "B", 1);
        push_new(&mut outline, b, "C", 0);
        let text = serialize(&outline);
        assert_eq!(text, "A\n\t\tB\n\t\t\tC\n");
        assert_eq!(shape(&parse(&text)), shape(&outline));
    }

    // F-FILE-03
    #[test]
    fn lines_as_read_are_reindented_when_their_spaces_would_give_another_depth() {
        let read = parse("A:\n\tB:\n      \n\tC:\n  \nD\n");
        let edit = |line: &Line| line.origin().is_some_and(|origin| origin.line == "  ");
        let outline = rebuilt_without(&read, edit);
        let text = serialize(&outline);
        assert_eq!(text, "A:\n\tB:\n\t\t\t\n\tC:\nD\n");
        assert_eq!(shape(&parse(&text)), shape(&outline));
    }

    #[test]
    fn an_outline_no_indentation_can_give_is_written_with_the_remembered_unit() {
        let mut outline = Outline::new(FileFormat {
            indent: IndentUnit::Spaces(2),
            ..FileFormat::default()
        });
        let root = outline.root();
        let a = push_new(&mut outline, root, "A", 0);
        push_new(&mut outline, a, "", 0);
        assert_eq!(serialize(&outline), "A\n  \n");
    }

    // F-FILE-03
    #[test]
    fn edits_change_only_their_lines_and_rebuilding_unedited_restores_the_bytes() {
        let text = "A:\r\n  - b\n\n  - c  \r\n";
        let outline = parse_bytes(text.as_bytes()).unwrap();
        let edit = |line: &str| match line {
            "- b" => "- B".to_owned(),
            "- c  " => "- c".to_owned(),
            other => other.to_owned(),
        };
        let edited = rebuilt(&outline, edit, &[0]);
        assert_eq!(serialize(&edited), "A:\r\n  - B\n\n  - c\r\n");
        let unedited = rebuilt(&outline, str::to_owned, &[0]);
        assert_eq!(serialize(&unedited), text);
    }

    // F-FILE-03, N-REL-02
    #[test]
    fn a_blank_line_after_a_carriage_return_is_not_read_back_as_part_of_it() {
        let cases: [(&[u8], &str); 3] = [
            (b"- a\r- b\n\n\n- c\n", "- a\r\r\r- c\n- b\n"),
            (b"- a\r- b\n\r\n- c\n", "- a\r\r\n- c\n- b\n"),
            (b"- a\r- b\n- c\n", "- a\r- c\n- b\n"),
        ];
        for (bytes, expected) in cases {
            let read = parse_bytes(bytes).unwrap();
            let count = read.get(read.root()).unwrap().children().len();
            let order: Vec<usize> = [0].into_iter().chain(2..count).chain([1]).collect();
            let outline = rebuilt(&read, str::to_owned, &order);
            let text = serialize(&outline);
            assert_eq!(text, expected);
            assert_eq!(shape(&parse(&text)), shape(&outline), "{text:?}");
        }
    }

    // F-FILE-03, file-format §3.1
    #[test]
    fn an_indent_jump_that_would_nest_a_line_under_an_earlier_sibling_is_dropped() {
        let mut outline = Outline::default();
        let root = outline.root();
        let p = push_read(&mut outline, root, "P:", 0, origin("P:", 0, 0, "\n"));
        push_read(&mut outline, p, "- b", 1, origin("\t\t- b", 2, 2, "\n"));
        push_read(&mut outline, p, "- c", 0, origin("\t- c", 1, 1, "\n"));
        push_read(&mut outline, p, "- a", 2, origin("\t\t\t- a", 3, 3, "\n"));
        push_new(&mut outline, root, "Q", 1);
        let text = serialize(&outline);
        assert_eq!(text, "P:\n\t\t- b\n\t- c\n\t- a\nQ\n");
        assert_eq!(
            shape(&parse(&text)),
            [
                r#"0 0 "P:""#,
                r#"1 1 "- b""#,
                r#"1 0 "- c""#,
                r#"1 0 "- a""#,
                r#"0 0 "Q""#
            ]
        );
    }

    #[test]
    fn a_line_whose_depth_changed_is_reindented() {
        let mut outline = Outline::default();
        let root = outline.root();
        push_read(&mut outline, root, "A", 0, origin("\tA", 1, 1, "\n"));
        assert_eq!(serialize(&outline), "A\n");
    }

    #[test]
    fn line_breaks_follow_the_lines_and_the_end_of_the_file() {
        let mut outline = Outline::new(FileFormat {
            line_ending: LineEnding::CrLf,
            trailing_newline: false,
            ..FileFormat::default()
        });
        let root = outline.root();
        push_read(
            &mut outline,
            root,
            "was last",
            0,
            origin("was last", 0, 0, ""),
        );
        push_read(
            &mut outline,
            root,
            "now last",
            0,
            origin("now last", 0, 0, "\u{C}"),
        );
        assert_eq!(serialize(&outline), "was last\r\nnow last");
    }

    // file-format §3.1, §6
    #[test]
    fn a_last_empty_line_ends_with_a_line_break_even_without_a_trailing_newline() {
        let mut outline = parse("P:");
        let root = outline.root();
        push_new(&mut outline, root, "", 0);
        assert_eq!(serialize(&outline), "P:\n\n");
        let mut outline = parse("");
        let root = outline.root();
        push_new(&mut outline, root, "", 0);
        assert_eq!(serialize(&outline), "\n");
        assert_eq!(
            serialize(&parse("P:\n  ")),
            "P:\n  ",
            "a blank line written as read"
        );
    }

    #[test]
    fn a_blank_line_is_kept_while_the_next_line_still_gives_it_its_depth() {
        let mut outline = Outline::default();
        let root = outline.root();
        let a = push_new(&mut outline, root, "A", 0);
        push_read(&mut outline, a, "", 0, origin("", 0, 0, "\n"));
        push_new(&mut outline, a, "B", 0);
        assert_eq!(serialize(&outline), "A\n\n\tB\n");
    }

    #[test]
    fn a_blank_line_is_reindented_when_the_next_line_no_longer_gives_it_its_depth() {
        let mut outline = Outline::default();
        let root = outline.root();
        let a = push_new(&mut outline, root, "A", 0);
        push_read(&mut outline, a, "", 0, origin("", 0, 0, "\n"));
        push_new(&mut outline, root, "C", 0);
        assert_eq!(serialize(&outline), "A\n\t\nC\n");
    }

    #[test]
    fn a_blank_line_moved_above_a_shallower_line_loses_its_indentation() {
        let mut outline = Outline::default();
        let root = outline.root();
        push_read(&mut outline, root, "", 0, origin("\t", 1, 1, "\n"));
        push_new(&mut outline, root, "B", 0);
        assert_eq!(serialize(&outline), "\nB\n");
    }

    #[test]
    fn whitespace_only_lines_do_not_give_a_blank_line_its_depth() {
        let mut outline = Outline::default();
        let root = outline.root();
        let a = push_new(&mut outline, root, "A", 0);
        push_read(&mut outline, a, "", 0, origin("", 0, 0, "\n"));
        push_new(&mut outline, a, " ", 0);
        assert_eq!(serialize(&outline), "A\n\t\n\t \n");
    }

    /// One generated line: how many levels to go up first, the extra indent and the text.
    type LineSpec = (usize, usize, String);

    /// Builds an outline that is canonical (file-format.md §3.1): sibling extra indents never
    /// increase (placement), empty items have no children and trailing empty items are top-level.
    fn canonical(specs: &[LineSpec], format: FileFormat) -> Outline {
        let trailing_empty = specs
            .iter()
            .rposition(|(_, _, text)| !text.is_empty())
            .map_or(0, |last_text| last_text + 1);
        let mut outline = Outline::new(format);
        let mut path: Vec<(LineId, bool, Option<usize>)> = vec![(outline.root(), false, None)];
        for (index, (up, extra, text)) in specs.iter().enumerate() {
            let up = if index >= trailing_empty {
                path.len()
            } else {
                *up
            };
            path.truncate(path.len().saturating_sub(up).max(1));
            while path.last().is_some_and(|&(_, empty, _)| empty) {
                path.pop();
            }
            let (parent, _, last_extra) = path.last_mut().unwrap();
            let extra = if index >= trailing_empty {
                0
            } else {
                (*extra).min(last_extra.unwrap_or(*extra))
            };
            *last_extra = Some(extra);
            let parent = *parent;
            let id = push_new(&mut outline, parent, text, extra);
            path.push((id, text.is_empty(), None));
        }
        outline
    }

    fn line_specs() -> impl Strategy<Value = Vec<LineSpec>> {
        let text = "([a-z:@()\\\\\\-é][a-z:@()\\\\\\- \t\u{2028}é]{0,8})?";
        prop::collection::vec((0..4usize, 0..3usize, text), 0..12)
    }

    fn formats() -> impl Strategy<Value = FileFormat> {
        let indent = prop_oneof![
            Just(IndentUnit::Tab),
            (1..5usize).prop_map(IndentUnit::Spaces)
        ];
        let line_ending = prop_oneof![
            Just(LineEnding::Lf),
            Just(LineEnding::CrLf),
            Just(LineEnding::Cr)
        ];
        (indent, line_ending, any::<bool>(), any::<bool>()).prop_map(
            |(indent, line_ending, bom, trailing_newline)| FileFormat {
                indent,
                line_ending,
                bom,
                trailing_newline,
            },
        )
    }

    fn file_text() -> impl Strategy<Value = String> {
        let piece = prop_oneof![
            Just("\n"),
            Just("\r\n"),
            Just("\r"),
            Just("\u{C}"),
            Just("\u{2029}"),
            Just("\u{2B7F}"),
            Just("\u{2028}"),
            Just("\u{FEFF}"),
            Just("\t"),
            Just(" "),
            Just("  "),
            Just("\u{A0}"),
            Just("- "),
            Just(":"),
            Just(" @a(1)"),
            Just("x"),
            Just("é"),
        ];
        prop_oneof![
            prop::collection::vec(piece, 0..40).prop_map(|pieces| pieces.concat()),
            any::<String>(),
        ]
    }

    proptest! {
        // F-FILE-03, N-REL-02
        #[test]
        fn any_unedited_text_round_trips(text in file_text()) {
            let outline = parse_bytes(text.as_bytes()).unwrap();
            prop_assert_eq!(serialize(&outline), text);
        }

        // F-FILE-03, N-REL-02
        #[test]
        fn canonical_outlines_parse_back_to_the_same_outline(
            specs in line_specs(),
            format in formats(),
        ) {
            let outline = canonical(&specs, format);
            let text = serialize(&outline);
            let reparsed = parse_bytes(text.as_bytes()).unwrap();
            prop_assert_eq!(shape(&reparsed), shape(&outline), "{:?}", text);
            prop_assert_eq!(serialize(&reparsed), text);
        }
    }

    #[test]
    fn parse_without_bom_keeps_a_leading_bom_in_the_text() {
        let outline = parse("\u{FEFF}A");
        assert_eq!(serialize(&outline), "\u{FEFF}A");
        assert!(!outline.format().bom);
    }
}
