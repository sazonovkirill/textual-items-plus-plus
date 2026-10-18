+++
title = "Dates"
weight = 80
description = "Start and due dates: the date prompt and every word it understands, the chips that show them, overdue items, nudging a day at a time, and Later."
+++

Any task, section or project can have a **start date**, the day it becomes relevant (Things 3's When), and a **due date**, the day it must be done (Things 3's Deadline), or both, or neither. An area has no dates: dating one alone says `an area has no start or due date`, and in a selection areas are skipped. Dates are calendar days in your local time zone, without a time of day. A container's dates are its own and do not change the dates of what is inside it.

Where dated items show up: a start or due date today or earlier puts an item in [Today](@/reference/views.md#today), and a later one in [Upcoming](@/reference/views.md#upcoming); a start date takes a task out of the [Inbox](@/reference/views.md#inbox) view. In the file the dates are `@start(YYYY-MM-DD)` and `@due(YYYY-MM-DD)` tags.

## The date prompt

**Set Start Date** (`ms`) and **Set Due Date** (`md`) open the date prompt above the status line, titled `Start date` or `Due date`, for the selected items, or for the container shown while its title row is focused, or in the sidebar for the focused area or project. It holds the focused item's date as `YYYY-MM-DD`, selected, so typing replaces it and `Enter` keeps it.

As you type, the line below shows what the text resolves to, such as `Fri 23 Oct 2026`, or why it is not a date:

| The prompt says | When |
|---|---|
| `Fri 23 Oct 2026` | the text is a date; `Enter` sets it |
| `No date: Enter removes it` | the text is empty; `Enter` removes the date |
| `Not a date from “…”` | the text stops being a date at the part quoted |
| `Not a date: something is missing` | the text ends too early, as `+3` or `next` |
| `No such day: “02-30”` | a written date the calendar does not have |
| `Out of range` | the date falls outside the years 0000 to 9999 |

`Enter` with a text that is not a date keeps the prompt open. `Esc` closes it and changes nothing. Setting the date of several items is one step to undo; items already on that date are left alone.

The edit dialog's Start and Due fields take the same words (see [The edit dialog](@/reference/editing.md#the-edit-dialog)), and so does [Quick Add](@/reference/editing.md#quick-add), which finds them among the words of a task.

{{ shot(src="reference-1/date-prompt", alt="The date prompt titled Start date at the bottom of the screen, holding the text next fri +1w, and below it the date it resolves to, Fri 6 Nov 2026.", caption="The date prompt shows the date the words resolve to as they are typed.") }}

{{ keys(topic="dates", titles="Set Start Date|Set Due Date") }}

{{ window_keys(window="Date Prompt") }}

## The words it understands

A date is an optional **day** followed by any number of **offsets**, each applied in turn. Without a day, the offsets count from today. Letters can be in any case, and spaces between the parts do not matter: `Next Fri`, `+ 3 D` and `today+1d` all work. The examples below are for Sunday, 18 October 2026.

**Days**

| Words | Day | Example |
|---|---|---|
| `today`, `tomorrow`, `yesterday` | that day | `tomorrow` is Mon 19 Oct |
| a weekday: `monday` … `sunday`, or its first three letters, `mon` … `sun` | the next such day after today, never today itself | `fri` is Fri 23 Oct; `sun` is Sun 25 Oct |
| `this` and a weekday | the same as the weekday alone | `this fri` is Fri 23 Oct |
| `next` and a weekday | that weekday 7 to 13 days ahead, as its chip names it | `next fri` is Fri 30 Oct |
| `last` and a weekday | the latest such day before today | `last fri` is Fri 16 Oct |
| `weekend`, `this weekend` | the coming Saturday, today on a Saturday | Sat 24 Oct |
| `next weekend`, `last weekend` | a week after or before that Saturday | Sat 31 Oct, Sat 17 Oct |
| `this week`, `next week`, `last week` | the Monday of this, next or last week (weeks start on Monday) | `next week` is Mon 19 Oct |
| `this month`, `next month`, `last month` | the first day of that month | `next month` is Sun 1 Nov |
| `this year`, `next year`, `last year` | 1 January of that year | `next year` is Fri 1 Jan 2027 |
| `in` a number and a unit, or `in a` / `in an` and a unit | that far from today | `in 3 days` is Wed 21 Oct; `in a month` is Wed 18 Nov |
| `YYYY-MM-DD`, written in full | that day | `2026-12-25` |
| a month and a day, `MM-DD` or `M/D` | that day this year; for a **due date** already past this year, next year's | `12/25` is 25 Dec 2026; as a due date, `3/7` is 7 Mar 2027 |

**Offsets**

| Words | Moves the date |
|---|---|
| `+N` and a unit | `N` units later: `+3d`, `+ 2 weeks` |
| `-N` and a unit | `N` units earlier: `-1d`, `-1 month` |
| `N` and a unit | `N` units later, as `+N`: `3d`, `2 weeks` |

The units are `d`, `day`, `days`; `w`, `week`, `weeks`; `m`, `month`, `months` (months, not minutes: dates have no time); and `y`, `year`, `years`. A month or year offset that lands past the end of a month stops at its last day: `2026-01-31 +1m` is 28 February. Offsets combine in order, so `fri +1w` is the Friday after next, `next week -1d` the Sunday before next week, and `2026-01-31 +1m -1d` 27 February.

**What is not a date**

- A day can only come first: `+1w fri`, `today tomorrow` and `fri in 2d` are refused.
- Only these spellings of a weekday: `fr` and `fridays` are not dates.
- `YYYY-MM-DD` must have four, two and two digits: `2026-1-5` is not a date.
- Month names, such as `Oct 5`, and times of day, such as `in an hour`, are not understood.
- A day the calendar lacks is refused, as `02-30` or `2026-02-29`, and so is a date outside 0000-01-01 to 9999-12-31.

## Date chips

A row shows its dates at its right edge as **chips**, the start date first, in green with a calendar-start icon (`↦` without icons), then the due date, in red with a calendar-end icon (`⇥`). Each is named relative to today:

| Date | Chip |
|---|---|
| today, tomorrow, yesterday | `today`, `tomorrow`, `yesterday` |
| 2 to 6 days ahead | the weekday: `Tue` |
| 7 to 13 days ahead | `next` and the weekday: `next Tue` |
| 14 days ahead or more | `in 16 days` |
| 2 days ago or more | `3 days ago` |

Project and section rows and title rows show chips too; an area has none, and dates a file gives an area stay in the file unseen. [Today](@/reference/views.md#today) shows only the due chip, since every row there has started. While a title is edited, its chips step aside. The edit dialog names dates in full, as `Fri 23 Oct 2026 · Fri`, and the day headings of Upcoming and the Logbook as `Tomorrow · Mon 19 Oct`.

## Overdue

A due date before today is **overdue** until the item is completed. Its chip is drawn bold and reversed, red behind the text, with a calendar-alert icon (`!` without icons), wherever the item shows. [Today](@/reference/views.md#today) lists overdue items first.

## Nudging a day at a time

**Start a Day Earlier** and **Start a Day Later** (`[` and `]`) move the start date of the selected items a day back or forward, and **Due a Day Earlier** and **Due a Day Later** (`{` and `}`) their due date. An item without that date counts from today, so `]` starts it tomorrow and `}` makes it due tomorrow. Each press is one step to undo. They act where the date prompt does, also on the focused entry in the sidebar.

{{ keys(topic="dates", titles="Start a Day Earlier|Start a Day Later|Due a Day Earlier|Due a Day Later") }}

## Start Today

**Start Today** (`mt`) sets the start date of the selected items to today, putting them in Today, in one step.

{{ keys(topic="dates", titles="Start Today") }}

## Later and start dates

[Later](@/reference/tasks.md#later) is Things 3's Someday, and as there an item is either scheduled for a day or put off: Later and a start date exclude each other. **Marking an item Later removes its start date**, and **giving it a start date**, by the prompt, the edit dialog, Start Today or `[`/`]`, **takes it out of Later**. A due date stays either way. A Later item with a due date shows in Today once it is due, but not in Upcoming before.
