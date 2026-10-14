# Implementation Plan

- Status: Draft
- Last updated: 2026-10-13

The order v1 is built in. Each milestone ends in an app the owner can run and use for real (owner, 2026-10-13), and the plan is shaped so several agents can work at once (owner, 2026-10-13). What each feature does is in the [functional requirements](functional-requirements.md) and [requirements.md](requirements.md); this file only orders the work.

## Milestones and acceptance

Development goes one milestone at a time. Each milestone is one Todoist task labeled `milestone` in the project `textual-items++`, titled like `M1 · A todo list you can use`. Each step is a task of its own, titled like `M1.2 · TaskPaper format`, whose description names its milestone and holds what the step covers, its **After** dependencies and what to try; steps are separate tasks rather than subtasks so each can move through the sections on its own. Agents claim, work and deliver steps as they would any task ([agent workflow](../docs/agent-workflow.md#claims)); a step is runnable once every step in its **After** column is on `main`, and **After M1** means the M1 milestone task is closed. The agent that delivers a milestone's last step asks the owner for the milestone review. When every step is on `main`, the owner runs the [milestone review](../docs/agent-workflow.md#milestone-review), its verified findings are fixed, the milestone task moves to Done and work stops: the owner runs the app, gives feedback and closes the task, which accepts the milestone. Each milestone task names the previous one as a dependency, so it is not runnable until that one is closed. Feedback that changes later steps is recorded here before the next milestone starts.

## Rules for every step

- **Reuse taskpaper-terminal's code.** Where taskpaper-terminal already does the same job, port its code and tests instead of writing new ones, then adapt them to this app's model: the TaskPaper parser and serializer, attachments, dates and the date prompt, saving, backups and external changes, the text field, finders, the Navigator, themes, icons, settings, animations, weather and the shortcuts panel. Ported code meets this repository's rules like any other code, and the task records the source path and commit.
- A step ships its core logic, its commands registered with their key bindings, its entries in the `?` panel and the command palette (once those exist), its tests and any doc updates together.
- A key does nothing until the step that implements its command lands. No step adds a key that only reports "not implemented", and no step leaves a feature reachable before it does something useful.
- **Commands act on the selection.** From step 1.1 a command that acts on items takes the selection, which is just the focused item until step 2.4 lets it hold several. Steps before 2.4 therefore write their commands for the selection and get multi-selection for free.
- **Views register themselves.** Each view is its own module in the view registry of step 1.1; the step that adds a view also adds it to the Navigator and gives it its `g` key (FR-KEY-06), so no view step waits on another.
- A step that adds a user-visible feature also adds content to the demo document ([FR-STORE-01](functional-requirements.md#fr-store--storage), D-29) that shows it, so the owner can try every step by running `tipp` without a path.
- Crates are added in the step that first uses them.
- **Hub steps.** Steps marked **hub** change the files every feature touches: the update function's dispatch, the command registry, the keymap and the main list widget. At most one hub step is in progress at a time; pair it with non-hub steps. Step 1.1 builds extension points so most features add their own module and one registration line rather than branches in a shared `match`.
- **Shared rules.** When two steps that run at the same time share a rule, for example hiding a completed container's contents (1.10) in the Navigator (1.9), or filtering by tags (3.2) in a view (3.3–3.8), the step that lands second applies the rule to what the first built.
- Splitting a step that turns out too large is the implementer's call; merging or reordering steps is the owner's.

## M1 · A todo list you can use

`tipp` opens a file or the demo data; through the Navigator you can browse areas, projects and sections, add, rename and complete tasks, undo, and every change to a file is saved safely. Deleted items stay in the file, tagged, and come back with undo until the Trash view arrives in 3.8.

| # | Step | Covers | After | Try it |
|---|---|---|---|---|
| 1.1 | **Skeleton and extension points. hub** The four layers, Domain, Store, App and UI ([code quality](../docs/code-quality.md#architecture)); `update(state, message) -> effects` and the event loop; terminal setup with the kitty keyboard protocol and its fallback; `Ctrl+Q`; the selection; a command registry (id, title, default keys, context) that builds the keymap and later the palette and `?` panel; a view registry; the shared `TestBackend` test kit. | code-quality Architecture, N-PORT-03, FR-KEY-01 | — | `cargo run` opens an empty screen and quits |
| 1.2 | **TaskPaper format.** Port the parser and serializer into Store: types, tags, indentation, byte-identical round trip, UTF-8 refusal, indent style, line endings, BOM, golden corpus and property tests. No visible change. | F-FILE-02, F-FILE-03, F-FILE-07, F-FILE-08, N-REL-02 | 1.1 | (tests only) |
| 1.3 | **Themes and icons.** The four themes of [themes.md](reference/themes.md) plus Terminal Dark, Nerd Font icons and plain markers; one default theme until settings arrive in 5.5. | FR-UI-03, FR-UI-04, FR-UI-02 (palette) | 1.1 | (tests only; seen in 1.8) |
| 1.4 | **Text field.** Port the one-line and multi-line text editor core: grapheme-aware editing with standard keys (OQ-12). | FR-EDIT-04, N-PORT-04 | 1.1 | (tests only) |
| 1.5 | **Saving pipeline.** Port the text side of saving into Store: atomic writes keeping permissions, debounce, saving on quit and on signals (the signal hook itself lives in the frontend), daily backups, watching the file and the check before each save, writing a recovery file. It takes text and a path, so it does not wait for the model. The policy of when to save (debounce, quitting, signals) is pure decision logic and lives in App as `app::autosave`, since App may not import Store; Store does the disk work. | F-FILE-04, F-FILE-05, F-FILE-06 (text side), F-FILE-09, N-REL-01 (text side), N-UX-01 | 1.1 | (tests only) |
| 1.6 | **Domain model and demo data.** The model in Domain and its mapping to and from the outline in Store: areas, projects, sections and tasks with kind tags (OQ-17), Inbox recognition, guarded descriptions, notes and comments kept as attachments, checklists as nested tasks, unknown content preserved; round trip through the model; the demo document in memory (D-29). | FR-ORG-01–04, FR-ORG-07, FR-STORE-01–10 (model side), FR-TASK-01, FR-ATT-01 (storage), FR-CHK-01 (storage) | 1.2 | (tests only) |
| 1.7 | **Undo engine.** Operations and transactions with apply and invert in the domain, the undo history in the app. No visible change. | FR-EDIT-06 (engine), N-UX-02 | 1.6 | (tests only) |
| 1.8 | **Container view. hub** A container drawn as FR-VIEW-05 arranges it: task rows with icons and tags, inline descriptions, section headings; focus, vi motions, scrolling and the status line; 80×24. `tipp <file>` shows the file read-only until 1.12, and commands that would change it say so in the status line. | FR-VIEW-05, FR-ORG-05 (display), FR-ORG-08, FR-TASK-03 (title, tags, icons; date chips in 3.1, checklist progress in 4.3), FR-ORG-10 (display), FR-TAG-03, FR-NAV-01, FR-UI-01, FR-UI-02, FR-UI-05 (wrap on), FR-UI-06 | 1.3, 1.6 | `tipp` shows the demo Inbox and you can move through it |
| 1.9 | **Navigator, opening and history. hub** Port the Navigator window over the panes, with filter-as-you-type, listing the Inbox and every area and project (later views add themselves); open a container, go up, home is the Inbox, breadcrumbs, Go Back and Go Forward. | FR-VIEW-07, FR-NAV-02, FR-NAV-03, FR-NAV-04 | 1.8, 1.13 | Open Work › Website from the Navigator and come back |
| 1.10 | **Completing and undo.** `x` completes and uncompletes the selection, containers included; completed items go last and hide until `Ctrl+O`; a completed container's contents are hidden everywhere. Binds undo and redo as taskpaper-terminal does. | FR-TASK-02, FR-ORG-09, FR-VIEW-11, FR-EDIT-06 | 1.7, 1.8 | Check off a task, show it with `Ctrl+O`, undo and redo |
| 1.11 | **Editing tasks. hub** Edit a title in place with `Enter`, add a task below or above with `o`/`O`, delete the selection to the Trash (OQ-19). | FR-EDIT-01, FR-EDIT-02, FR-EDIT-05 (tasks), FR-OPS-05 (delete), FR-TASK-01 | 1.4, 1.7, 1.9 | Add and rename a task |
| 1.12 | **Saving edits and outside changes.** Every change to an opened file is saved through 1.5, and the demo data is never saved, which the status line says. A file changed by another program reloads into the model with each pane's focus matched again and the undo history cleared, and a conflict gets taskpaper-terminal's Keep mine / Take theirs / Show diff prompt. The panic hook writes the current document as the recovery file and offers it on the next start. | FR-STORE-09, F-FILE-01, F-FILE-05, F-FILE-06, N-REL-01 | 1.5, 1.9, 1.10 | Complete a task in a real file, then edit it in another editor while the app is open |
| 1.13 | **Fuzzy picker.** The shared filter-as-you-type modal list (port the finder). No visible change on its own. | FR-UI-09 | 1.1, 1.4 | (tests only) |

**Parallel:** after 1.1, steps 1.2, 1.3, 1.4 and 1.5 run at once, and 1.13 once 1.4 is in. After 1.6, steps 1.7 and 1.8. After 1.8 and 1.13, step 1.9 (hub) beside 1.10; after 1.9, step 1.11 (hub) beside 1.12.

## M2 · Organize like Things 3

Build and rearrange the whole hierarchy from the keyboard.

| # | Step | Covers | After | Try it |
|---|---|---|---|---|
| 2.1 | **Containers. hub** Create, rename and delete areas, projects and sections, opening the new one in edit mode. | FR-ORG-06 (create, rename, delete), FR-EDIT-05 | M1 | Add an area with a project and a section |
| 2.2 | **Descriptions.** Edit any item's description in place, Markdown text in a multi-line field. | FR-ORG-10, FR-ORG-05, FR-EDIT-03 (description) | M1 | Describe a project |
| 2.3 | **Moving and indenting.** `Alt` with a motion moves the selection, crossing sections naturally; `[`/`]` and `Tab`/`Shift+Tab` indent and outdent where the hierarchy allows it. | FR-OPS-02, FR-OPS-03, FR-OPS-10, FR-TASK-05, FR-ORG-06 (reorder) | M1 | Move a task from one section into the next |
| 2.4 | **Multi-selection.** `Shift` with a motion extends the selection; every command written for the selection now acts on all of it. | FR-OPS-01, FR-OPS-06 | 2.3 | Complete three tasks at once |
| 2.5 | **Move to.** Move the selection to any container, sections included, with the picker. | FR-OPS-04, FR-ORG-06 (move) | M1 | Move a task to another project |
| 2.6 | **Sidebar.** A hotkey shows and hides the left sidebar with the area and project tree; in it, create, rename, delete, indent and outdent, and reorder, through the commands of 2.1 and 2.3. | FR-VIEW-10 | 2.1, 2.3 | Reorder projects in the sidebar |
| 2.7 | **Converting and grouping.** Convert a task to a project, an area to a project, a section to a project and back; group tasks into a section, and tasks and sections into a project; refused moves get a message. | FR-TASK-06, FR-ORG-06 (convert), FR-OPS-07, FR-OPS-08 | 2.1, 2.4 | Group three tasks into a section |
| 2.8 | **Clipboard.** Duplicate, copy, cut and paste items with their contents; pasting several lines makes several items. | FR-OPS-05 | 2.1 | Paste a list from the system clipboard |

**Parallel:** at the start, 2.1 (hub) runs beside 2.2, 2.3 and 2.5. After 2.3: 2.4. After 2.1 and 2.3: 2.6. After 2.1: 2.8. After 2.1 and 2.4: 2.7.

## M3 · Dates, tags and views

Plan your days as in Things 3.

| # | Step | Covers | After | Try it |
|---|---|---|---|---|
| 3.1 | **Dates.** Port the date grammar and date prompt with natural-language dates; `ms`, `md`, `mt` on the selection; date chips on every item's row; overdue highlighting. | FR-DATE-01–05, FR-KEY-06 (`m` keys), FR-OPS-06 (dates) | M2 | `md` then `fri` |
| 3.2 | **Tags.** The tag picker, creating tags as you type, on the selection; filtering a view or container by tags. | FR-TAG-01, FR-TAG-02, FR-TAG-04 (views and containers), FR-OPS-06 (tags) | M2 | Tag two tasks and filter by the tag |
| 3.3 | **Inbox view.** Scheduled tasks leave the Inbox; `gi`. | FR-VIEW-01, FR-KEY-06 | M2 | The demo's scheduled Inbox task is not in the Inbox view |
| 3.4 | **Today.** Open items whose start or due date has come, overdue first; `gt`. | FR-VIEW-06 | M2 | Open Today |
| 3.5 | **Upcoming.** Items with future dates, grouped by day and then month. | FR-VIEW-02 | M2 | Open Upcoming |
| 3.6 | **All.** Every open task, grouped by where it lives. | FR-VIEW-08 | M2 | Open All |
| 3.7 | **Logbook.** Every completed item, newest first, grouped by completion date; uncompleting from it. | FR-VIEW-03 | M2 | Uncomplete a task from the Logbook |
| 3.8 | **Trash.** Deleted items, newest first, restore and empty. | FR-VIEW-04 | M2 | Restore a deleted task |

**Parallel:** all eight run at once; views read dates from the file (1.6), so they do not wait for 3.1. Each view is its own module, so view steps do not collide.

## M4 · Notes, comments, checklists and panes

The right-hand attachments panel, copied from taskpaper-terminal, and side-by-side panes.

| # | Step | Covers | After | Try it |
|---|---|---|---|---|
| 4.1 | **Attachments panel with notes.** Port the modal full-height panel on the right, opened per item; its notes section, rendered as Markdown and edited as raw text, for any item; each change one undo step and saved. | FR-ATT-01, FR-ATT-02 (notes key), FR-ATT-03, FR-ATT-05, FR-TASK-04, FR-EDIT-03 | M3 | Write a note on a project |
| 4.2 | **Comments.** The comments section and its key: timestamped one-line comments, newest first, added, edited and deleted. | FR-ATT-04, FR-ATT-02 (comments key) | 4.1 | Comment on a task |
| 4.3 | **Checklists.** The checklist section and its key: a task's nested checklist with the same keys as elsewhere; a checked item dims its subtree; progress on the row. | FR-CHK-01–04, FR-ATT-02 (checklist key) | 4.1 | Build a nested checklist |
| 4.4 | **Panes. hub** Panes side by side with their own focus and history; open the focused item or a Navigator entry in a new pane; focus, close and resize panes; scroll sideways when panes do not fit. | FR-PANE-01, FR-PANE-02 (items, Navigator) | M3 | Open Today beside a project |
| 4.5 | **Layout restore.** The panes and what they show come back on the next launch. | FR-PANE-03 | 4.4 | Quit and reopen |

**Parallel:** 4.1 and 4.4 (hub) at the start; after 4.1, steps 4.2 and 4.3; after 4.4, step 4.5.

## M5 · Find, palettes and settings

Find anything fast and make the app yours.

| # | Step | Covers | After | Try it |
|---|---|---|---|---|
| 5.1 | **Command palette.** Every registered command with its keys, filtered as you type. | FR-NAV-06 | M4 | `Ctrl+P`, type `move` |
| 5.2 | **Go to Symbol.** Any project, section or task by fuzzy match, opened here or in a new pane. | FR-NAV-05, FR-PANE-02 (palette) | M4 | `/`, type a task |
| 5.3 | **Full-text search.** Titles, descriptions, notes, comments, tags and checklists; completed marked, Trash left out; results open where the item lives or in a new pane, and can be filtered by tags. | FR-SRCH-01–03, FR-PANE-02 (search), FR-TAG-04 (search) | M4 | Search a word that is only in a comment |
| 5.4 | **Shortcuts panel.** `?` shows the bindings for the current context, from the registry. | FR-KEY-03, FR-KEY-02 | M4 | Press `?` |
| 5.5 | **Settings.** Port the TOML config that keeps comments and unknown keys, and the settings window with live preview: theme, icons, word wrap. | FR-SET-01–04, FR-UI-03 (switching), FR-UI-04 (toggle), FR-UI-05 | M4 | Switch to Flexoki Light |

**Parallel:** all five run at once; none is a hub, since each adds its own window and registers its commands.

## M6 · Motion and performance

| # | Step | Covers | After | Try it |
|---|---|---|---|---|
| 6.1 | **Animations.** Port the animation layer: focus, opening and closing, panels, panes and palettes, completing and deleting, refusals; never delaying input; its setting. | FR-UI-07 | M5 | Complete a task and watch it fade |
| 6.2 | **Weather.** Port the idle weather animation, its settings and its palette command. | FR-UI-08, N-PERF-03 | M5 | Leave the app idle |
| 6.3 | **Performance and portability.** The 50,000-item benchmark, keystroke budget, idle CPU and terminal checks. | N-PERF-01–03, N-PORT-01–04 | M5 | `validate.sh bench` |
| 6.4 | **Mouse.** Mouse capture with its terminal setup and teardown; clicking focuses an item, toggles its completion, opens a crumb or a Navigator entry; the wheel scrolls panes, panels and pickers. Clicks become the same messages keys send, so the core stays free of mouse events. Moved up from Later by the owner (2026-10-14). | FR-KEY-05 | M5 | Click a task's checkbox |

**Parallel:** all four run at once.

## Later

After v1, unless the owner moves them up:

- **Should:** Quick Add (FR-EDIT-07), renaming and deleting tags (FR-TAG-05), custom key bindings (FR-KEY-04), `NO_COLOR` and 16-color themes (N-UX-03), `cargo install` (N-DIST-01).
- **Later:** repeating tasks (FR-TASK-07), search operators and saved searches (FR-SRCH-04), more themes (FR-UI-10).

The other Should rows, F-FILE-08 and N-PORT-03, are already in steps 1.2 and 1.1.

## What can run in parallel

| Milestone | Most steps at once | Hub steps |
|---|---|---|
| M1 | 4 (1.2, 1.3, 1.4, 1.5, then 1.13 once 1.4 lands) | 1.1, 1.8, 1.9, 1.11 |
| M2 | 5 (2.2, 2.5, 2.6, 2.7, 2.8, once 2.1, 2.3 and 2.4 land) | 2.1 |
| M3 | 8 (3.1–3.8) | none |
| M4 | 3 (4.2, 4.3, 4.4 or 4.5) | 4.4 |
| M5 | 5 (5.1–5.5) | none |
| M6 | 4 (6.1–6.4) | none |

M3 and M4's attachment steps (4.1–4.3) touch separate parts of the app, so the owner could let them run as one wave after M2.
