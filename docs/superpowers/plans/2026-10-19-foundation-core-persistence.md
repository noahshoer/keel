# Foundation: Core, Persistence & iOS Shell — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the iOS Tauri+Dioxus app with a local libSQL store, the domain model, a uuid-idempotent command log, and task/project CRUD proven end-to-end on the simulator.

**Architecture:** A pure-Rust `core` crate holds the domain model, command definitions, and a `Repository` trait (no DB or Tauri deps). A `storage` crate implements `Repository` on libSQL (local file) and owns the SQL migrations. `src-tauri` wires `core` + `storage` and exposes IPC commands. A Dioxus WASM frontend renders in the Tauri webview and calls those commands. All domain logic lives in `core`; the UI stays thin.

**Tech Stack:** Rust (edition 2021), Tauri v2, Dioxus 0.6.x, libSQL (`libsql` crate), `ulid`, `chrono` + `chrono-tz`, `serde`/`serde_json`. iOS target via `tauri ios`.

**Spec:** `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md`

## Global Constraints

- Rust edition **2021**, toolchain **≥ 1.80**. Workspace with crates `core`, `storage`, `src-tauri`, `ui`.
- **Tauri v2** (≥ 2.0), **Dioxus 0.6.x**, Dioxus compiled to **WASM in the Tauri webview** (not Dioxus standalone mobile).
- Persistence is **libSQL** against a **local file**; embedded-replica/remote sync is **not** configured in v1 (dormant).
- IDs are **ULID** strings, generated client-side.
- **Priority** stored API-style: `4` = highest ("P1") … `1` = none.
- **English only**; no integrations, collaboration, comments, attachments, location reminders.
- Every mutation goes through a **command** with a client `uuid`; applying the same `uuid` twice is a no-op (idempotent).
- All domain logic in `core` (no Tauri/DB types leak into it). UI calls Rust via Tauri IPC only.

## Review Focus

- **Re-running migrations** on an existing DB must be a safe no-op (not an error or duplicate schema) — pinned in Task 3.
- **Replaying a command with an already-seen `uuid`** must not double-apply (e.g., add the task twice) — pinned in Task 4.
- **Floating vs zoned due dates:** a task with `due_timezone = NULL` (floating) must round-trip through storage as NULL, not coerced to a device tz — pinned in Task 3.
- **Deleting a project** with tasks, and **deleting a task** with sub-tasks/label links, must not orphan rows (soft-delete cascade or explicit handling) — pinned in Task 4.
- **Very long task content** (e.g., 10k chars) must persist and read back intact (no truncation) — pinned in Task 3.

---

## Task 1: Workspace scaffold + iOS/stack feasibility spike

**Files:**
- Create: `Cargo.toml` (workspace), `rust-toolchain.toml`, `.gitignore`
- Create: `crates/core/Cargo.toml`, `crates/core/src/lib.rs`
- Create: `crates/storage/Cargo.toml`, `crates/storage/src/lib.rs`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Create: `ui/Cargo.toml`, `ui/src/main.rs`, `ui/index.html`
- Test: `src-tauri/tests/smoke.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: a Tauri command `ping() -> String` returning `"pong"`; the workspace crate layout all later tasks build in.

- [ ] **Step 1: `git init`** and add a Rust/Tauri/Dioxus `.gitignore` (`/target`, `dist/`, `src-tauri/gen/`, `.DS_Store`).

- [ ] **Step 2: Create the Cargo workspace** with members `crates/core`, `crates/storage`, `src-tauri`, `ui`. Pin dependencies at the Global Constraints versions.

- [ ] **Step 3: Scaffold the Tauri v2 app + Dioxus UI** — a `ping` IPC command in `src-tauri`, and a Dioxus page in `ui` that calls `ping` and renders the result. Follow the current Tauri v2 + Dioxus-in-webview setup (Trunk or dx build feeding Tauri's `frontendDist`).

- [ ] **Step 4: Write the smoke test** `tests/smoke.rs::ping_returns_pong` asserting the `ping` command function returns `"pong"`.

- [ ] **Step 5: Run it and verify it passes** — `cargo test -p app smoke` → PASS.

- [ ] **Step 6: Spike — run on iOS simulator.** `cargo tauri ios init` then `cargo tauri ios dev` (simulator). Confirm the Dioxus page renders and shows `pong`. Record in the plan's repo a short `docs/notes/ios-spike.md` with: Tauri/Dioxus/libSQL versions used, whether libSQL links on `aarch64-apple-ios`, and any workarounds.
  - **Gate:** if the stack cannot build/run on iOS, STOP and report — the design's stack assumption is invalidated and must be revisited before continuing.

- [ ] **Step 7: Commit** — `git add -A && git commit -m "chore: workspace scaffold + iOS stack spike"`.

---

## Task 2: Core domain model

**Files:**
- Create: `crates/core/src/model.rs`, `crates/core/src/ids.rs`
- Modify: `crates/core/src/lib.rs` (module wiring)
- Test: `crates/core/src/model.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `Id(String)` newtype with `Id::new() -> Id` (ULID) and `Display`.
  - Structs: `Task`, `Project`, `Section`, `Label`, `Filter`, `Due`, `Reminder`.
  - `Priority` (`P1..P4`, `None`) with `fn to_api(self) -> u8` / `from_api(u8)` using the 4=highest mapping.
  - `ViewStyle { List, Board, Calendar }`.
  - `Due { date: NaiveDate | DateTime, timezone: Option<String>, string: String, lang: String, is_recurring: bool, rrule: Option<String> }` (store recurrence rule as text now; parser lands in a later plan).
  - All types `#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]`.

- [ ] **Step 1: Write failing tests** in `model.rs`:
  - `id_new_is_unique_and_sortable` — two `Id::new()` differ; later-generated sorts ≥ earlier (ULID).
  - `priority_api_roundtrip` — `Priority::P1.to_api() == 4`; `Priority::from_api(4) == P1`; `None.to_api() == 1`.
  - `task_serde_roundtrip` — a fully-populated `Task` serializes to JSON and back equal.
  - `floating_due_keeps_none_timezone` — a `Due` with `timezone: None` round-trips with `None`.

- [ ] **Step 2: Run** `cargo test -p core model` → FAIL (types not defined).

- [ ] **Step 3: Define the types and `Id`/`Priority` impls** in `model.rs`/`ids.rs` (ULID via `ulid`, dates via `chrono`). Fields per the design §6; no multi-user fields.

- [ ] **Step 4: Run** `cargo test -p core model` → PASS.

- [ ] **Step 5: Commit** — `feat(core): domain model, ids, priority mapping`.

---

## Task 3: Storage — libSQL migrations & Repository

**Files:**
- Create: `crates/storage/src/migrations.rs` (embedded SQL), `crates/storage/src/sqlite_repo.rs`
- Create: `crates/core/src/repository.rs` (the trait)
- Modify: `crates/core/src/lib.rs`, `crates/storage/src/lib.rs`
- Test: `crates/storage/tests/repo.rs`

**Interfaces:**
- Consumes: `core::model::*`.
- Produces:
  - In `core`: trait `Repository` with async methods `upsert_task(&Task)`, `get_task(&Id) -> Option<Task>`, `list_tasks(project: &Id) -> Vec<Task>`, `upsert_project`, `get_project`, `list_projects`, and equivalents for section/label/filter; `soft_delete_task(&Id)`; plus `has_command(uuid: &str) -> bool` / `record_command(uuid, type, args_json)`.
  - In `storage`: `SqliteRepo` implementing `Repository`, with `SqliteRepo::open(path: &str) -> Result<Self>` that runs migrations on open.
  - `migrations::run(conn)` — idempotent DDL creating tables from design §6 (`task`, `project`, `section`, `label`, `task_label`, `filter`, `reminder`, `command_log`, `streak_event`) and a `schema_version` row.

- [ ] **Step 1: Write failing tests** in `tests/repo.rs` (each opens a temp-file DB):
  - `migrations_are_idempotent` — `run()` twice succeeds; tables exist once.
  - `task_roundtrip` — upsert then `get_task` returns an equal `Task`.
  - `long_content_roundtrips` — 10_000-char `content` persists and reads back byte-identical.
  - `floating_due_stored_as_null_tz` — task with `due.timezone = None` reads back `None`.
  - `list_tasks_filters_by_project_and_excludes_deleted` — soft-deleted tasks are excluded.

- [ ] **Step 2: Run** `cargo test -p storage repo` → FAIL.

- [ ] **Step 3: Implement migrations + `SqliteRepo`** — embed DDL as `CREATE TABLE IF NOT EXISTS …`; map rows ↔ model; `upsert` via `INSERT … ON CONFLICT(id) DO UPDATE`. Use the `libsql` local-file connection.

- [ ] **Step 4: Run** `cargo test -p storage repo` → PASS.

- [ ] **Step 5: Commit** — `feat(storage): libSQL migrations and Repository impl`.

---

## Task 4: Command log & command application

**Files:**
- Create: `crates/core/src/commands.rs`, `crates/core/src/engine.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/tests/commands.rs`

**Interfaces:**
- Consumes: `core::model::*`, `core::repository::Repository`.
- Produces:
  - `enum Command { TaskAdd{..}, TaskUpdate{..}, TaskComplete{id}, TaskUncomplete{id}, TaskDelete{id}, TaskMove{id, project_id, section_id}, ProjectAdd{..}, ProjectUpdate{..}, ProjectDelete{id} }`, each carrying a `uuid: String` and (for creates) a `temp_id`→real `Id`.
  - `async fn apply<R: Repository>(repo: &R, cmd: Command) -> Result<ApplyResult>` where `ApplyResult { id_mapping: HashMap<String, Id> }`.
  - `apply` records the `uuid` in `command_log` and returns early (no-op) if `repo.has_command(uuid)` is already true.

- [ ] **Step 1: Write failing tests** in `tests/commands.rs` (in-memory/temp `SqliteRepo`):
  - `task_add_creates_and_maps_temp_id` — `TaskAdd` with `temp_id` returns an `id_mapping` entry and the task is retrievable.
  - `duplicate_uuid_is_noop` — applying the same `TaskAdd` `uuid` twice yields exactly one task.
  - `task_complete_sets_checked_and_completed_at`.
  - `delete_project_soft_deletes_its_tasks` — after `ProjectDelete`, its tasks are excluded from `list_tasks` (no orphans).
  - `delete_task_removes_its_label_links_and_subtasks` — children + `task_label` rows gone/soft-deleted.

- [ ] **Step 2: Run** `cargo test -p core commands` → FAIL.

- [ ] **Step 3: Implement `Command`, `apply`, and cascade rules.** Recurring-completion behavior (advance due + log history) is deferred to the recurrence plan; `TaskComplete` here just sets `checked`/`completed_at`.

- [ ] **Step 4: Run** `cargo test -p core commands` → PASS.

- [ ] **Step 5: Commit** — `feat(core): command log and idempotent command application`.

---

## Task 5: Tauri IPC command layer

**Files:**
- Create: `src-tauri/src/ipc.rs`, `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs` (register handlers, manage state)
- Test: `src-tauri/tests/ipc.rs`

**Interfaces:**
- Consumes: `core::commands::{Command, apply}`, `core::repository::Repository`, `storage::SqliteRepo`.
- Produces Tauri commands (thin wrappers, each returning `Result<T, String>`):
  - `dispatch(cmd: Command) -> ApplyResult`
  - `list_tasks(project_id: String) -> Vec<Task>`
  - `list_projects() -> Vec<Project>`
  - App state `AppState { repo: SqliteRepo }` initialized at the platform app-data path; on first run seeds a default **Inbox** project.

- [ ] **Step 1: Write failing tests** in `tests/ipc.rs` calling the underlying handler fns directly (not through the webview):
  - `dispatch_task_add_then_list_returns_it`.
  - `first_run_seeds_inbox` — fresh DB → `list_projects()` contains an `is_inbox` project.

- [ ] **Step 2: Run** `cargo test -p app ipc` → FAIL.

- [ ] **Step 3: Implement the handlers + state init + Inbox seed**; register with `tauri::generate_handler!`.

- [ ] **Step 4: Run** `cargo test -p app ipc` → PASS.

- [ ] **Step 5: Commit** — `feat(app): Tauri IPC command/query layer with seeded Inbox`.

---

## Task 6: Minimal end-to-end UI (Dioxus)

**Files:**
- Create: `ui/src/app.rs`, `ui/src/api.rs` (typed wrappers over Tauri `invoke`)
- Modify: `ui/src/main.rs`
- Test: `ui/src/api.rs` (`#[cfg(test)]` for pure serialization helpers)

**Interfaces:**
- Consumes: the Task 5 IPC commands via `invoke`.
- Produces: a running screen listing the Inbox's tasks with a quick-add text box (plain text only — no NL parse yet), a complete toggle, and delete.

- [ ] **Step 1: Write a failing test** `api::tests::task_add_args_serialize` asserting the `TaskAdd` argument payload serializes to the exact JSON shape the IPC command expects (field names/types).

- [ ] **Step 2: Run** `cargo test -p ui api` → FAIL.

- [ ] **Step 3: Implement `api.rs`** typed wrappers (`add_task`, `list_tasks`, `complete_task`, `delete_task`) and the Dioxus `app.rs` screen (list + quick-add + toggle/delete), wired to live-reload the list after each mutation.

- [ ] **Step 4: Run** `cargo test -p ui api` → PASS.

- [ ] **Step 5: Manual verification on iOS simulator** — `cargo tauri ios dev`: add a task, see it listed, complete it, delete it; relaunch and confirm persistence. Note result in `docs/notes/ios-spike.md`.

- [ ] **Step 6: Commit** — `feat(ui): minimal task list + quick-add, end-to-end on iOS`.

---

## Done when
- The app builds and runs on the iOS simulator.
- `cargo test` passes across `core`, `storage`, `app`, `ui`.
- A task added in the UI persists across relaunch, backed by libSQL, through the command log.
- The stack spike (Task 1/6 notes) confirms Tauri v2 iOS + Dioxus + libSQL is viable — or has STOPPED the line with findings.

## Follow-on plans (not in this plan)
1. NL date/recurrence parser + recurrence engine.
2. Labels, sections, sub-tasks UI + filter-query evaluator + search.
3. Streaks engine + UI.
4. iOS local notifications.
5. (Later) sync activation (Turso embedded replicas) + account/device-link, when platform #2 begins.
