+++
title = "Selection and rearranging"
weight = 70
description = "Selecting several items, moving them up and down or to another container, and duplicating, copying, cutting, pasting and deleting them."
+++

Commands that act on items act on the **selection**: the rows you picked, or the focused row when none is picked. Every item carries what is inside it: moving, copying or deleting a project moves, copies or deletes its sections and tasks, with their descriptions and attachments.

## Selecting

Picked rows get the focus bar in the gutter too, so the selection and the focus read as one marked run. Plain motions move the focus and keep what is picked.

- **Extend Selection Down** and **Up** (`J`/`K`, `Shift+↓`/`Shift+↑`) move the focus a row and pick every row from where the run began to it. Going back shrinks the run; a new run adds to what was picked before.
- **Toggle Selected** (`v`) picks the focused row, or unpicks it. With `v`, `j`, `j`, `v` you pick rows far apart one by one.
- **Visual Line Mode** (`V`), as in vim, picks the focused row, and from then on every motion, `j`/`k`, `gg`/`G` and the arrows, picks the rows from that row to the focus. The status line says `VISUAL LINE`. `V` again ends the mode, keeping the picks; `Esc` ends it picking nothing. Any other command acts on the picks and ends the mode, keeping them. `V` on a title row does nothing.
- **Select All** (`Ctrl+A`) picks every row of the pane, the title row aside.
- **Clear Selection** (`Esc`) picks nothing, so commands take the focused row again.

The title row of the container shown is never picked; commands that act on an item's own details, such as dates, act on the container there when nothing is picked. A picked row that leaves the pane, because it was deleted, completed and hidden or moved away, is unpicked.

Completing, dates, Later, flags, pins, tags, moving, deleting and the clipboard commands act on the whole selection at once, as one step to undo. The edit dialog, editing a title or description and the attachments act on the focused item alone.

{{ keys(topic="selection", titles="Extend Selection Down|Extend Selection Up|Toggle Selected|Visual Line Mode|Select All|Clear Selection") }}

## Moving

**Move Up** and **Move Down** (`Alt+k`/`Alt+j`, `Alt+↑`/`Alt+↓`, or `mk`/`mj`, for a macOS terminal that types a character for `Option` with a letter) move the selection one step among the rows it is shown with, as in Things 3. On screen, the tasks outside sections and each section's tasks read as one list with the headings as dividers, and a task moves through it:

- a task swaps with the task next to it under the same heading;
- moving down past a section heading makes it the **first task of that section**;
- moving up from the top of a section makes it the **last task of the section above**, or, from the first section, the last of the tasks outside sections;
- the last task of the last section moving down stays.

A **section** moves only among the sections, never above the tasks outside sections, and a **project row** only among the projects of its area. Open and [Later](@/reference/tasks.md#later) items move apart from each other, each among its own kind, and a completed item does not move, its place among the [Completed Items](@/reference/tasks.md#completed-items) being its date. A move passes over rows the pane hides, such as a task with a start date in the Inbox, which keeps its place in the file.

Several picked items move together and keep their order; one that would have to pass another picked item stays. The file is rewritten to match what the screen shows.

In [Today](@/reference/views.md#today) the same keys set Today's own order instead, which the file never keeps, and in the other views they do nothing. In the [sidebar](@/reference/views.md#the-sidebar) they move the focused area or project past the entry of its kind beside it.

{{ keys(topic="selection", titles="Move Up|Move Down") }}

## Move To

**Move To** (`M`) opens a picker of the places the selection can go, filtered as you type: the top level, when every item is a project, then the Inbox, then each area, project and section in a tree. It lists only the places that take every item: a section takes only tasks, the Inbox only tasks, an area or a project anything but areas, and a project no project. The items themselves, the place they are all in already, the Trash and completed items are left out.

`Enter` moves the items, with everything inside them and in their order, to the end of their group in the chosen place: tasks after its tasks, sections after its sections, projects after its projects. In the sidebar `M` moves the focused area or project, which is how a project goes into an area or out of one. In the Trash, Move To does nothing and says `only restore and delete work in the Trash`.

{{ keys(topic="selection", titles="Move To") }}

## Duplicate, copy, cut and paste

- **Duplicate** (`D`) puts a copy of each selected item, with everything inside it, right after it, and focuses the copy of the last one.
- **Copy** (`y`, `Ctrl+C`) puts the selected items on the clipboard as TaskPaper text: each item's line with its tags, its description, attachments and contents, indented with tabs, so you can paste it into any TaskPaper app or text editor.
- **Cut** (`Ctrl+X`) copies the selected items and takes them out of the document. They do not go to the Trash; undo puts them back.
- **Paste Below** (`p`, `Ctrl+V`) and **Paste Above** (`P`, `Alt+v`) read the clipboard's text as TaskPaper and put the items in on that side of the focused row, in order, each where a new item of its kind would go. Plain lines become tasks, so a pasted list of lines becomes a list of tasks. Pasting below the title row puts the items first in the container; above it, nothing is pasted. Pasted items never land in the Trash, even when copied from it.

Where an item of the clipboard cannot go, such as a section into the Inbox, nothing is pasted. Today takes only tasks: `only tasks paste in Today`.

The clipboard is the desktop's when the app can reach a display server. Otherwise, as over SSH, copying goes to the terminal through OSC 52, which the terminal must allow (in tmux, `set-clipboard on`); such a clipboard cannot be read back, so pasting then takes what the app copied last, and text pasted into the terminal arrives as a paste.

{{ keys(topic="selection", titles="Duplicate|Copy|Cut|Paste Below|Paste Above") }}

## Delete

**Delete** (`dd`, `Delete`) moves the selected items to the [Trash](@/reference/views.md#trash), with everything inside them, as one step to undo; the file tags each one `@deleted(YYYY-MM-DD)` where it is, and the Trash can restore it there. In the Trash, the same keys delete the selected items for good. The focus moves to the next row that is still there, else the one before.

Deleting the area or project the pane shows is [Delete Container](@/reference/organization.md#deleting), from the command palette.

{{ keys(topic="selection", titles="Delete") }}
