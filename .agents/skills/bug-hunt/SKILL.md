---
name: bug-hunt
description: Take a role in a textual-items++ bug hunt round, as Bug Finder, Bug Reviewer or Bug Fixer, or run the rounds as the coordinator. Use when a brief names one of these roles or asks for a bug hunt.
---

# Bug hunt

The rules are in [the bug hunt](../../../docs/bug-hunt.md); this is the procedure. Do your own role's steps and nothing else.

## Coordinator

1. Start two or three Bug Finders in parallel as subagents, each with its [set of areas](../../../docs/bug-hunt.md#areas), rotated from the last round, plus the features added and the bugs fixed since then.
2. When every Finder has reported, start the Bug Reviewer with their lists.
3. Ask the owner the Reviewer's NEEDS_CLARIFICATION questions. Split the WILL_FIX bugs by area and start the Bug Fixers in parallel, each in its own worktree, naming each one's bugs and the latest `D-n` on `main`.
4. Report to the owner: questions, fixes and the severity trend. When the owner asked for rounds until the hunt is clean, start the next round unless this one's Reviewer accepted no P1 or P2 bug.

## Bug Finder

1. Update to the latest `main` and build: `cargo build --release --manifest-path app/Cargo.toml`.
2. List the open Todoist tasks labelled `bug` in the project `textual-items++`.
3. Explore your areas with the [`explore-app` skill](../explore-app/SKILL.md), steps 2 to 5.
4. For each new bug, add a task in the Inbox section with the label `bug`, an honest priority, and a description that opens with `Found by Bug Finder <A|B|C>, round <n> <date>, tested on main <commit> in tmux.` followed by **Repro**, **Expected** and **Actual**.
5. Report to the coordinator each bug's title, priority and link, or that you found nothing worth filing.

## Bug Reviewer

1. List the open `bug` tasks that carry none of WILL_FIX, NEEDS_CLARIFICATION and WILL_NOT_FIX.
2. For each, check the specs and Decisions table, and reproduce it when in doubt.
3. Set the task's labels to its existing labels plus exactly one verdict, set its final priority, and comment `Bug Reviewer: <LABEL>. <reason>` in one line. Move a WILL_NOT_FIX task to Ready.
4. Report to the coordinator each bug's verdict and priority, and the questions for the owner.

## Bug Fixer

1. For each bug in your brief, run the [`claim-task` skill](../claim-task/SKILL.md) on it.
2. Write a test that fails because of the bug, then fix it, and run the [`review-commit` skill](../review-commit/SKILL.md).
3. Record any new decision as the next `D-n`; renumber it if another Fixer reached `main` with that number first.
4. Run the [`deliver-task` skill](../deliver-task/SKILL.md).
5. Report to the coordinator each bug's commit and any new decision.
