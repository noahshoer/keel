# React Plan 5: Task view and gestures Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tapping a task in the React app opens its task view as the Dioxus app does: a sheet on the phone (two heights, drag from anywhere, swipe down or tap the scrim to close, edge swipe for Back) and a 360px panel on windows at least 900px wide, with the title and description edited in place, field rows and dotted "+ Field" pills that open the editor's pickers and save at once, the ••• menu (Streak, Delete), and the sub-task card. A swipe right or left opens or closes the drawer on narrow layouts.

**Architecture:** What a task view shows about the data moves into `keel-core` (`keel_core::task_view`: which fields are set, the pill order, a level's detail, a task's ancestor chain) and is exported through `core-wasm`; the Dioxus crate re-exports it. Everything else is a screen rule in TypeScript with Vitest tests: the view stack, draft handling, and the 17 embedded gesture scripts' logic (`gestures.rs`) as plain `.ts` modules (`sheet-rules.ts` is the pure decisions, `sheet.ts` / `swipe.ts` install the listeners). Components connect them through a `TaskNav` context. Saving goes through Plan 3's `saveEdit` and `addFromEditor`, the same path as the editor.

**Tech Stack:** Rust `keel-core` + ts-rs + wasm-bindgen, React 19, TypeScript (strict), Vitest + Testing Library. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-23-task-view-design.md` (the view), `docs/superpowers/specs/2026-10-29-react-frontend-design.md` (the port); roadmap item 5 in `docs/superpowers/plans/2026-10-29-react-foundation.md`; Plan 3's "Left for later plans".

## Global Constraints

- Behavior parity with the Dioxus app's no-glass path (`crates/ui/src/task_view/*`, `gestures.rs`, the sheet in `main.rs`); a difference is a bug unless "Decisions" or "As built" lists it. Copy strings verbatim ("+ Description", "Show completed", "Delete task", "Add sub-task", "Task name", "Description", titles such as "Complete sub-task", "Mark as not done", "Open task", "All sub-tasks done").
- Constants copied exactly from `gestures.rs`: `NARROW_MAX 700`, `SHEET_MAX 900`, `MIN_DISTANCE 60`, `MAX_SLOPE 0.6`, `CLOSE_DISTANCE 80`, `FLICK_SPEED 0.5`, `FLICK_MIN 16`, `STRONG_FLICK 1.5`, `DRAG_SLOP 8`, `EDGE_WIDTH 24`, `BACK_SHARE 0.4`, `BACK_FLICK 0.5`, `SETTLE_S 0.3`, ease `cubic-bezier(.32,.72,0,1)`; `CIRCLE_SETTLE_MS 400` (`nav.rs`).
- Keep the Dioxus markup and class names: `.tv-scrim`, `.tv-sheet`, `.tv-grabber`, `.tv-stack`, `.task-view`, `.tv-under`, `.tv-head-space`, `.tv-head`, `.tv-close.tv-round`, `.tv-back`, `.tv-crumb`, `.tv-head-actions`, `.tv-more`, `.tv-body`, `.tv-scroll`, `.tv-titleline`, `.tv-grow`, `.tv-grow-mirror`, `.tv-title`, `.tv-desc`, `.tv-card`, `.tv-fields`, `.tv-row`, `.tv-row-icon`, `.tv-field-btn`, `.tv-repeat`, `.tv-pills`, `.tv-pill-slot`, `.tv-pill`, `.tv-subtasks`, `.tv-sub-head`, `.tv-sub-side`, `.tv-toggle`, `.tv-sub-list`, `.tv-progress`, `.tv-sub`, `.tv-sub-body`, `.tv-add-sub`, `.tv-add-editor`, `.tv-large`, `data-detent`, `data-view-id`, and `.app.view-open`. `app.css` applies unchanged; no CSS changes expected (if one is needed, it goes in both `app.css` and `style.rs`).
- Contracts: `window.keelKeyboard({ kb, secs, ease })` (called by `crates/plugin-ios` `Keyboard.swift`); `scripts/sim-checks/subtask-view.js` and `edge-back.js` must pass on the React build. No `plugin:glass|*` calls, no `.native-glass` paths.
- Data rules in `keel-core` with Rust tests; TypeScript never decides whether a field is set, how sub-tasks count, or a task's ancestry. `Action` variants and serialized shapes unchanged.
- Strict TS, no `any`, Biome clean; `pnpm check`, `cargo test`, `crates/ui` `cargo test` and its wasm `cargo check` (0 warnings), `scripts/check-bindings.sh` green.
- Forms opened by a tap focus their field inside the tap (`shell/focus.ts`).
- Commit per task on branch `react-task-view`; merge `--no-ff` to `main` at the end.

## Decisions

- **Keyboard on the phone without glass.** In Dioxus the sheet's keyboard handling (`viewport`, `keelKeyboard`) only runs under `.native-glass`, i.e. on iPhone with glass. The React app has no glass, so it gates the same code on `currentPlatform() === "touch"` and `visualViewport.scale === 1`. Without this the sheet would sit under the keyboard on the phone, which the Dioxus no-glass path never had to face.
- **The ancestor chain is data** (`keel_core::views::chain`), the stack itself is screen state (`web/src/taskview/stack.ts`).
- **Pickers take an optional `face`** (`{ className, text, icon }`), as Dioxus `Face`: with it the trigger uses those classes and text, with the icon only when `icon`, and no caret or ⟳.

## Review Focus

1. **Typing then leaving fast.** Type in the title, then tap Back (or a sub-task, or the scrim) before blurring: the draft saves once, the view moves once, and when the save's reload comes back a draft typed since stays (Task 3 `takesStoredText` tests; Task 6 UI test "typed title saves on Back").
2. **A double tap on a circle.** The second tap must not complete the task twice or hit the parent's circle that took its place after the pop (Task 5 `holdCircles` test; Task 6 UI test).
3. **The task is deleted or completed elsewhere while open.** Deleted: its view and every view above it close on the next load. Completed elsewhere: the view stays, shown done (Task 3 `prune` tests; Task 7 App test).
4. **iOS sends Enter as a newline.** "Buy milk\n" in the title commits and blurs; the description keeps its line breaks (Task 3 `typedText` tests).
5. **An emptied title.** Clearing the title and blurring puts the saved title back and saves nothing (Task 6 UI test).

## Files

| Path | Responsibility |
|---|---|
| `crates/core/src/task_view.rs` | new: `Field`, `Missing`, `field_layout`, `TaskDetail`, `detail` |
| `crates/core/src/views.rs` | `chain(id, all)` |
| `crates/ui/src/task_view/{fields,nav}.rs` | use the moved code |
| `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts` | exports |
| `web/src/taskview/stack.ts`, `drafts.ts` | the view stack; title / description draft rules |
| `web/src/taskview/nav.tsx` | `TaskNav` context and hook |
| `web/src/taskview/{TaskView,FieldControl,SubtaskRow,GrowingText,TaskSheet}.tsx` | the view |
| `web/src/gestures/sheet-rules.ts`, `sheet.ts`, `swipe.ts` | gesture decisions; sheet drag, edge back, keyboard; drawer swipe |
| `web/src/editor/{Menu,DatePicker,DeadlinePicker,ReminderPicker,LabelPicker}.tsx`, `pop.ts` | `face` |
| `web/src/lists/{TaskRow,CardList,CompletedRow}.tsx`, `web/src/app/App.tsx` | open the view; host the sheet |

---

### Task 1: Task view data rules move into keel-core

A move: the bodies and tests of `fields::layout` and `nav::detail` go to core unchanged; `chain` is `ViewStack::open_chain`'s walk.

**Files:**
- Create: `crates/core/src/task_view.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/src/views.rs`, `crates/core/tests/ts_bindings.rs`, `crates/ui/src/task_view/fields.rs`, `crates/ui/src/task_view/nav.rs`

**Interfaces (`keel_core`):**
- `task_view::Field` (`Description | Date | Deadline | Priority | Labels | Reminders | Location`) and `task_view::Missing` (`Field(Field) | Repeat`, serde `#[serde(tag = "kind", content = "field")]`), both `Serialize` + `TS`; `Missing::label(&self) -> &'static str`.
- `task_view::FieldLayout { fields: Vec<Field>, missing: Vec<MissingPill> }`, `MissingPill { missing: Missing, label: String }` (`TS`); `field_layout(v: &EditorValues, inbox_id: &Id) -> FieldLayout` (the `layout` body).
- `task_view::TaskDetail { task, parent: Option<Task>, progress: Option<Progress>, visible: Vec<CardRow> }` (`Serialize` + `TS`); `detail(id, all, show_completed) -> Option<TaskDetail>`.
- `views::chain(id: &Id, all: &[Task]) -> Vec<Id>`: `id` and its live ancestors, top-level first, stopping at a missing or deleted parent or a cycle.
- Dioxus: `fields.rs` keeps `subtask_header`, `pending_save_before_nav`, `takes_stored_text` and re-exports the rest; `ViewStack::open_chain` calls `views::chain`; `nav::detail` re-exports core's.

- [ ] **Step 1: Move the tests first:** `layout_*`, `newly_set_field_lands_in_order`, `empty_description_whitespace_is_missing`, `pill_labels` into `task_view.rs` (calling `field_layout(...).fields` / `.missing`); `detail_*`, `visible_rows_carry_their_own_progress`, `completed_elsewhere_stays_open`'s detail half; `open_chain_*` as `chain_*` tests in `views.rs` (`chain(G) == [P, C, G]`, top-level `[P]`, missing / deleted parent `[C]`, cycle terminates with `A` last and length ≤ 2). Add `missing_serializes_tagged` (`Missing::Repeat` → `{"kind":"Repeat"}`, `Missing::Field(Deadline)` → `{"kind":"Field","field":"Deadline"}`).
- [ ] **Step 2: Run** `cargo test -p keel-core task_view chain` → FAIL (no module).
- [ ] **Step 3: Move the code**, point the Dioxus crate at it, export `Field`, `Missing`, `MissingPill`, `FieldLayout`, `TaskDetail` in `ts_bindings.rs`.
- [ ] **Step 4: Run** `cargo test`, `cd crates/ui && cargo test && cargo check --target wasm32-unknown-unknown`, `scripts/check-bindings.sh` → green, 0 warnings.
- [ ] **Step 5: Commit** `refactor(core): task view layout, detail and ancestor chain move into keel-core`.

### Task 2: Exports to the web

**Files:**
- Modify: `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts`, `web/src/core/core.test.ts`

**Interfaces (TS, `web/src/core`):** `fieldLayout(v: EditorValues, inboxId: Id): FieldLayout`; `taskDetail(id: Id, all: Task[], showCompleted: boolean): TaskDetail | null`; `chain(id: Id, all: Task[]): Id[]`; `progressLong(p: Progress): string` ("2 of 5").

- [ ] **Step 1: Failing tests** in `core.test.ts`: `fieldLayout` of an Inbox task with a description, date, priority 2 and a label → `fields` `["Description","Date","Priority","Labels"]`, `missing` labels `["+ Repeat","+ Deadline","+ Reminders","+ Location"]`; `taskDetail` hides a checked sub-task unless `showCompleted`, carries `parent`, and is `null` for an unknown id; `chain` of a grandchild is `[P, C, G]`; `progressLong({done: 2, total: 5}) === "2 of 5"`.
- [ ] **Step 2: Run** `cd web && pnpm vitest run core` → FAIL.
- [ ] **Step 3: Implement** (the existing `input`/`output` pattern).
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(core-wasm): task view layout, detail and chain exports`.

### Task 3: The view stack and draft rules

**Files:**
- Create: `web/src/taskview/stack.ts` (+ `stack.test.ts`), `web/src/taskview/drafts.ts` (+ `drafts.test.ts`)

**Interfaces:**
- `stack.ts` (immutable; a stack is `readonly Id[]`, oldest first): `openChain(id, all): Id[]` (`chain`), `push(s, id)`, `back(s)` (pop; empty stays empty), `prune(s, all: Task[])` (cut at the first id not in `all`, returns `s` itself when nothing changed), `top(s): Id | null`, `under(s): Id | null` (the one below the top).
- `drafts.ts`: `pendingSaveBeforeNav(before: EditorValues, draft: EditorValues): boolean`; `takesStoredText(stored, seen, saved): boolean`; `typedText(raw: string, multiline: boolean): { text: string; enter: boolean }`; `commitDrafts(base: EditorValues, title: string, desc: string): { title: string; save: { before: EditorValues; after: EditorValues } | null }` (the `commit` closure in `view.rs`: trimmed title, an empty one reverts to `base.content`; `save` is null when nothing changed).

- [ ] **Step 1: Failing tests**, ported from `nav.rs` / `fields.rs` / `view.rs`: `back_pops_then_closes`, `prune_cuts_at_deleted`, `prune_cuts_even_if_later_ids_exist`, `prune_everything_closes`, `completed_elsewhere_stays_open` (a checked task stays in the stack), `prune` returns the same array when unchanged, `under` of `[A, B]` is `A` and of `[A]` null; `pending_save_detects_title_and_description`, `our_own_save_coming_back_keeps_the_draft`, `a_newline_in_the_text_is_an_enter`, `a_multiline_field_keeps_its_line_breaks`; `commitDrafts`: `"  Pack bag "` → title `"Pack bag"` and a save with `after.content === "Pack bag"`; `""` → title back to `base.content`, `save` null; same text → null.
- [ ] **Step 2: Run** `pnpm vitest run stack drafts` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): task view stack and draft rules`.

### Task 4: Gesture rules

The pure parts of `gestures.rs`, with all its Rust tests ported as Vitest (same names, same numbers).

**Files:**
- Create: `web/src/gestures/sheet-rules.ts` (+ `sheet-rules.test.ts`)

**Interfaces:**
- `type Swipe = { dx: number; dy: number; width: number; blocked: boolean; inSheet: boolean }`; `drawerSwipe(s: Swipe, open: boolean): boolean | null`.
- `type Detent = "medium" | "large"`; `type DragStart = { scrollTop: number; blocked: boolean; inField: boolean; fieldFocused: boolean; width: number; detent: Detent }`; `type SheetDrag = "pending" | "drag" | "scroll" | "ignore"`; `sheetDragAction(s, dx, dy): SheetDrag`; `holdsScroll(s, dx, dy): boolean`; `dragOffset(start, dy)`; `dragFrom(detent, settling: number | null, medium)`; `type Settle = Detent | "close"`; `sheetSettle(from: Detent, y, medium, velocity): Settle`; `settleOffset(settle, medium)` (`Infinity` for close); `keyboardExpands(detent, width, kb)`; `keyboardInset(clientH, vvTop, vvH, held: number | null)`; `atEdge(x)`; `type EdgeBack = "pending" | "drag" | "ignore"`; `edgeBackAction(can, dx, dy): EdgeBack`; `edgeOffset(dx, width)`; `edgeBackCommits(dx, velocity, width)`; `velocity(samples: Array<[number, number]>): number` (first to last sample, 0 with under two or no elapsed time).
- Constants from Global Constraints, exported (`SHEET_MAX`, `SETTLE_S`, `EASE`, …).

- [ ] **Step 1: Failing tests:** every `#[test]` in `gestures.rs` except `a_touch_on_the_scrim_counts_as_in_the_sheet` (that one moves to Task 5), with the same inputs and expected values; plus `velocity([[0,0],[100,50]]) === 0.5` and `velocity([[5,0]]) === 0`.
- [ ] **Step 2: Run** `pnpm vitest run sheet-rules` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): sheet, edge-back and drawer gesture rules`.

### Task 5: Gesture listeners and the nav context

**Files:**
- Create: `web/src/gestures/swipe.ts` (+ test), `web/src/gestures/sheet.ts` (+ `sheet.test.ts`), `web/src/taskview/nav.tsx` (+ `nav.test.tsx`)

**Interfaces:**
- `swipe.ts`: `installSwipe(doc: Document, onSwipe: (s: Swipe) => void): () => void` (`SWIPE_JS`: `BLOCK` selector verbatim, `inSheet` from `.tv-sheet, .tv-scrim`).
- `sheet.ts`: `installSheet(doc: Document, win: Window, onClose: () => void): () => void` installs `SHEET_DRAG_JS` without glass: the vertical drag (touchstart / touchmove non-passive / touchend / touchcancel, capture), the edge swipe back (which clicks `.tv-back` after sliding), `win.keelKeyboard` and the visual-viewport listeners (`--vv-top`, `--vv-h`, `--kb`, gated per Decisions on touch and `scale === 1`); `slideClose(doc, win): boolean` (the `keelSheetSlideClose` body: false at ≥ 900px; true and `onClose` after the slide). `onClose` replaces `dioxus.send({ close: true })`. Reduced motion snaps. Removing listeners and `keelKeyboard` on uninstall.
- `nav.tsx`: `TaskNavContext` with `{ stack: Id[]; showCompleted: boolean; addSub: boolean; closeAsked: boolean; open(id) /* openChain, showCompleted off */; openAdding(id); push(id); back(); close(); askClose() /* only when open */; setShowCompleted(b); clearAddSub(); clearCloseAsked(); completedHere() /* back */; holdCircles(): boolean; releaseCircles(): Promise<void> /* after 400 ms */; prune(all) }`; `TaskNavProvider({ all: Task[], children })` (keeps `all` for `open`) and `useTaskNav()`.

- [ ] **Step 1: Failing tests** (jsdom, synthetic `TouchEvent`s with a `touches` list; fake timers): `swipe`: a 120px rightward touch reports `{dx:120, …, inSheet:false}`; one starting in `.tv-scrim` reports `inSheet: true` (moved Rust test); one starting in an `input` reports `blocked: true`. `sheet`: at 390px, a 100px downward drag on `.tv-sheet` at rest at medium adds `.tv-large` during the drag and calls `onClose` after 300 ms; a 30px drag doesn't and leaves `data-detent="medium"`; at 1280px nothing moves; a 150px edge swipe from x 6 inside a sheet with `.tv-back` clicks it after 300 ms, a 40px one doesn't; `slideClose` is false at 1280px and calls `onClose` once at 390px even when called twice; `keelKeyboard({kb: 328, secs: 0.25, ease: "ease"})` on a touch platform sets `data-detent="large"` on a medium sheet, and is a no-op on `"other"`. `nav`: `open` of a grandchild gives `[P, C, G]` and turns show-completed off; `askClose` with an empty stack leaves `closeAsked` false; `holdCircles` is true then false until `releaseCircles` resolves after 400 ms.
- [ ] **Step 2: Run** `pnpm vitest run swipe sheet nav` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): sheet drag, edge back, keyboard and drawer swipe listeners; task nav context`.

### Task 6: The task view

**Files:**
- Create: `web/src/taskview/{TaskView,FieldControl,SubtaskRow,GrowingText}.tsx`, `web/src/taskview/taskview.test.tsx`
- Modify: `web/src/editor/pop.ts` (`Face`), `web/src/editor/{Menu,DatePicker,DeadlinePicker,ReminderPicker,LabelPicker}.tsx` (optional `face`), `web/src/editor/pickers.test.tsx`

**Interfaces:**
- `pop.ts`: `type Face = { className: string; text: string; icon: boolean }`. Each picker's trigger with a `face`: `className` plus `chip-open` when open, the picker's icon only when `face.icon`, then `face.text`; no caret, no ⟳ (Dioxus `Face`).
- `GrowingText({ className: "tv-title" | "tv-desc", value, placeholder, multiline?, autoFocus?, onInput(text), onCommit() })`: `.tv-grow` > `.tv-grow-mirror` (`value` + `​`) and `textarea rows=1`; input runs `typedText`, and an Enter (keydown when single-line, or a typed newline) blurs, and the blur commits. Logs `"<class>: Enter keydown"` / `"<class>: newline typed"` through `diag.ts`.
- `FieldControl({ field: Field | null, missing: Missing | null, values, view: EditorView, labels, today, open, setOpen, onPick(p: EditorPick), onToggle(name), onLabelsChanged, onOpenDescription })`: the non-native branch of `view.rs`'s `field` closure. Date row: `DatePicker` with face `tv-field-btn due-chip <bucket>` and the filled calendar icon, then, when the task repeats, the repeat `Menu` with face `tv-field-btn tv-repeat`; Repeat pill: the repeat `Menu`; Deadline: `deadline-chip urgent|set`, text `Deadline …`; Priority: `Priority n` with the flag in its colour; Labels: names joined by ", "; Reminders: `reminders_text`; Location: `location_text`. A pill's face is `tv-pill` with the pill's label and no icon; "+ Description" is a plain `button.tv-pill` that calls `onOpenDescription`.
- `SubtaskRow({ row: CardRow, onOpen(id), onChanged })`: `.tv-sub[.done]`, the circle (`check done` when checked; titles "Mark as not done" / "Complete sub-task") completing or reopening under `holdCircles`, `.tv-sub-body` (title "Open task") with `.task-content`, `SubtaskCount`, `.card-chev`.
- `TaskView({ detail: TaskDetail, depth: number, active: boolean, loaded: Loaded, onChanged })`: `view.rs` `TaskView` without the native branches. State: `base`, `seen`, `title`, `desc`, `descOpen`, `adding`, `streak`, `moreOpen`, `open: Pop`. The stored values are `valuesFromTask(task)` with this task's reminders as drafts; what the fields draw comes from one `editorView(base with desc, false, keepLabel, rule, …)` per render (`keepLabel` is the rule's text when `repeat === "keep"`), and picks go through `applyPick` / `toggleLabel` with quick add off, then `saveEdit` when the values changed. On a change of them, `takesStoredText` decides each draft. `navigate(to)` commits drafts (save fire-and-forget, then `onChanged`) and moves at once. While `active`: `addSub` opens the sub-task editor and is cleared; `closeAsked` navigates to close and is cleared. Header: `.tv-close` (title "Close"; `slideClose` else close), `.tv-back` when `depth > 1`, `.tv-crumb` (parent name, else `location_text` of the project), ••• (`.tv-more`, title "More") with "Streak" for a repeating task and "Delete task" (`menu-item danger`; deletes, and the reload prunes). Title line: circle (`holdCircles`; saves drafts, then uncompletes, or completes and `completedHere`; reload; release), `GrowingText` title. Fields card from `fieldLayout` (+ "+ Description" opens an empty description row that autofocuses). Streak panel. Sub-task card: header from `progressLong` and the toggle text, `done-check` when all done, `.tv-progress` width `done*100/max(total,1)`%, rows, then the "Add sub-task" `TaskEditor` (blank values with this task as parent, quick add, keep-open, submit text "Add sub-task", `addFromEditor`) or the `.tv-add-sub` button. An inactive view renders `.task-view.tv-under` (`aria-hidden`) with `.tv-head-space` in place of the header.

- [ ] **Step 1: Failing tests** (mock backend + wasm; fixtures get a parent with three sub-tasks, one checked, one with its own sub-task): `pickers.test.tsx`: a `DatePicker` with a face renders the face's class and text with the icon and no ⟳, and still opens. `taskview.test.tsx`: the header shows Close, the crumb (project name) and More, no Back at depth 1; "2 of 3"-style count, "Show completed" toggles to "Hide completed" and shows the checked row struck through; fields show as rows in order and missing as pills ("+ Deadline" …); picking "Priority 1" from the "+ Priority" pill saves (mock task priority) and the row "Priority 1" appears; "+ Description" opens a focused description field, typing and blurring saves it; typed title saves on Back (Review Focus 1): type "Pack bag", click `.tv-back` → one `task_update` with content "Pack bag" and the stack popped; an emptied title reverts and sends nothing (Review Focus 5); a double click on the title-line circle completes once (Review Focus 2) and pops; a sub-task's circle completes it and the view stays; a checked sub-task's circle reopens it; clicking `.tv-sub-body` pushes its view; "Delete task" in ••• deletes and the view closes after the reload; "Add sub-task" opens the editor and Enter adds a sub-task with this parent, the editor staying open; an inactive view has `.tv-under` and `.tv-head-space` and ignores `closeAsked`.
- [ ] **Step 2: Run** `pnpm vitest run taskview pickers` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green.
- [ ] **Step 5: Commit** `feat(web): task view with fields, pills, sub-tasks and in-place editing`.

### Task 7: The sheet in the app, rows that open it, drawer swipes

**Files:**
- Create: `web/src/taskview/TaskSheet.tsx`
- Modify: `web/src/app/App.tsx`, `web/src/app/App.test.tsx`, `web/src/lists/TaskRow.tsx`, `web/src/lists/CardList.tsx`, `web/src/lists/CompletedRow.tsx`, `web/src/lists/lists.test.tsx`

**Interfaces:**
- `TaskSheet({ loaded: Loaded, onChanged })`: when the stack has a top whose `taskDetail` exists, `.tv-scrim` (click: `slideClose` else `askClose`) and `.tv-sheet` > `.tv-grabber` + `.tv-stack` with the under view (inactive) then the top view (active), keyed by task id so a push or Back keeps mounted views. Nothing when closed.
- `App`: wraps in `TaskNavProvider` (`all` from `loaded.all`); `.app` gets `view-open` when a view is open; after each load, `prune(loaded.all)`; installs `installSwipe` (drawer: `drawerSwipe` against the current drawer state), `installSheet` (its `onClose` → `askClose`); renders `TaskSheet` between the sidebar and `MainPane`.
- Rows: `TaskRow`'s `.task-body` click → `open(task.id)`; its "Add sub-task" action → `openAdding(task.id)`; `CardHead` click → `open(head id)`; `CompletedRow` body → `open(id)`. Search rows already use `TaskRow` / `CompletedRow`.

- [ ] **Step 1: Failing tests:** `lists.test.tsx`: clicking a row's body opens its chain (nav stack spy); "Add sub-task" opens adding; a card head opens its task. `App.test.tsx`: clicking a row's `.task-body` shows `.tv-sheet` with the task's title and `.app.view-open`; clicking `.tv-scrim` at desktop width closes it; deleting the open task from another row's actions closes the sheet after the reload (Review Focus 3); a rightward synthetic swipe at 390px opens the drawer and a leftward one closes it.
- [ ] **Step 2: Run** `pnpm vitest run App lists` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm check` → green; `pnpm dev`: open a task at 375px and at desktop width, light and dark; push into a sub-task and back; open each field's picker from a row and a pill.
- [ ] **Step 5: Commit** `feat(web): task sheet and panel, rows open it, drawer swipes`.

### Task 8: Parity pass, docs, merge

**Files:**
- Modify: `CLAUDE.md` (React section: task view and gestures work; Plan 6 next), `docs/superpowers/plans/2026-10-29-react-foundation.md` (roadmap item 5 done), `docs/superpowers/plans/2026-10-30-react-plan-2-backlog.md` (tick "the drawer's swipes and the other gestures"), this plan's "As built"

- [x] **Step 1: Simulator, React build** (iPhone 17 with test data): `scripts/sim-check.sh <udid> scripts/sim-checks/subtask-view.js end@12` and `edge-back.js end@14` → the same `[check]` lines as the Dioxus build gives (header buttons differ only by the glass ones; record the lines in "As built"). Diag log: no JS errors.
- [ ] **Step 2: Phone check for Noah:** open a task (sheet at medium), drag up to large and down to close, tap the scrim; push a sub-task and edge-swipe back; edit the title and description (the keyboard keeps the sheet's header in view); swipe the drawer open and closed. Recorded in "As built" when done.
- [x] **Step 3: Write the docs.**
- [ ] **Step 4: Run** `cargo test`, UI `cargo test` + wasm check, `pnpm check`, `scripts/check-bindings.sh` → green.
- [ ] **Step 5: Commit** `docs: React task view as built`, merge `react-task-view` `--no-ff` to `main`, push.

## Left for later plans

- Settings, import, reminder settings and the diagnostics toggle: Plan 6.
- The glass X, ‹ and ••• over the sheet (`keelSheetOpened`, `plugin:glass|sheet`): parked with Liquid Glass.

## As built

Deviations from this plan:

- `useTaskNav`, the `TaskNav` type, the context and `CIRCLE_SETTLE_MS` are in `web/src/taskview/nav-context.ts`, and `nav.tsx` keeps only the provider: Biome's `useComponentExportOnlyModules` wants component files to export only components.
- `core-wasm` exports three more helpers the view needed: `locationText`, `repeatText` and `reminderDraftOf`.
- Extra files: `SubtaskCard.tsx` (the sub-task card), `MoreMenu.tsx` (the ••• menu), `view-rules.ts` (the view's small screen rules, with tests) and `editor/items.tsx` (menu items shared by the editor's chips and the view's fields).
- `FieldControl` takes a `MissingPill`, so a pill's label comes from core.
- `window.keelSheetViewport` isn't ported: nothing calls it.
- `edgeBackAction` drops the unused `x` parameter.
- The sheet and `.app.view-open` draw from the stack pruned against the current load in the render itself (`prune` in `TaskSheet` and `Frame`; the effect still stores it), as Dioxus prunes in the loader: deleting an open sub-task makes its parent active in the same sheet instead of closing and reopening it.
- No `pnpm dev` browser walk for Task 7 (no Chrome on this Mac). The simulator checks below cover the sheet.

Checked in the simulator (React build, iPhone 17, test data, 2026-10-30), the same `[check]` lines as the Dioxus build gave on 2026-10-28 except for the header buttons:

- `subtask-view.js`: `header: Back / Pack for Boston /`, `header buttons: tv-close 36, tv-back 56, tv-more 36`, `after Back: tv-close 36, tv-more 36`. The Dioxus build had glass there (`Pack for Boston /`, all three 40 wide, no "Back" text); 36px round buttons and a "Back" label are its no-glass styles.
- `edge-back.js`: `in sub-task: ? back=true`, `after short swipe: ? back=true`, `mid-drag: under visible "Pack for Boston" at translateX(-94px); top moved translateX(120px)`, `before: under 01M5NWXPPQ0J8CHFERGBQWSH4J views 2`, `parent kept, not remounted: true …; views 1; styles left ""`, `after long swipe: ? back=false`. (`?` is the script's title selector matching the `.tv-title` wrapper, as in Dioxus.)
- Screenshots: the sheet at medium over Today with the date and project rows, the "+ Description / + Repeat / + Deadline / + Priority" pills and "0 of 3 · Show completed"; the sub-task's view with "‹ Back  Pack for Boston"; mid-drag, the parent sliding in under the sub-task. The diag logs have no JS errors.

Phone check (Task 8 step 2): pending, for Noah. Open a task (sheet at medium), drag up to large and down to close, tap the scrim; push a sub-task and edge-swipe back; edit the title and description (the keyboard keeps the sheet's header in view); swipe the drawer open and closed.
