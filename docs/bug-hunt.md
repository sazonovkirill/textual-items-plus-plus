# Bug Hunt

- Status: Accepted
- Last updated: 2026-10-14

A bug hunt has agents use the real app the way a user would, to find the bugs the tests miss, and then fixes the ones worth fixing. It was ported from the owner's taskpaper-terminal project on 2026-10-14, at the owner's request. This document defines the round and its roles. The [`bug-hunt` skill](../.agents/skills/bug-hunt/SKILL.md) sequences each role's steps. Everything else, such as claims, the review gate and delivery, follows the [agent workflow](agent-workflow.md).

## The round

A round runs in three steps:

1. **Two or three Bug Finders**, A, B and C, in parallel, each on one [set of areas](#areas).
2. **One Bug Reviewer**, once every Finder has reported.
3. **One to three Bug Fixers** in parallel, each on part of the bugs the Reviewer accepted.

Each role does only its own job: Finders file and fix nothing, the Reviewer labels and fixes nothing, and Fixers fix only the bugs given to them. The coordinator starts each step when the one before it has finished, briefs each role with what the earlier steps produced, and reports to the owner. It does not file, judge or fix bugs itself.

The coordinator is the session the owner asked for the hunt, and each role is a subagent it starts, each Fixer in its own worktree (owner, 2026-10-14). The owner can ask for a number of rounds, or for rounds until the hunt is clean: the coordinator then starts the next round as soon as one ends, and stops after the first round whose Reviewer accepts no P1 or P2 bug.

### Areas

The app's areas are split into three sets, one for each Finder:

- **Set X:** tasks, editing and undo, selection and rearranging (moving, indenting, multi-selection, converting, the clipboard), notes, comments and checklists.
- **Set Y:** the sidebar and projects, views (Inbox, Today, Upcoming, All, Logbook, Trash), dates and tags.
- **Set Z:** storage (saving, reload and external-change conflicts, backups, layout restore), panes, the Navigator, Go to Symbol, the command palette, search, settings, themes and small terminals.

The Finders rotate sets each round, so each set gets a fresh pair of eyes; with two Finders, the round leaves out the set the last round tested most closely. A feature added since the last round goes to the set it belongs to and is named in that Finder's brief, as are the fixes since the last round, which are worth testing again.

## Severity

Agents tend to find something because they are asked to, so the owner asked for honest results over long ones (owner, 2026-10-10, in taskpaper-terminal):

- "Nothing serious found" is a good and expected result. A Finder does not pad its report with nitpicks.
- Priority is honest. **P1:** loses or corrupts the user's data, or crashes. **P2:** a feature does not work in daily use and there is no easy way around it. **P3:** wrong behavior a user meets in daily use, with a way around it. **P4:** cosmetic. P1 and P2 are the major bugs.
- The Reviewer is a real filter: a cosmetic P4 the owner would not notice in daily use is WILL_NOT_FIX.

The coordinator records each round's final priorities as a severity trend, for example `P3 1 · P4 4 · won't-fix 1`, and reports it to the owner after each round.

## Bug Finder

1. Update to the latest `main` and build `tipp`.
2. Read the open Todoist tasks labelled `bug` in the project `textual-items++`, so you file no duplicates. A bug labelled WILL_NOT_FIX is known and stays unfiled.
3. Test your areas in a real terminal with the [`explore-app` skill](../.agents/skills/explore-app/SKILL.md): real tasks, combined features, plausible edge cases. Judge each screen and each saved file against the specs and the Decisions table of [functional-requirements.md](../specs/functional-requirements.md). Put bugs a real user would hit in daily use first.
4. File each new bug as a Todoist task in the Inbox section with the label `bug` and an honest priority. Title it with what goes wrong, in the user's terms. The description opens with who found it, in which round and on which commit, for example `Found by Bug Finder A, round 1 2026-10-14, tested on main 89b96a3 in tmux.`, then gives **Repro** (the shortest key sequence, starting from a file's contents), **Expected** (citing the spec or decision it follows from) and **Actual**. When the app follows the letter of the spec but the result looks wrong, say so: `Possibly spec-conformant; for the Bug Reviewer to judge.` A likely cause in the code is welcome, but not required.
5. Fix nothing and push nothing: you commit no scenarios either, as the Fixer writes the test.
6. Report to the coordinator: each bug's title, priority and link, or that you found nothing worth filing, and which areas held up.

## Bug Reviewer

1. Take every open `bug` task without one of the three labels below, not only those of this round.
2. Check each one against the specs and the Decisions table, where later decisions win, and reproduce it when the report leaves doubt.
3. Give it exactly one label and a one-line comment `Bug Reviewer: <LABEL>. <reason>`, and set its final priority:
   - **WILL_FIX:** a real bug worth fixing. The comment says what the fix should achieve when the report leaves room.
   - **NEEDS_CLARIFICATION:** fixing it needs a product decision only the owner can make. The comment puts the question with the options and a recommendation.
   - **WILL_NOT_FIX:** not a bug, already decided, or a cosmetic P4 the owner would not notice in daily use. Move it to Ready for the owner to see; the label keeps agents from claiming it there ([task selection](agent-workflow.md#backlog-and-task-selection)).
4. Fix nothing and change no code or documents.
5. Report to the coordinator: the verdict and final priority of each bug, and the questions for the owner.

The coordinator asks the owner the NEEDS_CLARIFICATION questions. When the owner answers, the coordinator records the answer as a comment on the task and relabels the bug WILL_FIX or WILL_NOT_FIX by it, and a WILL_FIX bug goes to a Fixer.

## Bug Fixer

The coordinator splits the WILL_FIX bugs between the Fixers by area, so that they do not change the same code, and names each Fixer's bugs in its brief. A Fixer leaves the others' bugs and every NEEDS_CLARIFICATION bug alone.

1. Take each bug through the usual pipeline. A bug named in your brief counts as a task the owner named, though it sits in Inbox: claim it, fix it test-first with a test that fails before the fix, validate, get the fresh review, deliver to `main` and move it to Done ([agent workflow](agent-workflow.md)).
2. Record an owner's answer, or a rule the fix settles, as a decision (`D-n`) and update the spec it changes. Fixers add decisions at the same time, so whoever reaches `main` first keeps the number and the others renumber before pushing.
3. Report to the coordinator: each bug's commit and any new decision, and what you could not fix.
