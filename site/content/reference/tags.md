+++
title = "Tags"
weight = 90
description = "Tagging projects, sections and tasks from the tag picker, filtering a pane by tags, and how tags are kept in the file."
+++

A tag is a word after `@`, such as `@errands` or `@deep`, as TaskPaper writes it. Any project, section or task can carry any number of tags; an area carries none. Tags go on and come off from the **tag picker**, and a pane can be **filtered** to the items that carry chosen tags, as Things 3 filters by tag.

## Tags on rows

A row shows its tags after its title, each as `@name`, dimmed in the theme's tag color, wrapping with the title when it is long. A tag with a value shows it, as `@where(home)`. Once the item is completed its tags fade with the rest of the row.

The tags the app writes itself for dates, completion and the like (see [how tags are stored](#how-tags-are-stored)) never show as tags; they show as date chips, the done marker, the flag and so on.

A title cannot take a tag by typing: typing `@` at the start of a word in a title is refused, and the status line says `a title has no word starting with @; add tags with mg`. The same goes for Quick Add and the title in the [edit dialog](@/reference/editing.md#the-edit-dialog), which has a Tags field of its own that opens the same picker.

## The tag picker

**Tags** (`mg`, **m**odify, ta**g**) opens the tag picker on the [selected items](@/reference/selection.md), or the focused row when nothing is selected; in the sidebar, on the focused area or project. It lists every tag used in the document, outside the Trash and outside what completed items hold, in alphabetical order whatever the case. At the right of each tag a mark says how it stands with the items:

| Mark | Means |
|---|---|
| `✓` | every selected item carries it; choosing it takes it off them all |
| `–` | some of them carry it; choosing it adds it to the rest |
| none | none carries it; choosing it adds it to them all |
| `new tag` | no item carries it yet; choosing it creates it |

Typing narrows the list by [fuzzy match](@/reference/finders.md#fuzzy-matching). When the typed name is not a tag yet, it comes first, marked `new tag`, so `Enter` creates it. Spaces in it become `-`: typing `phone call` offers `phone-call`. A name a TaskPaper tag cannot hold, such as one with `:` or `(`, or one of the [reserved names](#reserved-names), is not offered.

Choosing a tag closes the picker and changes every selected item at once, as one step to [undo](@/reference/editing.md#key-undo). A tag with a value counts by its name, so taking `home` off also takes off `@home(kitchen)`. `Esc` closes the picker without a change.

Tagging an area alone is refused with `an area has no tags`; in a selection that also holds other items, the areas are left out and the rest are tagged.

{{ shot(src="reference-2/tag-picker", alt="The Launch the billing page project with two tasks selected and the tag picker open over it, listing calls, deep, errands, waiting and writing, with waiting and writing marked with a dash because one of the two tasks carries each", caption="mg on two selected tasks: each carries one of waiting and writing.") }}

{{ keys(topic="tags", titles="Tags") }}

## Filtering by tags

**Filter by Tags** (`gf`) opens the same picker, titled Filter by tags, on the tags the pane's rows carry, the tags they inherit from the containers they are in, and those carried inside them. Choosing one adds it to the pane's filter, or takes it out when the filter has it; open the picker again to add another. The filter shows at the right end of the pane's title line, after the tag icon, as `@errands @home`. **Clear Tag Filter** (`gF`) takes them all off.

A filtered pane shows only the rows whose item carries **every** chosen tag, counting the tags it inherits, as Things 3 does: a task in a project tagged `@work` counts as tagged `@work`. A section heading stays over a task that matches, and a project's row stays when an open item inside it matches. The filter works on a project or area, the Inbox, any view and [search results](@/reference/finders.md#search).

A task added in a filtered pane is given the filter's tags, so that it does not vanish as soon as it is added.

The filter belongs to one pane and lasts while the pane shows the same place: opening another place in the pane clears it, and it is not kept with the [pane layout](@/reference/panes.md#layout-on-the-next-launch). A new pane starts without one.

{{ shot(src="reference-2/tag-filter", alt="The All view filtered by @errands, the tag shown at the right of its title line, listing only the four tasks tagged @errands under Home, Weekend in Lisbon and Run a half marathon", caption="All, filtered by @errands.") }}

{{ keys(topic="tags", titles="Filter by Tags|Clear Tag Filter") }}

## Reserved names

These names belong to the app, which writes them itself or reads them as something other than a tag, so they cannot be given as tags from the picker:

`start`, `due`, `done`, `deleted`, `later`, `hidden`, `area`, `section`, `pinned`, `flagged`, `id`, `text`, `type`

Tag names are case-sensitive: `@Home` and `@home` are two tags.

## How tags are stored

Tags are written in the file at the end of the item's line, after the tags the app writes for the item's kind, pin, flag, dates and Later mark, and before its completion and deletion:

```
- Wire the card form to the payments API @start(2026-10-18) @deep @phone-call
```

A tag the picker adds is a bare `@name`. Tags the file gives in other forms are kept as they are: a tag with a value, `@where(home)`, or a date tag whose value is not a `YYYY-MM-DD` day, such as `@due(soon)`, which stays an ordinary tag and shows on the row. Tags a file gives an area stay in the file but are not shown, offered or filtered by.
