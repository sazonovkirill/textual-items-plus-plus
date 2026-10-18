+++
title = "Keyboard and mouse"
weight = 130
description = "How keys are written, key sequences and the g and m prefixes, the shortcuts panel and the Keymap, terminal settings, and what the mouse does."
+++

Everything in the app can be done from the keyboard. Keys follow vim and taskpaper-terminal where they can: `j` and `k` move, `gg` and `G` go to the ends, and two-key sequences starting with `g` or `m` go somewhere or change something. The mouse can focus, complete, choose and scroll, but no feature needs it.

## Key notation

This Reference writes keys as the app's own panels write them:

| Written | Means |
|---|---|
| `j`, `G`, `?` | the character: `G` is `Shift+g`, `?` whatever your keyboard types it with |
| `Ctrl+P`, `Alt+k`, `Alt+Shift+H` | the key with those modifiers held |
| `Enter`, `Esc`, `Tab`, `Space`, `Delete`, `Backspace`, `Home`, `End`, `PgUp`, `PgDn`, `↑` `↓` `←` `→` | those keys |
| `gg`, `mg`, `g?` | a sequence: `g`, let go, then the next key |
| `Ctrl+W v` | a sequence starting with a modified key: `Ctrl+W`, then `v` |

Where a command has several keys they are separated by commas, as `x, ., Ctrl+Enter`. The app binds no `Cmd` or `Super` key.

## Sequences

A sequence waits **one second** for its next key. While it waits, the keys pressed so far show at the right end of the status line, such as `g`. A key that continues no binding cancels the sequence and does nothing else; waiting longer than a second drops it. No key that starts a sequence is a binding of its own, so `g` alone never does anything, and `gg` never waits for a third key.

## The g and m families

Most two-key sequences belong to one of two families:

- **`g` goes** somewhere: to a view (`gi` Inbox, `gt` Today, `gu` Upcoming, `g!` Flagged, `ga` All, `gl` Logbook, `gd` Trash), a pane (`g0` to `g9`, `gn`, `gp`), a sidebar entry (`gj`, `gk`) or a window (`gc` the command palette, `gs` search, `gf` the tag filter, `g?` the Keymap, `g,` the settings). See [views](@/reference/views.md), [panes](@/reference/panes.md) and [finding things](@/reference/finders.md).
- **`m` modifies** the focused item or the selection: `ms` and `md` set the start and due dates, `mt` starts it today, `mg` tags it, `ml` marks it Later, `mf` flags it, `mp` pins a project, `me` puts it in the Evening, `mn` edits its description, `mi` opens the edit dialog, `my` converts it and `mj`/`mk` move it. See [dates](@/reference/dates.md), [tags](@/reference/tags.md), [tasks](@/reference/tasks.md) and [editing](@/reference/editing.md).

A few more prefixes group their own keys: `n` adds (`np` a project, `na` an area), `w` wraps the selection (`ws` in a section, `wp` in a project), `d` deletes (`dd`, and `de` empties the Trash) and `Ctrl+W` works on [panes](@/reference/panes.md).

## The shortcuts panel

**Toggle Shortcuts** (`?`) shows the shortcuts panel at the right of the panes, or hides it. It lists the commands the app takes now, with every key bound to them, under the groups Moving, Selection, Going, Editing and Creating, Completing, Dates and Tags, Structure, View and Panels, and Conflict. While a window, a picker, the attachments panel or an edit has the keys, that window's own keys come first, then the text keys it takes, then the commands it lets through, so the panel always shows what works now.

**Scroll Shortcuts Down** (`)`) and **Scroll Shortcuts Up** (`(`) scroll it a page; dim `↑ more` and `↓ more` lines say when there is more and name these keys. The mouse wheel over it scrolls it three lines a notch. It opens at the top.

Under the list it notes what the terminal reports: until a key with `Alt` has arrived, how to make `Option` send `Alt` on macOS, and whether the [kitty keyboard protocol](#terminals) is on.

The panel takes a quarter of the panes' width, 36 to 48 columns. Below 80 columns it is drawn over the pane, and below 40 columns or 9 rows it is left out.

{{ keys(topic="keyboard", titles="Toggle Shortcuts|Scroll Shortcuts Down|Scroll Shortcuts Up") }}

## The Keymap window

**Keymap** (`g?`) opens a window over nine tenths of the screen listing every key binding as a cheat sheet: each command's keys under its group, then the own keys of every window (the pickers, the date prompt, the edit dialog and its tag picker, Save As, Quick Add, Quick Comment, search, the settings window, the attachments panel and the editors), then the text keys every field takes. The groups go in as many columns as fit. Where `?` shows what works now, the Keymap shows everything; the same list is on the [cheat sheet](@/reference/cheatsheet.md).

While it is open, only quitting and the Navigator, which closes it, get through.

{{ shot(src="reference-2/keymap", alt="The Keymap window over the app, in three columns: Moving, Selection and Going on the left, View and Panels in the middle, the Settings window's keys, the Attachments Panel's keys and Editing on the right, with j/k scroll and Esc close on its bottom edge", caption="The Keymap window, at its top.") }}

{{ keys(topic="keyboard", titles="Keymap") }}

{{ window_keys(window="Keymap", caption="Keymap window") }}

## Terminals

The app asks the terminal for the **kitty keyboard protocol** when it starts and uses it where the terminal has it, as kitty and Ghostty do; elsewhere it reads the terminal's legacy key reporting, which cannot tell some keys apart. Every command keeps a key that works either way:

| Key | Without the kitty protocol | Use instead |
|---|---|---|
| `Ctrl+Enter` (Complete) | usually arrives as plain `Enter` | `x` or `.` |
| `Ctrl+Shift+Z` (Redo) | arrives as `Ctrl+Z`, which undoes | `Z` or `Ctrl+Y` |
| `Ctrl+\` (Context Menu) | arrives as `Ctrl+4`, which also opens it | |

**`Alt` on macOS.** macOS terminals type a character for `Option` with a letter, such as `˚` for `Option+k`, unless set to send it as `Alt`: in Terminal.app turn on Use Option as Meta Key, in iTerm2 set the Option key to Esc+. Every command with an `Alt` key has another without it: `mk`/`mj` for `Alt+k`/`Alt+j`, `P` for `Alt+v`, `g\` for `Alt+\`, `H`/`L` for `Alt+←`/`Alt+→`, `Ctrl+W <`/`>` for `Alt+Shift+H`/`L`.

**iTerm2's Natural Text Editing** preset sends `Alt+b` and `Alt+f` for `Option+←` and `Option+→`, and `Ctrl+A`, `Ctrl+E` and `Ctrl+U` for `Cmd+←`, `Cmd+→` and `Cmd+Delete`. The app binds those too: `Option+←`/`→` go back and forward in navigate mode and move by a word in a text field, and the `Cmd` keys move to the line's ends and delete to its start in a field (see [text keys](@/reference/editing.md#keys-text)).

**tmux** takes `Ctrl+B` as its prefix, so Toggle Sidebar has `|` as well.

## The mouse

The app reports the mouse for the whole session, so to select text in the terminal, hold the terminal's override while dragging, such as `Shift`, or `Option` in many macOS terminals.

| Where | Click | Wheel |
|---|---|---|
| a row | gives its pane the keys and focuses the row, picking nothing | moves the focus of the pane under the pointer three rows a notch, giving it the keys |
| a task's checkbox | as on its row, and completes or uncompletes the task | |
| elsewhere in a pane | gives the pane the keys | |
| a sidebar entry | shows it in the first pane, as `gj`/`gk` do, the keys staying where they are | browses the sidebar's entries |
| a picker's entry | chooses it; a click outside the picker closes it | moves the highlight, wherever the pointer is |
| the shortcuts panel | | scrolls it three lines |
| the attachments panel with the keys | | scrolls it, as `j`/`k` there |
| the Keymap window | | scrolls it three lines |

While a prompt, a window, the edit dialog, the conflict prompt or an edit has the keys, clicks do nothing. Moving the pointer, dragging, the other buttons and the sideways wheel do nothing, but like any input they end the [weather](@/reference/appearance.md#weather).
