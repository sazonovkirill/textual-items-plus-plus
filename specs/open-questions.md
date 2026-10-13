# Open questions

- Status: Living
- Last updated: 2026-10-13

Product choices not settled by [functional-requirements.md](functional-requirements.md). Each lists the options, the **proposed default** the requirements assume while it is open, and what it affects. When the owner decides one, record the answer under **Decision**, update the affected requirements and specs, and keep the entry for history. Entries are numbered `OQ-n` and never renumbered.

## Open

None.

## Decided

The owner answered every question below on 2026-10-13.

### OQ-1 · Do areas nest? — decided

Things 3 areas do not nest; the owner asked for nesting projects only (D-2).

- **A. No:** areas sit at the top level only.
- **B. Yes:** an area can contain areas.

**Proposed default:** A.
**Decision (2026-10-13):** A, areas do not nest.
**Affects:** FR-ORG-01, FR-STORE-02.

### OQ-2 · What converting between kinds does with contents that do not fit — decided

A project can hold sections and projects; a section holds only tasks; an area holds projects, sections and tasks (D-2).

- **A. Refuse:** a conversion whose contents the target cannot hold (a project with sections or sub-projects into a section) is refused, saying why; every other conversion keeps the contents as they are.
- **B. Flatten:** the contents that do not fit move up a level.

**Proposed default:** A.
**Decision (2026-10-13):** A, refuse.
**Affects:** FR-ORG-06, FR-TASK-06.

### OQ-3 · What the Inbox holds — decided

- **A. Tasks only,** as in Things 3.
- **B. Tasks and sections.**

**Proposed default:** A.
**Decision (2026-10-13):** A, tasks only.
**Affects:** FR-ORG-07, FR-STORE-08.

### OQ-4 · Completing a project — decided

Things 3 asks whether to complete or cancel the project's open to-dos, then logs the project with them ([reference](reference/things3.md)).

- **A.** Completing a project with open tasks asks for confirmation, then completes it and every open task and checklist item in it, and logs them together.
- **B.** Completing a project leaves its open tasks open.
- **C.** As A, plus a separate Canceled state for tasks and projects, as in Things 3.

**Proposed default:** A; a Canceled state later if wanted.
**Decision (2026-10-13):** none of the options as written: completing any item, an area, project, section or task, leaves everything inside it uncompleted, in its own state, but no longer shown anywhere; uncompleting the item shows it all again. Recorded in FR-ORG-09.
**Affects:** FR-ORG-09, FR-VIEW-03.

### OQ-5 · When completed items move to the Logbook — decided

Things 3 has a setting: immediately, daily or manually ([reference](reference/things3.md)).

- **A. Daily:** completed items stay where they are, dimmed and struck through, until the next local day; then they show only in the Logbook.
- **B. Immediately:** they leave their list when completed.
- **C. A setting** with immediately, daily and manually, plus a Log Completed command.

**Proposed default:** C, with daily as the default value.
**Decision (2026-10-13):** never: completed items never leave their list. The Logbook is just a view that shows completed items from all lists. Recorded in FR-TASK-02 and FR-VIEW-03.
**Affects:** FR-TASK-02, FR-VIEW-03, FR-SET-03.

### OQ-6 · Where the checklist is edited — decided

- **A. Inline,** under the task's title when the task is opened in place, as in Things 3.
- **B. In the attachments panel,** with the description and comments.

**Proposed default:** A, since the owner asked for inline editing as in Things 3 (D-4) and reserved the panel for the description and comments (D-5).
**Decision (2026-10-13):** B, in the attachments panel, between the description and the comments.
**Affects:** FR-CHK-02, FR-EDIT-03.

### OQ-7 · Checking nested checklist items — decided

- **A. Independent:** each item keeps its own state.
- **B. Cascading:** checking an item checks everything under it, and checking the last open child checks its parent.

**Proposed default:** A.
**Decision (2026-10-13):** A with a display rule: checking an item does not check its children or parent, but a checked item and everything under it are drawn dimmed, recursively. Recorded in FR-CHK-04.
**Affects:** FR-CHK-04.

### OQ-8 · What Upcoming shows and how it groups — decided

- **A.** An item appears on its start date, or on its due date when it has no start date; its due date shows as a chip. The next seven days are grouped one by one, then the rest of the month, then whole months, as in Things 3.
- **B.** An item appears on both dates.

**Proposed default:** A.
**Decision (2026-10-13):** A.
**Affects:** FR-VIEW-02.

### OQ-9 · Today, Anytime and Someday — decided

The owner named Inbox, Upcoming, Logbook and Trash (D-3). Things 3 also has Today (items whose start date has come, plus deadlines due), Anytime and Someday ([reference](reference/things3.md)).

- **A.** Today as a Should; Anytime and Someday later.
- **B.** All three later.
- **C.** All three now.

**Proposed default:** B.
**Decision (2026-10-13):** Today and an All view now; All replaces Anytime and Someday, which the app will not have. As in Things 3, a scheduled task (one with a start date) leaves the Inbox view. Recorded in FR-VIEW-01, FR-VIEW-06, FR-VIEW-08 and FR-VIEW-09.
**Affects:** FR-VIEW-06.

### OQ-10 · How the user gets around: navigator window or sidebar — decided

Things 3 has a permanent sidebar on the left; taskpaper-terminal, whose look this app follows (D-8), has a Navigator window that a key opens over the panes and no sidebar ([navigation.md §6](reference/taskpaper-terminal/specs/navigation.md#6-navigator-fr-ui-01-d-7-d-11-d-28-d-79)).

- **A. Navigator window,** as in taskpaper-terminal, listing the Inbox, the views and the area and project tree; the panes get the whole width.
- **B. A left sidebar** with the same contents, toggled by a key.

**Proposed default:** A. The home view, where Go Home leads, is the Inbox.
**Decision (2026-10-13):** both: the Navigator window to navigate, and a sidebar on the left, collapsible with a hotkey, to create, rename, delete, indent and outdent, and reorder areas and projects. Recorded in FR-VIEW-07 and FR-VIEW-10.
**Affects:** FR-VIEW-07, FR-NAV-02.

### OQ-11 · How tags are written in the file, and nested tags — decided

TaskPaper tag names allow letters, digits, `_`, `-` and `.`, but no spaces or `/` ([taskpaper.md §3.2](reference/taskpaper.md#32-tag-name-rules)). Things 3 tags can contain spaces and have parent tags.

- **A. Plain TaskPaper tags:** each tag is `@name` on the item's line, with names limited to TaskPaper's characters (a space typed in a name becomes `-`). The tags the app uses itself (`start`, `due`, `done`, `hidden`, `area`, `section` and the Trash tag) are reserved. Nested tags later, written with `.` as the separator (`@work.urgent`).
- **B. One tag holding a list:** `@tags(home, deep work)`, allowing any names, but other TaskPaper apps see one tag.

**Proposed default:** A.
**Decision (2026-10-13):** A, plain `@tags`, and tags never nest, as in Things 3 (FR-TAG-06 is Won't).
**Affects:** FR-TAG, FR-STORE-10.

### OQ-12 · Keys inside the text editor — decided

taskpaper-terminal's edit mode uses standard text-editing keys, not vim's normal mode, while navigate mode is vi-style.

- **A. Standard keys,** as in taskpaper-terminal.
- **B. vim normal and insert modes** inside the editor.

**Proposed default:** A, since key bindings mimic taskpaper-terminal (D-15).
**Decision (2026-10-13):** A, standard keys.
**Affects:** FR-EDIT-04.

### OQ-13 · Times of day — decided

- **A. Days only:** dates are `YYYY-MM-DD`, as in taskpaper-terminal and Things 3's When.
- **B. Optional times,** `YYYY-MM-DD HH:MM`, as in Todoist.

**Proposed default:** A.
**Decision (2026-10-13):** A, days only.
**Affects:** FR-DATE-03, FR-STORE-07.

### OQ-14 · What search covers — decided

- **A.** Everything except the Trash, completed items included and marked.
- **B.** Open items only, with a key to include completed ones.

**Proposed default:** A.
**Decision (2026-10-13):** A, everything except the Trash.
**Affects:** FR-SRCH-03.

### OQ-15 · Which themes — decided

The owner wants 12 to 16 (D-11). taskpaper-terminal has Terminal Dark plus twelve truecolor themes: Catppuccin Mocha and Latte, Tokyo Night, Dracula, Nord, Gruvbox Dark, Rosé Pine and Rosé Pine Dawn, Kanagawa Wave, Everforest Dark, One Dark and Solarized Light ([themes.md](reference/taskpaper-terminal/reference/themes.md)).

- **A.** Those thirteen plus three permissively licensed favorites: GitHub Dark, GitHub Light and Ayu Mirage, for sixteen.
- **B.** Those thirteen only.

**Proposed default:** A.
**Decision (2026-10-13):** neither: research and pick a fresh set of 12 to 16 modern themes, not tied to taskpaper-terminal's ([themes.md](reference/themes.md)). Recorded in FR-UI-03. Later the same day the owner cut v1 to four themes, more to follow (D-22).
**Affects:** FR-UI-03.

### OQ-16 · Which file, and the binary's name — decided

- **A.** The app is run as `tipp [file]`. With a path it opens that file, creating it when missing; without one it opens the file named by the `file` setting, by default `~/textual-items.taskpaper`, creating it with an Inbox when missing. Settings live in `~/.config/textual-items-plus-plus/config.toml`.
- **B.** As A, but the binary is named `textual-items` or another name the owner picks.

**Proposed default:** A.
**Decision (2026-10-13):** A.
**Affects:** FR-STORE-01, FR-SET-01.

### OQ-17 · How a TaskPaper project says whether it is an area, project or section — decided

- **A. A tag:** `@area` marks an area and `@section` a section; a project carries neither. Projects written by other apps are projects.
- **B. Position:** top-level projects are areas and projects without tasks are sections; ambiguous, since projects can sit at the top level (FR-ORG-02).

**Proposed default:** A, which gives files like this:

```
Inbox:
	- Call the bank @due(2026-10-15)
Work: @area
	Everything for the day job.
	Website: @due(2026-11-01)
		Launch: @section
			What has to happen before launch.
			- Write the announcement @start(2026-10-20) @writing
				NOTE @hidden
					Draft in the shared doc.
				COMMENTS @hidden
					[2026-10-13 21:30] Ask design for screenshots
				- Outline
					- Intro
				- First draft @done(2026-10-13)
```

**Decision (2026-10-13):** A, kind tags.

**Affects:** FR-STORE-02.

### OQ-18 · How checklist items are stored — decided

- **A. Nested tasks:** checklist items are TaskPaper tasks nested under their task, done ones tagged `@done`. Since tasks never contain tasks otherwise, any task under a task is a checklist item, and other TaskPaper apps show the checklist naturally. There are no subtasks, as in Things 3.
- **B. A hidden node** `CHECKLIST @hidden` holding the items, invisible as a checklist to other apps.

**Proposed default:** A.
**Decision (2026-10-13):** A, nested tasks.
**Affects:** FR-STORE-06, FR-CHK.

### OQ-19 · How the Trash is stored — decided

- **A. In place:** a deleted item gets `@deleted(YYYY-MM-DD)` and stays where it was, hidden everywhere but the Trash, so restoring it puts it back exactly; emptying the Trash removes its lines. Other TaskPaper apps still show it, tagged.
- **B. Moved:** deleted items move under a hidden `TRASH @hidden` node, remembering their old location in a tag.

**Proposed default:** A.
**Decision (2026-10-13):** A, a tag in place.
**Affects:** FR-VIEW-04, FR-STORE-07.

### OQ-20 · Whether a container's dates affect what is inside it — decided

In Things 3 a project with a future When date takes its to-dos out of Today and Anytime until then.

- **A. No:** a container's dates are its own; Upcoming lists the container itself.
- **B. Yes:** items inside a container that has not started count as not started.

**Proposed default:** A.
**Decision (2026-10-13):** A, a container's dates are its own.
**Affects:** FR-DATE-05, FR-VIEW-02.
