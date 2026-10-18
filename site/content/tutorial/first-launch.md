+++
title = "First launch"
weight = 20
description = "Open the sample file and find your way around the screen."
+++

## Open the file

In a terminal, go to the folder with the sample file and run:

```sh
tipp tutorial.taskpaper
```

The screen has three parts.

{{ shot(src="tutorial/first-launch", alt="textual-items++ at first launch: the sidebar on the left lists Inbox, Today, Upcoming, Flagged, All, Logbook and Trash, a pinned Customer newsletter project, Bake sourdough bread, and the Work and Home areas with their projects; the pane on the right shows the Inbox's four tasks", caption="The sample file at first launch.") }}

**The sidebar** on the left lists, from the top:

- the **Inbox**, with the number of tasks waiting in it;
- the **views**: Today, Upcoming, Flagged, All, Logbook and Trash;
- **pinned projects**, here Customer newsletter, which you can reach without scrolling;
- **projects in no area**, here Bake sourdough bread;
- the **areas**, WORK and HOME, each followed by its projects.

**The pane** on the right shows whatever the sidebar points at, the Inbox to begin with. The blue bar on the left of "Call the vet about Pepper's vaccinations" marks the **focused** task: most keys act on it.

**The status line** at the bottom says which mode you're in (`NAVIGATE` for now), the file's name, and whether your changes are saved.

## Ask for help

Press `?`. A panel opens on the right with the most useful keys for what you're doing right now. It changes as you go: open a dialog, and it shows that dialog's keys. Press `)` and `(` to scroll it, and `?` again to close it.

{{ shot(src="tutorial/shortcuts", alt="The shortcuts panel open on the right of the screen, listing keys under Moving, Selection and Going, such as Next Item j and Go to Today gt", caption="The shortcuts panel follows what you're doing.") }}

## Quit, and saving

Press `Ctrl+Q` to quit. There's no save key to remember: the app writes every change to the file by itself, a moment after you make it. Run `tipp tutorial.taskpaper` again and you're back where you were.

Run `tipp` with no file at all to try a demo list that's never saved.

The [Reference](@/reference/keyboard.md) has more on the shortcuts panel and on keys in different terminals.
