# Built-in Themes

- Status: Reference
- Last updated: 2026-10-15

This page defines the color themes compiled into the app: the four of v1, two dark and two light, and fifteen more added on 2026-10-15 (D-125), twelve dark and three light. Each was chosen on its own merits from what is popular and maintained in 2025-2026 across Neovim (the dotfyle usage and trending rankings), terminal emulators (Ghostty's bundled catalog, which comes from iTerm2-Color-Schemes), and editors. Each one has a permissive license, verified in its own repository, so its palette can be built into the binary with attribution. Every hex value below comes from the theme's official palette file. The app also keeps its "Terminal Dark" theme, which uses the terminal's own 16 ANSI colors; its roles are listed [after the others](#terminal-dark). Candidates evaluated and set aside are listed at the end, without palettes.

A theme is taken only when its palette fills every role below within the contrast the app's tests hold it to: text and dim text at least 4.5:1, text on both bands at least 4.5:1, checklist done and each view icon at least 3:1, and seven view-icon colors that differ. That rule is what kept out several popular light themes, whose yellows or greys are too faint on their paper (see [Rejected candidates](#rejected-candidates)).

## Selection

| # | Theme | Light/dark | Origin repo | License (copyright holder) | Why |
|---|---|---|---|---|---|
| 1 | Catppuccin Mocha | dark | [catppuccin/palette](https://github.com/catppuccin/palette) | MIT, © 2021 Catppuccin | Ranked #1 in Neovim usage on dotfyle and ported to almost every tool. Its pastel accents stay well above 7:1 on its base background, and it has a large, well-documented palette with clear role guidance. |
| 2 | Tokyo Night (Night) | dark | [folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim) | Apache-2.0, © Folke Lemaitre (the LICENSE does not fill in the holder, so attribute the repo author and ship the license text) | Ranked #2 on dotfyle and the default look of many modern Neovim setups. It is a cooler, more saturated neon-blue counterpart to Mocha, and all its UI roles pass 4.5:1 except the faded checklist-done grey. |
| 3 | GitHub Light Default | light | [primer/github-vscode-theme](https://github.com/primer/github-vscode-theme) (palette from [primer/primitives](https://github.com/primer/primitives)) | MIT, © 2020 Primer / © 2018 GitHub Inc. | The most familiar light UI in software. Primer's accessibility-tuned scales give the only candidate where every role, including all six tag colors, clears WCAG AA on white. |
| 4 | Flexoki Light | light | [kepano/flexoki](https://github.com/kepano/flexoki) | MIT, © 2023 Steph Ango | A modern "ink on paper" palette (2023) by the creator of Obsidian, with ports to Ghostty, Neovim, VS Code and more. It takes the warm, paper-like spot Solarized Light used to hold, with near-black text (18.6:1) and a full 50-950 extended scale for fine contrast adjustments. |
| 5 | Rosé Pine | dark | [rose-pine/neovim](https://github.com/rose-pine/neovim) | MIT, © 2023 Rosé Pine | Dotfyle #4 and one of the most ported themes. Soft pine-and-rose pastels on a deep violet night; its own name for each color says its role. |
| 6 | Kanagawa Wave | dark | [rebelot/kanagawa.nvim](https://github.com/rebelot/kanagawa.nvim) | MIT, © 2021 Tommaso Laurenzi | Dotfyle #3. Inspired by Hokusai's *The Great Wave*: ink blues and muted woodblock accents. |
| 7 | Kanagawa Dragon | dark | [rebelot/kanagawa.nvim](https://github.com/rebelot/kanagawa.nvim) | MIT, © 2021 Tommaso Laurenzi | Kanagawa's own dark variant with a separate palette: warm ink black and earthy, low-saturation accents, a calmer look than Wave. |
| 8 | Gruvbox Dark | dark | [ellisonleao/gruvbox.nvim](https://github.com/ellisonleao/gruvbox.nvim) | MIT, © 2021 NpX (palette from [morhetz/gruvbox](https://github.com/morhetz/gruvbox), MIT) | The retro-groove classic, still in every terminal's theme list and in Ghostty's catalog. |
| 9 | Everforest Dark | dark | [sainnhe/everforest](https://github.com/sainnhe/everforest) | MIT, © 2019 sainnhe | Dotfyle #9. A green, low-glare forest palette built for long sessions. |
| 10 | GitHub Dark Default | dark | [primer/github-vscode-theme](https://github.com/primer/github-vscode-theme) (palette from [primer/primitives](https://github.com/primer/primitives)) | MIT, © 2020 Primer / © 2018 GitHub Inc. | Dotfyle #8 (github-nvim-theme). The familiar GitHub dark UI, the dark partner of GitHub Light Default. |
| 11 | Flexoki Dark | dark | [kepano/flexoki](https://github.com/kepano/flexoki) | MIT, © 2023 Steph Ango | The dark side of Flexoki's ink-on-paper palette, built into Ghostty and Obsidian themes. |
| 12 | Melange Dark | dark | [savq/melange-nvim](https://github.com/savq/melange-nvim) | MIT, © 2021 Sergio Alejandro Vargas | Dotfyle #23. A warm, cozy palette of browns and soft accents, made for the evening. |
| 13 | Moonfly | dark | [bluz71/vim-moonfly-colors](https://github.com/bluz71/vim-moonfly-colors) | MIT, © 2017-present vim-moonfly-colors authors | Dotfyle #21. A crisp charcoal theme on near-black with bright, well-separated accents. |
| 14 | Nightfly | dark | [bluz71/vim-nightfly-colors](https://github.com/bluz71/vim-nightfly-colors) | MIT, © 2020-present vim-nightfly-colors authors | Moonfly's midnight-blue sibling in the spirit of Night Owl, with a palette of its own. |
| 15 | Cyberdream | dark | [scottmckendry/cyberdream.nvim](https://github.com/scottmckendry/cyberdream.nvim) | MIT, © 2023 Scott McKendry | Dotfyle #10 and rising. A high-contrast futuristic neon look. |
| 16 | Synthwave '84 | dark | [robb0wen/synthwave-vscode](https://github.com/robb0wen/synthwave-vscode) | MIT, © 2019 Robb Owen | The retro-futurist outrun theme, one of the most installed VS Code themes, on a purple night sky. |
| 17 | Gruvbox Light | light | [ellisonleao/gruvbox.nvim](https://github.com/ellisonleao/gruvbox.nvim) | MIT, © 2021 NpX (palette from [morhetz/gruvbox](https://github.com/morhetz/gruvbox), MIT) | Gruvbox's warm parchment light mode; its `faded_*` accents keep tags and chips readable on cream. |
| 18 | Melange Light | light | [savq/melange-nvim](https://github.com/savq/melange-nvim) | MIT, © 2021 Sergio Alejandro Vargas | Melange's light mode: warm brown ink on soft grey paper. |
| 19 | Dayfox | light | [EdenEast/nightfox.nvim](https://github.com/EdenEast/nightfox.nvim) | MIT, © 2021 James Simpson | Dotfyle #5 family (Nightfox). A warm light theme whose deep accents all clear 3:1. |

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

## 5. Rosé Pine

- Key: `rose-pine` · dark
- Palette source: [`rose-pine/neovim` · `lua/rose-pine/palette.lua` → `main`](https://github.com/rose-pine/neovim/blob/main/lua/rose-pine/palette.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#191724` | base | — |
| Text | `#e0def4` | text | 13.39:1 |
| Dim text | `#908caa` | subtle | 5.48:1 |
| Primary | `#c4a7e7` | iris | 8.43:1 |
| Accent | `#ebbcba` | rose | 10.45:1 |
| Selection | `#403d52` | highlight_med | text on it 7.93:1 |
| Focus marker | `#9ccfd8` | foam | 10.37:1 |
| Current-line band | `#26233a` | overlay | text on it 11.51:1 |
| Headers | `#9ccfd8` | foam | 10.37:1 |
| Sections | `#f6c177` | gold | 10.77:1 |
| Rule lines | `#524f67` | highlight_high | 2.25:1 (non-text) |
| Links | `#9ccfd8` | foam | 10.37:1 |
| Start-date chip | `#95b1ac` | leaf | 7.70:1 |
| Due-date chip | `#ebbcba` | rose | 10.45:1 |
| Overdue | `#eb6f92` | love | 6.07:1 |
| Error | `#eb6f92` | love | 6.07:1 |
| Tag text | `#9ccfd8` | foam | 10.37:1 |
| Checklist done | `#6e6a86` | muted | 3.42:1 **(<4.5)** |
| Tag 1 | `#9ccfd8` | foam | 10.37:1 |
| Tag 2 | `#95b1ac` | leaf | 7.70:1 |
| Tag 3 | `#f6c177` | gold | 10.77:1 |
| Tag 4 | `#ebbcba` | rose | 10.45:1 |
| Tag 5 | `#eb6f92` | love | 6.07:1 |
| Tag 6 | `#c4a7e7` | iris | 8.43:1 |
| Inbox icon | `#9ccfd8` | foam | 10.37:1 |
| Today icon | `#f6c177` | gold | 10.77:1 |
| Upcoming icon | `#eb6f92` | love | 6.07:1 |
| Flag | `#ebbcba` | rose | 10.45:1 |
| All icon | `#95b1ac` | leaf | 7.70:1 |
| Logbook icon | `#c4a7e7` | iris | 8.43:1 |
| Trash icon | `#6e6a86` | muted | 3.42:1 |

Notes: Rosé Pine has no green of its own: `leaf`, its muted sage, takes the start chip, Tag 2 and the All icon. With one red, the due chip takes `rose` and overdue `love`, which the app also draws bold and reversed.

## 6. Kanagawa Wave

- Key: `kanagawa-wave` · dark
- Palette source: [`rebelot/kanagawa.nvim` · `lua/kanagawa/colors.lua`](https://github.com/rebelot/kanagawa.nvim/blob/master/lua/kanagawa/colors.lua), roles as `lua/kanagawa/themes.lua` maps them for `wave`

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#1f1f28` | sumiInk3 | — |
| Text | `#dcd7ba` | fujiWhite | 11.26:1 |
| Dim text | `#938aa9` | springViolet1 | 5.02:1 |
| Primary | `#7e9cd8` | crystalBlue | 5.94:1 |
| Accent | `#ffa066` | surimiOrange | 8.15:1 |
| Selection | `#223249` | waveBlue1 | text on it 8.93:1 |
| Focus marker | `#d27e99` | sakuraPink | 5.59:1 |
| Current-line band | `#363646` | sumiInk5 | text on it 8.16:1 |
| Headers | `#7fb4ca` | springBlue | 7.23:1 |
| Sections | `#e6c384` | carpYellow | 9.73:1 |
| Rule lines | `#54546d` | sumiInk6 | 2.23:1 (non-text) |
| Links | `#7fb4ca` | springBlue | 7.23:1 |
| Start-date chip | `#98bb6c` | springGreen | 7.52:1 |
| Due-date chip | `#e46876` | waveRed | 5.09:1 |
| Overdue | `#ff5d62` | peachRed | 5.44:1 |
| Error | `#ff5d62` | peachRed | 5.44:1 |
| Tag text | `#7aa89f` | waveAqua2 | 6.17:1 |
| Checklist done | `#727169` | fujiGray | 3.33:1 **(<4.5)** |
| Tag 1 | `#7e9cd8` | crystalBlue | 5.94:1 |
| Tag 2 | `#98bb6c` | springGreen | 7.52:1 |
| Tag 3 | `#e6c384` | carpYellow | 9.73:1 |
| Tag 4 | `#ffa066` | surimiOrange | 8.15:1 |
| Tag 5 | `#d27e99` | sakuraPink | 5.59:1 |
| Tag 6 | `#957fb8` | oniViolet | 4.67:1 |
| Inbox icon | `#7e9cd8` | crystalBlue | 5.94:1 |
| Today icon | `#e6c384` | carpYellow | 9.73:1 |
| Upcoming icon | `#e46876` | waveRed | 5.09:1 |
| Flag | `#ff5d62` | peachRed | 5.44:1 |
| All icon | `#98bb6c` | springGreen | 7.52:1 |
| Logbook icon | `#957fb8` | oniViolet | 4.67:1 |
| Trash icon | `#717c7c` | katanaGray | 3.80:1 |

Notes: Dim text is `springViolet1`, not the comment grey `fujiGray`, which sits below 4.5:1; `fujiGray` is the checklist-done grey. Errors use `peachRed`, brighter than `samuraiRed`.

## 7. Kanagawa Dragon

- Key: `kanagawa-dragon` · dark
- Palette source: [`rebelot/kanagawa.nvim` · `lua/kanagawa/colors.lua`](https://github.com/rebelot/kanagawa.nvim/blob/master/lua/kanagawa/colors.lua), roles as `lua/kanagawa/themes.lua` maps them for `dragon`

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#181616` | dragonBlack3 | — |
| Text | `#c5c9c5` | dragonWhite | 10.76:1 |
| Dim text | `#a6a69c` | dragonGray | 7.34:1 |
| Primary | `#8ba4b0` | dragonBlue2 | 6.90:1 |
| Accent | `#b6927b` | dragonOrange | 6.34:1 |
| Selection | `#223249` | waveBlue1 | text on it 7.74:1 |
| Focus marker | `#a292a3` | dragonPink | 6.16:1 |
| Current-line band | `#393836` | dragonBlack5 | text on it 6.99:1 |
| Headers | `#949fb5` | dragonTeal | 6.77:1 |
| Sections | `#c4b28a` | dragonYellow | 8.65:1 |
| Rule lines | `#625e5a` | dragonBlack6 | 2.80:1 (non-text) |
| Links | `#8ea4a2` | dragonAqua | 6.85:1 |
| Start-date chip | `#87a987` | dragonGreen | 6.91:1 |
| Due-date chip | `#c4746e` | dragonRed | 5.21:1 |
| Overdue | `#e82424` | samuraiRed | 4.04:1 **(<4.5)** |
| Error | `#e82424` | samuraiRed | 4.04:1 **(<4.5)** |
| Tag text | `#8ea4a2` | dragonAqua | 6.85:1 |
| Checklist done | `#737c73` | dragonAsh | 4.17:1 **(<4.5)** |
| Tag 1 | `#8ba4b0` | dragonBlue2 | 6.90:1 |
| Tag 2 | `#87a987` | dragonGreen | 6.91:1 |
| Tag 3 | `#c4b28a` | dragonYellow | 8.65:1 |
| Tag 4 | `#b6927b` | dragonOrange | 6.34:1 |
| Tag 5 | `#a292a3` | dragonPink | 6.16:1 |
| Tag 6 | `#8992a7` | dragonViolet | 5.78:1 |
| Inbox icon | `#8ba4b0` | dragonBlue2 | 6.90:1 |
| Today icon | `#c4b28a` | dragonYellow | 8.65:1 |
| Upcoming icon | `#c4746e` | dragonRed | 5.21:1 |
| Flag | `#e82424` | samuraiRed | 4.04:1 |
| All icon | `#87a987` | dragonGreen | 6.91:1 |
| Logbook icon | `#8992a7` | dragonViolet | 5.78:1 |
| Trash icon | `#7a8382` | dragonGray3 | 4.63:1 |

Notes: Wave's darker sibling has a palette of its own: warm ink black and muted, earthy accents. The flag takes `samuraiRed`, the theme's error red.

## 8. Gruvbox Dark

- Key: `gruvbox-dark` · dark
- Palette source: [`ellisonleao/gruvbox.nvim` · `lua/gruvbox.lua` → `Gruvbox.palette`](https://github.com/ellisonleao/gruvbox.nvim/blob/main/lua/gruvbox.lua), the dark contrast mapping

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#282828` | dark0 | — |
| Text | `#ebdbb2` | light1 | 10.75:1 |
| Dim text | `#a89984` | light4 | 5.30:1 |
| Primary | `#83a598` | bright_blue | 5.48:1 |
| Accent | `#fe8019` | bright_orange | 5.84:1 |
| Selection | `#504945` | dark2 | text on it 6.43:1 |
| Focus marker | `#d3869b` | bright_purple | 5.37:1 |
| Current-line band | `#3c3836` | dark1 | text on it 8.45:1 |
| Headers | `#8ec07c` | bright_aqua | 7.01:1 |
| Sections | `#fabd2f` | bright_yellow | 8.69:1 |
| Rule lines | `#665c54` | dark3 | 2.26:1 (non-text) |
| Links | `#83a598` | bright_blue | 5.48:1 |
| Start-date chip | `#b8bb26` | bright_green | 7.14:1 |
| Due-date chip | `#fb4934` | bright_red | 4.29:1 **(<4.5)** |
| Overdue | `#fb4934` | bright_red | 4.29:1 **(<4.5)** |
| Error | `#fb4934` | bright_red | 4.29:1 **(<4.5)** |
| Tag text | `#8ec07c` | bright_aqua | 7.01:1 |
| Checklist done | `#928374` | gray | 4.02:1 **(<4.5)** |
| Tag 1 | `#83a598` | bright_blue | 5.48:1 |
| Tag 2 | `#b8bb26` | bright_green | 7.14:1 |
| Tag 3 | `#fabd2f` | bright_yellow | 8.69:1 |
| Tag 4 | `#fe8019` | bright_orange | 5.84:1 |
| Tag 5 | `#fb4934` | bright_red | 4.29:1 **(<4.5)** |
| Tag 6 | `#d3869b` | bright_purple | 5.37:1 |
| Inbox icon | `#83a598` | bright_blue | 5.48:1 |
| Today icon | `#fabd2f` | bright_yellow | 8.69:1 |
| Upcoming icon | `#fb4934` | bright_red | 4.29:1 |
| Flag | `#fe8019` | bright_orange | 5.84:1 |
| All icon | `#b8bb26` | bright_green | 7.14:1 |
| Logbook icon | `#d3869b` | bright_purple | 5.37:1 |
| Trash icon | `#928374` | gray | 4.02:1 |

Notes: The classic retro palette, with the `bright_*` accents Gruvbox gives its dark mode. Its one red serves the due chip, overdue (drawn bold and reversed) and the Upcoming icon; the flag takes `bright_orange`, since `neutral_red` is too dark on `dark0`.

## 9. Everforest Dark

- Key: `everforest-dark` · dark
- Palette source: [`sainnhe/everforest` · `autoload/everforest.vim`](https://github.com/sainnhe/everforest/blob/master/autoload/everforest.vim), dark, `medium` background

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#2d353b` | bg0 | — |
| Text | `#d3c6aa` | fg | 7.38:1 |
| Dim text | `#9da9a0` | grey2 | 5.12:1 |
| Primary | `#a7c080` | green | 6.23:1 |
| Accent | `#e69875` | orange | 5.41:1 |
| Selection | `#543a48` | bg_visual | text on it 5.99:1 |
| Focus marker | `#d699b6` | purple | 5.40:1 |
| Current-line band | `#343f44` | bg1 | text on it 6.40:1 |
| Headers | `#7fbbb3` | blue | 5.74:1 |
| Sections | `#dbbc7f` | yellow | 6.84:1 |
| Rule lines | `#56635f` | bg5 | 1.99:1 (non-text) |
| Links | `#7fbbb3` | blue | 5.74:1 |
| Start-date chip | `#a7c080` | green | 6.23:1 |
| Due-date chip | `#e67e80` | red | 4.55:1 |
| Overdue | `#e67e80` | red | 4.55:1 |
| Error | `#e67e80` | red | 4.55:1 |
| Tag text | `#83c092` | aqua | 5.89:1 |
| Checklist done | `#7a8478` | grey0 | 3.21:1 **(<4.5)** |
| Tag 1 | `#7fbbb3` | blue | 5.74:1 |
| Tag 2 | `#a7c080` | green | 6.23:1 |
| Tag 3 | `#dbbc7f` | yellow | 6.84:1 |
| Tag 4 | `#e69875` | orange | 5.41:1 |
| Tag 5 | `#e67e80` | red | 4.55:1 |
| Tag 6 | `#d699b6` | purple | 5.40:1 |
| Inbox icon | `#7fbbb3` | blue | 5.74:1 |
| Today icon | `#dbbc7f` | yellow | 6.84:1 |
| Upcoming icon | `#e67e80` | red | 4.55:1 |
| Flag | `#e69875` | orange | 5.41:1 |
| All icon | `#a7c080` | green | 6.23:1 |
| Logbook icon | `#d699b6` | purple | 5.40:1 |
| Trash icon | `#859289` | grey1 | 3.84:1 |

Notes: Projects take Everforest's signature green. With one red, overdue shares it with the due chip, drawn bold and reversed; the flag takes `orange`.

## 10. GitHub Dark Default

- Key: `github-dark` · dark
- Palette source: [`@primer/primitives`](https://github.com/primer/primitives) dark scale, as vendored in [`projekt0n/github-nvim-theme` · `lua/github-theme/palette/primitives/dark.lua`](https://github.com/projekt0n/github-nvim-theme/blob/main/lua/github-theme/palette/primitives/dark.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#0d1117` | canvas.default | — |
| Text | `#e6edf3` | fg.default | 16.02:1 |
| Dim text | `#848d97` | fg.muted | 5.62:1 |
| Primary | `#58a6ff` | blue.3 | 7.49:1 |
| Accent | `#a371f7` | done.fg | 5.64:1 |
| Selection | `#0c2d6b` | blue.8 | text on it 11.08:1 |
| Focus marker | `#db6d28` | severe.fg | 5.61:1 |
| Current-line band | `#161b22` | canvas.subtle | text on it 14.64:1 |
| Headers | `#79c0ff` | blue.2 | 9.73:1 |
| Sections | `#d29922` | attention.fg | 7.50:1 |
| Rule lines | `#30363d` | border.default | 1.55:1 (non-text) |
| Links | `#2f81f7` | accent.fg | 5.05:1 |
| Start-date chip | `#3fb950` | success.fg | 7.45:1 |
| Due-date chip | `#ff7b72` | red.3 | 7.51:1 |
| Overdue | `#f85149` | danger.fg | 5.65:1 |
| Error | `#f85149` | danger.fg | 5.65:1 |
| Tag text | `#ffa657` | orange.2 | 9.77:1 |
| Checklist done | `#6e7681` | fg.subtle | 4.12:1 **(<4.5)** |
| Tag 1 | `#58a6ff` | blue.3 | 7.49:1 |
| Tag 2 | `#3fb950` | green.3 | 7.45:1 |
| Tag 3 | `#d29922` | yellow.3 | 7.50:1 |
| Tag 4 | `#db6d28` | orange.4 | 5.61:1 |
| Tag 5 | `#db61a2` | pink.4 | 5.63:1 |
| Tag 6 | `#a371f7` | purple.4 | 5.64:1 |
| Inbox icon | `#58a6ff` | blue.3 | 7.49:1 |
| Today icon | `#d29922` | yellow.3 | 7.50:1 |
| Upcoming icon | `#ff7b72` | red.3 | 7.51:1 |
| Flag | `#f85149` | red.4 | 5.65:1 |
| All icon | `#3fb950` | green.3 | 7.45:1 |
| Logbook icon | `#a371f7` | purple.4 | 5.64:1 |
| Trash icon | `#6e7681` | fg.subtle | 4.12:1 |

Notes: The dark partner of GitHub Light Default, with the same role choices one step lighter on Primer's scales. The selection is `blue.8`, the dark counterpart of the light theme's `accent.subtle`.

## 11. Flexoki Dark

- Key: `flexoki-dark` · dark
- Palette source: [`kepano/flexoki` · `README.md` → Colors / Extended palette](https://github.com/kepano/flexoki#colors). Role names follow Flexoki's own dark-theme mapping (`bg`, `bg-2`, `ui`, `ui-2`, `ui-3`, `tx`, `tx-2`).

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#100f0f` | black (bg) | — |
| Text | `#cecdc3` | base-200 (tx) | 11.98:1 |
| Dim text | `#878580` | base-500 (tx-2) | 5.19:1 |
| Primary | `#4385be` | blue-400 | 4.86:1 |
| Accent | `#da702c` | orange-400 | 5.77:1 |
| Selection | `#343331` | base-850 (ui-2) | text on it 7.90:1 |
| Focus marker | `#ce5d97` | magenta-400 | 5.14:1 |
| Current-line band | `#1c1b1a` | base-950 (bg-2) | text on it 10.77:1 |
| Headers | `#8b7ec8` | purple-400 | 5.39:1 |
| Sections | `#d0a215` | yellow-400 | 8.08:1 |
| Rule lines | `#403e3c` | base-800 (ui-3) | 1.80:1 (non-text) |
| Links | `#3aa99f` | cyan-400 | 6.70:1 |
| Start-date chip | `#879a39` | green-400 | 6.12:1 |
| Due-date chip | `#d14d41` | red-400 | 4.42:1 **(<4.5)** |
| Overdue | `#e8705f` | red-300 | 6.30:1 |
| Error | `#d14d41` | red-400 | 4.42:1 **(<4.5)** |
| Tag text | `#3aa99f` | cyan-400 | 6.70:1 |
| Checklist done | `#6f6e69` | base-600 | 3.75:1 **(<4.5)** |
| Tag 1 | `#4385be` | blue-400 | 4.86:1 |
| Tag 2 | `#879a39` | green-400 | 6.12:1 |
| Tag 3 | `#d0a215` | yellow-400 | 8.08:1 |
| Tag 4 | `#da702c` | orange-400 | 5.77:1 |
| Tag 5 | `#ce5d97` | magenta-400 | 5.14:1 |
| Tag 6 | `#8b7ec8` | purple-400 | 5.39:1 |
| Inbox icon | `#4385be` | blue-400 | 4.86:1 |
| Today icon | `#d0a215` | yellow-400 | 8.08:1 |
| Upcoming icon | `#d14d41` | red-400 | 4.42:1 |
| Flag | `#e8705f` | red-300 | 6.30:1 |
| All icon | `#879a39` | green-400 | 6.12:1 |
| Logbook icon | `#8b7ec8` | purple-400 | 5.39:1 |
| Trash icon | `#6f6e69` | base-600 | 3.75:1 |

Notes: The dark partner of Flexoki Light, with the 400 accents Flexoki gives its dark theme.

## 12. Melange Dark

- Key: `melange-dark` · dark
- Palette source: [`savq/melange-nvim` · `lua/melange/palettes/dark.lua`](https://github.com/savq/melange-nvim/blob/master/lua/melange/palettes/dark.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#292522` | a.bg | — |
| Text | `#ece1d7` | a.fg | 11.81:1 |
| Dim text | `#c1a78e` | a.com | 6.65:1 |
| Primary | `#a3a9ce` | b.blue | 6.60:1 |
| Accent | `#e49b5d` | c.yellow | 6.62:1 |
| Selection | `#403a36` | a.sel | text on it 8.70:1 |
| Focus marker | `#cf9bc2` | b.magenta | 6.60:1 |
| Current-line band | `#34302c` | a.float | text on it 10.17:1 |
| Headers | `#89b3b6` | b.cyan | 6.65:1 |
| Sections | `#ebc06d` | b.yellow | 8.90:1 |
| Rule lines | `#867462` | a.ui | 3.39:1 (non-text) |
| Links | `#89b3b6` | b.cyan | 6.65:1 |
| Start-date chip | `#85b695` | b.green | 6.60:1 |
| Due-date chip | `#d47766` | b.red | 4.80:1 |
| Overdue | `#bd8183` | c.red | 4.80:1 |
| Error | `#d47766` | b.red | 4.80:1 |
| Tag text | `#89b3b6` | b.cyan | 6.65:1 |
| Checklist done | `#867462` | a.ui | 3.39:1 **(<4.5)** |
| Tag 1 | `#a3a9ce` | b.blue | 6.60:1 |
| Tag 2 | `#85b695` | b.green | 6.60:1 |
| Tag 3 | `#ebc06d` | b.yellow | 8.90:1 |
| Tag 4 | `#e49b5d` | c.yellow | 6.62:1 |
| Tag 5 | `#d47766` | b.red | 4.80:1 |
| Tag 6 | `#cf9bc2` | b.magenta | 6.60:1 |
| Inbox icon | `#a3a9ce` | b.blue | 6.60:1 |
| Today icon | `#ebc06d` | b.yellow | 8.90:1 |
| Upcoming icon | `#d47766` | b.red | 4.80:1 |
| Flag | `#bd8183` | c.red | 4.80:1 |
| All icon | `#85b695` | b.green | 6.60:1 |
| Logbook icon | `#cf9bc2` | b.magenta | 6.60:1 |
| Trash icon | `#867462` | a.ui | 3.39:1 |

Notes: A warm, low-glare theme. Melange names its colors by group: `a` for the background and text greys, `b` for the bright accents and `c` for the softer ones.

## 13. Moonfly

- Key: `moonfly` · dark
- Palette source: [`bluz71/vim-moonfly-colors` · `lua/moonfly/init.lua`](https://github.com/bluz71/vim-moonfly-colors/blob/master/lua/moonfly/init.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#080808` | black | — |
| Text | `#c6c6c6` | white | 11.72:1 |
| Dim text | `#949494` | grey58 | 6.60:1 |
| Primary | `#80a0ff` | blue | 8.00:1 |
| Accent | `#de935f` | orange | 8.05:1 |
| Selection | `#323437` | grey0 | text on it 7.31:1 |
| Focus marker | `#ae81ff` | purple | 7.04:1 |
| Current-line band | `#1c1c1c` | grey11 | text on it 9.98:1 |
| Headers | `#74b2ff` | sky | 9.11:1 |
| Sections | `#e3c78a` | yellow | 12.21:1 |
| Rule lines | `#444444` | grey27 | 2.06:1 (non-text) |
| Links | `#79dac8` | turquoise | 12.09:1 |
| Start-date chip | `#8cc85f` | green | 10.05:1 |
| Due-date chip | `#ff5d5d` | red | 6.65:1 |
| Overdue | `#ff5189` | crimson | 6.48:1 |
| Error | `#ff5d5d` | red | 6.65:1 |
| Tag text | `#36c692` | emerald | 9.20:1 |
| Checklist done | `#626262` | grey39 | 3.28:1 **(<4.5)** |
| Tag 1 | `#80a0ff` | blue | 8.00:1 |
| Tag 2 | `#8cc85f` | green | 10.05:1 |
| Tag 3 | `#e3c78a` | yellow | 12.21:1 |
| Tag 4 | `#de935f` | orange | 8.05:1 |
| Tag 5 | `#ff5189` | crimson | 6.48:1 |
| Tag 6 | `#ae81ff` | purple | 7.04:1 |
| Inbox icon | `#80a0ff` | blue | 8.00:1 |
| Today icon | `#e3c78a` | yellow | 12.21:1 |
| Upcoming icon | `#ff5d5d` | red | 6.65:1 |
| Flag | `#ff5189` | crimson | 6.48:1 |
| All icon | `#8cc85f` | green | 10.05:1 |
| Logbook icon | `#ae81ff` | purple | 7.04:1 |
| Trash icon | `#808080` | grey50 | 5.07:1 |

Notes: A dark charcoal theme on near-black. The roles follow its own highlight groups: `CursorLine` is `grey11`, `Visual` is `grey0`, `Comment` is `grey58`.

## 14. Nightfly

- Key: `nightfly` · dark
- Palette source: [`bluz71/vim-nightfly-colors` · `lua/nightfly/init.lua`](https://github.com/bluz71/vim-nightfly-colors/blob/master/lua/nightfly/init.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#011627` | black | — |
| Text | `#c3ccdc` | white | 11.34:1 |
| Dim text | `#a1aab8` | cadet_blue | 7.82:1 |
| Primary | `#82aaff` | blue | 7.98:1 |
| Accent | `#f78c6c` | orange | 7.79:1 |
| Selection | `#1d3b53` | regal_blue | text on it 7.21:1 |
| Focus marker | `#c792ea` | violet | 7.62:1 |
| Current-line band | `#092236` | dark_blue | text on it 10.04:1 |
| Headers | `#87bcff` | malibu | 9.31:1 |
| Sections | `#e3d18a` | yellow | 12.01:1 |
| Rule lines | `#2c3043` | slate_blue | 1.41:1 (non-text) |
| Links | `#7fdbca` | turquoise | 11.25:1 |
| Start-date chip | `#a1cd5e` | green | 9.96:1 |
| Due-date chip | `#fc514e` | red | 5.61:1 |
| Overdue | `#ff5874` | watermelon | 6.02:1 |
| Error | `#fc514e` | red | 5.61:1 |
| Tag text | `#21c7a8` | emerald | 8.56:1 |
| Checklist done | `#7c8f8f` | grey_blue | 5.40:1 |
| Tag 1 | `#82aaff` | blue | 7.98:1 |
| Tag 2 | `#a1cd5e` | green | 9.96:1 |
| Tag 3 | `#e3d18a` | yellow | 12.01:1 |
| Tag 4 | `#f78c6c` | orange | 7.79:1 |
| Tag 5 | `#ff5874` | watermelon | 6.02:1 |
| Tag 6 | `#ae81ff` | purple | 6.45:1 |
| Inbox icon | `#82aaff` | blue | 7.98:1 |
| Today icon | `#e3d18a` | yellow | 12.01:1 |
| Upcoming icon | `#fc514e` | red | 5.61:1 |
| Flag | `#ff5874` | watermelon | 6.02:1 |
| All icon | `#a1cd5e` | green | 9.96:1 |
| Logbook icon | `#ae81ff` | purple | 6.45:1 |
| Trash icon | `#768799` | graphite_blue | 4.97:1 |

Notes: A deep midnight-blue theme in the spirit of Night Owl. The roles follow its own highlight groups: `CursorLine` is `dark_blue`, `Visual` is `regal_blue`.

## 15. Cyberdream

- Key: `cyberdream` · dark
- Palette source: [`scottmckendry/cyberdream.nvim` · `lua/cyberdream/colors.lua` → `default`](https://github.com/scottmckendry/cyberdream.nvim/blob/main/lua/cyberdream/colors.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#16181a` | bg | — |
| Text | `#ffffff` | fg | 17.80:1 |
| Dim text | `#7b8496` | grey | 4.73:1 |
| Primary | `#5ea1ff` | blue | 6.80:1 |
| Accent | `#ffbd5e` | orange | 10.76:1 |
| Selection | `#3c4048` | bg_highlight | text on it 10.40:1 |
| Focus marker | `#ff5ef1` | magenta | 6.89:1 |
| Current-line band | `#1e2124` | bg_alt | text on it 16.18:1 |
| Headers | `#5ef1ff` | cyan | 13.14:1 |
| Sections | `#f1ff5e` | yellow | 16.28:1 |
| Rule lines | `#3c4048` | bg_highlight | 1.71:1 (non-text) |
| Links | `#5ef1ff` | cyan | 13.14:1 |
| Start-date chip | `#5eff6c` | green | 13.56:1 |
| Due-date chip | `#ff6e5e` | red | 6.48:1 |
| Overdue | `#ff5ea0` | pink | 6.24:1 |
| Error | `#ff6e5e` | red | 6.48:1 |
| Tag text | `#5ef1ff` | cyan | 13.14:1 |
| Checklist done | `#7b8496` | grey | 4.73:1 |
| Tag 1 | `#5ea1ff` | blue | 6.80:1 |
| Tag 2 | `#5eff6c` | green | 13.56:1 |
| Tag 3 | `#f1ff5e` | yellow | 16.28:1 |
| Tag 4 | `#ffbd5e` | orange | 10.76:1 |
| Tag 5 | `#ff5ea0` | pink | 6.24:1 |
| Tag 6 | `#bd5eff` | purple | 5.26:1 |
| Inbox icon | `#5ea1ff` | blue | 6.80:1 |
| Today icon | `#f1ff5e` | yellow | 16.28:1 |
| Upcoming icon | `#ff6e5e` | red | 6.48:1 |
| Flag | `#ff5ea0` | pink | 6.24:1 |
| All icon | `#5eff6c` | green | 13.56:1 |
| Logbook icon | `#bd5eff` | purple | 5.26:1 |
| Trash icon | `#7b8496` | grey | 4.73:1 |

Notes: A neon theme on near-black: every accent but `purple` is above 7:1. Its one grey serves dim text, checklist done and the Trash icon.

## 16. Synthwave '84

- Key: `synthwave-84` · dark
- Palette source: [`robb0wen/synthwave-vscode` · `themes/synthwave-color-theme.json`](https://github.com/robb0wen/synthwave-vscode/blob/master/themes/synthwave-color-theme.json); the palette names are its VS Code color keys and token scopes

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#262335` | editor.background | — |
| Text | `#ffffff` | foreground | 15.28:1 |
| Dim text | `#848bbd` | Comment | 4.66:1 |
| Primary | `#36f9f6` | Function | 11.67:1 |
| Accent | `#f97e72` | Constant | 6.01:1 |
| Selection | `#463465` | menu.background | text on it 10.83:1 |
| Focus marker | `#ff7edb` | Variable | 6.74:1 |
| Current-line band | `#34294f` | statusBarItem.prominentHoverBackground | text on it 13.32:1 |
| Headers | `#ff7edb` | Markup heading | 6.74:1 |
| Sections | `#fede5d` | Keyword | 11.51:1 |
| Rule lines | `#495495` | editorGroup.border | 2.17:1 (non-text) |
| Links | `#2ee2fa` | JS object path | 9.73:1 |
| Start-date chip | `#72f1b8` | HTML or XML tag | 10.91:1 |
| Due-date chip | `#fe4450` | editorError.foreground | 4.48:1 **(<4.5)** |
| Overdue | `#fe4450` | editorError.foreground | 4.48:1 **(<4.5)** |
| Error | `#fe4450` | editorError.foreground | 4.48:1 **(<4.5)** |
| Tag text | `#72f1b8` | HTML or XML tag | 10.91:1 |
| Checklist done | `#848bbd` | Comment | 4.66:1 |
| Tag 1 | `#36f9f6` | Function | 11.67:1 |
| Tag 2 | `#72f1b8` | HTML or XML tag | 10.91:1 |
| Tag 3 | `#fede5d` | Keyword | 11.51:1 |
| Tag 4 | `#f97e72` | Constant | 6.01:1 |
| Tag 5 | `#ff7edb` | Variable | 6.74:1 |
| Tag 6 | `#b893ce` | gitDecoration.modifiedResourceForeground | 5.90:1 |
| Inbox icon | `#36f9f6` | Function | 11.67:1 |
| Today icon | `#fede5d` | Keyword | 11.51:1 |
| Upcoming icon | `#fe4450` | editorError.foreground | 4.48:1 |
| Flag | `#f97e72` | Constant | 6.01:1 |
| All icon | `#72f1b8` | HTML or XML tag | 10.91:1 |
| Logbook icon | `#b893ce` | gitDecoration.modifiedResourceForeground | 5.90:1 |
| Trash icon | `#848bbd` | Comment | 4.66:1 |

Notes: The retro neon theme of outrun synths. It has no blue: the Inbox takes its electric cyan. Its comment grey clears 4.5:1, so it serves dim text, checklist done and the Trash icon. The theme's optional glow is an editor effect a terminal cannot draw.

## 17. Gruvbox Light

- Key: `gruvbox-light` · light
- Palette source: [`ellisonleao/gruvbox.nvim` · `lua/gruvbox.lua` → `Gruvbox.palette`](https://github.com/ellisonleao/gruvbox.nvim/blob/main/lua/gruvbox.lua), the light contrast mapping

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#fbf1c7` | light0 | — |
| Text | `#3c3836` | dark1 | 10.22:1 |
| Dim text | `#665c54` | dark3 | 5.74:1 |
| Primary | `#076678` | faded_blue | 5.82:1 |
| Accent | `#af3a03` | faded_orange | 5.40:1 |
| Selection | `#d5c4a1` | light2 | text on it 6.76:1 |
| Focus marker | `#8f3f71` | faded_purple | 5.94:1 |
| Current-line band | `#ebdbb2` | light1 | text on it 8.45:1 |
| Headers | `#427b58` | faded_aqua | 4.40:1 **(<4.5)** |
| Sections | `#b57614` | faded_yellow | 3.33:1 **(<4.5)** |
| Rule lines | `#bdae93` | light3 | 1.92:1 (non-text) |
| Links | `#076678` | faded_blue | 5.82:1 |
| Start-date chip | `#79740e` | faded_green | 4.29:1 **(<4.5)** |
| Due-date chip | `#9d0006` | faded_red | 7.60:1 |
| Overdue | `#9d0006` | faded_red | 7.60:1 |
| Error | `#9d0006` | faded_red | 7.60:1 |
| Tag text | `#427b58` | faded_aqua | 4.40:1 **(<4.5)** |
| Checklist done | `#928374` | gray | 3.24:1 **(<4.5)** |
| Tag 1 | `#076678` | faded_blue | 5.82:1 |
| Tag 2 | `#79740e` | faded_green | 4.29:1 **(<4.5)** |
| Tag 3 | `#b57614` | faded_yellow | 3.33:1 **(<4.5)** |
| Tag 4 | `#af3a03` | faded_orange | 5.40:1 |
| Tag 5 | `#9d0006` | faded_red | 7.60:1 |
| Tag 6 | `#8f3f71` | faded_purple | 5.94:1 |
| Inbox icon | `#076678` | faded_blue | 5.82:1 |
| Today icon | `#b57614` | faded_yellow | 3.33:1 |
| Upcoming icon | `#9d0006` | faded_red | 7.60:1 |
| Flag | `#cc241d` | neutral_red | 4.82:1 |
| All icon | `#79740e` | faded_green | 4.29:1 |
| Logbook icon | `#8f3f71` | faded_purple | 5.94:1 |
| Trash icon | `#928374` | gray | 3.24:1 |

Notes: The light partner of Gruvbox Dark, with the `faded_*` accents Gruvbox gives its light mode. The flag takes `neutral_red`, brighter than the Upcoming icon's `faded_red`.

## 18. Melange Light

- Key: `melange-light` · light
- Palette source: [`savq/melange-nvim` · `lua/melange/palettes/light.lua`](https://github.com/savq/melange-nvim/blob/master/lua/melange/palettes/light.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#f1f1f1` | a.bg | — |
| Text | `#54433a` | a.fg | 8.30:1 |
| Dim text | `#7d6658` | a.com | 4.75:1 |
| Primary | `#465aa4` | b.blue | 5.71:1 |
| Accent | `#bc5c00` | c.yellow | 3.98:1 **(<4.5)** |
| Selection | `#d9d3ce` | a.sel | text on it 6.32:1 |
| Focus marker | `#904180` | b.magenta | 5.71:1 |
| Current-line band | `#e9e1db` | a.float | text on it 7.25:1 |
| Headers | `#3d6568` | b.cyan | 5.70:1 |
| Sections | `#a06d00` | b.yellow | 3.97:1 **(<4.5)** |
| Rule lines | `#a98a78` | a.ui | 2.82:1 (non-text) |
| Links | `#3d6568` | b.cyan | 5.70:1 |
| Start-date chip | `#3a684a` | b.green | 5.71:1 |
| Due-date chip | `#bf0021` | b.red | 5.74:1 |
| Overdue | `#bf0021` | b.red | 5.74:1 |
| Error | `#bf0021` | b.red | 5.74:1 |
| Tag text | `#3d6568` | b.cyan | 5.70:1 |
| Checklist done | `#7d6658` | a.com | 4.75:1 |
| Tag 1 | `#465aa4` | b.blue | 5.71:1 |
| Tag 2 | `#3a684a` | b.green | 5.71:1 |
| Tag 3 | `#a06d00` | b.yellow | 3.97:1 **(<4.5)** |
| Tag 4 | `#bc5c00` | c.yellow | 3.98:1 **(<4.5)** |
| Tag 5 | `#bf0021` | b.red | 5.74:1 |
| Tag 6 | `#904180` | b.magenta | 5.71:1 |
| Inbox icon | `#465aa4` | b.blue | 5.71:1 |
| Today icon | `#a06d00` | b.yellow | 3.97:1 |
| Upcoming icon | `#bf0021` | b.red | 5.74:1 |
| Flag | `#bc5c00` | c.yellow | 3.98:1 |
| All icon | `#3a684a` | b.green | 5.71:1 |
| Logbook icon | `#904180` | b.magenta | 5.71:1 |
| Trash icon | `#7d6658` | a.com | 4.75:1 |

Notes: The light partner of Melange Dark, warm brown ink on light grey. `a.ui` and `c.red` are under 3:1 on `a.bg`, so checklist done and the Trash icon take `a.com`, the dim-text grey, and the flag takes `c.yellow`, an orange.

## 19. Dayfox

- Key: `dayfox` · light
- Palette source: [`EdenEast/nightfox.nvim` · `lua/nightfox/palette/dayfox.lua`](https://github.com/EdenEast/nightfox.nvim/blob/main/lua/nightfox/palette/dayfox.lua)

| Role | Hex | Palette name | Contrast |
|---|---|---|---|
| Background | `#f6f2ee` | bg1 | — |
| Text | `#3d2b5a` | fg1 | 11.15:1 |
| Dim text | `#643f61` | fg2 | 7.77:1 |
| Primary | `#2848a9` | blue | 7.28:1 |
| Accent | `#955f61` | orange | 4.61:1 |
| Selection | `#e7d2be` | sel0 | text on it 8.49:1 |
| Focus marker | `#a440b5` | pink | 4.74:1 |
| Current-line band | `#e4dcd4` | bg0 | text on it 9.16:1 |
| Headers | `#6e33ce` | magenta | 6.23:1 |
| Sections | `#ac5402` | yellow | 4.69:1 |
| Rule lines | `#aab0ad` | bg4 | 1.98:1 (non-text) |
| Links | `#287980` | cyan | 4.56:1 |
| Start-date chip | `#396847` | green | 5.81:1 |
| Due-date chip | `#a5222f` | red | 6.57:1 |
| Overdue | `#a5222f` | red | 6.57:1 |
| Error | `#a5222f` | red | 6.57:1 |
| Tag text | `#287980` | cyan | 4.56:1 |
| Checklist done | `#837a72` | comment | 3.78:1 **(<4.5)** |
| Tag 1 | `#2848a9` | blue | 7.28:1 |
| Tag 2 | `#396847` | green | 5.81:1 |
| Tag 3 | `#ac5402` | yellow | 4.69:1 |
| Tag 4 | `#955f61` | orange | 4.61:1 |
| Tag 5 | `#a440b5` | pink | 4.74:1 |
| Tag 6 | `#6e33ce` | magenta | 6.23:1 |
| Inbox icon | `#2848a9` | blue | 7.28:1 |
| Today icon | `#ac5402` | yellow | 4.69:1 |
| Upcoming icon | `#a5222f` | red | 6.57:1 |
| Flag | `#955f61` | orange | 4.61:1 |
| All icon | `#396847` | green | 5.81:1 |
| Logbook icon | `#6e33ce` | magenta | 6.23:1 |
| Trash icon | `#837a72` | comment | 3.78:1 |

Notes: The light member of the Nightfox family. Its `yellow` is a deep amber, so the Today icon and sections clear 4.5:1.

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

## Grayscale

- Setting: `grayscale = true`, off by default (FR-UI-11, D-122)

Grayscale draws any theme above, Terminal Dark included, without a hue, so that text and icons all read in the default text color. The theme stays dark or light. Each role takes a gray:

| Roles | Gray |
|---|---|
| Background, dim text, selection, current-line band, text on the band, rule lines, checklist done, Trash icon | the gray of the same WCAG luminance as the theme's own, so every contrast above holds within rounding; Terminal Dark's own text and greys stay as they are |
| Text | the same, from the theme's text |
| Every other role: primary, accent, focus marker, headers, sections, links, date chips, overdue, error, tags, the view icons and the flag | the text's gray |

Weight, italics, underlines and dimming still set roles apart: area titles stay bold capitals, overdue chips bold and reversed, links underlined. The weather draws in 256-color greys of about each color's lightness.

## Contrast summary

- Text and dim text clear 4.5:1 in every theme. The lowest is dim text in Synthwave '84 at 4.66:1, its comment grey.
- The roles below 4.5:1 are deliberate or marginal:
  - Checklist done is faded on purpose: Mocha 4.44, Tokyo Night 4.10, Flexoki 3.59.
  - Flexoki `cyan-600` (links and tag text, 4.43) and `green-600` (start chip and tag 2, 4.39) are marginal. Swap in `cyan-700` (`#1c6c66`) and `green-700` (`#536907`) from the extended palette if strict AA is required.
- GitHub Light Default passes every role; Nightfly and Cyberdream pass every role too.
- Of the fifteen added themes, the roles below 4.5:1 other than checklist done are chips, tags and icons in the theme's own hue. The light themes are where this shows most: Gruvbox Light's `faded_yellow` (3.33) and Melange Light's `b.yellow` (3.97) are the darkest yellows their palettes have. Every icon still clears 3:1, as the tests require.

## Later

These candidates were evaluated for a future release. Their licenses are verified permissive. Palettes are intentionally not included yet.

- **Catppuccin Latte** (MIT). The natural light partner for Mocha, but its yellow (`#df8e1d`, 2.3:1) and peach (2.9:1) are under 3:1 on `base`, so the Today icon would fail the test. It needs a darker yellow the palette does not have.
- **Rosé Pine Dawn** (MIT). Its `subtle` grey is 4.2:1 and `gold` 2.2:1 on `base`, failing dim text and the Today icon.
- **Tokyo Night Moon / Storm, Catppuccin Macchiato / Frappé, Rosé Pine Moon.** Sibling variants that share their accents with a chosen theme.

## Rejected candidates

- **Modus Operandi / Vivendi.** Best-in-class contrast, but GPL-3.0, so not permissive.
- **Material (material.nvim).** GPL-2.0.
- **Monokai Pro, Dracula PRO.** Proprietary licenses.
- **Vesper.** No license file could be verified in a public repository.
- **Dracula** (MIT). Iconic, but its only grey (`Comment`, `#6272a4`) is 3.03:1, so no palette color can serve dim text. **Alucard**, its light variant, has a single light surface, so there is nothing for the current-line band apart from the selection.
- **Nord** (MIT). Its greys `nord1`-`nord3` are under 2:1 on `nord0`, so checklist done and the Trash icon have no color. **Nordic** (MIT) has the same gap: its `gray5` is 3.3:1.
- **One Dark / One Light** (navarasu/onedark.nvim, MIT). `light_grey` is about 4:1 on One Dark and 3.6:1 on One Light, so neither has a dim text color; One Light's `grey` is 2.5:1.
- **Sonokai, Edge Light, Bamboo, Cyberdream Light, Everforest Light, Kanagawa Lotus** (all MIT). Each palette's dim grey is under 4.5:1 on its background (Kanagawa Lotus 4.26, Sonokai 3.9, Bamboo Light 3.6, Cyberdream Light 3.8, Everforest Light 3.3, Edge Light 3.1).
- **Gruvbox Material Light** (MIT). Its `grey2` is 4.29:1 on `bg0`. Gruvbox Material Dark passes, but original Gruvbox was taken for both modes instead, so the family is not split.
- **Poimandres** (MIT). It builds its greys from alpha blends, and its one opaque grey is too faint for dim text.
- **Carbonfox** (MIT). Its background shades are computed at runtime, and its yellow and orange slots are teal, which weakens tag separation. **Oxocarbon** (MIT) has the same IBM Carbon palette and no yellow at all.
- **Solarized Osaka** (Apache-2.0). Its palette is defined in HSL and converted at runtime, so there is no hex value to cite.
- **Solarized Light** (MIT). Body text `base00` is 4.13:1 and accents are 2.9-4.3:1 on `base3`. Flexoki Light covers the same warm-paper niche with far better contrast.
- **Tokyo Night Day.** There is no static palette: `day.lua` inverts the night palette at runtime, so there is no official hex to cite.
- **Ayu Light / Mirage** (MIT). The canonical palette is generated at build time (`src/generated` is not committed), and the older `ayu-vim` values differ, so there is no single source of truth.
- **VS Code Dark Modern (vscode.nvim).** No license file could be found in the repository.

## Sources

- Catppuccin palette and license: https://github.com/catppuccin/palette (`palette.json`, `LICENSE`)
- Tokyo Night: https://github.com/folke/tokyonight.nvim (`lua/tokyonight/colors/{storm,night,init,day}.lua`, `LICENSE`)
- GitHub/Primer: https://github.com/primer/github-vscode-theme (`LICENSE`), https://github.com/primer/primitives (`LICENSE`), https://github.com/projekt0n/github-nvim-theme (`lua/github-theme/palette/primitives/light.lua`)
- Flexoki: https://github.com/kepano/flexoki (`README.md` Colors and Extended palette, `LICENSE`), https://stephango.com/flexoki
- Neovim colorscheme usage ranking: https://dotfyle.com/neovim/colorscheme/top
- Ghostty theme catalog: https://ghostty.org/docs/features/theme and https://github.com/mbadolato/iTerm2-Color-Schemes (`ghostty/` contains each of the four v1 themes)
- Added themes (D-125): https://github.com/rose-pine/neovim (`lua/rose-pine/palette.lua`, `LICENSE`), https://github.com/rebelot/kanagawa.nvim (`lua/kanagawa/colors.lua`, `lua/kanagawa/themes.lua`, `LICENSE`), https://github.com/ellisonleao/gruvbox.nvim (`lua/gruvbox.lua`, `LICENSE`), https://github.com/sainnhe/everforest (`autoload/everforest.vim`, `LICENSE`), https://github.com/projekt0n/github-nvim-theme (`lua/github-theme/palette/primitives/dark.lua`), https://github.com/savq/melange-nvim (`lua/melange/palettes/{dark,light}.lua`, `LICENSE`), https://github.com/bluz71/vim-moonfly-colors and https://github.com/bluz71/vim-nightfly-colors (`lua/*/init.lua`, `LICENSE`), https://github.com/scottmckendry/cyberdream.nvim (`lua/cyberdream/colors.lua`, `LICENSE`), https://github.com/robb0wen/synthwave-vscode (`themes/synthwave-color-theme.json`, `LICENSE`), https://github.com/EdenEast/nightfox.nvim (`lua/nightfox/palette/dayfox.lua`, `LICENSE`)
- Neovim trending ranking: https://dotfyle.com/neovim/colorscheme/trending
- Later and rejected candidates: https://github.com/rose-pine/neovim, https://github.com/rebelot/kanagawa.nvim, https://github.com/sainnhe/gruvbox-material, https://github.com/sainnhe/everforest, https://github.com/EdenEast/nightfox.nvim, https://github.com/dracula/dracula-theme, https://draculatheme.com/spec, https://github.com/nordtheme/nord, https://github.com/altercation/solarized, https://github.com/ayu-theme/ayu-colors
- WCAG 2.2 contrast ratio definition: https://www.w3.org/TR/WCAG22/#dfn-contrast-ratio
