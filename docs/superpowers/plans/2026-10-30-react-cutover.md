# React cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the React app (`web/`) as Keel's only frontend: Tauri serves `web/dist`, CI builds `web/` everywhere, and `crates/ui` (Dioxus) is deleted.

**Architecture:** The overlay config `tauri.react.conf.json` becomes the base `tauri.conf.json` (dev server, `pnpm dev` / `pnpm build`, `frontendDist` `../../web/dist`, no `glass` feature) and is deleted. The Windows and iOS CI jobs set up Node, pnpm and the wasm-bindgen CLI through one local composite action shared with the `web` job, and build `web/` where they built `crates/ui`. The parked glass plugin stays in the tree behind the `keel` crate's `glass` feature, which nothing turns on.

**Tech Stack:** Tauri v2, GitHub Actions, pnpm 11 / Node 24, wasm-bindgen 0.2.129.

**Spec:** `docs/superpowers/specs/2026-10-29-react-frontend-design.md` (Shape: "Cutover switches `frontendDist` to `web/dist` and deletes `crates/ui`"); roadmap item 7 in `docs/superpowers/plans/2026-10-29-react-foundation.md`.

## Global Constraints

- Behavior parity with the Dioxus no-glass app is the bar; a difference is a bug unless a plan lists it.
- Glass stays parked: `crates/plugin-glass` and the `glass` Cargo feature remain; no config turns the feature on.
- CI stays verification only (nothing signed or deployed); actions pinned by SHA; `pnpm install --frozen-lockfile`; no `npx` / `pnpm dlx`.
- Public repo: nothing from `node_modules`, `dist`, or the phone's data committed.
- `.claude/` is git-ignored: the `/deploy-keel` change is local only.

## Review Focus

1. **`cargo test` on a fresh checkout** fails at `tauri::generate_context!` when `web/dist` is missing → every CI job that compiles the `keel` crate builds `web/` first (Task 2); CLAUDE.md says so (Task 4).
2. **The Xcode build phase** (`cargo tauri ios xcode-script`) reads the base config: after Task 1 a plain `cargo tauri ios build` must bundle the React UI without `--config` (Task 1 Step 3 checks the simulator app shows `#root`, not `#main`).
3. **Windows paths in `build-wasm.ts`** (`.tools/bin/wasm-bindgen.exe`) are exercised by CI for the first time → the Windows job installs the CLI into `.tools` and runs `pnpm build` (Task 2).
4. **Sim-checks querying Dioxus-only or glass-only markup** (`.float-menu`, `.float-search`) silently report `null` → Task 3 runs every scenario against the React build and fixes or removes each one.
5. **A stale `crates/ui` reference** (CI cache paths, `Cargo.toml` exclude, docs) breaks a build or misleads → Task 4 ends with a `git grep` that leaves only provenance comments.

---

### Task 1: Tauri serves the React app

**Files:**
- Modify: `crates/src-tauri/tauri.conf.json` (`build` block)
- Delete: `crates/src-tauri/tauri.react.conf.json`
- Modify: `crates/src-tauri/Cargo.toml` (`glass` feature comment)

- [ ] **Step 1:** Replace `build` in `tauri.conf.json` with the overlay's: `devUrl` `http://localhost:5173`, `beforeDevCommand` `{ "script": "pnpm dev", "cwd": "../../web" }`, `frontendDist` `../../web/dist`, `beforeBuildCommand` `{ "script": "pnpm build", "cwd": "../../web" }`, `features` `[]`. Delete the overlay.
- [ ] **Step 2:** Reword the `glass` feature comment: parked, no config turns it on (enable with `--features glass` when it comes back).
- [ ] **Step 3: Verify.** `cargo test` passes; `cd crates/src-tauri && cargo tauri ios build --target aarch64-sim --debug`, install on the iPhone 17 simulator, launch with `SIMCTL_CHILD_KEEL_SCRIPT` checking `document.querySelector('#root .sidebar') !== null`, expect `true` in the diag log.
- [ ] **Step 4: Commit** `build: Tauri serves web/dist; drop the glass feature from the config`.

### Task 2: CI builds web/ in every job

**Files:**
- Create: `.github/actions/web-setup/action.yml` (composite: setup-node from `.nvmrc`, `corepack enable`, read the wasm-bindgen pin, cache and install the CLI into `.tools`, `pnpm install --frozen-lockfile` in `web/`)
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1:** Move the `web` job's Node / Corepack / wasm-bindgen / install steps into the composite action (`shell: bash` on each run step) and call it from the `web` job. Drop the `crates/ui/dist` placeholder step (the job's `pnpm build` makes `web/dist`).
- [ ] **Step 2:** Windows: remove `dioxus-cli` from install-action and the `crates/ui` cache workspace; replace "Build the web UI" and "UI tests and wasm type-check" with the composite action and `pnpm build` in `web/`. iOS: same swap. Remove `DX_VERSION`; update the header comment.
- [ ] **Step 3: Verify.** Parse the YAML (`ruby -ryaml -e 'YAML.load_file(ARGV[0])'` on both files); after the merge push, all three jobs green (`gh run watch`).
- [ ] **Step 4: Commit** `ci: build the React app in the Windows and iOS jobs`.

### Task 3: Sim-checks against the React build

**Files:**
- Modify: `scripts/sim-checks/drawer-buttons.js` (open the drawer with `.menu-btn`, the no-glass header button)
- Delete: `scripts/sim-checks/float-buttons.js` (glass-only floating buttons; the React app never draws them)

- [ ] **Step 1:** Run each scenario with `scripts/sim-check.sh 53917026-E8EC-4D54-8C70-A657555D8C47 scripts/sim-checks/<name>.js end@<secs>` on the Task 1 build.
- [ ] **Step 2:** Fix a scenario whose selector is Dioxus- or glass-only; a failing behavior (not selector) is a parity bug, fixed in `web/` with a Vitest test first.
- [ ] **Step 3: Verify.** Every remaining scenario prints its `[check]` lines with no `FAILED` and no `null` element.
- [ ] **Step 4: Commit** `test(sim): sim-checks run against the React app`.

### Task 4: Delete crates/ui and update the docs

**Files:**
- Delete: `crates/ui/`
- Modify: `Cargo.toml` (drop `exclude`), `CLAUDE.md`, `docs/notes/build-and-run.md`, `docs/notes/ios-notifications-checklist.md`, `docs/superpowers/plans/2026-10-29-react-foundation.md` (roadmap item 7 done), `docs/superpowers/plans/2026-10-30-react-plan-2-backlog.md`
- Local only: `.claude/skills/deploy-keel/hooks/register.tsx` (plain `cargo tauri ios build --export-method debugging`)

- [ ] **Step 1:** `git rm -r crates/ui`; drop the workspace `exclude`.
- [ ] **Step 2:** CLAUDE.md: the layout table loses `keel-ui` and `web/` describes the shipped app; build commands become `cd web && pnpm check` / `cargo tauri dev` (runs `pnpm dev`) and note that `cargo test` needs `web/dist` (`cd web && pnpm build`); drop "until the cutover" lines (CSS in both, overlay config, `--config tauri.react.conf.json`); the logo lives in `web/src/shell/icons.tsx`; the `crates/ui/src/…` comments in `web/src` name the Dioxus source as of the cutover commit (history); Dioxus-only conventions (`OpenPop`, the capture-phase submit listener, `dx` notes) go.
- [ ] **Step 3: Verify.** `cargo test`, `cd web && pnpm check`, `cargo tauri build --no-bundle --debug` pass; `git grep -n "crates/ui\|keel-ui\|tauri.react.conf" -- ':!docs/superpowers' ':!web/src' ':!crates/plugin-glass'` prints nothing.
- [ ] **Step 4: Commit** `chore: remove the Dioxus UI` and `docs: React is the shipped frontend`.

### Task 5: Phone pass (Noah)

- [ ] `/deploy-keel`, then the open phone checks in CLAUDE.md: tap-target sizes, the keyboard covering the editor, the task view's packing flow (open a task, add and complete sub-tasks), whether "+ Description" raises the keyboard, the card circle's hit area. Findings become a follow-up plan.
