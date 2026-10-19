# Design — Standalone Todoist-style App (Tauri + Dioxus), v1

**Date:** 2026-10-19
**Status:** Draft for approval
**Reference RE:** `docs/todoist-reverse-engineering.md` (shell), `docs/todoist-layer-b-api-and-model.md` (data model & sync, verified)

---

## 1. Intent & why

Build a **standalone, local-first task manager** that reproduces the parts of Todoist the user relies on — **with no paywall** — and then grows features Todoist doesn't offer. It is *not* a Todoist client: it does not touch Todoist's servers, account, or data. We reuse Todoist's **data model and UX as a reference** (reverse-engineered), implementing our own schema, sync, and UI.

**Why now:** the user hits Todoist's premium wall on everyday features (filters, etc.) and wants capabilities Todoist has never shipped — starting with **habit streaks** on recurring tasks. Owning the whole stack is the only way to add those freely.

## 2. Goals (v1)

1. A faithful, Todoist-like iOS app: projects, labels, filters (with query language), sections, sub-tasks, priorities, due dates, deadlines, recurring tasks — **all free**.
2. **Streaks** as the first original feature (see §7).
3. **Full natural-language date/recurrence input** (see §8).
4. **Basic iOS local notifications** for due/timed tasks.
5. **Local-first / fully offline** on device, with a sync layer **designed in** (dormant in v1) so Mac/Windows/Web and multi-device sync drop in later with minimal rework.
6. Rust-centric codebase (**Tauri v2 + Dioxus**) that cross-compiles to more platforms later.

## 3. Non-goals / anti-goals (v1)

- **Non-goals (deliberately deferred):** Mac/Windows/Web builds; multi-device sync *activation*; the external integrations (Apple Reminders, Microsoft To-Do, Google Calendar, Outlook, Notion); comments/notes, file attachments, location reminders, Karma/productivity gamification beyond streaks, collaboration/sharing/workspaces/assignees, templates, activity log, backups.
- **Anti-goals (would make v1 a failure even if it "works"):** depending on Todoist's servers or account in any way; a data model so iOS-specific or sync-naive that adding platform #2 or multi-device sync forces a rewrite; a half-working NL parser that silently misreads dates (correctness of date parsing is load-bearing for streaks).

## 4. Constraints & decisions (as decided with the user)

| Area | Decision |
|------|----------|
| Product | Standalone; **full native rebuild** (not a Todoist shell-swap) |
| v1 platform | **iOS only** |
| Stack | **Tauri v2** app shell + **Dioxus** frontend (Rust → WASM in the webview); Rust backend owns data |
| Local store | **SQLite** on device, offline-first |
| Shared source of truth | **Turso / libSQL** (embedded replicas); SQLite-compatible, self-hostable later |
| Sync in v1 | **Dormant** — schema + command log are sync-ready, but no sync UI/login ships until platform #2 |
| UI | **Mimic Todoist** initially; redesign later |
| IDs | Our own **string IDs** (ULID/UUID), generated client-side |
| Streaks | **Opt-in per task**, **hard reset on miss**, **optional per-task goal**, always track current + best |
| Recurrence/date input | **Full natural-language parser** + structured |
| Notifications | **Basic iOS local notifications** in v1 |

## 5. Architecture

```
┌───────────────────────── iOS app (Tauri v2) ─────────────────────────┐
│  Dioxus UI (WASM in webview)                                          │
│    - views: Today, Upcoming, Project, Label, Filter                   │
│    - quick-add w/ NL parse preview                                    │
│      │  (Tauri IPC commands/events)                                   │
│  Rust core (Tauri backend)                                            │
│    - domain model (tasks, projects, sections, labels, filters, …)     │
│    - command log (uuid-idempotent mutations)   ← Todoist-style        │
│    - NL date/recurrence parser                                         │
│    - recurrence engine + streak engine                                │
│    - filter-query parser/evaluator                                    │
│    - local notification scheduler (tauri-plugin-notification)         │
│    - persistence: libSQL local file                                   │
│         └── (dormant) libSQL embedded-replica sync → Turso primary    │
└───────────────────────────────────────────────────────────────────────┘
```

**Topology (resolved call).** Dioxus is used as the **frontend compiled to WASM and loaded inside the Tauri v2 webview** (not Dioxus's own standalone mobile renderer). All domain logic — model, command log, parsers, engines, persistence — lives in the **Rust Tauri core**; the Dioxus UI stays thin and talks to the core over Tauri IPC commands/events. Rationale: keeps one authoritative Rust core reusable across every future platform, avoids splitting logic into WASM, and keeps the UI swappable. *Feasibility spike in phase 1:* confirm Tauri v2 iOS + Dioxus-in-webview and the **libSQL client + embedded replicas on iOS (aarch64)** build and run on device — both are young; this is the first thing to de-risk.

**Local-first + command log.** Every mutation is a **command** (`{id, type, args, ts}`) applied to the local store and appended to a durable log, mirroring the Todoist sync model we verified. In v1 the log simply drives local state; when sync activates, the same log replays to the shared DB and deltas merge back. Designing this now is what keeps platform #2 cheap.

**Why libSQL embedded replicas:** reads/writes hit a local SQLite file (full offline); the replica syncs to a Turso primary when enabled. No bespoke sync protocol to invent for the happy path; conflict strategy is last-writer-wins per field initially, revisited if real multi-device editing exposes problems. *(My call; flagged for revisit.)*

## 6. Data model (local SQLite schema)

Shaped after the verified Todoist model, **minus** multi-user fields (no `workspace_id`, `collaborators`, `assigned_by_uid`, `responsible_uid`). Core tables:

- **task** — `id`, `project_id`, `section_id`, `parent_id`, `content`, `description`, `priority` (1–4), `due_*` (see Due), `deadline_date`, `duration_amount`, `duration_unit`, `child_order`, `order_key`, `day_order`, `is_collapsed`, `checked`, `is_deleted`, `added_at`, `completed_at`, `updated_at`, **streak fields** (§7).
- **project** — `id`, `name`, `color`, `icon`, `parent_id`, `child_order`, `view_style` (list/board/calendar), `is_favorite`, `is_collapsed`, `is_archived`, `is_inbox`, `is_deleted`, timestamps.
- **section** — `id`, `project_id`, `name`, `section_order`, `is_collapsed`, `is_archived`, `is_deleted`, timestamps.
- **label** — `id`, `name`, `color`, `item_order`, `is_favorite`, `is_deleted`.
- **task_label** — join (`task_id`, `label_id`).
- **filter** — `id`, `name`, `query`, `color`, `item_order`, `is_favorite`, `is_deleted`.
- **reminder** — `id`, `task_id`, `type` (relative/absolute), `due_*`, `minute_offset`, `is_deleted`. (location type deferred.)
- **Due (embedded on task):** `due_date`, `due_timezone` (nullable = floating), `due_string` (the raw NL text), `due_lang`, `due_is_recurring`, plus the parsed recurrence rule (§8).
- **command_log** — `id` (uuid), `type`, `args_json`, `created_at`, `applied`, `synced`.
- **streak_event** — append-only completion history per streak task (see §7).

**Notes (resolved calls):**
- **Priority** stores the API convention (4 = highest / UI "P1" … 1 = none); the UI inverts for display.
- **Filter query language (v1 subset):** `today`, `overdue`, `#project`, `@label`, `pN`, `/section`, `no date`, `recurring`, date expressions (`due before: +N days`), boolean `& | !`, and `,` (OR). The exotic long tail (`assigned to:`, `added:` etc. tied to deferred features) is out of v1.
- **Search:** basic text search over task content/description is in v1; saved-search niceties deferred.

## 7. Streaks — detailed spec

**Opt-in.** Any task with a recurrence can toggle **Track streak**. Non-recurring tasks can't (nothing to be "in a row").

**Recurring-task completion model (resolved call).** Mirrors Todoist: completing a recurring task does **not** create a new row — it advances the **same task's** due date to the next occurrence and appends a **completion record** (a `streak_event`). An uncompleted occurrence stays overdue (the due pointer does not auto-advance). This completion history — not the task's current due pointer — is the source of truth for streaks.

**Period = one recurrence occurrence.** The streak counts *scheduled occurrences completed on time*, not raw completions. Daily task → one period per day; weekly → one per week; "every Mon/Wed/Fri" → one period per scheduled weekday.

**On completion of a scheduled occurrence:** `current_streak += 1`; append a `streak_event`. Completing twice within one period does **not** double-count.

**Miss detection (resolved call):** the streak engine **enumerates expected occurrences from the recurrence rule** over elapsed time and checks each against the completion history. Any scheduled period with no matching completion record is a miss. This is computed independently of the task's current due pointer, lazily at app open / day rollover and on the scheduler tick.

**On a miss:** **hard reset** `current_streak = 0` (append a `reset` streak_event). A task completed **late** (after its period, before the next) does **not** rescue the missed period — strict reset, consistent with the user's "hard reset" choice. A grace/"streak freeze" is a documented later refinement, not v1.

**Best record:** `best_streak = max(best_streak, current_streak)`, retained across resets.

**Goal (opt-in):** optional per-task integer `goal`. UI shows progress toward goal and distance to best record. Hitting the goal is celebrated but does not stop the streak.

**Task fields added:** `streak_enabled` (bool), `streak_current` (int), `streak_best` (int), `streak_goal` (int, nullable), `streak_last_period` (the period key last credited, to dedupe and detect misses).

**Boundaries:** day = local midnight; week starts Monday (user-configurable later). Period keys are computed in the task's own timezone (or the device's, for floating due).

**Streak UI (v1):** a flame/count badge on streak tasks showing current streak; task detail shows current, best record, and (if set) goal with progress. Visual design is a later mockup pass.

## 8. Natural-language date/recurrence parser

The biggest single module. Targets the verified Todoist **Due object** as output: `{date, timezone, string, lang, is_recurring}` plus a structured recurrence rule for the engine.

- **Scope:** reproduce Todoist's everyday phrases *and* the long tail — `today`, `tomorrow`, `next monday`, `jan 5`, `in 3 days`, `every day`, `every weekday`, `every 2 weeks`, `every 3rd of the month`, `every last workday`, times (`at 5pm`), and combinations.
- **Approach:** a dedicated Rust crate with a tokenizer + grammar; recurrence compiles to an internal rule (RRULE-like) the recurrence engine iterates. English only in v1 (`lang: "en"`).
- **Validation:** build a test corpus of input→expected-date/rule pairs. Todoist's own behavior is the reference oracle; where we're unsure, the RE'd live API (`due.string`→`due.date`) can be consulted during development, not at runtime.
- **UX:** quick-add shows a live parse preview (like Todoist's highlighted date chip) so misreads are visible before commit.

**Risk/My call:** "full" parser is open-ended. The plan (next phase) will cut it into tiers — ship a strong core first, expand the long tail iteratively — rather than gating v1 on 100% coverage. Flagged for approval.

## 9. Notifications (v1)

- iOS **local notifications** via `tauri-plugin-notification` (v2). Schedule on task due datetime and on explicit reminders (`minute_offset` before due).
- Requires iOS notification permission flow; reschedule on task edit/complete/delete; clear on completion.
- Recurring tasks schedule the **next** occurrence's notification after each completion/rollover.
- Push and location reminders are out of v1.

## 10. Calls I made (confirm while reading) & items deferred to the plan

Resolved during the review pass — flag any you disagree with:
- **Dioxus-in-Tauri-webview, all logic in the Rust core** (§5).
- **Recurring completion advances the same row + logs history; miss = rule-enumeration vs history; late completion = strict reset** (§7).
- **Priority stored API-style; filter-query v1 subset; basic search in** (§6).
- **IDs: ULID** (sortable, offline-generatable).
- **Sync conflict policy:** last-writer-wins per field as the starting point; revisited when real multi-device editing exists (sync is dormant in v1 anyway).

Deferred to the planning step (not blocking approval):
- Exact **NL-parser coverage tiers** (ship a strong core, expand the long tail).
- **Account/device-link** model for when sync activates (lightweight account issuing a Turso token).
- Specific crates (libSQL client, Dioxus state mgmt, notification scheduling) and the full SQLite DDL.

## 11. Rough shape of the build (full plan comes next, not here)

1. Project scaffold: Tauri v2 + Dioxus, iOS target building to simulator/device; libSQL wired.
2. Domain model + schema + command log; basic CRUD for tasks/projects.
3. Core views (Today / project list) mimicking Todoist; quick-add.
4. NL date parser (core tier) + recurrence engine.
5. Labels, filters + filter-query evaluator, sections, sub-tasks.
6. Streaks engine + UI.
7. Local notifications.
8. Polish pass on the Todoist-like UI; NL parser long-tail expansion.

*(The sync layer is schema-ready throughout but not activated in v1.)*

## 12. What's left to the builder

Concrete library choices within the decided stack (libSQL crate, state management in Dioxus, NL-parser internals, notification scheduling specifics), the exact SQLite DDL, and the phase-by-phase plan — all produced in the planning step that follows approval of this design.
