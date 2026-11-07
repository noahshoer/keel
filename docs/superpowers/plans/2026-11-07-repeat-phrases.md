# Repeat phrases and repeat ends (issue #8, part a)

Issue #8 asks for "every X days", "for X days" and "Xth day/week of the month". An audit of the quick-add parser (2026-11-07) found that intervals and nth weekdays already work. These failed or were misread:

| Typed | Before |
|---|---|
| every three days, every two weeks | stays text (no word numbers) |
| every second week, every 3rd day | text / read as "every 3rd" + leftover "day" |
| every 2nd day of the month | "every 2nd", leftover "day of the month" |
| every 2nd week of the month | "every 2nd", leftover "week of the month" |
| every month on the 2nd monday | read as the 2nd of the month |
| every 3 months on the 1st | the day ignored, leftover "on the 1st" |
| every tue & thu | only Tue |
| every weekend | "weekend" read as a date, "every" left over |
| every 10 days starting nov 1 | "starting" left in the title |
| every day for 10 days, every day until nov 30 | the end ignored; "for 10 days" / "until" left in the title |

## Decisions (Noah, 2026-11-07)
- "for N days/weeks/months" and "until <date>" **end the repeat**: completing the last occurrence finishes the task for good.
- "every Nth week of the month" is the **Monday** of that week; "… on fri" picks the weekday instead. Both are `MonthlyNthWeekday` (so "2nd week on Fri" is the 2nd Friday), and "last week of the month" is the last Monday. No new `Recurrence` variant.

## a1: parser only (no model change)
- Word numbers one–twelve in intervals: "every three days", "every two weeks".
- Ordinal + unit is an interval: "every second week" = every 2 weeks, "every 3rd day" = every 3 days. With "of the month", a day is the date ("every 2nd day of the month" = the 2nd) and a week is the nth week (above).
- "every month on the 2nd monday" → nth weekday; "every N months on the 1st" → that day.
- "&" joins weekdays like "and".
- "every weekend" → Sat and Sun.
- "starting / from / beginning <date>" right after a repeat is part of it (the date anchors the repeat).

## a2: repeat end
- `Due.until: Option<String>` (`YYYY-MM-DD`, `#[serde(default)]`), set from `QuickAdd.until` through the editor plans and `DueEdit.until`.
- "for N days|weeks|months" ends N units after the first occurrence, minus a day ("every day for 10 days" = 10 days); "until <date>" ends on that date.
- Completing a repeating task whose next occurrence is past `until` completes it for real (like a plain task).
- `Due.string` and the Repeat pill add "until Nov 30". `DueEdit.until`: `None` keeps the end while the rule stays the same, `""` clears it; a typed repeat brings its own end (or none). `EditorValues.until` carries it through the editors.
- Reminders need nothing: they only look at the current due, and the task is checked once the repeat ends.
- Not done: the date picker's calendar still rings repeat days past the end (`calendar::month_view` would need the end passed through the Scheduler's props).

Out of scope: several days of the month ("every 1st and 15th"), "every!" (from completion), hours, "every quarter".
