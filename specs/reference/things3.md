# Things 3 Reference

- Status: Reference
- Last updated: 2026-10-13

A factual snapshot of the Things 3 (Cultured Code) feature set, paraphrased from Cultured Code's own support articles, guide and release notes, kept as the model for textual-items++'s lists, data model and keyboard behavior.

Legend: **(unverified)** = not confirmed in an official Cultured Code source during this research; **(inferred)** = our reading of an official source, not stated outright. Shortcuts are the Mac ones unless noted (Cmd = ⌘, Opt = ⌥, Ctrl = ^).

---

## 1. Data model

Sources: [Getting Productive with Things](https://culturedcode.com/things/guide/), [Using Headings in Projects](https://culturedcode.com/things/support/articles/2803577/), [Using Tags](https://culturedcode.com/things/support/articles/2803581/), [Things Shortcuts Actions](https://culturedcode.com/things/support/articles/9596775/), [AppleScript Commands](https://culturedcode.com/things/support/articles/4562654/), [URL Scheme](https://culturedcode.com/things/support/articles/2803573/)

### To-dos

- A to-do is a single task; a project is something that takes more than one step ([guide](https://culturedcode.com/things/guide/)).
- Fields (union of the URL scheme `add`/`update` parameters and the AppleScript dictionary):
  - **Title** (plain text; Markdown styling is *not* rendered in titles, [Writing Notes](https://culturedcode.com/things/support/articles/4438545/)).
  - **Notes**: multi-line, Markdown-styled (see section 8).
  - **Checklist**: ordered checklist items; only to-dos have checklists ([Shortcuts Actions](https://culturedcode.com/things/support/articles/9596775/)).
  - **Tags**: zero or more.
  - **When** (start date): Today, This Evening, a specific date, Anytime (no date) or Someday. Optional reminder time attached to the start date.
  - **Deadline**: a date by which it must be finished; separate from When. Deadlines cannot carry reminders ([Setting a Reminder](https://culturedcode.com/things/support/articles/2803585/)).
  - **Repeat** rule (turns the to-do into a repeating template, section 8).
  - **Status**: open, completed or canceled; plus creation, modification, completion and cancellation dates ([AppleScript](https://culturedcode.com/things/support/articles/4562654/)).
  - **Parent**: none (Inbox / loose), a project (optionally under one of its headings), or an area.

### Headings

- Headings split a long project into groups such as categories or milestones ([Using Headings](https://culturedcode.com/things/support/articles/2803577/)).
- **Headings exist only in projects**, not in areas or any other list (same article; also "Only projects can contain headings" in [Shortcuts Actions](https://culturedcode.com/things/support/articles/9596775/)).
- Moving a heading moves all its to-dos; duplicating (Cmd+D) copies the heading with its to-dos; copy/paste of a heading includes its to-dos.
- A heading (with its to-dos) can be **converted to a project** ("Convert to Project", added Mac 3.9, [release notes](https://culturedcode.com/things/support/articles/1100684/)).
- A heading is **archived** (not "completed") once its to-dos are done; it moves to the logged items at the bottom of the project. Archiving a heading also archives its to-dos ([features page](https://culturedcode.com/things/features/)).
- Headings have a title only. No official source documents notes, dates or tags on headings **(unverified that they cannot have them; none are documented)**.

### Projects

- Fields per URL scheme `add-project` / AppleScript: title, notes, When, deadline, tags, area, status (open/completed/canceled), contained to-dos and headings ([URL Scheme](https://culturedcode.com/things/support/articles/2803573/)).
- A project can live at the top level or inside an area. Projects can only move into or out of areas ([Moving Items](https://culturedcode.com/things/support/articles/9651894/)); there is no documented way to put a project inside another project (**projects do not nest**, inferred from the Move rules).
- Projects with a future start date are hidden from the sidebar until that date; projects in Someday are also removed from the sidebar ([In-Depth Look](https://culturedcode.com/things/support/articles/4001304/)).
- Projects can repeat (Repeat button beside the project title, [Repeating To-Dos](https://culturedcode.com/things/support/articles/2803564/)).
- Completed projects go to the Logbook and are listed in the "Logged Projects" Quick Find list.
- **Progress pie**: Things shows a project's completion as a small pie icon. **(unverified: not described in any official article fetched; widely seen in the UI.)**

### Areas

- Areas group projects and to-dos for an ongoing responsibility ("Family & Friends", "Money", "Health", "Career") with no end date ([guide](https://culturedcode.com/things/guide/)).
- An area **can contain**: projects and loose to-dos.
- An area **cannot contain**: headings ([Using Headings](https://culturedcode.com/things/support/articles/2803577/)), other areas (no nesting documented **(inferred)**).
- Area properties documented in AppleScript: name and tags. Areas have no documented notes, dates or completion state **(inferred from absence)**.
- Deleting an area does not trash the area itself but moves its children to the Trash ([AppleScript](https://culturedcode.com/things/support/articles/4562654/)); logged items from deleted areas stay in the Logbook (Mac 1.3.4 release note).
- In the sidebar, areas can be expanded/collapsed; "Show/Hide later items" toggles inside areas ([Keyboard Shortcuts](https://culturedcode.com/things/support/articles/2785159/)).

### Tags

- Tags categorize to-dos and filter lists ([Using Tags](https://culturedcode.com/things/support/articles/2803581/)).
- **Nested tags**: drop one tag onto another in Window → Tags (Mac) or use Group (iOS). Tag searches include items with nested (child) tags (Mac 3.5 release note).
- Projects and areas can be tagged; their tags are **inherited** by everything inside them. Inherited tags are not shown on the to-do but count for filtering and search.
- Each tag can have a single-letter keyboard shortcut (see section 7). Tag order controls the order in the filter bar.
- Removing a tag from an item does not delete the tag.

### Inbox

- Default capture location: new to-dos land in Inbox and are later filed or scheduled ([guide](https://culturedcode.com/things/guide/)). Review it daily.

---

## 2. Lists and views

Sources: [An In-Depth Look at Today, Upcoming, Anytime, and Someday](https://culturedcode.com/things/support/articles/4001304/), [Scheduling To-Dos](https://culturedcode.com/things/support/articles/2803579/), [Quick Find](https://culturedcode.com/things/support/articles/2803584/)

| List | Shortcut | Shows |
|---|---|---|
| Inbox | Cmd+1 | Unprocessed captured to-dos |
| Today | Cmd+2 | To-dos to start today, with This Evening at the bottom |
| Upcoming | Cmd+3 | Future-dated to-dos, deadlines and repeats on a timeline |
| Anytime | Cmd+4 | Every active to-do that can be started now |
| Someday | Cmd+5 | Undecided ideas and projects |
| Logbook | Cmd+6 | Completed and canceled items |
| Trash | (none documented) | Deleted items |

- **Today**: any to-do whose start date, deadline or repeat date matches today appears here, app-wide. Optional calendar events at the top. Can be grouped by project/area (Settings → General). Order is set manually by dragging.
- **This Evening**: a section at the bottom of Today; items stay visible but de-emphasised. Set with Cmd+E or `when=evening`.
- **Upcoming**: the next seven days (from tomorrow) shown individually at the top, then later dates. Dragging a to-do onto a day changes its start date. When the start date arrives, the item moves to Today automatically.
- **Anytime**: only active items; never overlaps Upcoming or Someday. Items with deadlines but no start date stay here. Today's items also appear here, marked with a yellow star. Loose to-dos float to the top; others are grouped by direct project or area; a project/area with no active to-dos is not shown.
- **Someday**: items with no plan; excluded from Anytime and Upcoming; shown de-emphasised; Someday projects are removed from the sidebar.
- **Logbook**: completed and canceled to-dos and projects, sorted by completion date. Projects logged are also listed in the "Logged Projects" Quick Find list.
- **Trash**: deleted items, most recently deleted at the top (Mac 3.15.7). Items can be dragged back out of the Trash (3.15.3). Emptied with an "Empty Trash" confirmation (3.12.5) or the AppleScript `empty trash` command.
- **Special lists reachable only via Quick Find** (type their name): Tomorrow, Deadlines (chronological), Repeating (all templates), All Projects, Logged Projects. They cannot be pinned to the sidebar.

### Movement rules

- Clearing the When date moves a to-do to Anytime; picking a calendar date moves it to Upcoming ([Scheduling](https://culturedcode.com/things/support/articles/2803579/)).
- Inside a project, inactive (future or Someday) to-dos are listed beneath active ones.
- **Completion → Logbook** timing is a setting ("Move completed items to Logbook") with options **Immediately, Daily, Manually**. Under Daily/Manually, completed items stay in place (checked) until logged; manual logging is Shift+Cmd+Y ("Move completed to Logbook") and can be undone (Mac 3.15.10). **(Option names confirmed via release-note mentions of "daily or manually"/"immediate" logging and a third-party description; no dedicated official article found.)**
- How overdue to-dos behave (they stay in Today) is **not stated** in the articles fetched **(unverified)**. Bumping with Ctrl+] / Ctrl+[ shifts start dates.

---

## 3. Editing

Sources: [features page](https://culturedcode.com/things/features/), [Keyboard Shortcuts](https://culturedcode.com/things/support/articles/2785159/), [Moving Items](https://culturedcode.com/things/support/articles/9651894/), [Quick Entry](https://culturedcode.com/things/support/articles/2249437/), [Gestures](https://culturedcode.com/things/support/articles/2803582/)

- **Inline editing**: opening a to-do (Return) expands it in place into a "clean white card" for notes; tags, checklist, start date and deadline controls stay tucked in a corner until needed. Cmd+Return saves and closes. In the main app Esc also saves (stated in the Quick Entry article as a contrast).
- **New items**: Cmd+N new to-do; Space new to-do below selection; Opt+Cmd+N new project; Shift+Cmd+N new heading; Opt+Shift+Cmd+N new heading containing the selection.
- **Magic Plus button**: tap for a new to-do; drag it to insert at a precise spot (in Today, under a heading, on an Upcoming day); dragged to the left margin in a project it creates a heading; dropped on the Inbox target it files to Inbox.
- **Quick Entry** (Mac, system-wide): Ctrl+Space opens a floating window; title plus details; normal Things shortcuts work inside it (dates, tags, Move with Shift+Cmd+M, including filing under a heading). Cmd+Return saves, Esc discards. **Quick Entry with Autofill** (Ctrl+Opt+Space) adds a link to the current Safari page, Mail message or Finder file into notes.
- **Paste**: Cmd+V with text on the clipboard creates new to-dos from clipboard lines; Cmd+C / Cmd+V copy and paste to-dos or projects; Opt+Cmd+V moves a copied item here; Cmd+D duplicates.
- **Move**: Shift+Cmd+M opens the Move dialog; type to filter destinations (project headings appear only when typed); "+ New Project" creates a project from the dialog; "No Project/Area" removes the parent. Works on single, open or multiple selected to-dos and inside Quick Entry. Destination rules: to-dos anywhere; projects only into/out of areas; headings only between projects.
- **Reorder**: Cmd+Up/Down; Opt+Cmd+Up/Down to top/bottom. Drag and drop reorders to-dos, headings, checklist items, projects and areas, and drops to-dos/headings on sidebar lists (Mac/iPad).
- **Multi-select**: Shift+Up/Down extends selection; Opt+Shift+Up/Down extends to top/bottom; Cmd+A selects all. Batch actions (Move, tags, dates, complete) apply to the selection. On iOS, swipe left to select.
- **Complete / cancel**: Cmd+K complete; Opt+Cmd+K cancel (Mac 3.12.5).
- **Convert to Project**: a to-do can be converted to a project (disabled for repeating to-dos since Mac 3.17). Whether checklist items become the project's to-dos is **(unverified)**.

---

## 4. Dates

Sources: [Natural Language Input](https://culturedcode.com/things/support/articles/9780167/), [Scheduling To-Dos](https://culturedcode.com/things/support/articles/2803579/), [Setting a Reminder](https://culturedcode.com/things/support/articles/2803585/), [Things 3.1 blog](https://culturedcode.com/things/blog/2017/07/things-3-1-repeating-to-dos-date-parsing)

- When picker: Cmd+S. Offers Today, This Evening, a calendar, Someday, "+ Add Reminder" and Clear. Direct shortcuts: Cmd+T Today, Cmd+E This Evening, Cmd+R Anytime, Cmd+O Someday.
- Deadline picker: Shift+Cmd+D; typed deadlines supported on Mac (and iPad/Vision with a keyboard).
- Bumping: Ctrl+] / Ctrl+[ start date ±1 day, with Shift ±1 week; Ctrl+. / Ctrl+, deadline ±1 day, with Shift ±1 week. If an item has a deadline but no When date, Ctrl+[ sets a When relative to the deadline (Mac 3.15.13).
- **Natural-language input** in the When field (all platforms) and Deadline field. Abbreviations accepted (w weeks, mo months, y years); numeric formats follow the region; languages: English, German, French, Italian, Spanish, Russian, Chinese, Japanese. Official examples:

| Input | Result |
|---|---|
| `tod` | Today |
| `tom` / "Tom(orrow)" | Tomorrow ([features](https://culturedcode.com/things/features/)) |
| "tomorrow", "Monday", "next week" | Relative dates, valid for When and Deadline |
| `10-3` | October 3 (region dependent) |
| `10-3 9` | October 3, 9 AM reminder |
| `2p` | 2 PM reminder (today) |
| `17d` | 17 days from today |
| `17d from jul9` / `jul9 + 17d` | 17 days after July 9 |
| `3rd wed jul`, `3rd wed jul 2026` | Third Wednesday in July |
| `last fri jul` | Last Friday in July |
| "in 5 weeks", "in 35 days", "in fou(r days)" | Relative offsets |
| "tomorrow at 2pm", "Wed 8pm" | Date plus reminder |

- `fri` alone is not an official example, but weekday names are documented ("Monday") **(inferred to work)**.
- **Evening** is a property of a Today item, not a time; the URL scheme accepts `when=evening`.
- **Reminders** attach only to the start date, never to deadlines. They fire once (not persistent alarms) and can be snoozed 10/30/60 minutes. To-dos cannot be sorted by reminder time. URL scheme: `when=date@time` adds a reminder.
- Repeating to-dos with a deadline let you set how many days before the deadline each copy appears in Today.

---

## 5. Checklists

Sources: [features page](https://culturedcode.com/things/features/), [Mac release notes](https://culturedcode.com/things/support/articles/1100684/), [Gestures](https://culturedcode.com/things/support/articles/2803582/), [Shortcuts Actions](https://culturedcode.com/things/support/articles/9596775/)

- Checklists break a to-do into finer steps (shopping or packing lists). Only to-dos have checklists; projects and headings do not.
- Shift+Cmd+C adds a checklist to the open to-do.
- Items are a **flat list**: no official source documents nesting, indentation or sub-items **(inferred one level only)**.
- Each item has a title and a status; Shortcuts uses Markdown task syntax to set status, implying open/completed/canceled states (`- [ ]`, `- [x]`, `- [~]`) **(inferred)**.
- Pasting a bulleted list or multi-line text into a checklist creates one item per line; empty lines are dropped (Mac 3.12.4).
- Reorder by dragging the grip; complete via its circle; swipe left to delete (iOS).
- In repeating to-dos, checklist items can be checked only in the generated copy, not in the template.
- Converting a checklist item to a to-do (or vice versa) is **not documented (unverified)**.

---

## 6. Search

Sources: [Searching and Navigating with Quick Find](https://culturedcode.com/things/support/articles/2803584/), [Using Tags](https://culturedcode.com/things/support/articles/2803581/), [Writing Notes](https://culturedcode.com/things/support/articles/4438545/)

- **Quick Find** merges search and navigation. Mac: just start typing (this is **Type Travel**), or Cmd+F. Results appear as you type and narrow with each key.
- Default matching: names of to-dos, projects, areas and tags. **Continue Search** extends it to notes, checklists and completed Logbook items.
- Navigation by typing a destination: "tod" → Today, "finan" → Finances area, "next action" → a heading of that name in a project. Settings are also reachable by name. Cmd+Return opens a result in a new window.
- Typing a tag name gives an app-wide tag filter (all items with that tag, including inherited and nested tags).
- **Find in Text** (Shift+Cmd+F) searches only inside the open to-do's notes; Cmd+G / Shift+Cmd+G next/previous; Find & Replace (Opt+Shift+Cmd+F) is Mac only.
- **Tag filter bar**: at the top of a list, shows tags used in that list; selecting one hides items without it; Cmd+click adds tags (AND semantics: items must have all selected tags); "All" clears; "No Tag" filter exists (Mac 3.5). Ctrl+Opt+tag-shortcut filters by keyboard; Ctrl+Esc clears.
- Navigation popover listing all lists: Shift+Cmd+O.

---

## 7. Keyboard shortcuts (Mac)

Source: [Keyboard Shortcuts for Mac](https://culturedcode.com/things/support/articles/2785159/) (iPad equivalents: [Keyboard Shortcuts for iPad & Vision](https://culturedcode.com/things/support/articles/2939808/))

| Group | Action | Keys |
|---|---|---|
| Navigate | Inbox / Today / Upcoming / Anytime / Someday / Logbook | Cmd+1 … Cmd+6 |
| Navigate | Navigation popover | Shift+Cmd+O |
| Navigate | Show in parent list | Cmd+L |
| Navigate | Enter selected project / go back | Return or Cmd+Right / Cmd+Left |
| Navigate | Sidebar up / down | Ctrl+Opt+Cmd+Up / Down |
| Navigate | Search (Quick Find) | Cmd+F or just type |
| Create | New to-do / below selection | Cmd+N / Space |
| Create | New project / heading / heading with selection | Opt+Cmd+N / Shift+Cmd+N / Opt+Shift+Cmd+N |
| Create | New checklist in open to-do | Shift+Cmd+C |
| Create | Quick Entry / with Autofill | Ctrl+Space / Ctrl+Opt+Space |
| Create | New to-dos from clipboard | Cmd+V |
| Edit | Open selected item / save and close | Return / Cmd+Return |
| Edit | Duplicate / copy / paste | Cmd+D / Cmd+C / Cmd+V |
| Edit | Complete / cancel | Cmd+K / Opt+Cmd+K |
| Edit | Move completed to Logbook | Shift+Cmd+Y |
| Select | First / last | Opt+Up / Opt+Down |
| Select | Extend up / down; to top / bottom | Shift+Up / Down; Opt+Shift+Up / Down |
| Select | Select all | Cmd+A |
| Move | Move to another list | Shift+Cmd+M |
| Move | Move copied item here | Opt+Cmd+V |
| Move | Up / down; to top / bottom | Cmd+Up / Down; Opt+Cmd+Up / Down |
| Dates | When picker | Cmd+S |
| Dates | Today / This Evening / Anytime / Someday | Cmd+T / Cmd+E / Cmd+R / Cmd+O |
| Dates | Start date ±1 day / ±1 week | Ctrl+] / Ctrl+[ ; Ctrl+Shift+] / [ |
| Dates | Deadline / ±1 day / ±1 week | Shift+Cmd+D ; Ctrl+. / Ctrl+, ; Ctrl+Shift+. / , |
| Dates | Repeat schedule | Shift+Cmd+R |
| Tags | Tag window / edit tags of selection | Ctrl+Cmd+T / Shift+Cmd+T |
| Tags | Toggle tag / filter by tag | Ctrl+letter / Ctrl+Opt+letter |
| Tags | Clear filter | Ctrl+Esc |
| Search | Find in text / replace / next / previous | Shift+Cmd+F / Opt+Shift+Cmd+F / Cmd+G / Shift+Cmd+G |
| View | Sidebar / toolbar / later items | Cmd+/ / Opt+Cmd+T / Shift+Cmd+E |
| View | New window | Ctrl+Cmd+N |
| Links | Open link in notes | Opt+Cmd+Return |

Markdown shortcuts in notes are listed in section 8. Log Completed moved from Cmd+L to Shift+Cmd+Y in Mac 3.14. No official shortcut for Trash was found.

---

## 8. Other features

- **Repeating to-dos and projects** ([article](https://culturedcode.com/things/support/articles/2803564/)): two modes, *fixed schedule* (copies on set dates, e.g. weekly on chosen weekdays) and *after completion* (next copy created when the previous one is done). Creating one makes a **template** (listed under the "Repeating" Quick Find list); edits to the template's title, notes, tags or checklist affect future copies, edits to a copy do not. Template actions: Change, Pause, Resume, Stop (Stop converts it to a normal to-do). Optional reminder and per-copy deadline. Copies cannot be completed ahead of schedule. Repeating to-dos work inside projects since 3.1 ([blog](https://culturedcode.com/things/blog/2017/07/things-3-1-repeating-to-dos-date-parsing)).
- **Logbook**: see section 2; manual logging is undoable.
- **Trash**: see section 2. Deleting a project trashes its children too.
- **Things URL scheme** ([article](https://culturedcode.com/things/support/articles/2803573/)): `things:///` commands `add`, `add-project`, `update`, `update-project`, `show`, `search`, `version`, `json` (bulk). `update` needs an `auth-token`. `when` accepts `today`, `tomorrow`, `evening`, `anytime`, `someday`, `yyyy-mm-dd`, English natural language, or `date@time`. Parameters include `checklist-items`, `heading`, `list`, `tags`, `completed`, `canceled`, `reveal`.
- **Markdown in notes** ([Markdown Guide](https://culturedcode.com/things/support/articles/4651820/), [Writing Notes](https://culturedcode.com/things/support/articles/4438545/)): headings `#` (Ctrl+Cmd+1…), italics, bold, highlight `::text::`, strikethrough `~~`, quotes `>`, bulleted, numbered and task lists (`- [ ]`, `- [x]`, `- [~]`), inline code and fenced code blocks (no syntax highlighting), links, horizontal breaks. Syntax always stays visible; links cannot be hidden behind text; no text color; no styling in titles or project/area names. Added in Mac 3.14.
- **Type Travel**: typing anywhere in the Mac app starts Quick Find and jumps to the named list, to-do or tag (Mac 3.0).
- **Automation**: Apple Shortcuts actions (Create To-Do / Heading / Project, Find Items, Edit Items, Show Items, Open List, Get Items, Get Selected Items, Duplicate, Delete, Run Things URL; [article](https://culturedcode.com/things/support/articles/9596775/)); AppleScript on Mac ([article](https://culturedcode.com/things/support/articles/4562654/)); Mail to Things; Siri.
- **Sync**: Things Cloud syncs across Mac, iPad, iPhone, Watch and Vision.
- **Appearance**: Light and Dark Mode only (Dark Mode added Mac 3.7; follows system or forced Light/Dark; setting in General since 3.18). No custom themes or text colors are documented.
- **Calendar**: events from chosen calendars can appear in Today and Upcoming.

---

## 9. Limitations relevant to textual-items++

- Checklists are one level deep: flat items only, no nesting documented; only to-dos have them.
- Areas cannot contain headings; headings exist only inside projects.
- Headings have a title only; no notes, dates or tags are documented for them.
- Projects cannot nest; a project can only sit at top level or in an area. Areas do not nest either.
- No comments or activity threads on to-dos; the only free text is notes **(inferred: no such feature documented)**.
- Single-user: no sharing, assignment or collaboration is documented **(inferred)**; Things Cloud syncs one person's devices.
- Reminders attach to start dates only, never to deadlines; reminders fire once.
- Markdown is not rendered in titles; syntax is never hidden.
- Repeating to-dos cannot be completed ahead of schedule.

---

## Sources

- Getting Productive with Things (guide): https://culturedcode.com/things/guide/
- Keyboard Shortcuts for Mac: https://culturedcode.com/things/support/articles/2785159/
- Keyboard Shortcuts for iPad & Vision: https://culturedcode.com/things/support/articles/2939808/
- An In-Depth Look at Today, Upcoming, Anytime, and Someday: https://culturedcode.com/things/support/articles/4001304/
- Scheduling To-Dos in Things: https://culturedcode.com/things/support/articles/2803579/
- Natural Language Input: https://culturedcode.com/things/support/articles/9780167/
- Setting a Reminder: https://culturedcode.com/things/support/articles/2803585/
- Using Repeating To-Dos: https://culturedcode.com/things/support/articles/2803564/
- Using Headings in Projects: https://culturedcode.com/things/support/articles/2803577/
- Using Tags: https://culturedcode.com/things/support/articles/2803581/
- Moving Items in Things: https://culturedcode.com/things/support/articles/9651894/
- Searching and Navigating with Quick Find: https://culturedcode.com/things/support/articles/2803584/
- Quick Entry: https://culturedcode.com/things/support/articles/2249437/
- Using Gestures: https://culturedcode.com/things/support/articles/2803582/
- Markdown Guide: https://culturedcode.com/things/support/articles/4651820/
- Writing Notes in Things: https://culturedcode.com/things/support/articles/4438545/
- Things URL Scheme: https://culturedcode.com/things/support/articles/2803573/
- Things AppleScript Commands: https://culturedcode.com/things/support/articles/4562654/
- Things Shortcuts Actions: https://culturedcode.com/things/support/articles/9596775/
- Discover Great Features: https://culturedcode.com/things/support/articles/1059358/
- Mac Release Notes: https://culturedcode.com/things/support/articles/1100684/
- Features page: https://culturedcode.com/things/features/
- Things 3.1 blog post: https://culturedcode.com/things/blog/2017/07/things-3-1-repeating-to-dos-date-parsing
