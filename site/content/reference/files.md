+++
title = "Files and saving"
weight = 10
description = "One plain-text TaskPaper file holds everything; every change is saved to it automatically, with daily backups and recovery after a crash."
+++

textual-items++ keeps everything in one `.taskpaper` file: the areas, projects, sections and tasks, their dates, tags, descriptions, notes, comments and checklists, the Trash and the Inbox. There is no database. The file is plain text that TaskPaper and any text editor can open, and the app writes back every line it did not change exactly as it read it.

## Starting the app

`tipp Life.taskpaper` opens that file. A file that does not exist yet is created by the first save, as long as its folder exists; with no such folder, `tipp` stops with `Error: cannot open <file>: the folder <folder> does not exist`. A file that is not valid UTF-8 is not opened at all, so it can never be damaged: `tipp` stops with `Error: cannot open <file>: the file is not valid UTF-8` and names the first invalid byte.

`tipp` with no file opens a built-in **demo document** of toy data, with areas, projects, sections and tasks that show dates, tags, descriptions, notes, comments and checklists. It lives in memory only. The status line says `Demo data · not saved`, and nothing you change is written anywhere, unless you keep it with [Save As](#save-as).

`tipp` takes at most one argument; anything more stops it with `Error: usage: tipp [file]`.

## How items are stored

The app maps its items onto TaskPaper's three kinds of line: projects (a line ending in `:`), tasks (a line starting with `- `) and notes (any other text). Indentation, one tab per level for new lines, makes the outline.

```
Inbox:
	- Call the dentist @calls @due(2026-10-19)
Work: @area
	Everything for the day job.
	Launch the billing page: @pinned @flagged @due(2026-10-30)
		The new self-serve billing page.
		NOTE @hidden
			# Goals
			- Customers upgrade without talking to sales
		COMMENTS @hidden
			[2026-10-17 17:40] Legal approved the new terms.
		- Agree on the plan names @done(2026-10-16)
		Build: @section
			- Wire the card form @start(2026-10-18) @deep
				- Card number field
				- Error messages @done(2026-10-17)
```

| What | In the file |
|---|---|
| Inbox | The first top-level project named exactly `Inbox` with no kind tag. If the file has none, the app writes one at the top. Any other project named Inbox is an ordinary project. |
| Area | A top-level project tagged `@area`. |
| Project | A project with no kind tag, at the top level or inside an area. |
| Section | A project tagged `@section`, inside an area or a project. |
| Task | A task line inside the Inbox, an area, a project or a section. |
| Checklist item | A task line directly under a task. Checklists are flat: items nested deeper, as earlier versions wrote them, are read as one flat list, and the next save writes them flat. |
| Description | The text lines directly under an item, before anything else inside it. |
| Note | A `NOTE @hidden` line after the description, holding the note's lines. |
| Comments | A `COMMENTS @hidden` line after the note, one line per comment, newest first, each starting with its local time, `[YYYY-MM-DD HH:MM]`. |
| Start and due date | `@start(YYYY-MM-DD)` and `@due(YYYY-MM-DD)`. |
| Completed | `@done(YYYY-MM-DD)`. A bare `@done`, as other apps may write it, completes an item with no date. |
| Later | `@later`. |
| Pinned project | `@pinned`. |
| Flagged | `@flagged`. |
| In the Trash | `@deleted(YYYY-MM-DD)`, the day it was deleted. An item in the Trash stays where it was in the file. |
| Your tags | Any other tag, `@name` or `@name(value)`, in the order you gave them. |

The names the app uses itself, `start`, `due`, `done`, `deleted`, `later`, `hidden`, `area`, `section`, `pinned` and `flagged`, together with `id`, `text` and `type`, which TaskPaper reserves, are never offered as [tags](@/reference/tags.md).

Description, note and comment lines can hold any Markdown, even a line such as `- item`, `Goals:` or indented code. The app writes an invisible zero-width space (U+200B) in front of such a line, so it reads back as text at its depth and not as a task or a project, and removes it again when it reads the file.

When the app rewrites an item's line, it writes the title, then the kind tag, `@pinned`, `@flagged`, `@start`, `@due`, `@later`, your tags in their order, and last `@done` and `@deleted`. A repeated tag, which TaskPaper ignores, is dropped then. A date tag whose value is not a `YYYY-MM-DD` day stays an ordinary tag. An area's tags and dates, which areas do not have in the app, stay in the file but are not shown.

Lines the app does not interpret are kept exactly where they are, with everything under them, and not shown: a task at the top level, a project inside a section, an `@area` below the top level, a line tagged `@hidden`, a project tagged both `@area` and `@section`, and text after an item's first child. Some state never reaches the file at all: the [Evening](@/reference/views.md#today) and the order you give Today.

## Saving

There is no Save command. Every change is saved automatically, a second after the last one, and again when you quit. `SIGTERM` and `SIGHUP`, as when the terminal window closes, save first too. A title or description being edited is saved once editing ends. Saving runs in the background, so typing never waits for the disk.

The status line, after the mode and the file's path, says where the file stands:

| Status line | Means |
|---|---|
| `saved` | The file holds every change. |
| `unsaved` | A change is waiting for its save. |
| `save failed: <reason>` | The last save failed. It is tried again on the next change and when you quit. |
| `not on disk` | Another program deleted or moved the file. The next save creates it again. |
| `not saved` | The demo document, which is never saved. |

After something happens to the file, the status line says so instead until the next command: `reloaded (changed on disk)`, `saved as <path>`, `backup failed: <reason>` and the conflict messages below.

Every save is atomic: the app writes a temporary file beside yours and renames it over the original, so a crash halfway never leaves a half-written file. Lines you did not change keep their bytes, indentation, line endings and byte order mark.

If you quit after a save failed, the app does not quit; the status line starts with `quit again to quit without saving`, and quitting again ends without the changes.

## Backups

Before the app changes a file on a given day, it copies the file as it is on disk to a backup folder, once a day: when it opens the file, and again on the first save of a new day if it runs past midnight.

- **Where:** `$XDG_DATA_HOME/textual-items-plus-plus/backups/`, or `~/.local/share/textual-items-plus-plus/backups/` when `XDG_DATA_HOME` is not set, on macOS too.
- **One folder per file**, named after the file plus a hash of its full path, such as `Life.taskpaper-4d9508cae9aba711/`, so two files with one name never mix. A `source-path.txt` in it names the file.
- **One copy per day**, named by the date, such as `2026-10-18.taskpaper`, with the file's permissions.
- **Skipped** for a missing or empty file and when the file equals the newest copy, so a file left alone does not use up its copies. The demo document has none.
- **The newest 30** daily copies are kept; older ones are deleted.

A failed backup never stops saving: the status line says `backup failed: <reason>`, and the app tries again the next day. There is no restore command. A backup is an ordinary `.taskpaper` file: open it with `tipp` and copy back what you need.

## When the file changes on disk

The app watches the file for changes by other programs, such as a sync tool, TaskPaper or a text editor.

**With nothing unsaved**, the file reloads at once and the status line says `reloaded (changed on disk)`. Each pane stays on what it showed and on its focused row, found again by their titles, else on the nearest item above that is still there. The undo history is cleared, and an edit under way ends first.

**With unsaved changes**, saving pauses and the **conflict prompt** opens, saying `The file changed on disk. Your changes are not saved yet.` It waits for one of its keys:

- **Keep mine** (`Ctrl+E`): the version on disk is first kept in a conflict copy, then your version is saved over it. The status line says `kept mine; theirs is in <copy>`.
- **Take theirs** (`Ctrl+R`): your version is kept in a conflict copy, then the file reloads. The status line says `took theirs; mine is in <copy>`.
- **Show diff** (`Ctrl+L`): shows or hides the lines that differ, from the file on disk (`-`) to your version (`+`), with three unchanged lines around each change and `…` between changes further apart. The diff shows what fits; it does not scroll.
- **Quit** (`Ctrl+Q`): leaves the file on disk as it is, keeps your version in a conflict copy and prints `tipp: left theirs on disk; mine is in <copy>` when the app ends.

The prompt can open while you type, so its choices are `Ctrl` keys that typing never sends; `Esc` does nothing. When the file on disk is no longer UTF-8, the prompt opens even with nothing unsaved, and Take theirs is refused with `cannot take theirs: the file on disk is not UTF-8`.

Conflict copies are named `<YYYY-MM-DD>-<HHMMSS>-conflict.taskpaper` and go into the file's backup folder, where the newest 30 are kept apart from the daily copies. If a copy cannot be written, the status line says `conflict copy failed: <reason>` and the prompt stays open to choose again, so the version being replaced is never lost.

{{ keys(topic="files", titles="Keep Mine|Take Theirs|Show Diff") }}

## Recovery after a crash

If the app crashes, or ends because the terminal is gone or it was told to stop after a save failed, it first writes the unsaved document to a recovery file, then restores the terminal and prints `tipp: the unsaved changes are in <path>`. Recovery files go to `$XDG_STATE_HOME/textual-items-plus-plus/recovery/` (`~/.local/state/…` when unset) and are named `<backup folder name>-<YYYYMMDD-HHMMSS>.taskpaper`, or `untitled-…` for the demo document. A save that landed leaves no recovery file.

The next `tipp <file>` finds a recovery file at least as new as the file and asks on the terminal, before the app starts:

```
<recovery file> holds changes to <file> that were not saved when tipp stopped.
Open the file with these changes? [y/N]
```

`y` keeps the file on disk in a conflict copy, then opens it with the recovered document as unsaved changes, which the next save writes. Any other answer opens the file as it is. The app never deletes recovery files.

## Save As

**Save As…** (`Ctrl+S`) writes the document to another file and goes on working there. A prompt above the status line shows the path of the open file, selected, so typing replaces it, or nothing for the demo document. `~` stands for your home folder, and a relative path starts from the folder you started `tipp` in.

- `Enter` saves. The status line then says `saved as <path>`, and from now on saves, backups, watching for outside changes, the kept pane layout and recovery all go to the new file. The old file keeps what it last held.
- When the file exists, the prompt says `The file exists: Enter replaces it, Esc goes back`. `Enter` keeps the old file in a conflict copy in its backup folder, then replaces it; `Esc` goes back to the path.
- When saving fails, the prompt says why, such as `Cannot save: the folder /home/me/notes does not exist`, and takes the path again.
- `Esc` closes the prompt and saves nothing.

Save As is how you keep the demo document: once saved, it is a file like any other.

{{ window_keys(window="Save As") }}

## Where the app keeps its own files

| What | Where |
|---|---|
| Settings | `$XDG_CONFIG_HOME/textual-items-plus-plus/config.toml`, or `~/.config/…` (see [Settings](@/reference/settings.md)) |
| Pane layouts | `$XDG_STATE_HOME/textual-items-plus-plus/layouts.json`, or `~/.local/state/…` |
| Recovery files | `$XDG_STATE_HOME/textual-items-plus-plus/recovery/`, or `~/.local/state/…` |
| Backups and conflict copies | `$XDG_DATA_HOME/textual-items-plus-plus/backups/`, or `~/.local/share/…` |

The same folders are used on macOS. When you quit, the app remembers each file's [panes](@/reference/panes.md): what each shows, its focused row, its history, its width, whether it shows completed items and its search words, and which pane has the keys. They come back the next time you open the same file. The tag filter, the selection, the sidebar and open windows are not kept, nor is anything for the demo document. `layouts.json` is readable only by you.

## Quitting

**Quit** (`Ctrl+Q`) works from anywhere: in navigate mode, while you edit, and in every window and prompt. It saves first. A title, description, note or comment being edited is kept, and the edit dialog applies its fields as if you had closed it. Quick Add, the date prompt and Save As are prompts: quitting with one open adds or changes nothing.

{{ keys(topic="files", titles="Quit|Save As…") }}
