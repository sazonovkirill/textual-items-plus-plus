+++
title = "Finding things"
weight = 110
description = "The Navigator, Go to Symbol, full-text search, the command palette and the context menu, and the fuzzy matching they share."
+++

Five windows find things by what you type: the **Navigator** goes to a place, **Go to Symbol** to any item, **search** finds words anywhere in the document, the **command palette** runs any command and the **context menu** runs a command on the selected items. All but search are [pickers](#picker-keys) that open over the panes and filter their list as you type.

## The Navigator

**Open Navigator** (`Ctrl+K` or `Ctrl+G`) lists every place a pane can show: the Inbox, the views (Today, Upcoming, Flagged, All, Logbook and Trash), then the [pinned projects](@/reference/views.md), then every area with its projects and the projects outside areas, in file order, Later projects last. Each entry has its kind's icon, the area it is in dimmed after it where that tells two apart, and at the right how many open tasks it holds. Completed projects and what is in the Trash are left out.

It opens with the entry of what the pane shows highlighted, or of the area or project holding it. `Enter` shows the chosen place in the pane, as a step in its [history](@/reference/navigation.md#history); `Alt+Enter` or `Ctrl+T` shows it in a [new pane](@/reference/panes.md#opening-a-pane). Its key works from anywhere, also while a title is edited, and pressed again, or while another picker is open, closes that and opens the Navigator.

{{ shot(src="reference-2/navigator", alt="The Navigator open over the Inbox with lis typed, listing the project Weekend in Lisbon in the area Home with 3 open tasks and Read Designing Data-Intensive Applications with 2", caption="The Navigator after typing lis.") }}

## Go to Symbol

**Go to Symbol** (`/`) finds any item by its title, as VS Code's Go to Symbol does. It lists every area, project and section, then every task, each in file order, so that with nothing typed the places come first. Each shows its kind's icon and title, with the titles above it dimmed after it. The Inbox itself, checklist items, completed items and what they hold, and the Trash are left out.

Choosing an item shows it where it lives, focused: an area or project opens itself, a section opens the area or project holding it, focused on its heading, and a task opens the container listing it, or for a task no container lists, such as a scheduled task of the Inbox, the first view that lists it. When the item is in the place the pane shows already, only the focus moves, and the selection and the [tag filter](@/reference/tags.md#filtering-by-tags), which could hide it, are cleared. `Alt+Enter` or `Ctrl+T` opens it in a new pane.

{{ shot(src="reference-2/go-to-symbol", alt="Go to Symbol open with write typed, listing the tasks Write the pricing copy and Write the release notes, each followed by its location Work, Launch the billing page", caption="Go to Symbol finds tasks by their title.") }}

## Search

**Search** (`Ctrl+F` or `gs`) shows the pane's search results in place of what it showed, titled Search, with the words searched for after the name. The words are selected, so typing replaces them, and the results follow each change, the first one focused.

Every word must be found, in any case, somewhere in an item: its title, its tags (name or value), its description, its note, its comments (not their times) or the text of its checklist items, which find their task. A word can match part of a word: `pay` finds `payments`.

The results are every task, section and project that holds all the words, in file order, the completed ones last, marked as completed, whether or not completed items are shown. The Trash and what completed items hold are left out. Each result says after its title where it is, as `Work › Launch the billing page › Build`, with the words found marked in its title and tags. Under it is the first line, outside the title, that holds a word the title lacks, after the icon of the note, comments or checklist it comes from, the words marked.

`Enter` or `Esc` stops typing and gives the keys to the results, which then take every navigate key: `Enter` on a result [opens or edits](@/reference/navigation.md#opening) it as on any row: a project opens in the pane, a task opens in the edit dialog; and [Open in New Pane](@/reference/panes.md#key-open-in-new-pane) shows it where it lives in a new pane. Any command also ends typing. `Esc` in the results, with nothing selected, goes back to where the pane was before them, or to the Inbox when the results were the first place it showed. The words stay with the pane, so `Ctrl+F` or [Go Forward](@/reference/navigation.md#history) return to them, and they are kept with the [pane layout](@/reference/panes.md#layout-on-the-next-launch). Search results can be [filtered by tags](@/reference/tags.md#filtering-by-tags).

{{ shot(src="reference-2/search", alt="Search results for sandbox keys: the task Wire the card form to the payments API, its location Work, Launch the billing page, Build under it, and the description line Use the sandbox keys until the security review is done with both words underlined", caption="The words are found in the task's description, shown under it.") }}

{{ window_keys(window="Search", caption="While the words are typed") }}

## The command palette

**Command Palette** (`Ctrl+P` or `gc`) lists every command the app takes where it was opened, in the app's own order of commands, each with its keys dimmed at the right. `Enter` runs the chosen command, as its keys would. It is the way to the commands that have no key, such as [Change Theme](@/reference/appearance.md#the-theme-picker), [Start Weather Animation](@/reference/appearance.md#weather) or Open Checklist.

## The context menu

**Context Menu** (`Ctrl+\`, which a legacy terminal sends as `Ctrl+4`) drops down below the focused row, or above it where there is no room below, with the commands that act on the selected items, or the focused row, and would change something for them, grouped under the [shortcuts panel's](@/reference/keyboard.md#the-shortcuts-panel) headings, each after its icon with its keys at the right. While the sidebar has the keys it opens on the focused area or project.

It leaves out what does not apply: in the Trash only Restore and Delete; in a view other than Today nothing that adds or moves; Pin only for projects, Flag and Later only for tasks and projects, Open Checklist only on a task; dates, tags and completion not on areas alone. With several items selected, the commands that act on one row, such as Edit Title or Open Note, are left out. When nothing applies it does not open.

Typing filters it; each group's matches stay together under its heading, the groups in order of their best match, so `Enter` runs the best one.

{{ shot(src="reference-2/context-menu", alt="The context menu dropped down under a task of the Inbox, listing under Editing and Creating Edit Title, New Item Below, New Item Above, Edit Item, Edit Description, Open Note, Open Comments, Open Checklist and Quick Comment, then under Completing, Dates and Tags Complete, Set Start Date, Set Due Date, Start Today, Toggle Later and Start a Day Earlier, each with its keys", caption="The context menu on a task.") }}

## Picker keys

The Navigator, Go to Symbol, the command palette, the context menu and the other windows that choose from a list, such as Move to, the [tag picker](@/reference/tags.md#the-tag-picker) and the [theme picker](@/reference/appearance.md#the-theme-picker), share their keys. The first line is what you type, after `›`; the first match is highlighted. Text keys edit what you type as in any [text field](@/reference/editing.md#keys-text). With nothing matching, the list says `No matches`.

With the mouse, a click on an entry chooses it, a click outside the window closes it, and the wheel moves the highlight.

{{ window_keys(window="Picker") }}

`Alt+Enter` and `Ctrl+T` open the choice in a new pane only where a choice is a place to show: the Navigator and Go to Symbol.

## Fuzzy matching

What you type is split into words at spaces, and every word must match an entry's text, fuzzily: its letters in order, not necessarily next to each other, so `lbp` finds `Launch the billing page`. Matching ignores case unless a word has a capital letter, and accents unless it has one. Characters such as `!` or `^` match themselves. To match a space, type `\` before it. The best matches come first, entries that match as well keeping their order.

## Keys

{{ keys(topic="finders") }}
