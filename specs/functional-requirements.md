# Functional Requirements

- Status: Draft
- Last updated: 2026-10-13

What textual-items++ does, as stated by the owner on 2026-10-13. The owner's answers are recorded under [Decisions](#decisions) and referenced as **(D-n)**; the questions in [open-questions.md](open-questions.md), referenced as **(OQ-n)**, were all decided on 2026-10-13, and each records its decision there.

Priorities: **Must** is required for v1, **Should** is expected soon after, **Later** is planned after v1 and must not be precluded by the design, **Won't** is out of scope.

Requirement IDs have the form `FR-<AREA>-<nn>` and are stable: never renumber or reuse one; mark a dropped requirement **Won't** instead. `bash app/scripts/validate.sh trace` reads the Must rows of these tables ([testing strategy](../docs/testing-strategy.md#traceability)).

References: [Things 3](reference/things3.md), [Todoist](reference/todoist.md), [TaskPaper](reference/taskpaper.md), [Workflowy](reference/workflowy.md), [TUI best practices](reference/tui-best-practices.md) and the owner's earlier app, [taskpaper-terminal](reference/taskpaper-terminal/README.md), whose setups this app copies where a requirement says so.

## Product summary

textual-items++ is a keyboard-first todo app for the terminal, in the spirit of Things 3 and Todoist, for software engineers and other people who live in a terminal. Work is organized as areas, projects, sections and tasks, as in Things 3, with three changes: projects nest, areas can hold sections, and sections have descriptions (D-2). Titles and dates are edited in place, as in Things 3; a task's description, checklist and comments open in a full-height panel on the right, as in taskpaper-terminal (D-5, OQ-6). Everything is stored in one plain-text `.taskpaper` file (D-6). The look, keys and settings follow taskpaper-terminal (D-8).

## Terms

- **Area**: a top-level area of responsibility, such as Work or Home, that groups projects, sections and tasks (FR-ORG).
- **Project**: a goal made of tasks; it can sit in an area or at the top level and can contain other projects (FR-ORG).
- **Section**: a titled group of tasks inside an area or a project, like a heading in Things 3 (FR-ORG).
- **Task**: one thing to do, with a title and optional description, start date, due date, checklist and comments (FR-TASK).
- **Container**: an area, project or section, or the Inbox: anything a task can be in.
- **Item**: an area, project, section or task.
- **Checklist item**: one line of a task's checklist; checklist items nest to any depth (FR-CHK).
- **Description**: Markdown text attached to an item: shown under an area, project or section, and in the attachments panel for a task.
- **Comment**: a dated one-line remark on a task, as in Todoist but for one user (FR-ATT).
- **Inbox**: the built-in place for tasks not yet organized (FR-ORG-07).
- **View**: a list the app builds, such as Inbox, Today, Upcoming, All, Logbook, Trash or search results, as opposed to a container shown as itself.
- **Pane**: one column of the split view, showing its own view or container (FR-PANE).
- **Navigate mode** and **edit mode**: in navigate mode keys act on items; in edit mode keys edit the text of one item in place (FR-EDIT).
- **Start date**: the day an item becomes relevant, Things 3's "When". **Due date**: the day it must be done, Things 3's "Deadline". Any item can have either or both (D-13).
- **Tag**: a label the user attaches to any item, as in Things 3 (D-12).

## FR-ORG · Organization

| ID | Requirement | Priority |
|---|---|---|
| FR-ORG-01 | Areas sit at the top level and contain projects, sections and tasks, in an order the user sets. Areas do not nest (OQ-1). | Must |
| FR-ORG-02 | Projects sit in an area or at the top level, and contain sections, tasks and other projects, nested to any depth, as in Todoist (D-2). | Must |
| FR-ORG-03 | Sections sit in an area or a project, unlike Things 3, where areas cannot have headings, and contain tasks (D-2). | Must |
| FR-ORG-04 | Tasks sit in the Inbox, an area, a project or a section. | Must |
| FR-ORG-05 | Every area, project and section has a title, an optional start date, due date and tags (FR-DATE, FR-TAG), and an optional description, Markdown text of any length shown under its title when it is open; sections have descriptions, unlike Things 3 (D-2). | Must |
| FR-ORG-06 | The user creates, renames, reorders, moves and deletes areas, projects and sections. Moving or deleting one carries everything inside it. An area can be turned into a project, but a project can't be turned into an area (D-25); a section can be turned into a project and back, keeping their contents where the target allows them (OQ-2). | Must |
| FR-ORG-07 | The Inbox is a built-in container, always first, where tasks go when no other place is chosen, as in Things 3. It holds tasks only (OQ-3). | Must |
| FR-ORG-08 | Items keep the order the user gives them everywhere; nothing is sorted automatically except in views that say so. | Must |
| FR-ORG-09 | Any item can be completed, an area, project or section as well as a task. Completing an item leaves everything inside it in its own state, uncompleted items included. The completed item itself is shown like any other completed item (FR-VIEW-03, FR-VIEW-11), but everything inside it, completed or not, is no longer shown anywhere in the app: not in any view, the Logbook and search included, nor in palettes, pickers, the Navigator, the sidebar or the item opened as a container. Uncompleting the item shows it all again (OQ-4). | Must |

## FR-TASK · Tasks

| ID | Requirement | Priority |
|---|---|---|
| FR-TASK-01 | A task has a title, and optionally a description, a start date, a due date, tags, a checklist and comments (D-1, D-12, D-13). | Must |
| FR-TASK-02 | A task is completed and uncompleted with one key; completing records the date. A completed item never moves in the file: it stays where it is, dimmed and struck through, and the Logbook only collects it (OQ-5); lists show it after the uncompleted items (FR-VIEW-11). | Must |
| FR-TASK-03 | A task row shows its title and, compactly, its start and due dates as date chips (FR-DATE-02), its tags, its checklist progress (`2/5`) and icons for a description and comments, as taskpaper-terminal does ([ui.md §2](reference/taskpaper-terminal/specs/ui.md#2-blocks)). | Must |
| FR-TASK-04 | A task's description is Markdown text of any length, rendered in the attachments panel and edited there as raw text (FR-ATT, D-5). | Must |
| FR-TASK-05 | Tasks are reordered and moved to another container like every item (FR-OPS). | Must |
| FR-TASK-06 | A task can be converted to a project, in its place: its title, description, dates and tags carry over, its checklist items become the project's tasks, nested ones as their checklists, and its comments are appended to the project's description under a Comments heading, since projects have none, so nothing is lost (D-25). Where a project can't go, in a section or the Inbox, the conversion is refused with a message (OQ-2). Converting a project back to a task is not required. | Must |
| FR-TASK-07 | Repeating tasks, as in Things 3 and Todoist. | Later |

## FR-CHK · Checklists

| ID | Requirement | Priority |
|---|---|---|
| FR-CHK-01 | A task's checklist is a list of items, each with text and a done state, nested to any depth, unlike Things 3, whose checklists are flat (D-1). | Must |
| FR-CHK-02 | The checklist is shown and edited in the attachments panel, in a Checklist section between the description and the comments (FR-ATT, OQ-6). | Must |
| FR-CHK-03 | Checklist items are added, edited, deleted, checked and unchecked, reordered, indented and outdented with the same keys as items elsewhere. Pasting several lines adds one item per line, keeping their indentation as nesting. | Must |
| FR-CHK-04 | Checking an item does not check or complete anything else: its children, its parent and the task keep their own state. A checked item and everything under it, at any depth, are drawn dimmed, as taskpaper-terminal draws a done block and its contents (OQ-7). | Must |

## FR-ATT · Descriptions, checklists and comments of tasks

These copy taskpaper-terminal's attachments setup ([attachments.md](reference/taskpaper-terminal/specs/attachments.md)), which has exactly the layout the owner wants (D-5).

| ID | Requirement | Priority |
|---|---|---|
| FR-ATT-01 | A task's description, checklist and comments are read and edited in a modal panel over the right side of the screen, from the top of the screen to the status line, showing the description, below it the checklist and below that the comments, as taskpaper-terminal's attachments panel does. This is the one place the app intentionally differs from Things 3 and Todoist (D-5, OQ-6). | Must |
| FR-ATT-02 | The panel opens with one key each for the description, the checklist and the comments, from the task's row, and closes with `Esc`. | Must |
| FR-ATT-03 | The description is rendered as Markdown in the panel and edited as raw text in a multi-line editor there (D-5). | Must |
| FR-ATT-04 | Comments are single lines, each stamped with its local date and time, listed newest first; the user adds, edits and deletes them. There is one user, so comments carry no author (D-1). | Must |
| FR-ATT-05 | Every change made in the panel is one undo step and is saved like any edit. | Must |

## FR-VIEW · Views

| ID | Requirement | Priority |
|---|---|---|
| FR-VIEW-01 | Inbox: the tasks in the Inbox that have no start date, in their order. Giving an Inbox task a start date takes it out of the Inbox view, as scheduling does in Things 3; it stays under the Inbox in the file and shows in Today, Upcoming and All (D-3, OQ-9). | Must |
| FR-VIEW-02 | Upcoming: every open item with a start or due date after today, tasks and also areas, projects and sections (D-13), grouped by day, the next days one by one and later dates by month, as in Things 3 (D-3, OQ-8). | Must |
| FR-VIEW-03 | Logbook: a view of every completed item from every list, newest first, grouped by completion date, each with its location. It moves nothing; uncompleting an item there uncompletes it where it is (D-3, OQ-5). | Must |
| FR-VIEW-04 | Trash: deleted items, newest first, each with its location. An item in the Trash can be restored to where it was, or deleted for good; Empty Trash deletes everything in it for good after confirmation (D-3). | Must |
| FR-VIEW-05 | Opening an area, project or section shows its description and its contents in order: sections as headings with their tasks below, and, for areas and projects, their child projects, each as one row; a child project's tasks and sections are not shown there, only when that project is opened (D-2, D-26). | Must |
| FR-VIEW-06 | Today: every open item whose start date or due date is today or earlier, grouped by area and project, overdue ones first, as in Things 3 (OQ-9). | Must |
| FR-VIEW-07 | A Navigator window, opened by a key over the panes as in taskpaper-terminal, lists the Inbox, Today, Upcoming, All, Logbook and Trash and every area and project as a tree, with counts of open tasks, filters as the user types, and opens the chosen entry in the focused pane ([navigation.md §6](reference/taskpaper-terminal/specs/navigation.md#6-navigator-fr-ui-01-d-7-d-11-d-28-d-79), OQ-10). | Must |
| FR-VIEW-08 | All: every open task in every list, grouped by where it lives, in place of Things 3's Anytime and Someday; this app has no Anytime/Someday distinction (OQ-9). | Must |
| FR-VIEW-09 | Anytime and Someday views (OQ-9). | Won't |
| FR-VIEW-10 | A sidebar on the left, shown and hidden with a hotkey, lists the areas and projects as a tree and is where they are created, renamed, deleted, indented and outdented and reordered with keys (OQ-10). | Must |
| FR-VIEW-11 | In every list, completed items come after the uncompleted ones within each area, project and section, without changing their order in the file. They are hidden, except in the Logbook and search results (FR-SRCH-03), until **Show Completed Items** (`Ctrl+O`, as in taskpaper-terminal) shows them, per pane; the same key hides them again. The Logbook always shows them. Checklists are not lists of items: checked checklist items stay in place and shown, dimmed (FR-CHK-04) (D-21). | Must |

## FR-EDIT · Editing

| ID | Requirement | Priority |
|---|---|---|
| FR-EDIT-01 | Navigate mode moves between items and acts on them; edit mode edits the text of one item. `Enter` or a vi-style key starts editing, `Esc` returns, as in taskpaper-terminal (D-4, D-8). | Must |
| FR-EDIT-02 | Editing happens in place: the editor opens where the item is, keeping the surrounding items and the scroll position, as in Things 3 (D-4). | Must |
| FR-EDIT-03 | A task's title and dates are edited in place; its description, checklist and comments open in the attachments panel instead (D-4, D-5, OQ-6). | Must |
| FR-EDIT-04 | The text editor uses standard text-editing keys (arrows, Home/End, word motion and deletion, selection, clipboard), as taskpaper-terminal's edit mode does, not vim's normal mode (D-8, OQ-12). | Must |
| FR-EDIT-05 | Creating an item opens it directly in edit mode: a new task below or above the focused one, a new section, project or area, each from one key or a short key sequence. | Must |
| FR-EDIT-06 | Undo and redo every change, structural ones included. | Must |
| FR-EDIT-07 | Quick Add: a key opens a one-line prompt that adds a task to a chosen container, the Inbox by default, without leaving the current view, recognizing dates typed in natural language (FR-DATE). | Should |

## FR-OPS · Selection and rearranging

These copy taskpaper-terminal's selection and structural operations ([editing.md §4](reference/taskpaper-terminal/specs/editing.md#4-structural-operations-fr-ops), [navigation.md §2.1](reference/taskpaper-terminal/specs/navigation.md#21-selection-fr-ops-01), D-18).

| ID | Requirement | Priority |
|---|---|---|
| FR-OPS-01 | Several items are selected by extending the selection with `Shift` and a motion (`Shift+↓`/`Shift+↑`, `Shift+j`/`Shift+k`) (D-24), or by toggling items one by one, as in taskpaper-terminal (D-18). | Must |
| FR-OPS-02 | The focused item or the selection moves up and down among its siblings with `Alt` and a motion (`Alt+↓`/`Alt+↑`, `Alt+j`/`Alt+k`) (D-24), crossing into the neighboring container at the edges, carrying everything inside it (D-18). | Must |
| FR-OPS-03 | The focused item or the selection is indented with `]` and outdented with `[` (D-24), beside taskpaper-terminal's `Tab`/`Shift+Tab`, where the hierarchy allows it: a project into or out of the project above it, a checklist item under or out of the one above it; a move the hierarchy does not allow is refused with a short message (D-18, FR-ORG). | Must |
| FR-OPS-04 | The focused item or the selection moves to another container chosen in a fuzzy picker (Move to), as in taskpaper-terminal (D-18). | Must |
| FR-OPS-05 | The selection is deleted, duplicated, copied, cut and pasted with its contents; pasted text with several lines becomes several items, as in taskpaper-terminal (D-18). | Must |
| FR-OPS-06 | Completing, setting dates, tagging and deleting apply to the whole selection at once (D-18). | Must |
| FR-OPS-07 | Group into section: the selected tasks become the tasks of a new section created where the first of them was, in the same area or project, whose title is then edited in place (D-25); it is refused with a message for tasks already in a section or in the Inbox, where a section can't go (OQ-2). | Must |
| FR-OPS-08 | Group into project: the selected tasks and sections become the contents of a new project created where the first of them was, whose title is then edited in place (D-25); it is refused with a message where a project can't go, in a section or the Inbox (OQ-2). | Must |
| FR-OPS-09 | Converting a project to an area: no command does it (D-25). | Won't |

## FR-DATE · Dates

| ID | Requirement | Priority |
|---|---|---|
| FR-DATE-01 | Any item, an area, project, section or task, can have a start date, a due date, both or neither (D-13). | Must |
| FR-DATE-02 | Start and due dates are set and shown as in taskpaper-terminal (D-14): a date prompt, opened by a key for each date (`ms` and `md` there), takes natural language such as `today`, `fri`, `next fri`, `weekend`, `next month`, `+3d`, `2 weeks` or `2026-10-20`, as in Todoist and Things 3, previews the resolved date as it is typed and clears the date when empty ([search.md §4](reference/taskpaper-terminal/specs/search.md#4-dates-must-fr-tag-03-d-6), [ui.md §1](reference/taskpaper-terminal/specs/ui.md#1-layout), D-7); on a row, dates are right-aligned chips, start in green and due in red, named relative to today, an overdue one bold and reversed ([ui.md §2](reference/taskpaper-terminal/specs/ui.md#2-blocks)). | Must |
| FR-DATE-03 | Dates are calendar days in the local time zone, without a time of day (OQ-13). | Must |
| FR-DATE-04 | A due date before today is overdue and highlighted wherever the item is shown, until the item is completed. | Must |
| FR-DATE-05 | An area, project or section shows its date chips on its title row, as a task row does. A date on a container does not change the dates of what is inside it (OQ-20). | Must |

## FR-TAG · Tags

| ID | Requirement | Priority |
|---|---|---|
| FR-TAG-01 | Any item, an area, project, section or task, can carry any number of tags, as in Things 3 (D-12). | Must |
| FR-TAG-02 | Tags are added and removed from a picker that filters existing tags as the user types and creates a new one from the typed text; the same picker works on several selected items. | Must |
| FR-TAG-03 | Tags are shown on the item's row, compactly and in a dimmed style, after its title. | Must |
| FR-TAG-04 | The current view, container or search results can be filtered to items carrying chosen tags, as Things 3 filters by tag. | Must |
| FR-TAG-05 | Tags can be renamed and deleted everywhere at once from a list of all tags. | Should |
| FR-TAG-06 | Nested tags, as Things 3's parent tags; tags never nest here (OQ-11). | Won't |

## FR-NAV · Navigation

| ID | Requirement | Priority |
|---|---|---|
| FR-NAV-01 | vi-style motions move between items: next and previous, first and last, parent container, next and previous section; arrow keys work too (D-4). | Must |
| FR-NAV-02 | Opening a container or view shows it in the pane, like zooming in Workflowy; a key goes up to the parent container and another to the home view, the Inbox (OQ-10). | Must |
| FR-NAV-03 | Breadcrumbs at the top of each pane show the path to what it shows, such as `Work › Website › Launch`, as in Workflowy; a crumb can be opened by key or click (D-9). | Must |
| FR-NAV-04 | Go Back and Go Forward move through a history of where the user has been, like a browser's, each pane with its own history, as Workflowy's panes have (D-10). | Must |
| FR-NAV-05 | Go to Symbol: a palette that finds any area, project, section or task by fuzzy matching, filtering as the user types, and opens it in the focused pane, like VS Code's Go to Symbol (D-4). | Must |
| FR-NAV-06 | Command palette: a palette that lists every command with its key binding and filters as the user types, like VS Code's, and runs the chosen command (D-4). | Must |

## FR-SRCH · Search

| ID | Requirement | Priority |
|---|---|---|
| FR-SRCH-01 | A full-text search view finds the typed words in the titles, descriptions and tags of every item, in checklist items and in comments, updating as the user types, and lists the matching items with their location and the matching text highlighted (D-4). | Must |
| FR-SRCH-02 | Choosing a result opens it where it lives; results can be opened in a new pane. | Must |
| FR-SRCH-03 | Completed items are included and marked as such; items in the Trash are left out (OQ-14). | Must |
| FR-SRCH-04 | Search operators, such as `due:`, `in:` or `is:done`, and saved searches. | Later |

## FR-PANE · Panes

Panes copy taskpaper-terminal's, which follow Workflowy's ([navigation.md §5](reference/taskpaper-terminal/specs/navigation.md#5-panes-fr-ui-02--fr-ui-04-d-13), [Workflowy](reference/workflowy.md)).

| ID | Requirement | Priority |
|---|---|---|
| FR-PANE-01 | Any number of panes side by side as columns, each showing its own container or view, with its own breadcrumbs, history and focus; an edit in one pane appears in every pane at once (D-4). | Must |
| FR-PANE-02 | Open the focused item, or a palette or search result, in a new pane next to the focused one; focus a pane by number or move between panes; close a pane; resize panes. When panes do not fit, the view scrolls sideways. | Must |
| FR-PANE-03 | The pane layout and what each pane shows are restored on the next launch. | Must |

## FR-KEY · Keyboard and mouse

| ID | Requirement | Priority |
|---|---|---|
| FR-KEY-01 | Every feature is reachable from the keyboard alone (D-4). | Must |
| FR-KEY-02 | Key bindings mimic taskpaper-terminal's wherever the same action exists there, so the same keys do the same things in both apps ([keybindings.md](reference/taskpaper-terminal/specs/keybindings.md), [key-mnemonics.md](reference/taskpaper-terminal/key-mnemonics.md)); new actions get vi-style bindings in the same scheme (`j`/`k`, `gg`/`G`, prefix keys). Every binding works in common terminals on macOS and Linux, with no `Cmd`/`Super` binding ([keybindings.md §1](reference/taskpaper-terminal/specs/keybindings.md#1-terminal-constraints), D-4, D-15). | Must |
| FR-KEY-03 | A keyboard shortcuts panel, toggled with `?`, lists the most useful bindings for the current mode and context in a sidebar on the right, grouped, scrollable and showing the user's own bindings, as in taskpaper-terminal ([ui.md §4](reference/taskpaper-terminal/specs/ui.md#4-shortcuts-panel-fr-key-04), D-17). | Must |
| FR-KEY-04 | Key bindings can be changed in the settings file. | Should |
| FR-KEY-05 | The mouse can focus items, open crumbs and navigator entries, toggle completion and scroll; no feature depends on it. | Should |
| FR-KEY-06 | Two prefix families, as in taskpaper-terminal's mnemonics: `g` then a letter **g**oes somewhere, a view, place or palette, such as `gi` Inbox and `gt` Today; `m` then a letter **m**odifies the focused item or the selection, such as `ms` set the start date, `md` set the due date and `mt` start today (start date set to today). Every view and every item property gets a key in its family; where a letter is already taken in taskpaper-terminal, the keybindings spec picks the nearest free mnemonic (D-23). | Must |

## FR-UI · Look and feel

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-01 | The UI is modern, sleek, intuitive and fast, following the [TUI best practices](reference/tui-best-practices.md) (D-4). | Must |
| FR-UI-02 | The look follows taskpaper-terminal: its layout, typography by color, weight and dimming, focus marker and current-line band, breadcrumbs, status line, dialogs that dim the screen behind them, and its icons ([ui.md](reference/taskpaper-terminal/specs/ui.md), D-8). | Must |
| FR-UI-03 | Four built-in color themes for v1, two dark and two light, a fresh selection of modern, well-made light and dark themes ([themes.md](reference/themes.md)), switched with a live preview, plus a theme that draws in the terminal's own colors, like taskpaper-terminal's Terminal Dark (D-11, OQ-15, D-22). | Must |
| FR-UI-04 | Nerd Font icons mark item kinds, dates, attachments, views and the status line; one setting turns them off, for plain Unicode markers, as in taskpaper-terminal (D-11). | Must |
| FR-UI-05 | Word wrap is a setting: on, long titles wrap; off, every item takes one row and long text ends in `…` (D-11). | Must |
| FR-UI-06 | The app works in an 80×24 terminal and degrades gracefully below it. | Must |
| FR-UI-07 | Modern, smooth and fluid animations show where focus landed, what a command changed and when an action was refused: moving focus, opening and closing items, panes, palettes and the attachments panel, completing and deleting. They follow taskpaper-terminal's rules: they never delay input or layout, run only while something moves, and a setting reduces or turns them off ([ui.md §9](reference/taskpaper-terminal/specs/ui.md#9-animation-fr-ui-11), [tui-animation.md](reference/taskpaper-terminal/reference/tui-animation.md), D-16). | Must |
| FR-UI-08 | Weather: after the app has had no input for a few minutes, a full-screen animation of the season's weather plays over it until the next key or click, as in taskpaper-terminal: autumn leaves, winter snow or spring rain. A setting, on by default, turns it off, another sets the idle minutes, and the command palette starts it at once ([ui.md §10](reference/taskpaper-terminal/specs/ui.md#10-weather-fr-ui-12), D-19). | Must |
| FR-UI-09 | Every modal window that chooses an item, such as Move to, the tag picker, a parent or Go to Symbol, filters its list as the user types, as taskpaper-terminal's finders do: typed text narrows the list by fuzzy match, the first match is focused, arrows and `Ctrl+P`/`Ctrl+N` move, `Enter` chooses and `Esc` closes (D-20). | Must |
| FR-UI-10 | More built-in themes, up to the 12 to 16 of D-11 (D-22). | Later |

## FR-SET · Settings

| ID | Requirement | Priority |
|---|---|---|
| FR-SET-01 | Settings are stored in one TOML file, `~/.config/textual-items-plus-plus/config.toml` (`$XDG_CONFIG_HOME` respected, also on macOS), readable and editable by hand; the app runs with defaults when it is missing, as in taskpaper-terminal (D-11, OQ-16). | Must |
| FR-SET-02 | A settings window, opened by a key and from the command palette, lists every setting grouped by topic, with its value and a one-line description; a change applies at once with a live preview and is saved immediately, as in taskpaper-terminal ([ui.md §8](reference/taskpaper-terminal/specs/ui.md#8-settings-window-fr-set-02-fr-set-03), D-11). | Must |
| FR-SET-03 | The settings include at least the theme (FR-UI-03), icons (FR-UI-04) and word wrap (FR-UI-05) (D-11). | Must |
| FR-SET-04 | Writing a setting keeps the file's comments, order and unknown keys; invalid values are reported and ignored, and the app still starts. | Must |

## FR-STORE · Storage

| ID | Requirement | Priority |
|---|---|---|
| FR-STORE-01 | All data is stored in one `.taskpaper` file, the source of truth, readable and editable in any text editor and in TaskPaper ([reference](reference/taskpaper.md), D-6). The app runs as `tipp [file]`: it opens the given file, or the one the `file` setting names, by default `~/textual-items.taskpaper`, creating it when missing (OQ-16). | Must |
| FR-STORE-02 | Areas, projects and sections are TaskPaper projects; which of the three a project is, is marked with a tag (OQ-17) (D-6). | Must |
| FR-STORE-03 | Tasks are TaskPaper tasks (D-6). | Must |
| FR-STORE-04 | The description of an area, project or section is its TaskPaper text lines (notes) directly below it, before anything else inside it (D-6). Its lines are guarded as taskpaper-terminal guards attachment lines, with an invisible U+200B written and removed by the app, so a Markdown line such as `- item`, `Goals:` or indented code never reads back as a task, a project or a different depth ([attachments.md §1.1](reference/taskpaper-terminal/specs/attachments.md#11-guarding-content-lines)). | Must |
| FR-STORE-05 | A task's description and comments are stored as taskpaper-terminal stores attachments: hidden child nodes `NOTE @hidden` and `COMMENTS @hidden`, their content lines guarded so they never read as tasks or projects ([attachments.md §1](reference/taskpaper-terminal/specs/attachments.md#1-storage-fr-att-01), D-6). | Must |
| FR-STORE-06 | Checklist items are stored as TaskPaper tasks nested under their task (OQ-18). | Must |
| FR-STORE-07 | Dates and states are TaskPaper tags: `@start(YYYY-MM-DD)`, `@due(YYYY-MM-DD)`, `@done(YYYY-MM-DD)`, and a tag for items in the Trash (OQ-19). | Must |
| FR-STORE-08 | The Inbox is the first top-level project named exactly `Inbox` with no kind tag; the app creates it at the top of the file when there is none. Any other project named Inbox is an ordinary project. | Must |
| FR-STORE-09 | Every change is saved automatically. Text the app does not interpret, including unknown tags and lines, is kept unchanged on save ([requirements.md](requirements.md#data-handling)). | Must |
| FR-STORE-10 | User tags are TaskPaper tags on the item's line, `@name`, and the tags the app itself uses are reserved names the user cannot give a tag (OQ-11). | Must |

## Out of scope

- Sync, sharing, collaborators and assignees: the app is for one user and one file (D-1).
- Reminders and notifications.
- A GUI, web or mobile frontend, though the core must allow one later ([code quality](../docs/code-quality.md#ui-independent-core)).

## Decisions

| ID | Date | Decision |
|---|---|---|
| D-1 | 2026-10-13 | A task has a title, description, start date, due date, checklist and comments. The checklist is like Things 3's but nests to any depth; comments are like Todoist's but for a single user (owner). |
| D-2 | 2026-10-13 | The hierarchy is Area → Project → Section → Task, as in Things 3, except that projects nest, as in Todoist; areas can contain sections; and sections have descriptions (owner). |
| D-3 | 2026-10-13 | The app has an Inbox, as in Things 3, and Upcoming, Logbook and Trash views, as in Things 3 (owner). |
| D-4 | 2026-10-13 | The UI is keyboard first with vi-style bindings for most operations, for software engineers; editing is inline, as in Things 3; it has a command palette and a Go to Symbol palette, both with key bindings shown and filter-as-you-type, a full-text search view, and panes as in Workflowy; it is modern, sleek, fast and intuitive, following TUI best practices (owner). |
| D-5 | 2026-10-13 | A task's description and comments are edited in a modal, full-height panel that opens on the right side of the screen, copied from taskpaper-terminal's attachments panel. This is the one place the UI intentionally differs from Things 3 and Todoist (owner). |
| D-6 | 2026-10-13 | Data is stored in the `.taskpaper` format: areas, projects and sections become TaskPaper projects, tasks become TaskPaper tasks, the descriptions of areas, projects and sections become text lines, and tasks' descriptions and comments are stored like taskpaper-terminal's attachments (owner). |
| D-7 | 2026-10-13 | Start and due dates are entered in natural language, as in Todoist or Things 3 (owner). |
| D-8 | 2026-10-13 | The overall look and feel is close to taskpaper-terminal (owner). |
| D-9 | 2026-10-13 | The app has breadcrumbs, as in Workflowy (owner). |
| D-10 | 2026-10-13 | The app has Go Back and Go Forward through a navigation history (owner). |
| D-11 | 2026-10-13 | Settings follow taskpaper-terminal and include the theme, with 12 to 16 modern color schemes, Nerd Font icons on or off, and word wrap (owner). |
| D-12 | 2026-10-13 | Any item, an area, project, section or task, can be tagged, as in Things 3 (owner). |
| D-13 | 2026-10-13 | Any item, an area, project, section or task, can have a start date, a due date or both (owner). |
| D-14 | 2026-10-13 | Start and due dates are edited and displayed as in taskpaper-terminal (owner). |
| D-15 | 2026-10-13 | Key bindings mostly mimic taskpaper-terminal's, wherever they apply (owner). |
| D-16 | 2026-10-13 | The app has modern, nice and fluent animations (owner). |
| D-17 | 2026-10-13 | The app has a keyboard shortcuts panel, as in taskpaper-terminal (owner). |
| D-18 | 2026-10-13 | The app supports multi-selection and rearranging items, as taskpaper-terminal does (owner). |
| D-19 | 2026-10-13 | The app has taskpaper-terminal's weather animation (owner). |
| D-20 | 2026-10-13 | Every modal window that chooses an item, such as Move to, filters as the user types, as in taskpaper-terminal (owner). |
| D-21 | 2026-10-13 | Completed items are shown at the end of the list within each item, and a Show Completed Items command on `Ctrl+O` shows them (owner). The agent chose, after taskpaper-terminal: hidden by default, toggled per pane, file order untouched, checklists left as FR-CHK-04 has them. |
| D-22 | 2026-10-13 | v1 ships four themes, to save time; more can be explored and added later (owner). Narrows D-11 and OQ-15. |
| D-23 | 2026-10-13 | `g` shortcuts go to things (`gi` Inbox, `gt` Today, and so on) and `m` shortcuts modify things (`ms` set start date, `md` set due date, `mt` start today) (owner). |
| D-24 | 2026-10-13 | `Shift` with a motion extends the multi-selection, `Alt` with a motion moves the selected items up or down, and `[`/`]` outdent and indent, wherever these apply (owner). `[`/`]` replace taskpaper-terminal's start-date nudge on those keys; the keybindings spec gives that nudge other keys. |
| D-25 | 2026-10-13 | A task can be converted to a project; several tasks can be grouped into a section; several tasks and sections can be grouped into a project; a project can't be converted to an area (owner). The agent chose what a converted task's checklist and comments become, and that a group is created where its first item was. |
| D-26 | 2026-10-13 | An area's view and a project's view don't show the tasks of their child projects (owner). |
