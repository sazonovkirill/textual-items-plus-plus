+++
title = "Areas, projects and sections"
weight = 20
description = "The containers that hold tasks: what each one is, what it shows when opened, and creating, renaming, moving, converting, grouping and deleting them."
+++

Work is organized as in Things 3: **areas** hold **projects**, both can hold **sections**, and all of them, with the **Inbox**, hold **tasks**. Together they are called containers. Each kind can only go in certain places, and the app refuses, with a message in the status line, anything that would put one somewhere else.

## Areas, projects, sections and the Inbox

| Kind | Marker | Sits in | Holds | Has |
|---|---|---|---|---|
| Area | cube icon (`◆` without icons), title in the area color | the top level | tasks, sections, projects | title, description, notes, comments |
| Project | `#` | the top level or an area | tasks, sections | title, description, notes, comments, start and due date, tags, Later, flag, pin |
| Section | `/` | an area or a project | tasks | title, description, notes, comments, start and due date, tags |
| Inbox | inbox icon | always first | tasks | description, notes, comments |

- **Areas** are areas of responsibility, such as Work or Home. They do not nest, have no dates or tags, and are never completed, only deleted. Asking to date, tag or complete one says `an area has no start or due date`, `an area has no tags` or `an area can't be completed`.
- **Projects** are goals made of tasks. Projects do not nest in the app: no command puts a project inside another, and a project trying to go into a section or the Inbox is refused with `a project goes only in an area or at the top level`. A file from another editor may still nest one; the app keeps and shows it.
- **Sections** are titled groups of tasks, like headings in Things 3, but they can be in an area too and have descriptions. A section never opens on its own: wherever something would open one, the pane opens the area or project holding it, focused on the section's heading. Anything but a task going into a section is refused with `a section holds only tasks`, and a section going anywhere but an area or a project with `a section goes only in an area or a project`.
- **The Inbox** is built in, always first, and holds tasks only. It cannot be renamed, moved or deleted. A task in it that gets a start date leaves the Inbox view, as scheduling does in Things 3, but stays in the Inbox in the file; see [Inbox](@/reference/views.md#inbox).

How each kind is written in the file is in [Files and saving](@/reference/files.md#how-items-are-stored).

## What an area or a project shows

Opening an area, a project or the Inbox (`Enter` on its row in the pane or the sidebar, or the [Navigator](@/reference/finders.md#key-open-navigator)) shows it in the pane in a fixed arrangement, whatever order the file has its contents in:

1. The **title row**: its marker, title and, for a project, flag, attachment icons and date chips. In an area or a project the row can be focused, by `k` from the first row or by `gg`, so commands that act on an item's own details, such as dates, Later, flags and pins, act on the container itself there.
2. Its **description**, in full, dimmed.
3. Its **tasks outside any section**, in file order.
4. Its **sections**, in file order, each a bold heading with the first line of its description and then its tasks.
5. For an area, its **projects**, in file order, each as one row with its description's first line, attachment icons and date chips. A project's tasks and sections show only when you open it.
6. With [Show Completed Items](@/reference/tasks.md#completed-items) on, the **Completed Items**.

Within each group the open items come first, then the [Later](@/reference/tasks.md#later) ones, dimmed. Completed items are hidden until Show Completed Items shows them, all together at the bottom. The groups never mix on screen, and the file keeps its own order until you move something.

{{ shot(src="reference-1/area", alt="The Work area opened in the pane: its title and description, two tasks outside any section, the Team section with two tasks, then the projects Launch the billing page, Hire a backend engineer and Q4 planning, each as one row with its due date chip.", caption="An area: its tasks, its sections, then its projects.") }}

## Creating

Every new container is created empty with its title being edited in place; type the title and press `Enter` or `Esc`. A new item left with an empty title is not added.

- **New Section at Bottom** (`s`) and **New Section at Top** (`S`) add a section last or first among the sections of the area or project shown. In a section's place, as from a project's task, the section goes in that project; from the Inbox none is added.
- **New Project at Bottom** (`np`) and **New Project at Top** (`nP`) add a project last or first among the projects of the area shown. Shown a project, which cannot hold one, the new project goes beside it in its area, or at the top level; from the Inbox it goes first at the top level. A new project that is not a row of the pane opens in it with its title edited in the heading; left empty, the pane goes back.
- **New Area at Bottom** (`na`) and **New Area at Top** (`nA`) work in the [sidebar](@/reference/views.md#the-sidebar): they add an area last or first among the areas. In a pane they do nothing.

In the sidebar, `np` and `nP` add a project last or first among the projects in no area, and `o` or `Space` and `O` add a project below or above the focused entry: below an area, as its first project; below or above a project, beside it; from the Inbox or a view, first among the projects in no area.

New tasks are on [Editing](@/reference/editing.md#new-items).

{{ keys(topic="organization", titles="New Section at Bottom|New Section at Top|New Project at Bottom|New Project at Top|New Area at Bottom|New Area at Top") }}

## Renaming

**Rename Container** (`R`) edits the title of the area or project the pane shows, in its heading. In the sidebar `R`, like `i`, renames the focused entry. A section, or a project row inside an area, is renamed like a task: `i` on its row, or its Title field in the [edit dialog](@/reference/editing.md#the-edit-dialog). The Inbox and the views are never renamed; in the [Trash](@/reference/views.md#trash) `R` restores instead.

A container's title cannot start with `-` and a space, which would read back as a task, nor have a word starting with `@`, which would read back as a tag.

{{ keys(topic="organization", titles="Rename Container") }}

## Moving

A section moves among the sections of its container, and a project row among the projects of its area, with **Move Up** and **Move Down** (`Alt+k`/`Alt+j`); a section never moves above the tasks outside sections. **Move To** (`M`) moves a section into another area or project, and a project into an area or to the top level. In the sidebar, `Alt+k` and `Alt+j` swap the focused area or project with the entry of its kind above or below it, and `M` opens Move To for it, which is how a project goes into an area or out of one. Everything inside moves along. See [Selection and rearranging](@/reference/selection.md#moving).

## Deleting

**Delete Container**, from the [command palette](@/reference/finders.md#key-command-palette), moves the area or project the pane shows to the Trash, with everything in it, and the pane shows the container above it. A section or a project row is deleted like any row, with `dd`; in the sidebar, `dd` deletes the focused area or project. Deleting is one step to undo, and the [Trash](@/reference/views.md#trash) restores it where it was.

{{ keys(topic="organization", titles="Delete Container") }}

## Converting

**Convert to Project or Section** (`my`) converts each selected item in its place: a task, a section or an area becomes a project, or, when every selected item is a project, each becomes a section. It keeps its title, description, notes, comments, dates and tags, and everything it holds; a task's checklist items become the project's tasks. A project never becomes an area. In the sidebar, `my` converts the focused entry, which is how an area becomes a project.

Where the result could not go, the item stays as it is, and the status line says why:

- a task in the Inbox, a section or a project, a section in a project, or an area holding projects, cannot become a project: `a project goes only in an area or at the top level`;
- a top-level project cannot become a section: `a section goes only in an area or a project`;
- a project holding sections cannot become a section: `a section holds only tasks`;
- a task whose title starts with `-` and a space: `only a task's title can start with "- "`;
- a task tagged `@area` or `@section`: `only a task can be tagged @area or @section`;
- something inside it in the Trash that could not go in the result: `something of it in the Trash can't go in a project; delete it for good first`, or `… in a section`.

With several items selected, those that can convert do; the reason is shown only when none converts.

{{ keys(topic="organization", titles="Convert to Project or Section") }}

## Grouping

**Group into Section** (`ws`, **w**rap in a **s**ection) makes the selected tasks the tasks of a new section, and **Group into Project** (`wp`) makes the selected tasks and sections the contents of a new project. The new container goes where the first selected item was, takes the items in their order, and has its title edited in its row. Adding it, moving the items and naming it are one step to undo; leaving the title empty groups nothing.

A section groups only tasks in an area or a project, outside any section; grouping tasks in a section or the Inbox is refused with `a section goes only in an area or a project`. A project groups in an area or at the top level, never in a project or the Inbox: `a project goes only in an area or at the top level`.

{{ keys(topic="organization", titles="Group into Section|Group into Project") }}
