//! Comments: dated one-line remarks on an item, each starting with its local 24-hour timestamp,
//! `[YYYY-MM-DD HH:MM] text`, as taskpaper-terminal writes them
//! ([attachments.md §5](../../../specs/reference/taskpaper-terminal/specs/attachments.md#5-comments-fr-att-05),
//! FR-ATT-04).

use jiff::civil::DateTime;

/// The timestamp a comment made at `now` starts with, and the space after it.
#[must_use]
pub fn stamp(now: DateTime) -> String {
    format!("{} ", now.strftime("[%Y-%m-%d %H:%M]"))
}

/// `line`, a comment as stored, split into its timestamp with the space after it, and its text.
/// A line without a valid timestamp, typed by hand or by another app, is all text.
#[must_use]
pub fn split(line: &str) -> (&str, &str) {
    if time(line).is_some() {
        line.split_at(19)
    } else {
        ("", line)
    }
}

/// The local date and time the timestamp `line`, a comment as stored, starts with gives, unless
/// it starts with none.
#[must_use]
pub fn time(line: &str) -> Option<DateTime> {
    let stamp = line.get(..19)?.strip_prefix('[')?.strip_suffix("] ")?;
    DateTime::strptime("%Y-%m-%d %H:%M", stamp).ok()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    // FR-ATT-04
    #[test]
    fn a_comment_is_stamped_with_its_local_date_and_time() {
        assert_eq!(
            stamp(date(2026, 10, 4).at(7, 5, 59, 0)),
            "[2026-10-04 07:05] "
        );
    }

    // FR-ATT-04
    #[test]
    fn a_comment_splits_into_its_stamp_giving_its_time_and_its_text_unless_it_has_none() {
        assert_eq!(
            split("[2026-10-04 07:05] Called them"),
            ("[2026-10-04 07:05] ", "Called them")
        );
        assert_eq!(split("[2026-10-04 07:05] "), ("[2026-10-04 07:05] ", ""));
        assert_eq!(
            time("[2026-10-04 07:05] Called them"),
            Some(date(2026, 10, 4).at(7, 5, 0, 0))
        );
        for line in [
            "plain",
            "",
            "[2026-10-04 07:05]x",
            "[2026-13-04 07:05] month",
            "[2026-10-04 7:05] hour",
            "[2026-10-04 07:05",
            "[2026-10-04 ün] wide",
        ] {
            assert_eq!(split(line), ("", line), "{line}");
            assert_eq!(time(line), None, "{line}");
        }
    }
}
