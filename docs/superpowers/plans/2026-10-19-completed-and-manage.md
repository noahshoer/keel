# Hidden Completed Tasks + Rename/Delete Labels & Projects — Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Completed tasks stay hidden by default with a per-view "Show completed" toggle (reopen by unticking); labels and projects can be renamed or deleted from the sidebar.

**Architecture:** Core gains `TaskUncomplete`, `ProjectUpdate { name }` and `LabelUpdate { name }`; the Inbox is protected from rename/delete. The service adds completed-task queries (today / project / label). The UI adds a header toggle (off by default) that appends a Completed section, and hover actions (rename inline, delete with inline confirm) on sidebar projects and labels.

**Spec:** `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md`

## Global Constraints
- "Completed" = checked (non-recurring) tasks. Recurring tasks reschedule and never appear there.
- Completed lists are newest-first by `completed_at`. Today's list = completed on the local calendar day.
- Inbox: `ProjectUpdate`/`ProjectDelete` on the Inbox are no-ops.
- Renaming a label renames it on every task; renaming onto an existing label's name **merges** (one label remains, tasks deduped).
- Deleting a project soft-deletes its tasks (existing behavior); the UI confirms with the task count.

## Review Focus
- **Reopening** clears `completed_at` and returns the task to normal views — Task 1.
- **Inbox protection** against rename and delete — Task 1.
- **Label merge** leaves no task with the label twice — Task 1.
- **Completed-today boundary** uses local date of `completed_at` — Task 2.
- **Deleting the view you're on** returns the UI to Inbox (not a dead view) — Task 3.

---

## Task 1: Commands
**Files:** `crates/core/src/commands.rs`; test `crates/storage/tests/manage.rs`.
- [ ] Tests: `uncomplete_reopens_task`, `rename_project`, `inbox_cannot_be_renamed_or_deleted`, `rename_label_updates_tasks`, `rename_label_onto_existing_merges`. RED → implement → GREEN → commit.

## Task 2: Completed queries
**Files:** `crates/service/src/lib.rs`, test `crates/service/tests/completed.rs`, `crates/src-tauri/src/lib.rs`.
**Produces:** `AppService::{completed_in_project, completed_with_label, completed_today_at(today)}` (+ IPC `completed_in_project(project_id)`, `completed_with_label(name)`, `completed_today`).
- [ ] Tests: `project_completed_newest_first`, `label_completed`, `completed_today_only_today`. RED → implement → GREEN → commit.

## Task 3: UI
**Files:** `crates/ui/src/{main.rs, sidebar.rs, task_row.rs (CompletedRow), api.rs, style.rs, icons.rs}`.
- Header "Show completed / Hide completed" toggle on Today, Inbox, project and label views (hidden by default).
- Completed section: filled check (click = reopen), struck-through title, "Completed 19 Oct", delete.
- Sidebar hover actions on projects/labels (not Inbox): rename inline (Enter saves, Esc cancels), delete with inline confirm ("Delete project and its N tasks?").
- Deleting/renaming the current view keeps the view consistent (delete → Inbox; rename → updated title).
- [ ] `dx build` clean; stage `dist/`; commit.
