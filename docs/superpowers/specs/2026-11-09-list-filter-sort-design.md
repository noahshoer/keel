# Filter and sort on list pages (issue #7)

Mockups Noah chose from (2026-11-09): https://claude.ai/artifact/TkaKgEUvE3kdhYUoVreRw2 (options 1A, 2A, sort icon S1).

## Why

Every list page has one fixed layout today: a project shows its sections, Today shows Overdue and Today, a label page is one flat list. Noah wants to look at the same tasks another way without building a saved filter for it. His example: on Today, group by project and section, show labels inside that, and inside each label order the tasks by due date and then priority. He also wants to narrow any page with the filter language that already exists, either typed on the spot or picked from his saved filters.

## Goals

- Any list page can carry a **filter** (a query in the existing language) and a **sort** (headings to group under, then an order for the rows).
- Each page remembers its own filter and sort. Coming back the next day shows the same thing (Noah).
- A page nobody has touched looks and behaves exactly as it does now (Noah).
- A page with a filter on it always shows that filter (Noah), so a missing task is never a mystery.
- The filter reuses the existing filter code and field. No second parser (Noah).

## Non-goals

- Typed sort syntax (`sort by …`). Sort is picked from lists.
- Sorting by name, created date or anything outside the issue's five fields plus section.
- A flip for the direction of headings; only the row order can be flipped.
- Named, reusable sort presets.
- A page filter that stays linked to a saved filter. Picking one copies its query.
- Filter and sort on the Search screen or on the "Show completed" list.
- Manual drag ordering while a page is grouped (see "What turns off").

## Anti-goals

- An untouched page changing in any way: its order, its headings, its drag and drop, its DOM.
- A remembered filter that hides tasks without saying so.
- TypeScript deciding what matches, what groups together or what comes first. That is `keel-core`.

## Which pages

Inbox, Today, Upcoming, every project, every label page and every saved-filter page. On a saved-filter page the page's filter is applied on top of the saved filter's own query.

## The controls (option 1A)

Two `icon-btn`s join the header's right-hand icons in `web/src/screens/MainPane.tsx`, before the completed toggle: **Filter** (the existing `FunnelIcon`) and **Sort** (Phosphor `ArrowsDownUp`, bold, added to `web/src/shell/icons.tsx`). Each is tinted (`--active-tint` fill, `--accent-text` icon) while its part is set. They are hidden on the Search screen.

Under the title, above the list, a chip row appears only when something is set:

- **Filter chip:** funnel, the query text, ✕. Tapping the chip opens the filter sheet; ✕ clears the filter.
- **Sort chip:** sort icon, a summary such as "Project, Section, Label · Due date, Priority" (headings, a dot, then the order; either half may be missing), ✕. Tapping opens the sort sheet; ✕ resets the sort.

The chips look like the Search scope chip (`.scope-chip`). The summary text comes from `keel-core`.

On the phone each control opens a bottom sheet; at 900px and up it opens a popover under its icon (*my call:* reuse `useAnchoredPopup` and the popup styles the ••• menu uses).

## The sort sheet (option 2A)

Title "Sort", with "Reset to default" on the right. Two lists:

1. **Group under headings.** An ordered list of fields, outermost first. "+ Add group" offers the fields not yet used in this list.
2. **Then order tasks by.** An ordered list of fields, each with a direction. "+ Add order" offers the fields not yet used in this list.

Rows can be dragged to reorder and removed with ✕. A field may be in both lists (grouping by priority and also ordering by it is pointless but harmless). Every change saves at once and the page behind updates.

The fields:

| Field | As a heading | As an order (default direction, flipped) |
|---|---|---|
| Project | project name, in sidebar order | sidebar order, reversed |
| Section | section name, in the project's order; "No section" first | section order, reversed |
| Label | label name with its color, in label order; "No label" last | first label's order, reversed |
| Priority | "Priority 1" … "Priority 4" | highest first, lowest first |
| Due date | the Upcoming page's headings: Overdue, then one per day; "No date" last | soonest first, latest first |
| Deadline | Overdue, then one per day; "No deadline" last | soonest first, latest first |

Rules (*my calls* unless marked):

- A task with several labels appears under each of its label headings (Noah, from the mockup). As an order, a task's label is its first one in label order.
- Missing values go last whatever the direction: no label, no date, no deadline.
- Due date compares the task's next date and then its time of day; a task with no time comes after timed ones on the same day.
- Section headings are keyed by section, not by name. When Project is not a heading above it and the page spans projects, the heading reads "Project › Section".
- Ties after the last order level keep the order the page had before sorting.

### How headings and order combine with the page's own layout

- **Order only, no headings:** the page keeps its built-in headings (a project's sections, Today's Overdue / Today, Upcoming's days, a saved filter's comma groups) and the rows inside each are reordered.
- **Any heading set:** the chosen headings replace the page's built-in ones. On Upcoming that means days disappear unless Due date is one of the headings.

### How nested headings are drawn

Every level except the innermost joins into one `section-title` line with " › " ("Home › Kitchen") and a task count. With two or more levels the innermost is a smaller sub-heading above each card group; a label sub-heading has the tag icon in the label's color. With one level it is just the `section-title` line. Rows don't change (a row keeps showing its project and labels; *my call*, simpler than the mockup's caption suggested and the row layout stays one thing).

## The filter sheet

Title "Filter <page name>", with "Clear" on the right.

- The field is `FilterField` driven by `useQueryCheck` (`web/src/shell/FilterField.tsx`, `filterQuery.ts`): same highlighting, red unknown names with the "Use #Name" fix, Backspace-to-quotes.
- The page behind narrows as you type, and a line under the field says "2 of 14 tasks match". A query that doesn't parse shows its problem there and leaves the page as it was; it is not saved.
- **Your filters:** the saved filters, each with its query in grey. Tapping one copies its query into the field.
- **Save as a filter:** shown when the field has a valid query that isn't already a saved filter's. It asks for a name in place and adds a saved filter (`FilterAdd`).
- The filter saves when the sheet closes or Enter is pressed (*my call:* not per keystroke, so the command log gets one entry per edit).

A page filter may contain comma groups; a task shows when it matches any of them, with no per-group headings (*my call*).

If a remembered filter stops parsing later (a project it names was deleted), the page shows all its tasks and the chip turns to the problem style with the reason (*my call:* showing everything is safer than showing nothing).

## What turns off while a page is arranged

- While any heading is set on a project or Inbox page: dragging a task between sections, the per-section "+ Add task" rows and the "Add section" row at the bottom. One "+ Add task" row stays at the end of the list, and "Add section" stays in the ••• menu.
- Nothing else changes: completing, the long-press menu, the task view, "Show sub-tasks in lists", "Show completed" (its list is not filtered or sorted), sidebar counts (they count the whole page).

## Data and where the logic lives

**One value per page**, part of the user's data so it follows them once sync exists (*my call;* "Show sub-tasks in lists" stays per device):

- `Arrangement { filter: String, groups: Vec<SortField>, order: Vec<OrderLevel> }`, `OrderLevel { field: SortField, reversed: bool }`, `SortField { Project, Section, Label, Priority, DueDate, Deadline }` in `keel-core`'s model. `SortField` is append-only, like `Recurrence`. An arrangement with an empty filter and both lists empty is the default.
- `ViewPlace { Today, Upcoming, Project { id }, Label { id }, Filter { id } }`. Inbox is its project. The UI's label page knows its label by name (`View` in `web/src/app/view.ts`) and looks the id up in the loaded labels, so a renamed label keeps its arrangement; a label page whose name has no label row shows no Filter or Sort icons.
- A new `Action::ViewArrange { place, arrangement }` (an empty arrangement resets). It goes through the command log like every other change.
- Storage: a new `view_arrangement` table (place key, arrangement as JSON), a migration, and `Repository` methods to upsert and list. Deleting a project, label or filter may leave its row behind; nothing reads it.
- A `list_arrangements` command; `loadView` (`web/src/app/load.ts`) reads it with the rest and `Loaded` carries the current page's arrangement.

**The rule itself is one core function** (new module, e.g. `crates/core/src/arrange.rs`), exported to wasm so typing in the filter field needs no IPC:

- In: the page's tasks in their current order, the arrangement, projects, sections, labels, today.
- It filters (`filter::parse` on every call so relative dates stay current, then `filter::matches`), groups and orders.
- Out: what the page draws: an ordered list of groups, each with its heading line, optional sub-heading (text, color), and tasks; plus the filter's problem if it didn't parse, and the "x of y" numbers.
- The page then builds cards per group with the existing `views::cards` and the current `CardMode`, so sub-tasks behave as they do now inside each group.
- The chip's sort summary and each field's display names and direction labels also come from core (as `keel_core::settings` does for the settings menus).

`MainPane` branches once: no arrangement means the code path it has today, untouched. An arrangement means the arranged path.

## Left to the builder

- The exact shape of the arranged output and the wasm binding names.
- How sheet rows are dragged to reorder (the existing `web/src/gestures/drag.ts` or something smaller), as long as there is a keyboard-reachable way to reorder too.
- React keys for a task that appears under two label headings, and making its completion animate sensibly in both places.
- Keeping the filter sheet's field above the phone keyboard. The task view's sheet already solves this (`web/src/gestures/sheet.ts`, `pageHold.ts`); reuse it where it fits. A headless simulator shows no keyboard, so this part needs the phone.
- CSS names for the chip row, sub-heading and sheets. New text colors must pass `web/src/styles/contrast.test.ts` in both themes.

## Testing

- `keel-core`: Rust tests for the arrange function: each field as heading and as order, both directions, missing values last, multi-label duplication, nested headings, order-only keeping input groups, filter + sort together, a filter that doesn't parse, the summary text. Action tests for `ViewArrange` (set, replace, reset, idempotent replay).
- `keel-storage`: the migration and round trip.
- Web (Vitest): the header icons and chips appear only when set; an untouched page renders the same markup as before; the sheets' add / remove / reorder / flip logic in plain `.ts`; saving on close.
- The mock backend (`web/src/ipc/mock.ts`) handles the new command and action so `pnpm dev` shows it.
- A simulator scenario (`scripts/sim-checks/`) that sets Noah's example on Today and checks the headings.
- Not verifiable without the phone: the keyboard under the filter sheet, tap-target sizes of five header icons.
