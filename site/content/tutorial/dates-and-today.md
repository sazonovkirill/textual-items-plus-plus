+++
title = "Dates and Today"
weight = 70
description = "Give tasks start and due dates in plain words, and plan your day in Today."
+++

A task can have two dates: the **start** date, when you want to begin it, and the **due** date, when it has to be done. They show as chips at the end of the row, green for start and red for due, named the way you'd say them: `tomorrow`, `Fri`.

## Start today

Go to the Work area and press `m` `t` on "Submit the expense report": **start today**. Then press `g` `i` and do the same on "Call the vet about Pepper's vaccinations". The vet task leaves the Inbox: a task with a start date has been scheduled, and the Inbox only holds what you haven't dealt with yet.

## Any date, in plain words

Go to the Home area, move to "Pay the council tax" and press `m` `d` for its due date. A prompt opens at the bottom. Type `fri`: the line below shows the date it understood. Press `Enter`.

Now open the "Weekend in Edinburgh" project with `Enter` and set two more:

- On "Book the train tickets", press `m` `d`, type `tomorrow` and press `Enter`.
- On "Find a place to stay near the Old Town", press `m` `s` for the start date, type `+3d` and press `Enter`.

The prompt also understands words like `next week`, `weekend` or `in a month`, offsets like `2 weeks`, and dates like `2026-12-24`. Erase the text and press `Enter` to remove a date.

To nudge a date without typing, use `]` and `[` to move the start date a day later or earlier, and `}` and `{` for the due date. Press `]` on the place to stay: its chip moves on by a day.

## Today

Press `g` `t`. **Today** lists everything from the whole file that starts or is due today or earlier, overdue first: the vet call, and the expense report with "Work" under it to say where it lives.

Some things are for after work. On "Submit the expense report", press `m` `e`: it moves under an **Evening** heading at the bottom. `m` `e` works only in Today, and the Evening empties overnight.

{{ shot(src="tutorial/today", alt="The Today view with Call the vet about Pepper's vaccinations, and below an Evening heading with a moon, Submit the expense report with Work as its location", caption="Today, with one task saved for the evening.") }}

## Upcoming

Press `g` `u`. **Upcoming** lists everything dated after today, grouped by day: the train tickets, the place to stay, the council tax and the car insurance from Quick Add.

{{ shot(src="tutorial/upcoming", alt="The Upcoming view grouped by day: Book the train tickets under Tomorrow with a red due chip, Find a place to stay near the Old Town later in the week with a green start chip, and Pay the council tax under Friday with a red due chip", caption="Upcoming shows the days ahead.") }}

## Later

Some tasks have no date, just "not now". In the Inbox, press `m` `l` on "Look into a standing desk". It's now **Later**: dimmed and moved to the bottom of the list until you press `m` `l` again. A whole project can be Later too.

Every date word the prompt understands is in the [Reference](@/reference/dates.md), and the views in [Views](@/reference/views.md).
