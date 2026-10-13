# Requirements

- Status: Draft
- Last updated: 2026-10-13

[functional-requirements.md](functional-requirements.md) defines what the app does and overrides this file. This file adds what that one leaves open: how data is handled safely, and the non-functional requirements. Undecided choices are in [open-questions.md](open-questions.md). Most of them follow taskpaper-terminal's, which handle the same format ([its requirements](reference/taskpaper-terminal/requirements.md)).

Priorities use the same scale as the functional requirements: **Must** (v1), **Should** (soon after), **Later**, **Won't**. IDs are stable; reference them from specs, tasks and commits. `F-<AREA>-<nn>` covers data handling and `N-<AREA>-<nn>` the non-functional requirements.

## Principles

1. **The file is the source of truth.** No database and no hidden store: everything the app knows is in the `.taskpaper` text (FR-STORE-01).
2. **Never lose data.** Crashes, external edits and bad input must not corrupt or drop the user's data ([AGENTS.md](../AGENTS.md#2-correct-not-just-green)).
3. **Preserve what the app does not understand.** Anything that writes to disk keeps content it does not interpret.
4. **TaskPaper compatible.** Files the app writes open correctly in TaskPaper 3 and other TaskPaper editors, and files they write open here ([reference](reference/taskpaper.md)).
5. **Terminal-native.** Works over SSH, in tmux and in 80×24.

## Data handling

| ID | Requirement | Priority |
|---|---|---|
| F-FILE-01 | Open the `.taskpaper` file the app is given, creating it when missing (OQ-16). | Must |
| F-FILE-02 | Parse files exactly as TaskPaper 3 does: item types, tags, indentation, blank lines ([taskpaper-terminal's file-format spec](reference/taskpaper-terminal/specs/file-format.md)). | Must |
| F-FILE-03 | Write files that TaskPaper 3 parses to the same outline. Opening and saving an unchanged file reproduces it byte for byte. | Must |
| F-FILE-04 | Saves are atomic (temporary file and rename) and keep the file's permissions. | Must |
| F-FILE-05 | Save automatically shortly after every change and on exit (FR-STORE-09). | Must |
| F-FILE-06 | Detect when another program changes the open file, and reload it without losing local edits ([taskpaper-terminal's file-io spec](reference/taskpaper-terminal/specs/file-io.md)). | Must |
| F-FILE-07 | Refuse to open a file that is not UTF-8, with a clear error, rather than corrupt it. | Must |
| F-FILE-08 | Keep the file's indent style, line endings, byte order mark and trailing newline. | Should |
| F-FILE-09 | Back up the file once a day, before the first change written that day, into a backups folder outside its own directory, keeping the newest 30 copies, as taskpaper-terminal does. | Must |

## Non-functional requirements

| ID | Requirement | Priority |
|---|---|---|
| N-PERF-01 | Open and draw a file of 50,000 items in under 300 ms on a modern laptop. | Must |
| N-PERF-02 | A keystroke reaches the screen in under 16 ms for typical actions on that file, palettes and search included (FR-UI-01). | Must |
| N-PERF-03 | Idle CPU near zero: draw only on input, resize, file change, timers and while an animation, the weather included, plays. | Must |
| N-REL-01 | A panic never loses data: the terminal is restored and unsaved changes are written to a recovery file. | Must |
| N-REL-02 | Parsing and writing are covered by round-trip property tests and a corpus of TaskPaper samples. | Must |
| N-PORT-01 | Runs on macOS and Linux in common terminals (iTerm2, Terminal.app, Ghostty, kitty, WezTerm, Alacritty, GNOME Terminal) and inside tmux. | Must |
| N-PORT-02 | Works in 80×24 and degrades gracefully below it (FR-UI-06). | Must |
| N-PORT-03 | Uses the kitty keyboard protocol when the terminal has it and falls back to legacy key reporting otherwise. | Should |
| N-PORT-04 | Measures wide characters, emoji and combining marks correctly; never splits a grapheme. | Must |
| N-UX-01 | Never blocks the UI on disk I/O for longer than a frame. | Must |
| N-UX-02 | Every destructive action can be undone (FR-EDIT-06). | Must |
| N-UX-03 | Honors `NO_COLOR`; every built-in theme is usable in a 16-color terminal. | Should |
| N-DIST-01 | A single binary, installable with `cargo install`. | Should |
