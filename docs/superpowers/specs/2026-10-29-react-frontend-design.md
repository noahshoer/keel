# React + TypeScript frontend (2026-10-29)

Replace the Dioxus UI (`crates/ui`, ~11k lines of Rust across 22 files, 124
tests, 17 JavaScript scripts embedded as Rust strings) with a React +
TypeScript frontend in the same Tauri shell. Reason: development speed and
compile-time safety for the UI code that has been fragile (gestures, the
sheet, the keyboard, glass sync). The bugs found so far were in WebKit, Tauri
plugins or our own code, not Dioxus; the change is about the development
loop, not correctness.

## Decisions (Noah, 2026-10-29)

- **Stack:** React 19 + TypeScript (strict) + Vite, in a new top-level `web/`.
  The Tauri shell, `keel-core`, `keel-storage`, `keel-service`, the IPC
  commands and the Swift glass plugin are unchanged.
- **App source is `.ts` / `.tsx`, strictly typed** (`web/src/`, `allowJs:
  false`). JavaScript is fine elsewhere where it's the natural fit (the
  simulator check scripts, generated glue); no blanket ban.
- **Linting and formatting: Biome**, with its React domain on.
- **Where logic lives:** data rules in `keel-core`; the frontend holds
  presentation and interaction (see CLAUDE.md, Conventions).
- **IPC types come from Rust through `ts-rs`.** Every type that crosses IPC
  derives `TS`; the generated files are committed and CI fails when they are
  stale.
- **Strict npm supply-chain checks** (see below) are part of the foundation,
  before any UI work.

## Decisions (Noah, 2026-10-30)

- **Behavior parity** with the Dioxus app is the bar for the port; a
  difference is a bug unless a plan lists it.
- **Liquid Glass is parked.** The React app never calls `plugin:glass|*` and
  takes the Dioxus no-glass paths everywhere, so iPhone parity means parity
  with the no-glass behavior. Its build doesn't load the glass plugin: the
  diagnostics log, keyboard pace and the sim-check script runner moved to a
  scoped plugin (`crates/plugin-ios`), and `tauri-plugin-glass` sits behind
  the `keel` crate's `glass` feature, which only the Dioxus config turns on.
  Glass comes back after parity and Noah's own web UI updates, once he is
  comfortable with the iOS app on the web UI.

## Shape

- `web/`: the Vite app. `src/bindings/` (ts-rs output, committed),
  `src/ipc/` (typed `invoke` client + mock backend for the browser),
  `src/core/` (typed wrapper over the wasm build of `keel-core`), then the
  screens.
- `crates/core-wasm` (`keel-core-wasm`): wasm-bindgen exports of the
  `keel-core` functions the UI calls in-process today (quick add parsing on
  every keystroke, due labels, cards, calendar, recurrence text). Domain
  logic that lives in the Dioxus crate today (e.g. `editor::repeat_text`,
  `recurrence_for`, `changes`, `labels_to_create`) moves into `keel-core`
  with its tests and is exported the same way. Pure UI logic (sheet and
  gesture rules, glass geometry, nav stack) is ported to TypeScript with its
  tests as Vitest.
- Both UIs exist side by side until cutover: Tauri's `frontendDist` stays on
  `crates/ui/dist`; the React app runs through an alternate Tauri config
  (`devUrl` = the Vite server) for desktop and `cargo tauri ios dev`.
  Cutover switches `frontendDist` to `web/dist` and deletes `crates/ui`.

## Contracts the port must keep

The Swift plugin, `Demo.swift` and the simulator checks depend on the page:
the CSS class names they query (`.tv-sheet`, `.tv-head .tv-close`,
`.task-row`, `.float-btns`, …), the `window.*` hooks Swift calls
(`keelKeyboard`, `keelSheetOpened`, …), the `plugin:glass|*` command names
and payloads, and the `diag` log sources. A screen is ported when it keeps
these.

## npm supply-chain policy

- Node pinned (`.nvmrc`), pnpm pinned with a hash in `packageManager`
  (installed through Corepack); no other package manager.
- `pnpm-workspace.yaml`: packages must be at least 7 days old
  (`minimumReleaseAge: 10080`), no dependency lifecycle scripts run
  (`strictDepBuilds: true`, every script-bearing package listed explicitly as
  ignored or allowed), no provenance downgrades (`trustPolicy: no-downgrade`),
  no git/tarball sub-dependencies (`blockExoticSubdeps: true`), exact versions
  only (`saveExact: true`), `engineStrict: true`.
- Every direct dependency is listed in `web/deps-allowlist.json` with a
  reason; a check fails on any dependency missing from it, any non-exact
  version, or any allowlist entry no longer used.
- CI installs with `--frozen-lockfile`, runs `pnpm audit --audit-level
  moderate` and OSV-Scanner over `pnpm-lock.yaml`.
- No `npx` / `pnpm dlx` in scripts or docs; GitHub Actions pinned by SHA.
- Dependencies kept minimal: no `@tauri-apps/api` (the global
  `window.__TAURI__` gets a typed declaration), Biome instead of ESLint +
  Prettier, no Vite wasm plugin.
- Adding a dependency follows the checklist in `web/DEPENDENCIES.md`.
