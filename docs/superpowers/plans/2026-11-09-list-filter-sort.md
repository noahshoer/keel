# Filter and Sort on List Pages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every list page can carry a remembered filter (existing query language) and a nested sort (headings, then row order), with two header icons and chips that show what is set (issue #7).

**Architecture:** A page's `Arrangement` (filter query, heading fields, order levels) is user data: a new `ViewArrange` action stores it per `ViewPlace` in a `view_arrangement` table. One pure function, `keel_core::arrange::arrange`, filters, groups and orders a page's tasks; the web UI calls it through wasm, so typing a filter needs no IPC. `MainPane` uses it only when the page has an arrangement; a page without one runs today's code unchanged.

**Tech Stack:** Rust (`keel-core`, `keel-storage` rusqlite, `keel-service`, Tauri v2 commands, `keel-core-wasm`), React + TypeScript (Vitest, Biome), ts-rs bindings.

**Spec:** `docs/superpowers/specs/2026-11-09-list-filter-sort-design.md` (read it first; mockups linked at its top).

## Global Constraints

- Branch `issue-7-filter-sort`; commit per task; never merge or push without Noah's say-so. Commits use the repo's configured noreply email.
- A page with no arrangement must render the same DOM and keep the same behavior as before this work.
- Data rules live in `keel-core` with Rust tests. TypeScript never decides what matches, groups or sorts, and never compares dates.
- `SortField` is append-only; new serialized fields are `#[serde(default)]`; existing `Action` shapes don't change.
- No second parser: filtering is `filter::parse` + `filter::matches`; the field is `FilterField` + `useQueryCheck`.
- Tauri commands: `#[tauri::command(rename_all = "snake_case")]`, one `…Args` struct, entry in `command_args!`, entry in `web/src/ipc/commands.ts`, mock handler, bindings regenerated with `scripts/check-bindings.sh`.
- Liquid Glass stays off; no `plugin:glass|*` calls.
- New text colors pass `web/src/styles/contrast.test.ts` (4.5:1) in both themes. Styles go in `web/src/styles/app.css`.
- Missing values (no label, no date, no deadline) sort last whatever the direction. "No section" comes first inside a project.
- CI gates: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd web && pnpm check`, `scripts/check-bindings.sh`.

## Review Focus

1. **A task under two label headings.** Grouping by Label shows it twice; both copies need distinct React keys and both leave when it is completed. Test in Task 5.
2. **A remembered filter that no longer parses** (it names a deleted project). The page shows every task and the chip shows the problem. Tests in Tasks 3 and 6.
3. **A stored row this version can't read** (a `SortField` added by a newer version, or a place whose project is gone). `list_arrangements` skips the unreadable row instead of failing the whole load. Test in Task 1.
4. **A filter that matches nothing.** The page says "No tasks match this filter." and keeps its "+ Add task" row, not the page's normal empty state. Test in Task 5.
5. **A saved-filter page whose comma groups share a task, with headings set.** The task appears once per heading, not once per comma group. Test in Task 2.

## File Structure

| File | Responsibility |
|---|---|
| `crates/core/src/arrange.rs` (new) | `SortField`, `OrderLevel`, `Arrangement`, `ViewPlace`, `PlacedArrangement`; `arrange`, `summary`, `sort_choices` |
| `crates/core/src/commands.rs` | `Action::ViewArrange` |
| `crates/core/src/repository.rs`, `crates/storage/src/{migrations,sqlite_repo}.rs` | `view_arrangement` table and its three methods |
| `crates/service/src/lib.rs`, `crates/src-tauri/src/lib.rs`, `crates/src-tauri/tests/commands_list.rs` | `list_arrangements` |
| `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts` | `arrange`, `arrangementSummary`, `sortChoices` |
| `web/src/arrange/place.ts` (new) | a `View` → its `ViewPlace` and current `Arrangement` |
| `web/src/arrange/sortEdit.ts` (new) | pure edits of the two sort lists |
| `web/src/arrange/ArrangedList.tsx` (new) | draws arranged groups (headings, sub-headings, cards) |
| `web/src/arrange/ArrangeBar.tsx` (new) | the chip row |
| `web/src/arrange/ArrangePanel.tsx` (new) | sheet on the phone, popover at 900px and up |
| `web/src/arrange/SortSheet.tsx`, `FilterSheet.tsx` (new) | the two panels' contents |
| `web/src/screens/MainPane.tsx` | header icons, preview state, the one branch |

---

### Task 1: Arrangement types, the `ViewArrange` action and storage

**Files:**
- Create: `crates/core/src/arrange.rs`, `crates/storage/tests/arrangements.rs`
- Modify: `crates/core/src/lib.rs` (`pub mod arrange;`), `crates/core/src/commands.rs`, `crates/core/src/repository.rs`, `crates/storage/src/migrations.rs`, `crates/storage/src/sqlite_repo.rs`, `crates/core/tests/ts_bindings.rs`

**Interfaces — Produces:**

```rust
// crates/core/src/arrange.rs — all derive Clone, PartialEq, Debug, Serialize, Deserialize (+ ts-rs export like views::CardMode)
#[serde(rename_all = "snake_case")]
pub enum SortField { Project, Section, Label, Priority, DueDate, Deadline }   // append-only
pub struct OrderLevel { pub field: SortField, #[serde(default)] pub reversed: bool }
#[derive(Default)]
pub struct Arrangement {
    #[serde(default)] pub filter: String,
    #[serde(default)] pub groups: Vec<SortField>,
    #[serde(default)] pub order: Vec<OrderLevel>,
}
impl Arrangement {
    pub fn is_default(&self) -> bool;        // blank filter, both lists empty
    pub fn normalized(self) -> Arrangement;  // filter trimmed; a repeated field in `groups` or in `order` keeps its first entry
}
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ViewPlace { Today, Upcoming, Project { id: Id }, Label { id: Id }, Filter { id: Id } }
impl ViewPlace { pub fn key(&self) -> String; } // "today", "upcoming", "project:<id>", "label:<id>", "filter:<id>"
pub struct PlacedArrangement { pub place: ViewPlace, pub arrangement: Arrangement }

// commands.rs — new variant, type_name "view_arrange"
Action::ViewArrange { place: ViewPlace, arrangement: Arrangement }

// repository.rs
fn upsert_arrangement(&self, placed: &PlacedArrangement) -> Result<(), RepoError>;
fn delete_arrangement(&self, place_key: &str) -> Result<(), RepoError>;
/// Rows that don't deserialize are skipped.
fn list_arrangements(&self) -> Result<Vec<PlacedArrangement>, RepoError>;
```

Table (in `migrations::run`'s batch): `view_arrangement (place_key TEXT PRIMARY KEY, place_json TEXT NOT NULL, arrangement_json TEXT NOT NULL)`.

- [ ] **Step 1: Write failing tests** in `crates/storage/tests/arrangements.rs` (open an in-memory `SqliteRepo` the way `crates/storage/tests/filters.rs` does; dispatch with `commands::apply_at`):
  - `view_arrange_stores_and_lists`: apply `ViewArrange { place: Today, arrangement: { filter: " p1 ", groups: [Project, Section, Label], order: [DueDate, Priority] } }`; `list_arrangements()` has one entry with `filter == "p1"` and the same lists.
  - `view_arrange_replaces`: a second `ViewArrange` for `Today` leaves one row with the new value.
  - `default_arrangement_removes_the_row`: applying `Arrangement::default()` for `Today` leaves `list_arrangements()` empty.
  - `replayed_command_is_ignored`: the same `Command` (same uuid) applied twice, with a different arrangement applied in between, leaves the in-between one.
  - `places_are_separate`: `Project { id: "a" }` and `Label { id: "a" }` are two rows.
  - `repeated_fields_keep_the_first`: `groups: [Label, Project, Label]` is stored as `[Label, Project]`.
  - `unreadable_row_is_skipped` (Review Focus 3): insert a row whose `arrangement_json` is `{"groups":["size"]}` with raw SQL next to a good row; `list_arrangements()` returns only the good one.
  - In `arrange.rs` unit tests: `ViewPlace::key` gives the five strings above; `Arrangement` deserializes from `{}`.
- [ ] **Step 2: Run** `cargo test -p keel-storage --test arrangements` → fails to compile (types missing).
- [ ] **Step 3: Implement** the types, the action arm (normalize; default → `delete_arrangement`, else `upsert_arrangement`), the trait methods, the table, the `SqliteRepo` methods (JSON via `serde_json`, same `backend` error mapping as `upsert_filter`), and add the five new types to `ts_bindings.rs`.
- [ ] **Step 4: Run** `cargo test -p keel-core -p keel-storage` → all pass.
- [ ] **Step 5: Commit** `feat(core): page arrangements and the view_arrange action (#7)`.

---

### Task 2: `arrange` — headings and order

**Files:**
- Modify: `crates/core/src/arrange.rs`, `crates/core/src/views.rs` (make `short_date` and `weekday_long` `pub(crate)`), `crates/core/tests/ts_bindings.rs`

**Interfaces — Consumes:** Task 1's types; `views::{due_date_of, due_time}`; `filter` (Task 3 fills the filter in).

**Produces:**

```rust
pub struct ArrangeNames<'a> { pub projects: &'a [Project], pub sections: &'a [Section], pub labels: &'a [Label] }

#[derive(Clone, PartialEq, Debug, Serialize)]  // + ts-rs export
pub struct SubHeading { pub text: String, /// A label heading's color name.
    pub label_color: Option<String> }
pub struct ArrangedGroup {
    /// Unique on the page and stable across reloads (built from the heading values' ids).
    pub key: String,
    /// Every heading level but the innermost, joined with " › "; with one level, that level. `None` with no headings.
    pub title: Option<String>,
    /// This group opens a new title line.
    pub starts_title: bool,
    /// Distinct tasks under this title line (valid where `starts_title`).
    pub title_count: usize,
    /// The innermost level when there are two or more.
    pub sub: Option<SubHeading>,
    pub tasks: Vec<Task>,
}
pub struct Arranged {
    pub groups: Vec<ArrangedGroup>,
    pub filter_problem: Option<String>,  // Task 3
    pub shown: usize,                    // distinct tasks in `groups`
    pub total: usize,                    // distinct tasks given
}
pub fn arrange(tasks: Vec<Task>, arrangement: &Arrangement, names: &ArrangeNames, today: NaiveDate) -> Arranged;
```

Behavior the tests pin (spec sections "The sort sheet" and "How nested headings are drawn"):
- Tasks are de-duplicated by id first, keeping the first.
- No headings: exactly one group (`title: None`, `sub: None`, `key: "all"`), even when empty.
- Heading order per field: Project by position in `names.projects`; Section "No section" first, then `section_order`; Label by position in `names.labels`, "No label" last; Priority P1 → P4 as "Priority 1" … "Priority 4"; Due date "Overdue" first, then one per day titled `"{short_date} · {weekday_long}"`, "No date" last; Deadline the same with "No deadline".
- A Section value reads "Project › Section" when Project is not an earlier heading and the tasks span more than one project; otherwise the section's name.
- A task with several labels is in every one of its label groups; a label name with no `Label` row gets its own group after the known ones, by name.
- Order levels apply inside each group in sequence; ties keep the input order (stable sort). Due date compares date, then time of day with untimed after timed. As an order, Label is the task's first label by position.

- [ ] **Step 1: Write failing tests** (unit tests in `arrange.rs`; a small `task(id)` builder plus setters, two projects "Home" and "Work", sections "Kitchen" (Home), labels "errand", "phone"):
  - `no_headings_is_one_group_in_input_order`
  - `noahs_example`: groups `[Project, Section, Label]`, order `[DueDate, Priority]`; assert the sequence of `(title, sub.text, task ids)` is `("Home › No section", "errand", …)`, `("Home › Kitchen", "errand", [due-yesterday-p3, due-today-p1, due-today-p3])`, `("Home › Kitchen", "phone", …)`, `("Work › No section", "No label", …)`; `starts_title` is true only on the first group of each title; `title_count` counts distinct tasks.
  - `one_heading_has_no_sub`: groups `[Priority]` → titles "Priority 1", "Priority 4", `sub == None`.
  - `two_labels_two_groups`: a task labelled errand and phone is in both groups; `shown == 1`.
  - `missing_values_go_last_both_directions`: order `[DueDate]` and `[DueDate reversed]` both end with the undated task; same for `Deadline`.
  - `due_date_headings`: "Overdue", `"9 Nov · Monday"` (today = 2026-11-09), "No date", in that order.
  - `section_names_its_project_across_projects` and `section_alone_in_one_project`.
  - `ties_keep_input_order`.
  - `shared_task_appears_once` (Review Focus 5): the input lists the same task id twice; with groups `[Priority]` it is in one group once.
  - `group_keys_are_unique_and_stable`: two sections named "Kitchen" in different projects get different keys; calling twice gives the same keys.
- [ ] **Step 2: Run** `cargo test -p keel-core arrange` → the new tests fail.
- [ ] **Step 3: Ask Noah for the comparator (Learn by Doing).** Add this stub with a `TODO(human)` and ask Noah to write its body (5–8 lines) before continuing:

```rust
/// Orders two optional sort values: a missing value is always after a present
/// one; `reversed` flips only the order between two present values.
fn cmp_missing_last<T: Ord>(a: Option<T>, b: Option<T>, reversed: bool) -> std::cmp::Ordering
```

- [ ] **Step 4: Implement `arrange`.** Approach: expand each task into one entry per label only when `Label` is a heading; compute each entry's heading sort keys and display values; stable-sort by heading keys then order keys (all through `cmp_missing_last`); split into runs of equal heading keys. Filter is not applied yet (`filter_problem: None`).
- [ ] **Step 5: Run** `cargo test -p keel-core` and `cargo clippy -p keel-core --all-targets -- -D warnings` → pass.
- [ ] **Step 6: Commit** `feat(core): group and order a page's tasks (#7)`.

---

### Task 3: `arrange` — the filter, and the words the UI shows

**Files:** Modify `crates/core/src/arrange.rs`, `crates/core/tests/ts_bindings.rs`

**Interfaces — Consumes:** `filter::{parse, matches, names_of, FilterContext}`.

**Produces:**

```rust
/// "Project, Section, Label · Due date, Priority"; one half alone has no dot; "" with no sort.
/// A reversed order level reads with its flipped label in brackets: "Due date (latest first)".
pub fn summary(arrangement: &Arrangement) -> String;
pub struct SortChoice { pub field: SortField, pub name: String, pub order_label: String, pub reversed_label: String }
/// The six fields in the spec's table order, with its labels.
pub fn sort_choices() -> Vec<SortChoice>;
```

Exact labels: Project "Sidebar order" / "Reversed"; Section "Section order" / "Reversed"; Label "Label order" / "Reversed"; Priority "Highest first" / "Lowest first"; Due date and Deadline "Soonest first" / "Latest first". Names: "Project", "Section", "Label", "Priority", "Due date", "Deadline".

- [ ] **Step 1: Write failing tests:**
  - `filter_narrows`: filter `"@errand & p1"` keeps only matching tasks; `shown`/`total` are e.g. 2 and 14.
  - `comma_groups_match_any`: `"p1, @phone"` keeps tasks matching either, each once.
  - `blank_filter_keeps_everything`.
  - `broken_filter_shows_everything` (Review Focus 2): filter `"#Gone & p1"` where no project "Gone" exists, or `"p1 &"`: `filter_problem.is_some()`, `shown == total`.
  - `relative_dates_follow_today`: filter `"today"` with two different `today` values keeps different tasks.
  - `filter_then_headings`: filter plus groups `[Label]` gives only label groups that have matching tasks.
  - `summary_texts`: the five shapes above. `sort_choices_cover_every_field`.
- [ ] **Step 2: Run** → fail.
- [ ] **Step 3: Implement.** Parse on every call; on `Err(e)` set `filter_problem` to its message and skip filtering. Build `FilterContext` from `names`. If `filter::parse` accepts an unknown `#name` without error, take the problem from `filter::read(...).problem` so a deleted project counts as broken.
- [ ] **Step 4: Run** `cargo test -p keel-core` → pass.
- [ ] **Step 5: Commit** `feat(core): filter inside arrange, sort summary and labels (#7)`.

---

### Task 4: Across the boundary — service, command, wasm, web plumbing

**Files:**
- Modify: `crates/service/src/lib.rs`, `crates/src-tauri/src/lib.rs`, `crates/src-tauri/tests/commands_list.rs`, `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts`, `web/src/core/core.test.ts`, `web/src/ipc/commands.ts`, `web/src/ipc/mock.ts`, `web/src/ipc/actions.ts`, `web/src/app/load.ts`, `web/src/app/load.test.ts`, `web/src/bindings/*` (generated)
- Create: `web/src/arrange/place.ts`, `web/src/arrange/place.test.ts`

**Interfaces — Produces:**

```rust
impl AppService { pub fn list_arrangements(&self) -> Result<Vec<PlacedArrangement>, RepoError>; }
// Tauri: list_arrangements => NoArgs => Vec<PlacedArrangement>
```

```ts
// web/src/core/index.ts
export function arrange(tasks: Task[], arrangement: Arrangement, projects: Project[], sections: Section[], labels: Label[], today: string): Arranged;
export function arrangementSummary(a: Arrangement): string;
export function sortChoices(): SortChoice[];
// web/src/ipc/actions.ts
export const arrangeView = (place: ViewPlace, arrangement: Arrangement) => send({ type: "view_arrange", place, arrangement });
// web/src/app/load.ts — Loaded gains:
arrangements: PlacedArrangement[];
// web/src/arrange/place.ts
export const NO_ARRANGEMENT: Arrangement;  // { filter: "", groups: [], order: [] }
/** null for Search, and for a label page whose name has no label row. */
export function placeOf(view: View, loaded: Pick<Loaded, "inbox" | "labels">): ViewPlace | null;
export function arrangementOf(place: ViewPlace | null, all: PlacedArrangement[]): Arrangement;  // NO_ARRANGEMENT when none
export function isArranged(a: Arrangement): boolean;
```

- [ ] **Step 1: Write failing tests:**
  - `crates/service` test `list_arrangements_returns_what_was_dispatched`.
  - `place.test.ts`: `placeOf` maps inbox → `{kind:"project", id: inbox.id}`, today, upcoming, project, filter; a label page → the label's id (case-insensitive name match); search → `null`; unknown label name → `null`. `arrangementOf` finds by place and falls back to `NO_ARRANGEMENT`.
  - `core.test.ts`: `arrange` over three fixture tasks with groups `["priority"]` returns titles in priority order; `arrangementSummary` and `sortChoices().length === 6`.
  - `load.test.ts`: after dispatching `arrangeView({kind:"today"}, …)` against the mock, `loadView({kind:"today"}, …).arrangements` contains it.
- [ ] **Step 2: Run** `cargo test -p keel-service` and `cd web && pnpm vitest run src/arrange src/core src/app` → fail.
- [ ] **Step 3: Implement.** Service method; sync Tauri command (no plugin call, so not `async`) registered in the handler list and `command_args!`; wasm exports following `cards` / `date_groups` for the serde bridge; wrappers in `web/src/core/index.ts`; `COMMAND_TYPES.list_arrangements`; mock state `arrangements` with a `list_arrangements` handler and a `view_arrange` case (replace by place key, drop when default); `loadView` adds `invoke("list_arrangements", {})` to its `Promise.all`.
- [ ] **Step 4: Regenerate bindings:** `scripts/check-bindings.sh` (it fails once because files changed; commit them and re-run → "bindings up to date").
- [ ] **Step 5: Run** `cargo test` and `cd web && pnpm check` → pass.
- [ ] **Step 6: Commit** `feat: arrangements across IPC and wasm (#7)`.

---

### Task 5: Draw an arranged page

**Files:**
- Create: `web/src/arrange/ArrangedList.tsx`, `web/src/arrange/ArrangedList.test.tsx`
- Modify: `web/src/screens/MainPane.tsx`, `web/src/screens/MainPane.test.tsx`, `web/src/screens/SectionBlock.tsx`, `web/src/styles/app.css`, `web/src/ipc/fixtures.ts` (if fixtures need a two-label task)

**Interfaces — Consumes:** `arrange`, `placeOf`, `arrangementOf`, `isArranged`, `Loaded.arrangements`.

**Produces:** `MainPane` computes `const arrangement = preview ?? arrangementOf(place, loaded.arrangements)` (`preview` state arrives in Task 8; use `null` for now) and:

- **Not arranged:** the current body, byte for byte.
- **Arranged, no headings:** the current body, but every task list goes through `shape(tasks) = arrange(tasks, arrangement, …).groups[0].tasks` after the page's own grouping (sections, `dateGroups`, comma groups). A date group or comma group left empty is dropped; sections stay.
- **Arranged, with headings:** `<ArrangedList groups={arrange(loaded.tasks, …).groups} … />` replaces the body, followed by one `addRow`. No `DropZone`, no `SectionBlock`, no `AddSection` row; the ••• "Add section" entry stays.
- **Filter set and nothing shown:** `<p className="view-sub arrange-empty">No tasks match this filter.</p>` then `addRow`.

`ArrangedList` props: `{ groups: ArrangedGroup[]; all: Task[]; mode: CardMode; today: string; … the props CardList takes }`. Per group: when `starts_title`, a `.section-title` with the title and `<span class="muted">{title_count}</span>`; when `sub`, a `.arrange-sub` line (tag icon and `--label-<color>` color when `label_color`); then `<CardList cards={cards(group.tasks, all, mode, today)} … />` wrapped in an element keyed by `group.key`.

- [ ] **Step 1: Write failing tests:**
  - `MainPane.test.tsx` `untouched page is unchanged`: render Today and a project with `arrangements: []`; every existing `MainPane` test still passes unedited, and the project page has no `.arrange-sub`, no `.arrange-empty`, and still has its `.add-section-row` and one `.add-task-row` per section. Also spy on `arrange` from `web/src/core`: it is never called for a page with no arrangement.
  - `order only keeps sections`: project page with order `[priority]`; section titles still present, first row in a section is its p1 task.
  - `headings replace sections`: project page with groups `[label]`; `.section-title` texts are the label names, no `.add-section-row`, exactly one `.add-task-row`.
  - `today grouped like Noah's example`: titles "Home › Kitchen" …, `.arrange-sub` texts "errand", "phone".
  - `two-label task shows twice and leaves twice` (Review Focus 1): no React duplicate-key warning (`console.error` spy), two rows with the task's title; after completing it and reloading, zero.
  - `nothing matches` (Review Focus 4): filter `"p1 & @nope"` → the text "No tasks match this filter." and one `.add-task-row`.
  - `upcoming order only keeps days`: order `[priority]` on Upcoming keeps one heading per day, in date order.
- [ ] **Step 2: Run** `cd web && pnpm vitest run src/screens src/arrange` → fail.
- [ ] **Step 3: Implement** as above. In `SectionBlock`, accept an optional `shape?: (tasks: Task[]) => Task[]` applied to its tasks. CSS: `.arrange-sub` (12.5px, weight 600, `padding: 4px 10px 5px`, inline-flex with a 13px icon), `.arrange-empty`.
- [ ] **Step 4: Run** `cd web && pnpm check` → pass (includes the contrast test).
- [ ] **Step 5: Commit** `feat(web): draw filtered and sorted pages (#7)`.

---

### Task 6: Header icons and the chip row

**Files:**
- Create: `web/src/arrange/ArrangeBar.tsx`, `web/src/arrange/ArrangeBar.test.tsx`
- Modify: `web/src/shell/icons.tsx` (`SortIcon`: Phosphor `ArrowsDownUp` via `Ph`, size 18, deep import like `Funnel`), `web/src/shell/icons.test.tsx`, `web/src/screens/MainPane.tsx`, `web/src/styles/app.css`, `web/src/styles/contrast.test.ts`

**Interfaces — Produces:**
- `MainPane` state `const [panel, setPanel] = useState<"filter" | "sort" | null>(null)`, reset when `place` changes.
- Two buttons in `.view-head`, before the completed toggle, only when `placeOf(...) !== null`: `className="icon-btn arrange-btn"`, `aria-label="Filter"` / `"Sort"`, `aria-pressed` and class `on` while that part is set, `onClick` toggles `panel`.
- `<ArrangeBar arrangement problem onOpen={(which) => setPanel(which)} onClearFilter onResetSort />` as the first child of `.list-body`, rendering nothing when not arranged. Filter chip: `button.scope-chip.arrange-chip` with funnel, the query, and a separate ✕ button (`aria-label="Clear filter"`); with `problem` it gets class `problem` and `title={problem}`. Sort chip: sort icon, `arrangementSummary(arrangement)`, ✕ (`aria-label="Reset sort"`).
- `onClearFilter` / `onResetSort` call `arrangeView(place, {...arrangement, filter: ""})` / `{...arrangement, groups: [], order: []}` then `onChanged`.

- [ ] **Step 1: Write failing tests:** icons absent on Search and present on Today; no chips and no `on` class when not arranged; with filter `"p1"` the filter chip shows "p1" and the Filter button has `aria-pressed="true"`; "Clear filter" dispatches an arrangement with `filter: ""` and the same sort; "Reset sort" keeps the filter; a broken filter (Review Focus 2) gives the chip class `problem` and the list still shows every task; the sort chip text equals `arrangementSummary`. Contrast: `.arrange-btn.on` icon color on `--active-tint`, and the problem chip's text on its fill, both themes.
- [ ] **Step 2: Run** → fail.
- [ ] **Step 3: Implement.** CSS: `.arrange-btn.on { background: var(--active-tint); color: var(--accent-text); }`; `.arrange-bar { display: flex; flex-wrap: wrap; gap: 6px; margin: 0 0 12px; }`; `.arrange-chip` wraps long text (`min-width: 0; overflow-wrap: anywhere; height: auto; min-height: 28px`); `.arrange-chip.problem` uses the filter-problem colors already in `app.css`. On the phone header (`max-width` block near `.view-toggle`), give `.arrange-btn` the 44px box the toggle has.
- [ ] **Step 4: Run** `cd web && pnpm check` → pass.
- [ ] **Step 5: Commit** `feat(web): filter and sort icons and chips (#7)`.

---

### Task 7: The sort panel

**Files:**
- Create: `web/src/arrange/sortEdit.ts`, `web/src/arrange/sortEdit.test.ts`, `web/src/arrange/ArrangePanel.tsx`, `web/src/arrange/SortSheet.tsx`, `web/src/arrange/SortSheet.test.tsx`
- Modify: `web/src/screens/MainPane.tsx`, `web/src/styles/app.css`

**Interfaces — Produces:**

```ts
// sortEdit.ts — pure; each returns a new Arrangement
export function addGroup(a: Arrangement, field: SortField): Arrangement;
export function removeGroup(a: Arrangement, index: number): Arrangement;
export function moveGroup(a: Arrangement, from: number, to: number): Arrangement;
export function addOrder(a: Arrangement, field: SortField): Arrangement;       // reversed: false
export function removeOrder(a: Arrangement, index: number): Arrangement;
export function moveOrder(a: Arrangement, from: number, to: number): Arrangement;
export function flipOrder(a: Arrangement, index: number): Arrangement;
export function unusedGroups(a: Arrangement, choices: SortChoice[]): SortChoice[];
export function unusedOrders(a: Arrangement, choices: SortChoice[]): SortChoice[];
export function resetSort(a: Arrangement): Arrangement;                        // keeps the filter
```

```tsx
// ArrangePanel: a bottom sheet under 900px (useWideLayout), a popover placed by useAnchoredPopup at 900px and up.
<ArrangePanel title={string} action={{ label: string; run: () => void }} anchor={RefObject<HTMLElement>} onClose={() => void}>{children}</ArrangePanel>
// SortSheet: every edit calls onChange with the new arrangement.
<SortSheet arrangement={Arrangement} onChange={(a: Arrangement) => void} />
```

`MainPane` renders `<ArrangePanel title="Sort" action={{label: "Reset to default", run: …}}>` with `<SortSheet>` while `panel === "sort"`; `onChange` dispatches `arrangeView(place, a)` then `onChanged` (every change saves at once).

Copy: section labels "Group under headings" and "Then order tasks by"; buttons "Add group" and "Add order"; each row's remove button `aria-label="Remove {name}"`; the direction is a button showing `order_label` or `reversed_label`.

- [ ] **Step 1: Write failing tests.** `sortEdit.test.ts`: each function, including `moveGroup` to the same index (no change), `unusedGroups` excluding used fields, `resetSort` keeping the filter. `SortSheet.test.tsx`: "Add group" lists only unused fields and picking "Label" calls `onChange` with it appended; the direction button flips; remove works; a row's "Move up" / "Move down" buttons (`aria-label="Move {name} up"`) reorder, and are disabled at the ends. `ArrangePanel`: Escape and a click on the scrim call `onClose`; it has `role="dialog"` and `aria-label={title}`.
- [ ] **Step 2: Run** → fail.
- [ ] **Step 3: Implement.** Reordering ships as the move buttons (keyboard reachable); add a drag on the row's grip with `web/src/gestures/drag.ts` only if it fits without changing that module's task-drag behavior, otherwise leave it and note it for Noah. The sheet reuses `.modal-scrim` and the task view sheet's surface styles where they apply; rows are 44px tall on touch.
- [ ] **Step 4: Run** `cd web && pnpm check` → pass. Then `pnpm dev`, open http://localhost:5173 at 375px and at 1200px, set Noah's example on Today, and confirm headings and both panel layouts in light and dark.
- [ ] **Step 5: Commit** `feat(web): the sort panel (#7)`.

---

### Task 8: The filter panel

**Files:**
- Create: `web/src/arrange/FilterSheet.tsx`, `web/src/arrange/FilterSheet.test.tsx`
- Modify: `web/src/screens/MainPane.tsx`, `web/src/styles/app.css`

**Interfaces — Consumes:** `FilterField`, `useQueryCheck` (`web/src/shell/`), `addFilter` (`web/src/ipc/actions.ts`), `ArrangePanel`, `arrange`.

**Produces:**

```tsx
<FilterSheet
  pageName={string}
  initial={string}                         // the saved filter
  names={FilterNames}
  filters={Filter[]}
  counts={{ shown: number; total: number }} // from arrange() over the page with the draft
  onDraft={(query: string | null) => void}  // valid draft → preview; null → no preview
  onSave={(query: string) => void}          // "" clears
  onClose={() => void}
/>
```

`MainPane` holds `const [preview, setPreview] = useState<Arrangement | null>(null)`; `onDraft(q)` sets `{...saved, filter: q}` (or `null`); the page renders from `preview ?? saved` (Task 5). Closing the panel or pressing Enter in the field calls `onSave` when the draft is valid and differs from `initial`, then clears `preview`. Panel title `Filter {pageName}`, action "Clear" (saves `""` and closes).

Copy: count line `"{shown} of {total} tasks match"`; list label "Your filters"; button "Save as a filter", which swaps to a name field with placeholder "Filter name" and Enter saving via `addFilter(name, query)`.

- [ ] **Step 1: Write failing tests:** typing `p1` calls `onDraft("p1")`; typing `p1 &` shows the reading's problem under the field and calls `onDraft(null)`; Enter with a valid changed draft calls `onSave` once then `onClose`; closing with an invalid draft does not call `onSave`; tapping a saved filter puts its query in the field (and calls `onDraft`); "Save as a filter" is hidden when the draft is blank, invalid, or equal to a saved filter's query, and otherwise creates a filter with the typed name; "Clear" calls `onSave("")`. In `MainPane.test.tsx`: with the panel open and `p1` typed, the list behind shows only p1 tasks before anything is dispatched.
- [ ] **Step 2: Run** → fail.
- [ ] **Step 3: Implement.** Focus the field inside the opening tap (`web/src/shell/focus.ts` `openAndFocus`) so iOS raises the keyboard. On touch keep the sheet above the keyboard the way the task view sheet does (`web/src/gestures/sheet.ts` / `window.keelKeyboard`) and hold the page (`web/src/app/pageHold.ts`) while it is open.
- [ ] **Step 4: Run** `cd web && pnpm check` → pass; check in `pnpm dev` at 375px and 1200px.
- [ ] **Step 5: Commit** `feat(web): the page filter panel (#7)`.

---

### Task 9: Whole-feature checks and notes

**Files:**
- Create: `scripts/sim-checks/arrange-today.js`
- Modify: `CLAUDE.md` (a short paragraph under "React frontend" and `arrange` in the Layout table's `keel-core` row), `.claude/issue-roadmap.md` (local, git-ignored)

- [ ] **Step 1: Run everything:** `cd web && pnpm build`, `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --target aarch64-apple-ios -- -D warnings`, `cargo test`, `cd web && pnpm check`, `scripts/check-bindings.sh`. All pass.
- [ ] **Step 2: Simulator scenario.** `arrange-today.js` dispatches `view_arrange` for Today with Noah's example through `window.__TAURI__.core.invoke("dispatch", …)`, reloads, and `check`s that a `.section-title` contains " › " and an `.arrange-sub` exists, then resets it and `check`s both are gone. Run `scripts/sim-check.sh 53917026-E8EC-4D54-8C70-A657555D8C47 scripts/sim-checks/arrange-today.js set@8 end@12` after a simulator build and install; every `[check]` line passes and the `set` screenshot shows the headings.
- [ ] **Step 3: Update `CLAUDE.md`** with where the feature lives (core `arrange`, `web/src/arrange/`, the `view_arrange` action, the scenario).
- [ ] **Step 4: Commit** `docs: filter and sort notes, simulator scenario (#7)`.
- [ ] **Step 5: Stop and report to Noah.** Say what was verified and how, and list what only the phone can show: the keyboard under the filter sheet, five header icons' tap targets on the phone's width, sheet drag feel. Do not merge or push.
