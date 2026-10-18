+++
title = "Appearance"
weight = 140
description = "Themes and the theme picker, grayscale, icons, word wrap, animations, the weather and small terminals."
+++

How the app looks is set in the [settings window](@/reference/settings.md#the-settings-window) or `config.toml`, and every change shows at once. This page says what each choice does; the [settings page](@/reference/settings.md) lists their values and defaults.

## Themes

The app has twenty built-in themes, fifteen dark and five light, set by [Theme](@/reference/settings.md#setting-theme). The default, **Terminal Dark**, draws in the terminal's own colors: its 16 ANSI colors, with its own background and text color, and four grays of the 256-color palette for dim text, completed items and the selection. It follows whatever palette the terminal is set to and looks right in a terminal without 24-bit color. The other themes bring their own 24-bit colors, background included, from each theme's official palette, and want a terminal with truecolor support.

Areas, projects and sections each draw their titles in a color of their own, in every theme, wherever they show.

{{ themes() }}

## The theme picker

**Change Theme**, in the [command palette](@/reference/finders.md#the-command-palette) and on no key, opens a picker of the built-in themes, the dark ones under Dark Themes and then the light ones under Light Themes, filtered as you type, with the theme the app is drawn in highlighted. The whole app is drawn at once in the theme highlighted, without saving anything; while nothing matches, the last one shown stays. `Enter` keeps the theme chosen and writes it to `config.toml`; `Esc`, a click outside the picker or the Navigator's key goes back to the theme it opened on. The settings window's Theme setting turns through the same themes.

{{ keys(topic="appearance", titles="Change Theme") }}

## Grayscale

[Grayscale](@/reference/settings.md#setting-grayscale) draws whichever theme is chosen without any hue, keeping it light or dark. Each shade of the theme, its background, text, dim text, bands, rules and completed items, becomes the gray of the same lightness, so its contrasts hold; every colored role, such as titles, icons, date chips and tags, takes the text's gray, and only weight, italics and dimming set them apart. The weather falls in grays too.

## Icons

With [Icons](@/reference/settings.md#setting-icons) set to Nerd Font, the default, the app marks item kinds, the views, date chips, attachments, tags and the status line with [Nerd Font](https://www.nerdfonts.com) icons, which need a Nerd Font in the terminal. Set to plain, it draws plain Unicode markers that any font has instead:

| What | Plain marker |
|---|---|
| Inbox, Today, Upcoming, Flagged, All, Logbook, Trash | `⊔` `☼` `»` `⚑` `∷` `▤` `✕` |
| area | `◆` |
| start date, due date, overdue | `↦` `⇥` `!` |
| note, comments, checklist | `≡` `❞` `☑` |
| tag | `@` |
| navigate mode, edit mode | `↕` `✎` |

The task markers `☐` and `✔`, a project's `#` and a section's `/` are the same in both.

## Word wrap

With [Word wrap](@/reference/settings.md#setting-word-wrap) on, the default, a long title wraps onto as many lines as it needs, its tags with it, the date chips staying at the right of its first line. Off, every title takes one line, cut with `…` before its chips, and a description's first line takes at most one more.

## Animations

[Animations](@/reference/settings.md#setting-animations) set how much moves:

- **full**: the row the focus jumps to flashes, and so do the rows a command changed or brought in (pasting, duplicating, Move to, undo and redo), and the focused row when a command is refused. Completing, deleting, moving or adding a task fades its rows into place; a pane that opens or shows another place fades in; and the screen passes smoothly from how it was when a window, a panel or a pane opens or closes, or the theme changes.
- **reduced**: only the flashes, each in one step, without fades.
- **off**: nothing moves.

Animations never hold back input or layout: every character is in its final place from the first frame, and any key ends the running effects first. Terminal Dark, drawn in the terminal's own colors, cannot fade, so text that would fade is drawn dimmed for the first half instead.

Without a setting in `config.toml`, animations are full, or reduced when the app runs over SSH (when `SSH_CONNECTION` or `SSH_TTY` is set), where every frame crosses the network.

## Weather

After the app has had no input for half an hour, the season's weather falls over the whole screen, as in taskpaper-terminal: autumn leaves from September to November, snow from December to February and rain from March to May, piling up at the bottom, slowly. Summer has none. The next key, click, paste or resize ends it and does nothing else.

[Weather animation](@/reference/settings.md#setting-weather) turns it off, and [Weather after idle](@/reference/settings.md#setting-weather-idle-minutes) sets the minutes it waits. It starts by itself only while animations are full.

**Start Weather Animation**, in the command palette and on no key, starts it at once, whatever the settings say; in summer the status line says `no weather in summer` instead.

{{ keys(topic="appearance", titles="Start Weather Animation") }}

## Small terminals

The app is made for a terminal of at least 80 columns and 24 rows, and keeps working in less room. The sidebar takes at most a third of the screen, the [shortcuts panel](@/reference/keyboard.md#the-shortcuts-panel) is drawn over the pane below 80 columns and left out below 40 columns or 9 rows, an [attachments panel](@/reference/attachments.md#width) covers a pane narrower than 40 columns, and [panes](@/reference/panes.md#when-panes-don-t-fit) that do not fit scroll sideways.
