# Liquid Glass on iPhone Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On iPhone, the task composer, the task editor, their pickers and the small floating buttons become native SwiftUI Liquid Glass, while Windows/desktop stay on the web editor.

**Architecture:**
- A Tauri v2 mobile plugin (`crates/plugin-glass`: a Rust crate plus a Swift package) overlays SwiftUI views on the WKWebView.
- The Dioxus UI stays the brain:
  - It builds a `CardState` from `EditorValues` (quick-add spans and chip labels included) and sends it to Swift.
  - Swift sends user actions back as plugin events.
  - Dioxus applies them and saves through the existing `editor` save functions.
- Swift holds no task logic.

**Tech Stack:**
- Tauri 2 (tauri-cli 2.12.3) mobile plugin API (`Plugin`, `@objc` commands, `trigger`)
- SwiftUI `.glassEffect()` (iOS 26)
- Dioxus 0.6 wasm
- `keel-core::quickadd`

**Spec:** `docs/superpowers/specs/2026-10-20-liquid-glass-ios-design.md`

## Global Constraints

- **Platforms:**
  - Minimum iOS 26.0; use `.glassEffect()` directly, with no availability fallbacks.
  - Windows/desktop behavior is unchanged: the plugin's Rust side compiles there and does nothing.
  - CI (`windows` and `ios` jobs) must stay green on every commit.
- **Shared logic:**
  - There is one quick-add parser, `keel_core::quickadd::parse`. No parsing in Swift.
  - Saving goes through `editor::add_from_editor` (add) and the edit save function from Task 4 (edit). Moves, reminders and the command log behave exactly as on the web.
  - The edit card never parses quick add (`quick_add: false`), so its title has no highlighting.
- **Chip behavior:**
  - The chip row is, in order: Date, Priority, Labels, Reminders, Deadline, Repeat, Project/section. There is no "+ (more)" chip.
  - The send button is pinned at the right end of the row: a green arrow when adding, a checkmark when editing. Accent `#268366`.
  - Picking a value on a chip removes the matching typed phrase(s) from the title (the chip wins).
- **Card lifecycle:**
  - After an add, the card stays open and empty (`keep_open`).
  - Swiping the card down or tapping outside it discards it, in both add and edit mode.
- **Navigation and buttons:**
  - The sidebar drawer stays web and stays the navigation. No tab bar.
  - Floating glass buttons: hamburger (top left), search (top right), green + (bottom right).
  - The in-list "Add task" rows stay and open the native composer with their own section/date.
- **Repo rules:**
  - The repo is public: no secrets.
  - Commit email `99695018+noahshoer@users.noreply.github.com`.
  - TDD for the Rust/wasm logic; Swift views are checked by hand on the simulator and on the phone.
- **If the spike (Task 1) fails** on touch pass-through or keyboard layout: stop and report to Noah. No fallback without him.

## Review Focus

1. **Non-ASCII titles** (emoji, "café", "naïve" before a date phrase): highlight ranges must land on the right characters. Rust spans are byte offsets, but Swift's `NSRange` is UTF-16, so `CardState` carries UTF-16 offsets. Tested in Task 3.
2. **The same kind of phrase typed twice, then the chip picked** ("tomorrow call mom tomorrow"): every span of that kind is removed, not just the first, and the leftover whitespace collapses to single spaces. Tested in Task 3.
3. **Editing a task whose title looks like quick add** ("Buy p1 batteries #2"): edit-mode `CardState` has no spans, the chips show the stored values, and saving leaves the title untouched. Tested in Task 3 and Task 4.
4. **Fast typing:** stale updates must not overwrite newer text. `CardState.title` echoes the text it was computed for, and Swift ignores a state whose title differs from the field's current text. Tested in Task 3 (echo) and checked by hand in Task 5.
5. **Return in the title submits** (as on the web), and an empty or whitespace-only title can't be sent. Tested in Task 3 (`can_submit`) and checked by hand in Task 5.

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/core/src/quickadd.rs` | + `strip_spans`: remove typed phrases of given kinds (pure, host-tested). |
| `crates/plugin-glass/` (new, workspace member) | Tauri plugin `tauri-plugin-glass`. `src/lib.rs` (init, `available` command), `ios/Sources/GlassPlugin/*.swift` (overlay, card, pickers, buttons), `permissions/` (generated), `build.rs`. |
| `crates/src-tauri/src/lib.rs`, `Cargo.toml`, `capabilities/default.json` | Register the plugin and allow its commands. |
| `crates/ui/src/glass.rs` (new) | Wire types (`CardState`, `ChipState`, `GlassSpan`, `GlassEvent`, `PickerData`), `card_state()`, `apply_event()`, the JS bridge (`show_card`, `update_card`, `hide_card`, event listener). |
| `crates/ui/src/editor.rs` | Extract `save_edit()` from `task_row.rs`. `AddTask` routes to native when available. |
| `crates/ui/src/task_row.rs` | Editing and "Add sub-task" route to native when available. |
| `crates/ui/src/main.rs`, `style.rs` | Floating-button events. Hide the web `.menu-btn` when native glass is on. |

---

### Task 1: Spike — glass overlay over the Tauri web view (throwaway)

**Branch:** `spike/glass-overlay` from `feat/liquid-glass`; never merged.

**Files:**
- Create: whatever the spike needs. `cargo tauri plugin new glass --ios` (run in `crates/`) is the quickest start.

- [ ] **Step 1:** In the plugin's `load(webview:)`, add a `UIHostingController` view over the web view.
  - It holds one `.glassEffect()` circle button (bottom right) and a `TextField` in a glass card pinned above the keyboard (`.safeAreaInset(edge: .bottom)` or keyboard-following layout).
  - The host view's background is clear, and its `hitTest` returns `nil` outside the glass controls, via a `UIView` subclass that overrides `point(inside:with:)`.
- [ ] **Step 2:** Round-trip:
  - Typing in the field triggers `trigger("titleChanged", data: ["text": …])`.
  - A tiny JS listener (`window.__TAURI__.core.addPluginListener('glass', 'titleChanged', …)`) echoes back via `invoke('plugin:glass|update', {spans: [[0,3]]})`.
  - Swift colors those characters.
- [ ] **Step 3:** Run on the simulator (`cd crates/src-tauri && cargo tauri ios dev`, iPhone 17 / iOS 26) and check:
  - (a) The task list scrolls and tasks are tappable everywhere outside the glass views.
  - (b) The card sits directly on the keyboard, with no black gap and no web page scroll.
  - (c) The highlight follows typing with no visible lag.
  - (d) The swipe-to-open drawer still works.
- [ ] **Step 4:** Write the findings (what worked, the exact overlay and keyboard technique, any gotchas) into this plan under "Spike notes" on `feat/liquid-glass` and commit. **If (a) or (b) fails, stop and report to Noah.**

### Spike notes
_(filled in by Task 1)_

---

### Task 2: Plugin crate, registered and building everywhere

**Files:**
- Create: `crates/plugin-glass/{Cargo.toml,build.rs,src/lib.rs,ios/Package.swift,ios/Sources/GlassPlugin/GlassPlugin.swift}`
- Modify: root `Cargo.toml` (members), `crates/src-tauri/Cargo.toml`, `crates/src-tauri/src/lib.rs` (`.plugin(tauri_plugin_glass::init())`), `crates/src-tauri/capabilities/default.json` (`"glass:default"`)
- Test: `crates/plugin-glass/src/lib.rs` (unit test)

**Interfaces:**
- Produces:
  - The plugin name `glass`.
  - Swift commands `show(CardState+PickerData JSON)`, `update(CardState JSON)`, `hide()` and `setButtons({visible: bool})`.
  - Swift events: `title`, `description`, `pick`, `submit`, `cancel`, `fab`, `menu`, `search`. Payloads are in Task 3.
  - Rust command `plugin:glass|available -> bool`: `cfg!(target_os = "ios")`.

- [ ] **Step 1:** Write a failing test, `available_is_false_on_desktop`: `assert!(!available())` under `#[cfg(not(target_os = "ios"))]`.
- [ ] **Step 2:** Run `cargo test -p tauri-plugin-glass`. Expected: FAIL (no crate or function yet).
- [ ] **Step 3:** Build the plugin crate from the spike's skeleton.
  - Use `tauri_plugin::Builder` in `build.rs` with commands `["available"]`.
  - On desktop, only `available` exists. On iOS, register the Swift plugin with `ios_plugin_binding!(init_plugin_glass)`.
  - The Swift `GlassPlugin` has empty `show/update/hide/setButtons` stubs for now.
- [ ] **Step 4:** Run `cargo test` (workspace) and `cd crates/src-tauri && cargo tauri ios build --target aarch64 --debug --no-sign`. Expected: both succeed.
- [ ] **Step 5:** Commit (`feat(glass): plugin crate registered on all platforms`), push, and confirm both CI jobs pass.

---

### Task 3: Card state and event logic (pure, wasm/host-tested)

**Files:**
- Modify: `crates/core/src/quickadd.rs` (+ `strip_spans`), tests in the same file's test module.
- Create: `crates/ui/src/glass.rs` (types plus pure functions; the bridge comes in Task 6), `mod glass;` in `main.rs`.
- Test: `crates/ui/src/glass.rs` `#[cfg(test)]` (run with `cd crates/ui && cargo test`).

**Interfaces:**
- Produces:
  - `keel_core::quickadd::strip_spans(input: &str, spans: &[Span], kinds: &[SpanKind]) -> String`
  - In `glass.rs` (all `Serialize`/`Deserialize`, `rename_all = "camelCase"`):
    - `enum CardMode { Add, Edit }`
    - `enum ChipId { Date, Priority, Labels, Reminders, Deadline, Repeat, Location }` (in that order)
    - `struct GlassSpan { start: usize, end: usize, kind: String }`: UTF-16 offsets. `kind` is the lowercase `SpanKind` name, which picks the color.
    - `struct ChipState { id: ChipId, label: String, set: bool }`
    - `struct CardState { mode: CardMode, title: String, description: String, spans: Vec<GlassSpan>, chips: Vec<ChipState>, can_submit: bool }`
    - `struct PickerData { projects: Vec<PickProject>, labels: Vec<String>, keep_label: Option<String>, values: EditorValuesWire }`. Here `PickProject { id, name, sections: Vec<(Id, String)> }`, and `EditorValuesWire` mirrors the `EditorValues` fields Swift pickers need to preselect: date, time, priority, repeat, location, labels, deadline, reminders.
    - `enum GlassEvent { Title { text }, Description { text }, Pick(Pick), Submit, Cancel }`
    - `enum Pick { Date { date: String, time: String }, Priority { value: String }, Labels { names: Vec<String> }, Reminders { drafts: Vec<ReminderDraft> }, Deadline { date: String }, Repeat { value: String }, Location { project_id: Id, section_id: Option<Id> } }`. Value formats are exactly `EditorValues`' formats.
    - `fn card_state(v: &EditorValues, mode: CardMode, projects: &[Project], sections: &[Section], today: NaiveDate) -> CardState`
    - `fn apply_event(v: &mut EditorValues, ev: &GlassEvent, mode: CardMode, projects: &[Project], sections: &[Section], today: NaiveDate) -> Outcome`, with `enum Outcome { Changed, Submit, Cancel }`.
- **Chip-to-kind mapping for stripping:**
  - Date → [Date, Time]
  - Priority → [Priority]
  - Labels → [Label]
  - Deadline → [Deadline]
  - Repeat → [Repeat]
  - Location → [Project, Section]
  - Reminders → []
- **Chip labels:**
  - In add mode, they show the merged values (chips plus quick-add parse), the same merge `new_task_from` does. Make `new_task_from` `pub(crate)` and reuse it.
  - The Date chip reuses the web date chip's text helper.
  - Empty chips show their name: "Date", "Priority", "Labels", "Reminders", "Deadline", "Repeat", "Project".

- [ ] **Step 1:** Write failing tests in `quickadd.rs`:
  - `strip_spans_removes_every_span_of_the_kinds`: `"tomorrow call mom tomorrow"` with its parsed spans and `[Date]` gives `"call mom"`.
  - `strip_spans_keeps_other_kinds`: `"call mom tomorrow p1"`, `[Priority]` → `"call mom tomorrow"`.
  - `strip_spans_collapses_spaces`: `"a  p1  b"` → `"a b"`.
- [ ] **Step 2:** Run `cargo test -p keel-core strip_spans`. Expected: FAIL.
- [ ] **Step 3:** Implement `strip_spans`.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Write failing tests in `glass.rs`:
  - `spans_are_utf16`: title `"café 🎉 tomorrow"` in add mode gives one Date span with `start == 8, end == 16` (UTF-16: `café ` = 5, 🎉 = 2, space = 1).
  - `add_mode_chip_shows_typed_date`: `"call mom tomorrow"` gives a Date chip with `set == true` and the label the web chip shows for tomorrow.
  - `edit_mode_has_no_spans_and_shows_stored_values`: a task titled `"Buy p1 batteries #2"` with priority 4 → `spans.is_empty()`, Priority chip `set == false`, `title == "Buy p1 batteries #2"`.
  - `title_is_echoed`: `card_state(...).title == v.content`.
  - `can_submit_needs_text`: `"   "` → `false`; `"x"` → `true`.
  - `picking_date_strips_typed_date`: content `"call mom tomorrow 5pm"`, then `Pick::Date{date:"2026-10-23",time:""}` gives `v.content == "call mom"`, `v.date == "2026-10-23"`, `v.time == ""`, and returns `Outcome::Changed`.
  - `picking_in_edit_mode_does_not_touch_title`: edit mode, content `"Buy p1 batteries"`, `Pick::Priority{value:"1"}` → content unchanged, `v.priority == "1"`.
  - `submit_and_cancel_outcomes`: `Submit` → `Outcome::Submit`; `Cancel` → `Outcome::Cancel`.
  - `events_deserialize_from_swift_json`: `{"type":"pick","pick":{"type":"date","date":"2026-10-23","time":"09:00"}}` parses to the matching `GlassEvent`. The serde tag layout chosen here is the contract Swift must follow.
- [ ] **Step 6:** Run `cd crates/ui && cargo test glass`. Expected: FAIL.
- [ ] **Step 7:** Implement the types, `card_state` and `apply_event`.
- [ ] **Step 8:** Run `cd crates/ui && cargo test && cargo check --target wasm32-unknown-unknown`, then `cargo test` at the root. Expected: all PASS, 0 warnings.
- [ ] **Step 9:** Commit (`feat(glass): card state and events, chips strip typed phrases`).

---

### Task 4: One edit-save function shared by web and native

**Files:**
- Modify: `crates/ui/src/editor.rs` (+ `save_edit`), `crates/ui/src/task_row.rs:80-96` (call it).
- Test: `crates/ui/src/editor.rs` tests.

**Interfaces:**
- Produces:
  - `pub async fn save_edit(id: Id, before: EditorValues, after: EditorValues, existing: Option<Recurrence>)`
  - It does what `task_row.rs` does now: move if moved, then `sync_reminders`, then `update_task` if the patch is non-empty.

- [ ] **Step 1:** Write a failing test, `edit_changes_leave_quick_add_text_alone`: `changes(before, after)` where only `after.priority` changed on the content `"Buy p1 batteries #2"`. The patch has `content == None` and `priority == Some(P1)`. This pins Review Focus 3 on the save side.
- [ ] **Step 2:** Run `cd crates/ui && cargo test edit_changes`. It may already pass; it's a guard. If it passes, note it and continue.
- [ ] **Step 3:** Extract `save_edit` and call it from `task_row.rs`.
- [ ] **Step 4:** Run `cd crates/ui && cargo test && cargo check --target wasm32-unknown-unknown`. Expected: PASS. Rebuild `dist`, run `cargo tauri dev` on the Mac, and edit a task in the web editor to confirm it still saves, moves and changes reminders.
- [ ] **Step 5:** Commit (`refactor(ui): save_edit shared by web and native editors`).

---

### Task 5: Swift card — title, description, chip row, send

**Files:**
- Create: `crates/plugin-glass/ios/Sources/GlassPlugin/{Overlay.swift,CardView.swift,CardModel.swift}`
- Modify: `GlassPlugin.swift` (`show`/`update`/`hide` fill `CardModel`)

**Interfaces:**
- Consumes:
  - The `CardState`/`PickerData` JSON from Task 3 (decode with `Codable`, camelCase).
  - Events emitted with the exact `GlassEvent` JSON layout from Task 3's deserialization test.
- Produces: `CardModel` (an `@Observable` class) that holds the current state. The pickers (Task 7) read `PickerData` from it and emit `pick`.

- [ ] **Step 1:** Build the overlay per the spike notes:
  - Touches pass through outside the glass views.
  - The card is a `.glassEffect()` rounded rectangle on the keyboard.
- [ ] **Step 2:** Build the card:
  - **Title field:** attributed highlighting from `spans` (UTF-16 `NSRange`). Colors match the web `quick-chip` classes per kind.
  - **Description field:** smaller, below the title.
  - **Chip row:** a horizontal `ScrollView` of glass capsule chips in `chips` order; set chips are tinted green.
  - **Send button:** pinned outside the scroll view. A green arrow in `.add` mode, a checkmark in `.edit`, disabled when `!canSubmit`.
- [ ] **Step 3:** Wire the behavior:
  - Each edit sends `title`/`description` events. The title field ignores an `update` whose `title` ≠ its current text (Review Focus 4).
  - Return in the title sends `submit` when `canSubmit` (Review Focus 5).
  - Swipe down or a tap outside sends `cancel`.
  - Tapping a chip calls the picker hook (a stub until Task 7).
  - Hand-test the title field with a temporary JS snippet in the web console (Safari → Develop → Simulator):
    - Call `invoke('plugin:glass|show', …)` with sample JSON.
    - Type quickly and check the highlight never jumps back.
    - Check that Return sends `submit` (logged by `addPluginListener`).
- [ ] **Step 4:** Build for the simulator and hand-check:
  - glass rendering in light and dark mode
  - the card on the keyboard with no gap
  - chips scroll while send stays put
  - emoji titles highlight correctly
- [ ] **Step 5:** Commit (`feat(glass): native card with highlighted title and chip row`). Push, then check CI.

---

### Task 6: Dioxus bridge — composer and editor go native on iPhone

**Files:**
- Modify: `crates/ui/src/glass.rs` (bridge), `crates/ui/src/editor.rs` (`AddTask`), `crates/ui/src/task_row.rs` (edit and sub-task), `crates/ui/src/api.rs` (`glass_available()`).

**Interfaces:**
- Consumes:
  - `card_state`, `apply_event` and `Outcome` (Task 3)
  - `save_edit` (Task 4)
  - `add_from_editor` (existing)
- Produces:
  - `glass::NativeGlass(Signal<bool>)` context, provided in `App` from `api::glass_available()`.
  - `glass::open_card(session: CardSession)`, where `CardSession { mode, values: EditorValues, before: Option<EditorValues>, task_id: Option<Id>, existing: Option<Recurrence>, projects, sections, labels, keep_label, reload: Signal<u32> }`.
  - One listener per event type, registered once in `App` via `addPluginListener`. It calls `apply_event` on the open session, then `update`, `hide` or a save:
    - On `Submit` in add mode: `add_from_editor`, then reset to the session's initial values and `update` (the card stays open).
    - On `Submit` in edit mode: `save_edit`, then `hide`.
    - On `Cancel`: `hide`.
    - Either save bumps `reload`.

- [ ] **Step 1:** Write a failing test, `add_submit_resets_to_defaults`:
  - Model the session's pure part as `fn after_submit(mode: CardMode, initial: &EditorValues) -> Option<EditorValues>`.
  - In `Add` mode it returns `Some(initial.clone())`; in `Edit` it returns `None` (close).
- [ ] **Step 2:** Run `cd crates/ui && cargo test after_submit`. Expected: FAIL.
- [ ] **Step 3:** Implement `after_submit` and the bridge.
  - `AddTask` and the two `TaskEditor` sites in `task_row.rs` check `NativeGlass`. When it's true they call `open_card` instead of rendering `TaskEditor`.
  - Desktop has no change (`available` is false).
- [ ] **Step 4:** Run `cd crates/ui && cargo test && cargo check --target wasm32-unknown-unknown`. Expected: PASS. Rebuild `dist`, run on the simulator and check:
  - Adding "call mom tomorrow 5pm p1 #Work" makes the right task, and the card stays open and empty.
  - Editing a task saves.
  - Sub-task add lands under the parent.
  - Section "Add task" lands in that section, and an Upcoming day row on that day.
  - Swipe down discards.
  - The desktop app is unchanged on `cargo tauri dev`.
- [ ] **Step 5:** Commit (`feat(glass): composer and editor open natively on iPhone`).

---

### Task 7: Native pickers

**Files:**
- Create: `crates/plugin-glass/ios/Sources/GlassPlugin/Pickers/{DateSheet,PriorityMenu,LabelsSheet,RemindersSheet,DeadlineSheet,RepeatMenu,LocationSheet}.swift`

**Interfaces:**
- Consumes: `PickerData` (current values, projects/sections, labels, `keepLabel`) via `CardModel`.
- Emits `pick` events in Task 3's `Pick` JSON.

The choices mirror the web pickers in `crates/ui/src/pickers.rs` and `editor.rs`. Read them and offer the same options, nothing dropped:

| Chip | Picker |
|---|---|
| Date | Sheet: quick choices (as the web Scheduler offers), a graphical calendar, the time wheel, "No date". |
| Priority | Glass menu P1–P4, with the web's flag colors. |
| Labels | Multi-select list of existing labels. New labels still come from typing `@name`. |
| Reminders | The web's offset presets plus an absolute time. Sends the full `drafts` list, keeping `id` for existing ones so `sync_reminders` diffs correctly. |
| Deadline | Calendar plus "No deadline". |
| Repeat | none / daily / weekdays / weekly / monthly, plus the "keep" option labeled `keepLabel` when present. |
| Project/section | Projects with nested sections. |

- [ ] **Step 1:** Build each picker.
- [ ] **Step 2:** On the simulator, for each chip, check:
  - Picking updates the chip label.
  - For Date, Priority, Labels, Deadline, Repeat and Location, a typed phrase of that kind disappears from the title.
  - The saved task has the value.
  - Editing a task with a custom recurrence keeps it when Repeat is left on "keep".
- [ ] **Step 3:** Commit (`feat(glass): native pickers for every chip`). Push, then check CI.

---

### Task 8: Floating glass buttons

**Files:**
- Create: `crates/plugin-glass/ios/Sources/GlassPlugin/Buttons.swift`
- Modify: `GlassPlugin.swift` (`setButtons`), `crates/ui/src/main.rs` (listeners: `fab` → open the composer with the view's header defaults, the same defaults the view's top `AddTask` uses; `menu` → `drawer.set(true)`; `search` → switch to `View::Search` and focus), `crates/ui/src/style.rs` (hide `.menu-btn` under a `.native-glass` class on `<body>`).

- [ ] **Step 1:** Build the buttons:
  - A round glass hamburger (top left, inside the safe area) and a round glass search button (top right).
  - A green glass + (bottom right, above the home indicator).
  - All are hidden while the card or a picker is open, or while the drawer is open (Dioxus calls `setButtons({visible})` when `drawer` changes).
- [ ] **Step 2:** On the simulator, check:
  - Each button works.
  - The list scrolls under them and taps between them reach tasks.
  - The web hamburger is gone on iOS but still present in `cargo tauri dev` on desktop.
  - Swiping the drawer still works.
- [ ] **Step 3:** Commit (`feat(glass): floating hamburger, search and + buttons`).

---

### Task 9: Phone check, docs, merge

**Files:**
- Modify: `CLAUDE.md` (iOS section: the glass plugin, where it lives, that Swift is driven by `ui::glass`), `docs/superpowers/plans/2026-10-20-liquid-glass-ios.md` (tick boxes, notes).

- [ ] **Step 1:** Install on Noah's phone (CLAUDE.md recipe; rebuild `dist` first). Ask Noah to check:
  - the composer, editor, pickers and buttons
  - tap targets
  - that the keyboard never leaves a gap
- [ ] **Step 2:** Run all checks: `cargo test`; `cd crates/ui && cargo test && cargo check --target wasm32-unknown-unknown`. Both CI jobs must be green on the branch.
- [ ] **Step 3:** Update `CLAUDE.md`. Commit, merge `--no-ff` into `main`, push.
