# Specs

- Status: Draft
- Last updated: 2026-10-13

Requirements, specifications and reference material for textual-items++. How agents build it lives in [docs/](../docs/README.md).

## Vision

textual-items++ is a keyboard-first todo app for the terminal, inspired by [Things 3](https://culturedcode.com/things/) and [Todoist](https://www.todoist.com), that keeps its data in one plain-text `.taskpaper` file and looks and feels like the owner's earlier app, taskpaper-terminal. The [product summary](functional-requirements.md#product-summary) says more.

## Contents

| Document | Status | What it holds |
|---|---|---|
| [functional-requirements.md](functional-requirements.md) | Draft | What the app does, as decided by the owner. Once accepted, overrides every other document here. |
| [requirements.md](requirements.md) | Draft | Non-functional requirements (performance, reliability, portability) and data-handling requirements the functional requirements leave open. |
| [implementation-plan.md](implementation-plan.md) | Draft | The order the app is built in: milestones of small steps, each leaving a working app. |
| [open-questions.md](open-questions.md) | Living | Undecided choices, each with a proposed default, plus the ones already decided. |
| [reference/](reference/README.md) | Reference | Research on Things 3, Todoist, TaskPaper, Workflowy and TUI best practices, and a snapshot of taskpaper-terminal's specs. |

Specs for each area (data model, storage, views, keybindings, UI and so on) are added here as `<area>.md` and listed in this table as they are written.

Draft documents become Accepted once the owner has reviewed them. Where a draft disagrees with an accepted document, the accepted document wins and the draft is a bug.
