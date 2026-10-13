# taskpaper-terminal snapshot

- Status: Reference
- Last updated: 2026-10-13

A copy of the requirements, decisions and specs of the owner's earlier app, [taskpaper-terminal](https://github.com/sazonovkirill/taskpaper-terminal), taken from its `docs/` on 2026-10-13. textual-items++ copies several of its setups on purpose ([functional-requirements.md](../../functional-requirements.md) names each), so agents can read them here without the other repository. Inside these files, "the app" and `tpt` mean taskpaper-terminal, and `FR-…`, `D-n` and `OQ-n` are taskpaper-terminal's IDs, not ours. Where textual-items++ copies a setup, our own requirement says so and wins over this copy; everything else here is background, not a requirement.

| File | What textual-items++ takes from it |
|---|---|
| [specs/attachments.md](specs/attachments.md) | The storage of notes and comments as hidden child nodes, and the full-height attachments panel on the right. |
| [specs/ui.md](specs/ui.md) | The look: layout, block styling, breadcrumbs, panes, themes, icons, settings window, dialogs, animation. |
| [specs/navigation.md](specs/navigation.md) | Panes, breadcrumbs and zoom history, Go to Symbol and Go to Command. |
| [specs/keybindings.md](specs/keybindings.md), [key-mnemonics.md](key-mnemonics.md) | The vi-style key scheme and its terminal constraints. |
| [specs/editing.md](specs/editing.md) | Navigate and edit modes, the inline editor, the date prompt and Quick Add. |
| [specs/file-format.md](specs/file-format.md), [specs/special-nodes.md](specs/special-nodes.md), [specs/file-io.md](specs/file-io.md) | Lossless TaskPaper parsing and writing, hidden data nodes, atomic saves, backups and external changes. |
| [specs/search.md](specs/search.md) | Natural-language date entry. |
| [reference/themes.md](reference/themes.md), [reference/tui-animation.md](reference/tui-animation.md) | The twelve color themes and the animation research. |
| [functional-requirements.md](functional-requirements.md), [requirements.md](requirements.md), [open-questions.md](open-questions.md) | The decisions behind the specs above. |

The TaskPaper format and Workflowy references it relies on are at [../taskpaper.md](../taskpaper.md) and [../workflowy.md](../workflowy.md).
