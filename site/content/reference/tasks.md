+++
title = "Tasks and completing"
weight = 30
description = "What a task row shows, completing and uncompleting, showing completed items, and marking items Later or flagged."
+++

A task is one thing to do. Besides its title it can have a description, [notes, comments and a checklist](@/reference/attachments.md), a [start and a due date](@/reference/dates.md), and [tags](@/reference/tags.md); it can be marked Later or flagged. Tasks sit in the Inbox, an area, a project or a section.

## The task row

```
☐ ⚑ Wire the card form to the payments API @deep        ☑ 1/5 ≡ ❞ 1 ↦ today ⇥ Fri
    Use the sandbox keys until the security review is done.
```

From left to right, a task row shows:

- its **checkbox**, `☐` while open and `✔` once completed;
- a red **flag** when it is [flagged](#flags);
- its **title**, and after it its **tags**, dimmed;
- at the right edge, its **checklist progress**, such as `1/5` checked of five, the **note** icon when it has a note, the **comments** icon with their count, and its **date chips**, the start date in green and the due date in red, named relative to today (see [Dates](@/reference/dates.md#date-chips));
- below the title, the **first line of its description**, dimmed.

The markers above are those drawn without Nerd Font icons ([Icons](@/reference/settings.md#setting-icons) off); with icons on, they are Nerd Font glyphs, and only the checkbox stays as shown. With [Word wrap](@/reference/settings.md#setting-word-wrap) on, a long title wraps; off, it takes one row ending in `…`, and the description line too.

Project and section rows show the same icons and chips; a project has a `#` and a section heading a `/` in place of the checkbox. An area's row shows its attachment icons only, since an area has no dates or tags. A completed item is drawn dimmed with its `✔`, never struck through, and a [Later](#later) item dimmed.

## Completing

**Complete** (`x`, `.` or `Ctrl+Enter`) completes the selected items, or the focused one, on today's date, or the project shown while its title row has the focus, or, in the [sidebar](@/reference/views.md#the-sidebar), the focused project. It uncompletes them when every one of them is completed already. It is one step to undo however many items it changes. A click on a task's checkbox completes it too.

- **Tasks, projects and sections** can be completed. An area cannot (`an area can't be completed`), nor the Inbox; checklist items are checked in the [attachments panel](@/reference/attachments.md).
- **A completed item stays where it is** in the file, tagged `@done(YYYY-MM-DD)`. Lists show it after the open items, and hide it until you show completed items. The [Logbook](@/reference/views.md#logbook) collects every completed item; completing or uncompleting there acts on the item where it is.
- **Completing leaves what is inside alone.** The tasks of a completed project or section keep their own state, open ones included, but are shown nowhere any more: not in views, the Logbook, search, the Navigator, the sidebar or pickers. Uncompleting the project shows them all again.
- **A project with open tasks asks first.** The first press changes nothing and says `9 open tasks; press x again to complete anyway`; pressing it again completes the project, and any other command cancels. The count takes the open tasks at any depth, in its sections too, and a selection asks once for all its projects. Sections, tasks, projects without open tasks and uncompleting never ask.

When the focused row hides because it was completed, the focus moves to the next row, else the one before.

{{ keys(topic="tasks", titles="Complete") }}

## Completed items

**Show Completed Items** (`Ctrl+O`) shows the completed items in the pane, or hides them again. Each pane has its own setting, which its saved layout keeps.

In an opened area, project or the Inbox, the completed items are not mixed with the others: they are listed together at the bottom under the heading **Completed Items**, most recently completed first, those completed on the same day in file order and those with no date last. Completed tasks of open sections are listed there too, no longer under their heading, and so are completed sections and child projects, each as one row. These rows are focused, selected and uncompleted like any others; uncompleting one puts it back in its place. Moving a completed row with `Alt` does nothing, since its place is its date.

In [All](@/reference/views.md#all), each group lists its completed tasks last the same way. Today, Upcoming and Flagged list them after the open items. The Logbook always shows them, and so do search results, marked as completed.

{{ shot(src="reference-1/completed-items", alt="The project Launch the billing page with completed items shown: its open tasks and the sections Build, Launch and Polish, then the heading Completed Items with two checked, dimmed tasks below it.", caption="Show Completed Items lists them under Completed Items, newest first.") }}

{{ keys(topic="tasks", titles="Show Completed Items") }}

## Later

**Toggle Later** (`ml`) marks the selected tasks and projects Later, as Things 3's Someday, or takes them out of Later when every one of them is Later already. In a pane it acts on the selection, or on the container shown while its title row is focused; in the sidebar on the focused project.

- A Later item is drawn **dimmed** and listed **after the open items** of its group, and before the completed ones, in its container, in [All](@/reference/views.md#all) and in the sidebar and the Navigator, without moving in the file. The file marks it `@later`.
- Everything inside a Later project counts as Later and is dimmed too, without a mark of its own.
- **Later and a start date exclude each other.** Marking an item Later removes its start date, and giving it a start date takes it out of Later. See [Dates](@/reference/dates.md#later-and-start-dates).
- [Upcoming](@/reference/views.md#upcoming) leaves Later items out, and [Today](@/reference/views.md#today) lists one only once it is due.
- Only tasks and projects can be Later: on areas or sections alone the status line says `only a task or a project can be Later`, and in a mixed selection they are skipped.

{{ keys(topic="tasks", titles="Toggle Later") }}

## Flags

**Toggle Flag** (`mf`) flags the selected tasks and projects as important, or unflags them when every one of them is flagged. It acts where Toggle Later does: the selection or the title row of the container shown in a pane, the focused project in the sidebar. The [edit dialog](@/reference/editing.md#the-edit-dialog) flags and unflags too.

A flagged item shows a red flag in front of its title in its list, the sidebar and the Navigator, and is listed in the [Flagged](@/reference/views.md#flagged) view. The file marks it `@flagged`. Only tasks and projects take a flag: on areas or sections alone, and on the Inbox or a view in the sidebar, the status line says `only a task or a project can be flagged`.

{{ keys(topic="tasks", titles="Toggle Flag") }}
