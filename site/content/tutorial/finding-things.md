+++
title = "Finding things and panes"
weight = 100
description = "Jump anywhere with the Navigator, search the whole file, run any command and split the screen."
+++

## The Navigator

Press `Ctrl+K` (or `Ctrl+G`) from anywhere. The **Navigator** lists the Inbox, the views, and every area and project. Type `edin`: only "Weekend in Edinburgh" is left. Press `Enter` to open it.

{{ shot(src="tutorial/navigator", alt="The Navigator over the Inbox with edin typed and Weekend in Edinburgh in Home as the only match", caption="The Navigator opens any list in a few keys.") }}

## Go to Symbol

The Navigator lists places; **Go to Symbol** finds any single item, a task or a section too. Press `/`, type `tap` and press `Enter`. The Home area opens with the focus on the dripping tap.

## Search

Press `Ctrl+F` and type `ana`. Search looks through everything: titles, descriptions, notes, comments and checklists. The matches are highlighted, each with where it lives.

{{ shot(src="tutorial/search", alt="Search results for ana: Reply to Ana about the conference talk in Work, and the Customer newsletter project with its comment Kick-off with Ana and Tom went well, with Ana highlighted", caption="Search finds words in comments and notes too.") }}

Press `Enter` to move into the results, then `j` and `Enter` on one to open it where it lives.

## The command palette and the context menu

You don't have to remember every key. Press `Ctrl+P` for the **command palette**, which lists every command with its keys. Type `date` to narrow it to commands about dates, and press `Enter` to run the one you pick, or `Esc` to close it.

{{ shot(src="tutorial/palette", alt="The command palette with date typed, listing Set Start Date ms, Set Due Date md, Start a Day Later and Due a Day Later with their keys", caption="The command palette, also a quick way to look up a key.") }}

`Ctrl+\` opens the **context menu** right under the focused row, with just the commands that apply to it.

## Panes

You can look at two lists side by side. Press `Ctrl+F`, type `ana`, press `Enter`, then move to the "Customer newsletter" result with `j`. Press `Ctrl+T` (or `Alt+Enter`, or `Ctrl+W` `v`). The project opens in a new pane to the right, while the results stay on the left.

{{ shot(src="tutorial/panes", alt="Two panes: search results for ana on the left, numbered 1, and the Customer newsletter project on the right, numbered 2", caption="Search results and a project side by side.") }}

`Tab` and `Shift+Tab` move between the sidebar and the panes, and an edit in one pane shows in every other at once. `Ctrl+W` `c` closes the pane you're in. The app remembers your panes for the next time you start it.

The [Reference](@/reference/finders.md) has more on the finders, and [Panes](@/reference/panes.md) on splitting the screen.
