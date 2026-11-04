# Sub-task fixes (2026-11-03)

## 1. Completing a parent completes every sub-task

Plain sub-tasks already complete with their parent. Repeating ones were skipped ("keep their own schedule"), which is what Noah's daily parent with two daily streak sub-tasks hit.

- Parent that doesn't repeat: every sub-task is checked, repeating ones too (no streak credit; they're finished with the parent).
- Repeating parent: plain sub-tasks uncheck for the new occurrence, as before. A repeating sub-task due on or before the occurrence being completed completes one occurrence of its own: its date moves on and its streak is credited. One already ahead of that occurrence is left alone, so a sub-task done earlier is never credited twice and a weekly sub-task isn't pulled forward by a daily parent.
- The counter counts a repeating parent's repeating sub-tasks, done when their date is past the parent's. Otherwise such a parent would have no counter once its sub-tasks are hidden.

## 2. Sub-tasks in lists are a setting

Settings → Appearance → "Show sub-tasks in lists", off by default, per device (`localStorage`, like the completion mark).

- Off, as in Keel UI Directions: one row per top-level task with its "2 of 5" counter, in every list. A matching sub-task under a parent that doesn't match shows the parent's context head (no circle), so due work never disappears. New `CardMode::Heads`.
- On: sub-tasks sit under their parent in every list, projects too (`CardMode::Grouped`).
- Sidebar counts follow: off counts rows, on counts every open task. `sidebar_counts` takes `show_subtasks`.
- Search always lists sub-tasks it finds, and its rows gain the counter (the mockup's Search row has it).
