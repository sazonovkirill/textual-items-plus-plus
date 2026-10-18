+++
title = "Adding and editing"
weight = 40
description = "Add tasks, change their titles and descriptions, and use Quick Add."
+++

## Add a task

Press `g` `i` to go to the Inbox, then `G` to move to its last task, "Return the library books". Press `o` to open a new task below it, and type:

```
Order more coffee beans
```

The status line now says `EDIT`: keys type text instead of running commands.

{{ shot(src="tutorial/new-task", alt="The Inbox with a new task, Order more coffee beans, being typed below Return the library books, and EDIT in the status line", caption="Typing a new task. The status line says EDIT.") }}

Press `Enter` (or `Esc`) to finish. You're back in navigate mode, on your new task. `O` adds a task above the focused one instead, and `Space` works like `o`.

## Change a title

Move to "Look into a standing desk" and press `i`. The cursor appears at the end of the title, right in the list. Type ` for the home office` and press `Enter`. The usual text keys work: arrows, `Home` and `End`, `Alt+←` and `Alt+→` by word, `Ctrl+W` to delete a word. An `@` at the start of a word is refused, because tags have their own picker.

## Add a description

Any item can have a description, a few lines of notes shown dimmed under its title. Stay on the standing desk task and press `m` `n`. Type `Something under 300, ask Tom which one he has.` and press `Esc`. In a description `Enter` starts a new line, so `Esc` is the key that finishes.

## The edit dialog

Move to "Call the vet about Pepper's vaccinations" and press `Enter`. The edit dialog opens at the bottom with every property of the task: title, description, tags, start and due dates, and whether it's flagged.

{{ shot(src="tutorial/edit-dialog", alt="The Edit Task dialog at the bottom of the screen for Call the vet about Pepper's vaccinations, with fields for title, description, tags, start, due and flagged", caption="The edit dialog shows everything about a task in one place.") }}

`Tab` and `Shift+Tab` move between fields, and `Enter` confirms one. Each change is applied as soon as you confirm it, so there's nothing to save: `Esc` simply closes the dialog. Press `Esc` now.

## Quick Add

To jot something down without leaving what you're looking at, press `q`. A prompt opens at the bottom. Type:

```
Renew the car insurance next week
```

The words `next week` light up, and the line below shows the start date they mean. Type `/` to choose where the task goes from a list of places; without it, the task goes to the Inbox.

{{ shot(src="tutorial/quick-add", alt="The Quick Add prompt at the bottom with Renew the car insurance next week typed, next week highlighted, and the start date it means shown below", caption="Quick Add understands dates in plain words.") }}

Press `Enter` to add it. You won't see it in the Inbox, because the Inbox only lists tasks that have no start date yet. It's waiting in Upcoming, which you'll visit in the dates chapter.

The [Reference](@/reference/editing.md) covers every editing key and Quick Add's place picker.
