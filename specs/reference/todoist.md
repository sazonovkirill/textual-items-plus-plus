# Todoist Reference
- Status: Reference
- Last updated: 2026-10-13

Factual notes on Todoist, taken from the official Todoist Help Center (todoist.com/help) and Todoist's own
"Inspiration" blog. Every claim links its source inline. Anything no official source confirmed is marked
**(unverified)**. Plan names: Beginner (free), Pro, Business.

## 1. Data model

### Projects
- Projects hold tasks. Each has a color (picked from the hashtag icon). Projects with no color, including
  Inbox, show gray. The color appears next to the project name in the sidebar
  ([Introduction to projects](https://www.todoist.com/help/articles/introduction-to-projects-TLTjNftLM)).
- **Sub-projects:** "Todoist currently supports sub-projects on three indent levels for personal projects."
  You create one by dragging a project left or right in the sidebar. Team projects can't have sub-projects:
  they use **folders** instead. When you hit the limit, Todoist suggests sections
  ([Create a sub-project](https://www.todoist.com/help/articles/create-a-sub-project-in-todoist-aTA15C70)).
- Sharing a parent project doesn't share its sub-projects, and duplicating a parent doesn't duplicate them
  ([Introduction to projects](https://www.todoist.com/help/articles/introduction-to-projects-TLTjNftLM)).
- Filters can target a project with `#Work`, or a project plus its sub-projects with `##Work`
  ([Introduction to filters](https://www.todoist.com/help/articles/introduction-to-filters-V98wIH)).
- Projects can have a **description** of up to 16,383 characters, shown as a one-line preview under the name
  ([Introduction to projects](https://www.todoist.com/help/articles/introduction-to-projects-TLTjNftLM)).
- **Favorites** pin projects to the top of the sidebar. **Archive** hides a project. Unarchiving restores
  all of its data: tasks, comments, reminders and attachments. **Delete** is permanent. A team project
  must be archived before it can be deleted (same source).
- **Limits:** Beginner 5 active projects, Pro 300, Business 500 (same source).
- **Layouts:** a project can show as **list**, **board** or **calendar**. `Shift+V` switches between them.
  Calendar layout needs Pro or Business
  ([Keyboard shortcuts](https://www.todoist.com/help/articles/use-keyboard-shortcuts-in-todoist-Wyovn2)).

### Sections
- Sections split a project into groups. A project can have up to **20 sections**
  ([Introduction to sections](https://www.todoist.com/help/articles/introduction-to-sections-rOrK0aEn)).
- **Sections have descriptions** on all plans, shown as a one-line preview below the section name (same
  source).
- You can reorder sections by dragging, move a section to another project, or archive it. An archived
  section's tasks show as completed, and they stay completed when the section is unarchived (same source).
- In board layout, each section is a column (same source).

### Tasks
- **Fields:** name, description, date (the "due date"), deadline, duration, priority, labels, reminders,
  comments, sub-tasks
  ([Introduction to tasks](https://www.todoist.com/help/todoist/features/introduction-to-tasks-080OAXric)).
  The help pages give no length limit for task names or descriptions **(unverified)**.
- **Priority:** four levels that can't be customized. P1 red ("Most important, urgent"), P2 orange,
  P3 blue, P4 none or white ("Least important"). **New tasks default to P4**
  ([Set a priority](https://www.todoist.com/help/articles/set-a-priority-in-todoist-Wy82Jp)).
- **Labels:** typed as `%label` in the task name. The page says "`@` also works for now" but is "planned to
  be retired by the end of 2026". Personal labels keep their colors. Shared labels (from collaborators)
  are always gray. Deleting a label leaves it on completed tasks
  ([Introduction to labels](https://www.todoist.com/help/articles/introduction-to-labels-dSo2eE)).
- **Date vs deadline:** "While dates schedule when you plan to work on a task, deadlines represent the
  hard cutoff." Deadlines are for "one-time, fixed dates", not recurring ones. They're typed as
  `{next Friday}` and need Pro or Business. Filters: `deadline: December 31`, `deadline before`,
  `deadline after`, `no deadline`
  ([Introduction to deadlines](https://www.todoist.com/help/todoist/features/introduction-to-deadlines-in-todoist-uMqbSLM6U)).
- **Duration:** needs Pro or Business, and the task must already have a date **and** a time. It's typed
  as `for 2h`, `for 1h15m`, `from 21:00 to 22:00` or `9:30-10:00`. The maximum is **24 hours**, and
  multi-day durations aren't supported
  ([Set a task duration](https://www.todoist.com/help/articles/set-a-task-duration-L1kYkZv8d)).
- **Reminders:** automatic (added whenever a task gets a date and time), custom (an absolute time, or
  relative such as "1h before"), recurring, and location. Inline `!` syntax and multiple custom reminders
  need Pro or Business
  ([Introduction to reminders](https://www.todoist.com/help/articles/introduction-to-reminders-9PezfU)).
- **Sub-tasks:** "Todoist supports four indent levels." Inline indenting needs list layout with no sorting
  or grouping, and both tasks in the same project. `Ctrl+]` indents and `Ctrl+[` outdents. Completed
  sub-tasks show in the parent's task view under "Show completed"
  ([Use sub-tasks](https://www.todoist.com/help/articles/introduction-to-sub-tasks-kMamDo)).
  The help page doesn't say what happens to sub-tasks when the parent is completed **(unverified)**.

### Inbox
- Inbox is the built-in default project. It has no color, so it shows gray
  ([Introduction to projects](https://www.todoist.com/help/articles/introduction-to-projects-TLTjNftLM)).
  `G then I` (or `Cmd/Ctrl+1` on desktop) goes to it
  ([Keyboard shortcuts](https://www.todoist.com/help/articles/use-keyboard-shortcuts-in-todoist-Wyovn2)).
  Tasks added without a project land in Inbox **(unverified; widely known but not quoted here)**.

## 2. Natural-language dates ("smart date recognition")

Source for this list:
[Schedule a date and time](https://www.todoist.com/help/articles/introduction-to-dates-and-time-q7VobO).

| Input | Meaning |
|---|---|
| `today` / `tod` | today |
| `tomorrow` / `tom` | tomorrow |
| `next week` | next Monday by default (configurable in Settings) |
| `next month` | same day of the month, one month later |
| `next <weekday>` | that weekday in the following week. Example: "next Friday" is "the second Friday to occur" |
| `Friday` | the next Friday |
| `jan 27`, `27 jan`, `27/1` | January 27 |
| `01/27/2023` | January 27, 2023 (other date orders also work) |
| `27th` | the 27th of the current month |
| `mid January` | January 15 |
| `end of month` | the last day of the current month |
| `Fri 7pm` | Friday at 7pm |
| `6pm` | today at 6pm, or tomorrow if 6pm has passed |
| `in 5 days`, `+5 days` | 5 days from today |
| `in 3 weeks`, `in 2 hours` | counted forward from now |
| `in the morning` / `afternoon` / `evening` | 9am / 12pm / 7pm |
| `tom morning` / `afternoon` / `evening` / `night` | tomorrow at 9am / 12pm / 7pm / 10pm |
| `someday` | two months from now |
| `later this week` | a later day this week |
| `this weekend` / `next weekend` | the coming Saturday / the Saturday after that |
| `next year` | January 1 of next year |
| `3rd friday jan` | the third Friday of next January |
| `50 days before new year's eve`, `6 weeks before 21 Jul`, `28 days after 21 July` | dates relative to another date |
| `today at 10`, `tomorrow at 16:00` | a date with a time |
| `no date`, `no due date` | removes the date |

- **Times:** a time without a colon, such as `1300`, is **not** recognized. Use `13:00` (same source).
- **`eod`:** no official page lists it **(unverified; don't assume it is supported)**.
- **Recurring** ([Introduction to recurring dates](https://www.todoist.com/help/todoist/features/introduction-to-recurring-dates-YUYVJJAV),
  [Dates and time](https://www.todoist.com/help/articles/introduction-to-dates-and-time-q7VobO)):
  - `every day` / `daily`
  - `every weekday` / `every workday`
  - `every other week`, `every other Wednesday`
  - `every July 19`
  - `every 2 days at 12:00`
  - `every last workday at 3pm`
  - `everyday starting on aug 3`, `everyday ending aug 3` (the end date is inclusive)
  - `every 3rd Tuesday starting Aug 29 ending in 6 months`
  - `every! 2 months` counts from the day you complete the task, and `after 10 days` becomes `every! 10 days`
  - `monthly` is recognized inside "Create monthly report"
- **Deadline:** natural language in curly braces, such as `{Feb 28}` or `{march 30}`. It can sit next to
  a date: `every 2 days at 12:00 {Feb 28}` (same sources).
- **Highlighting:** recognized date text is highlighted in the task name as you type
  ([Turn smart date recognition on or off](https://www.todoist.com/help/articles/turn-smart-date-recognition-on-or-off-63WfIr)).
- **Undoing recognition:** on web or desktop, click the highlighted word, or press Delete/Backspace right
  after it's highlighted. "The word or phrase becomes a part of the task name." You can also turn
  recognition off globally in Settings > General > Smart date recognition. After that, dates can only be
  set through the scheduler (same source).

### Quick Add inline syntax
Source: [Use Task Quick Add](https://www.todoist.com/help/articles/use-task-quick-add-in-todoist-va4Lhpzz),
plus the shortcuts page.

| Token | Effect | Example |
|---|---|---|
| `#name` | project | `#Work` |
| `/name` | section (after the project) | `#Work /Admin` |
| `%name` (legacy `@name`) | label | `%email` |
| `p1`-`p4` | priority (the Quick Add page lists p1-p3, since p4 is the default) | `p1` |
| `+name` | assignee (shared projects only; set `#project` first) | `+Lucile` |
| `!time` | reminder (Pro/Business) | `!14:00`, `!30 min before` |
| `{date}` | deadline | `{march 30}` |

The description isn't typed inline. Press `↓` or click `+` under the name to open the description field.
Quick Add opens with `Q`.

## 3. Comments
Main source: [Add comments or files to tasks](https://www.todoist.com/help/articles/introduction-to-comments-and-file-uploads-CwiA50).
- **Task comments** are added from the speech-bubble icon or the task view. `C` comments on the focused
  task ([Keyboard shortcuts](https://www.todoist.com/help/articles/use-keyboard-shortcuts-in-todoist-Wyovn2)).
- **Project comments** belong to the whole project and open from the project's three-dot menu, under
  "Comments".
- **Ordering:** comments are chronological. "You can't manually change their order."
- **Edit/delete:** each comment has a three-dot menu with Edit and Delete. You can only delete your own
  comments. Removing an attached file is done with Edit.
- **Attachments:** files (pdf, docx, xlsx, pptx, csv, png, jpg, gif, webp, mp4, mov, webm, zip, and more),
  audio recordings and emoji. The size limit isn't documented on this page **(unverified)**.
- **Notifications:** use `@mention` or the Notify option. They only apply in shared projects.
- **Archiving:** comments are kept when a project is archived and restored when it's unarchived
  ([Introduction to projects](https://www.todoist.com/help/articles/introduction-to-projects-TLTjNftLM)).
- **Timestamp display:** not documented. In practice each comment shows its author and a date and time
  next to it **(unverified)**. Markdown formatting in comments is also **(unverified)**.
- Reporting logs "added comments" and "deleted comments" events
  ([View the activity log](https://www.todoist.com/help/articles/view-the-activity-log-oOra6D)).

## 4. Views
- **Today:** "every task scheduled for today across all your projects". Tasks with no date don't appear.
  P1 tasks show at the top in red. Tasks can be sorted or grouped, for example grouped by project. In list
  layout, dragging a task to the bottom of Today postpones it to tomorrow
  ([Plan your day with Today](https://todoist.com/help/articles/plan-your-day-with-the-today-view-UVUXaiSs)).
  Overdue tasks show in a separate group at the top **(unverified in help text)**.
- **Upcoming:** "every task scheduled for the next 7 days (and beyond) across all your projects", grouped
  day by day. It has a week picker, arrows to move between weeks, a "Today" button, and a **Reschedule**
  button for overdue tasks. List layout scrolls through the days, board layout shows one column per day,
  and calendar layout (Pro) shows a week or month with a "Plan" sidebar. Dragging a task to a day
  reschedules it. You can look up to two years ahead
  ([Plan your week with Upcoming](https://www.todoist.com/help/articles/plan-your-week-with-the-upcoming-view-OKOg1mR8)).
  Keys: `Shift+→/←` for next or previous week
  ([Keyboard shortcuts](https://www.todoist.com/help/articles/use-keyboard-shortcuts-in-todoist-Wyovn2)).
- **Filters & Labels:** filter queries combine `today`, `overdue`/`od`, `p1`, `%label`, `#project`,
  `##project`, `&`, `|`, `!` and parentheses, for example `(today | overdue) & #Work`. Filters can't
  include completed tasks
  ([Introduction to filters](https://www.todoist.com/help/articles/introduction-to-filters-V98wIH),
  [View completed tasks](https://www.todoist.com/help/articles/view-completed-tasks-in-todoist-J19h2s)).
- **Completed tasks:** in a project, Display > "Completed tasks" shows them at the bottom of each section
  or parent. Unchecking a completed task restores it. **Reporting** (the activity log) lists completions
  from every project and records added, edited, completed, uncompleted and deleted tasks, plus comments.
  Beginner sees 7 days of history, and paid plans see the full history
  ([View completed tasks](https://www.todoist.com/help/articles/view-completed-tasks-in-todoist-J19h2s),
  [View the activity log](https://www.todoist.com/help/articles/view-the-activity-log-oOra6D)).
- **Deleted tasks: there is no trash.** "Once the task is deleted, it can't be restored to your
  project." "There's no option to undo task deletion." The only way back is from a backup or a CSV export
  ([Introduction to tasks](https://www.todoist.com/help/todoist/features/introduction-to-tasks-080OAXric)).
- **Search** (`/` or `F`): results are grouped into Top, Tasks, Task descriptions, Projects, Sections,
  Comments, Labels (and Folders). Up to 5 recent searches are kept. Completed tasks only appear after you
  choose "Search completed tasks", and partial words don't match them
  ([Introduction to search](https://todoist.com/help/articles/how-to-use-search-fAfiDSAp)).

## 5. Command menu (Cmd/Ctrl+K)
- `Cmd/Ctrl+K` opens it. The shortcuts page calls it "Quick Find"
  ([Keyboard shortcuts](https://www.todoist.com/help/articles/use-keyboard-shortcuts-in-todoist-Wyovn2)).
- Todoist calls it "your central hub of actions in Todoist that helps you navigate, add, and edit tasks
  without leaving your keyboard". Example entries are Quick Add, Go to home view, Go to Inbox and Go to
  Today. It also finds projects, and users describe it as a way to "learn or be reminded of" the shortcuts
  ([Command menu article](https://www.todoist.com/inspiration/todoist-command-menu-keyboard-shortcuts)).
- Typing filters the list, and each command shows its keyboard shortcut next to it
  **(unverified; not stated in the help text)**.
- `?` opens the full shortcut list. `Esc` closes dialogs.

## 6. Keyboard shortcuts (web/desktop)
Source: [Use keyboard shortcuts in Todoist](https://www.todoist.com/help/articles/use-keyboard-shortcuts-in-todoist-Wyovn2).
Shortcuts work on QWERTY and QWERTZ layouts.

| Key | Action |
|---|---|
| `Q` | Quick Add |
| `A` / `Shift+A` | add a task at the bottom / top of the list |
| `/` or `F` | search |
| `Cmd/Ctrl+K` | command menu / Quick Find |
| `?` | show keyboard shortcuts |
| `M` | toggle sidebar |
| `Esc` | dismiss or cancel |
| `J`/`↓`, `K`/`↑` | move focus down / up |
| `←`/`→`, `Tab`/`Shift+Tab` | move focus left/right; cycle through task view elements |
| `G` then `I`/`T`/`U`/`L`/`P`/`V`/`A` | go to Inbox / Today / Upcoming / Labels / Projects / Filters & Labels / Reporting |
| `H` or `G H` | go Home |
| `Shift+G` | open the task in its project |
| `Enter` | open the task view |
| `Cmd/Ctrl+E` | edit task |
| `E` | complete the focused task |
| `T` / `Shift+T` | set / remove the date |
| `1`-`4`, `Y` | set priority / change priority |
| `C` | comment |
| `L` | add label |
| `V` | move to... |
| `Shift+R` | assign |
| `.` | more task actions |
| `X` | select the focused task (multi-select). `,` focuses the multi-select toolbar |
| `Cmd+Delete` / `Shift+Delete` | delete the selected task(s) |
| `Shift+Cmd/Ctrl+C` | copy link to task |
| `Ctrl+]` / `Ctrl+[` | indent / outdent sub-task |
| `Shift+E` | show/hide sub-tasks |
| `Cmd/Ctrl+Alt+0` | collapse/expand sections, sub-tasks and folders |
| `D`/`P`/`N`/`R` | sort by date / priority / name / assignee |
| `S` | add section |
| `Shift+V` | change layout (list/board/calendar) |
| `W` | more project actions |
| `Shift+→/←` | next / previous week (Upcoming, calendar) |
| `Enter` / `Ctrl+Enter` (while editing) | save and create a task below / above |
| `Cmd/Ctrl+↑/↓` (while editing) | move to the task above / below |

## 7. Relevant to textual-items++ (inferred)
Everything in this section is **inferred** from the facts above. These are design suggestions, not
Todoist facts.
- **Nestable projects:** Todoist caps sub-projects at 3 levels and sub-tasks at 4. A fixed, small depth
  limit is a reasonable and proven cap. Keep a filter that means "this project and its sub-projects"
  (like `##`).
- Archiving a parent project should keep its children's data so it can be restored. Todoist restores
  everything on unarchive.
- Section descriptions and project descriptions both exist in Todoist. A one-line preview under the
  heading is the established display.
- **Comments (single user):** keep them chronological and append-only in order, with edit and delete
  through a per-comment menu. A per-task shortcut (`C`) jumps to comments. Attachments, @mentions and
  notifications can be dropped.
- Since Todoist has no trash, deletion in textual-items++ could add an undo or trash, to improve on it.
- **NLP dates:** support the core table above. Priority set: `today/tod`, `tomorrow/tom`, weekday
  names, `next week`, `in N days/weeks`, `+N days`, `jan 27`/`27 jan`, `mid <month>`, `end of month`,
  `someday`, times with a colon, and `no date`. Let `next <weekday>` follow Todoist's documented meaning
  (the following week).
- Highlight the recognized phrase in the input, and let one keystroke (Backspace right after the match)
  turn it back into plain title text, as Todoist does. Offer a global switch to turn recognition off.
- Inline tokens worth borrowing: `#project`, `/section`, `p1`-`p4`, `{deadline}`, and labels (`%` is
  Todoist's new label sigil; `@` is being retired).
- Keyboard: `j/k` navigation, `q`/`a` to add, `e` to complete, `t` for date, `c` for comment, `v` to move,
  `g`-prefixed jumps, `?` for help, and a `Ctrl+K` command menu that lists commands with their shortcuts.

## Sources
- https://www.todoist.com/help/articles/introduction-to-dates-and-time-q7VobO
- https://www.todoist.com/help/articles/turn-smart-date-recognition-on-or-off-63WfIr
- https://www.todoist.com/help/todoist/features/introduction-to-recurring-dates-YUYVJJAV
- https://www.todoist.com/help/todoist/features/introduction-to-deadlines-in-todoist-uMqbSLM6U
- https://www.todoist.com/help/articles/set-a-task-duration-L1kYkZv8d
- https://www.todoist.com/help/articles/introduction-to-reminders-9PezfU
- https://www.todoist.com/help/articles/use-task-quick-add-in-todoist-va4Lhpzz
- https://www.todoist.com/help/articles/use-keyboard-shortcuts-in-todoist-Wyovn2
- https://www.todoist.com/inspiration/todoist-command-menu-keyboard-shortcuts
- https://www.todoist.com/help/articles/introduction-to-projects-TLTjNftLM
- https://www.todoist.com/help/articles/create-a-sub-project-in-todoist-aTA15C70
- https://www.todoist.com/help/articles/introduction-to-sections-rOrK0aEn
- https://www.todoist.com/help/todoist/features/introduction-to-tasks-080OAXric
- https://www.todoist.com/help/articles/introduction-to-sub-tasks-kMamDo
- https://www.todoist.com/help/articles/set-a-priority-in-todoist-Wy82Jp
- https://www.todoist.com/help/articles/introduction-to-labels-dSo2eE
- https://www.todoist.com/help/articles/introduction-to-filters-V98wIH
- https://www.todoist.com/help/articles/introduction-to-comments-and-file-uploads-CwiA50
- https://todoist.com/help/articles/plan-your-day-with-the-today-view-UVUXaiSs
- https://www.todoist.com/help/articles/plan-your-week-with-the-upcoming-view-OKOg1mR8
- https://www.todoist.com/help/articles/view-completed-tasks-in-todoist-J19h2s
- https://www.todoist.com/help/articles/view-the-activity-log-oOra6D
- https://todoist.com/help/articles/how-to-use-search-fAfiDSAp
