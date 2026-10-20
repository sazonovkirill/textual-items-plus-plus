+++
title = "Panes"
weight = 120
description = "Showing several places side by side: opening, focusing, closing and resizing panes, and the layout that comes back on the next launch."
+++

The screen shows any number of **panes** side by side as columns, a dim rule apart, beside the [sidebar](@/reference/views.md) while it shows. Each pane shows its own area, project, Inbox or view under its title line, with its own focus, selection, [history](@/reference/navigation.md#history), [tag filter](@/reference/tags.md#filtering-by-tags), [attachments panel](@/reference/attachments.md#one-panel-per-pane) and choice of showing completed items. All panes show the same document: an edit in one shows at once in every pane, each keeping its focus on the same item where it can.

One pane, or the sidebar, has the keys; only it marks its focused row.

{{ shot(src="reference-2/panes", alt="Three panes side by side without the sidebar: the Launch the billing page project numbered 1, Today numbered 2 with its focused row marked, and Upcoming numbered 3", caption="Three panes; the second has the keys.") }}

## The title line and the pane number

Each pane starts with its title line: the area, project or Inbox title, or the view's icon and name. While there are several panes, the title line ends with the pane's number, dimmed at its right edge, the number `g1` to `g9` go to. A single pane shows no number. The tags the pane is filtered by show just before the number. The title line scrolls away with the rows under it.

## Opening a pane

**Open in New Pane** (`Alt+Enter`, `Ctrl+T` or `Ctrl+W v`) adds a pane right of the one with the keys and gives it the keys. It shows what the focused row opens: an area or project row opens that place, a section its area or project focused on it, and a task in a view the container listing it, focused on the task. On any other row it shows the place the pane shows, focused on the same row.

The new pane starts with an empty history and no tag filter, and shows completed items when the pane it came from does or when it opens on a completed item. The [Navigator](@/reference/finders.md#the-navigator) and [Go to Symbol](@/reference/finders.md#go-to-symbol) open their choice in a new pane with `Alt+Enter` or `Ctrl+T`, and so do search results with Open in New Pane.

{{ keys(topic="panes", titles="Open in New Pane") }}

## Focusing panes and the sidebar

Several keys move the keys between the panes and the sidebar:

- **Focus Next Pane** (`Tab` or `gn`) and **Focus Previous Pane** (`Shift+Tab` or `gp`) go round the sidebar, while it shows, and the panes, wrapping at either end. A pane with an open attachments panel is two stops, its list then its panel.
- **Focus Pane Left** (`Ctrl+W h`, `Ctrl+W ←`, `Ctrl+Shift+←` or `Ctrl+,`) and **Focus Pane Right** (`Ctrl+W l`, `Ctrl+W →`, `Ctrl+Shift+→` or `Ctrl+.`) step to the stop on that side without wrapping; from the first pane, left goes to the sidebar while it shows, and right from the sidebar goes back to the pane that had the keys.
- **Focus Pane 1** to **9** (`g1` to `g9`) give a pane its keys by number, and **Focus Sidebar** (`g0`) gives them to the sidebar, showing it when it is hidden.

The first pane is linked to the sidebar: moving in the sidebar, or [Next and Previous Sidebar Entry](@/reference/views.md) (`gj`/`gk`) from anywhere, shows the sidebar's entry in the first pane.

A click in a pane gives it the keys too (see [the mouse](@/reference/keyboard.md#the-mouse)).

{{ keys(topic="panes", titles="Focus Next Pane|Focus Previous Pane|Focus Pane Left|Focus Pane Right|Focus Sidebar|Focus Pane 1|Focus Pane 2|Focus Pane 3|Focus Pane 4|Focus Pane 5|Focus Pane 6|Focus Pane 7|Focus Pane 8|Focus Pane 9") }}

## Closing a pane

**Close Pane** (`Ctrl+W c`) closes the pane with the keys, with its attachments panel, and gives the keys to the pane on its left, or on its right when it was the first. The last pane is never closed.

{{ keys(topic="panes", titles="Close Pane") }}

## Resizing

**Narrow Pane** (`Ctrl+W <` or `Alt+Shift+H`) and **Widen Pane** (`Ctrl+W >` or `Alt+Shift+L`) make the pane with the keys 4 columns narrower or wider, no narrower than 30 columns. A pane resized this way keeps its width; the panes that keep none share the rest of the screen equally, none narrower than 30 columns.

{{ keys(topic="panes", titles="Narrow Pane|Widen Pane") }}

## When panes don't fit

When the panes do not all fit at 30 columns or more, the strip of panes scrolls sideways by whole panes, only as far as it must to keep the pane with the keys in view, and the last pane drawn takes the columns left over. Moving the keys to a pane off screen scrolls it into view.

## Layout on the next launch

When the app quits, it remembers the layout of the file: the panes in order, what each shows, its focused row, its history back and forward, whether it shows completed items, the width it keeps and the words it searches for, and which pane has the keys. The next time the same file is opened, the panes come back as they were.

A pane's tag filter and selection, its attachments panel, the sidebar and open windows are not kept, and the demo data keeps no layout. An item that is no longer there gives way to the nearest place above it that is, else the Inbox; a focused row that is gone gives way to the first row.

Layouts are kept in `$XDG_STATE_HOME/textual-items-plus-plus/layouts.json`, or `~/.local/state/textual-items-plus-plus/layouts.json` when `XDG_STATE_HOME` is not set, on macOS too, one entry per file. A missing or unreadable layouts file opens the file as if it had none.
