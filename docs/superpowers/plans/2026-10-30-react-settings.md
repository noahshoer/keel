# React Plan 6: Settings, import, reminders and diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The sidebar's gear opens Settings in the React app as it does in Dioxus (`crates/ui/src/settings.rs`): the Notifications section (Task reminders switch with the permission request, Remind me / Tasks without a time / Daily summary menus, Send test notification, the coming-up list), Appearance (the completion mark), Diagnostics (the logging switch), Import (Todoist CSV files, one result line per file) and the delivery note for this device.

**Architecture:** The text Settings shows about data moves into `keel-core` (`keel_core::settings`: the reminder menus' choices, labels and selection, what a pick changes, when an upcoming reminder fires relative to today, an import's result line) and is exported through `core-wasm`; the Dioxus crate uses it. The dialog's screen rules (the switch hint, the delivery note, the project name from a file name, the turn-on flow) are a plain `.ts` module with Vitest tests. The dialog is `web/src/settings/Settings.tsx`, opened from `Sidebar`. All commands already exist (`get_reminder_settings`, `set_reminder_settings`, `upcoming_reminders`, `notification_permission`, `request_notification_permission`, `reminders_when_closed`, `test_notification`, `diag_enabled`, `set_diag_enabled`, `import_todoist_csv`) with mock handlers.

**Tech Stack:** Rust `keel-core` + ts-rs + wasm-bindgen, React 19, TypeScript (strict), Vitest + Testing Library. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-29-react-frontend-design.md`; roadmap item 6 in `docs/superpowers/plans/2026-10-29-react-foundation.md`; Plan 5's "Left for later plans".

## Global Constraints

- Behavior parity with the Dioxus `SettingsDialog`; a difference is a bug unless "Decisions" or "As built" lists it. Strings verbatim ("Settings", "Notifications", "Task reminders", "Blocked by the system", "On", "Off", "Notifications are blocked. Allow them for Keel in your system settings, then try again.", "Remind me", "Tasks without a time", "Daily summary", "How many tasks are due that day", "Coming up", "Send test notification", "No reminders in the next two weeks.", "Turn on reminders to see what's coming up.", "Daily summary · …", "Appearance", "Completion mark", "Fills the circle when you complete a task", "Diagnostics", "Diagnostics logging", "Writes a log for bug reports. Off by default.", "Import", "From Todoist", the export hint, "Choose CSV files", "Importing…", "…: couldn't read the file", the two delivery notes, "Loading…").
- Keep the Dioxus markup and class names: `.modal-scrim`, `.modal`, `.modal-head`, `.modal-section`, `.setting-row`, `.setting-text`, `.setting-name`, `.setting-hint`, `.setting-warning`, `.setting-group[.disabled]`, `.switch[.on]` + `.knob` (`role="switch"`, `aria-checked`), `.upcoming-list`, `.upcoming-when`, `.upcoming-title`, `.upcoming-empty`, `.completion-setting`, `.mark-choices`, `.mark-choice[.chosen]`, `.mark-dot`, `.mark-name`, `.file-btn[.busy]`, `.import-results li.ok|.bad`, `.modal-foot`, `.pop-backdrop`. `app.css` applies unchanged; no CSS changes expected.
- Data rules in `keel-core` with Rust tests: TypeScript never formats a time, compares a reminder's date to today, or parses a stored setting's time. `ReminderSettings` and the commands are unchanged.
- Strict TS, no `any`, Biome clean; `pnpm check`, `cargo test`, `crates/ui` `cargo test` and its wasm `cargo check` (0 warnings), `scripts/check-bindings.sh` green.
- Commit per task on branch `react-settings`; merge `--no-ff` to `main` at the end.

## Decisions

- **The reminder menus are data** (`settings::reminder_menus`): their choices, labels, which one is checked and the button text come from core, and a pick goes back through `settings::pick_setting(s, menu, value)`. The stored times cross IPC as `HH:MM:SS`, so comparing them in TypeScript would mean parsing them there.
- **The completion mark is screen state** (localStorage `completion-icon`, as Dioxus); App owns it and Settings sets it through a callback, so rows redraw at once.

## Review Focus

1. **Turning reminders on while permission is denied.** The request comes back denied: the switch stays off, the warning shows, nothing is saved (Task 4 test).
2. **A menu pick while another save is in flight.** Each pick saves the full settings built from the latest ones shown, not a stale copy (Task 4 test: two picks in a row, the second save carries the first's change).
3. **Several files, one unreadable.** Every file gets its own line in order, the bad one marked `bad`, and the lists reload once at the end (Task 4 test).
4. **The diag switch fails.** It flips back and the page's logging stays as it was (Task 4 test).
5. **Escape with a menu open** closes the menu, not the dialog (Task 4 test).

## Files

| Path | Responsibility |
|---|---|
| `crates/core/src/settings.rs` | new: `offset_label`, `SettingChoice`, `SettingMenu`, `ReminderMenus`, `reminder_menus`, `MenuKind`, `pick_setting`, `upcoming_when`, `import_line` |
| `crates/ui/src/settings.rs` | uses the moved code |
| `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts` | exports |
| `web/src/settings/rules.ts` | `permissionHint`, `deliveryNote`, `projectNameOf`, `turnOn` |
| `web/src/settings/Settings.tsx` | the dialog |
| `web/src/lists/completion.ts` | `saveIcon` |
| `web/src/editor/pop.ts` | `Pop` gains `"offset" \| "allday" \| "summary"` |
| `web/src/shell/Sidebar.tsx`, `web/src/app/App.tsx` | the gear opens Settings; App's icon setter |

---

### Task 1: Settings text moves into keel-core

**Files:**
- Create: `crates/core/src/settings.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/tests/ts_bindings.rs`, `crates/ui/src/settings.rs`

**Interfaces (`keel_core::settings`):**
- `offset_label(m: i64) -> String` ("At the due time", "1 hour before", "N minutes before").
- `SettingChoice { value: String, label: String }`, `SettingMenu { text: String, selected: String, items: Vec<SettingChoice> }`, `ReminderMenus { offset, all_day, summary: SettingMenu }` (`Serialize` + `TS`). Values: minutes as text for the offset; `"off"` or `"HH:MM"` for the times. Offsets 0, 5, 10, 15, 30, 60; all-day "No reminder", 7, 8, 9, 10, 12, 18 o'clock; summary "Off", 6:30, 7, 7:30, 8, 9.
- `reminder_menus(s: &ReminderSettings) -> ReminderMenus`.
- `MenuKind` (`Offset | AllDay | Summary`, `Deserialize` + `TS`); `pick_setting(s, kind, value: &str) -> ReminderSettings` (an unreadable offset is 0, an unreadable time is off, as Dioxus).
- `upcoming_when(at: NaiveDateTime, today: NaiveDate) -> String` (Dioxus `when_text`: "Today 5 PM", "Tomorrow 9 AM", "Tue 9 AM", else `due_label` + time).
- `import_line(s: &ImportSummary) -> String`.

- [ ] **Step 1: Failing tests** in `settings.rs`: offset labels; `reminder_menus` of the defaults (offset text "At the due time", selected "0"; all-day selected "09:00", text "9 AM"; summary "Off" / "off") and the item counts and first labels; a stored time with seconds still selects its item; `pick_setting` for each menu, plus "x" → offset 0 and "off" → `None`; `upcoming_when` for today, tomorrow, 3 days ahead (weekday), 10 days ahead, yesterday; `import_line` with only tasks ("Travel: 1 task"), and with every part plural.
- [ ] **Step 2: Run** `cargo test -p keel-core settings` → FAIL.
- [ ] **Step 3: Implement**; the Dioxus dialog calls `offset_label`, `reminder_menus`, `pick_setting`, `upcoming_when`, `import_line`; export the three types in `ts_bindings.rs`.
- [ ] **Step 4: Run** `cargo test`, `cd crates/ui && cargo test && cargo check --target wasm32-unknown-unknown`, `scripts/check-bindings.sh` → green, 0 warnings.
- [ ] **Step 5: Commit** `refactor(core): settings menus, upcoming times and import lines move into keel-core`.

### Task 2: Exports to the web

**Files:**
- Modify: `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts`, `web/src/core/core.test.ts`

**Interfaces (TS):** `reminderMenus(s: ReminderSettings): ReminderMenus`; `pickSetting(s, kind: MenuKind, value: string): ReminderSettings`; `upcomingWhen(at: string, today: string): string`; `importLine(s: ImportSummary): string`.

- [ ] **Step 1: Failing tests** in `core.test.ts`: `reminderMenus` of the fixture settings (`all_day_time: "09:00:00"`) selects "09:00"; `pickSetting(s, "Summary", "07:30")` gives `daily_summary` "07:30:00"; `upcomingWhen("2026-10-30T17:00:00", "2026-10-30") === "Today 5 PM"`; `importLine` of a two-task summary.
- [ ] **Step 2: Run** `cd web && pnpm vitest run core` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(core-wasm): settings exports`.

### Task 3: Screen rules

**Files:**
- Create: `web/src/settings/rules.ts` (+ `rules.test.ts`)
- Modify: `web/src/lists/completion.ts` (+ test)

**Interfaces:**
- `permissionHint(permission: string, enabled: boolean): string` ("Blocked by the system" when denied, else "On" / "Off").
- `deliveryNote(whenClosed: boolean): string` (ported test `delivery_note_describes_only_this_device`).
- `projectNameOf(fileName: string): string` (last path part, one trailing `.csv` / `.CSV` dropped).
- `turnOn(permission: string, request: () => Promise<string>): Promise<{ permission: string; granted: boolean }>` (asks only when not already granted; a failed request counts as denied).
- `saveIcon(icon: CompletionIcon): void`.

- [ ] **Step 1: Failing tests** for each.
- [ ] **Step 2: Run** `pnpm vitest run rules completion` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): settings screen rules`.

### Task 4: The Settings dialog

**Files:**
- Create: `web/src/settings/Settings.tsx`, `web/src/settings/settings.test.tsx`
- Modify: `web/src/editor/pop.ts`

**Interfaces:** `Settings({ onClose, onImported, icon, setIcon })`. Loads the settings, permission, when-closed, diag state and upcoming list on mount ("Loading…" until the settings land). Markup and behavior as the Dioxus dialog; menus are the editor's `Menu` with ids `offset` / `allday` / `summary`; one `open` popover with `.pop-backdrop`; Escape closes the menu first, then the dialog; a click on the scrim closes; the `.modal` takes focus on mount. Saving sets the shown settings at once, then `set_reminder_settings` (an error shows as the warning) and reloads the upcoming list. The diag switch calls `set_diag_enabled`, then `diag.setActive`, flipping back on failure. Import reads each chosen file with `File.text()` and calls `import_todoist_csv` in order, then `onImported` once.

- [ ] **Step 1: Failing tests** (mock backend): the sections and rows render with the mock's values; the reminders switch turns off and saves; turning on with permission denied shows the warning and saves nothing (Review Focus 1); picking "5 minutes before" then "Off" for the summary sends both changes (Review Focus 2); "Send test notification" calls `test_notification`; upcoming rows show `upcomingWhen` text and "Daily summary · …" for a summary; the empty text depends on `enabled`; a mark choice calls `setIcon` and stores it; the diag switch saves and flips back when the command fails (Review Focus 4); importing two files, one failing, gives an ok and a bad line in order and calls `onImported` once (Review Focus 3); Escape with a menu open closes only the menu, a second Escape closes the dialog (Review Focus 5); the delivery note follows `reminders_when_closed`.
- [ ] **Step 2: Run** `pnpm vitest run settings` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): settings dialog`.

### Task 5: The gear opens Settings

**Files:**
- Modify: `web/src/shell/Sidebar.tsx`, `web/src/shell/Sidebar.test.tsx`, `web/src/app/App.tsx`, `web/src/app/App.test.tsx`

- [ ] **Step 1: Failing tests:** clicking `.settings-btn` shows `.modal` with "Settings"; its Close hides it; in App, choosing "Pine" changes the mark the rows' circles use (`CompletionIconContext`).
- [ ] **Step 2: Run** `pnpm vitest run Sidebar App` → FAIL.
- [ ] **Step 3: Implement:** `Sidebar` takes `icon` / `setIcon` and renders `Settings` after `.brand` while open (`onImported` → `onChanged`); App's icon state gets its setter.
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): the sidebar's gear opens Settings`.

### Task 6: Parity pass, docs, merge

**Files:**
- Modify: `CLAUDE.md` (React section: Settings works; Plan 7, the cutover, next), `docs/superpowers/plans/2026-10-29-react-foundation.md` (roadmap item 6 done), `docs/superpowers/plans/2026-10-30-react-plan-2-backlog.md` (tick Settings), this plan's "As built"

- [ ] **Step 1: Simulator, React build:** open Settings through `KEEL_SCRIPT` (click `.settings-btn`), screenshot it; diag log has no JS errors.
- [ ] **Step 2: Phone check for Noah:** turn reminders on (the system prompt the first time), change each menu, send a test notification, pick a mark and complete a task, toggle diagnostics, import a Todoist CSV.
- [ ] **Step 3: Write the docs.**
- [ ] **Step 4: Run** `cargo test`, UI `cargo test` + wasm check, `pnpm check`, `scripts/check-bindings.sh` → green.
- [ ] **Step 5: Commit** `docs: React settings as built`, merge `react-settings` `--no-ff` to `main`, push.

## As built

(Filled in at the end.)
