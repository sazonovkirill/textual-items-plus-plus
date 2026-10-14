# Built-in Themes

- Status: Reference
- Last updated: 2026-10-14

This page defines the color themes compiled into the app for v1: two dark and two light. Each was chosen on its own merits from what is popular and maintained in 2025-2026 across Neovim (the dotfyle usage ranking), terminal emulators (all four ship in Ghostty's bundled catalog, which comes from iTerm2-Color-Schemes), and editors. Each one has a permissive license, verified in its own repository, so its palette can be built into the binary with attribution. Every hex value below comes from the theme's official palette file. The app also keeps its "Terminal Dark" theme, which uses the terminal's own 16 ANSI colors; its roles are listed [after the four](#terminal-dark). Candidates evaluated for later releases are listed at the end, without palettes.

## Selection

| # | Theme | Light/dark | Origin repo | License (copyright holder) | Why |
|---|---|---|---|---|---|
| 1 | Catppuccin Mocha | dark | [catppuccin/palette](https://github.com/catppuccin/palette) | MIT, © 2021 Catppuccin | Ranked #1 in Neovim usage on dotfyle and ported to almost every tool. Its pastel accents stay well above 7:1 on its base background, and it has a large, well-documented palette with clear role guidance. |
| 2 | Tokyo Night (Night) | dark | [folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim) | Apache-2.0, © Folke Lemaitre (the LICENSE does not fill in the holder, so attribute the repo author and ship the license text) | Ranked #2 on dotfyle and the default look of many modern Neovim setups. It is a cooler, more saturated neon-blue counterpart to Mocha, and all its UI roles pass 4.5:1 except the faded checklist-done grey. |
| 3 | GitHub Light Default | light | [primer/github-vscode-theme](https://github.com/primer/github-vscode-theme) (palette from [primer/primitives](https://github.com/primer/primitives)) | MIT, © 2020 Primer / © 2018 GitHub Inc. | The most familiar light UI in software. Primer's accessibility-tuned scales give the only candidate where every role, including all six tag colors, clears WCAG AA on white. |
| 4 | Flexoki Light | light | [kepano/flexoki](https://github.com/kepano/flexoki) | MIT, © 2023 Steph Ango | A modern "ink on paper" palette (2023) by the creator of Obsidian, with ports to Ghostty, Neovim, VS Code and more. It takes the warm, paper-like spot Solarized Light used to hold, with near-black text (18.6:1) and a full 50-950 extended scale for fine contrast adjustments. |

## Roles

Each theme provides an exact hex value for every role below. Roles are foreground colors drawn on **Background** unless marked as a background. The contrast column in each theme table is WCAG 2.x contrast against Background. For the two band roles, it is Text drawn on that band.

| Role | Kind | Use |
|---|---|---|
| Background | background | Screen fill behind everything. |
| Text | foreground | Item titles and body text. Must reach 4.5:1. |
| Dim text | foreground | Notes, secondary metadata, hints, help text. Must reach 4.5:1. |
| Primary | foreground | App title, active pane border, key hints, the main brand color; also the titles of projects and the Inbox. |
| Accent | foreground | Secondary emphasis: counts, search matches, mode indicator. |
| Selection | background | Fill of the selected row(s). Text must stay at least 4.5:1 on it. |
| Focus marker | foreground | The glyph marking the focused row or pane, such as `▌` or `›`. |
| Current-line band | background | Subtle fill of the cursor row when it is not selected, and the zebra or hover band. |
| Headers | foreground | Area titles, date-group headings, checklist progress and the attachment icons. |
| Sections | foreground | Section titles: the theme's yellow, Tag 3's color, so that areas, projects and sections never share a color. |
| Rule lines | foreground (non-text) | Separators and box borders. 3:1 is not required, but rules must stay visible on the current-line band. |
| Links | foreground | URLs and note links (also underlined). |
| Start-date chip | foreground | Start/defer date label. A green hue. |
| Due-date chip | foreground | Deadline label. A red hue. |
| Overdue | foreground | Past-due date label and overdue counter. It should be distinguishable from the due chip by hue or, when the palette has only one red, by bold or reverse video. |
| Error | foreground | Error messages and validation failures in the status line. |
| Tag text | foreground | Default color of `@tag` labels that have no assigned tag color. |
| Checklist done | foreground | Checked checklist items and completed tasks. They are intentionally faded and may sit slightly below 4.5:1, but never below 3:1. |
| Tag 1-6 | foreground | Six user-assignable tag/label colors, in this order: blue, green, yellow, orange, pink/red, purple. Each theme uses its nearest palette hue. |
| View icons | foreground (non-text) | The icons of the Inbox and the views in the sidebar and the Navigator, each in a hue of its own, as the owner chose: Inbox blue, Today yellow, Upcoming red, All green, Logbook purple, Trash grey (D-78), and Flagged the Flag role's red. Each theme uses its nearest palette hue, and each clears 3:1. |
| Flag | foreground (non-text) | The flag in front of a flagged item's title, and the Flagged view's icon: a red, as the owner asked, apart from the Upcoming icon's so that each view keeps a hue of its own (D-101). It clears 3:1. |

## 1. Catppuccin Mocha

- Key: `catppuccin-mocha` · dark
- Palette source: [`catppuccin/palette` · `palette.json` → `mocha`](https://github.com/catppuccin/palette/blob/main/palette.json)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#1e1e2e` | base | — |
| Text | `#cdd6f4` | text | 11.34:1 |
| Dim text | `#a6adc8` | subtext0 | 7.37:1 |
| Primary | `#cba6f7` | mauve | 8.07:1 |
| Accent | `#fab387` | peach | 9.27:1 |
| Selection | `#45475a` | surface1 | text on it 6.31:1 |
| Focus marker | `#f5c2e7` | pink | 10.74:1 |
| Current-line band | `#313244` | surface0 | text on it 8.69:1 |
| Headers | `#89b4fa` | blue | 7.79:1 |
| Sections | `#f9e2af` | yellow | 12.91:1 |
| Rule lines | `#585b70` | surface2 | 2.46:1 (non-text) |
| Links | `#74c7ec` | sapphire | 8.69:1 |
| Start-date chip | `#a6e3a1` | green | 11.03:1 |
| Due-date chip | `#eba0ac` | maroon | 7.93:1 |
| Overdue | `#f38ba8` | red | 7.08:1 |
| Error | `#f38ba8` | red | 7.08:1 |
| Tag text | `#94e2d5` | teal | 11.01:1 |
| Checklist done | `#7f849c` | overlay1 | 4.44:1 **(<4.5)** |
| Tag 1 | `#89b4fa` | blue | 7.79:1 |
| Tag 2 | `#a6e3a1` | green | 11.03:1 |
| Tag 3 | `#f9e2af` | yellow | 12.91:1 |
| Tag 4 | `#fab387` | peach | 9.27:1 |
| Tag 5 | `#f5c2e7` | pink | 10.74:1 |
| Tag 6 | `#cba6f7` | mauve | 8.07:1 |
| Inbox icon | `#89b4fa` | blue | 7.79:1 |
| Today icon | `#f9e2af` | yellow | 12.91:1 |
| Upcoming icon | `#f38ba8` | red | 7.08:1 |
| Flag | `#eba0ac` | maroon | 7.93:1 |
| All icon | `#a6e3a1` | green | 11.03:1 |
| Logbook icon | `#cba6f7` | mauve | 8.07:1 |
| Trash icon | `#9399b2` | overlay2 | 5.81:1 |

Notes: `subtext0` is used for dim text rather than `overlay0`/`overlay1` (the comment greys) so secondary text clears 4.5:1. Checklist done uses `overlay1` at 4.44:1, deliberately just below body-text level.

## 2. Tokyo Night (Night)

- Key: `tokyo-night` · dark
- Palette source: [`folke/tokyonight.nvim` · `lua/tokyonight/colors/storm.lua`](https://github.com/folke/tokyonight.nvim/blob/main/lua/tokyonight/colors/storm.lua), with the `bg*` overrides in [`night.lua`](https://github.com/folke/tokyonight.nvim/blob/main/lua/tokyonight/colors/night.lua) and `bg_visual` from [`init.lua`](https://github.com/folke/tokyonight.nvim/blob/main/lua/tokyonight/colors/init.lua) (`blend_bg(blue0, 0.4)` = `#283457`)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#1a1b26` | bg | — |
| Text | `#c0caf5` | fg | 10.59:1 |
| Dim text | `#a9b1d6` | fg_dark | 8.10:1 |
| Primary | `#7aa2f7` | blue | 6.79:1 |
| Accent | `#ff9e64` | orange | 8.40:1 |
| Selection | `#283457` | bg_visual (blend of blue0 into bg; tokyonight theme spec) | text on it 7.57:1 |
| Focus marker | `#bb9af7` | magenta | 7.39:1 |
| Current-line band | `#292e42` | bg_highlight | text on it 8.32:1 |
| Headers | `#7dcfff` | cyan | 9.96:1 |
| Sections | `#e0af68` | yellow | 8.55:1 |
| Rule lines | `#3b4261` | fg_gutter | 1.74:1 (non-text) |
| Links | `#2ac3de` | blue1 | 8.11:1 |
| Start-date chip | `#9ece6a` | green | 9.35:1 |
| Due-date chip | `#f7768e` | red | 6.46:1 |
| Overdue | `#ff007c` | magenta2 | 4.51:1 |
| Error | `#f7768e` | red | 6.46:1 |
| Tag text | `#73daca` | green1 | 10.26:1 |
| Checklist done | `#737aa2` | dark5 | 4.10:1 **(<4.5)** |
| Tag 1 | `#7aa2f7` | blue | 6.79:1 |
| Tag 2 | `#9ece6a` | green | 9.35:1 |
| Tag 3 | `#e0af68` | yellow | 8.55:1 |
| Tag 4 | `#ff9e64` | orange | 8.40:1 |
| Tag 5 | `#bb9af7` | magenta | 7.39:1 |
| Tag 6 | `#7dcfff` | cyan | 9.96:1 |
| Inbox icon | `#7aa2f7` | blue | 6.79:1 |
| Today icon | `#e0af68` | yellow | 8.55:1 |
| Upcoming icon | `#f7768e` | red | 6.46:1 |
| Flag | `#db4b4b` | red1 | 4.17:1 |
| All icon | `#9ece6a` | green | 9.35:1 |
| Logbook icon | `#9d7cd8` | purple | 5.13:1 |
| Trash icon | `#737aa2` | dark5 | 4.10:1 |

Notes: dim text is `fg_dark`, not `comment` (`#565f89`, 2.76:1). Overdue uses `magenta2` (`#ff007c`), the theme's hot accent, so it stands apart from the normal red due chip. `red1` (`#db4b4b`, the theme's error color in diagnostics) is only 4.17:1, so error uses `red` instead.

## 3. GitHub Light Default

- Key: `github-light` · light
- Palette source: [`@primer/primitives`](https://github.com/primer/primitives) light scale, as vendored in [`projekt0n/github-nvim-theme` · `lua/github-theme/palette/primitives/light.lua`](https://github.com/projekt0n/github-nvim-theme/blob/main/lua/github-theme/palette/primitives/light.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#ffffff` | canvas.default | — |
| Text | `#1f2328` | fg.default | 15.80:1 |
| Dim text | `#656d76` | fg.muted | 5.25:1 |
| Primary | `#0969da` | accent.fg | 5.19:1 |
| Accent | `#8250df` | done.fg | 5.05:1 |
| Selection | `#ddf4ff` | accent.subtle / blue.0 | text on it 13.89:1 |
| Focus marker | `#bc4c00` | severe.fg | 5.03:1 |
| Current-line band | `#f6f8fa` | canvas.subtle | text on it 14.84:1 |
| Headers | `#0550ae` | blue.6 | 7.59:1 |
| Sections | `#9a6700` | yellow.5 | 4.87:1 |
| Rule lines | `#d0d7de` | border.default | 1.45:1 (non-text) |
| Links | `#0969da` | accent.fg | 5.19:1 |
| Start-date chip | `#1a7f37` | success.fg | 5.08:1 |
| Due-date chip | `#cf222e` | red.5 | 5.36:1 |
| Overdue | `#a40e26` | red.6 | 7.87:1 |
| Error | `#d1242f` | danger.fg | 5.24:1 |
| Tag text | `#953800` | orange.6 | 7.39:1 |
| Checklist done | `#6e7781` | fg.subtle | 4.55:1 |
| Tag 1 | `#0969da` | blue.5 | 5.19:1 |
| Tag 2 | `#1a7f37` | green.5 | 5.08:1 |
| Tag 3 | `#9a6700` | yellow.5 | 4.87:1 |
| Tag 4 | `#bc4c00` | orange.5 | 5.03:1 |
| Tag 5 | `#bf3989` | pink.5 | 5.05:1 |
| Tag 6 | `#8250df` | purple.5 | 5.05:1 |
| Inbox icon | `#0969da` | blue.5 | 5.19:1 |
| Today icon | `#9a6700` | yellow.5 | 4.87:1 |
| Upcoming icon | `#cf222e` | red.5 | 5.36:1 |
| Flag | `#a40e26` | red.7 | 7.87:1 |
| All icon | `#1a7f37` | green.5 | 5.08:1 |
| Logbook icon | `#8250df` | purple.5 | 5.05:1 |
| Trash icon | `#6e7781` | fg.subtle | 4.55:1 |

Notes: every role clears 4.5:1. Checklist done uses `fg.subtle`, which happens to pass as well.

## 4. Flexoki Light

- Key: `flexoki-light` · light
- Palette source: [`kepano/flexoki` · `README.md` → Colors / Extended palette](https://github.com/kepano/flexoki#colors). Role names follow Flexoki's own light-theme mapping (`bg`, `bg-2`, `ui-2`, `ui-3`, `tx`, `tx-2`).

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#fffcf0` | paper (bg) | — |
| Text | `#100f0f` | black (tx) | 18.62:1 |
| Dim text | `#6f6e69` | base-600 (tx-2) | 4.97:1 |
| Primary | `#205ea6` | blue-600 | 6.36:1 |
| Accent | `#bc5215` | orange-600 | 4.69:1 |
| Selection | `#dad8ce` | base-150 (ui-2) | text on it 13.39:1 |
| Focus marker | `#a02f6f` | magenta-600 | 6.53:1 |
| Current-line band | `#f2f0e5` | base-50 (bg-2) | text on it 16.74:1 |
| Headers | `#5e409d` | purple-600 | 7.58:1 |
| Sections | `#8e6b01` | yellow-700 | 4.80:1 |
| Rule lines | `#cecdc3` | base-200 (ui-3) | 1.55:1 (non-text) |
| Links | `#24837b` | cyan-600 | 4.43:1 **(<4.5)** |
| Start-date chip | `#66800b` | green-600 | 4.39:1 **(<4.5)** |
| Due-date chip | `#af3029` | red-600 | 6.23:1 |
| Overdue | `#942822` | red-700 | 7.90:1 |
| Error | `#af3029` | red-600 | 6.23:1 |
| Tag text | `#24837b` | cyan-600 | 4.43:1 **(<4.5)** |
| Checklist done | `#878580` | base-500 | 3.59:1 **(<4.5)** |
| Tag 1 | `#205ea6` | blue-600 | 6.36:1 |
| Tag 2 | `#66800b` | green-600 | 4.39:1 **(<4.5)** |
| Tag 3 | `#8e6b01` | yellow-700 | 4.80:1 |
| Tag 4 | `#bc5215` | orange-600 | 4.69:1 |
| Tag 5 | `#a02f6f` | magenta-600 | 6.53:1 |
| Tag 6 | `#5e409d` | purple-600 | 7.58:1 |
| Inbox icon | `#205ea6` | blue-600 | 6.36:1 |
| Today icon | `#8e6b01` | yellow-700 | 4.80:1 |
| Upcoming icon | `#af3029` | red-600 | 6.23:1 |
| Flag | `#d14d41` | red-400 | 4.21:1 |
| All icon | `#66800b` | green-600 | 4.39:1 |
| Logbook icon | `#5e409d` | purple-600 | 7.58:1 |
| Trash icon | `#878580` | base-500 | 3.59:1 |

Notes: the yellow tag uses `yellow-700` from the extended palette because `yellow-600` is 3.39:1 on paper. Cyan-600 and green-600 sit just under 4.5:1 (4.43 and 4.39). Their darker 700 steps are available if the app wants AA for every role.
## Terminal Dark

- Key: `terminal-dark` · the terminal's own colors, the default until settings choose a theme (D-30)

It draws in the terminal's 16 ANSI colors, so it follows the user's palette and needs no truecolor. Where the 16 have no color that fits, it uses greys of the 256-color palette, as taskpaper-terminal does. Their contrast depends on the terminal's palette, so none is given.

| Role | Color |
|---|---|
| Background | the terminal's own |
| Text | the terminal's own |
| Dim text | grey 248 |
| Primary | bright blue |
| Accent | yellow |
| Selection | grey 237 |
| Focus marker | bright blue |
| Current-line band | black |
| Text on the band and selection | grey 255, light whatever the palette |
| Headers | bright magenta |
| Sections | yellow |
| Rule lines | bright black |
| Links | cyan |
| Start-date chip | green |
| Due-date chip | red |
| Overdue | bright red |
| Error | red |
| Tag text | cyan |
| Checklist done | grey 244 |
| Inbox icon | bright blue |
| Today icon | yellow |
| Upcoming icon | red |
| Flag | bright red |
| All icon | green |
| Logbook icon | magenta |
| Trash icon | grey 244 |

## Contrast summary

- Text and dim text clear 4.5:1 in all four themes. The lowest is dim text in Flexoki Light at 4.97:1.
- The roles below 4.5:1 are deliberate or marginal:
  - Checklist done is faded on purpose: Mocha 4.44, Tokyo Night 4.10, Flexoki 3.59.
  - Flexoki `cyan-600` (links and tag text, 4.43) and `green-600` (start chip and tag 2, 4.39) are marginal. Swap in `cyan-700` (`#1c6c66`) and `green-700` (`#536907`) from the extended palette if strict AA is required.
- GitHub Light Default passes every role.

## Later

These candidates were evaluated for a future release. Their licenses are verified permissive. Palettes are intentionally not included yet.

- **Rosé Pine / Rosé Pine Dawn** (MIT, © 2023 Rosé Pine). Hugely popular (dotfyle #4) and pairs well. The Dawn variant's dim text (4.02:1) and accents (2.0-3.9:1) need bold or reverse chips.
- **Kanagawa Wave** (MIT, © 2021 Tommaso Laurenzi). Dotfyle #3, with a distinctive ink-painting palette. Use `springViolet1` for dim text and `peachRed` for errors, because the stock comment and error colors fall below 4.5:1.
- **Catppuccin Latte** (MIT). The natural light partner for Mocha. Several accents (green, yellow, peach, pink) are under 3:1 on `base`, so tags and chips would need bold or reverse rendering.
- **Gruvbox Material Dark** (MIT, © 2020 sainnhe) and **Everforest Dark** (MIT, © 2019 sainnhe). Warm, low-glare dark themes, both in the dotfyle top 10.
- **GitHub Dark Default** (MIT). A strong-contrast dark partner for GitHub Light.
- **Carbonfox** (MIT, © 2021 James Simpson). A near-black, neutral IBM Carbon look. Its yellow and orange slots are teal, which weakens tag separation.
- **Alucard** (Dracula light, MIT, © Dracula Theme). Every role passes AA. **Dracula** itself is iconic, but its only grey (`Comment`) is 3.03:1.
- **Nord** (MIT, © 2016-present Sven Greb, per the `license` file on the default `develop` branch). Its comment grey `nord3` is 1.69:1 and its red `nord11` is 3.05:1.

## Rejected candidates

- **Modus Operandi / Vivendi.** Best-in-class contrast, but GPL-3.0, so not permissive.
- **Monokai Pro, Dracula PRO.** Proprietary licenses.
- **Vesper.** No license file could be verified in a public repository.
- **Solarized Light** (MIT). Body text `base00` is 4.13:1 and accents are 2.9-4.3:1 on `base3`. Flexoki Light covers the same warm-paper niche with far better contrast.
- **Everforest Light** (MIT). Accents are 2.1-3.1:1 on `bg0`, too faint for chips and tags.
- **Tokyo Night Day.** There is no static palette: `day.lua` inverts the night palette at runtime, so there is no official hex to cite.
- **Ayu Light / Mirage** (MIT). The canonical palette is generated at build time (`src/generated` is not committed), and the older `ayu-vim` values differ, so there is no single source of truth.
- **One Dark / One Dark Pro** (MIT). Still popular, but visually close to Tokyo Night and GitHub Dark and older in style.
- **Oxocarbon** (MIT). Nearly the same IBM Carbon palette as Carbonfox.
- **Cyberdream** (MIT). Rising (dotfyle #10), but its neon look overlaps with Tokyo Night.
- **Sibling variants** (Catppuccin Frappé/Macchiato, Rosé Pine Moon, Tokyo Night Storm/Moon, Kanagawa Dragon). Too similar to a chosen or listed sibling.

## Sources

- Catppuccin palette and license: https://github.com/catppuccin/palette (`palette.json`, `LICENSE`)
- Tokyo Night: https://github.com/folke/tokyonight.nvim (`lua/tokyonight/colors/{storm,night,init,day}.lua`, `LICENSE`)
- GitHub/Primer: https://github.com/primer/github-vscode-theme (`LICENSE`), https://github.com/primer/primitives (`LICENSE`), https://github.com/projekt0n/github-nvim-theme (`lua/github-theme/palette/primitives/light.lua`)
- Flexoki: https://github.com/kepano/flexoki (`README.md` Colors and Extended palette, `LICENSE`), https://stephango.com/flexoki
- Neovim colorscheme usage ranking: https://dotfyle.com/neovim/colorscheme/top
- Ghostty theme catalog: https://ghostty.org/docs/features/theme and https://github.com/mbadolato/iTerm2-Color-Schemes (`ghostty/` contains every theme named on this page)
- Later and rejected candidates: https://github.com/rose-pine/neovim, https://github.com/rebelot/kanagawa.nvim, https://github.com/sainnhe/gruvbox-material, https://github.com/sainnhe/everforest, https://github.com/EdenEast/nightfox.nvim, https://github.com/dracula/dracula-theme, https://draculatheme.com/spec, https://github.com/nordtheme/nord, https://github.com/altercation/solarized, https://github.com/ayu-theme/ayu-colors
- WCAG 2.2 contrast ratio definition: https://www.w3.org/TR/WCAG22/#dfn-contrast-ratio
