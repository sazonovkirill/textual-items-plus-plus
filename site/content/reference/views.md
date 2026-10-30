+++
title = "Views and the sidebar"
weight = 40
description = "The Inbox, Today, Upcoming, Flagged, All, Logbook and Trash views, and the sidebar that lists them with the areas and projects."
+++

Besides the containers, which show their own contents (see [What an area or a project shows](@/reference/organization.md#what-an-area-or-a-project-shows)), the app builds **views**: lists that collect items from everywhere, such as everything due today. A view moves nothing in the file; it only collects. Every view and every container shows in a pane, and each has a `g` key, listed under [Keys](#keys).

Rows of a view work like rows anywhere: they are focused, selected, completed, dated, tagged and edited the same way, and a change made in a view shows at once wherever the item is. Rows from different places say where their item is, as `Work › Launch the billing page › Build`; a task in the Inbox says nothing.

## Inbox

The Inbox is where tasks go when no other place is chosen, as [Quick Add](@/reference/editing.md#quick-add) does by default. Its view shows its description and its tasks that have **no start date**, in their order. Giving an Inbox task a start date takes it out of the Inbox view, as scheduling does in Things 3: it stays under the Inbox in the file and shows in Today, Upcoming and All. Removing the start date brings it back.

## Today

Today lists every open task, section and project whose start date or due date is today or earlier, in one list. Its title line names the date, as `Today, Oct 18 2026`.

- **Overdue items**, due before today, come first, then the others, each in file order. Each row says under its title where its item is.
- Rows show their due date's chip, overdue or not, but **not their start date's**.
- A [Later](@/reference/tasks.md#later) item, or one inside a Later project, is listed only once it is due, not when it starts.
- **Move Up** and **Move Down** (`Alt+k`/`Alt+j`) reorder Today. The order you give it comes first, before the overdue items. It is the app's own state, not the file's: it is one step to undo, but does not make the document unsaved, and it is gone when the app restarts.
- Pasting in Today takes only tasks: `only tasks paste in Today`.

### Evening

**Toggle Evening** (`me`) puts the selected items in the **Evening**, a part of Today listed after the rest under an Evening heading with a moon, as Things 3's This Evening, and takes them out again when every one of them is in it. It works only in Today. Like Today's order, the Evening is the app's own state: it is one step to undo, does not change the file, holds an item only on the day it was put there and empties when the app restarts.

{{ keys(topic="views", titles="Toggle Evening") }}

## Upcoming

Upcoming lists every open task, section and project with a date after today, each once: on its start date, or on its due date when it has no start date. Items whose start date has come are in Today instead, even when their due date is later. A container's dates are its own, so its tasks are listed by their own dates.

The items are grouped by day for the seven days after today, then the rest of the month the seventh day is in, then by month:

| Heading | Holds |
|---|---|
| `Tomorrow · Mon 19 Oct` | tomorrow |
| `Tue 20 Oct` … | each of the next days, named by the date alone when the name would only repeat its weekday |
| `Next Sun · Sun 25 Oct` | the seventh day, named `next` as its chip is |
| `Rest of October` | the days after that in the same month |
| `November`, `January 2027` | each later month, with its year when it is not this year's |

Within a group the items go by date, then in file order. Later items, and everything inside a Later project, are not listed. Completed items are listed last in their group, only while completed items are shown. The groups have no order of their own to change: Move Up and Move Down do nothing here.

## Flagged

Flagged lists every [flagged](@/reference/tasks.md#flags) project and task from every list: the projects first, then the tasks, each in file order, each saying after its title where it is. Unflagging an item there takes it out of the view.

## All

All lists every open task in every list, grouped by where it lives, in place of Things 3's Anytime and Someday: first the Inbox's tasks under no heading, those with a start date included, then a group for each area and each project that holds tasks of its own, under its name, in the order of the file. Each group lists its tasks as its container does, its sections' tasks after its other tasks without the section headings. Later tasks come after the open ones of their group, and a Later project's group after the groups of its siblings. While completed items are shown, each group lists its completed tasks last, newest first.

## Logbook

The Logbook lists every completed task, section and project, grouped by the day it was completed, the newest day first, with headings such as `Yesterday · Sat 17 Oct` or `5 days ago · Tue 13 Oct`; items completed without a date come last. Each row says after its title where it is. It always shows completed items, whatever Show Completed Items says. It only collects: uncompleting an item there (`x`) uncompletes it where it is, and the item leaves the Logbook.

## Trash

Deleting (`dd` or `Delete`, see [Selection and rearranging](@/reference/selection.md#delete)) does not remove an item: it moves it to the Trash, with everything inside it, by tagging it `@deleted(YYYY-MM-DD)` where it is. The Trash lists each deleted area, project, section and task, the latest deleted first, those deleted the same day in file order. A task is listed as a task row; an area, project or section as one row, without its contents. Each row says after its title where it was.

- **Restore** (`R`) puts the selected items back where they were. If that place is inside a completed item, so no view would show them, the status line says so, as `restored into Launch the billing page, which is completed; uncomplete it to see it`.
- **Delete** (`dd`, `Delete`) in the Trash deletes the selected items for good.
- **Empty Trash** (`de`) deletes everything in the Trash for good. The first press asks, `press d e again to delete 3 items for good`; a second `de` right after empties it, and any other command cancels. With the Trash empty, nothing is asked.
- Duplicate and Move To do nothing in the Trash and say `only restore and delete work in the Trash`; copying a row is allowed, and its paste elsewhere is a live copy.

Each of these is one step to undo. Items in the Trash are left out of every other view, of search and of every count but the Trash's own.

{{ keys(topic="views", titles="Restore|Empty Trash") }}

## The sidebar

The sidebar on the left lists, as Things 3's does, in groups set apart by blank lines:

1. the **Inbox**;
2. the **views**: Today, Upcoming, Flagged, All, Logbook and Trash;
3. the **pinned projects**, when there are any;
4. the **projects in no area**;
5. each **area**, its title in capitals, followed by its projects.

Within each group entries keep the order of the file, Later projects listed after the others and dimmed. The entry of what the first pane shows is highlighted. The sidebar takes up to a third of the screen, 28 columns at most.

**Counts.** After each entry stands how many tasks it lists: for the Inbox, an area or a project, its open tasks, at any depth; for a view, the tasks it lists with completed items hidden, so the Logbook counts its completed tasks and the Trash its deleted ones. Sections and projects are not counted. The [Task counts](@/reference/settings.md#setting-task-counts) setting hides them.

**Showing and focusing.** The sidebar shows at start, without the keys. **Toggle Sidebar** (`Ctrl+B` or `|`) gives it the keys, shows it with the keys when it is hidden, and hides it when it has them. `|` is there because tmux often takes `Ctrl+B`. The [pane keys](@/reference/panes.md) reach it too: `Ctrl+W h` from the first pane, `g0`, and `Tab`/`Shift+Tab`.

**Browsing.** With the keys in the sidebar, `j`/`k` and the other motions move through its entries, and each entry the focus lands on shows in the first pane; `Enter` shows the focused one, the keys staying in the sidebar. From a pane, **Next Sidebar Entry** and **Previous Sidebar Entry** (`gj`/`gk`, `Ctrl+↓`/`Ctrl+↑`) move the sidebar's focus and show that entry in the first pane, the keys staying in the pane. A click on an entry shows it the same way.

**Acting on entries.** With the keys, the sidebar's keys act on the focused area or project: `R` or `i` renames it, `dd` deletes it, `Alt+k`/`Alt+j` move it past the entry of its kind beside it, `M` moves it with Move To, `my` converts it, `x`, `.` or `Ctrl+Enter` completes a project, `e` opens the edit dialog on it, `mn` edits its description in the first pane, the date, Later, flag, pin and tag keys change it, `\` opens its attachments in the first pane, and `c` comments on it. `o`, `O`, `Space`, `np`, `nP`, `na` and `nA` add projects and areas (see [Creating](@/reference/organization.md#creating)). The Inbox and the views are only shown. Undo, the Navigator, the command palette, search, going to a view, Back and Forward work as in a pane; the pane's other commands do nothing there.

{{ keys(topic="views", titles="Toggle Sidebar|Next Sidebar Entry|Previous Sidebar Entry") }}

### Collapsing

**Collapse or Expand** (`'`) folds the group of the focused entry into its first entry, and unfolds it again: the views fold into Today, the pinned projects into the first of them, the projects in no area into the first of them, and an area's projects into the area. A folded entry shows a dimmed `▸` before its count, and the focus moves to it. `'` on a folded entry unfolds it, and so does focusing another entry of its group, as adding a project there does. The Inbox stands alone. Which groups are folded lasts as long as the app runs.

{{ keys(topic="views", titles="Collapse or Expand") }}

### Pinned projects

**Pin or Unpin Project** (`mp`) pins the selected projects, or unpins them when every one is pinned. In a pane it acts on the selection, or on the project shown while its title row is focused, skipping anything that is not a project; in the sidebar on the focused project. A pinned project is listed twice, in the Pinned group after the views and in its usual place, in the sidebar and in the Navigator. The file marks it `@pinned`. With no project to act on, and on the Inbox or a view, the status line says `only a project is pinned`. A project converted to a section loses its pin.

In the Pinned group, `Alt+k`/`Alt+j` move a pinned project past the pinned project beside it only when that one is its sibling of the same kind, so both lists change together; otherwise the status line says `from Pinned it moves only past a pinned sibling`.

{{ keys(topic="views", titles="Pin or Unpin Project") }}

## Keys

Each view has a **Go to** key: `g` and a letter. [Go to Inbox](@/reference/navigation.md#key-go-to-inbox) is `gi`. Going to a view is a step in the pane's [history](@/reference/navigation.md#history).

{{ keys(topic="views", titles="Go to Today|Go to Upcoming|Go to Flagged|Go to All|Go to Logbook|Go to Trash") }}
