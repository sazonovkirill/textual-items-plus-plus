# Requirements

- Status: Draft
- Last updated: 2026-10-13

[functional-requirements.md](functional-requirements.md) defines what the app does and overrides this file. This file adds what that one leaves open: how data is handled safely, and the non-functional requirements. Undecided choices are in [open-questions.md](open-questions.md).

Priorities use the same scale as the functional requirements: **Must** (v1), **Should** (soon after), **Later**, **Won't**. IDs are stable; reference them from specs, tasks and commits. `F-<AREA>-<nn>` covers data handling (for example `F-FILE-01`) and `N-<AREA>-<nn>` the non-functional requirements (for example `N-PERF-01`).

## Principles

To be agreed with the owner's requirements. Two hold already, from [AGENTS.md](../AGENTS.md#2-correct-not-just-green):

1. **Never lose data.** Crashes, external edits and bad input must not corrupt or drop the user's data.
2. **Preserve what the app does not understand.** Anything that writes to disk keeps content it does not interpret.

## Data handling

| ID | Requirement | Priority |
|---|---|---|

## Non-functional requirements

| ID | Requirement | Priority |
|---|---|---|
