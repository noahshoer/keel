# Deadlines & Per-task Reminders — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Two remaining v1 items (spec §2, §9). **Deadlines**: a "must be done by" date separate from the due date (Todoist's deadline). **Per-task reminders**: explicit reminders on a task, relative ("30 min before") or absolute ("Tue 9 AM"), on top of the global rule.

**Architecture:**
- **Deadline** is the existing `Task.deadline` (`YYYY-MM-DD`, already stored). `TaskAdd.deadline`; `TaskUpdate.deadline` (`Some("")` clears). Filter terms `deadline: D`, `deadline before: D`, `deadline after: D`, `no deadline`. Quick add: `{oct 30}` / `{friday}` (Todoist's brace syntax) → deadline, highlighted. Rows show a deadline chip (red when today/past).
- **Reminders** use the existing `reminder` table/model: `Relative { minute_offset }` (before the due time; date-only tasks use the all-day time) or `Absolute { due.date = "YYYY-MM-DDTHH:MM:00" }`. `Action::{ReminderAdd, ReminderDelete}` (idempotent like every command); `Repository::{upsert_reminder, list_reminders}`; deleting a task deletes its (and its sub-tasks') reminders.
- **Planner**: `reminders::plan(tasks, reminders, settings, now)` adds explicit reminders and a "Deadline today" reminder (at the all-day time) to the global ones, deduped by task + time. The master switch still gates everything. iOS needs nothing new — it schedules whatever the plan says.
- **UI**: editor gets a Deadline chip (the scheduler without a time) and a Reminders chip (list + presets + "custom" field that reads natural language like "tomorrow 9am"). Rows show a bell when a task has reminders. Reminders are shared through Dioxus context (no prop threading).

## Global Constraints
- Relative reminders follow each occurrence of a recurring task; absolute ones fire once.
- Reminders can be added while creating a task (applied after the add, using the new id).
- Deadlines don't move tasks into Today/Upcoming (as in Todoist); they're shown and filterable.

## Review Focus
- **Relative reminder on a date-only task** uses the all-day time; skipped when that's off — Task 2.
- **Dedupe**: an explicit "at time of task" plus the global rule fire once — Task 2.
- **Cascade**: deleting a parent removes sub-task reminders — Task 3.
- **Clearing a deadline** via `Some("")` vs leaving it untouched (`None`) — Task 1.

---

## Task 1: Deadlines (core)
- [x] Commands (add/update/clear), filter terms, quick-add braces + span. Tests. Commit.

## Task 2: Reminders (core)
- [x] `ReminderAdd/Delete`, repo trait methods, cascade on delete; planner with explicit + deadline reminders + dedupe. Tests. Commit.

## Task 3: Storage + service + IPC
- [x] SQLite `upsert_reminder`/`list_reminders`/cascade; `AppService::list_reminders`; plan uses them; IPC `list_reminders`. Tests. Commit.

## Task 4: UI
- [x] Deadline chip in editor + rows; Reminders chip/popover (add/remove, presets, custom NL); bell on rows; add-mode drafts. `dx build` clean; preview check. Commit.

## Task 5: Verify in the real app, merge, push
- [x] Drive the desktop app over the WebView debug port: set a deadline, add a "1 min" custom reminder, confirm it fires. Merge, push.

## Notes
- Found while verifying in the real app: clicking a form's submit button (Add task / Save / every sidebar form) did a native form submit and reloaded the page — Dioxus' `prevent_default()` in `onsubmit` isn't honored on web. Fixed globally with one capture-phase `submit` listener (`App`), so in-flight work (e.g. saving reminders after the task) is no longer cut off.
- The custom-reminder field parses "tomorrow 9am" with quick add as if after a title (quick add applies nothing to a phrase that would leave the title empty).
