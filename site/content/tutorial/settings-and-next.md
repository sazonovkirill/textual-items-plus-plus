+++
title = "Settings, themes and where next"
weight = 110
description = "Change the look and the settings, and find out where to learn the rest."
+++

## Settings

Press `g` `,` to open the settings window. It lists every setting with a line about what it does: the theme, icons, word wrap, animations, task counts in the sidebar, and the weather that falls over the app when you leave it alone. Move with `j` and `k`, change a value with `l` and `h` (or `Enter`), and `r` resets one to its default. A change applies at once and is saved straight away to `~/.config/textual-items-plus-plus/config.toml`, which you can also edit by hand. `Esc` closes the window.

## Themes

The app comes with twenty themes, dark and light. Press `Ctrl+P`, type `theme` and press `Enter` to open **Change Theme**. Move through the list, or type part of a name: the whole app is redrawn in the theme you're on as you go. Type `tokyo` to try Tokyo Night.

{{ shot(src="tutorial/theme", alt="The Change Theme picker with tokyo typed and Tokyo Night chosen, the app behind it already drawn in Tokyo Night's colors", caption="The theme picker shows each theme live.") }}

`Enter` keeps the theme, and `Esc` goes back to the one you had. Every setting is described in [Settings](@/reference/settings.md), and the themes in [Appearance](@/reference/appearance.md).

## Where next

You've now used most of what you'll need every day. When you want more:

- **`?`** shows the keys for whatever you're doing right now.
- **`g` `?`** opens the **Keymap**, a full-screen cheat sheet of every key.
- **`Ctrl+P`** finds any command by name and shows its keys.
- The [Reference](@/reference/_index.md) explains every feature, topic by topic.
- The [cheat sheet](@/reference/cheatsheet.md) puts the keys you'll use most on one page.

{{ shot(src="tutorial/keymap", alt="The Keymap window over the app, listing every key binding in three columns under headings like Moving, Selection, Going, View and Panels, Settings and Attachments Panel", caption="The Keymap: every key on one screen.") }}

Then delete the sample file, run `tipp` on a file of your own, and start with the Inbox.
