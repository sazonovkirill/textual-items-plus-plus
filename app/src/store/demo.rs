//! The demo document that a start without a path opens, kept in memory and never saved
//! (FR-STORE-01, D-29): `demo.taskpaper`, with its date placeholders resolved to a given day.
//! Ported from taskpaper-terminal's `app/src/app/demo.rs`.

use jiff::{Span, civil::Date};

use super::taskpaper::{parse, read_document};
use crate::domain::Document;

/// The demo outline as written, with placeholders.
const DEMO: &str = include_str!("demo.taskpaper");

/// The demo document, with every date placeholder resolved relative to `today`.
#[must_use]
pub fn document(today: Date) -> Document {
    read_document(parse(&resolve(DEMO, today))).0
}

/// `text` with each `{today}`, `{today+N}` and `{today-N}` replaced by that day, `N` days from
/// `today`, as `YYYY-MM-DD`, so due and overdue dates stay meaningful whenever the demo is made.
/// Braces holding anything else are kept as written.
fn resolve(text: &str, today: Date) -> String {
    let mut parts = text.split('{');
    let mut out = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        if let Some((day, after)) = part
            .split_once('}')
            .and_then(|(inside, after)| Some((day(inside, today)?, after)))
        {
            out.push_str(&day.to_string());
            out.push_str(after);
        } else {
            out.push('{');
            out.push_str(part);
        }
    }
    out
}

/// The day a placeholder's inside, such as `today-2`, stands for.
fn day(inside: &str, today: Date) -> Option<Date> {
    let offset = inside.strip_prefix("today")?;
    let days = if offset.is_empty() {
        0
    } else if offset.starts_with(['+', '-']) {
        offset.parse::<i64>().ok()?
    } else {
        return None;
    };
    today.checked_add(Span::new().try_days(days).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::domain::{Completion, Item, ItemId, ItemKind, Node};
    use crate::store::taskpaper::{serialize, write_document};

    const TODAY: Date = date(2026, 10, 5);

    #[track_caller]
    fn check(text: &str, expected: &str) {
        assert_eq!(resolve(text, TODAY), expected, "{text}");
    }

    #[test]
    fn resolves_today_and_days_before_and_after_it() {
        check("@due({today})", "@due(2026-10-05)");
        check("@due({today+1})", "@due(2026-10-06)");
        check("@due({today-5})", "@due(2026-09-30)");
        check("{today+27} and {today-1}.", "2026-11-01 and 2026-10-04.");
        check("- 日本 {today+0} 語", "- 日本 2026-10-05 語");
    }

    #[test]
    fn keeps_braces_that_hold_no_placeholder() {
        for text in [
            "",
            "no braces",
            "{",
            "}",
            "{}",
            "{today",
            "today}",
            "{tomorrow}",
            "{today1}",
            "{today+}",
            "{today+x}",
            "{today+99999999}",
            "{today*2}",
            "{ today}",
            "{{today}",
        ] {
            let expected = text.replace("{{today}", "{2026-10-05");
            check(text, &expected);
        }
    }

    /// Every node of `document` below `id`, depth first.
    fn below(document: &Document, id: ItemId) -> Vec<ItemId> {
        document
            .children(id)
            .iter()
            .flat_map(|&child| std::iter::once(child).chain(below(document, child)))
            .collect()
    }

    /// Every item of `document`, which holds nothing but items.
    fn items(document: &Document) -> Vec<&Item> {
        let ids: Vec<ItemId> = std::iter::once(document.inbox())
            .chain(document.top_level().iter().copied())
            .flat_map(|id| std::iter::once(id).chain(below(document, id)))
            .collect();
        let items: Vec<&Item> = ids
            .iter()
            .filter_map(|&id| document.node(id)?.item())
            .collect();
        assert_eq!(
            items.len(),
            ids.len(),
            "the demo has content it does not show"
        );
        items
    }

    // FR-STORE-01, D-29
    #[test]
    fn demo_resolves_every_placeholder() {
        let text = resolve(DEMO, TODAY);
        assert!(!text.contains('{'), "an unresolved placeholder");
        assert!(text.contains("@done(2026-10-04)"));
    }

    // FR-STORE-01, D-29
    #[test]
    fn demo_shows_every_kind_of_item_and_what_items_hold() {
        let document = document(TODAY);
        let items = items(&document);
        for kind in [
            ItemKind::Area,
            ItemKind::Project,
            ItemKind::Section,
            ItemKind::Task,
            ItemKind::ChecklistItem,
        ] {
            assert!(
                items.iter().any(|item| item.kind == kind),
                "demo lacks a {kind:?}"
            );
        }
        let has = |what: &str, test: &dyn Fn(&Item) -> bool| {
            assert!(items.iter().any(|item| test(item)), "demo lacks {what}");
        };
        has("a start date", &|item| item.start.is_some());
        has("a due date to come", &|item| {
            item.due.is_some_and(|due| due > TODAY)
        });
        has("an overdue task", &|item| {
            item.due.is_some_and(|due| due < TODAY)
        });
        has("a tag", &|item| !item.tags.is_empty());
        has("a description", &|item| !item.description.is_empty());
        has("a note", &|item| !item.note.is_empty());
        has("comments", &|item| !item.comments.is_empty());
        has("a completed task", &|item| {
            item.completed == Some(Completion::On(date(2026, 10, 4)))
        });
        assert!(
            document
                .top_level()
                .iter()
                .any(|&id| matches!(document.node(id), Some(Node::Item(item)) if item.kind == ItemKind::Project)),
            "demo lacks a project outside any area"
        );
        assert!(
            document
                .children(document.inbox())
                .iter()
                .any(|&id| matches!(
                    document.node(id),
                    Some(Node::Item(item)) if item.start > Some(TODAY)
                )),
            "demo lacks a scheduled Inbox task"
        );
    }

    // FR-STORE-01, D-29
    #[test]
    fn demo_is_written_back_as_resolved() {
        let text = resolve(DEMO, TODAY);
        let (document, source) = read_document(parse(&text));
        assert_eq!(serialize(&write_document(&document, &source)), text);
    }
}
