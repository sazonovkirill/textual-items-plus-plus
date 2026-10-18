+++
title = "Settings"
weight = 150
description = "The settings window, the config.toml file it keeps, and every setting with its values and default."
+++

The app's settings are kept in one TOML file, `config.toml`, which the **settings window** changes for you and which you can also edit by hand. Without the file the app runs on its defaults. What each setting does is explained on the [Appearance](@/reference/appearance.md) page.

## The settings window

**Open Settings** (`g,`, **g**o to the settings, `,` as macOS apps open theirs), also in the command palette, opens a window listing every setting under its group, each with its value at the right and a one-line description under it. A `*` before a setting marks one whose value is not its default. The window's last line shows where the settings file is.

`j`/`k` move between settings; `l` or `h` (also `→`/`←`, and `Enter` or `Space` for the next) turn the highlighted setting to its next or previous value, going round its choices, or for the idle minutes a minute up or down, no lower than 1. `r` puts it back to its default. Every change applies at once, with the app drawn anew behind the window, and is written to `config.toml` straight away. `Esc` closes the window.

{{ shot(src="reference-2/settings", alt="The settings window over the Inbox, listing under Appearance Theme set to Terminal Dark, Grayscale Off, Icons Nerd Font, Word wrap On, Animations Off marked with a star, and Display task counts On, and under Weather Weather animation Off marked with a star and Weather after idle 30 minutes, each with a one-line description, and the path of config.toml on the last line", caption="The settings window; the stars mark the two settings changed from their defaults.") }}

{{ keys(topic="settings") }}

{{ window_keys(window="Settings") }}

## The config.toml file

The file is `$XDG_CONFIG_HOME/textual-items-plus-plus/config.toml`, or `~/.config/textual-items-plus-plus/config.toml` when `XDG_CONFIG_HOME` is not set, on macOS too.

**Reading.** The app reads the file when it starts. Each setting is read on its own, so a problem with one leaves the others in force, and the app always starts:

- A setting with a value it cannot take keeps its default.
- A key the app has no setting for is ignored.
- A file that is not valid TOML, or cannot be read, gives every setting its default.

Each problem is reported in the status line, after `config.toml:`, as ``config.toml: `word_wrap`: invalid type: string "yes", expected a boolean; unknown setting `foo` ``. Changes made to the file while the app runs take effect at its next start.

**Writing.** Changing a setting in the settings window or the [theme picker](@/reference/appearance.md#the-theme-picker) rewrites only that setting's line. The file's comments, the order of its lines, keys the app does not know and invalid values of other settings are all kept, and so is a comment after the value replaced. A missing file, and its folder, are created. The file is replaced in one step, never seen half written, and a link to it stays a link. When it cannot be written, for instance because it is not valid TOML, it is left as it was and the status line says why, as ``config.toml: `theme` not saved: …``.

Key bindings cannot be changed in the file.

## Appearance

{{ settings(group="Appearance") }}

Theme, grayscale, icons, word wrap and animations are explained under [Appearance](@/reference/appearance.md). **Display task counts** shows, at the right of each entry of the [sidebar](@/reference/views.md), how many tasks it lists. The default for animations is `"reduced"` when the app runs over SSH.

## Weather

{{ settings(group="Weather") }}

See [the weather](@/reference/appearance.md#weather).

## Example

```toml
# ~/.config/textual-items-plus-plus/config.toml
theme = "tokyo-night"
grayscale = false
icons = "plain"        # no Nerd Font in this terminal
word_wrap = true
animations = "reduced"
task_counts = true

weather = true
weather_idle_minutes = 10
```
