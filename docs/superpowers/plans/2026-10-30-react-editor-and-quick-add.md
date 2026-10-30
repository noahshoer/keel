# React Plan 3: Editor and quick add Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The React app adds tasks the way the Dioxus app does: the "+ Add task" rows open the task editor, whose title is parsed as quick add with live highlighting, preview chips and Todoist's backspace, and whose chips open the date, deadline, reminder, priority, repeat, project and label pickers. The editor's edit mode (Save) is built and tested here, ready for the task view (Plan 5).

**Architecture:** The editor's data rules leave `crates/ui/src/editor.rs`, `pickers.rs` and `glass.rs` for a new `keel_core::editor` module: `EditorValues`, `ReminderDraft`, `Pick`, `TaskPatch`, `NewTask` and every function that reads or changes them. The Dioxus crate re-exports them, so both UIs run the same code. One core call, `editor_view`, turns the values into everything the editor draws (title segments, preview chips, chip texts, menus, whether it can submit), so the React editor parses the title once per keystroke. Saving is a core plan of `Action`s (`add_plan`, `edit_plan`, `reminder_actions`) that `web/src/editor/save.ts` dispatches in order. Popover placement, the month wheel's motion, week start and touch detection are screen rules in TypeScript.

**Tech Stack:** Rust `keel-core` + ts-rs + wasm-bindgen, React 19, TypeScript (strict), Vitest + Testing Library. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-29-react-frontend-design.md`; roadmap item 3 in `docs/superpowers/plans/2026-10-29-react-foundation.md`; the Plan 2 decisions in `docs/superpowers/plans/2026-10-30-react-lists-and-rows.md`.

## Global Constraints

- Behavior parity with the Dioxus app's no-glass path (`crates/ui/src/editor.rs`, `pickers.rs`); a difference is a bug unless "As built" lists it. Copy strings verbatim (placeholders, titles, hints, problem texts).
- Data rules live in `keel-core` with Rust tests; TypeScript never parses a title, compares a date to today, builds date or repeat text, or decides what an edit changes.
- Keep the Dioxus markup and class names (`.editor`, `.title-wrap`, `.title-mirror`, `.title-input`, `.quick-preview`, `.chips`, `.chip-wrap`, `.popover`, `.scheduler`, `.cal-grid`, `.wheel`, `.label-pop`, `.reminder-pop`, `.add-task-row`, `.view-add`); `app.css` applies unchanged. No CSS changes expected.
- `Action` variants and serialized shapes unchanged. `Pick` and `ReminderDraft` keep their camelCase serde shapes (the glass card sends them from Swift).
- Strict TS, no `any`, Biome clean, `pnpm check` green; `cargo test`, `crates/ui` `cargo test` and its wasm `cargo check` (0 warnings) green.
- Forms opened by a tap focus their field inside the tap (`shell/focus.ts`), so iOS raises the keyboard.
- Commit per task on branch `react-editor`; merge `--no-ff` to `main` at the end.

## Review Focus

1. **Non-ASCII titles.** Spans are UTF-8 byte offsets and the caret is UTF-16. "Café tomorrow p1 🍰" must highlight the right words, and Backspace after "tomorrow" must un-highlight it, not a shifted range (Task 2 test `segments_and_caret_survive_non_ascii`; Task 6 test).
2. **A fast second Enter.** In keep-open mode the form resets to blank on submit, so a second Enter on the empty title does nothing; one task is added, not two (Task 6 test).
3. **A pick replaces the typed phrase.** "call mom tomorrow", then picking Friday from the calendar, saves Friday and the title "call mom", and a typed repeat goes when the day changes (moved Dioxus tests in Task 1; Task 6 UI test).
4. **A label typed as `@new`** that doesn't exist yet is created once before the task is added, and its case-insensitive twin isn't created again (Task 2 `add_plan` test).
5. **Popover near the screen edge.** On a 375px screen a popover opened from the last chip stays inside the viewport and flips above the chip when there's no room below (Task 4 `placePopover` tests).

## Files

| Path | Responsibility |
|---|---|
| `crates/core/src/editor.rs` | new: editor values, picks, chip texts, quick-add merge, edit and add plans |
| `crates/core/src/lib.rs`, `crates/core/tests/ts_bindings.rs` | register the module; export its types |
| `crates/ui/src/{editor,pickers,glass,api}.rs`, `task_view/*.rs` | use the moved code |
| `crates/core-wasm/src/lib.rs` | exports for the editor and calendar |
| `web/src/core/index.ts` | typed wrappers |
| `web/src/editor/popover.ts`, `wheel.ts`, `locale.ts` | screen rules: placement, wheel motion, week start, touch |
| `web/src/editor/{Menu,LabelPicker,ReminderPicker,DeadlinePicker,DatePicker,Scheduler}.tsx` | the pickers |
| `web/src/editor/{TaskEditor,HighlightedTitle,AddTask}.tsx`, `save.ts` | the editor, its title, the add row, saving |
| `web/src/shell/icons.tsx` | the icons the pickers use |
| `web/src/screens/{MainPane,SectionBlock}.tsx` | place the add rows |

---

### Task 1: Editor rules move into keel-core

A move, not a rewrite: the Dioxus bodies and tests go to core unchanged except that `today` becomes a parameter where they read `crate::today()`.

**Files:**
- Create: `crates/core/src/editor.rs`
- Modify: `crates/core/src/lib.rs`, `crates/ui/src/editor.rs`, `pickers.rs`, `glass.rs`, `api.rs`, `task_view/view.rs`, `task_view/fields.rs`

**Interfaces (`keel_core::editor`):**
- `EditorValues` (fields as today; `Serialize`, `Deserialize`, `TS`): `blank(project_id, section_id, date, labels)`, `from_task(&Task)`, `pub fn priority(&self) -> Priority`, `location(&self) -> String`, `due_string(&self, today) -> String`.
- `ReminderDraft` (camelCase serde, `TS`): `from_reminder(&Reminder)`, `label(&self, today)`. `REMINDER_PRESETS`, `reminder_presets() -> Vec<(i64, String)>` (minutes, `offset_text`).
- `Pick` (the glass enum, now with `Serialize` + `TS` too; same serde attributes).
- `TaskPatch` (from `api.rs`, plus `is_empty`) and `NewTask` (from `api.rs`) with `NewTask::into_action(self) -> Action` (the `TaskAdd` that `api::add_task` builds today).
- Functions, same bodies as Dioxus: `repeat_choice`, `recurrence_for(choice, date, existing, today)`, `repeat_text`, `labels_text`, `priority_text`, `repeat_options(date, keep_label, today)` (values become `String`), `repeat_chip_text`, `location_text`, `location_pick(value) -> Pick`, `changes(before, after, existing, today) -> (TaskPatch, Option<(Id, Option<Id>)>)`, `parse_title(v, projects, sections, today)`, `has_name`, `apply_pick`, `toggle_label`, `with_title`, `unhighlight(v, …, end)`, `byte_at_utf16`, `new_task_from(v, qa, today) -> NewTask`, `labels_to_create`, `date_chip(date, time, today) -> (String, Option<DueBucket>)` (the class becomes the bucket; a time alone is `Today`), `deadline_text`, `reminders_text`, `quick_dates(today)`.
- New: `location_choices(projects, sections) -> Vec<LocationChoice>` with `LocationChoice { value: String, label: String, kind: LocationKind /* Inbox | Project | Section */ }` (`TS`); Dioxus `location_items` maps it to `MenuItem`. `custom_reminder_at(text, today) -> Option<String>` (the `ReminderPicker` custom parse: `x {text}`, needs date and time, `YYYY-MM-DDTHH:MM`).
- Dioxus: `editor.rs`, `pickers.rs` and `api.rs` `pub use` the moved items, keeping their call sites; components and `flag_class`/`priority_items`/`location_items` stay.

- [ ] **Step 1: Move the tests first.** Into `editor.rs` `#[cfg(test)]`: `a_label_picked_by_a_new_name_is_created_once`, `edit_changes_leave_quick_add_text_alone`, `a_web_pick_replaces_the_typed_phrase_like_the_card`, `toggling_a_label_keeps_the_typed_ones`, `a_picked_time_is_saved_with_a_typed_date` (today passed in). Add `custom_reminder_needs_a_time` ("tomorrow 9am" on 2026-10-30 → `Some("2026-10-31T09:00")`; "tomorrow" → `None`), `location_choices_nest_sections` (Inbox, then a project, then its section with value `"p|s"` and kind `Section`), `date_chip_texts` (date+time, date, time-only → `("Today 5 PM", Some(Today))`, none → `("Date", None)`).
- [ ] **Step 2: Run** `cargo test -p keel-core editor` → FAIL (no module).
- [ ] **Step 3: Move the code**; point the Dioxus crate at it (glass's own `Pick` and `byte_at_utf16` are deleted in favour of core's).
- [ ] **Step 4: Run** `cargo test`, `cd crates/ui && cargo test && cargo check --target wasm32-unknown-unknown` → green, 0 warnings.
- [ ] **Step 5: Commit** `refactor(core): editor rules move from the Dioxus crate into keel_core::editor`.

### Task 2: What the editor draws, and what saving sends

**Files:**
- Modify: `crates/core/src/editor.rs`, `crates/core/tests/ts_bindings.rs`, `crates/ui/src/editor.rs` (`HighlightedTitle` and the preview use the new functions)

**Interfaces (`keel_core::editor`, all `Serialize` + `TS`):**
- `TitleSegment { text: String, kind: Option<SpanKind>, level: u8 }`; `title_segments(value, spans) -> Vec<TitleSegment>`: the Dioxus `HighlightedTitle` loop (skips overlapping or non-boundary spans; `level` is the priority phrase's last digit, 4 otherwise, 0 for non-priority).
- `PreviewChip { kind: SpanKind, text: String }`; `preview_chips(qa, projects, sections, today)`: the Dioxus `preview` (Date with time, Repeat, Priority `P1`, Project `#Name / Section`, one per Label `@x`, Deadline `Deadline …`), with kind `Date` for the date chip.
- `EditorView { can_submit, segments, preview, shown_labels, repeat_rule: Option<Recurrence>, has_due, location, location_text, locations: Vec<LocationChoice>, priority_text, labels_text, repeat_options: Vec<(String, String)>, repeat_chip_text, date_chip: (String, Option<DueBucket>), deadline_text, deadline_urgent: Option<bool> /* None without a deadline */, reminders_text, reminder_labels: Vec<String> }`; `editor_view(v, quick_add, keep_label: Option<&str>, existing: Option<Recurrence>, projects, sections, today) -> EditorView`, assembled from the Task 1 functions exactly as `TaskEditor` and the pickers compute them (one parse).
- `unhighlight_at(v: &mut EditorValues, projects, sections, today, caret_utf16) -> bool` (`byte_at_utf16` then `unhighlight`).
- `reminder_actions(task_id, before: &[ReminderDraft], after: &[ReminderDraft]) -> Vec<Action>`: the `sync_reminders` order (deletes, then adds).
- `AddPlan { labels: Vec<String>, task: Action, reminders: Vec<ReminderDraft> }`; `add_plan(v, projects, sections, labels: &[Label], today) -> AddPlan`: `add_from_editor` without the I/O (labels the title mentions that no label has, any case).
- `EditPlan { moved: Option<Action>, reminders: Vec<Action>, update: Option<Action> }`; `edit_plan(id, before, after, existing, today) -> EditPlan`: `save_edit` without the I/O (`update` is `None` for an empty patch).
- Dioxus `add_from_editor`, `save_edit`, `sync_reminders` dispatch these plans.

- [ ] **Step 1: Failing tests:** `segments_cover_the_title` ("Call mom tomorrow p2" → `["Call mom ", "tomorrow"(Date), " ", "p2"(Priority, level 2)]`); `segments_and_caret_survive_non_ascii` ("Café tomorrow p1 🍰": "tomorrow" is a Date segment; `unhighlight_at` with the UTF-16 caret after "tomorrow" (13) returns true and the next view has no Date segment; a caret of 5 returns false); `preview_lists_what_was_typed` ("x tomorrow 5pm every day p1 #Home @phone {nov 2}" on 2026-10-30 → Date "Tomorrow 5 PM", Repeat "Every day", Priority "P1", Project "#Home", Label "@phone", Deadline "Deadline 2 Nov"); `editor_view_cannot_submit_a_phrase_alone` ("tomorrow" → `can_submit` false; "x tomorrow" → true; edit mode with "tomorrow" → true); `shown_labels_merge_typed_and_picked`; `add_plan_creates_unknown_labels_once` (known "Home"; title "x @home @Errands @errands" → labels `["Errands"]`, the task's labels hold both); `add_plan_task_is_a_task_add` (date, priority, project, parent, deadline, description trimmed, `None` description when blank); `edit_plan_orders_move_reminders_update` (moved section + one reminder removed + one added + priority → `moved` is `TaskMove`, reminders `[ReminderDelete, ReminderAdd]`, update carries only `priority`); `edit_plan_without_changes_is_empty`.
- [ ] **Step 2: Run** `cargo test -p keel-core editor` → FAIL.
- [ ] **Step 3: Implement**; export the new types in `ts_bindings.rs` (`EditorValues`, `ReminderDraft`, `Pick`, `TaskPatch`, `LocationChoice`, `LocationKind`, `TitleSegment`, `PreviewChip`, `EditorView`, `AddPlan`, `EditPlan`); point Dioxus at the plans and `title_segments`.
- [ ] **Step 4: Run** `cargo test`, UI `cargo test` + wasm check, `scripts/check-bindings.sh` → green.
- [ ] **Step 5: Commit** `feat(core): editor view, title segments and save plans`.

### Task 3: Exports to the web

**Files:**
- Modify: `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts`, `web/src/core/core.test.ts`

**Interfaces (TS, `web/src/core`):** `blankValues(projectId: Id, sectionId: Id | null, date: string, labels: string[]): EditorValues`; `valuesFromTask(t: Task): EditorValues`; `editorView(v, quickAdd: boolean, keepLabel: string | null, existing: Recurrence | null, projects, sections, today): EditorView`; `withTitle(v, text): EditorValues`; `applyPick(v, pick: Pick, quickAdd, projects, sections, today): EditorValues`; `toggleLabel(v, name, quickAdd, projects, sections, today): EditorValues`; `unhighlightAt(v, projects, sections, today, caret: number): EditorValues | null` (`null` when nothing ends there); `locationPick(value: string): Pick`; `addPlan(v, projects, sections, labels, today): AddPlan`; `editPlan(id, before, after, existing, today): EditPlan`; `reminderActions(taskId, before, after): Action[]`; `reminderDraftLabel(d: ReminderDraft, today): string`; `reminderPresets(): Array<[number, string]>`; `customReminderAt(text, today): string | null`; `quickDates(today): Array<[string, string, string]>` (label, hint, date); `monthView(year, month, weekStart: number /* 1 Mon … 7 Sun */, today, selected: string | null, rule: Recurrence | null): MonthView`; `shiftMonth(year, month, delta): [number, number]`; `yearChoices(today, shown): number[]`; `typedTime(text): string | null` (`quickadd::parse_time` as `HH:MM`); `nativeTime(value): string | null | undefined` (core-wasm's port of `pickers::native_time`: `""` → `null`, `HH:MM[:SS]` → `HH:MM`, else `undefined`; its Rust test moves with it); `monthNames(): string[]` (`calendar::MONTHS`, for the wheel).

- [ ] **Step 1: Failing tests** in `core.test.ts` (2026-10-30 as today): `editorView` on "Call mom tomorrow p1" has a Date and a Priority segment and `can_submit`; `applyPick` of a `date` pick strips "tomorrow"; `unhighlightAt` after "tomorrow" returns values whose view has no Date segment, and at 0 returns `null`; `addPlan(...).task.type === "task_add"`; `editPlan` with only a priority change has `update.type === "task_update"` and no move; `customReminderAt("tomorrow 9am")` is `"2026-10-31T09:00"`; `quickDates` starts with `["Today", "Fri", "2026-10-30"]`; `monthView(2026, 10, 1, …).title === "October 2026"` with 6 weeks; `shiftMonth(2026, 12, 1)` is `[2027, 1]`; `typedTime("5pm") === "17:00"`; `nativeTime("")` null, `"07:05:00"` → `"07:05"`, `"5pm"` → undefined.
- [ ] **Step 2: Run** `pnpm vitest run core` → FAIL.
- [ ] **Step 3: Implement** the exports (the existing `input`/`output` pattern and `unchecked_*_type` names) and the wrappers.
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(core-wasm): editor, calendar and reminder exports`.

### Task 4: Screen rules for popovers and the wheels

**Files:**
- Create: `web/src/editor/popover.ts` (+ test), `web/src/editor/wheel.ts` (+ test), `web/src/editor/locale.ts` (+ test)
- Modify: `web/src/shell/icons.tsx`

**Interfaces:**
- `popover.ts`: `type Rect = { top: number; bottom: number; left: number; width: number; height: number }`; `placePopover(chip: Rect, pop: Rect, vw: number, vh: number): { top: number; left: number }` (the Dioxus `PLACE_JS`: below + 6, flipped above when it doesn't fit and above does, clamped to an 8px margin); `usePlacedPopover(ref)`: on mount, positions `ref.current` against its previous sibling with fixed positioning.
- `wheel.ts`: `ROW = 36`; `wrapOffset(i, pos, n): number` (the row's distance from the band, in −n/2…n/2); `flickTarget(pos, velocity): number` (`Math.round(pos - (v * 320) / ROW)`); `installMonthWheel(col: HTMLElement, start: number, onSettle: (month0: number) => void): () => void` and `installYearWheel(col, startIndex, onSettle): () => void`, the `WHEEL_JS` behaviour (drag, flick, tap-to-spin, mouse wheel, year snap-scroll with a 110 ms settle).
- `locale.ts`: `weekStart(): number` (1 Mon … 7 Sun from `Intl.Locale` `getWeekInfo()`/`weekInfo`, default 1; `firstDay` 7 → 7, 6 → 6, else 1, as `pickers::week_start`); `isTouch(): boolean` (`matchMedia('(pointer: coarse)')`).
- Icons ported verbatim from `crates/ui/src/icons.rs`: `CalendarFilledIcon` not needed; add `ClockSmallIcon`, `FlagSmallIcon`, `HashSmallIcon`, `SectionSmallIcon`, `ChevronLeftIcon`, `ChevronDownSmallIcon`, `SofaIcon`, `ArrowRightIcon`, `NoDateIcon`, and `PlusIcon` size 14 (exists).

- [ ] **Step 1: Failing tests:** `placePopover` below the chip; flipped above near the bottom; clamped to 8px at the right edge of a 375px screen; stays at 8px top when neither fits. `wrapOffset(11, 0, 12) === -1`, `wrapOffset(1, 11.5, 12) === 1.5`; `flickTarget(3.2, 0) === 3`, `flickTarget(3, 0.5) === -1` (a fast downward flick coasts back about four rows). `weekStart` with a stubbed `Intl.Locale` returning `firstDay: 7` → 7, missing → 1.
- [ ] **Step 2: Run** `pnpm vitest run popover wheel locale` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): popover placement, wheel motion, week start and picker icons`.

### Task 5: The pickers

**Files:**
- Create: `web/src/editor/{Menu,LabelPicker,ReminderPicker,DeadlinePicker,DatePicker,Scheduler}.tsx`, `web/src/editor/pickers.test.tsx`

**Interfaces:** one open popover per editor: `type Pop = "date" | "deadline" | "reminders" | "priority" | "repeat" | "location" | "labels" | null`, owned by the editor and passed as `open`/`setOpen`.
- `Menu({ id, title, icon, iconClass, text, items: MenuItem[], selected, open, setOpen, onSelect })`, `type MenuItem = { value: string; label: string; icon?: ReactNode; iconClass?: string; indent?: boolean }`.
- `LabelPicker({ labels: Label[], selected: string[], text, open, setOpen, onToggle(name), onChanged })`: the new-label field adds through `addLabel`, then `onChanged()`.
- `ReminderPicker({ reminders: ReminderDraft[], labels: string[] /* view.reminder_labels */, text, hasDue, today, open, setOpen, onChange(drafts) })`.
- `DeadlinePicker({ deadline, text, urgent: boolean | null, today, open, setOpen, onChange(date) })`.
- `DatePicker({ date, time, text, bucket: DueBucket | null, rule: Recurrence | null, today, open, setOpen, onChange(date, time) })` → `Scheduler({ selected, time, withTime, emptyText, rule, today, onPick(date: string | null, time: string), close })`: header, quick dates (icons and `q-*` classes as Dioxus), "No date", month nav (prev, today dot, next), the month/year chooser with the two wheels, the day grid (`cal-day other past is-today selected sel-<bucket> repeat-day`), and the time row: on touch an `<input type="time">` with the "Add time" hint (`nativeTime`), else a text field (`typedTime`, `time-input bad` on failure, Enter commits and closes, blur commits), and "×" when a time is set. A time with no date means today.
- Chip markup and `title`s exactly as Dioxus (`chip chip-btn due-chip <bucket>`, `deadline-chip urgent|set`, `reminder-chip`, `chip-open`, `chip-caret`), plus `.pop-backdrop` drawn by the editor.

- [ ] **Step 1: Failing tests** (mock + wasm): a Menu opens on click, ticks the selected item, picking calls `onSelect` and closes; LabelPicker ticks case-insensitively, toggles, and Enter in "New label, then Enter" dispatches `label_add` and toggles the name on; ReminderPicker presets are disabled without a due and show "Give the task a date to use these:", adding a preset twice keeps one, "×" removes, a custom "tomorrow 9am" adds an absolute draft and "tomorrow" shows the "Needs a time…" problem; DatePicker: "Tomorrow" picks 2026-10-31 and closes; a day cell picks its date; the arrows change the title; typing "5pm" and Enter picks today's date with `17:00`; "bad" marks the field `bad`; with `matchMedia` coarse the time field is `type="time"`; Escape inside the scheduler closes it without reaching the form; DeadlinePicker has no time row and "No deadline" as its empty header.
- [ ] **Step 2: Run** `pnpm vitest run pickers` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): date, deadline, reminder, label and menu pickers`.

### Task 6: The task editor and saving

**Files:**
- Create: `web/src/editor/TaskEditor.tsx`, `HighlightedTitle.tsx`, `save.ts`, `web/src/editor/editor.test.tsx`, `save.test.ts`
- Modify: `web/src/ipc/actions.ts` (export `send(action, temp)` as `dispatchAction`)

**Interfaces:**
- `save.ts`: `addFromEditor(v, projects, sections, labels, today): Promise<void>` (`addPlan`: each label through `addLabel` ignoring failures, then the task, rejecting on failure, then `reminderActions(newId, [], plan.reminders)` ignoring failures; the new id is the first `id_mapping` value); `saveEdit(id, before, after, existing, today): Promise<void>` (`editPlan`: move rejecting, reminders ignoring, update rejecting).
- `HighlightedTitle({ segments, value, placeholder, onInput(text), onEnter(), onUnhighlight(caret): boolean, inputRef })`: `.title-wrap` > `.title-mirror` (spans and `mark.hl.hl-<kind>` from the segment kind: Date/Time → `hl-date`, Repeat → `hl-repeat`, Priority → `hl-p<level>`, Project/Section → `hl-project`, Label → `hl-label`, Deadline → `hl-deadline`; then `​`) and `textarea.name.title-input` (`rows=1`, `spellCheck=false`). Newlines in input become spaces; Enter calls `onEnter`; Backspace with a collapsed caret exactly at a mark's end (summing segment `text.length`, UTF-16) calls `onUnhighlight(caret)` and prevents the delete when it returns true.
- `TaskEditor({ initial, projects, sections, labels, keepLabel, existing, submitText, quickAdd, keepOpen, onSubmit(v), onCancel(), onChanged, focusRef? })`: the Dioxus `TaskEditor`. State: values, `open: Pop`, `submitted`. Renders `form.editor` with Escape (close the popover, else cancel), the backdrop, the title (placeholder `Task name — try “Call mom tomorrow 5pm p1 #Home @phone”` with quick add, else `Task name`), `.quick-preview` chips (`quick-chip chip-<kind>`), `input.desc` "Description", the seven chips in Dioxus order, and the footer (Cancel, submit disabled unless `can_submit`). Submit: ignored when already submitted or unable; keep-open resets to `initial` and closes the popover, else marks submitted; then `onSubmit(values)`.

- [ ] **Step 1: Failing tests:** `save.test.ts`: `addFromEditor` with "Buy milk tomorrow @Errands" adds the label then a task due 2026-10-31 with label "Errands" (mock state); a picked reminder lands on the new task; a failing task add rejects. `saveEdit` with a moved section, a removed reminder and a new priority applies all three in the mock. `editor.test.tsx`: typing "Call mom tomorrow p1" shows `mark.hl-date` and `mark.hl-p1` and preview chips "Tomorrow" and "P1"; "Add task" is disabled for "tomorrow" alone; Enter adds once and two quick Enters add one task (Review Focus 2); after submit the title is empty and focus stays in it; Backspace right after "tomorrow" turns it plain (no `mark`, the text kept) and a second Backspace deletes a character; with "Café tomorrow" the same works (Review Focus 1); picking Friday from the date chip while "tomorrow" is typed leaves "call mom" in the title and the chip reads "Friday" (Review Focus 3); Escape with a popover open closes only the popover, a second Escape calls `onCancel`; edit mode (no quick add, `submitText` "Save") never highlights.
- [ ] **Step 2: Run** `pnpm vitest run editor save` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): task editor with quick add highlighting, and saving`.

### Task 7: "+ Add task" rows

**Files:**
- Create: `web/src/editor/AddTask.tsx`
- Modify: `web/src/screens/MainPane.tsx`, `web/src/screens/SectionBlock.tsx`, `web/src/screens/MainPane.test.tsx`

**Interfaces:**
- `AddTask({ defaults: EditorValues, loaded: Pick<Loaded, "projects" | "sections" | "labels">, main?: boolean, onChanged })`: the collapsed `.add-task-row` (+ `view-add` when `main`) with the 14px plus and "Add task"; a click opens a keep-open quick-add `TaskEditor` (submit text "Add task") and focuses its title inside the click; submit runs `addFromEditor` then `onChanged()`; Cancel collapses.
- `MainPane` places it as Dioxus does, keyed by the view: after the filter groups, after the date groups, and after the unsectioned cards of a project (before the sections); not in Search. Defaults: the project (or Inbox) of a project view, else Inbox; today's date for Today and Upcoming; `[name]` for a label view. `SectionBlock` ends with one for its section (`blankValues(project, section, "", [])`).

- [ ] **Step 1: Failing tests:** Today: "+ Add task" opens the editor with the title focused; typing "Water plants" and Enter adds a task due today that appears in the list after the reload, with the editor still open and empty; a label view's add carries that label; a section's row adds into that section; Search has no add row; Cancel collapses the row.
- [ ] **Step 2: Run** `pnpm vitest run MainPane` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green; `pnpm dev`, add tasks in each view at 375px and desktop width, light and dark, and open every picker near the bottom of the screen.
- [ ] **Step 5: Commit** `feat(web): add-task rows in every list`.

### Task 8: Parity pass, docs, merge

**Files:**
- Modify: `CLAUDE.md` (React section: what works now; Plan 5 next), `docs/superpowers/plans/2026-10-29-react-foundation.md` (roadmap item 3 done), this plan's "As built"

- [ ] **Step 1: Simulator, React build** (`scripts/sim-check.sh` with a new `scripts/sim-checks/quick-add.js`): open "+ Add task" in Today, type "Call mom tomorrow 5pm p1", check the marks and preview chips, submit, check the new row. Open each picker and check it sits inside the viewport. Diag log: no JS errors. The same scenario on the Dioxus build gives the same rows.
- [ ] **Step 2: Phone check for Noah:** the add row raises the keyboard, the time field uses the iOS wheel, popovers fit the screen. Recorded in "As built" when done.
- [ ] **Step 3: Write the docs.**
- [ ] **Step 4: Run** `cargo test`, UI `cargo test` + wasm check, `pnpm check`, `scripts/check-bindings.sh` → green.
- [ ] **Step 5: Commit** `docs: React editor as built`, merge `react-editor` `--no-ff` to `main`, push.

## Left for later plans

- Editing a task in place, "Add sub-task", and the task view's field rows and pills (`Face`): Plan 5, which reuses `TaskEditor`, the pickers and `saveEdit`.
- The glass card's quick add: parked with Liquid Glass.

## As built
