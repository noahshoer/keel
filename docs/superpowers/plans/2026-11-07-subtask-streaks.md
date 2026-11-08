# Streaks in Sub-tasks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sub-task rows in the task view show their streak, a parent can be set to complete itself when its sub-tasks are done, and reopening a sub-task reopens a done parent.

**Architecture:** The data rules live in `keel-core`. There's a new `Task.done_with_subtasks` flag, a new `TaskSetDoneWithSubtasks` action, and the two cascades (auto-complete upward, reopen upward) run inside `apply_at`'s `TaskComplete` / `TaskUncomplete`, with an `auto_completed` list on `ApplyResult`. The frontend draws the badge, adds the ••• item, and closes the view when the task it shows gets auto-completed.

**Tech Stack:** Rust (keel-core, keel-storage, keel-service tests), ts-rs bindings, React + TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-07-subtask-streaks-design.md`

## Global Constraints

- Branch `issue-11-subtask-streaks`. TDD: write the failing test first, and commit after each task. Don't merge or push.
- Add new `Action` variants only, and never change existing serialized shapes. New fields are `#[serde(default)]`.
- Data rules stay in `keel-core`. TypeScript doesn't decide anything about a task.
- Menu copy is exactly **"Done with sub-tasks"**. The badge is the list row's `<span class="streak" title="Current streak">` with `FireIcon` and `streak_current`.
- The setting is off by default, and it applies on every platform.
- After changing a Rust type that's exported to TS, run `scripts/check-bindings.sh` and commit the regenerated `web/src/bindings`.

## Review Focus

1. **A plain sub-task under a repeating parent, completed with yesterday's parent and due to reopen today** (`reopens_on` ≤ today, but not yet reopened in the DB because no read happened). It must count as *open* when deciding whether today's parent is all done. Test: `stale_reopen_counts_as_open` (Task 2).
2. **Reopening a sub-task the day after the parent was auto-completed** must not undo yesterday's parent completion. Test: `reopening_today_leaves_yesterdays_parent_alone` (Task 3).
3. **Replaying the same command uuid** (idempotency) must not auto-complete twice or move the parent's date twice. Test: `replayed_complete_does_not_cascade_again` (Task 2).
4. **A parent that's already done** (checked, or completed early by hand for its next occurrence) must not be completed again when a sub-task finishes. Test: `already_done_parent_is_left_alone` (Task 2).
5. **Deleted sub-tasks** don't count toward "all done", so a parent whose only open sub-task was deleted is still all done once the rest are. That's covered by `views::progress` skipping deleted tasks; there's no new test (the spec says deleting doesn't trigger, only a completion does).

---

### Task 1: The flag, its action, storage and bindings

**Files:**
- Modify: `crates/core/src/model.rs` (`Task`, `Task::new`)
- Modify: `crates/core/src/commands.rs` (`Action`, `type_name`, `apply_at`, `ApplyResult`)
- Modify: `crates/storage/src/migrations.rs:126` (column loop), `crates/storage/src/sqlite_repo.rs` (row read, column list, upsert)
- Modify: `crates/core/src/task_view.rs` (`TaskDetail`)
- Test: `crates/storage/tests/streaks.rs`, plus `task_view.rs` unit tests
- Regenerate: `web/src/bindings/*`

**Interfaces:**
- Produces: `Task.done_with_subtasks: bool` (`#[serde(default)]`), `Action::TaskSetDoneWithSubtasks { id: Id, enabled: bool }` with type name `"task_set_done_with_subtasks"`, `ApplyResult.auto_completed: Vec<Id>` (`#[serde(default)]`), `TaskDetail.has_subtasks: bool` (true when the task has any non-deleted direct sub-task, done or not).

- [ ] **Step 1: Write failing tests**
  - `set_done_with_subtasks_round_trips` in `crates/storage/tests/streaks.rs`: add a task, apply `TaskSetDoneWithSubtasks { enabled: true }`, then `repo.get_task(..).done_with_subtasks == true`. Apply `false` and get `false`. A fresh task is `false`.
  - In `task_view.rs` tests: `has_subtasks` is true for a parent whose only sub-task is checked (with `show_completed` off), and false for a task with none.
- [ ] **Step 2: Run** `cargo test -p keel-storage --test streaks set_done_with_subtasks` and `cargo test -p keel-core task_view`. Expected: they fail to compile (no such field or variant).
- [ ] **Step 3: Implement.**
  - Add the field (default false in `Task::new`) and the variant with its `type_name` arm.
  - In `apply_at`, the arm sets the flag and `updated_at`.
  - Add `auto_completed` to `ApplyResult`.
  - Migration: add the `done_with_subtasks` column. It needs `INTEGER NOT NULL DEFAULT 0`, so give it its own `ALTER` beside the TEXT loop. Read and write it in `sqlite_repo.rs` the way `streak_enabled` is handled.
  - `has_subtasks` in `task_view::detail`.
  - Fix any `Task { .. }` / `TaskDetail { .. }` literals the compiler flags.
- [ ] **Step 4: Run** `cargo test` (needs `web/dist`), then `scripts/check-bindings.sh` and commit its output. Expected: all pass, and the bindings are regenerated with the new fields.
- [ ] **Step 5: Commit** with the message `feat: done_with_subtasks flag and its action (#11)`.

### Task 2: Completing the last sub-task completes the parent

**Files:**
- Modify: `crates/core/src/commands.rs` (`TaskComplete` arm moves into a helper; new cascade)
- Modify: `crates/core/src/views.rs` (new `done_for_now`)
- Create: `crates/service/tests/done_with_subtasks.rs` (copy the `Fx` fixture from `crates/service/tests/subtasks.rs`, adding `fn set_done_with(&mut self, id)` and a `complete` that returns the `ApplyResult`)

**Interfaces:**
- Consumes: Task 1's flag and `ApplyResult.auto_completed`.
- Produces: `pub fn done_for_now(t: &Task, today: NaiveDate) -> bool` in `views.rs`. It's true when `t.checked`, or when the task repeats, its due date is after `today`, and `streak::undoable_completion(t, today).is_some()`. It's like `repeating_done` but without the sub-task-only condition, and Task 3 uses it too. In `commands.rs`, private `fn complete_task<R: Repository>(repo: &R, id: &Id, today: NaiveDate, now: DateTime<Utc>) -> Result<bool, RepoError>` holds today's `TaskComplete` body. It returns false when nothing changed (missing task, or an already-credited occurrence).

- [ ] **Step 1: Write failing tests** in `done_with_subtasks.rs`, all on 2026-01-05 unless noted:
  - `sharpen_case`: a daily parent with streak and the flag, and three daily sub-tasks with streaks. Completing two leaves the parent's date at `2026-01-05` and streak 0, with `auto_completed` empty. Completing the third moves the parent's date to `2026-01-06` and its streak to 1, and the result's `auto_completed == [parent]`.
  - `flag_off_does_nothing`: the same setup without the flag. After all three, the parent's date is still `2026-01-05`.
  - `one_off_parent_is_checked`: a parent with no date, the flag, and two plain sub-tasks. After both, the parent is `checked`.
  - `two_levels`: grandparent (flag) → parent (flag) → one plain sub-task. Completing the sub-task checks both, and `auto_completed == [parent, grandparent]`.
  - `repeating_parent_waits_for_its_date`: a weekly parent (flag) due 2026-01-09 with one daily sub-task due today. Completing the sub-task leaves the parent's date at `2026-01-09`.
  - `one_off_parent_waits_for_open_repeating_sub_task`: a one-off parent (flag) with a plain sub-task and a daily sub-task. Completing the plain one leaves the parent unchecked.
  - `already_done_parent_is_left_alone`: a daily parent (flag) is completed by hand, so its date is `2026-01-06`. Then a daily sub-task due today is completed. The parent's date stays `2026-01-06` and its streak 1.
  - `stale_reopen_counts_as_open`: a daily parent (flag) with plain sub-tasks A and B. Complete the parent on 01-05 (A and B get `reopens_on` 01-06). On 01-06, without reading tasks first, complete A with `dispatch_at(.., 01-06)`. The parent's date stays `2026-01-06` (B is open again today).
  - `replayed_complete_does_not_cascade_again`: in the Sharpen setup, send the third sub-task's completion twice with the same uuid. The parent's date is `2026-01-06` and its streak 1.
- [ ] **Step 2: Run** `cargo test -p keel-service --test done_with_subtasks`. Expected: the cascade tests fail (the parent never completes).
- [ ] **Step 3: Implement.** Move the `TaskComplete` body into `complete_task`. The arm calls it, then, when it returned true, runs the cascade:

```text
child = the completed task
loop:
  P = child's parent, else stop
  all = repo.list_all_tasks(), each with reopen_if_due(today) applied in memory
  stop unless P.done_with_subtasks
          and !done_for_now(P, today)
          and progress(&P.id, &all, today) is Some(p) with p.done == p.total
          and (P repeats  → its due date <= today)
          and (P one-off  → no direct sub-task that repeats and !is_done(sub, today))
  complete_task(P); push P.id onto result.auto_completed; child = P
```

  The replay test passes because `apply_at` already returns early for a recorded uuid. Check that it does. If it doesn't, the cascade must not run on a replay.
- [ ] **Step 4: Run** `cargo test`. Expected: everything passes, including the existing `subtasks.rs` and `streaks.rs`.
- [ ] **Step 5: Commit** with the message `feat: finishing the last sub-task completes a "done with sub-tasks" parent (#11)`.

### Task 3: Reopening a sub-task reopens its done parent

**Files:**
- Modify: `crates/core/src/commands.rs` (`TaskUncomplete` arm moves into `fn uncomplete_task<R: Repository>(repo: &R, id: &Id, today: NaiveDate) -> Result<(), RepoError>`, plus the upward cascade)
- Test: `crates/service/tests/done_with_subtasks.rs`

**Interfaces:**
- Consumes: `views::done_for_now` from Task 2.

- [ ] **Step 1: Write failing tests:**
  - `reopening_a_sub_task_reopens_the_parent`: the Sharpen case completed by its third sub-task. Reopening Push-ups (`TaskUncomplete`) puts the parent's date back to `2026-01-05` and its streak to 0. Planks and Stretch stay done (`is_done`).
  - `reopening_reopens_a_parent_without_the_flag`: a one-off parent without the flag, completed by hand (all sub-tasks checked). Reopening one sub-task unchecks the parent, and the other sub-task stays checked.
  - `reopening_today_leaves_yesterdays_parent_alone`: the Sharpen case completed on 01-05. On 01-06, complete Push-ups, then reopen it, both with `dispatch_at(.., 01-06)`. The parent's date is still `2026-01-06` and its streak 1.
  - `reopening_climbs_two_levels`: the `two_levels` setup after the cascade. Reopening the sub-task unchecks both the parent and the grandparent.
- [ ] **Step 2: Run** `cargo test -p keel-service --test done_with_subtasks reopen`. Expected: they fail (the parent stays done).
- [ ] **Step 3: Implement.** Move the body into `uncomplete_task`. After it, walk up: while the task has a parent `P` with `done_for_now(P, today)`, call `uncomplete_task(P)` and continue from `P`. It doesn't depend on the flag.
- [ ] **Step 4: Run** `cargo test`. Expected: everything passes.
- [ ] **Step 5: Commit** with the message `feat: reopening a sub-task reopens its done parent (#11)`.

### Task 4: Sub-task rows show their streak, and the view closes on auto-complete

**Files:**
- Modify: `web/src/taskview/SubtaskRow.tsx`
- Test: `web/src/taskview/SubtaskRow.test.tsx`

**Interfaces:**
- Consumes: `ApplyResult.auto_completed` (bindings from Task 1). `nav.completedHere()` from `useTaskNav()`.

- [ ] **Step 1: Write failing tests:**
  - `a sub-task with a streak shows its count`: `task({ streak_enabled: true, streak_current: 6 })` renders `.tv-sub .streak` with text `6`, after `.task-content`.
  - `no badge without a streak`: the default task has no `.streak`.
  - `completing the last sub-task closes the parent's view`: mock `invoke` to resolve `{ id_mapping: {}, auto_completed: ["p"] }` for a task with `parent_id: "p"`. Spy on `completedHere` (render inside a provider whose value is stubbed, or assert the nav stack popped, whichever `nav.test.tsx` already does). After the click resolves, `completedHere` is called once. With `auto_completed: []`, it isn't called.
- [ ] **Step 2: Run** `cd web && pnpm vitest run src/taskview/SubtaskRow.test.tsx`. Expected: the new tests fail.
- [ ] **Step 3: Implement.**
  - Add the badge markup copied from `TaskRow.tsx` (`FireIcon` from `../shell/icons`).
  - On complete, if the result's `auto_completed` includes `t.parent_id`, call `nav.completedHere()` before `onChanged`.
- [ ] **Step 4: Run** `cd web && pnpm check`. Expected: it passes.
- [ ] **Step 5: Commit** with the message `feat: sub-task rows show their streak; the view closes when its task completes itself (#11)`.

### Task 5: "Done with sub-tasks" in the ••• menu

**Files:**
- Modify: `web/src/taskview/MoreMenu.tsx`, `web/src/taskview/TaskView.tsx` (around line 390), `web/src/ipc/actions.ts`, `web/src/ipc/mock.ts`
- Test: `web/src/taskview/taskview.test.tsx` (it already opens `.tv-more` at about line 465)

**Interfaces:**
- Consumes: `TaskDetail.has_subtasks`, `Task.done_with_subtasks`, `Action` `task_set_done_with_subtasks`.
- Produces: `setDoneWithSubtasks(id: Id, enabled: boolean): Promise<ApplyResult>` in `actions.ts`. `MoreMenu` gains the props `doneWith: boolean | null` (`null` hides the item) and `onDoneWith: () => void`.

- [ ] **Step 1: Write failing tests** in `taskview.test.tsx`:
  - For a task with sub-tasks, the ••• menu has a `.menu-item` whose label is "Done with sub-tasks", between Streak and Delete task.
  - Clicking it sends `{ type: "task_set_done_with_subtasks", id, enabled: true }` and closes the menu.
  - With `done_with_subtasks: true`, the item has `.checked` and a check mark, and clicking it sends `enabled: false`.
  - For a task without sub-tasks and with the flag off, there's no item.
- [ ] **Step 2: Run** `cd web && pnpm vitest run src/taskview/taskview.test.tsx`. Expected: the new tests fail.
- [ ] **Step 3: Implement.**
  - `TaskView` passes `doneWith = detail.has_subtasks || task.done_with_subtasks ? task.done_with_subtasks : null`.
  - Clicking closes the menu and then calls `settle(setDoneWithSubtasks(id, !on)).then(onChanged)`.
  - For the item, use `.menu-item checked` plus a `.menu-check` holding `CheckIcon` when it's on. That's the same look as other checked menu rows, e.g. the priority menu.
  - The mock's `task_set_done_with_subtasks` case sets the field and returns null. Its dispatch result needs `auto_completed: []`.
- [ ] **Step 4: Run** `cd web && pnpm check`. Then open the browser preview (`web` launch config), open a task with sub-tasks, open •••, toggle the item and screenshot it. Expected: the checks pass and the item shows checked after the toggle.
- [ ] **Step 5: Commit** with the message `feat: "Done with sub-tasks" in the task view's ••• menu (#11)`.

### Task 6: Docs

- [ ] Update `CLAUDE.md`'s task view paragraph:
  - the 🔥 count on sub-task rows;
  - "Done with sub-tasks" (`Task.done_with_subtasks`, the cascade in `TaskComplete`, `ApplyResult.auto_completed`);
  - reopening a sub-task reopens a done parent.
- [ ] Update `.claude/issue-roadmap.md` (#11's status).
- [ ] Run `cargo test` and `cd web && pnpm check` one last time. Expected: all pass.
- [ ] Commit with the message `docs: streaks in sub-tasks (#11)`.
