# Sections + Sub-tasks — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Projects can be divided into sections, and tasks can have nested sub-tasks (collapsible), Todoist-style.

**Architecture:** A pure `core::views::nest` turns a flat task list into display order with depth (host-tested). Core gains section commands (`SectionAdd/Update/Delete`), `TaskMove` (project + section, moves the whole sub-tree), `TaskUpdate.is_collapsed`, and cascading completion to sub-tasks. Storage implements section persistence (table exists). The service lists a project's sections. The UI groups project views by section, nests sub-tasks with a collapse chevron, adds "+ Add sub-task" and "+ Add section", section rename/delete, and a combined Project/Section picker in the editor.

**Spec:** `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md` (§2 sections, sub-tasks)

## Global Constraints
- Sub-tasks nest under their parent when the parent is in the same list; otherwise they show at top level. Today/Upcoming stay flat.
- Collapsing hides all descendants. Sibling order is preserved from the input.
- Moving a task (project/section) moves its whole sub-tree; changing project clears section.
- Completing a non-recurring task completes its non-recurring descendants. A recurring parent only reschedules.
- Deleting a section soft-deletes it and its tasks (Todoist behavior), with an inline confirm in the UI.

## Review Focus
- **Deep nesting** (grandchildren) displays and collapses correctly — Task 1.
- **Orphaned sub-tasks** (parent filtered out) still appear — Task 1.
- **Move keeps sub-tree together** across projects — Task 2.
- **Cascade complete** skips recurring descendants — Task 2.
- **Section delete** removes its tasks but not other sections' tasks — Task 2.

---

## Task 1: `views::nest`
**Produces:** `struct Nested { task: Task, depth: usize, has_children: bool }`; `fn nest(tasks: Vec<Task>) -> Vec<Nested>` (DFS, skips descendants of `is_collapsed` tasks).
- [ ] Tests: `nests_children_under_parents_in_order`, `grandchildren_depth_2`, `collapsed_hides_descendants`, `orphan_child_is_top_level`. RED → GREEN → commit.

## Task 2: Sections, move, cascade
**Files:** `core/{commands,repository,model}.rs`, `storage/src/sqlite_repo.rs`; test `storage/tests/sections.rs`.
**Produces:** `Repository::{upsert_section, list_sections(project)}`; `Section::new`; `Action::{SectionAdd{project_id,name}, SectionUpdate{id,name}, SectionDelete{id}, TaskMove{id,project_id,section_id}}`; `TaskUpdate.is_collapsed`; cascading `TaskComplete`.
- [ ] Tests: `add_rename_list_sections`, `delete_section_removes_its_tasks_only`, `move_task_moves_subtree`, `update_project_clears_section`, `complete_cascades_to_nonrecurring_children`, `collapse_flag_persists`. RED → GREEN → commit.

## Task 3: Service + IPC
- [ ] `AppService::list_sections(project_id)` + IPC `list_sections`; test `lists_sections_for_project`. Commit.

## Task 4: UI
- Project/Inbox views: unsectioned tasks, then each section (header with count + hover rename/delete, its tasks, its own "+ Add task"), then "+ Add section".
- Nested rendering via `nest` with indentation and a chevron on tasks that have children (toggles `is_collapsed`).
- Row hover action "Add sub-task" → editor indented under the task (parent, project, section prefilled).
- Editor location picker: "Project" and "Project / Section" options; changing it dispatches `TaskMove`.
- [ ] `dx build` clean; stage; commit.
