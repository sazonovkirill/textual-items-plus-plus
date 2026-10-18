+++
title = "Notes, checklists and comments"
weight = 100
description = "The attachments panel: an item's Markdown note, a task's checklist and an item's dated comments, and Quick Comment."
+++

Besides its title and [description](@/reference/editing.md), every area, project, section and task, and the Inbox, can have **attachments**: a **note**, Markdown text of any length, and **comments**, dated one-line remarks. A task also has a **checklist**. All three are read and edited in the **attachments panel**, which opens over the right part of a pane.

## The row icons

A row shows what its item has at its right end, dimmed, before its date chips:

| Shows | Means | Plain marker |
|---|---|---|
| the checklist icon and `2/5` | a checklist, with how many of its items are checked | `☑ 2/5` |
| the note icon | a note | `≡` |
| the comment icon and `3` | comments, with how many | `❞ 3` |

The title line of an open area or project shows its note and comment icons the same way. The plain markers are those drawn with [plain icons](@/reference/appearance.md#icons).

## The attachments panel

**Open Note** (`\`) opens the panel on the focused row's item; **Open Container Note** (`Alt+\` or `g\`) opens it on the area, project or Inbox the pane shows. The panel is bordered and titled with the item. It lists, under their headings:

- **Note**: the note, rendered as [Markdown](#notes), or a dimmed `No note`.
- **Checklist**, on a task only: its items, or a dimmed `No checklist`.
- **Comments**: the comments, newest first, or a dimmed `No comments`.

One entry is **selected**, the note, one checklist item or one comment, and drawn on the current-line band with the focus marker; it opens on the note. `j`/`k` and `↓`/`↑` select the next and previous entry, scrolling through one taller than the panel before moving on; `Ctrl+D`/`Ctrl+U` and `PgDn`/`PgUp` go a page; `gg` or `Home` goes to the top and `G` or `End` to the bottom. The mouse wheel scrolls it while it has the keys. A hint at its bottom names the keys for what is selected, such as `e edit · o add · x check · dd del · c comment · \ full · Esc close` on a checklist item.

While the panel has the keys, the status line says `ATTACHMENTS`. Commands that do not apply in the panel do nothing there; quitting, pasting and the keys that [move between panes](@/reference/panes.md#focusing-panes-and-the-sidebar) still work. `Esc` closes it and gives the keys back to the pane's list.

The commands **Open Comments** and **Open Checklist** open the panel with the newest comment or the first checklist item selected, and **Open Container Comments** on the container shown; they have no key and are in the [command palette](@/reference/finders.md#the-command-palette) and the [context menu](@/reference/finders.md#the-context-menu). Open Checklist on anything but a task says `only a task has a checklist`.

Every change made in the panel is one step to [undo](@/reference/editing.md#key-undo) and is saved like any other edit.

{{ shot(src="reference-2/attachments-checklist", alt="The Launch the billing page project with the attachments panel open over its right half on the task Wire the card form to the payments API, showing its note, a checklist of five items with Expiry and CVC selected and Error messages checked and dimmed, and one comment dated Oct 18 09:12, with the panel's key hint at the bottom", caption="The panel with a checklist item selected.") }}

### Width

The panel opens at half its pane's width, between 40 and 72 columns, from the top of the pane to the status line; in a pane narrower than 40 columns it covers the whole pane. `\` in the panel goes round its states: from half to the whole pane, then hidden again, as `Esc` hides it. The hint names the key `\ full` at half and `\ hide` at full.

While a panel covers its whole pane, that pane's list is passed over when the keys move between panes, so that no command can act on rows out of sight.

### One panel per pane

Each pane has its own panel, so two panes can show the attachments of two items. A pane with an open panel has two stops for the keys, its list and then its panel, which `Tab`, `Shift+Tab` and the other [pane keys](@/reference/panes.md#focusing-panes-and-the-sidebar) go through. While the list has the keys, its panel stays on screen with a dim border and nothing marked; `Esc` closes only the panel that has the keys.

A panel stays on its item while the pane's list moves and while the pane shows other places; `\` on another row moves it there. It closes when its item leaves the document or goes to the Trash (unless the pane shows the Trash), and when its pane closes. Open panels are not kept with the [pane layout](@/reference/panes.md#layout-on-the-next-launch).

{{ window_keys(window="Attachments Panel") }}

## Notes

With the note selected, `e`, `i` or `Enter` edits it as raw text, in a multi-line editor with the [keys of a description](@/reference/editing.md#keys-editing-description): `Enter` starts a new line and `Esc` ends the edit, keeping it. Spaces and empty lines at the end of the note are dropped, so emptying a note removes it.

The panel renders the note's Markdown, the syntax hidden:

| Markdown | Shows as |
|---|---|
| `# Heading` to `###### Heading` | bold; the first level also underlined |
| `- item`, `* item`, `+ item`, `1. item` | a list item after a bullet or its number, wrapped lines hanging under its text; two spaces in nest it a level |
| `> quote` | dimmed after a bar |
| a fence of three backticks or tildes | code, dimmed and as typed |
| `**bold**`, `*emphasis*` or `_emphasis_`, `` `code` `` | bold, italic, dimmed code |
| `[title](url)`, `<url>` or a bare URL | underlined in the link color, the title shown |

A delimiter without its partner shows as typed. Links are not opened from the app.

## Checklists

A task's checklist is a flat list of items, each with text and a checked state; items do not nest. With a checklist entry selected, the usual keys act on the selected item as they act on a row:

| Keys | Does |
|---|---|
| `o` or `Space`, `O` | add an item below or above the selected one, or the first item of an empty checklist |
| `e`, `i` or `Enter` | edit the item's text |
| `x`, `.` or `Ctrl+Enter` | check it, or uncheck it |
| `dd` or `Delete` | delete it, for good rather than to the Trash (undo brings it back) |
| `Alt+j`/`Alt+k`, `mj`/`mk` | move it down or up |
| `p` or `Ctrl+V`, `P` or `Alt+v` | paste below or above: each line of what is pasted becomes an item, in order, whatever its indentation |

While an item's text is edited, `Enter` keeps it and adds the next item below; `Esc` keeps it and stops. An item left empty is not added, and what was selected before is selected again.

Checking an item changes nothing else: the other items and the task keep their state. A checked item is drawn dimmed with `✔`, not struck through.

{{ window_keys(window="Editing Checklist Item") }}

## Comments

Comments are single lines, each stamped with the local date and time it was added, listed newest first. There is one user, so a comment has no author. The panel writes the time short, as `Oct 18 09:12`, adding the year when it is not this year's.

`c` or `a` in the panel adds a comment, stamped with the time the key is pressed, in a one-line input at the top of the Comments section, its time shown above it. With a comment selected, `e`, `i` or `Enter` edits its text, keeping its time, and `dd` or `Delete` deletes it. In the input `Enter` keeps the comment and `Esc` drops what was typed. The text is trimmed: a new comment of nothing but spaces is not added, and emptying an existing one leaves it as it was. A paste goes in with its line breaks as spaces.

{{ window_keys(window="Editing Comment") }}

## Quick Comment

**Quick Comment** (`c`) adds a comment to the focused item without opening the panel. A bordered one-line input opens just above the status line, titled `Comment on` and the item's title. `Enter` adds the text, trimmed, as the item's newest comment, stamped with the time `Enter` is pressed, as one step to undo, and the status line says `comment added`; `Esc`, or `Enter` with nothing typed, closes it and adds nothing. Text keys and pasting work as in any field.

It takes any item but a checklist item: a task, section, project, area or the Inbox. In the sidebar it takes the focused entry, and on a view it says `a view takes no comments`.

{{ window_keys(window="Quick Comment") }}

## How attachments are stored

Notes and comments are kept in the TaskPaper file as hidden lines under their item, which other TaskPaper apps show as plain notes; checklist items are tasks under their task:

```
	- Wire the card form to the payments API @start(2026-10-18) @deep
		Use the sandbox keys until the security review is done.
		NOTE @hidden
			Retries must be idempotent: send the same idempotency key on every attempt.
		COMMENTS @hidden
			[2026-10-18 09:12] The sandbox returns 402 for expired cards, handle it.
		- Card number field
		- Error messages @done(2026-10-17)
```

The first line under the task is its description. A comment starts with its `[YYYY-MM-DD HH:MM]` stamp; a comment line without one, typed by hand or by another app, is kept and shown as text with no time. A checked item carries `@done` with the day it was checked. A file that nests checklist items, as earlier versions wrote them, is read flat, each nested item after the one it was in, and written flat at the next save.

## Keys

{{ keys(topic="attachments") }}
