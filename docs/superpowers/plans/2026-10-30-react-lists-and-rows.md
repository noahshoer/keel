# React Plan 2: Lists and rows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every list screen of the Dioxus app works the same way in the React app: all views, the sidebar with its management forms, search, completing and reopening tasks, streaks, the drawer and the theme.

**Architecture:** One app-wide loader (`web/src/app/load.ts`) fetches what the current view needs, as the Dioxus `App` resource does. Screens render it with the Dioxus markup and class names. Every decision about tasks (which date group, the chip colours, overdue, deadline urgency, reminder text) is a `keel-core` function exported through `crates/core-wasm`. Mutations are `Command`s built in `web/src/ipc/actions.ts` and sent through `dispatch`, followed by a reload.

**Tech Stack:** React 19, TypeScript (strict), Vitest + Testing Library (`fireEvent`; no new dependencies), Rust `keel-core` + wasm-bindgen, ts-rs.

**Spec:** `docs/superpowers/specs/2026-10-29-react-frontend-design.md`, plus the backlog `docs/superpowers/plans/2026-10-30-react-plan-2-backlog.md` and the foundation plan's roadmap item 2 (`2026-10-29-react-foundation.md`).

## Decisions (Noah, 2026-10-30)

- **Behavior parity with the Dioxus app** is the bar for the whole port. A difference is a bug unless this plan lists it.
- **Liquid Glass is parked.** The React app never calls `plugin:glass|*` and has no glass module. It always takes the paths the Dioxus UI takes when `Glass.native` is false (web editor, web menus, the header's menu button, inline forms). The glass code stays in the repo, unused by this path. The Dioxus UI keeps its glass until the cutover.
- **The React build doesn't load the glass plugin at all, so no overlay is attached.** What the app needs from Swift moves into a new plugin scoped to just that, `crates/plugin-ios` (`tauri-plugin-keel-ios`, plugin name `keel-ios`; the name is my call): the diagnostics log (`Diag.swift`), keyboard pace (`window.keelKeyboard`), and the sim-check script runner (the `js:` part of `Demo.swift`). `tauri-plugin-glass` becomes an optional dependency behind the `keel` crate's `glass` feature. Only the Dioxus config turns it on (Task 1). Roadmap item 4 (iPhone glass) is deferred until Noah is comfortable with the iOS app on the web UI, which comes after parity and his own web UI updates.
- So "parity" on the iPhone means parity with the Dioxus app's no-glass behavior (what it does in a 375px browser window).

## Global Constraints

- App source is `.ts`/`.tsx`, strict, no `any`. Biome clean; `pnpm check` green.
- Data rules live in `keel-core` with Rust tests. TypeScript never compares a task's date to today, groups tasks, or builds date text. See CLAUDE.md, Conventions, "Where logic lives".
- Keep the Dioxus markup and class names (`app.css` applies unchanged; `Demo.swift` and `scripts/sim-checks/*.js` query them).
- CSS changes go in both `web/src/styles/app.css` and `crates/ui/src/style.rs` until the cutover. This plan should need none.
- No new npm dependencies.
- `Action` variants and serialized shapes are unchanged. Every IPC type comes from ts-rs bindings.
- Commit per task on branch `react-lists`; merge `--no-ff` to `main` at the end.

## Review Focus

1. **The split must not lose anything Swift does today.** In the React build on the phone, the diag log still gets Swift, Rust and page lines, `window.keelKeyboard` still fires, and `scripts/sim-check.sh` still runs its scenario. In the Dioxus build, the glass card, buttons and menus still work, and their log lines still land in the same `diag.log` (Task 1 checks; Task 11 phone pass).
2. **Tapping the circle again during completion.** A tap during the fill cancels; a tap during the switch-off is ignored; a failed save brings the row back unchecked (Task 6 tests).
3. **A repeating task completed in a project view** stays listed with its next date. The row must come back switched on when the new due arrives, not stay hidden (Task 6 test).
4. **Stale async results.** A slow load for an earlier view, a stale filter check or a stale search must never replace a newer one (Tasks 5, 8 and 9 tests).
5. **iOS keyboard for inline forms.** iOS shows the keyboard only for a focus inside the user's tap. Forms opened by a tap (rename, add project, label, section or filter) focus their input synchronously in the click handler (`flushSync` then `ref.focus()`), not through `autoFocus` after a render. This is my call and a small improvement over Dioxus, whose fields got no keyboard there (Task 8 test: the input is `document.activeElement` right after the click).

## Files

| Path | Responsibility |
|---|---|
| `crates/plugin-ios/` | new scoped plugin: Swift diag log, keyboard pace, `KEEL_SCRIPT` runner |
| `crates/src-tauri/{Cargo.toml,build.rs,capabilities/}` | glass behind the `glass` feature, capabilities split by feature |
| `crates/src-tauri/tests/commands_list.rs` | also writes `Commands.ts` (args + return type per command) |
| `crates/core/src/views.rs`, `crates/core/src/reminders.rs` | `date_groups`, `upcoming_tasks`, `deadline_urgent`, `reminder_label`, `offset_text` (moved from the Dioxus crate) |
| `crates/core-wasm/src/lib.rs` | exports for the above + `dueBucket`, `allDone` |
| `web/src/core/index.ts` | typed wrappers |
| `web/src/ipc/actions.ts` | `command()` + one function per mutation |
| `web/src/ipc/mock.ts`, `fixtures.ts` | dispatch applies every action this plan sends; per-test reset |
| `web/src/app/view.ts` | `View` type and per-view rules (title, mode, empty text, …) |
| `web/src/app/load.ts` | the app-wide loader |
| `web/src/app/App.tsx` (replaces `src/App.tsx`) | view, reload, drawer, show-completed and theme state; layout |
| `web/src/lists/{TaskRow,CardList,CompletedRow,StreakPanel}.tsx` | rows and cards |
| `web/src/lists/completion.ts` | completion phases, timings, icon preference |
| `web/src/screens/{MainPane,SectionBlock,SearchPane}.tsx` | the main pane per view |
| `web/src/shell/{Sidebar,ManagedRow,FilterForm}.tsx`, `theme.ts`, `platform.ts` | sidebar and its forms |
| `web/src/gestures/long-press.ts` | touch long press → `.pressed` |

`Shell.tsx`, `Today.tsx` and `today-data.ts` from Plan 1 are replaced by these and deleted (Task 7).

---

### Task 1: A scoped iOS plugin; glass only in the Dioxus build

**Files:**
- Create: `crates/plugin-ios/` (`Cargo.toml` `tauri-plugin-keel-ios`, `build.rs`, `src/lib.rs`, `permissions/`, `ios/Package.swift`, `ios/Sources/KeelIosPlugin/{KeelIosPlugin,Diag,Keyboard,Script}.swift`)
- Move: `Diag.swift` and `Keyboard.swift` from `crates/plugin-glass/ios/Sources/GlassPlugin/` (`git mv`, so their history follows)
- Create: `crates/plugin-glass/ios/Sources/GlassPlugin/GlassLog.swift`
- Modify: the glass Swift files that call `Diag.log`/`Diag.attach`/`KeyboardPace` (`GlassPlugin.swift`, `CardModel.swift`, `PageGlass.swift`), `crates/plugin-glass/src/lib.rs` + `build.rs` (drop `diag`), `crates/src-tauri/{Cargo.toml,build.rs,src/lib.rs,src/diag.rs}`, `crates/src-tauri/capabilities/` (split), `crates/src-tauri/tauri.conf.json`, `tauri.react.conf.json`, `scripts/sim-check.sh`, `Cargo.toml` (workspace members)

**Interfaces:**
- `tauri_plugin_keel_ios::{init, diag(app, enabled, source, line), diag_fresh(app)}`: the same signatures `tauri_plugin_glass::diag`/`diag_fresh` have today, which `src-tauri/src/diag.rs` switches to. Swift command `diag` (same `DiagArgs`). The plugin's `load(webview:)` runs `Diag.attach(webview)`, `KeyboardPace.attach(webview)` and `Script.run(webview)`, and nothing else.
- `Script.run`: evaluates `KEEL_SCRIPT` (plain page JS) once the page has loaded, with the timing `Demo.swift` uses for `js:` today. `sim-check.sh` sets `SIMCTL_CHILD_KEEL_SCRIPT` instead of `KEEL_GLASS_DEMO="js:…"`. Glass's `Demo.swift` keeps its selector and card modes unchanged.
- Glass logs without depending on the new package: `GlassLog.log(_ msg: String)` posts `Notification.Name("keel.diag.line")` with `["line": msg, "source": "swift"]`, and `Diag` observes it. Separate SwiftPM packages, no path dependency between them. The overlay `HOST frame` lines `Diag.attach(…, hosting:)` writes today move into glass, through `GlassLog`.
- `keel` crate: `[features] glass = ["dep:tauri-plugin-glass"]`, not default; `.plugin(tauri_plugin_glass::init())` under `#[cfg(feature = "glass")]`; `.plugin(tauri_plugin_keel_ios::init())` always. `tauri.conf.json` `build.features: ["glass"]`; `tauri.react.conf.json` `build.features: []`. Capabilities: `capabilities/base/default.json` (`core:default`, `notification:default`, `keel-ios:default`) and `capabilities/glass/glass.json` (`glass:default`). `build.rs` passes `tauri_build::Attributes::new().capabilities_path_pattern(…)`: `"./capabilities/**/*"` when `CARGO_FEATURE_GLASS` is set, else `"./capabilities/base/*"`.
- CI keeps building what it builds today (`tauri.conf.json`, so glass on). Plain `cargo test` builds without glass.

- [ ] **Step 1: Failing check.** In `crates/src-tauri/tests/commands_list.rs` add `glass_is_opt_in`: it reads `Cargo.toml` and `tauri.react.conf.json` and asserts that `tauri-plugin-glass` is `optional = true`, `glass` isn't in `default`, and the React config's `build.features` is `[]`. Also `react_capabilities_skip_glass`: no file under `capabilities/base/` mentions `glass:`.
- [ ] **Step 2: Run** `cargo test -p keel --test commands_list` → FAIL.
- [ ] **Step 3: Implement** the Rust side (feature, capabilities split, `build.rs`, `diag.rs` → `tauri_plugin_keel_ios`). Check that the merge patch empties the features: `cargo tauri build --no-bundle --debug --config tauri.react.conf.json -v` shows no `--features glass`, and plain `cargo tauri build --no-bundle --debug` does.
- [ ] **Step 4: Implement** the Swift side: the new package, the moved files, `GlassLog`, and `sim-check.sh`.
- [ ] **Step 5: Run** `cargo test`, `cargo tauri build --no-bundle --debug`, and `cargo tauri ios build --target aarch64-sim --debug` once with each config → all build.
- [ ] **Step 6: Simulator, React build:** install it, then `scripts/sim-check.sh <udid> scripts/sim-checks/<an existing scenario>.js`. The `[check]` lines print (the script runner works), and the pulled `diag.log` has Swift, Rust and page lines but no `HOST frame` or `GLASS` lines (no overlay). `xcrun simctl spawn <udid> log show --predicate 'process == "Keel"'` shows no glass plugin load.
- [ ] **Step 7: Simulator, Dioxus build:** the same scenario passes, and `KEEL_GLASS_DEMO` opens the glass card (`@fab`). `diag.log` has `GLASS`/`HOST frame` lines from `GlassLog` next to the others.
- [ ] **Step 8: Update** the CLAUDE.md lines naming `Demo.swift`/`KEEL_GLASS_DEMO` for the script runner, and the layout table (new crate). **Commit** `feat(ios): scoped keel-ios plugin for diag, keyboard pace and scripts; glass behind a feature`.

### Task 2: Generated command return types

**Files:**
- Modify: `crates/src-tauri/tests/commands_list.rs`, `web/src/ipc/commands.ts`, `scripts/check-bindings.sh`
- Create (generated): `web/src/bindings/Commands.ts`

**Interfaces:**
- Produces: `web/src/bindings/Commands.ts` exporting `interface Commands { <cmd>: { args: <Args>; ret: <Ret> } }` for all 29 commands. `commands.ts` re-exports it (`export type { Commands } from "../bindings/Commands"`) and keeps `CommandName`, `COMMAND_TYPES` and `Handlers` built on it.

- [ ] **Step 1: Write the failing check.** In `web/src/ipc/ipc.test.ts`, add `it("return types are generated")`: `expectTypeOf<Commands["today_view"]["ret"]>().toEqualTypeOf<Task[]>()`, `…["dispatch"]["ret"]` → `ApplyResult`, `…["check_filter"]["ret"]` → `null`, and `Commands` is imported from `../bindings/Commands`.
- [ ] **Step 2: Run** `pnpm typecheck` → FAIL (no `bindings/Commands`).
- [ ] **Step 3: Extend `command_args!`** to `cmd => Args -> Ret`. It writes `Commands.ts` with `import type` lines for every named type and one member per command, using `<Ret as TS>::name()` (`Vec<T>` → `Array<T>`; `()` → `null`). Each `Ret` is the `T` of the command's `Result<T, String>` in `lib.rs`/`diag.rs`. Remove the hand-written interface from `commands.ts`. Add `Commands.ts` to `index.ts` the same way the other files are added.
- [ ] **Step 4: Backlog fixes in `check-bindings.sh`:** a `trap` restores `web/src/bindings` from `HEAD` (`git checkout HEAD -- "$dir"`) when cargo fails, and the staleness diff compares against `HEAD` (`git diff --exit-code HEAD -- "$dir"`).
- [ ] **Step 5: Run** `scripts/check-bindings.sh` (commit the regenerated files), then `pnpm check` → green. Change one `ret` in Rust locally (`today_view` → `Vec<Project>`), and `pnpm typecheck` must fail in `Today`. Revert.
- [ ] **Step 6: Commit** `feat(ipc): generate command return types from Rust`.

### Task 3: `describe()` survives any value

**Files:**
- Modify: `web/src/diag.ts`, `web/src/diag.test.ts`, `crates/ui/src/diag.rs` (its JS hook string)

**Interfaces:**
- Produces: `describe(x: unknown): string` (exported for tests). It never throws: a circular reference prints `[Circular]`, a BigInt prints `123n`, an `Error` prints its `stack` or `message`, and anything else that fails to serialize prints `String(x)`.

- [ ] **Step 1: Failing tests** in `diag.test.ts`: `describe(circular)` contains `"[Circular]"`; `describe(10n)` is `"10n"`; `describe({ n: 10n })` contains `"10n"`; `describe(new Error("boom"))` contains `"boom"`; `console.error(circular)` with logging on still sends one `console.error` line.
- [ ] **Step 2: Run** `pnpm vitest run diag` → FAIL.
- [ ] **Step 3: Implement** with a `JSON.stringify` replacer holding a `WeakSet` (seen → `"[Circular]"`, bigint → `` `${v}n` ``) inside a `try`/`catch` falling back to `String(x)`. Port the same function into the Dioxus hook string in `diag.rs`, so the two stay identical, as the spec requires.
- [ ] **Step 4: Run** `pnpm vitest run diag` → PASS; `cd crates/ui && cargo check --target wasm32-unknown-unknown` → 0 warnings.
- [ ] **Step 5: Commit** `fix(diag): describe() never throws (circular, BigInt)`.

### Task 4: List rules move into keel-core

**Files:**
- Modify: `crates/core/src/views.rs`, `crates/core/src/reminders.rs`, `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts`, `web/src/core/core.test.ts`, `crates/core/tests/ts_bindings.rs` (export `DateGroup`)
- Modify (Dioxus calls the moved code): `crates/ui/src/main.rs` (drop `date_groups`, `short_date`, `weekday`, `MONTHS`, `WEEKDAYS` where now unused; move `upcoming_groups_cards_per_day` to core), `crates/ui/src/pickers.rs` (`offset_text`, `ReminderDraft::label` delegate to core), `crates/ui/src/task_row.rs` (deadline class from `deadline_urgent`)

**Interfaces (Rust, `keel_core::views`):**
- `pub struct DateGroup { pub title: Option<String>, pub muted: String, pub tasks: Vec<Task> }` (serde + `TS`)
- `pub enum DateGroupMode { Today, Upcoming }` (serde + `TS`, serialized as `"Today"` / `"Upcoming"` like `CardMode`'s `"Grouped"`)
- `pub fn date_groups(mode: DateGroupMode, tasks: Vec<Task>, today: NaiveDate) -> Vec<DateGroup>`: the Dioxus `date_groups` body and text, unchanged ("Overdue"; Today's "Today" + "30 Oct · Friday" only when split; Upcoming's "30 Oct" + "Today · Friday" / "Tomorrow · Saturday" / "Sunday")
- `pub fn upcoming_tasks(today_tasks: Vec<Task>, window: Vec<Task>, today: NaiveDate) -> Vec<Task>`: the overdue ones from `today_tasks`, then `window` (the Dioxus loader's Upcoming branch)
- `pub fn deadline_urgent(deadline: NaiveDate, today: NaiveDate) -> bool`: `deadline <= today`
- `keel_core::reminders::offset_text(minutes: i64) -> String` and `reminder_label(r: &Reminder, today: NaiveDate) -> String`: the `pickers.rs` text ("At time of task", "1 hour before", "2 days before", "Tuesday 9 AM", fallback "Reminder")

**Interfaces (TS, `web/src/core`):** `dueBucket(due: string, today: string): DueBucket`, `allDone(p: Progress): boolean`, `dateGroups(mode: DateGroupMode, tasks: Task[], today: string): DateGroup[]`, `upcomingTasks(todayTasks: Task[], window: Task[], today: string): Task[]`, `deadlineUrgent(deadline: string, today: string): boolean`, `reminderLabel(r: Reminder, today: string): string`. Plus backlog: `initCore()` memoises its promise (two concurrent calls run `init()` once).

- [ ] **Step 1: Failing Rust tests** in `views.rs`/`reminders.rs`: move `upcoming_groups_cards_per_day` from `crates/ui/src/main.rs`; add `today_splits_overdue_only_when_there_is_some` (no overdue → one group, `title: None`, `muted: ""`; with overdue → `"Overdue"`, then `Some("Today")` with `"30 Oct · Friday"` for 2026-10-30); `upcoming_labels_today_and_tomorrow`; `upcoming_tasks_puts_overdue_first_and_drops_today` (a task due today in `today_tasks` is not repeated); `deadline_today_is_urgent` (today → true, tomorrow → false); `offset_text` cases 0, 10, 60, 120, 1440, 2880; `reminder_label` for a relative and an absolute reminder.
- [ ] **Step 2: Run** `cargo test -p keel-core` → FAIL.
- [ ] **Step 3: Implement** in core; point the Dioxus callers at it; `cargo test` and the UI `cargo check` are green.
- [ ] **Step 4: Failing TS tests** in `core.test.ts`: one case per new wrapper with the same values as the Rust tests (2026-10-30 as today); `initCore` called twice concurrently → the wasm `init` spy is called once.
- [ ] **Step 5: Export** each function from `core-wasm` (same pattern as `cards`: `unchecked_*_type` naming the bindings), add the wrappers, and regenerate bindings with `scripts/check-bindings.sh`.
- [ ] **Step 6: Run** `pnpm check` → green.
- [ ] **Step 7: Commit** `feat(core): date groups, deadline urgency and reminder text in keel-core, exported to wasm`.

### Task 5: Mutations, mock dispatch, and the app-wide loader

**Files:**
- Create: `web/src/ipc/actions.ts`, `web/src/ipc/actions.test.ts`, `web/src/app/view.ts`, `web/src/app/view.test.ts`, `web/src/app/load.ts`, `web/src/app/load.test.ts`
- Modify: `web/src/ipc/mock.ts`, `web/src/ipc/fixtures.ts`, `web/src/test/setup.ts`

**Interfaces:**
- `actions.ts`: `command(action: Action, temp: boolean): Command` (`uuid: crypto.randomUUID()`, `temp_id: temp ? "tmp-<uuid>" : null`). Then each one returns `Promise<ApplyResult>` via `dispatch`: `completeTask(id)`, `uncompleteTask(id)`, `deleteTask(id)`, `setStreak(id, enabled: boolean, goal: number | null)`, `addProject(name)`, `renameProject(id, name)`, `deleteProject(id)`, `addLabel(name)`, `renameLabel(id, name)`, `deleteLabel(id)`, `addSection(projectId, name)`, `renameSection(id, name)`, `deleteSection(id)`, `addFilter(name, query)`, `updateFilter(id, name, query)`, `deleteFilter(id)`. Their `temp` flags match `crates/ui/src/api.rs`.
- `view.ts`: `type View = { kind: "inbox" } | { kind: "today" } | { kind: "upcoming" } | { kind: "project"; id: Id; name: string } | { kind: "label"; name: string } | { kind: "filter"; id: Id; name: string; query: string } | { kind: "search" }`; `sameView(a, b): boolean`; `viewMode(v): CardMode`; `viewMotion(from, to): "fade" | "push" | "pop"`; `canShowCompleted(v)`; `showsProject(v)`; `viewTitle(v)`; `emptyText(v): [string, string]` (the Dioxus strings verbatim); `UPCOMING_DAYS = 14`. Mapping a view to a `CardMode` stays here as a table: it picks which core mode a screen uses and decides nothing about a task. (My call.)
- `load.ts`: `type Loaded = { view: View; projects; sections; labels; filters; inbox: Project; todayCount: number; reminders: Reminder[]; tasks: Task[]; all: Task[]; completed: Task[]; filterGroups: FilterGroup[]; filterError: string | null }`; `loadView(view: View, showCompleted: boolean, today: string): Promise<Loaded>`, the same calls and branches as the Dioxus `App` resource (Upcoming through `upcomingTasks`; `filter_view` errors become `filterError`); `useLoaded(view, showCompleted, reloadKey): { loaded: Loaded | null; error: string | null }`, which drops any result that isn't for the latest request.
- Mock: `resetMock()` (called in `setup.ts` `beforeEach`) re-clones every fixture, and handlers return clones. Dispatch applies every action above to its state: uncomplete clears `checked`; delete sets `is_deleted` on the sub-tree; streak sets the fields; projects/labels/sections/filters get add/rename/delete. An unknown id rejects with `"not found: <id>"`. Completed views return the mock's checked tasks. Remove the dead `todayTaskIds`. `task_add` keeps every field `Action::TaskAdd` carries.

- [ ] **Step 1: Failing tests.** `actions.test.ts`: `command(..., true)` has a `tmp-` temp id and `false` has `null`; `completeTask("t1")` then `all_tasks` no longer has `t1`, and `completed_today` has it; `deleteTask("nope")` rejects. `view.test.ts`: the Dioxus `motion_tests` (search pushes in and pops out, else fade), `project_views_are_top_level`, `dated_views_group`, and the empty texts. `load.test.ts`: loading Upcoming puts a fixture overdue task first; a bad filter query gives `filterError` and empty `tasks`; `useLoaded` with a slow first and fast second load ends on the second (fake timers + deferred mock).
- [ ] **Step 2: Run** `pnpm vitest run actions view load` → FAIL.
- [ ] **Step 3: Implement** the three modules and the mock changes.
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): mutations, view rules and the app-wide loader`.

### Task 6: Rows, cards and completing

**Files:**
- Create: `web/src/lists/completion.ts` (+ test), `web/src/lists/TaskRow.tsx`, `CardList.tsx`, `CompletedRow.tsx`, `StreakPanel.tsx`, `web/src/lists/lists.test.tsx`
- Modify: `web/src/shell/icons.tsx` (add any icon the rows use that Plan 1 didn't port: target, bell, subtask, trash, check-circle, the three completion marks)

**Interfaces:**
- `completion.ts`: `FILL_MS = 350`, `OFF_MS = 380`; `type CompletionIcon = "wave" | "pine" | "reflection"`, `savedIcon(): CompletionIcon` (localStorage `completion-icon`, default `"wave"`, guarded by try/catch); `type Phase = 0 | 1 | 2`; `nextOnTap(phase): "start" | "cancel" | "ignore"`.
- `TaskRow({ row: CardRow, today, location: string | null, reminders: Reminder[], onChanged(): void })`: the markup of `task_row.rs` `TaskRow` (no glass parts). Due class from `dueBucket` (`due overdue|today|tomorrow|week|later`), the deadline chip `deadline urgent|set` from `deadlineUrgent` with `dueLabel` text, the reminder chip titled with each `reminderLabel` joined by `", "`, labels, `SubtaskCount` with `.done-check` when `allDone`, the streak badge, `.task-actions` (Add sub-task and the body click render without a handler until Plan 5; Streak toggles `StreakPanel`; Delete calls `deleteTask`). Completing follows the Dioxus phases and `done_due` rule exactly.
- `CardList({ cards: Card[], today, locations: Map<Id, string> | null, reminders, onChanged })` and `CardHead`; `CompletedRow({ task, today, location, onChanged })` (filled check reopens via `uncompleteTask`; the "when" text is `dueLabel(localDateOf(completed_at), today)`, where `localDateOf` is a new `clock.ts` conversion from an ISO timestamp to the local `YYYY-MM-DD`); `StreakPanel({ task, onChanged })`.

- [ ] **Step 1: Failing tests** (fake timers). The circle click adds `completing`, then `switch-off` after 350 ms, and `complete` is dispatched after 730 ms. A second click during the fill cancels, and nothing is dispatched. A click during switch-off is ignored. A failed dispatch puts the row back to class `task`. A repeating task re-rendered with a new `due.date` after completing shows as `task` again. Due chip classes for overdue and today. Deadline `urgent` on today. The reminder chip's `title`. `.done-check` only when all sub-tasks are done. Streak panel: "Track streak" → `setStreak(id, true, null)`. `CompletedRow` reopen calls `uncompleteTask`. `completion.test.ts`: the Dioxus completion tests (default icon, keys round-trip, `FILL_MS >= 300 && OFF_MS >= 300`).
- [ ] **Step 2: Run** `pnpm vitest run lists completion` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): task rows, cards, completing and streaks`.

### Task 7: The main pane for every view

**Files:**
- Create: `web/src/screens/MainPane.tsx`, `web/src/screens/SectionBlock.tsx`, `web/src/screens/MainPane.test.tsx`, `web/src/app/App.tsx`, `web/src/app/App.test.tsx`
- Modify: `web/src/main.tsx`
- Delete: `web/src/App.tsx`, `App.test.tsx`, `web/src/shell/Shell.tsx`, `web/src/screens/Today.tsx`, `Today.test.tsx`, `today-data.ts` (their tests move into the new ones; the Plan 1 class-name assertions are kept)

**Interfaces:**
- `App()`: state `view: View` (start `today`), `lastView` (where closing Search returns), `reloadKey`, `showCompleted`, `drawer: boolean`, `searchQuery` (kept across visits), `theme` (Task 8); `useLoaded(...)`; renders `.app` (+ `drawer-open`), the `.drawer-scrim` (click closes), `Sidebar` (Task 8), and `MainPane`. Picking a view closes the drawer once that view's data has loaded, as in Dioxus. `onChanged` bumps `reloadKey`. Inbox view uses `loaded.inbox`.
- `MainPane({ loaded, today, showCompleted, onToggleCompleted, onOpenDrawer, searchQuery, setSearchQuery, motion, swap, onChanged })`: the Dioxus `MainPane` markup and branches. The view head has the menu button and the Show/Hide completed toggle (`canShowCompleted`); the subtitle is `daySubtitle()` for Today, `"Next 14 days"` for Upcoming and the query (`view-sub mono`) for a filter; `filter-problem big-problem` for a filter error; the empty state; date views as `dateGroups` with `.section-title` heads; filter groups with titles only when there is more than one; project views as unsectioned cards, then `SectionBlock`s, then `AddSection`; the Completed section. The content class alternates `view-in-a`/`view-in-b` per view swap; Search push/pop uses the Dioxus `keelGhost` copy ported as `web/src/screens/ghost.ts` (`ghost(dir)`, `ghostGo()`). The "+ Add task" rows are left out until Plan 3.
- `SectionBlock({ section, tasks, all, today, reminders, onChanged })` with rename (inline form), delete confirm (`Delete section “{name}” and its {n} task(s)?`, the Dioxus wording) and the count of top-level tasks; `AddSection({ projectId, onChanged })`.

- [ ] **Step 1: Failing tests** with the mock: Today shows the Overdue and Today heads when a fixture is overdue; Upcoming shows day heads and the overdue first; a project shows its sections in order with counts; renaming a section through the form changes its head; deleting asks first; a filter with two comma groups shows both group titles; a bad saved filter shows the problem text; Show completed lists the fixture's completed task and the label flips to "Hide completed"; Upcoming and filters have no toggle; an empty Inbox shows "No tasks yet."; completing a row in Today removes it after the reload. Keep Plan 1's checks (`.today` root, `.task-row` rows, a sub-task shown as its parent's `0/1`).
- [ ] **Step 2: Run** `pnpm vitest run MainPane App` → FAIL.
- [ ] **Step 3: Implement**; mount `App` from `main.tsx`.
- [ ] **Step 4: Run** `pnpm check` → green; `pnpm dev`, then walk every view in the browser at 375px and at desktop width, light and dark.
- [ ] **Step 5: Commit** `feat(web): every list view, sections and completed tasks`.

### Task 8: Sidebar, theme and drawer

**Files:**
- Create: `web/src/shell/Sidebar.tsx`, `ManagedRow.tsx`, `FilterForm.tsx`, `theme.ts` (+ test), `web/src/shell/Sidebar.test.tsx`

**Interfaces:**
- `theme.ts`: `type Theme = "system" | "light" | "dark"`; `savedTheme(): Theme`; `applyTheme(t)` (localStorage `theme`, `data-theme` on `<html>`, removed for system); `nextTheme(t)` (System → Light → Dark → System); `themeLabel(t)` (the Dioxus strings). `App` applies the saved theme before first paint, in `main.tsx`.
- `Sidebar({ view, setView, loaded, today, theme, setTheme, onChanged })`: the Dioxus markup. Search, Inbox, Today (with count and the day-number icon), Upcoming; "My Projects", "Labels" and "Filters" each with a + that opens an inline add form (`project-form`/`filter-form`); a new filter opens its view, as in Dioxus. `.nav-item active` for the current view. The Settings button renders and does nothing until Plan 6.
- `ManagedRow({ name, icon, iconClass, active, kind: "project" | "label", menuId, onOpen, onRename, onDelete })`: hover/pressed `.nav-actions` with rename and delete, the inline rename form, and the confirm box with the Dioxus warnings. `FilterRow` uses `FilterForm({ initialName, initialQuery, submitText, onSave, onCancel })`, whose live `check_filter` uses a generation counter so a stale check never wins, and keeps the hint text verbatim.
- Every inline form opened by a tap focuses its input inside the click handler (`flushSync(() => open())` then `inputRef.current?.focus()`). Review Focus 5.

- [ ] **Step 1: Failing tests:** nav clicks change the view and the active class; + Add project → typing a name → Enter dispatches `project_add` and the project appears; the form's input is `document.activeElement` right after the + click; Escape closes; rename and delete (with its confirm) for a project and a label; FilterForm: Save is disabled until the name and a valid query are in, an invalid query shows the backend's message, and a slow stale check that resolves after a newer one doesn't override it; theme cycles through the three titles and sets `data-theme`.
- [ ] **Step 2: Run** `pnpm vitest run Sidebar theme` → FAIL.
- [ ] **Step 3: Implement**; wire into `App` (menu button opens the drawer; the scrim closes it).
- [ ] **Step 4: Run** `pnpm check` → green; browser check at 375px: the menu button opens the drawer, picking a view swaps the page then closes it.
- [ ] **Step 5: Commit** `feat(web): sidebar with project, label and filter management, theme and drawer`.

### Task 9: Search

**Files:**
- Create: `web/src/screens/SearchPane.tsx`, `SearchPane.test.tsx`, `web/src/shell/platform.ts` (+ test), `web/src/app/shortcuts.ts` (+ test)

**Interfaces:**
- `platform.ts`: `detect(platform: string, touchPoints: number, userAgent: string): Platform`, `currentPlatform()`, `searchHint(p)`, `searchTooltip(p)`, ported from `crates/ui/src/platform.rs` with its tests.
- `SearchPane({ query, setQuery, today, showCompleted, loaded, reloadKey, onChanged })`: the `search.rs` markup and texts. Results load in the pane (not the app loader); a result for an older query is dropped.
- `shortcuts.ts`: `isSearchShortcut(ev: KeyboardEvent, typing: boolean): boolean` ("/" outside fields with no modifier; Ctrl/Cmd+K anywhere); `App` installs one `keydown` listener that switches to Search and focuses `.search-input` once it renders.

- [ ] **Step 1: Failing tests:** the empty query shows "Search your tasks" and the platform hint; typing shows "2 tasks" and the rows; no match with completed matches hidden shows the "turn on Show completed" line; Escape clears; the clear button keeps focus in the input; the query survives leaving and returning to Search; a slow earlier search resolving last doesn't replace the newer results; `isSearchShortcut` table ("/" typing → false, Cmd+K typing → true); the `platform.rs` tests.
- [ ] **Step 2: Run** `pnpm vitest run SearchPane platform shortcuts` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): search with shortcuts`.

### Task 10: Long press on touch

**Files:**
- Create: `web/src/gestures/long-press.ts`, `long-press.test.ts`

**Interfaces:**
- `installLongPress(doc: Document): () => void` (returns an uninstall): the `LONG_PRESS_JS` behavior without the glass branch. A 450 ms hold within 10 px on `.task-row`/`.nav-item` adds `.pressed` to the row's `.task` (or the row), touching elsewhere clears it, and the click that follows within 700 ms is swallowed unless it hits `.task-actions`/`.nav-actions`. `contextmenu` is prevented on rows. `App` installs it once.

- [ ] **Step 1: Failing tests** (fake timers, synthetic `TouchEvent`s): a hold adds `pressed`; a move over 10 px cancels it; the click right after is stopped; a click on `.task-actions` goes through; a touch elsewhere clears it.
- [ ] **Step 2: Run** `pnpm vitest run long-press` → FAIL.
- [ ] **Step 3: Implement**; install in `App`.
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): long press shows row actions on touch`.

### Task 11: Parity pass, glass decision in the docs

**Files:**
- Modify: `CLAUDE.md` (React section: what works now; "Liquid Glass is parked" with the Decisions text above), `docs/superpowers/specs/2026-10-29-react-frontend-design.md` (add the 2026-10-30 decisions), `docs/superpowers/plans/2026-10-29-react-foundation.md` (roadmap: item 4 deferred; item 7's cutover notes that `Demo.swift`'s `@fab`/`@menu`/`@search` targets are glass-only), `docs/superpowers/plans/2026-10-30-react-plan-2-backlog.md` (tick what this plan did), this plan's "As built"

- [ ] **Step 1: Desktop:** `cargo tauri dev --config tauri.react.conf.json` against the real `keel.db`. Side by side with `cargo tauri dev` (Dioxus; one at a time, as they share the target), compare every view, complete and reopen a task, track a streak, add, rename and delete a project, label, section and filter, search with Cmd+K, and cycle the theme. Note any difference and fix it, or list it under "As built" with a reason.
- [ ] **Step 2: Phone:** `cargo tauri ios dev --config tauri.react.conf.json`. Complete a task by tapping its circle, open a field so the keyboard rises (`keelKeyboard` lines in the diag log; Review Focus 1), long-press a row to see its actions, open the drawer from the menu button, add a project (does the keyboard rise? Review Focus 5), and search. Diagnostics on: `scripts/diag.sh` shows no JS errors.
- [ ] **Step 3: Write the docs** listed above.
- [ ] **Step 4: Run** `cargo test`, `pnpm check`, `scripts/check-bindings.sh`, UI `cargo check` → all green.
- [ ] **Step 5: Commit** `docs: React lists as built; Liquid Glass parked`, then merge `react-lists` `--no-ff` to `main` and push.

## Left for later plans

- "+ Add task" rows, the add-sub-task button, quick add: Plan 3 (editor).
- Opening a task (row body, card head): Plan 5 (task view). Drawer swipes and the sheet: Plan 5.
- Settings button and dialog, the completion-icon picker: Plan 6.
- Backlog minors not touched here (`check-deps.ts`, Biome scoping, vetting table, `commands_list.rs` parsing, `rebuild-loop.ts`, CI caching) stay in the backlog.

## As built (2026-10-30)

Deviations from this plan:

- `command_args!` uses `cmd => Args => Ret`: Rust forbids `->` after a type fragment.
- `check-bindings.sh` runs keel-core's bindings first: `Commands.ts` checks that its imports exist.
- `ReminderDraft::label` (Dioxus pickers) keeps its own time branch: it labels unsaved drafts; `offset_text` comes from core.
- The mock's `all_tasks` keeps checked tasks, as the backend does, and `project_view` lists the project's open tasks. Fixtures gained an overdue task, a broken filter and per-query search results.
- `.content.today` (a Plan 1 class Dioxus never had) is gone; nothing queried it.
- `MainPane` reads the clock itself and takes the Search pane as its children.
- Glass's card follows the keyboard through its own `GlassKeyboard` (same 0.9 speed), not a cross-package dependency.
- `sim-check.sh` dropped the "no `|`" rule. It needs one screenshot argument, because the log is copied after the last one.
- The add-project and add-label forms close with + (no Escape), as in Dioxus.
- The theme toggle passes an updater: two quick clicks computed both steps from one render (found in the simulator).
- Chrome isn't installed, so the browser walk ran in the iOS simulator against the test database. The Dioxus build matched side by side. Desktop was built with both configs but not driven, because on the Mac that would take the screen.

Checked in the simulator, React build (`scripts/sim-check.sh` scenarios):
- completing a task, then reopening it from Completed;
- long press showing row actions;
- adding a project (its field focused), then deleting it;
- search, the drawer and the theme cycle.

No JS errors in `diag.log`. The React build links only `keel_ios` and `notification`, and its log has no glass lines. The Dioxus build still shows the glass card and buttons, and their lines land in the same log.

Not done here (needs Noah's phone): Task 11 step 2 on the device. That covers taps, keyboard rise for the inline forms, `keelKeyboard` lines and long press, all by hand.
