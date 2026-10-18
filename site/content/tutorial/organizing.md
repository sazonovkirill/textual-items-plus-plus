+++
title = "Organizing"
weight = 60
description = "Group tasks into sections and projects, move them, and select several at once."
+++

The Home area has a loose pile of tasks. Time to sort it out.

Press `Tab` to go to the sidebar, move down to **HOME** and press `Tab` again. The focus is on "Fix the dripping bathroom tap".

## Select several tasks

Press `V` for **visual line mode**: the status line says `VISUAL LINE`, and every move stretches the selection. Press `j` once to select the tap and "Replace the hallway light bulb" together. Whatever you do now acts on both. (`J` and `K` also extend a selection, and `v` adds or removes just the focused task.)

## Group into a section

With the two tasks selected, press `w` `s`. They become the tasks of a new section, and the cursor waits for its name. Type `Repairs` and press `Enter`.

Sections always come after the loose tasks of an area or project, so Repairs now sits below "Pay the council tax".

## Turn a task into a project

"Plan Grandma's 90th" is more than one task: it has a two-step checklist. Press `k` to move up to it and press `m` `y`. It becomes a project in Home, its checklist items become its tasks, and it appears under HOME in the sidebar.

{{ shot(src="tutorial/organized", alt="The Home area with Pay the council tax, a Repairs section holding the tap and light bulb tasks, and two projects, Plan Grandma's 90th and Weekend in Edinburgh; the sidebar lists Plan Grandma's 90th under HOME", caption="Home, with a section and a new project.") }}

`w` `p` works like `w` `s` but wraps the selected tasks in a new project instead.

## Move a task somewhere else

The birthday card in the Inbox belongs to the party. Press `g` `i`, move to "Buy a birthday card for Grandma" and press `M`. A picker opens with every place a task can go. Type `grandma`: the list narrows as you type.

{{ shot(src="tutorial/move-to", alt="The Move to picker over the Inbox, with grandma typed and Plan Grandma's 90th in Home as the only match", caption="Move to finds a place by a few letters of its name.") }}

Press `Enter`. The card leaves the Inbox and joins the project.

To reorder tasks within a list, use `Alt+j` and `Alt+k`. Move to "Return the library books" and press `Alt+k` twice: it climbs to the top of the Inbox. Moving a task past a section heading takes it into that section.

## New projects and sections

Press `H` to go back to Home. Press `n` `p`, type `Paint the bedroom` and press `Enter`: that's a new project in the area. On its own, `s` adds a new section at the bottom of the area or project you're in, the same way.

Everything about areas, projects and sections is in the [Reference](@/reference/organization.md), and selections in [Selection](@/reference/selection.md).
