# Task Editing + Labels — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Edit any task after creation (name, description, date, repeat, priority, project, labels) and organize tasks with labels, Todoist-style.

**Architecture:** A patch-style `TaskUpdate` command in `core` (absent field = keep). Labels become stored entities (`label` table already exists) with `LabelAdd`/`LabelDelete` commands; tasks keep label **names** inline (ruling R6). The service adds `list_labels` and `label_view`. The UI extracts one shared `TaskEditor` used for both add and edit (click a task to edit in place), adds project/label pickers, a Labels section in the sidebar with label views, label tags on rows, and an Overdue section atop Upcoming.

**Spec:** `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md` (§2 labels, §6 model)

## Global Constraints
- Patch semantics: every `TaskUpdate` field is optional; omitted = unchanged. Due uses `DueEdit { date: Option<String>, recurrence: Option<Recurrence> }`: `due: None` keeps; `Some(DueEdit{date: None, recurrence: None})` clears.
- Label names are unique case-insensitively; adding an existing name is a no-op that maps the temp id to the existing label.
- Deleting a label soft-deletes it and strips the name from every task.
- Changing a task's recurrence resets `streak_current` and `streak_last_period` (best is kept): old occurrence history no longer applies.
- Keep UI Todoist-faithful and theme-aware (CSS variables only).

## Review Focus
- **Partial updates** must not clobber untouched fields (e.g. editing only the name keeps due, labels, streak) — Task 1.
- **Clearing a date** on a recurring task removes the recurrence too — Task 1.
- **Duplicate label names** differing only in case — Task 2.
- **Deleting a label** leaves no task still carrying it — Task 2.
- **Label view excludes completed tasks** and spans projects — Task 3.

---

## Task 1: `TaskUpdate`
**Files:** `crates/core/src/commands.rs`; test `crates/storage/tests/edit.rs`.
**Produces:** `Action::TaskUpdate { id, content?, description?, priority?, project_id?, labels?, due?: DueEdit }`; `pub struct DueEdit`.
- [ ] Tests: `update_content_only_keeps_everything_else`, `update_sets_date_priority_project_labels`, `clear_due_removes_recurrence`, `set_recurrence_without_date_uses_first_occurrence`, `changing_recurrence_resets_streak_keeps_best`. RED → implement → GREEN → commit.

## Task 2: Labels — storage + commands
**Files:** `crates/core/src/repository.rs`, `crates/storage/src/sqlite_repo.rs`, `crates/core/src/commands.rs`, `crates/core/src/model.rs` (`Label::new`); test `crates/storage/tests/labels.rs`.
**Produces:** `Repository::{upsert_label, list_labels}`; `Action::LabelAdd { name }`, `Action::LabelDelete { id }`.
- [ ] Tests: `add_and_list_labels`, `duplicate_name_case_insensitive_is_noop`, `delete_label_strips_it_from_tasks`. RED → implement → GREEN → commit.

## Task 3: Service + IPC
**Files:** `crates/service/src/lib.rs`, test `crates/service/tests/labels.rs`, `crates/src-tauri/src/lib.rs`.
**Produces:** `AppService::{list_labels, label_view_at(name, today), label_view}`; Tauri `list_labels`, `label_view(name)`.
- [ ] Test `label_view_spans_projects_and_hides_completed`. RED → implement → GREEN → backend builds → commit.

## Task 4: UI
**Files:** `crates/ui/src/{editor.rs (new, replaces add_task.rs), task_row.rs, sidebar.rs, main.rs, api.rs, style.rs, icons.rs}`.
- Shared `TaskEditor` (name, description, date, priority, repeat, **project** select, **labels** picker with inline "create label"), `Save`/`Add task` + `Cancel`.
- Click a task's text → row turns into the editor prefilled; Save dispatches `TaskUpdate` with only changed fields.
- Rows show `@label` tags in the meta line.
- Sidebar **Labels** section with "+", each opening a label view.
- Upcoming gets an **Overdue** section on top.
- [ ] `dx build` clean; stage `dist/`; commit.
