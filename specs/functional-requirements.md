# Functional Requirements

- Status: Draft
- Last updated: 2026-10-13

What textual-items++ does, as stated by the owner. The owner's answers to clarifying questions are recorded under [Decisions](#decisions) and referenced as **(D-n)**.

Priorities: **Must** is required for v1, **Should** is expected soon after, **Later** is planned after v1 and must not be precluded by the design, **Won't** is out of scope.

Requirement IDs have the form `FR-<AREA>-<nn>` (for example `FR-NAV-03`) and are stable: never renumber or reuse one; mark a dropped requirement **Won't** instead. Each area is a section with a table:

| ID | Requirement | Priority |
|---|---|---|

`bash app/scripts/validate.sh trace` reads the Must rows of these tables ([testing strategy](../docs/testing-strategy.md#traceability)).

## Product summary

A keyboard-driven todo app for the terminal, inspired by Things 3 and Todoist, written in Rust with Ratatui. To be completed from the owner's requirements.

## Terms

To be defined with the requirements. Code, tests and specs use these words ([code quality](../docs/code-quality.md#naming-and-size)).

## Out of scope

None recorded yet.

## Decisions

| ID | Date | Decision |
|---|---|---|
