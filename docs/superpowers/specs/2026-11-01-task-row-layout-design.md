# UI refresh: task rows and visual rules

Decided by Noah, 2026-11-01. Built on the `ui-refresh` branch. This records the task row, icons, contrast, corners and spacing, for all platforms, and the task creation pickers on iPhone.

Mockups: https://claude.ai/artifact/Jmd7n1csJfHq2bsRj1SSsN (private). The approved "When and what rows: all scenarios" board was version 19; later versions hold only the picker work.

## Why

Task rows must stay easy to scan when a task has every field set. Labels and project both help tell tasks apart, so neither can be sacrificed for the other.

## Row order

Each row appears only when it has content.

1. **Title**, wrapping across the card's full width, followed by the streak (Phosphor fire icon and count, Keel green).
2. **Deadline**, always its own row: filled red Phosphor flag and the date.
3. **Description**, one line across the full width, 2pt smaller than the title, regular weight, ending in "..." when too long.
4. **Bottom rows**, from the matrix below.

## Bottom rows

The project is always shown and always pinned bottom right. Tasks without a project show "Inbox".

The **when group** is, in order: sub-tasks, date or time with a calendar icon, repeat icon, ringing bell icon (when the task has reminders). Any one of these counts as the group. Repeat shows only the icon, not the frequency.

| When group | Labels | Bottom rows |
|---|---|---|
| No | No | `project` |
| Yes | No | `when` + `project` |
| No | Yes | `labels` + `project` |
| Yes | Yes | `when`, then `labels` + `project` below it |

## Labels on a task row

Labels are grouped by color, with one tag icon per color group. The icon and the label text take the group's color, for example: red tag `red_label 1, red_label 2`, blue tag `blue_label 1, blue_label 2`. Tinted text follows the contrast rule below, so label colors must reach 4.5:1 on the card.

Order follows when labels were added to the task. Color groups appear in the order their first label was added, and labels within a group stay in added order.

## Sub-task icon

Shown with "X of Y":

- None done: Phosphor `circle-dashed`.
- Some done: a ring whose edge fills by the percentage done, in Keel green.
- All done: Phosphor `check-circle`.

## Long text

- The when group never shortens.
- The project shortens beside it, section first, then ends in "...". It never moves to a new row, so a task's row count depends only on which fields it has.
- Labels and project sharing a row shrink together. The section shortens before the project name.
- Hidden labels show a count such as "+2".

## Icons

- Phosphor (MIT, SVG) on every platform. SF Symbols were ruled out because Apple's license limits them to Apple platforms.
- Bold weight, plain: no circles or added edges. Filled weight only where stated, such as the deadline flag.
- Gray icons are dark, `#3d3d3d` on light backgrounds, rather than today's `#8a8a8a`, and pure white on dark backgrounds (Noah, 2026-11-01). Icons whose color means something keep it: dates, flags, label tags, the streak.
- The green add button keeps its circle, since it isn't a gray icon.

## Contrast

- All text meets 4.5:1 against its background, in light and dark. Today's muted `#8a8a8a` is about 3:1 on the app background, so it can't be used for text.
- Secondary text is `#5a5a5a` in light mode.
- Mockup values on white: today `#1f6f55`, overdue dates and deadlines `#b83328`. Tomorrow, this week and later keep their own colors (Noah's phone check, 2026-11-01).
- The empty priority 4 task circle darkens from `#b8b8b8` to `#767676`.
- Dark mode values aren't chosen yet. They follow the same rule.

## Corners

Rounder throughout, using these mockup values:

| Element | Radius |
|---|---|
| Phone task cards | 20px |
| Desktop task cards | 16px |
| Desktop main panel | 20px |
| Sidebar items | 12px |
| Pills and round buttons | Fully rounded |

## Spacing and type

| | Phone | Desktop |
|---|---|---|
| View title | 34/40 bold | 30/36 bold |
| Task title | 17/23 medium | 15/21 medium |
| Secondary text | 14/19 | 13/18 |
| Task circle | 26px | 22px |
| Row padding | 14px by 16px | 12px by 16px |
| Gap from circle to text | 12px | 12px |

- Phone tap targets are at least 44pt, whatever the visible size of the icon or circle.
- The phone page gutter is 16px.
- Desktop surfaces are separated by tint and elevation: tinted sidebar, raised main panel.

## Task creation pickers

Why: pickers placed on the page feel disconnected from the task being typed. On iPhone they belong to the keyboard. Approved in the mockups' "Task creation: every setting" board (version 21).

**Task card.** A rounded card floating 8pt above the keyboard, with nothing behind the keyboard (Noah, 2026-11-01): title, description, a sideways-scrolling row of setting pills (Date, Deadline, Priority, Project, Labels, Reminder, Repeat), and a fixed save button. The Priority, Project and Labels pills show an icon: a flag in the priority's color, the project or section's icon, and a tag. The Labels pill reads in the color of the label chosen last. A row of pills fades on whichever side one runs past, in the card and the task view. In a picker, the chosen priority or label reads in its color, text and icon, and the pill keeps that color while open.

**Pickers.** Each opens as a rounded card just above the task card.

A repeat always has a date: today, by default. While a task repeats, the Date picker offers no "No date".

- **Date:** quick choices (Today, Tomorrow, This weekend, Next week, Pick a date and time, No date).
- **Pick a date and time:** a calendar, then a Time row and a Repeat row. Time is a wheel in the card on touch.
- **Repeat in the date card:** the row shows the current choice. Tapping it opens the repeat options with a way back, and choosing an option returns to the date card.
- **Repeat pill:** the same options, also available on their own.
- **Deadline:** a calendar with no time, plus No deadline.
- **Priority:** 1 to 4 with colored flags.
- **Project:** projects with their sections indented below.
- **Labels:** choose several, or find and add one. Each label has a tag icon in its color to the left of its name, and a Color option that opens the color choices with a way back.
- **Reminder:** choose several from the presets, or type a custom time.

**Fit above the keyboard** (Noah, 2026-11-01, from the simulator and two phone checks).

- **The bar above the keyboard is hidden everywhere**, since its check mark only closed the keyboard. That leaves about 324pt above the task card on an iPhone 17, below the Dynamic Island.
- **Calendars (Date and Deadline):** they need more room, so opening one lowers the keyboard. An X and a check at the top of the calendar go back to typing: the check keeps the choice and the X undoes what changed since the calendar opened. Another pill or a tap outside also brings the keyboard back, keeping the choice.
- **Time:** a Keel wheel inside the date card on touch. The iOS picker would take focus from the title. Desktop types a time.
- **Quick choices and other tall lists:** the keyboard stays up and the list scrolls inside the room above the card.
- **Colors:** the quick choices' text and the open Date pill follow the date's color, like the selected day (later dates in green). A deadline's selected day, open pill and set pill text are Keel green.

**Still open:**

- The label color palette, which is a placeholder. Colors must pass the contrast rule.
- How pickers look on desktop.

## Other screens

Approved in the mockups' iPhone and desktop screens boards (version 25). Desktop mirrors the iPhone layouts, with only the spacing adjusted.

**Task view**

- Title and description, then one rounded card of the filled-in fields, each with its icon: streak, date and time, repeat, reminders, deadline, priority, project, labels.
- The header has the X and the ••• menu, with Back inside a sub-task. The circle beside the title stays, as the way to complete a task from its view (Noah, 2026-11-01).
- **Streak row:** repeating tasks with a streak show the fire icon, the current streak in its repeat's unit ("3 week streak", or "Streak of 3" when one occurrence isn't one day, week, month or year; `keel_core::streak::streak_text`) and the best (`streak_best`). It opens the existing streak panel. Enabling streaks and deleting the task stay in the ••• menu.
- **Unset options:** one sideways-scrolling row of dotted "+ Field" pills, below the filled-in fields and above sub-tasks.
- **Inbox:** an Inbox task has no project row and shows a "+ Project" pill. Task lists still show "Inbox".
- **Sub-tasks:** a card under a header with the same progress icon as the list and "X of Y". With no sub-tasks, the count is hidden and only "Add sub-task" shows.

**Sidebar**

- The Keel logo and wordmark on the left, with the theme and settings icons together on the right. Search field below.
- Inbox, Today, Upcoming and each project show how many tasks they list (`keel_core::views::sidebar_counts`). A row with none shows no number.
- Separate rounded cards for Inbox, Today and Upcoming, then My Projects, Labels (colored tags) and Filters, each with an add control.

**Search**

- Search lives in a field in the sidebar, and a magnifying glass at the top right of the main page opens it too. On the phone it opens a Search screen; on desktop results fill the main area. `/` and Cmd+K keep working.
- Results use the same task row as the lists, under a result count.

**Settings**

- Grouped rounded cards with the existing options, in the order Notifications, Appearance, Import, Diagnostics. A full screen on the phone, a rounded dialog on desktop.

## Left to the builder

- Applying the row rules to the desktop layout; the desktop mockup still shows an older row.
- Choosing the dark mode values.
- Where the logic lives: which rows a task shows is about the screen, so it can stay in the frontend, while facts such as the sub-task counts come from `keel-core` as they do now.
- How the "+N" label count is measured.
