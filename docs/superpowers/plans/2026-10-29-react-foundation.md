# React Frontend Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up `web/`, a strict React + TypeScript app with npm supply-chain guardrails, IPC types from Rust (ts-rs), `keel-core` through wasm, and Tauri dev wiring with hot reload. Prove it with one real screen (the app shell + Today, read-only) running beside the Dioxus UI.

**Architecture:** The Tauri shell and the Rust crates stay. `web/` is a Vite app that calls the existing commands through a typed `invoke` whose argument and return types are generated from Rust. `keel-core` functions the UI needs in-process are exported from a new `crates/core-wasm`. The Dioxus UI remains the shipped frontend until the cutover plan (Plan 7 in the Roadmap).

**Tech Stack:** Node 24 LTS, pnpm 10 (Corepack), TypeScript 5.9+, React 19, Vite 7, Vitest + @testing-library/react + jsdom, Biome 2, ts-rs 11, wasm-bindgen 0.2.129 (+ matching `wasm-bindgen-cli`).

**Spec:** `docs/superpowers/specs/2026-10-29-react-frontend-design.md`

## Global Constraints

- App source (`web/src/`) is `.ts` / `.tsx` only (`allowJs: false`). JavaScript elsewhere is fine where it's the natural fit (the existing `scripts/sim-checks/*.js`, generated glue).
- `tsconfig`: `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noImplicitOverride`, `noPropertyAccessFromIndexSignature`, `noFallthroughCasesInSwitch`, `verbatimModuleSyntax`, `allowJs: false`. No `any` (Biome `noExplicitAny: error`), no non-null `!` (`noNonNullAssertion: error`), no `@ts-ignore` / `@ts-expect-error` without a reason comment.
- Scripts in `web/scripts/` are `.ts`, run with plain `node` (Node 24 type stripping; `erasableSyntaxOnly: true`). No `tsx`, `ts-node`, `npx`, `pnpm dlx`.
- pnpm settings exactly as in the spec: `minimumReleaseAge: 10080`, `strictDepBuilds: true`, `trustPolicy: no-downgrade`, `blockExoticSubdeps: true`, `saveExact: true`, `engineStrict: true`.
- Every version picked at install time is the newest one at least 7 days old, pinned exact.
- IPC arguments are snake_case (CLAUDE.md); commands stay `#[tauri::command(rename_all = "snake_case")]`.
- `crates/ui` keeps building and shipping unchanged in behaviour until cutover; CI keeps all its current jobs green.
- Public repo: nothing from `web/node_modules`, no tokens, no `.npmrc` auth lines committed.

## Review Focus

1. **Stale generated types:** a Rust type or command changes and `web/src/bindings/` isn't regenerated → CI must fail (Task 3, Task 4 check step).
2. **A dependency that wants to run an install script, or is under 7 days old, arrives transitively** (e.g. a patch bump of a Vite sub-dependency) → `pnpm install` must fail, not warn (Task 1 test `rejects-young-package`).
3. **Command argument names drift** (a Rust parameter renamed, TS still sends the old key) → a compile error, not a runtime "missing field" (Task 4 makes every command take one ts-rs `…Args` struct).
4. **Running in a plain browser** (no `window.__TAURI__`) → the mock backend serves data and nothing throws; inside Tauri the mock is never used (Task 5 tests).
5. **wasm not ready on first render** (quick add parse called during startup) → the app awaits `initCore()` before mounting; calling a core function before init throws a clear error (Task 6 test).

---

### Task 1: Toolchain and npm supply-chain guardrails

**Files:**
- Create: `.nvmrc` (`24`), `web/package.json`, `web/pnpm-workspace.yaml`, `web/deps-allowlist.json`, `web/DEPENDENCIES.md`, `web/scripts/check-deps.ts`, `web/scripts/check-deps.test.ts`, `web/.gitignore`
- Modify: `.gitignore` (add `web/node_modules`, `web/dist`, `web/src/core/pkg`)

**Interfaces:**
- Produces: `pnpm run check:deps` (exit 1 on violation). `checkDeps(pkg: PackageJson, allow: Allowlist): string[]` (error list; empty = pass), exported for tests.

- [ ] **Step 1: Install Node 24 and pnpm on the Mac.** `brew install node@24`, then `corepack enable`. In `web/package.json` set `"packageManager": "pnpm@<newest 10.x ≥7 days old>+sha512.<hash>"` (Corepack verifies the hash), `"engines": { "node": ">=24 <25" }`, `"private": true`, `"type": "module"`.
- [ ] **Step 2: Write `web/pnpm-workspace.yaml`** with the six settings from Global Constraints, `ignoredBuiltDependencies: [esbuild]` (esbuild works from its platform package; its postinstall is only an optimisation) and an empty `onlyBuiltDependencies`.
- [ ] **Step 3: Write the failing tests** in `web/scripts/check-deps.test.ts` (Vitest arrives in Task 2; until then run them with `node --test`, using `node:test` + `node:assert`):
  - `flags a dependency missing from the allowlist`: pkg `{dependencies:{react:"19.2.0", leftpad:"1.0.0"}}`, allowlist `{react:{reason:"UI"}}` → errors include `"leftpad is not in deps-allowlist.json"`.
  - `flags a range version`: `react: "^19.2.0"` → `"react must be pinned exactly (got ^19.2.0)"`.
  - `flags an unused allowlist entry`: allowlist has `vite`, pkg doesn't → `"vite is in deps-allowlist.json but not used"`.
- [ ] **Step 4: Run** `node --test web/scripts/check-deps.test.ts` → FAIL (function not defined).
- [ ] **Step 5: Implement** `checkDeps`; the script's `main` reads `package.json` and the allowlist and exits 1 printing the errors. Allowlist shape: `{ [name: string]: { reason: string } }`, covering `dependencies` and `devDependencies`.
- [ ] **Step 6: Run** the tests → PASS.
- [ ] **Step 7: Write `web/DEPENDENCIES.md`**: the policy from the spec plus the checklist to add a dependency: real need (no 20-line helpers), maintainers and download history, npm provenance present, no install scripts, transitive count (`pnpm why`, `pnpm list --depth Infinity | wc -l` before/after), licence (MIT/Apache/BSD/ISC), then an allowlist entry with the reason.
- [ ] **Step 8: Prove the guardrails bite** (`rejects-young-package`): in a scratch copy of `web/`, `pnpm add <any package published in the last 24 h>` → install refuses on `minimumReleaseAge`; `pnpm add` of a package with a postinstall (e.g. `core-js`) → refuses under `strictDepBuilds`. Paste both outputs into the commit message body. Discard the scratch copy.
- [ ] **Step 9: Commit** `chore(web): Node/pnpm pins and npm supply-chain guardrails`.

### Task 2: Strict TypeScript, Biome, Vitest, and a Vite app that renders

**Files:**
- Create: `web/tsconfig.json`, `web/tsconfig.node.json` (scripts + `vite.config.ts`, `erasableSyntaxOnly`), `web/vite.config.ts`, `web/biome.json`, `web/index.html`, `web/src/main.tsx`, `web/src/App.tsx`, `web/src/App.test.tsx`, `web/src/test/setup.ts`
- Modify: `web/package.json`, `web/deps-allowlist.json`

**Interfaces:**
- Produces: scripts `dev`, `build` (`tsc -b && vite build`), `typecheck` (`tsc -b --noEmit`), `lint` (`biome ci .`), `test` (`vitest run`), `check` (typecheck + lint + test + check:deps). `App` component as the root (`web/src/App.tsx`).

- [ ] **Step 1: Add the dependencies** (newest ≥7 days old, exact; each with an allowlist reason): `react`, `react-dom`; dev: `typescript`, `vite`, `@vitejs/plugin-react`, `vitest`, `@testing-library/react`, `@testing-library/dom`, `jsdom`, `@biomejs/biome`, `@types/react`, `@types/react-dom`, `@types/node`. `pnpm install` must succeed under the Task 1 settings; any newly blocked build script gets an explicit `ignoredBuiltDependencies` entry with a comment, never `onlyBuiltDependencies`, unless the package cannot work without it (then stop and ask).
- [ ] **Step 2: Write the configs.** `tsconfig.json` with the Global Constraints flags plus `jsx: react-jsx`, `moduleResolution: bundler`, `target: ES2022`, `lib: [ES2023, DOM, DOM.Iterable]`, `types: ["vite/client"]`. `biome.json`: recommended rules plus `noExplicitAny`, `noNonNullAssertion`, `noFloatingPromises` as errors; `linter.domains.react: "recommended"` (hooks rules `useExhaustiveDependencies` / `useHookAtTopLevel` included) plus `useComponentExportOnlyModules` (keeps hot reload clean) and `noNestedComponentDefinitions` as errors; formatter 2-space, line width 110. `vite.config.ts`: React plugin; `server.port 5173`, `strictPort: true`; `server.host = process.env.TAURI_DEV_HOST || false`, with HMR on that host at port 1421 when set (Tauri's iOS dev pattern); `build.target: safari18`; Vitest `environment: jsdom`, `setupFiles: ["src/test/setup.ts"]`.
- [ ] **Step 3: Write the failing test** `App.test.tsx` → `renders the app root`: `render(<App />)`, expect `screen.getByText("Keel")`.
- [ ] **Step 4: Run** `pnpm test` → FAIL (no App).
- [ ] **Step 5: Implement** `App` rendering `<div className="app"><h1>Keel</h1></div>` and `main.tsx` mounting it into `#root` (throw a clear error if `#root` is missing; no `!`).
- [ ] **Step 6: Move Task 1's tests to Vitest**: `web/scripts/check-deps.test.ts` switches from `node:test` / `node:assert` to `vitest` (same names and assertions), and Vitest's `include` covers `src/**` and `scripts/**`.
- [ ] **Step 7: Run** `pnpm check` → all green. Then `pnpm dev`, open http://localhost:5173, see "Keel"; edit the heading and confirm it hot-reloads without a page reload.
- [ ] **Step 8: Commit** `feat(web): strict TS + Vite + React + Biome + Vitest scaffold`.

### Task 3: ts-rs types for everything that crosses IPC

**Files:**
- Modify: `crates/core/Cargo.toml` (optional `ts-rs = { version = "11", features = ["chrono-impl", "serde-json-impl"] }`, feature `ts = ["dep:ts-rs"]`), every IPC type in `crates/core/src/{model,commands,recurrence,views,quickadd,reminders,search,filter,calendar,ids}.rs` and `crates/core/src/import/todoist.rs`, the same for `crates/service` and `crates/src-tauri` types that are command arguments or returns
- Create: `.cargo/config.toml` `[env] TS_RS_EXPORT_DIR = { value = "web/src/bindings", relative = true }`, `web/src/bindings/` (generated, committed), `web/src/bindings/index.ts` (re-exports, generated by the export test)

**Interfaces:**
- Produces: one `web/src/bindings/<Type>.ts` per type (`Task`, `Project`, `Section`, `Label`, `Filter`, `Reminder`, `Due`, `Priority`, `Recurrence`, `Action`, `Command`, `ApplyResult`, `DueEdit`, `CardRow`, `Card`, `Progress`, `DueBucket`, `QuickAdd`, `Span`, `SpanKind`, `PlannedReminder`, `ReminderSettings`, `SearchResults`, `FilterGroup`, `MonthView`, `ImportSummary`, `Id`, …) and `import type { Task } from "../bindings"`.

- [ ] **Step 1: Write the failing check**: `crates/core/tests/ts_bindings.rs` (behind `#![cfg(feature = "ts")]`) → `bindings_cover_ipc_types`: calls `<T as ts_rs::TS>::export_all()` for the list above and asserts `web/src/bindings/Task.ts` exists and contains `id: Id` and `content: string` (use the real field names from `model.rs`).
- [ ] **Step 2: Run** `cargo test -p keel-core --features ts --test ts_bindings` → FAIL (no `TS` impls).
- [ ] **Step 3: Derive** `#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]` on each type; serde attributes (`rename_all`, `tag`, `default`, `skip_serializing_if`) must carry through, and fields with `#[serde(default)]` become optional (`#[ts(optional)]`) so TS matches what Rust accepts. `Id` exports as `string` (`#[ts(type = "string")]`). `NaiveDate` → `string`.
- [ ] **Step 4: Run** the test → PASS. Spot-check `Action.ts` against `commands.rs` for one tagged variant and one `#[serde(default)]` field, and `Recurrence.ts` for every variant (append-only per CLAUDE.md).
- [ ] **Step 5: Add `scripts/check-bindings.sh`**: runs the export, then `git diff --exit-code -- web/src/bindings` (fails when Rust changed and bindings weren't regenerated). Run it → exit 0.
- [ ] **Step 6: Run** `cargo test` (without `ts`) → still green; ts-rs is not in the default build.
- [ ] **Step 7: Commit** `feat(core): ts-rs bindings for IPC types`.

### Task 4: One typed args struct per command

**Files:**
- Modify: `crates/src-tauri/src/lib.rs` (all 57 commands), `crates/ui/src/api.rs` (`invoke` helper only)
- Create: `crates/src-tauri/src/args.rs` (the `…Args` structs, derive `Deserialize` + ts-rs `TS`), `crates/src-tauri/tests/commands_list.rs`, `web/src/bindings/commands.json` (generated)

**Interfaces:**
- Consumes: Task 3's `ts` feature (src-tauri gets a `ts` feature forwarding to `keel-core/ts`).
- Produces: every command's signature is `async fn name(args: NameArgs, state…) -> Result<Ret, String>` (`NoArgs` for none); `web/src/bindings/<Name>Args.ts`; `commands.json` = sorted list of command names from `generate_handler!`.

- [ ] **Step 1: Write the failing test** `commands_list.rs` → `exports_command_names`: reads `src/lib.rs`, takes the identifiers inside `generate_handler![…]`, writes them sorted to `web/src/bindings/commands.json`, asserts the count is 57 and that `today_view` and `dispatch` are present (use the real names).
- [ ] **Step 2: Run** `cargo test -p keel --features ts --test commands_list` → FAIL (feature/file missing).
- [ ] **Step 3: Implement**: add the feature, move each command's parameters into `NameArgs` (same snake_case field names as today), keep `State`/`AppHandle` params. Commands that call mobile plugins stay `async` (CLAUDE.md).
- [ ] **Step 4: Keep the Dioxus UI working**: in `crates/ui/src/api.rs`, `invoke` wraps the args as `{ "args": <args> }` before calling `__invoke`. No other Dioxus change.
- [ ] **Step 5: Run** `cargo test`, `cargo test --features ts` (bindings regenerated, `check-bindings.sh` exit 0 after committing them), `cd crates/ui && cargo check --target wasm32-unknown-unknown` (0 warnings), then `cargo tauri dev` with the Dioxus UI: add a task, complete it, open Settings: works as before.
- [ ] **Step 6: Commit** `refactor(tauri): each command takes one typed args struct (ts-rs)`.

### Task 5: Typed IPC client and mock backend

**Files:**
- Create: `web/src/ipc/tauri.d.ts` (declares `window.__TAURI__?.core.invoke`), `web/src/ipc/commands.ts`, `web/src/ipc/invoke.ts`, `web/src/ipc/mock.ts`, `web/src/ipc/fixtures.ts`, `web/src/ipc/ipc.test.ts`

**Interfaces:**
- Consumes: Task 3/4 bindings (`…Args`, return types), `commands.json`.
- Produces:
  ```ts
  export interface Commands { today_view: { args: NoArgs; ret: Task[] }; /* one entry per command */ }
  export type CommandName = keyof Commands;
  export function invoke<K extends CommandName>(cmd: K, args: Commands[K]["args"]): Promise<Commands[K]["ret"]>;
  export type Handlers = { [K in CommandName]: (args: Commands[K]["args"]) => Promise<Commands[K]["ret"]> };
  export const mockHandlers: Handlers; // in-memory, seeded from fixtures.ts
  ```
  `invoke` sends `{ args }` to `window.__TAURI__.core.invoke` when it exists, else calls `mockHandlers` (dev in a browser only: `import.meta.env.DEV`; a production build without Tauri throws `"Keel must run inside its app"`). Errors reject with the backend's string.

- [ ] **Step 1: Write the failing tests** in `ipc.test.ts`:
  - `every command has a client entry`: `Object.keys(COMMAND_TYPES).sort()` equals `commands.json` (export a runtime `COMMAND_TYPES` record keyed by `CommandName` from `commands.ts`, so TS also checks it's exhaustive).
  - `sends args wrapped and snake_case`: stub `window.__TAURI__.core.invoke` with a spy; `await invoke("list_tasks", { project_id: "p1" })` → spy called with `("list_tasks", { args: { project_id: "p1" } })`.
  - `uses the mock without Tauri`: no `__TAURI__`; `await invoke("today_view", {})` returns the fixture's Today tasks.
  - `rejects with the backend message`: spy rejects with `"no such task"` → `invoke` rejects with `"no such task"`.
- [ ] **Step 2: Run** `pnpm test ipc` → FAIL.
- [ ] **Step 3: Implement** the files above. `Handlers` being a mapped type makes a missing mock handler a compile error; give `dispatch` a minimal in-memory `apply` for `TaskAdd`, `TaskComplete` and `TaskUpdate`, and have every other command return its fixture value.
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): typed invoke over ts-rs bindings, with a browser mock backend`.

### Task 6: `keel-core` in the browser through wasm

**Files:**
- Create: `crates/core-wasm/{Cargo.toml,src/lib.rs}` (workspace member, `crate-type = ["cdylib"]`, deps: `keel-core`, `wasm-bindgen = "=0.2.129"`, `serde-wasm-bindgen`, `getrandom` with `wasm_js` as in `crates/ui/Cargo.toml`), `web/scripts/build-wasm.ts`, `web/src/core/index.ts`, `web/src/core/core.test.ts`, `web/src/test/wasm.ts` (loads the `.wasm` with `initSync` for Vitest)
- Modify: `Cargo.toml` (workspace members), `web/package.json` (`build:wasm`; `dev`, `build` and `test` run it first), `web/src/main.tsx` (await `initCore()` before mounting)

**Interfaces:**
- Produces from `web/src/core/index.ts`: `initCore(): Promise<void>`, and typed wrappers whose types come from the bindings: `parseQuickAdd(text: string, today: string, projects: Project[], sections: Section[], labels: Label[]): QuickAdd`, `dueLabel(due: Due, today: string): string`, `timeLabel(…)`, `dueBucket(…)`, `cards(tasks: Task[], …): Card[]`, `progress(…)`, `monthView(…)`, `shiftMonth(…)`, `recurrenceText(rule: Recurrence): string`. Match each parameter list to the Rust function it calls (`keel_core::quickadd::parse`, `views::due_label`, etc.). This plan exports only what Task 7 uses (`cards`, `dueLabel`, `timeLabel`, `progress`); later plans add the rest the same way.
- Rust side: each export uses `#[wasm_bindgen(unchecked_param_type = "…", unchecked_return_type = "…")]` naming the ts-rs type, so the generated `.d.ts` is typed, not `any`.

- [ ] **Step 1: Write the failing tests** `core.test.ts`:
  - `due label matches Rust`: build a `Due` for 2026-10-30 with today 2026-10-29 → `dueLabel(...)` equals what `keel_core::views::due_label` returns in its own tests for the same input (copy the expected string from `crates/core/src/views.rs` tests).
  - `cards hide sub-tasks under their parent`: fixture parent + child → one card whose row has `progress` `{ done: 0, total: 1 }` (field names per the `Progress` binding).
  - `throws before init`: in a fresh module context without `initCore`, `dueLabel(...)` throws `"keel core not initialised: await initCore() first"`.
- [ ] **Step 2: Run** `pnpm test core` → FAIL.
- [ ] **Step 3: Install the matching CLI**: `cargo install wasm-bindgen-cli --version 0.2.129 --locked`. `build-wasm.ts` runs `cargo build -p keel-core-wasm --target wasm32-unknown-unknown --release` then `wasm-bindgen --target web --out-dir web/src/core/pkg …`, and fails if `wasm-bindgen --version` ≠ the crate's pinned version.
- [ ] **Step 4: Implement** the four exports and `index.ts`: wrappers check an `initialised` flag, then call the pkg function. Nothing outside `web/src/core/` imports `pkg/` (Biome `noRestrictedImports`).
- [ ] **Step 5: Run** `pnpm check` → green; `cargo test` → green.
- [ ] **Step 6: Commit** `feat(core-wasm): typed wasm exports of keel-core for the web UI`.

### Task 7: Tauri dev wiring, diagnostics, and the first real screen

**Files:**
- Create: `crates/src-tauri/tauri.react.conf.json`, `web/src/styles/app.css` (moved verbatim from the CSS string in `crates/ui/src/style.rs`), `web/src/assets/fonts/*.woff2` (decoded from the base64 in `style.rs`), `web/src/diag.ts`, `web/src/screens/Today.tsx`, `web/src/screens/Today.test.tsx`, `web/src/shell/Shell.tsx`
- Modify: `web/src/App.tsx`, `CLAUDE.md` (a "React frontend (in progress)" section with the dev commands)

**Interfaces:**
- Consumes: `invoke` (Task 5), `cards` / `dueLabel` / `timeLabel` / `progress` (Task 6).
- Produces: `cargo tauri dev --config tauri.react.conf.json` runs the React app with HMR; `cargo tauri ios dev --config tauri.react.conf.json` the same on the phone over Wi-Fi. `diag.log(msg: string): void` with the same command, `source` and capture hooks as `crates/ui/src/diag.rs` (JS errors, unhandled rejections, `console.error`), installed only when diagnostics are on.

- [ ] **Step 1: Write the failing test** `Today.test.tsx` → `shows today's cards from the backend`: with the mock backend, `render(<Today />)`, `await screen.findByText(<a fixture task's content>)`; a fixture sub-task's content is not shown, and its parent shows `"0/1"`; the root has class `today` and each row class `task-row` (the class names `Demo.swift` and the sim checks use).
- [ ] **Step 2: Run** `pnpm test Today` → FAIL.
- [ ] **Step 3: Implement** `Shell` (the sidebar and main layout, with the same class names as `crates/ui/src/main.rs` and `sidebar.rs`; nav items are not interactive yet) and `Today` (loads `today_view`, `list_projects`, renders `cards(...)` with titles and due labels; checkboxes are not interactive yet). Styles come from `app.css`; fonts load as files through Vite, so the base64 data-URL workaround isn't carried over.
- [ ] **Step 4: Write `tauri.react.conf.json`**: `build.devUrl: "http://localhost:5173"`, `build.beforeDevCommand: "pnpm --dir ../../web dev"`, `build.frontendDist: "../../web/dist"`, `build.beforeBuildCommand: "pnpm --dir ../../web build"`.
- [ ] **Step 5: Run** `pnpm check` → green. Then `cargo tauri dev --config tauri.react.conf.json`: Today shows the real tasks from `keel.db`; editing `Today.tsx` updates the window in place without restarting Rust. Then on the phone, `cargo tauri ios dev --config tauri.react.conf.json`: Today renders and an edit hot-reloads over Wi-Fi. Set diagnostics on, pull with `scripts/diag.sh`, and find a `diag.log` line from `web/src/diag.ts`.
- [ ] **Step 6: Commit** `feat(web): Tauri dev wiring with HMR, diagnostics, and a read-only Today`.

### Task 8: CI for `web/`

**Files:**
- Modify: `.github/workflows/ci.yml`
- Create: `.github/workflows/web-supply-chain.yml` (weekly scheduled run of audit + OSV against `main`)

**Interfaces:**
- Consumes: `pnpm check`, `scripts/check-bindings.sh`, `build:wasm`.

- [ ] **Step 1: Pin every action by SHA** (existing ones too) with the version tag in a trailing comment.
- [ ] **Step 2: Add a `web` job (Ubuntu)**: `actions/setup-node` with `node-version-file: .nvmrc`, `corepack enable`, Rust stable with the `wasm32-unknown-unknown` target, `cargo install wasm-bindgen-cli --version 0.2.129 --locked` (cached), `pnpm install --frozen-lockfile`, `pnpm check`, `pnpm build`, `scripts/check-bindings.sh`, `pnpm audit --audit-level moderate`, and OSV-Scanner (`google/osv-scanner-action`, SHA-pinned) on `web/pnpm-lock.yaml`.
- [ ] **Step 3: Leave the Windows and iOS jobs** building the Dioxus UI as today (they move to `web/` in Plan 7).
- [ ] **Step 4: Prove the gates on a throwaway branch**, three commits each expected red: change a field in `model.rs` without regenerating bindings; add `"leftpad": "^1.0.0"` to `package.json`; add `const x: number = "a"` to a `web/src/` file. Push, confirm the `web` job fails each time for the right reason, then delete the branch.
- [ ] **Step 5: Commit** `ci: web job with type, lint, test, bindings and supply-chain gates`.

---

## Roadmap after this plan

Each item gets its own plan in `docs/superpowers/plans/` when it starts. Every port moves its Rust UI tests to Vitest, or into `keel-core` when the logic is domain logic, and keeps the contracts listed in the spec.

2. **Lists and rows:** `task_row`, `cards`, `completion`, `sidebar`, `search`, `filters`, the other views, theme / dark mode, icons. Completing tasks works.
3. **Editor and quick add:** `editor`, `pickers`, live highlighting. `repeat_text`, `recurrence_for`, `changes`, `labels_to_create` and `new_task_from` move into `keel-core` and are exported through `core-wasm`.
4. **iPhone glass: deferred** (2026-10-30, Liquid Glass parked; see the spec). Comes back after parity and Noah's web UI updates. `platform` was ported in Plan 2.
5. **Task view and gestures:** `task_view/*`, `gestures` (the 17 embedded scripts become TS modules with Vitest tests, including today's keyboard pace and sheet rules).
6. **Settings, import, reminders and diagnostics UI.**
7. **Cutover:** `frontendDist` → `web/dist`; check `scripts/sim-checks/*.js` still find what they query (`Demo.swift`'s `@fab` / `@menu` / `@search` targets are glass-only; page scripts run through `KEEL_SCRIPT`); drop the `glass` feature from `tauri.conf.json`; the Windows and iOS CI jobs build `web/`; delete `crates/ui`; update CLAUDE.md and the `/deploy-keel` steps. Full pass on the phone, including the open phone checks listed in CLAUDE.md.

## As built (2026-10-30)

Deviations from this plan:

- pnpm 11.28.9 instead of 10: owner's choice; pnpm 11 replaced `ignoredBuiltDependencies`/`onlyBuiltDependencies` with `allowBuilds` (the esbuild entry was later dropped, as esbuild isn't installed).
- 29 IPC commands instead of 57: that is how many the app registers (`commands_list.rs`).
- `typecheck` is `tsc -b`: `--noEmit` fails with TS6310 on project references.
- TS 7, Vite 8 and Vitest 5: the newest versions at least 7 days old (`minimumReleaseAge`).
- The wasm-bindgen CLI is project-local in `.tools/`: owner's choice, no global install.
- `dueLabel` takes YYYY-MM-DD, with `dueDateOf`/`dueTime` wrappers for the due value: keeps the core rule in one place.
- Today uses Grouped mode like the Dioxus UI: matches existing behavior.
- `pnpm dev` runs `scripts/dev.ts` (Vite plus `watch-wasm.ts`): one command gives HMR and wasm rebuilds.
- Task 8 was verified locally with no push: CI wasn't run on it.
- The no-JS check was dropped: the owner relaxed that rule.
- Command return types are hand-written in `commands.ts`: generate them in Plan 2.
