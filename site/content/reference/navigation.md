+++
title = "Navigation"
weight = 50
description = "Moving the focus, opening areas and projects, and going back and forward through where you have been."
+++

In navigate mode each pane has one **focused row**, marked by a bar in the gutter on its left and a brighter band across the pane. Every command that acts on an item acts on the focused one, or on the [selection](@/reference/selection.md) when there is one. The pane with the keys is the only one whose focus is marked.

## Moving

`j` and `k`, or `↓` and `↑`, move to the next and previous row: tasks, section headings and project rows alike. `gg` or `Home` goes to the first row, which in an area or a project is its title row, and `G` or `End` to the last.

Above the rows of an area or a project stands its **title row**, with its description below it. It takes the focus like any row: `k` on the first row reaches it, so commands such as [Edit Title](@/reference/editing.md#key-edit-title) or [Set Due Date](@/reference/dates.md#key-set-due-date) can act on the container itself. The Inbox and the views such as Today have a title line too, but it is never focused.

When the focused row goes away, because it was completed and hidden, deleted, moved or changed in another pane, the focus moves to the next row that is still there, else the one before, as a list does when an item leaves it. The pane scrolls to keep the focus in view.

{{ keys(topic="navigation", titles="Next Item|Previous Item|First Item|Last Item") }}

## Opening

**Open or Edit** (`Enter`) opens the focused area or project row in the pane, showing its description and its contents (see [Areas, projects and sections](@/reference/organization.md#what-an-area-or-a-project-shows)). On a task, a section or the title row of what the pane shows, it opens the [edit dialog](@/reference/editing.md#the-edit-dialog) instead. In the sidebar it shows the focused entry in the first pane, the keys staying in the sidebar.

A section never opens on its own: wherever something would show one, such as [Go to Symbol](@/reference/finders.md#go-to-symbol) or a search result, the pane shows the area or project holding it, focused on the section.

**Go to Inbox** (`gi`) shows the Inbox. The other views have `g` keys of their own, listed with the [views](@/reference/views.md#keys), and the [Navigator](@/reference/finders.md#the-navigator) reaches every view, area and project by name.

To show something in a new pane next to this one instead, use [Open in New Pane](@/reference/panes.md#key-open-in-new-pane).

## History

Every change of what a pane shows, by `Enter`, a `g` key, the sidebar, the Navigator or a finder, is a step in that pane's history. **Go Back** (`H`, `Alt+←` or `Alt+b`) and **Go Forward** (`L`, `Alt+→` or `Alt+f`) walk it, like a browser, and put the focus back on the row it was on. Places that no longer exist, such as a deleted project, are passed over. Each pane has a history of its own.

`Alt+b` and `Alt+f` are what iTerm2 sends for `Option+←` and `Option+→` with its Natural Text Editing preset, so the arrows work there too. In a text field these keys move by a word instead (see [text keys](@/reference/editing.md#keys-text)).

{{ keys(topic="navigation", titles="Open or Edit|Go to Inbox|Go Back|Go Forward") }}
