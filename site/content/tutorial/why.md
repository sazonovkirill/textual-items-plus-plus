+++
title = "Why a plain-text todo app"
weight = 10
description = "Why one TaskPaper file, why Things 3's structure, and why the keyboard."
+++

Before any keys, a word on the three ideas the app is built on.

## One plain-text file

Most todo apps keep your tasks in a database you never see. textual-items++ keeps them all in one `.taskpaper` file. Open the sample file in a text editor and you'll find lines like these:

```
Customer newsletter: @pinned
	A monthly email for our customers. The first issue goes out in November.
	- Pick an email tool @done(2026-10-10)
	- Write the welcome issue @flagged
	- Design the header image
	Launch: @section
		- Import the customer list
```

A line ending in a colon is a project, a line starting with a dash is a task, and tags like `@done` carry the rest. That has some nice consequences:

- **You can read it anywhere:** on a phone, over SSH, in ten years' time.
- **Your tools already work with it.** In Git every change is a readable diff. Sync it with whatever you like, search it with `grep`.
- **There's no lock-in.** The TaskPaper app opens the same file, and if you stop using textual-items++, your list is still a text file.
- **It's safe to edit by hand.** The app keeps any line or tag it doesn't understand exactly as it was.

## A calm structure

The app borrows its shape from Things 3. **Areas** are the parts of your life that never end, like Work or Home. **Projects** are goals that do end, like a newsletter launch or a weekend away, and they live in an area or on their own. **Sections** split a long project into stages, and **tasks** are the single steps.

New things land in the **Inbox**, so you can write them down without deciding where they go. **Today** and **Upcoming** then gather tasks from everywhere by date. You look at a short list of what matters now, not at everything at once, and that keeps a big list calm.

{{ shot(src="tutorial/why", alt="The Customer newsletter project in the app: its description, two tasks with one flagged and showing a 0/3 checklist, and a Launch section with two tasks, with the sidebar of views, projects and areas on the left", caption="The same lines as above, as the app shows them.") }}

## The keyboard

You visit a todo list dozens of times a day, for a few seconds each time, and reaching for the mouse makes every visit slower. Here every action is a key or two away, and most keys follow a pattern: `g` then a letter **g**oes somewhere (`gt` is Today), and `m` then a letter **m**odifies the task you're on (`md` sets its due date). If you know vim, `j` and `k` will feel at home; if not, you'll pick them up in a minute.

The [Reference](@/reference/files.md) says more about the file format and how the app saves it.
