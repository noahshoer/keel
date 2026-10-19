# Todoist-Faithful UI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Replace the minimal test screen with a UI that mirrors Todoist's layout and feel, backed by real Today / Upcoming / project views.

**Architecture:** Pure date-bucketing/labeling lives in `core::views` (host-tested). Storage gains a cross-project query; the service exposes `today_view`, `upcoming_view`, `project_view` (each applies lazy streak-miss recompute and hides completed tasks), surfaced as Tauri commands. The Dioxus UI is rewritten into a sidebar + main pane with Todoist's visual language, CSS classes in one stylesheet, and an inline add-task editor card.

**Tech Stack:** Rust, chrono, Dioxus 0.6 (wasm), Tauri v2.

**Spec:** `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md` (§4 "UI mimics Todoist initially")

## Global Constraints
- Mirror Todoist's **layout, spacing, colors and interaction patterns**; draw our own icons (simple inline SVG) — **no Todoist logos, fonts, or copied assets**.
- Palette: accent `#dc4c3e`; sidebar `#fcfaf8`; text `#202020`, secondary `#808080`; dividers `#f0f0f0`; priorities P1 `#d1453b`, P2 `#eb8909`, P3 `#246fe0`, none `#b8b8b8`; due chips — overdue `#d1453b`, today `#058527`, tomorrow `#ad6200`, this week `#692fc2`, later `#808080`.
- Completed (checked) tasks are hidden from all views. Recurring tasks never become checked (they reschedule).
- Today in the wasm UI comes from JS `Date` (no `SystemTime` on wasm32).
- IPC snake_case; existing commands keep working.

## Review Focus
- **Overdue vs today vs tomorrow at month/year boundaries** (Dec 31 → Jan 1) — pinned in Task 1.
- **Undated tasks** must not appear in Today/Upcoming, and must still appear in their project — pinned in Task 3.
- **Completed non-recurring tasks** disappear from every view — pinned in Task 3.
- **Today view spans all projects** (not just Inbox) — pinned in Task 3.
- **Upcoming window** includes today and excludes the day after the window — pinned in Task 3.

---

## Task 1: Due buckets & labels (`core::views`)
**Files:** Create `crates/core/src/views.rs`; modify `crates/core/src/lib.rs`.
**Produces:** `enum DueBucket { Overdue, Today, Tomorrow, ThisWeek, Later }`; `fn due_bucket(due: NaiveDate, today: NaiveDate) -> DueBucket` (ThisWeek = 2–6 days ahead); `fn due_label(due: NaiveDate, today: NaiveDate) -> String` ("Today", "Tomorrow", "Yesterday", weekday name for ThisWeek e.g. "Friday", else "20 Oct" / "20 Oct 2027" when another year); `fn due_date_of(task: &Task) -> Option<NaiveDate>`.
- [ ] Tests: `buckets_around_today`, `buckets_cross_year_boundary` (Dec 31 → Jan 1 is Tomorrow), `labels` (Today/Tomorrow/Yesterday/weekday/"20 Oct"/other-year), `due_date_of_reads_date_and_datetime`. RED → implement → GREEN → commit.

## Task 2: Cross-project task query (storage)
**Files:** `crates/core/src/repository.rs`, `crates/storage/src/sqlite_repo.rs`, test `crates/storage/tests/repo.rs`.
**Produces:** `Repository::list_all_tasks() -> Result<Vec<Task>, RepoError>` (non-deleted, all projects).
- [ ] Test `list_all_tasks_spans_projects_and_excludes_deleted`. RED → implement → GREEN → commit.

## Task 3: Views in the service + IPC
**Files:** `crates/service/src/lib.rs`, test `crates/service/tests/views.rs`, `crates/src-tauri/src/lib.rs`.
**Produces:** `AppService::{today_view_at(today), upcoming_view_at(today, days), project_view_at(project_id, today)}` + non-`_at` variants. Today = unchecked tasks with due ≤ today, sorted (due asc, priority desc). Upcoming = unchecked with today ≤ due < today+days, sorted. Project = unchecked tasks of that project (child_order). All apply streak-miss recompute (shared helper). Tauri commands `today_view`, `upcoming_view(days)`, `project_view(project_id)`.
- [ ] Tests: `today_spans_projects_and_includes_overdue`, `undated_only_in_project_view`, `completed_hidden_everywhere`, `upcoming_window_bounds`. RED → implement → GREEN → commit (backend compiles).

## Task 4: Todoist-style UI rewrite
**Files:** `crates/ui/src/{main.rs, api.rs, style.rs, icons.rs, sidebar.rs, task_row.rs, add_task.rs}`.
- Sidebar: Inbox / Today (with count) / Upcoming; "My Projects" with inline "+ add project"; active item highlighted.
- Main: header (title; Today shows date), Today splits **Overdue** / **Today** sections; Upcoming groups by date with day headers; project view lists its tasks.
- Task row: round priority-colored checkbox (✓ on hover), title, grey description, meta row (due chip colored by bucket, ↻ for recurring, 🔥 n when tracking), project name on the right in Today/Upcoming; hover actions (streak, delete). Clicking a recurring task's 🔥/flame opens an inline streak panel (current / best / goal, track toggle, goal input).
- Inline "+ Add task" → editor card: name, description, Date (date input), Priority (P1–P4), Repeat (none/daily/weekdays/monthly), Cancel / **Add task** (accent, disabled when empty). Defaults: Today view → due today; project view → that project; else Inbox.
- [ ] `dx build --platform web` compiles; stage `dist/`; commit.

## Task 5: Housekeeping
- [ ] Remove the duplicate `<title>` in `crates/ui/index.html` (fixes "TodoTodo"); ignore `crates/src-tauri/gen/` and untrack it. Commit.
