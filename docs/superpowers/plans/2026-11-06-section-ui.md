# Sections: Add section and their menus (2026-11-06, issue #9)

Problem: on the phone, a project's "Add section" is a divider row drawn at 0%
opacity and shown only on mouse hover, and a section's rename and delete
buttons are hover-only too. Mocks approved by Noah (artifact "Keel #9 Section
UI Options"): option A for the row, and the project's menu pattern for sections.

Decisions:
- "Add section" is a row like "+ Add task" (a plus in an accent ring, accent
  text), always visible, under a hairline at the end of the project. The
  divider lines go.
- The project's ••• menu (`ViewMenu`, `ActionPopup`) gets an "Add section" entry
  above Edit and Delete, on a project page only. `ActionPopup` takes an optional
  `lead` action for it. The entry opens the same name field as the row, at the end
  of the project, inside the tap (so iOS raises the keyboard): `AddSection` hands
  its opener to the page through a ref, as `AddTask` would.
- Each section header gets a ••• (always visible; `ViewMenu` with its own
  label) opening the project's popup: Edit turns the title into its field
  (the existing inline rename), Delete asks first in the same popup with the
  section's task count. The header's inline confirm box and its hover-only
  pencil and trash go. A long press or right click on the header opens the
  same popup (`ROW_PRESS`, via `RowMenu`; `.section-row` joins the long-press
  rows).
- Sections still go at the end of the project (`section_add` has no position).

## Moving tasks between sections (added 2026-11-06, mocks approved)

- Drag: a touch drag starts from a hold on a task (`ROW_HOLD`, sent at 500 ms
  to `.task`) and begins once the finger has moved past the slop; a mouse drag is
  a press on a row that moves 6 px. `drag.ts` (rules in `drag-rules.ts`) makes the
  ghost, outlines the `[data-drop-zone]` under the finger (a project's own tasks,
  and each `SectionBlock`, via `DropZone`) and sends it `TASK_DROP` on release,
  which sends `task_move` (the end of that section; no order within a section).
  The page scrolls near the list's top and bottom. Only top-level tasks
  (`data-draggable`) and only where a drop zone exists.
- Timing (Noah, 2026-11-06): a half-second hold (`HOLD_MS`) lifts the row and opens
  its menu together (`ROW_HOLD`, then `ROW_PRESS`), finger still down. The lifted
  task can still be dragged: moving past the slop starts the drag, and `ROW_DRAG`
  closes the menu. Lifting without moving leaves the menu open. While a task is
  held, its touchmoves are `preventDefault`ed, so a few pixels of page scroll can't
  close the menu or take the touch. A right click still opens the menu at once.
  (Replaces the earlier menu-on-lift and menu-after-another-500-ms versions.)
- "Move to…" in the task's menu lists the projects with their sections
  (`location_choices`, exported from the wasm build) and sends the same `task_move`;
  it moves a task to another project too.
- Rows are `user-select: none`, or iOS's text selection takes the hold over.
- The dragged row's place is marked with `data-drag-source`, not a class: the
  row's component resets its `className` when the hold ends.
- The Add section row is also a drop zone (`data-drop-new`, never "where the task
  already is"): letting a task go on it opens the name field with the task waiting;
  naming the section creates it (`addSectionGetId`) and then moves the task in. Closing
  the field (Escape, a tap outside) drops the wait, and nothing moves. A project with no
  sections has only this zone.
- While the section is being named, the waiting task shows (option A on the canvas,
  Noah 2026-11-06): a dashed copy with "Moves here" sits under the field (`.move-preview`,
  inside the form so a tap on it keeps the field open), and the task's own row stays
  dashed (`data-move-waiting`). Both go when the field closes or the move is done.
- The keyboard after a drop on Add section: the field is focused inside the drop's
  touchend, which WebKit doesn't count as the user interacting. WebKit lets such a focus
  raise the keyboard only once the person has focused a field by a tap since the page
  loaded (from the logs, not WebKit's source). So the first real tap anywhere focuses a
  hidden `inputmode="none"` field and lets it go (`web/src/shell/keyboardPrime.ts`,
  touch only, once per page): no keyboard shows on that tap, and every drop raises it,
  the first one after launch included (Noah, simulator, 2026-11-06). Tried and dropped:
  a hook on `WKContentView`'s `_elementDidFocus:userIsInteracting:…`, and WebKit's input
  delegate (`_WKInputDelegate`, `decidePolicyForFocusedElement`). WebKit never calls
  either for this focus before it's primed, so neither could help. Drops injected by the
  simulator tool's `touch_path` count as no tap, so a scripted test needs a real tap first.
- A highlighted zone's label (`::after`) is laid over its top edge: a label that took
  room moved the zones below it out from under the finger, which flickered the target.
