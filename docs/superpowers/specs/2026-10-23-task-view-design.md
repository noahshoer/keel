# Task view

**Status:** approved by Noah, 2026-10-23. Issue [#4](https://github.com/noahshoer/keel/issues/4).

## Why

On the iPhone you can't see a task's sub-tasks. Tapping a task opens the edit card, which has no room for them, and the long-press menu only offers "Add sub-task" and "Delete". Noah's typical case is packing: "Pack for Boston" has sub-tasks for groups of things (Toiletries, Clothes, Chargers), and he wants to check them off one at a time to keep track. At other times he just completes the top-level task and wants that to complete everything under it (this already works in core).

## Goals

1. **Tapping an existing task opens its task view**, not the edit card. Every task opens the same view; the sub-task card only appears when the task has sub-tasks.
2. **Where it opens:**
   - **Phone:** a sheet that slides up over the list (like the date sheet), with the list visible behind it. Swipe down to close. The header shows where the task lives ("Inbox", the project) and a Done button.
   - **Desktop and web:** a panel on the right; the list stays usable beside it. A narrow window gets the phone layout.
3. **Navigation between tasks:** each view shows only its *direct* sub-tasks. Tapping a sub-task opens its own view inside the same sheet or panel, with Back to the parent. Nesting depth stays unlimited (the issue's "one level" limit is dropped).
4. **Title line:** the task's completion circle (priority-colored, as in the list) and its title. Completing the task from this circle closes its view (Noah's choice): back to the parent's view, or closing the sheet or panel at the top level. A repeating task also closes; it has moved to its next date.
5. **Fields card**, in a fixed order: description, date (with time and repeat), deadline, priority, labels, reminders, location (only where the add-task card offers it).
   - A field that has a value shows as a row, in its fixed place.
   - Fields without a value show as **dotted-outline pills** ("+ Deadline", "+ Labels", …) in **one row that scrolls sideways**, like the add-task card's chip row. Noah chose dotted pills over glass chips: they read better as "missing". Setting a value moves it out of the pill row and into its row in the fixed order.
   - On desktop, where there's room, the pills may wrap instead of scrolling (my call).
6. **Editing:**
   - Title and description are edited in place in the view.
   - Tapping a field row or a pill opens the same picker the add-task card uses for it: native sheets on the iPhone, the existing popovers on desktop and web.
   - Every change saves immediately. There is no Save button.
   - No quick-add parsing when editing the title in place, the same as the current editor.
   - The edit card is then only used for creating tasks.
7. **Sub-task card:**
   - Header: "Sub-tasks" on the left; on the right "*x* of *y* · Show completed" (or "Hide completed"). When every sub-task is done, a round green check (the same look as a completed task's circle) follows the toggle: "5 of 5 · Show completed ✓".
   - The header must never overlap: on a narrow screen the right-hand group wraps to its own line.
   - A thin progress bar under the header.
   - A row per direct sub-task: its completion circle, its name, and, if it has its own sub-tasks, "*x*/*y*" for those; when they're all done the count stays and the round green check follows it ("4/4 ✓"). Every row has a ›, since every task opens its view (Noah: "all tasks should pull up the same view"; the v5 mockup leaves it off "Chargers" by mistake).
   - Tapping the circle completes the sub-task and you stay in the view (my call). Tapping the circle of a completed sub-task reopens it (`TaskUncomplete`, my call). Tapping the rest of the row opens its view.
   - **What counts as done (my call):** a non-repeating sub-task is done when it's checked. A sub-task that repeats on its own is never `checked` (completing it moves its date), so it's left out of every count, the progress bar and the green check. One helper in core computes the counts for the list, the header and the rows.
   - With "Show completed" off, completed sub-tasks are hidden; with it on, they show greyed and struck through, like the main list.
   - Finishing every sub-task does **not** complete the parent. The green check only says it's ready; you complete the parent yourself.
   - "+ Add sub-task" at the bottom opens the add-task card already aimed at this task as parent, with full quick add. It stays open after each save, so a list can be typed in quickly.
8. **Main list:** sub-tasks no longer show in the list (Noah's choice). Only top-level tasks are rows; a task with sub-tasks shows "*x*/*y*" next to its name; when all are done the count stays and the round green check follows it ("5/5 ✓", Noah: the count always shows next to the check). Tapping the row opens the view, which is the way to see sub-tasks.
   - This applies to every list view: projects, Today, Upcoming, labels and filters. The indented sub-task rows, the collapse arrow (`is_collapsed`, `api::set_collapsed`) and the list's inline "Add sub-task" editor go away. `is_collapsed` stays in the data model and is simply no longer used.
   - Search results still list matching sub-tasks, each opening its own view (my call: search is for finding a task wherever it is).
   - The completed-tasks view still lists completed sub-tasks (my call), each opening its view.
   - Reminders and notifications for sub-tasks keep working as today.
   - **Every list shows its items as cards** (Noah's choice, mockup I in `dated-subtasks-v3.html`). Each top-level task is its own card. Cards are separated by space, with no dividing line between two cards.
   - **A sub-task due on a different day from its parent** (e.g. *Chargers* due today, *Pack for Boston* due Friday) shows in date-based views (Today, Upcoming, labels, filters) inside its parent's card:
     - The card's header is the parent, for context only: its priority flag, name, its own date, the "*x*/*y*" count and a ›. It has no circle, so it can't be completed from here. Tapping the header opens the parent's view.
     - Under the header, each sub-task that matches the view is a normal row with its own circle. Tapping the row opens the sub-task's view (Back goes to the parent).
     - Inside a card, a dividing line separates the header and each sub-task row.
     - Several matching sub-tasks of one parent share one card.
   - When the parent itself also matches the view, it's a normal completable row as the card's top, with its matching sub-tasks under it in the same card (my call).
   - Project views show only top-level tasks, so their cards hold one task each.
9. **Repeating tasks:** when a repeating parent is completed and moves to its next date, its sub-tasks come back unchecked, so a weekly "Pack gym bag" checklist starts fresh each time.
   - Today, completing a repeating parent leaves its sub-tasks as they are, so ticked ones stay ticked (`Action::TaskComplete`, recurring branch in `crates/core/src/commands.rs`).
   - The reset covers every non-repeating descendant, not just direct sub-tasks (my call; it mirrors how completing a one-off parent completes every descendant).
   - A sub-task that repeats on its own keeps its own schedule and isn't reset by the parent (my call, matching how completing a one-off parent already skips repeating children).
   - It's part of `TaskComplete` itself. Completions already in the log simply adopt the new behavior if replayed (Noah).
10. **Header buttons (my call):** the right side has a close button (Done on the phone, which only closes the sheet and never completes anything; ✕ on desktop) and a ••• menu with Delete, plus Streak for a repeating task. Deleting a task deletes all of its sub-tasks too (Noah; core already does this). Back appears on the left when you're inside a sub-task. The long-press menu and the desktop hover buttons in the list stay as they are.

Mockups (git-ignored; open on the Mac): `.superpowers/brainstorm/mockups/task-view-layout.html` (sheet vs full page, desktop panel) and `task-view-layout-v5.html` (the chosen phone layout: dotted pills; partial and all-done sub-task cards); `dated-subtasks-v3.html` (list cards, option I chosen); `parent-due-today.html` (parent due the same day; all sub-tasks done shows "5/5 ✓").

## Non-goals

- More than one level of sub-tasks in a single view.
- Reordering sub-tasks by drag inside the view (can come later).
- Comments, attachments, activity: still v1 non-goals.
- Changing how completing a one-off parent works (it already completes its non-repeating sub-tasks).

## Anti-goals (it fails even if it works if…)

- **The phone and desktop behave differently.** Every feature ships on every platform; the iPhone only differs in display tweaks and Liquid Glass. Layout and behavior are decided in Rust (the Dioxus UI and core); the iOS side only draws glass where it applies.
- **The view saves through a different path than the editor.** Changes go through the existing `TaskUpdate` / reminder / recurrence code, the same command log and the same idempotency.
- **Counts disagree** between the main list, the view header and a sub-task's row.
- **Text overlaps** anywhere in the header or rows at iPhone width.
- **The sheet stutters while scrolling** (the date sheet's `LazyVGrid`-in-`List` problem; measure if the sheet is native).

## Constraints

- Stack: Tauri v2, Dioxus 0.6 web UI (wasm), Rust core/service, SwiftUI glass plugin (`crates/plugin-glass`) on iOS 26+.
- Core rules: new `Action` variants only, never changed serialized shapes; new fields `#[serde(default)]`.
- Commands that call a mobile plugin must be `async` (CLAUDE.md).
- The repo is public.
- Work style (CLAUDE.md): a plan in `docs/superpowers/plans/`, a feature branch, TDD for Rust logic, a commit per task, merge `--no-ff`, push. Check on the iOS simulator (Demo driver) and on the phone, and on desktop.

## Left to the builder

- Whether the phone sheet is a web sheet styled in Dioxus with glass buttons from the plugin, or a native SwiftUI sheet driven by Dioxus (like the add-task card). Either way the content and behavior come from Rust.
- How the view's "back" stack is held, and what happens to an open view when its task is deleted or completed elsewhere.
- Where the counts are computed (a core helper over the task list is the obvious place, so all three places agree).
- The exact desktop panel width and the narrow-window breakpoint.
