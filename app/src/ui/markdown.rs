//! A note's Markdown as the attachments panel shows it: its lines read as headings, list items,
//! block quotes, fenced code and text, and their inline `**bold**`, `*emphasis*`, `` `code` ``
//! and links, the syntax hidden while the text stays as typed. Ported from taskpaper-terminal's
//! `document/markdown.rs` and `document/inline.rs` at fa0c6b5, its note syntax only
//! ([attachments.md §4](../../../specs/reference/taskpaper-terminal/specs/attachments.md#4-notes-fr-att-04),
//! FR-ATT-03).

use std::ops::Range;

/// One line of a note as Markdown shows it, its syntax taken off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NoteLine<'a> {
    /// A heading of this level, 1 to 6, and its title.
    Heading(u8, &'a str),
    /// A list item `depth` levels in, every two leading spaces one level, with its marker as
    /// typed (`-`, `*`, `+`, `1.` or `1)`) and its text.
    Item {
        /// How many levels the item is nested.
        depth: usize,
        /// Its marker.
        marker: &'a str,
        /// Its text.
        text: &'a str,
    },
    /// A line of a block quote.
    Quote(&'a str),
    /// A line inside a fenced code block, as typed.
    Code(&'a str),
    /// A line of text as typed, its leading whitespace kept so its indentation shows; an empty
    /// one separates paragraphs.
    Text(&'a str),
}

/// The lines of the note `note` as Markdown, the lines of code fences left out: a fence opens
/// with three backticks or tildes and closes with the same three, or at the end of the note.
pub(super) fn note_lines(note: &str) -> Vec<NoteLine<'_>> {
    let mut fence: Option<&str> = None;
    let mut shown = Vec::new();
    for line in note.lines() {
        let fenced = ["```", "~~~"]
            .into_iter()
            .find(|mark| line.trim_start().starts_with(mark));
        match (fence, fenced) {
            (None, Some(mark)) => fence = Some(mark),
            (Some(open), Some(mark)) if open == mark => fence = None,
            (Some(_), _) => shown.push(NoteLine::Code(line)),
            (None, None) => shown.push(block_line(line)),
        }
    }
    shown
}

/// The line `line` outside code.
fn block_line(line: &str) -> NoteLine<'_> {
    let text = line.trim_start();
    let indent = line.len() - text.len();
    heading(text)
        .or_else(|| {
            let quote = text.strip_prefix('>')?;
            Some(NoteLine::Quote(quote.strip_prefix(' ').unwrap_or(quote)))
        })
        .or_else(|| {
            let (marker, text) = text.split_once(' ')?;
            let number = marker.strip_suffix(['.', ')']).is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            });
            (matches!(marker, "-" | "*" | "+") || number).then_some(NoteLine::Item {
                depth: indent / 2,
                marker,
                text: text.trim_start(),
            })
        })
        .unwrap_or(NoteLine::Text(line))
}

/// The heading `text` is: one to six `#`, a space and a title that is not all whitespace, given
/// without the space around it.
fn heading(text: &str) -> Option<NoteLine<'_>> {
    let level = text.bytes().take_while(|&byte| byte == b'#').count();
    let title = text.get(level..)?.strip_prefix(' ')?.trim();
    let level = u8::try_from(level)
        .ok()
        .filter(|level| (1..=6).contains(level))?;
    (!title.is_empty()).then_some(NoteLine::Heading(level, title))
}

/// How a run of text is formatted; several apply at once, as for a link in bold text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "formats that apply together, as bold over a link, not states of one machine"
)]
pub(super) struct Format {
    /// Inside `**…**`.
    pub(super) bold: bool,
    /// Inside `` `…` ``.
    pub(super) code: bool,
    /// Inside `*…*` or `_…_`.
    pub(super) italic: bool,
    /// Part of a URL or the title of a link.
    pub(super) link: bool,
}

/// A piece of the text as shown, with the delimiters hidden, all of it formatted one way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Run<'a> {
    /// The text shown.
    pub(super) text: &'a str,
    /// How it is formatted.
    pub(super) format: Format,
}

/// What a formatted part of a text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A `**…**` span.
    Bold,
    /// A `` `…` `` span.
    Code,
    /// A `*…*` or `_…_` span.
    Emphasis,
    /// A bare URL or a `<url>` link, which shows its address.
    Url,
    /// A `[title](url)` link, which shows its title.
    Link,
}

/// One formatted part of a text: the bytes with its delimiters (outer) and the bytes shown
/// (inner); a bare URL's are the same.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Element {
    /// What it is.
    kind: Kind,
    /// The bytes with the delimiters.
    outer: Range<usize>,
    /// The bytes between the delimiters.
    inner: Range<usize>,
}

/// What a scan may find besides code spans: bold except inside bold, emphasis except inside
/// emphasis, and links except inside a link's title.
#[derive(Debug, Clone, Copy)]
struct Scope {
    bold: bool,
    emphasis: bool,
    links: bool,
}

/// `text` as shown: runs in order with the delimiters of bold, emphasis, code and links left out.
/// A delimiter without a partner is shown as typed. Code shows its content as typed; bold may
/// hold code and links but not more bold, and a link's title may hold bold and code but no link.
pub(super) fn runs(text: &str) -> Vec<Run<'_>> {
    let mut elements = Vec::new();
    let scope = Scope {
        bold: true,
        emphasis: true,
        links: true,
    };
    scan(text, 0..text.len(), scope, &mut elements);
    let mut bounds: Vec<usize> = elements
        .iter()
        .flat_map(|e| [e.outer.start, e.inner.start, e.inner.end, e.outer.end])
        .chain([0, text.len()])
        .collect();
    bounds.sort_unstable();
    bounds.dedup();
    let mut runs = Vec::new();
    for &[start, end] in bounds.array_windows() {
        let inside = |range: &Range<usize>| range.start <= start && end <= range.end;
        if elements
            .iter()
            .any(|e| inside(&e.outer) && !inside(&e.inner))
        {
            continue;
        }
        let mut format = Format::default();
        for element in elements.iter().filter(|e| inside(&e.inner)) {
            match element.kind {
                Kind::Bold => format.bold = true,
                Kind::Code => format.code = true,
                Kind::Emphasis => format.italic = true,
                Kind::Url | Kind::Link => format.link = true,
            }
        }
        runs.push(Run {
            text: text.get(start..end).unwrap_or_default(),
            format,
        });
    }
    runs
}

/// Adds the elements of the bytes `range` of `text` that `scope` allows to `found`.
fn scan(text: &str, range: Range<usize>, scope: Scope, found: &mut Vec<Element>) {
    let mut at = range.start;
    // Every element starts with one of these bytes, so the bytes before the next one start none.
    while let Some(skipped) = text
        .get(at..range.end)
        .and_then(|rest| rest.bytes().position(|byte| b"`*_[<h".contains(&byte)))
    {
        at += skipped;
        let end = range.end;
        let element = code_at(text, at, end)
            .or_else(|| scope.bold.then(|| bold_at(text, at, end)).flatten())
            .or_else(|| scope.emphasis.then(|| emphasis_at(text, at, end)).flatten())
            .or_else(|| scope.links.then(|| link_at(text, at, end)).flatten())
            .or_else(|| scope.links.then(|| autolink_at(text, at, end)).flatten())
            .or_else(|| scope.links.then(|| url_at(text, at, end)).flatten());
        let Some(element) = element else {
            // The byte is ASCII, so the next one starts a character.
            at += 1;
            continue;
        };
        at = element.outer.end;
        let inner = match element.kind {
            Kind::Bold => Some(Scope {
                bold: false,
                ..scope
            }),
            Kind::Emphasis => Some(Scope {
                emphasis: false,
                ..scope
            }),
            Kind::Link => Some(Scope {
                links: false,
                ..scope
            }),
            Kind::Code | Kind::Url => None,
        }
        .map(|inner| (element.inner.clone(), inner));
        found.push(element);
        if let Some((range, inner)) = inner {
            scan(text, range, inner, found);
        }
    }
}

/// The Markdown link starting at byte `at` and ending by byte `end`: `[`, a title of at least one
/// character without brackets outside code spans, `](`, an address of at least one character
/// without whitespace whose parentheses pair up, and `)`.
fn link_at(text: &str, at: usize, end: usize) -> Option<Element> {
    text.get(at..end)?.strip_prefix('[')?;
    let title = at + 1;
    let mut close = title;
    loop {
        if let Some(code) = code_at(text, close, end) {
            close = code.outer.end;
            continue;
        }
        match text.get(close..end)?.chars().next()? {
            ']' => break,
            '[' => return None,
            other => close += other.len_utf8(),
        }
    }
    let address = close + 2;
    if close == title || !text.get(close + 1..end)?.starts_with('(') {
        return None;
    }
    let mut depth = 0_usize;
    let mut stop = address;
    for c in text.get(address..end)?.chars() {
        match c {
            ')' if depth == 0 => break,
            ')' => depth -= 1,
            '(' => depth += 1,
            c if c.is_whitespace() => return None,
            _ => {}
        }
        stop += c.len_utf8();
    }
    (stop > address && text.get(stop..end)?.starts_with(')')).then(|| Element {
        kind: Kind::Link,
        outer: at..stop + 1,
        inner: title..close,
    })
}

/// The `<url>` link starting at byte `at` and ending by byte `end`: `<`, an address with a
/// scheme, something after its `:` and no whitespace, and `>`.
fn autolink_at(text: &str, at: usize, end: usize) -> Option<Element> {
    let rest = text.get(at..end)?.strip_prefix('<')?;
    let address = rest.get(..rest.find('>')?)?;
    let valid = has_scheme(address)
        && address
            .split_once(':')
            .is_some_and(|(_, after)| !after.is_empty())
        && !address.contains(|c: char| c.is_whitespace() || c == '<');
    valid.then(|| Element {
        kind: Kind::Url,
        outer: at..at + address.len() + 2,
        inner: at + 1..at + 1 + address.len(),
    })
}

/// Whether `address` starts with a URL scheme, such as `https:` or `mailto:`.
fn has_scheme(address: &str) -> bool {
    address.split_once(':').is_some_and(|(scheme, _)| {
        let mut chars = scheme.chars();
        chars
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

/// The code span starting at byte `at` and ending by byte `end`: a backtick, at least one
/// character and the next backtick.
fn code_at(text: &str, at: usize, end: usize) -> Option<Element> {
    let rest = text.get(at..end)?.strip_prefix('`')?;
    let close = at + 1 + rest.find('`')?;
    (close > at + 1).then(|| Element {
        kind: Kind::Code,
        outer: at..close + 1,
        inner: at + 1..close,
    })
}

/// The bold span starting at byte `at` and ending by byte `end`: `**`, text that neither starts
/// nor ends with whitespace, and the next `**` outside a code span after it.
fn bold_at(text: &str, at: usize, end: usize) -> Option<Element> {
    let content = at + 2;
    let first = text.get(at..end)?.strip_prefix("**")?.chars().next()?;
    if first.is_whitespace() {
        return None;
    }
    let mut close = content + first.len_utf8();
    while let Some(rest) = text.get(close..end).filter(|rest| !rest.is_empty()) {
        if let Some(code) = code_at(text, close, end) {
            close = code.outer.end;
            continue;
        }
        let before = text.get(..close)?.chars().next_back()?;
        if rest.starts_with("**") && !before.is_whitespace() {
            return Some(Element {
                kind: Kind::Bold,
                outer: at..close + 2,
                inner: content..close,
            });
        }
        close += rest.chars().next()?.len_utf8();
    }
    None
}

/// The emphasis span starting at byte `at` and ending by byte `end`: one `*` or `_` that is not
/// part of a run of them, text that neither starts nor ends with whitespace, and the next such
/// delimiter outside a code span after it. A `_` touches no letter or digit outside the span, so
/// that `snake_case` stays as typed.
fn emphasis_at(text: &str, at: usize, end: usize) -> Option<Element> {
    let delimiter = text.get(at..end)?.chars().next()?;
    if !matches!(delimiter, '*' | '_') {
        return None;
    }
    // Whether `c`, next to a delimiter on the side away from the span, makes it no delimiter.
    let joins = |c: Option<char>| {
        c == Some(delimiter) || delimiter == '_' && c.is_some_and(char::is_alphanumeric)
    };
    let before = |byte: usize| text.get(..byte)?.chars().next_back();
    let after = |byte: usize| text.get(byte + 1..)?.chars().next();
    if joins(before(at)) {
        return None;
    }
    let first = after(at).filter(|&first| !first.is_whitespace() && first != delimiter)?;
    let content = at + 1;
    let mut close = content + first.len_utf8();
    while let Some(rest) = text.get(close..end).filter(|rest| !rest.is_empty()) {
        if let Some(code) = code_at(text, close, end) {
            close = code.outer.end;
            continue;
        }
        if rest.starts_with(delimiter)
            && before(close).is_some_and(|last| !last.is_whitespace() && last != delimiter)
            && !joins(after(close))
        {
            return Some(Element {
                kind: Kind::Emphasis,
                outer: at..close + 1,
                inner: content..close,
            });
        }
        close += rest.chars().next()?.len_utf8();
    }
    None
}

/// The bare URL starting at byte `at` and ending by byte `end`: `http://` or `https://` not
/// right after a letter or digit, then everything up to whitespace or `` ` ``, `<`, `>` or `"`,
/// without trailing punctuation or a `)` that closes nothing in it.
fn url_at(text: &str, at: usize, end: usize) -> Option<Element> {
    let rest = text.get(at..end)?;
    let scheme = ["https://", "http://"]
        .into_iter()
        .find(|scheme| rest.starts_with(scheme))?;
    if text
        .get(..at)?
        .chars()
        .next_back()
        .is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let mut url = rest
        .split(|c: char| c.is_whitespace() || matches!(c, '`' | '<' | '>' | '"'))
        .next()?;
    loop {
        let trimmed = url.trim_end_matches(['.', ',', ':', ';', '!', '?', '\'', '*']);
        let unmatched =
            trimmed.ends_with(')') && trimmed.matches(')').count() > trimmed.matches('(').count();
        url = if unmatched {
            trimmed.strip_suffix(')')?
        } else {
            trimmed
        };
        if !unmatched {
            break;
        }
    }
    (url.len() > scheme.len()).then(|| Element {
        kind: Kind::Url,
        outer: at..at + url.len(),
        inner: at..at + url.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // FR-ATT-03
    #[test]
    fn headings_items_quotes_and_text_lose_their_syntax() {
        assert_eq!(
            note_lines(
                "# Title\n###### Six \n####### Seven\n#No\n# \n  text  \n\n> said\n>bare\n- a\n  * b\n    + c\n1. one\n12) twelve\n-no\n1.no\nx. no"
            ),
            [
                NoteLine::Heading(1, "Title"),
                NoteLine::Heading(6, "Six"),
                NoteLine::Text("####### Seven"),
                NoteLine::Text("#No"),
                NoteLine::Text("# "),
                NoteLine::Text("  text  "),
                NoteLine::Text(""),
                NoteLine::Quote("said"),
                NoteLine::Quote("bare"),
                NoteLine::Item {
                    depth: 0,
                    marker: "-",
                    text: "a"
                },
                NoteLine::Item {
                    depth: 1,
                    marker: "*",
                    text: "b"
                },
                NoteLine::Item {
                    depth: 2,
                    marker: "+",
                    text: "c"
                },
                NoteLine::Item {
                    depth: 0,
                    marker: "1.",
                    text: "one"
                },
                NoteLine::Item {
                    depth: 0,
                    marker: "12)",
                    text: "twelve"
                },
                NoteLine::Text("-no"),
                NoteLine::Text("1.no"),
                NoteLine::Text("x. no"),
            ]
        );
    }

    // FR-ATT-03
    #[test]
    fn indented_text_keeps_its_indentation() {
        assert_eq!(
            note_lines("    code *x*\n\tfn f()\n  #No"),
            [
                NoteLine::Text("    code *x*"),
                NoteLine::Text("\tfn f()"),
                NoteLine::Text("  #No"),
            ]
        );
    }

    // FR-ATT-03
    #[test]
    fn fenced_code_shows_as_typed_without_its_fences() {
        assert_eq!(
            note_lines("a\n```rust\n  # not a heading\n~~~\n```\n- b\n  ~~~\n```\nopen to the end"),
            [
                NoteLine::Text("a"),
                NoteLine::Code("  # not a heading"),
                NoteLine::Code("~~~"),
                NoteLine::Item {
                    depth: 0,
                    marker: "-",
                    text: "b"
                },
                NoteLine::Code("```"),
                NoteLine::Code("open to the end"),
            ]
        );
    }

    const PLAIN: Format = Format {
        bold: false,
        code: false,
        italic: false,
        link: false,
    };
    const BOLD: Format = Format {
        bold: true,
        ..PLAIN
    };
    const CODE: Format = Format {
        code: true,
        ..PLAIN
    };
    const ITALIC: Format = Format {
        italic: true,
        ..PLAIN
    };
    const LINK: Format = Format {
        link: true,
        ..PLAIN
    };

    #[track_caller]
    fn check(text: &str, expected: &[(&str, Format)]) {
        let runs: Vec<(&str, Format)> = runs(text)
            .into_iter()
            .map(|run| (run.text, run.format))
            .collect();
        assert_eq!(runs, expected, "{text:?}");
    }

    // FR-ATT-03
    #[test]
    fn bold_and_code_hide_their_delimiters() {
        check(
            "a **b** c `d` e",
            &[
                ("a ", PLAIN),
                ("b", BOLD),
                (" c ", PLAIN),
                ("d", CODE),
                (" e", PLAIN),
            ],
        );
        check("`a``b`c", &[("a", CODE), ("b", CODE), ("c", PLAIN)]);
        check("**日本**語", &[("日本", BOLD), ("語", PLAIN)]);
        check("plain", &[("plain", PLAIN)]);
        check("", &[]);
    }

    // FR-ATT-03
    #[test]
    fn emphasis_hides_its_delimiters_but_not_in_words() {
        check(
            "a *b* _c d_ e",
            &[
                ("a ", PLAIN),
                ("b", ITALIC),
                (" ", PLAIN),
                ("c d", ITALIC),
                (" e", PLAIN),
            ],
        );
        check(
            "*a **b** `*c*`*",
            &[
                ("a ", ITALIC),
                (
                    "b",
                    Format {
                        bold: true,
                        ..ITALIC
                    },
                ),
                (" ", ITALIC),
                (
                    "*c*",
                    Format {
                        code: true,
                        ..ITALIC
                    },
                ),
            ],
        );
        check(
            "*日本* _é_",
            &[("日本", ITALIC), (" ", PLAIN), ("é", ITALIC)],
        );
        check("_*a*_", &[("*a*", ITALIC)]);
        check("x*a*", &[("x", PLAIN), ("a", ITALIC)]);
        for text in [
            "snake_case_name",
            "a_b_",
            "_a_b",
            "* a*",
            "*a *",
            "*a",
            "**",
            "*a**",
            "a * b * c",
            "a ** b",
            "**a",
            "a`b",
            "``",
            "****",
            "** a**",
            "2 ** 3 ** 4",
        ] {
            check(text, &[(text, PLAIN)]);
        }
        check("**a **b**", &[("a **b", BOLD)]);
    }

    // FR-ATT-03
    #[test]
    fn code_shows_its_content_as_typed_and_binds_before_bold() {
        check("`**a**`", &[("**a**", CODE)]);
        check("`https://a.b`", &[("https://a.b", CODE)]);
        check(
            "**a `b**` c**",
            &[
                ("a ", BOLD),
                (
                    "b**",
                    Format {
                        bold: true,
                        code: true,
                        ..PLAIN
                    },
                ),
                (" c", BOLD),
            ],
        );
        check("**a `b** c", &[("a `b", BOLD), (" c", PLAIN)]);
    }

    // FR-ATT-03
    #[test]
    fn bare_urls_are_links_without_trailing_punctuation() {
        check(
            "See https://a.b/c?d=1.",
            &[("See ", PLAIN), ("https://a.b/c?d=1", LINK), (".", PLAIN)],
        );
        check(
            "(http://a.b/x_(y)) z",
            &[("(", PLAIN), ("http://a.b/x_(y)", LINK), (") z", PLAIN)],
        );
        check(
            "**https://a.b**",
            &[(
                "https://a.b",
                Format {
                    bold: true,
                    link: true,
                    ..PLAIN
                },
            )],
        );
        for text in [
            "https://",
            "https://.",
            "xhttps://a.b",
            "ftp://a.b",
            "http:/a",
        ] {
            check(text, &[(text, PLAIN)]);
        }
    }

    // FR-ATT-03
    #[test]
    fn markdown_links_show_their_title_or_address() {
        check(
            "See [the site](https://a.b/c) or <https://d.e>.",
            &[
                ("See ", PLAIN),
                ("the site", LINK),
                (" or ", PLAIN),
                ("https://d.e", LINK),
                (".", PLAIN),
            ],
        );
        check("<mailto:x@y.z>", &[("mailto:x@y.z", LINK)]);
        check("<a<b:c>", &[("<a", PLAIN), ("b:c", LINK)]);
        check("[w](https://x.y/a_(b))", &[("w", LINK)]);
        check("[notes](notes.txt)", &[("notes", LINK)]);
        check("[https://a.b](https://a.b)", &[("https://a.b", LINK)]);
        check("[日本](https://a.b)語", &[("日本", LINK), ("語", PLAIN)]);
        check("[a[b](https://c.d)", &[("[a", PLAIN), ("b", LINK)]);
        let both = |format: Format| Format {
            link: true,
            ..format
        };
        check(
            "[**a** `]`](https://b.c)",
            &[("a", both(BOLD)), (" ", LINK), ("]", both(CODE))],
        );
        check("**[a](https://b.c)**", &[("a", both(BOLD))]);
        check("**<https://b.c>**", &[("https://b.c", both(BOLD))]);
        for text in [
            "[](x.y)",
            "[a]()",
            "[a] (x.y)",
            "[a](b c)",
            "[a](x.y",
            "[a](x(y)",
            "[a",
            "<a.b>",
            "<https:>",
            "<x:a b>",
            "<>",
            "<-x:y>",
            "<1x:y>",
        ] {
            check(text, &[(text, PLAIN)]);
        }
        check(
            "[a](https://u.v w)",
            &[("[a](", PLAIN), ("https://u.v", LINK), (" w)", PLAIN)],
        );
    }

    proptest::proptest! {
        /// Any text shows as runs of itself in order.
        #[test]
        fn runs_are_pieces_of_any_text(text in "[a*_` hps:/.()#\\[\\]<>日]{0,24}") {
            let mut rest = text.as_str();
            for run in runs(&text) {
                let at = rest.find(run.text);
                proptest::prop_assert!(at.is_some(), "{:?} not in {:?}", run.text, rest);
                rest = rest.get(at.unwrap_or_default() + run.text.len()..).unwrap_or_default();
            }
        }
    }
}
