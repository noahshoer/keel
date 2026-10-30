# React frontend, Plan 2: backlog

Carried over from the React foundation (plan `2026-10-29-react-foundation.md`, merged 2026-10-30). This is a backlog, not yet a task-by-task plan; turn it into one before starting.

## Important

- **Done in Plan 2:** generated return types, `describe()`, Today's missing pieces, and these minors: `check-bindings.sh` trap and HEAD diff; the mock's copies, reset per test, unknown ids, task_add/task_update fields, dead `todayTaskIds`, completed views; `initCore` memoised; Shell and Today loading twice. Plan 2 also ported lists, sidebar, search and long press.
- ~~**Generated return types.**~~ `web/src/ipc/commands.ts` hand-writes each command's return type. Generate them from Rust (ts-rs or a `commands.json` extension) so a changed return shape fails `tsc`.
- ~~**`describe()` can throw.**~~ `web/src/diag.ts` and `crates/ui/src/diag.rs` throw on circular objects and BigInt, so logging such a value loses the log line. Fix both.
- ~~**Today's missing pieces.**~~ Due-chip colours, the Overdue split, and the all-done state need `dueBucket` / `allDone` from `keel-core` (exported through `crates/core-wasm`) plus core grouping. No date logic in TypeScript.
- **Remaining screens.** Task view, editor and quick add, Settings, the drawer's swipes and the other gestures, then the cutover (`tauri.conf.json` → `web/dist`, remove `crates/ui`).

## Minor (from the task reviews)

- `check-deps.ts`: deps + devDeps spread hides a duplicate's version; optional/peer deps and `pnpm.overrides` unchecked; tests lack passing-case and version-form tables; parsed JSON is cast without validation.
- Biome: `useLiteralKeys` is off repo-wide for one `process.env` access (scope it); note that `noFloatingPromises` is nursery.
- Vetting table only partly covers maintainers/downloads/ownership.
- `check-bindings.sh`: only runs `--test ts_bindings`; leaves bindings deleted on cargo failure (no trap); diffs against the index, not HEAD. The export list is hand-maintained and can miss a new type. `Due.recurrence` is `?: Recurrence | null`.
- `commands_list.rs` parses `generate_handler!` naively; `write_index` duplicated from the core test; plain `cargo test` rewrites `web/src/bindings/commands.json`.
- Mock backend: fixtures returned by reference, unknown ids ignored, module state leaks between tests, some `task_add`/`task_update` fields ignored, dead `todayTaskIds`, completed views don't reflect dispatch.
- `initCore` isn't memoised; `/// reference types="node"` in `src/test/wasm.ts` leaks Node types into the app; `pnpm test`/`dev` always run a cargo release build first.
- `rebuild-loop.ts`: `run()` clears a pending debounce (one wasted rebuild; `whenIdle` can resolve early); an in-flight build-wasm child isn't tracked.
- Shell and Today both call `list_projects` / `today_view`.
- CI: weekly scan failures only show in the Actions tab; apt packages uncached; WebKitGTK package list unproven on a real runner.

## From the Plan 2 final review (deferred minors)

- `GlassLog.watch` / `GlassKeyboard` observe even while logging is off; `Diag`'s doc says nothing is observed then (parked code).
- `commands_list.rs` lists each command's return type by hand; nothing ties it to the fn's real signature.
- Sidebar has one `adding` state (opening one add form closes another; Dioxus had three); `AddSection` keeps its form open after a failed add (Dioxus closes and reloads).
- No CI job builds the React-config Tauri app, and the simulator `cargo build` no longer compiles glass.
- No error boundary: a malformed stored deadline would throw in render (Dioxus skips it).
