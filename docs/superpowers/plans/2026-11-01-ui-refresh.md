# UI Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the approved task rows, icons, contrast, corners, spacing, label colors, keyboard-tied pickers, task view, sidebar, search and settings on every platform.

**Architecture:** Data rules (label grouping and order, label color) go in `keel-core` and reach the UI through wasm or IPC. Screen rules (which bottom rows show, keyboard anchoring) are plain `.ts` modules with Vitest tests. Styles stay in `web/src/styles/app.css`.

**Tech Stack:** Rust (`keel-core`, wasm-bindgen), React, TypeScript, Vitest, Biome, Tauri v2, `@phosphor-icons/react` (new).

**Spec:** `docs/superpowers/specs/2026-11-01-task-row-layout-design.md`

## Global Constraints

- Work on branch `ui-refresh`. TDD, one commit per task, `--no-ff` merge to `main` at the end.
- Features ship on all platforms; iPhone differs only in display.
- Never re-implement a data rule in TypeScript.
- New `Action` variants only; existing serialized shapes never change.
- Text meets 4.5:1 in light and dark. Phone tap targets are at least 44pt.
- Phosphor bold, plain, no circles or added edges. Filled weight only for the deadline flag and priority flags.
- The sim-check selectors `.task-row`, `.task-body`, `.task-content`, `.task-badges`, `.task-count`, `.task-project` keep existing or the scripts in `scripts/sim-checks/` are updated in the same task.
- Liquid Glass stays parked. Don't write to the phone's app container.
- Mock backend (`web/src/ipc`) gets a handler for each new command.

## Review Focus

1. A task label with no matching `Label` record (deleted, or differing in case): it still shows, grouped as the default color, and nothing crashes.
2. The keyboard closes while a picker is open (swipe down, hardware keyboard): the task card returns to the screen bottom and the picker stays attached to it.
3. A full when group beside a long project and section: the when group is intact, the project ends in "...", and the row count doesn't change.
4. Choosing Repeat in the date card with no date set: the date follows the existing `repeat_options` anchor (today), and the card returns showing that choice.
5. Removing a task's first label of a color: the color group moves to where its next label was added, per added order.

---

### Task 1: Simulator test of the open picker questions

Throwaway. Nothing from this task's code merges; only its findings do.

**Files:**
- Create: `scripts/sim-checks/picker-fit.js` (kept if useful, otherwise deleted)
- Modify: spec's "Still open" section

- [ ] **Step 1:** Build with `cd crates/src-tauri && cargo tauri ios build --target aarch64-sim --debug` and launch on iPhone 17 `53917026-E8EC-4D54-8C70-A657555D8C47` through the iOS Simulator tool. Open the panel first. Show the software keyboard with its `set_keyboard` action, since CLAUDE.md notes keyboards don't appear on their own in headless simulators.
- [ ] **Step 2:** On a temporary page state, focus the title, then focus a real `<input type="time">`. Screenshot. Record whether iOS shows a wheel in the keyboard's place or a popover, and whether it keeps the page's keyboard area.
- [ ] **Step 3:** Record `visualViewport.height` with the keyboard up, and the height of a 5-row calendar with a Time and a Repeat row (use the mockup's 40px cells) plus the task card. Decide whether the calendar fits above the keyboard. If not, the calendar replaces the keyboard by blurring the title.
- [ ] **Step 4:** Tell Noah the result with screenshots and get the choice. A simulator keyboard can differ from a real one, so re-check on the phone at the end (Task 8).
- [ ] **Step 5:** Write the decision into the spec, revert the temporary code, and commit the spec: `docs: record picker fit decision`.

### Task 2: Phosphor icons

**Files:**
- Modify: `web/package.json`, `web/src/shell/icons.tsx`, `web/src/lists/TaskRow.tsx` (the 🔥 action button)
- Test: `web/src/shell/icons.test.tsx`

**Interfaces:**
- Produces: the existing `*Icon` export names keep working. New: `FireIcon`, `FlagFillIcon`, `BellRingingIcon`, `TagIcon`, `CircleDashedIcon`, `CheckCircleIcon`.

- [ ] **Step 1:** Write `icons.test.tsx`: each export renders one `svg` with `fill="currentColor"` and `aria-hidden="true"`, inside `span.icon`. Run it; the existing stroke icons fail.
- [ ] **Step 2:** Add `@phosphor-icons/react` at an exact version at least two weeks old; `pnpm check`'s dependency vetting must pass. Ask Noah only if vetting rejects it.
- [ ] **Step 3:** Reimplement each export with Phosphor's bold weight at its current size, keeping `span.icon`. `logo()` is untouched. Set the icon colors to `#3d3d3d` on light backgrounds through a token, `--icon`.
- [ ] **Step 4:** Run `cd web && pnpm check`. Expected: pass.
- [ ] **Step 5:** Commit: `feat(web): Phosphor bold icons`.

### Task 3: Contrast, corners and spacing

**Files:**
- Modify: `web/src/styles/app.css`
- Test: `web/src/styles/contrast.test.ts`

**Interfaces:**
- Produces: tokens `--icon`, `--secondary` (light `#5a5a5a`), and radii `--r-card: 20px` (16px at 900px and up), `--r-panel: 20px`, `--r-item: 12px`.

- [ ] **Step 1:** Write `contrast.test.ts`: parse the light and dark token blocks from `app.css` and assert at least 4.5:1 for `--text`, `--secondary`, `--subtle`, `--muted`, `--due-*`, `--p1` as text, against both `--bg` and `--app-bg`. Run it; `--muted` `#8a8a8a` fails.
- [ ] **Step 2:** Fix the tokens. Light values from the spec: secondary `#5a5a5a`, due `#1f6f55`, overdue and deadline `#b83328`, empty circle `#767676`. Choose dark values that pass, tinted rather than inverted.
- [ ] **Step 3:** Apply the spec's Corners and Spacing and type tables. Phone values sit in the existing touch media block. Give `.check`, `.icon-btn`, pills and picker rows a 44pt hit area on the phone with `::after` where the visible size is smaller. Remove the `overflow: hidden` clip on `.task-card` that cuts the circle's hit area, keeping the card's rounded look.
- [ ] **Step 4:** Check 375px and dark mode in the browser preview, then run `pnpm check`. Expected: pass.
- [ ] **Step 5:** Commit: `feat(web): contrast, corners and spacing`.

### Task 4: Label colors and grouping in keel-core

**Files:**
- Modify: `crates/core/src/commands.rs`, `crates/core/src/views.rs`, `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts`, `crates/src-tauri/src/lib.rs`, `crates/src-tauri/src/args.rs`, `crates/src-tauri/tests/commands_list.rs`, `web/src/ipc/commands.ts` and its mock

**Interfaces:**
- Produces, Rust: `Action::LabelSetColor { id: Id, color: String }`; `views::LabelGroup { color: String, names: Vec<String> }`; `views::label_groups(task_labels: &[String], labels: &[Label]) -> Vec<LabelGroup>`.
- Produces, TS: `labelGroups(taskLabels: string[], labels: Label[]): LabelGroup[]`; command `set_label_color` with args `{ id, color }`, wired like the label rename command.

- [ ] **Step 1:** Write the tests:

```rust
// labels: red a, red b, blue c, charcoal d
assert_eq!(label_groups(&["c","a","d","b"], &ls), [("blue",["c"]),("red",["a","b"]),("charcoal",["d"])]);
// Review Focus 5: removing "c" keeps the rest in added order
assert_eq!(label_groups(&["a","d","b"], &ls), [("red",["a","b"]),("charcoal",["d"])]);
// Review Focus 1: an unknown name groups as the default "charcoal", keeping its own spelling
assert_eq!(label_groups(&["zzz"], &ls), [("charcoal",["zzz"])]);
// Names match case-insensitively
assert_eq!(label_groups(&["A"], &ls), [("red",["A"])]);
```

Also: `LabelSetColor` changes only that label's color, is idempotent by uuid, and ignores an unknown id.
- [ ] **Step 2:** Run `cargo test -p keel-core`. Expected: fail to compile.
- [ ] **Step 3:** Implement. Group order is first appearance in `task_labels`. Add the wasm export and the command, then run `scripts/check-bindings.sh`.
- [ ] **Step 4:** Run `cargo test` and `cd web && pnpm check`. Expected: pass.
- [ ] **Step 5:** Commit: `feat(core): label colors and grouping`.

### Task 5: Task row layout

**Files:**
- Create: `web/src/lists/rowLayout.ts`, `web/src/lists/rowLayout.test.ts`, `web/src/lists/SubtaskProgress.tsx`
- Modify: `web/src/lists/TaskRow.tsx`, `web/src/lists/CardList.tsx`, `web/src/lists/CompletedRow.tsx` if it shares the markup, `web/src/styles/app.css`, `web/src/lists/lists.test.tsx`, `scripts/sim-checks/row-badges.js`

**Interfaces:**
- Consumes: `labelGroups`, Phosphor icons.
- Produces: `bottomLayout(hasWhen: boolean, hasLabels: boolean): "one-row" | "two-rows"`, which is `"two-rows"` only when both are true.

- [ ] **Step 1:** Write the tests. `rowLayout.test.ts` covers all four matrix cases. `lists.test.tsx` asserts:
  - DOM order: title with streak, deadline, description, bottom rows.
  - When-group order: sub-tasks, date, repeat, bell. Repeat is an icon with no frequency text.
  - The project shows in every row, "Inbox" when the task has none, including in single-project views.
  - Sub-task states: `0 of 4` dashed, `2 of 5` ring with `stroke-dasharray` at 40%, `3 of 3` check.
  - Label groups render one tag per color, tinted by that color.
  - Review Focus 3: with a long project, the when group has no shrink class.
- [ ] **Step 2:** Run `cd web && pnpm vitest run src/lists`. Expected: fail.
- [ ] **Step 3:** Implement. `TaskRow`'s `location` becomes required. `CardList` passes the project (and section) in every view. When no project lookup exists, it resolves through the Inbox project's name. `.task-project` shortens with the section first (`flex-shrink` 100 on the section), `.task-labels` ends in "...". The hover and long-press actions stay. Keep `.task-badges` and `.task-count` on the new elements so sim checks still find them, and update `row-badges.js` to the new layout. Measure hidden labels for "+N" with a `ResizeObserver` hook in `web/src/lists/useHiddenCount.ts`.
- [ ] **Step 4:** Run `pnpm check`, then `scripts/sim-check.sh 53917026-E8EC-4D54-8C70-A657555D8C47 scripts/sim-checks/row-badges.js end@10`. Expected: pass and correct `[check]` lines. Compare a screenshot against the spec's matrix.
- [ ] **Step 5:** Commit: `feat(web): task row layout`.

### Task 6: Label color picking

**Files:**
- Create: `web/src/editor/labelColors.ts`, `web/src/editor/LabelColorPicker.tsx`
- Modify: `web/src/editor/LabelPicker.tsx`, `web/src/styles/app.css`, `web/src/editor/pickers.test.tsx`, `web/src/styles/contrast.test.ts`

**Interfaces:**
- Consumes: `set_label_color`.
- Produces: `LABEL_COLORS: Array<{ id: string; name: string }>`, with one CSS token `--label-<id>` per color, in light and dark.

- [ ] **Step 1:** Write the tests: each label row shows a tag icon in its color. "Color" opens the 12 choices with a selected ring and a way back. Choosing one sends `set_label_color` and returns to the labels list. Add every `--label-*` token to `contrast.test.ts`, against `--bg` in both themes.
- [ ] **Step 2:** Run them. Expected: fail.
- [ ] **Step 3:** Implement. Existing stored color names (such as `charcoal`) must map to a token, with unknown names falling back to the default. Show Noah the final palette before committing, since colors are a personal choice.
- [ ] **Step 4:** Run `pnpm check`. Expected: pass.
- [ ] **Step 5:** Commit: `feat(web): choose a label's color`.

### Task 7: Task card and pickers on the keyboard

**Files:**
- Create: `web/src/editor/keyboardAnchor.ts`, `web/src/editor/keyboardAnchor.test.ts`
- Modify: `web/src/editor/TaskEditor.tsx`, `web/src/editor/popover.ts`, `web/src/editor/popover.test.ts`, `web/src/editor/Menu.tsx`, `web/src/styles/app.css`, `web/src/editor/editor.test.tsx`

**Interfaces:**
- Produces: `keyboardInset(innerHeight: number, viewportHeight: number, viewportTop: number): number` (never negative); `placeAboveCard(card: Rect, pop: Rect, vw: number): { top: number; left: number }`; hook `useKeyboardInset(): number`.

- [ ] **Step 1:** Write the tests: `keyboardInset(844, 544, 0) === 300`; `keyboardInset(844, 844, 0) === 0` (Review Focus 2); a scrolled viewport `keyboardInset(844, 544, 20) === 280`; `placeAboveCard` keeps an 8px margin and clamps to the top of the screen when the picker is tall. Editor tests: on touch the form has class `on-keyboard`; `.chips` scrolls and the save button sits outside it; the pills are in the order Date, Deadline, Priority, Project, Labels, Reminder, Repeat.
- [ ] **Step 2:** Run them. Expected: fail.
- [ ] **Step 3:** Implement. On touch, the editor is fixed to the bottom at `keyboardInset`, and pickers place with `placeAboveCard` and re-place when the inset changes. Keep the title focused while a picker opens, except where Task 1 decided otherwise. Desktop keeps `placePopover`, with the new corners. On touch there is no Cancel button: tapping outside the task card cancels (Noah's decision). With a picker open, the first tap outside closes the picker and a second cancels.
- [ ] **Step 4:** Run `pnpm check`, then `scripts/sim-check.sh … scripts/sim-checks/quick-add.js end@10`. Expected: pass.
- [ ] **Step 5:** Commit: `feat(web): task card and pickers on the keyboard`.

### Task 8: Date card with Repeat, and final check

**Files:**
- Modify: `web/src/editor/DatePicker.tsx`, `web/src/editor/Scheduler.tsx`, `web/src/editor/TaskEditor.tsx`, `web/src/editor/pickers.test.tsx`, `CLAUDE.md`

**Interfaces:**
- Consumes: `view.repeat_options`, `view.repeat_chip_text`, `pick({ type: "repeat", value })`.

- [ ] **Step 1:** Write the tests: "Pick a date and time" shows the calendar, a Time row and a Repeat row showing the current text. Tapping Repeat shows the options and a way back. Choosing one returns to the card with the new text and doesn't close the picker. Review Focus 4: with no date set, the choice uses the existing anchor. Deadline has no Time or Repeat row. The standalone Repeat pill still works.
- [ ] **Step 2:** Run them. Expected: fail.
- [ ] **Step 3:** Implement as a local `"card" | "repeat"` view state in `DatePicker`, passing the repeat props from `TaskEditor`. Apply Task 1's time picker decision.
- [ ] **Step 4:** Run `cargo test`, `cd web && pnpm check` and `pnpm build`. Then install on the phone and have Noah check: keyboard attachment, calendar fit with a real keyboard, the time picker, tap targets, and rows with real data. Fix what that turns up before Task 9.
- [ ] **Step 5:** Commit: `feat(web): repeat in the date card`.

### Task 9: Task view layout

**Files:**
- Modify: `web/src/taskview/TaskView.tsx`, `web/src/taskview/FieldControl.tsx`, `web/src/taskview/SubtaskCard.tsx`, `web/src/taskview/MoreMenu.tsx`, `web/src/styles/app.css`, `web/src/taskview/taskview.test.tsx`

**Interfaces:**
- Consumes: `SubtaskProgress` from Task 5; `field_layout`, unchanged, which already gives an Inbox task a project pill.

- [ ] **Step 1:** Write the tests. Filled fields come before a single `.tv-pills` row, which comes before the sub-task card, and the row doesn't wrap at any width. A streak-enabled task shows its current and best streak, and the row opens the streak panel. A task without a streak has no such row. An Inbox task shows "+ Project" and no project row. Sub-task header states match Task 5, and with no sub-tasks only "Add sub-task" shows.
- [ ] **Step 2:** Run `cd web && pnpm vitest run src/taskview`. Expected: fail.
- [ ] **Step 3:** Implement. Remove `.tv-pills` wrapping at 900px and up, so desktop mirrors the phone. The ••• menu keeps enabling streaks and Delete.
- [ ] **Step 4:** Run `pnpm check` and `scripts/sim-check.sh 53917026-E8EC-4D54-8C70-A657555D8C47 scripts/sim-checks/subtask-view.js end@10`. Expected: pass.
- [ ] **Step 5:** Commit: `feat(web): task view layout`.

### Task 10: Sidebar and search

**Files:**
- Modify: `web/src/shell/Sidebar.tsx`, `web/src/shell/Sidebar.test.tsx`, the search screen and its tests (find them through the `/` and Cmd+K handlers), `web/src/styles/app.css`, `scripts/sim-checks/drawer-buttons.js`

**Interfaces:**
- Consumes: `TaskRow` from Task 5, with `location` required.

- [ ] **Step 1:** Write the tests. The brand row has the logo first and the theme and settings buttons last. A search field sits below it and opens search on phone and desktop. `/` and Cmd+K still open it. Labels show tags in their color. Results render `TaskRow`, with a result count above them, including the Inbox fallback.
- [ ] **Step 2:** Run them. Expected: fail.
- [ ] **Step 3:** Implement the sidebar cards and move the search entry into the field. Keep `.sidebar` and `.brand .theme-toggle`, and update `drawer-buttons.js` if its positions change.
- [ ] **Step 4:** Run `pnpm check` and the `drawer-buttons.js` sim check. Expected: pass.
- [ ] **Step 5:** Commit: `feat(web): sidebar and search layout`.

### Task 11: Settings layout, and final check

**Files:**
- Modify: `web/src/settings/Settings.tsx`, `web/src/settings/settings.test.tsx`, `web/src/styles/app.css`, `scripts/sim-checks/settings.js` if needed, `CLAUDE.md`

- [ ] **Step 1:** Write the test: sections in the order Notifications, Appearance, Import, Diagnostics, each in one card. Every existing control still works, which the existing tests cover.
- [ ] **Step 2:** Run it. Expected: fail.
- [ ] **Step 3:** Implement. A full screen on the phone, a 560px rounded dialog at 900px and up.
- [ ] **Step 4:** Run `cargo test`, `cd web && pnpm check`, `pnpm build` and the `settings.js` sim check. Check light, dark and 375px in the browser preview. Then install on the phone for Noah's final check of all the screens.
- [ ] **Step 5:** Update CLAUDE.md for Phosphor, the row layout, the keyboard card and the new screens. Commit: `feat(web): settings layout`.
