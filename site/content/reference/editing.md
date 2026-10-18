+++
title = "Editing"
weight = 60
description = "Navigate and edit mode, adding items, editing titles and descriptions in place, the edit dialog, Quick Add, undo and redo, and the keys of every text field."
+++

Titles, dates and descriptions are edited in place, where the item is, as in Things 3. Every property but an item's attachments can also be changed in the [edit dialog](#the-edit-dialog), and [Quick Add](#quick-add) adds a task from anywhere. Notes, comments and checklists are edited in the [attachments panel](@/reference/attachments.md).

## Navigate and edit mode

In **navigate mode** keys move between items and act on them; the status line starts with `NAVIGATE`. In **edit mode** keys edit the text of one item, in its row, and the status line says `EDIT`. Editing keeps the rows around the item and the scroll position where they were.

While a title is edited, its tags, chips and icons step aside, and the text is written to the document as you type, so every pane shows it. `Enter` or `Esc` ends editing, and the whole edit becomes **one step to undo**, saved once editing ends. Any command that leaves the row, such as the Navigator, ends editing first and keeps the text.

- A title is trimmed when editing ends. An **existing title you emptied keeps the text it had**; a **new item left empty is not added**.
- `Backspace` in an empty title ends editing, as `Esc` does.
- A title cannot have a word starting with `@`, which would read back as a [tag](@/reference/tags.md): typing one is refused and the status line says `a title has no word starting with @; add tags with m g`, naming the [Tags](@/reference/tags.md#key-tags) key `mg`. The title of a section, project or area cannot start with `-` and a space, which would read back as a task.

{{ window_keys(window="Editing") }}

## Editing titles in place

**Edit Title** (`i`, as vim's insert) edits the focused row's title in place, the caret at its end: a task's, a section heading's, or a project row's in an area. On the title row it edits the title of the container shown; in the sidebar, the focused entry's. [Rename Container](@/reference/organization.md#renaming) (`R`) edits the title of the area or project shown from any row.

**Open or Edit** (`Enter`) opens the edit dialog on a task, a section or the title row, and opens an area or project row in the pane (see [Navigation](@/reference/navigation.md#opening)).

{{ keys(topic="editing", titles="Edit Title") }}

## New items

Every new item is added with an empty title and opens in edit mode; type its title and press `Enter`.

- **New Item Below** (`o` or `Space`) and **New Item Above** (`O`) add a task below or above the focused row, among the tasks it is shown with. Below a section heading the new task is that section's first; below the title row it is the container's first. Beside a project row, which no task stands among, it is the container's last task.
- In **Today**, `o`, `Space` and `O` add a task to the Inbox that starts today, so Today lists it beside the focused row and the Inbox view does not. The other views take no new items.
- **New Task at Bottom** and **New Task at Top**, from the command palette, add a task last or first among the tasks of the container shown, outside its sections.
- In the sidebar, `o`, `Space` and `O` add a project instead.

New sections, projects and areas are on [Areas, projects and sections](@/reference/organization.md#creating).

{{ keys(topic="editing", titles="New Item Below|New Item Above|New Task at Bottom|New Task at Top") }}

## Descriptions

Every item, area, project, section or task, and the Inbox, can have a **description**: Markdown text of any length, shown dimmed. Under a row only its first line shows; an opened area, project or the Inbox shows its own in full under its title. A checklist item has none.

**Edit Description** (`mn`, **n** for notes, as TaskPaper calls the lines it is stored as) edits the focused item's description in place under its row, and **Edit Container Description** (`mN`) the description of the container shown, under its title. While it is edited the whole description shows, wrapped between words.

- `Enter` breaks the line and `Esc` ends editing.
- `↑` and `↓` move by lines on screen as they wrap, keeping the column; `Home` and `End` go to the start and end of the line, `Ctrl+Home` and `Ctrl+End` to the start and end of the text, and `Shift` with any of them selects.
- `@` can be typed anywhere: description lines are stored guarded, so nothing in them reads as a tag, task or project (see [Files and saving](@/reference/files.md#how-items-are-stored)).
- Space and empty lines at the end are dropped when editing ends, so emptying a description removes it. The edit is one step to undo.

{{ window_keys(window="Editing Description") }}

{{ keys(topic="editing", titles="Edit Description|Edit Container Description") }}

## The edit dialog

**Edit Item** (`e` or `mi`), and `Enter` on a task, a section or the title row, opens the **edit dialog**: a panel across the bottom of the screen, above the status line, that edits every property of an item but its attachments. It is titled `Edit Task`, `Edit Project`, `Edit Section` or `Edit Area`, with one row per field:

| Item | Fields |
|---|---|
| Task | Title, Description, Tags, Start, Due, Flagged |
| Project | Title, Description, Tags, Start, Due, Pinned, Flagged |
| Section | Title, Description |
| Area | Title, Description |

In the sidebar it opens on the focused area or project. The Inbox, a view and a checklist item open none.

The focused field is marked, and the hint row at the bottom names its keys. **Every change applies at once, as its own step to undo**, so there is nothing to save: `Esc` closes the dialog.

- `Tab` or `↓` goes to the next field and `Shift+Tab` or `↑` to the previous one; leaving a field applies what was typed in it, as `Enter` does.
- **Title** takes the text keys and refuses what editing in place refuses. An empty title is not applied: the field keeps the focus and the hint row says why.
- **Description** shows up to six lines. `Enter` breaks the line, and `↑` and `↓` move by lines, leaving the field past its first or last line.
- **Tags**: typing a character, or `Space`, opens the **tag picker** over the dialog, filtered by what you type. `Space` puts the focused tag on the item or takes it off at once, keeping the picker open, a tag taken off staying listed; a name no tag has yet is offered as a `new tag`. Spaces never reach the filter, since tags have none. `Enter` or `Esc` closes the picker. On the field itself, `Backspace` takes off the last tag.
- **Start** and **Due** show the date in full and relative to today, such as `Fri 23 Oct 2026 · Fri`. Focused, the field holds the date as `YYYY-MM-DD`, selected, so typing replaces it; it takes the words of the [date prompt](@/reference/dates.md#the-date-prompt) and shows what they resolve to as you type. `Enter` sets the date; an empty field removes it; a text that is not a date is not applied, and the hint row says why, as `Not a date from “soon”`.
- **Pinned** and **Flagged**: `Space` pins or unpins, flags or unflags.

A paste goes into the focused field, a description keeping its line breaks. Quitting with the dialog open applies the field being typed, as `Esc` does.

{{ shot(src="reference-1/edit-dialog", alt="The edit dialog open on the task Wire the card form to the payments API, across the bottom of the screen: rows for Title, Description, Tags @deep, Start Sun 18 Oct 2026 · today, Due None and Flagged Not flagged, with the hint Tab next field · Enter confirm · Esc close.", caption="The edit dialog on a task.") }}

{{ keys(topic="editing", titles="Edit Item") }}

{{ window_keys(window="Edit Dialog") }}

{{ window_keys(window="Tags") }}

## Quick Add

**Quick Add** (`q`) opens a one-line panel above the status line that adds a task without leaving what the pane shows. Its title names where the task goes, `Quick Add → Inbox` until you choose another place. Type the task; the line below shows the dates it will get, or how to start.

**Dates in the text.** Words that read as a date, in the grammar of the [date prompt](@/reference/dates.md#the-words-it-understands), become the task's start date, and words after the word `due` its due date. They are drawn in the start and due colors as you type and left out of the title: `Call Ann about the offsite next week due fri` adds `Call Ann about the offsite` starting next Monday and due on Friday. So that ordinary words stay text:

- only whole words count, the longest run of them that reads as a date at each word; punctuation after a word, such as `tomorrow.`, is ignored;
- a run starting with a bare number, such as `3 days`, is not a date; `+3d` and `in 3 days` are;
- a start date that is only a three-letter weekday, such as `sun` or `wed`, is not a date; `next sun`, `sunday` and `due sun` are;
- a start date before today, as in `call about last week`, stays text;
- of several start dates, or several due dates, the last one counts, and the earlier ones stay in the title.

**Choosing a place.** `/` at the start of a word opens a picker of every place a task can go: the Inbox, then each area, project and section in a tree. Typing filters it, and `Enter` makes the focused place the one the task goes to, which the panel's title then names. `Esc`, or a character after which nothing matches, closes the picker and types `/` and what you typed into the text; `Backspace` on an empty filter closes it.

`Enter` adds the task, once it has a title besides its dates, as the last task of its place, as one step to undo, and the status line says `added to Work › Launch the billing page`. `Esc` closes the panel, adding nothing. The text cannot start a word with `@`, as a title cannot.

{{ shot(src="reference-1/quick-add", alt="The Quick Add panel at the bottom, titled Quick Add → Inbox, holding the text Call Ann about the offsite next week due fri, with next week in the start color and due fri in the due color, and below it Start Mon 19 Oct 2026 · Due Fri 23 Oct 2026.", caption="Quick Add reads a start and a due date from the words typed.") }}

{{ keys(topic="editing", titles="Quick Add") }}

{{ window_keys(window="Quick Add") }}

## Undo and redo

**Undo** (`z`, `u` or `Ctrl+Z`) takes back the latest change, and **Redo** (`Z`, `Ctrl+Shift+Z` or `Ctrl+Y`) makes it again. Every change can be undone, structural ones included, and each command is one step however many items it changes; so is a whole edit of a title or description, and each field confirmed in the edit dialog. Undo and redo put the focus and the selection back where they were around the change. Putting items in the Evening and reordering Today are steps too, though they never change the file.

When the file reloads after another program changed it, the undo history starts over. A terminal without the kitty keyboard protocol sends `Ctrl+Shift+Z` as `Ctrl+Z`, which undoes; use `Z` or `Ctrl+Y` there.

{{ keys(topic="editing", titles="Undo|Redo") }}

## Text keys

Every text field, a title in place, a description, the edit dialog's fields, Quick Add, the date prompt, Save As, the filters of the pickers and the attachments panel's editors, takes the standard text keys of a desktop text field, not vim's. Typing replaces the selected text. They include the keys macOS terminals send for `Option` and `Cmd` with the arrows and `Delete` under iTerm2's Natural Text Editing preset: `Alt+b`/`Alt+f` move by a word, `Ctrl+A`/`Ctrl+E` go to the start and end of the line, `Ctrl+U` deletes to the start of the line and `Alt+Backspace` deletes a word.

`Ctrl+V`, and a paste from the terminal, put the clipboard's text in.

{{ window_keys(window="Text") }}
