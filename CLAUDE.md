# Keel

A standalone, local-first, Todoist-style task app with streaks, written in Rust: **Tauri v2** shell + **Dioxus 0.6** UI compiled to wasm. It's not a Todoist client and has no Todoist dependency; Todoist's UI was the starting reference, and Keel now has its own look (green `#268366` accent, a pine-reflection logo).

Owner: Noah (GitHub `noahshoer`, **public** repo `noahshoer/keel`; anything committed is visible to everyone, so never commit secrets, and keep Todoist reverse-engineering material out of the repo, as history was rewritten on 2026-10-20 to remove it). v1 targets **iOS**, minimum **iOS 26** (`bundle.iOS.minimumSystemVersion`). Windows desktop is the development target, and everything except the iOS build has been built and tested there.

## Current status / what's next

**iOS runs in the simulator** (iPhone 16, iOS 18.6, 2026-10-20). All checks in [docs/notes/ios-notifications-checklist.md](docs/notes/ios-notifications-checklist.md) pass. `crates/src-tauri/gen/apple` is committed. Fixes from that run:
- `tauri-plugin-notification` 2.7 on iOS reads `Schedule::At` as device-local time (upstream #3256), so `notifications::plugin_date` sends the local time labeled UTC. Its `pending()` and `cancel_all()` always fail, so the scheduled ids are kept in `local_setting` (`AppService::scheduled_reminder_ids`) and cancelled with `cancel(ids)`. Upstream issue draft (not yet filed): `docs/notes/upstream-notification-ios-issue.md`.
- The viewport has `maximum-scale=1, user-scalable=no`; otherwise iOS zooms into any focused input under 16px.
- Run `cargo tauri icon` on the Mac; the iOS icon set is only written when `gen/apple` exists.

**Keel runs on Noah's iPhone** (2026-10-20; personal team `SMD3LL6NFU` in `tauri.conf.json`). Phone-only additions: swipe right/left opens/closes the drawer (`ui::gestures`), and the settings note describes only the current device (`reminders_when_closed` command).
- Standalone install (UI bundled, no Mac needed): `cd crates/src-tauri && cargo tauri ios build --export-method debugging`, then `xcrun devicectl device install app --device 00008160-00082D8921400036 gen/apple/build/arm64/Keel.ipa`. With a free team the install expires after 7 days; re-run to renew. Rebuild `crates/ui/dist` first.
- `cargo tauri ios dev` loads the UI over Wi-Fi from the Mac (Local Network permission), so the app only works while that command is running.
- Not yet checked on the phone: tap-target sizes and the keyboard covering the editor.

After iOS: **sync** (dormant by design). Every change already goes through an idempotent command log, so the plan is to swap rusqlite for libSQL and add Turso, plus some device-link model. Then integrations (Apple Reminders, MS To Do, Google Calendar, Outlook, Notion). Spec non-goals for v1: comments, attachments, location reminders, collaboration, templates, activity log, backups.

v1 features done: projects, sections, sub-tasks, labels, filters (query language), priorities, due dates and times, deadlines, recurrence (incl. intervals, yearly, last day/workday, nth weekday), streaks, natural-language quick add with live highlighting, search, completed-task view, local notifications (global rule + per-task reminders + daily summary), dark mode, responsive layout, and Todoist CSV import (Settings → Import; `core::import::{csv, todoist}`, `AppService::import_todoist_csv`).

## Layout

| Crate | Package | What |
|---|---|---|
| `crates/core` | `keel-core` | Pure domain: `model`, `commands` (Action enum + `apply_at`), `recurrence`, `streak`, `filter` (query language), `quickadd` (NL parser + spans), `reminders` (notification planner), `search`, `calendar`, `views`. Host-tested, no I/O. |
| `crates/storage` | `keel-storage` | `SqliteRepo` (rusqlite, bundled) implementing `Repository`. Schema in `migrations.rs`; `local_setting` table holds device-local prefs (never synced). |
| `crates/service` | `keel-service` | `AppService`: views, dispatch, search, reminder settings and plan. |
| `crates/src-tauri` | `keel` (lib `keel_lib`) | Tauri IPC wrappers + `notifications.rs` (iOS: schedule with the OS; desktop: 20s ticker thread). App id `com.nshoe.keel`, store `keel.db`. |
| `crates/ui` | `keel-ui` | Dioxus web/wasm UI. **Excluded from the workspace**, so build it with `dx`. Talks to Rust via `window.__TAURI__.core.invoke` (`api.rs`). |

Design spec: `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md`. Each phase has a plan in `docs/superpowers/plans/` with its decisions and notes.

## Build, test, run

CI (`.github/workflows/ci.yml`, runs on every push): **Windows** runs the tests, the UI tests and wasm check, the web UI build, and `cargo tauri build --no-bundle`. **iOS** (`macos-26`) runs `cargo tauri ios build --target aarch64 --debug --no-sign --ci` (iPhone target; the simulator target fails on the runner's Xcode) plus a simulator Rust build. The web UI is built first in both jobs (the Tauri crate won't compile without `crates/ui/dist`). Verification only: nothing is signed or deployed. Keep `DX_VERSION` / `TAURI_CLI_VERSION` in the workflow in step with the local tools.

```bash
cargo test                                   # core + storage + service (UI excluded)
cd crates/ui && cargo check --target wasm32-unknown-unknown   # UI type-check (0 warnings expected)
cd crates/ui && dx build --platform web && cp -r target/dx/keel-ui/debug/web/public/* dist/
cd crates/src-tauri && cargo tauri dev       # desktop app (serves crates/ui/dist)
```
- Tauri's `frontendDist` is `crates/ui/dist` (git-ignored). **Rebuild and copy after every UI change**, then restart or reload the app.
- `dx` 0.7.x prints "dx and dioxus versions are incompatible" against Dioxus 0.6.3. It's harmless; the build completes. Because of that mismatch, Dioxus' `asset!` doesn't bundle files, so the Inter font is embedded as base64 data URLs in `style.rs`.
- App icons come from `crates/src-tauri/icons/keel.svg`: `cd crates/src-tauri && cargo tauri icon icons/keel.svg`. The sidebar logo is `icons::logo()` in `crates/ui/src/icons.rs`; keep the two in sync.
- Stop a running `cargo tauri dev` before big builds. Its watcher rebuilds on file changes and fights over `target/debug/keel.exe`.

## Conventions

- Workflow per phase: write a short plan in `docs/superpowers/plans/` → feature branch → **TDD** (failing test first) → commit per task → merge `--no-ff` to `main` → push. Noah prefers you build without asking for routine decisions; ask about genuinely personal choices (names, visual direction), ideally with visual options.
- Tauri commands use `#[tauri::command(rename_all = "snake_case")]`, and the frontend sends snake_case args.
- Commands are idempotent (uuid) with temp-id → id mapping. Add new `Action` variants (never change existing serialized shapes); new fields are `#[serde(default)]`. Adding a field to `TaskAdd`/`TaskUpdate` means updating struct literals in tests too.
- `Recurrence` variants are append-only (stored as JSON in `Due`). Consumers only use `contains` / `next_after` / `prev_before`.
- Quick add: an unrecognized `every …` phrase stays text as a whole (no silent misreads). A title that would become empty applies nothing.
- UI: one popover open per editor (`OpenPop`), fixed-positioned on mount by `pickers::place_popover`. Task reminders are shared through Dioxus context (`Signal<Vec<Reminder>>`). A global capture-phase `submit` listener in `App` prevents native form submits, because Dioxus' `prevent_default()` isn't honored on submit and the page would reload.
- On Windows the working tree is CRLF (`core.autocrlf=true`). Use exact-string edits; shell regex edits across lines are fragile.

## Verifying UI changes

- **Browser preview with a mock backend:** serve a copy of `dist/` whose `index.html` defines a fake `window.__TAURI__.core.invoke` returning sample data (one handler per IPC command). Good for layout, dark mode and 375px checks.
- **The real app over the WebView debug port (Windows):** run `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 cargo tauri dev`, then drive the page with CDP `Runtime.evaluate` from a small Node script (`fetch('http://127.0.0.1:9223/json')`, then a WebSocket to the page). This is how deadlines, reminders and notifications were verified end to end. Desktop reminders log `reminders: showed …` to the dev console.
- Don't screen-capture the desktop or steal window focus to check the app.
