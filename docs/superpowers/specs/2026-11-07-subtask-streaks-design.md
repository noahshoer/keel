# Streaks in sub-tasks (issue #11)

## Why

Noah's "Sharpen the Physical Saw" repeats every day and has three sub-tasks, each with its own "every day" and its own streak: 20 Push-ups, 1 Minute Plank, Stretch and/or Foam Roll. Today the parent and its sub-tasks have no link beyond "ticking the parent ticks them all". Two things are missing:

- In the parent's task view you can't see the sub-tasks' streaks.
- When the parent is just a container for its sub-tasks, finishing the last sub-task doesn't finish the parent. You have to tick the parent yourself, and if you forget, its streak resets.

## What already works and stays

- Ticking a parent completes all its sub-tasks, repeating ones included, and counts every streak (`TaskComplete` in `crates/core/src/commands.rs`).
- A task whose occurrence goes by unfinished drops to 0 (`streak::recompute_miss`), and that includes a parent whose sub-tasks were only partly done. So a parent's streak only goes up when it and all its sub-tasks got done. Nothing new is needed for this.
- A "Laundry" parent with a weekly streak and sub-tasks that have no streak needs nothing new either. The issue's "tie a sub-task's streak to the parent" setting is dropped (Noah, 2026-11-07).

## What changes

### 1. 🔥 count on sub-task rows (task view)

A sub-task row in the task view (`web/src/taskview/SubtaskRow.tsx`) shows the same badge a list row does when the sub-task tracks a streak: `<span class="streak">` with the fire icon and `streak_current`, right after the name (markup and style as in `web/src/lists/TaskRow.tsx`). It works at every level, since a sub-task's own view lists its sub-tasks the same way. Noah approved the look from a mockup in the real UI.

### 2. Setting: "Done with sub-tasks"

A per-task setting, off by default, so existing tasks behave as they do now.

**Where:** a checkable item in the task view's ••• menu (`web/src/taskview/MoreMenu.tsx`), under Streak and above Delete task, labelled **"Done with sub-tasks"** with a ✓ and the `.menu-item.checked` look when on (Noah chose this over a switch in the sub-task card and over a field row, 2026-11-07). Tapping it toggles the setting and closes the menu. Nothing else on the page shows the setting.

- *Builder's call:* the item shows only when the task has sub-tasks or the setting is already on (so it can be turned off after its sub-tasks are deleted).

**What it does:** when a sub-task is completed, and that makes all of its parent's counted sub-tasks done, and the parent has the setting on, the parent is completed too: the same `TaskComplete` as ticking it (its date moves on and its streak counts if it repeats; a one-off parent is checked). This repeats up the tree: completing the parent can in turn finish its own parent.

- "All counted sub-tasks done" means what the "x of y" count already says: `views::progress(parent)` has `done == total` (repeating sub-tasks count only under a repeating parent; `views::is_done` decides done).
- It works for one-off parents too (Noah, 2026-11-07), not only repeating ones.
- *Builder's call:* only completing a sub-task triggers it. Turning the setting on while every sub-task is already done, or deleting the last open sub-task, doesn't complete the parent.
- *My call:* a repeating parent only auto-completes when its current date is today or earlier. A weekly parent with daily sub-tasks would otherwise jump a week every day its sub-tasks are done. Ticking the parent by hand can still complete it early, as now.
- *My call:* a one-off parent doesn't auto-complete while it has a repeating sub-task that isn't done. Those aren't in its "x of y", and completing a one-off parent checks every sub-task for good, which would quietly end that sub-task's repeat.

**In the open task view:** when ticking a sub-task completes the task whose view is showing, the view closes just as if that task had been ticked (`nav.completedHere()`), (Noah, 2026-11-07). The frontend learns this from the command's result. *Builder's call:* how, e.g. an `auto_completed` list of ids on `ApplyResult` (`#[serde(default)]`).

### 3. Reopening a sub-task reopens its parent

When a sub-task is reopened (`TaskUncomplete`) and its parent is currently done, the parent is reopened too (`TaskUncomplete` on it, which also takes back its streak count when it repeats), and so on up the tree. This applies whether or not the parent has the setting on (Noah, 2026-11-07, "the way Todoist does").

- "Parent is currently done" is: it's checked, or it repeats, its date is after today, and it has an undoable completion (`streak::undoable_completion` is `Some`). This is `views::repeating_done` without its sub-task-only condition, so it applies to a top-level parent too. *My call:* the date condition stops a reopen today from undoing yesterday's completion of the parent.
- Reopening the parent does not reopen its other sub-tasks.

## Where the logic lives

- **keel-core** (data rules, Rust tests first): the new task field, the new action, the auto-complete and the reopen cascade inside `apply_at`, both in the same command (no new commands are dispatched by the frontend for the cascade).
  - New field on `Task`: `done_with_subtasks: bool`, `#[serde(default)]`.
  - New action variant `TaskSetDoneWithSubtasks { id, enabled }` (`"task_set_done_with_subtasks"`); existing variants untouched.
- **keel-storage:** a migration adding the column (default 0) and reading/writing it.
- **Frontend:** the badge, the menu item, `setDoneWithSubtasks` in `web/src/ipc/actions.ts`, the mock backend handling the new action (sets the field; it doesn't port the cascade), regenerated bindings.
- Every platform (desktop and phone).

## Non-goals

- A sub-task streak tied to the parent's streak.
- Any indicator of the setting outside the ••• menu.
- Changing what ticking a parent does to its sub-tasks.

## Verification

- Rust tests for: the cascade with the setting on and off; a repeating parent with repeating sub-tasks (the Sharpen case: tick two, nothing happens; tick the third, parent's date moves and streak +1); a one-off parent; two levels deep; reopening a sub-task reopens a done parent and takes back its streak count, and does nothing to a parent that isn't done; a parent with plain sub-tasks under a repeating parent.
- Vitest for the badge and the menu item.
- Look at it in the browser preview; then on the phone.
