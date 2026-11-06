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
