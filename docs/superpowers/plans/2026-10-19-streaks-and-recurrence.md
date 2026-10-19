# Streaks + Minimal Recurrence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recurring tasks that reschedule themselves on completion, with opt-in per-task streaks (current + best record, optional goal, hard reset on a miss).

**Architecture:** A date-based recurrence engine in `core` defines each rule's occurrences (`next_after`/`prev_before`/`contains`). A streak engine in `core` updates per-task counters from occurrence dates (counter-based, not a history scan). The command layer makes completing a recurring task advance its due and credit the streak; the service recomputes misses lazily on read. The Dioxus UI adds a recurrence picker and streak controls.

**Tech Stack:** Rust, `chrono` (`NaiveDate`, `Weekday`), existing `core`/`storage`/`service`/`ui` crates.

**Spec:** `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md` (§7 streaks, §8 recurrence)

## Global Constraints

- Builds on the merged foundation (`todo-core`, `todo-storage`, `todo-service`, `src-tauri`, `ui`).
- Recurrence in this plan is limited to **Daily**, **Weekly(weekdays)**, **MonthlyDay(day)**. `EveryNDays` and the natural-language parser are **deferred** (period math here is anchor-free).
- Streaks are **counter-based**: per-task `streak_current`, `streak_best`, `streak_last_period` (the last completed occurrence date, ISO `YYYY-MM-DD` string). A completion history (`streak_event`) may be logged for audit but is **not** the computation source. *(Simplification vs design §7's "history is source of truth" — confirm at review.)*
- **Miss = hard reset** to 0; **goals opt-in**; **opt-in per task**. Day boundary = local date (`today` is injected into engine functions, never read from the clock inside pure logic).
- IPC stays snake_case end-to-end; IDs ULID.

## Review Focus

- **MonthlyDay clamping:** a rule on day 31 (or 30) in a short month must resolve to that month's last valid day, not roll over — pinned in Task 1.
- **Double-completion same period:** completing a recurring task twice on the same occurrence date must not increment the streak twice — pinned in Task 2.
- **Miss exactly at the boundary:** on the day an occurrence is due (`today == next occurrence`), the streak is intact; only once that occurrence is strictly in the past uncompleted is it a miss — pinned in Task 2.
- **Weekly with multiple weekdays:** each scheduled weekday is its own occurrence/period (Mon done then Wed is consecutive, not a gap) — pinned in Task 1 + Task 2.
- **Completing a non-recurring task** still just checks it (no streak, no reschedule) — pinned in Task 3.

---

## Task 1: Recurrence model & engine

**Files:**
- Create: `crates/core/src/recurrence.rs`
- Modify: `crates/core/src/model.rs` (replace `Due.rrule: Option<String>` with `recurrence: Option<Recurrence>`), `crates/core/src/lib.rs`
- Test: `crates/core/src/recurrence.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `enum Recurrence { Daily, Weekly(Vec<u8>) /* 0=Mon..6=Sun */, MonthlyDay(u8) }`, serde-derived.
  - `Recurrence::contains(&self, date: NaiveDate) -> bool`
  - `Recurrence::next_after(&self, date: NaiveDate) -> NaiveDate` (strictly after)
  - `Recurrence::prev_before(&self, date: NaiveDate) -> NaiveDate` (strictly before)
  - `Due.recurrence: Option<Recurrence>`; `Due.is_recurring` stays and must equal `recurrence.is_some()`.

- [ ] **Step 1: Write failing tests** in `recurrence.rs`:
  - `daily_next_and_prev` — `Daily.next_after(d) == d+1`, `prev_before(d) == d-1`, `contains` always true.
  - `weekly_multi_weekday_occurrences` — `Weekly([Mon,Wed,Fri])`: `next_after(Mon) == Wed`, `next_after(Fri) == next Mon`, `contains(Tue) == false`.
  - `monthly_day_clamps_to_month_end` — `MonthlyDay(31).next_after(Jan 31) == Feb 28` (2026, non-leap); `MonthlyDay(15).next_after(Jan 20) == Feb 15`.
- [ ] **Step 2: Run** `cargo test -p todo-core recurrence` → FAIL (types missing).
- [ ] **Step 3: Implement** `Recurrence` + the three methods; update `Due` (replace `rrule` with `recurrence`, fix the two existing `Due { .. }` literals in `model.rs` tests and `storage/tests/repo.rs`).
- [ ] **Step 4: Run** `cargo test -p todo-core recurrence` and `cargo test` → PASS (existing suite still green after the `Due` change).
- [ ] **Step 5: Commit** — `feat(core): recurrence model and occurrence engine`.

## Task 2: Streak engine

**Files:**
- Create: `crates/core/src/streak.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/src/streak.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: `Recurrence`, `Task`.
- Produces (operate on a streak-enabled recurring `Task`, `today: NaiveDate` injected):
  - `fn credit_completion(task: &mut Task, today: NaiveDate)` — updates `streak_current`/`streak_best`/`streak_last_period` using `prev_before`.
  - `fn recompute_miss(task: &mut Task, today: NaiveDate)` — hard-resets `streak_current` to 0 when the next occurrence after `streak_last_period` is strictly before `today`.

- [ ] **Step 1: Write failing tests** in `streak.rs` (build a `Daily`-recurring, `streak_enabled` Task):
  - `first_completion_sets_current_1`.
  - `consecutive_completions_increment` — credit on d, then d+1 → current 2; `streak_best` tracks max.
  - `same_period_completion_no_double_count` — crediting twice on the same `today` leaves current unchanged.
  - `gap_completion_restarts_at_1` — credit on d, then d+3 → current 1.
  - `miss_resets_to_zero` — last=d, `recompute_miss(task, d+2)` → current 0; `recompute_miss(task, d+1)` (the due day) → unchanged.
  - `weekly_consecutive_across_weekdays` — `Weekly([Mon,Wed])`: credit Mon then Wed → current 2.
- [ ] **Step 2: Run** `cargo test -p todo-core streak` → FAIL.
- [ ] **Step 3: Implement** both functions per the counter logic (consecutive if `last == prev_before(today)`, else gap→1; miss if `next_after(last) < today`).
- [ ] **Step 4: Run** `cargo test -p todo-core streak` → PASS.
- [ ] **Step 5: Commit** — `feat(core): counter-based streak engine`.

## Task 3: Recurring completion + streak commands

**Files:**
- Modify: `crates/core/src/commands.rs` (TaskComplete branch; new `TaskSetStreak`; `recurrence` arg on `TaskAdd`)
- Test: `crates/storage/tests/streaks.rs`

**Interfaces:**
- Consumes: `recurrence`, `streak` engines, `Repository`.
- Produces:
  - `Action::TaskAdd` gains `recurrence: Option<Recurrence>` (sets `due.recurrence`/`is_recurring`).
  - `Action::TaskSetStreak { id: Id, enabled: bool, goal: Option<i64> }`.
  - `TaskComplete` behavior: **recurring** task → advance `due.date` to `recurrence.next_after(due.date)`, and if `streak_enabled` call `credit_completion`; leave `checked=false`. **Non-recurring** → unchanged (`checked=true`, `completed_at=now`).
  - `apply` takes an injected `today` for deterministic tests: add `pub fn apply_at<R: Repository>(repo, cmd, today: NaiveDate)` and make `apply` call it with `Local::today`. *(Interface note for Task 4.)*

- [ ] **Step 1: Write failing tests** in `crates/storage/tests/streaks.rs`:
  - `complete_recurring_advances_due_and_increments_streak` — add Daily+streak task due d; `apply_at(TaskComplete, d)` → due==d+1, streak_current==1, still not checked.
  - `complete_recurring_twice_same_day_no_double` — two completes at same `today` → current 1 (uuid differs; the same-period guard holds).
  - `complete_nonrecurring_checks_it` — non-recurring task → checked true, streak_current 0.
  - `set_streak_enables_and_sets_goal`.
- [ ] **Step 2: Run** `cargo test -p todo-storage --test streaks` → FAIL.
- [ ] **Step 3: Implement** the `TaskComplete` branch, `TaskSetStreak`, `TaskAdd.recurrence`, and `apply_at`/`apply`.
- [ ] **Step 4: Run** `cargo test -p todo-storage --test streaks` and `cargo test` → PASS.
- [ ] **Step 5: Commit** — `feat(core): recurring completion reschedules + credits streak`.

## Task 4: Service — lazy miss recompute on read

**Files:**
- Modify: `crates/service/src/lib.rs`
- Test: `crates/service/tests/streaks.rs`

**Interfaces:**
- Produces: `AppService::list_tasks` recomputes misses for streak-enabled recurring tasks (via `streak::recompute_miss` with today = local date), **persisting** any reset, before returning. Add `AppService::dispatch_at(cmd, today)` for deterministic tests; `dispatch` uses the real date.

- [ ] **Step 1: Write failing tests** in `crates/service/tests/streaks.rs`:
  - `completion_then_list_shows_streak` — add Daily+streak task, `dispatch_at(complete, d)`, `list_tasks` → streak_current 1.
  - `list_resets_streak_after_missed_period` — complete at d, then `list_tasks_at(d+2)` → streak_current 0 and the reset is persisted (a second call still 0).
  (Add a test-only `list_tasks_at(project, today)` seam mirroring `list_tasks`.)
- [ ] **Step 2: Run** `cargo test -p todo-service --test streaks` → FAIL.
- [ ] **Step 3: Implement** the recompute-on-read + the injectable-date seams.
- [ ] **Step 4: Run** `cargo test -p todo-service` and `cargo test` → PASS.
- [ ] **Step 5: Commit** — `feat(service): lazy streak-miss recompute on list`.

## Task 5: UI — recurrence picker + streak controls

**Files:**
- Modify: `crates/ui/src/api.rs` (wrappers: `set_streak`; `add_task` gains optional recurrence), `crates/ui/src/main.rs` (a simple recurrence select on quick-add: None/Daily/Weekdays; per-task streak toggle + goal input; show `🔥 current (best N)` on streak tasks)
- Verify: `dx build --platform web` compiles; logic is covered in core.

- [ ] **Step 1:** Add `api::set_streak(id, enabled, goal)` and extend `api::add_task` to pass `recurrence`.
- [ ] **Step 2:** Add the recurrence `select` to quick-add and a streak toggle + goal + flame badge to `TaskRow`.
- [ ] **Step 3: Build** `cd crates/ui && dx build --platform web` → compiles; stage to `dist/`.
- [ ] **Step 4: Manual verification** (desktop): add a Daily streak task, complete it → it reschedules and the flame shows 1; relaunch persists. Note result.
- [ ] **Step 5: Commit** — `feat(ui): recurrence picker and streak controls`.

---

## Done when
- `cargo test` passes across core/storage/service.
- Completing a recurring task reschedules it and advances its streak; a missed period resets the streak to 0 on next list.
- The desktop app shows and updates streaks.

## Notes / my calls
- Counter-based streaks (not history-scan) — see Global Constraints.
- `apply_at` / `dispatch_at` / `list_tasks_at` date-injection seams exist so streak logic is deterministic in tests; the non-`_at` variants use the local date.
- `streak_event` logging is out of scope here (counters suffice for v1); add later if an audit/history view is wanted.
