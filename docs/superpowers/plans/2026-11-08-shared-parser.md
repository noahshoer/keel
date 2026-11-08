# Shared Phrase Vocabulary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Quick add and filters read the same phrases from one module; filters accept text, quotes, every quick-add date and repeat phrase, and match occurrences; quick add marks new projects and labels and confirms them on save; filter fields highlight like the task card.

**Architecture:** The word matchers move out of `quickadd.rs` into a new `phrases.rs` that both parsers call. The filter parser keeps its operator grammar but reads each text piece word by word with those matchers, reports spans, and gains text, repeat, time and streak terms; date terms match any occurrence through a new `views::occurs_in`. New names and unknown names are flags on `TitleSegment`, computed in `keel-core`; the web draws them and adds a save prompt and highlighted filter fields.

**Tech Stack:** Rust (`keel-core`, `keel-service`, `keel-core-wasm`), React + TypeScript (`web/`), Vitest, Biome.

**Spec:** `docs/superpowers/specs/2026-11-08-shared-parser-design.md` (read it first; this plan argues from it).

## Global Constraints

- Branch `shared-parser`. TDD: failing test first. Commit per task. Never merge or push without Noah's approval.
- Data rules live in `keel-core` with Rust tests; TypeScript never re-implements them (CLAUDE.md "Where logic lives").
- `Recurrence` and `Action` shapes don't change. New enum variants are appended. New serialized fields are `#[serde(default)]`.
- `SpanKind` gains one appended variant, `Keyword`.
- Every query that parsed before still parses. Matching for date terms changes by design (occurrences).
- After any type change exported to TypeScript: regenerate bindings with `scripts/check-bindings.sh` (it must report no drift when committed).
- Copy, exactly: prompt title "Create these with the task?"; line "Untick any you don’t want. They stay as text in the title."; group headers "New project" / "New projects", "New label" / "New labels"; count "{ticked} of {total}"; buttons "Back to editing" and "Save"; filter error "No project named “{name}”." (and "No label named …", "No section named …"); fix "Use {#Name}".
- Colors: new (yellow): underline `#b07800` dotted 2px, pill fill `#f8e7bd`, pill text `#7a4a00`, pill border `1.5px dotted #b07800`; unknown (red): underline `var(--p1)` dotted 2px. Both themes must pass `web/src/styles/contrast.test.ts` (add dark-theme values as tokens `--new-*` / reuse `--p1`).
- Checks: `cargo test` and `cd web && pnpm check` green at the end of every task that touches their side.

## Review Focus

1. **Text before a span has multi-byte characters** ("café tomorrow", "🛒 #wrok"): offsets from wasm are UTF-16 for the browser, UTF-8 in Rust; backspace and the "Use #Work" fix must land on the right characters. Tests in Tasks 4 and 8.
2. **Quotes at the edges**: an unclosed `"tomorrow` reads to the end of the piece as text; `""` adds nothing; a query that is only `""` is "The query is empty". Test in Task 6.
3. **A repeating task's occurrences stop at `until`** and start at its next date (a moved occurrence's own date counts, nothing before it): "in 3 days" must not show a task whose repeat ended yesterday. Tests in Task 5.
4. **A saved filter naming a deleted project still runs** and shows its other groups; only the editors mark the name red. Test in Task 7 (service).
5. **A label both typed and picked** (`@errands` in the title and ticked in the label picker) is not offered in the save prompt, since picking it was explicit; unticking a typed-only label never drops a picked one. Test in Task 4.

---

### Task 1: Move the word matchers into `phrases.rs`

Pure move; no behavior change. The quick-add and editor tests are the proof.

**Files:**
- Create: `crates/core/src/phrases.rs`
- Modify: `crates/core/src/quickadd.rs` (remove what moved; `use crate::phrases::*` as needed), `crates/core/src/lib.rs` (`pub mod phrases;`)

**Interfaces:**
- Produces (all `pub(crate)` in `phrases`): `struct Token { raw, norm, start, end }`, `fn tokens(&str) -> Vec<Token>`, `fn weekday`, `fn month`, `fn day_number`, `fn match_date(&[Token], usize, NaiveDate) -> Option<(usize, NaiveDate)>`, `fn match_time(&[Token], usize) -> Option<(usize, NaiveTime)>`, `fn match_deadline(&str, &[Token], usize, NaiveDate) -> Option<(usize, NaiveDate)>`, `fn match_repeat(&[Token], usize) -> Option<(usize, RepeatSpec)>`, `fn repeat_word(&str) -> bool`, `fn number_word(&str) -> Option<u32>`, `fn match_name(&[Token], usize, &str, &[&str]) -> Option<(usize, usize)>`, `enum RepeatSpec` (unchanged variants), and a new `fn resolve_repeat(spec: RepeatSpec, start: NaiveDate) -> Recurrence` holding the `match spec { … }` block now at the end of `quickadd::parse_tokens`. Private helpers (`clock_ampm`, `hm_ampm`, `nth`, `ordinal_count`, `is_unit`, `weekday_list`, `of_month`, `idx`, `today_or_next`, `saturday_this_weekend`) move with them. `quickadd::parse_time` stays public in `quickadd` and calls `phrases::match_time`.

- [ ] **Step 1: Run the suite before the move:** `cargo test -p keel-core` → all pass; note the count.
- [ ] **Step 2: Move the code** listed above; `parse_tokens` calls `phrases::resolve_repeat(spec, start)`.
- [ ] **Step 3: Run** `cargo test -p keel-core` → same count, all pass; `cargo clippy -p keel-core` → no new warnings.
- [ ] **Step 4: Commit** `refactor(core): move quick add's word matchers into phrases`

---

### Task 2: Repeat shape matching

**Files:**
- Modify: `crates/core/src/phrases.rs`
- Test: same file, `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `pub(crate) fn repeat_matches(spec: &RepeatSpec, rule: &Recurrence) -> bool` — the task's `rule` has the kind and interval the phrase names, ignoring anchors, start and end. `RepeatSpec` derives `Clone, Debug, PartialEq`.

- [ ] **Step 1: Write the failing test** `repeat_phrase_matches_by_kind_and_interval`, reading each phrase with `match_repeat(&tokens(p), 0)`:

| phrase | matches | doesn't match |
|---|---|---|
| `every day` | `Daily` | `Weekly([0])` |
| `every week` | `Weekly([0])`, `Weekly([2,4])` | `EveryNWeeks{n:2,..}` |
| `every monday` | `Weekly([0])`, `Weekly([0,2])` | `Weekly([1])` |
| `every 2 weeks` | `EveryNWeeks{n:2, days:[3], anchor: 2026-10-01}`, same with anchor 2026-11-05 | `EveryNWeeks{n:3,..}`, `Weekly([3])` |
| `every 2 weeks on mon and wed` | `EveryNWeeks{n:2, days:[0,2,4],..}` | `EveryNWeeks{n:2, days:[0],..}` |
| `every 3 days` | `EveryNDays{n:3, anchor: any}` | `EveryNDays{n:4,..}` |
| `every month` | `MonthlyDay(5)`, `MonthlyDay(20)` | `EveryNMonths{n:2,..}` |
| `every 3 months` | `EveryNMonths{n:3, day:any, anchor:any}` | `EveryNMonths{n:2,..}` |
| `every year` | `Yearly{month:3, day:1}` | `MonthlyDay(1)` |
| `every last day` (whatever `match_repeat` reads as `MonthlyLastDay`) | `MonthlyLastDay` | `MonthlyDay(31)` |

- [ ] **Step 2: Run** `cargo test -p keel-core repeat_phrase_matches` → FAIL (not defined).
- [ ] **Step 3: Implement** `repeat_matches`. `Rule(r)` for anchor-free rules compares with `==`, except `Rule(Weekly(days))`, which is a subset test. `Weekly` / `Monthly` / `Yearly(None)` match any rule of that variant; `EveryNWeeks(1, days)` is treated as `Weekly(days)`.
- [ ] **Step 4: Run** the test → PASS.
- [ ] **Step 5: Commit** `feat(core): match a repeat phrase against a task's rule by kind and interval`

---

### Task 3: Quick add takes new project names; numbers stay text

**Files:**
- Modify: `crates/core/src/quickadd.rs`, `crates/core/src/phrases.rs`
- Test: `quickadd.rs` tests

**Interfaces:**
- Produces: `QuickAdd.new_project: Option<String>` (`#[serde(default)]` not needed; `QuickAdd` isn't stored) — the typed name, case kept, when `#name` names no project. Its span is `SpanKind::Project`. `pub(crate) fn phrases::is_name(word: &str) -> bool` — false when the word is empty or only digits and punctuation.

- [ ] **Step 1: Write the failing tests:**
  - `unknown_project_is_taken_as_new`: `p("Buy ink #wrok")` → `content == "Buy ink"`, `project_id == None`, `new_project == Some("wrok")`, one span `Project` over `#wrok`.
  - `known_project_still_wins`: with project "Work", `p("Buy ink #work")` → `project_id == Some(P-WORK)`, `new_project == None`.
  - `numbers_are_not_names`: `p("fix bug #123")` → content `"fix bug #123"`, no spans, `new_project == None`; `p("call @1 now")` → content `"call @1 now"`, `labels` empty.
  - Change the existing test that keeps `x #unknown` as text so it expects the new project instead (the spec names this exception).
- [ ] **Step 2: Run** `cargo test -p keel-core quickadd` → the new tests FAIL.
- [ ] **Step 3: Implement**: after `match_name` fails on a `#word`, if `phrases::is_name(rest)` and no project yet, set `new_project` (one word, trailing `,.;!?` trimmed) and take one token as `Project`. Apply `is_name` to `@` labels too. A new project scopes `/section` matching to nothing (sections need a known project).
- [ ] **Step 4: Run** `cargo test -p keel-core` → PASS.
- [ ] **Step 5: Commit** `feat(core): quick add takes an unknown #project as new; numbers stay text`

---

### Task 4: Editor: new-name flags, the save prompt's data, and the plan

**Files:**
- Modify: `crates/core/src/editor.rs`, `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts` (wrappers), `web/src/bindings/*` (regenerated)
- Test: `editor.rs` tests

**Interfaces:**
- Consumes: `QuickAdd.new_project` (Task 3).
- Produces:
  - `TitleSegment.is_new: bool` and `TitleSegment.unknown: bool` (Task 9 sets `unknown`; both false by default).
  - `editor_view(v, quick_add, keep_label, existing, projects, sections, labels: &[Label], today)` — new `labels` parameter (wasm `editorView` gains `labels` after `sections`). Marks `is_new` on the new project's segment and on each `@label` segment whose name no label has (any case).
  - `EditorView.new_project: Option<String>` and `EditorView.new_labels: Vec<String>` (typed names only, in title order, picked names excluded).
  - `pub struct NewNames { pub projects: Vec<String>, pub labels: Vec<String> }` and `pub fn new_names(v, projects, sections, labels, today) -> NewNames` — what the prompt lists; empty when nothing is new. wasm `newNames`.
  - `pub fn keep_as_text(v: &EditorValues, names: &[String], projects, sections, today) -> EditorValues` — adds the spans of those typed names (`#name` or `@name`, case-insensitive) to `v.plain`. wasm `keepAsText`.
  - `AddPlan.project: Option<String>` — the new project to create first.
  - `pub fn task_with_project(task: Action, project_id: Id) -> Action` — a `TaskAdd` with its project set and section cleared; other actions unchanged. wasm `taskWithProject`.
  - `pub fn with_project(v: &EditorValues, project_id: Id) -> EditorValues` — the values moved to a just-created project (section cleared); the task view uses it on `applyTitle`'s result before `saveEdit`. wasm `withProject`.

- [ ] **Step 1: Write the failing tests:**
  - `new_names_lists_typed_projects_and_labels`: known project Work, known label `home`; title `"Buy ink #wrok @errands @Home @office"` → `NewNames { projects: ["wrok"], labels: ["errands", "office"] }`.
  - `picked_label_is_not_offered`: same title, `v.labels = ["errands"]` (picked) → labels `["office"]`.
  - `segments_mark_new_names`: `editor_view(..)` for that title → segments for `#wrok`, `@errands`, `@office` have `is_new`, `@Home` doesn't; `view.new_project == Some("wrok")`.
  - `kept_names_become_text`: `keep_as_text(v, ["office", "wrok"])` then `add_plan` → `project == None`, labels to create `["errands"]`, task content `"Buy ink #wrok @office"`.
  - `kept_name_after_multibyte_text`: title `"café @office"`, `keep_as_text(["office"])` → `plain == [(6, 13)]` (byte offsets after the 5-byte "café ").
  - `add_plan_names_the_new_project`: title `"Buy ink #wrok"` → `AddPlan.project == Some("wrok")`, task's project is the card's.
  - `task_with_project_moves_the_add`: `task_with_project(TaskAdd{project_id:"P-IN", section_id:Some("S")..}, "P-NEW")` → project `P-NEW`, section `None`.
  - `retyped_title_with_new_project`: editing a task in P-HOME, retyped title `"Buy ink #wrok"` → `new_names` lists `wrok`; `apply_title` keeps project P-HOME; `with_project(after, "P-NEW")` then `edit_plan` → `moved == Some(TaskMove{project_id:"P-NEW", section_id: None})`, content `"Buy ink"`.
- [ ] **Step 2: Run** `cargo test -p keel-core editor` → FAIL.
- [ ] **Step 3: Implement** the interfaces above; add the wasm exports in `core-wasm/src/lib.rs` following `addPlan`'s pattern; add the wrappers in `web/src/core/index.ts`; pass `labels` at every `editorView` call (the add card, sub-task card, task view already hold `labels`).
- [ ] **Step 4: Run** `cargo test`, `scripts/check-bindings.sh`, `cd web && pnpm check` → PASS.
- [ ] **Step 5: Commit** `feat(core): new names in quick add: flags, the save prompt's list, the add plan`

---

### Task 5: Occurrences

**Files:**
- Modify: `crates/core/src/views.rs`
- Test: `views.rs` tests

**Interfaces:**
- Produces: `pub fn occurs_in(task: &Task, from: NaiveDate, to: NaiveDate) -> bool` — the task has an occurrence `d` with `from <= d < to`. Occurrences are its next date (`due_date_of`), and, when it repeats, every later `d` with `rule.contains(d)` and `d <= until` (when set). An undated task has none. Plus `pub fn occurs_on(task, d) -> bool` (= `occurs_in(task, d, d+1)`) and `pub fn occurs_after(task, d) -> bool` (any occurrence `> d`).

- [ ] **Step 1: Write the failing tests** (today Sun 2026-11-08):
  - one-off due 11-11: `occurs_on(11-11)` true, `occurs_on(11-12)` false.
  - daily due 11-08: `occurs_on(11-11)` true; `occurs_on(11-07)` false (before its next date).
  - weekly Mon due 11-09: `occurs_on(11-16)` true, `occurs_on(11-17)` false; `occurs_in(11-10, 11-16)` false; `occurs_in(11-10, 11-17)` true.
  - daily due 11-08, `until` 11-10: `occurs_on(11-11)` false.
  - moved: weekly Mon, due moved to 11-11 (`moved_from` 11-09): `occurs_on(11-11)` true, `occurs_on(11-09)` false, `occurs_on(11-16)` true.
  - `occurs_after`: daily without end → true for 11-30; with `until` 11-10 → false for 11-10.
- [ ] **Step 2: Run** `cargo test -p keel-core occurs` → FAIL.
- [ ] **Step 3: Implement** with `Recurrence::contains` / `next_after` (walk `next_after` from the next date while `< to`, bounded by `until`).
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): a task's occurrences in a date range`

---

### Task 6: Filter parser: quotes, text, phrases, spans

**Files:**
- Modify: `crates/core/src/filter.rs`, `crates/service/src/lib.rs` (callers pass `today`)
- Test: `filter.rs` tests

**Interfaces:**
- Consumes: `phrases::*` (Task 1), `RepeatSpec` (Task 2).
- Produces:
  - `pub struct FilterNames { pub projects: Vec<String>, pub sections: Vec<String>, pub labels: Vec<String> }` (`Default`) — the known names, so multi-word names read whole, as quick add's `match_name` does. Service builds it from the repo; `filter::read` (Task 8) from its arguments.
  - `pub fn parse(query: &str, today: NaiveDate, names: &FilterNames) -> Result<Vec<Expr>, FilterError>` (was `parse(query)`). Relative phrases resolve against `today`; callers parse on every run, so filters stay dynamic.
  - `pub fn spans(query: &str, today: NaiveDate, names: &FilterNames) -> Vec<Span>` — every recognized term's byte range and kind, even when the query has an error elsewhere (best effort per piece).
  - `FilterError` becomes `pub struct FilterError { pub message: String, pub at: Option<(usize, usize)> }`; `Display` prints `message` only.
  - New `Term` variants: `Text(String)` (matches like `search::matches` on its words), `Repeat(RepeatSpec)`, `DueAt(NaiveDate, NaiveTime)`, `Streak`. `Term::Search` stays (it's `search:`).
  - `SpanKind::Keyword` (appended) for `overdue`, `no date`, `no deadline`, `recurring`, `no labels`, `subtask`, `all`, `next N days`, `N days`, `streak`, and colon terms (`due before: …` etc.).
- Reading rules:
  - The tokenizer reads `"…"` as one quoted text token (operators inside are text; an unclosed quote runs to the end of the query; `""` yields nothing) and keeps each token's byte range.
  - A text piece that starts with a colon keyword (`due:`, `due before:`, `due after:`, `deadline:`, `deadline before:`, `deadline after:`, `search:`) is read whole as today, one `Keyword` span.
  - Otherwise the piece is scanned word by word: filter keywords first (longest first: `no deadline`, `no labels`, `no date`, `next N days`, `N days`, `overdue`, `recurring`, `subtask`, `all`, `streak`), then `p1`–`p4`, `{date}` (deadline on), `##name` / `#name`, `/name`, `@name` (the longest known name via `match_name`; an unknown name is one word, still a `Project`/`Section`/`Label` term that matches nothing), a repeat phrase, a date phrase (with a time after it → `DueAt`), a time alone (→ `DueAt(today, time)`). Leftover words join into one `Text` term. All terms of a piece are ANDed.
  - An `every …` phrase it can't fully read stays text as a whole (quick add's rule).
  - Errors: empty query, operator and parenthesis errors, and a colon term with a bad date, as today. A plain unknown word is never an error.

- [ ] **Step 1: Write the failing tests** (filter ctx today Mon 2026-10-19; names: projects Work, Home; section Next Up; label home; update the `m` helper to `parse(query, ctx().today, &names())`):
  - `multi_word_names_read_whole`: `"/Next Up & tomorrow"` → `Section("next up")` and a date; `"#Gone today"` → `Project("gone")` and `Today`.
  - `words_and_phrases_are_anded`: `parse("buy milk tomorrow")` → one group, `And(Text("buy milk"), DueOn(On(2026-10-20)))` (or the equivalent tree; assert by matching tasks: "Buy milk" due 10-20 yes, due 10-21 no, "Buy bread" due 10-20 no).
  - `quoted_word_is_text`: `"\"tomorrow\""` matches a task titled "Call mom tomorrow" with no date, not an untitled task due 10-20.
  - `quotes_keep_operators`: `"\"a & b\""` → one `Text("a & b")`, no error.
  - `unclosed_and_empty_quotes`: `"\"tomorrow"` parses as `Text("tomorrow")`; `"buy \"\""` → `Text("buy")`; `"\"\""` → error "The query is empty".
  - `quick_add_dates_in_filters`: `"in 3 days"` → `DueOn(On(2026-10-22))`; `"next friday"` → `On(2026-10-30)`… use what `match_date` gives for Mon 10-19 (`next friday` = 10-30 since 10-23 is this week); `"nov 30"` → `On(2026-11-30)`.
  - `repeat_phrase_term`: `"every 2 weeks"` → `Term::Repeat(EveryNWeeks(2, None))`.
  - `date_and_time`: `"tomorrow at 5pm"` → `DueAt(2026-10-20, 17:00)`.
  - `old_queries_still_parse`: every query string used in the existing tests (`today`, `overdue | @home`, `#Work & p1`, `(today | overdue) & !@x`, `due before: +3 days`, `deadline: 2026-10-31`, `7 days`, `next 7 days`, `no date, overdue`, `search: Milk`, `/Next Up`) parses without error.
  - `next_seven_days_is_a_range`: `"next 7 days"` → `NextDays(7)`, not a date.
  - `invalid_queries_error` becomes: `""`, `"&"`, `"today &"`, `"(today"`, `"today)"`, `"due: blursday"` error; `"foo"`, `"p5"`, `"3 dayz"` now parse as text.
  - `spans_for_highlighting`: `spans("buy milk tomorrow & #Work", today, &names())` → `[Date 9..17, Project 20..25]`; `spans("\"tomorrow\" p1", today, &names())` → `[Priority 11..13]`.
- [ ] **Step 2: Run** `cargo test -p keel-core filter` → FAIL.
- [ ] **Step 3: Implement** per the rules above. Update `keel-service` (`check_filter`, `filter_view_at`) to pass `today` and the `FilterNames` built from projects, sections and labels.
- [ ] **Step 4: Run** `cargo test` → PASS (matching tests for repeating tasks may fail until Task 7; mark them `#[ignore]` only if they do, and say so in the commit).
- [ ] **Step 5: Commit** `feat(core): filters read text, quotes and quick add's phrases, with spans`

---

### Task 7: Filter matching: occurrences and the new terms

**Files:**
- Modify: `crates/core/src/filter.rs` (`term_matches`), `crates/service/src/lib.rs` (test only)
- Test: `filter.rs` tests, `crates/service` tests

**Interfaces:**
- Consumes: `views::occurs_on` / `occurs_in` / `occurs_after` (Task 5), `phrases::repeat_matches` (Task 2).
- Matching: `Today` → `occurs_on(today)`; `Tomorrow` → `occurs_on(today+1)`; `DueOn(r)` → `occurs_on(r)`; `DueAt(d, t)` → `occurs_on(d)` and the due's time is `t`; `DueAfter(r)` → `occurs_after(r)`; `NextDays(n)` → `occurs_in(today, today+n)`; `DueBefore(r)` and `Overdue` unchanged (next date); `Repeat(spec)` → the task's rule `repeat_matches`; `Streak` → `task.streak_enabled`; `Text(s)` → `search::matches(task, &search::words(s))`. Deadline terms unchanged.

- [ ] **Step 1: Write the failing tests** (today Mon 2026-10-19):
  - `dates_match_occurrences`: daily task due 10-19 matches `today`, `tomorrow`, `in 3 days`, `next 7 days`, `due: +5 days`, `due after: +10 days`; with `until` 10-20 it matches `tomorrow` but not `in 3 days`.
  - `weekly_task_in_three_days`: weekly Thursday task due 10-22 matches `in 3 days`; on today 10-20 (`FilterContext.today`) it doesn't, and `in 3 days` = 10-23.
  - `overdue_is_next_date_only`: daily due 10-17 matches `overdue`; daily due 10-19 doesn't.
  - `repeat_terms_match_shape`: task `EveryNWeeks{n:2, days:[3], anchor:2026-10-01}` with `until` matches `every 2 weeks`; `Weekly([0,2])` matches `every monday`.
  - `streak_term`: `streak_enabled` task matches `streak`; another doesn't.
  - `time_term`: task due `2026-10-20T17:00:00` matches `tomorrow at 5pm`, not `tomorrow at 6pm`.
  - Service `saved_filter_with_deleted_project_still_runs`: `filter_view_at("#Gone, today", 10-19)` → `Ok`, two groups, first empty, second has today's tasks.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement** `term_matches` as listed. Remove any `#[ignore]` from Task 6.
- [ ] **Step 4: Run** `cargo test` → PASS.
- [ ] **Step 5: Commit** `feat(core): filter dates match occurrences; repeat, time, streak and text terms`

---

### Task 8: Filter reading for the editors: segments, unknown names, fix, backspace

**Files:**
- Modify: `crates/core/src/filter.rs`, `crates/core/src/phrases.rs`, `crates/core-wasm/src/lib.rs`, `web/src/core/index.ts`, bindings
- Test: `filter.rs`, `phrases.rs` tests

**Interfaces:**
- Consumes: `filter::parse`, `filter::spans` (Task 6), `editor::title_segments`, `TitleSegment.unknown` (Task 4).
- Produces:
  - `pub(crate) fn phrases::closest<'a>(name: &str, names: &[&'a str]) -> Option<&'a str>` — case-insensitive Levenshtein; returns the nearest name with distance ≤ 2 and ≤ a third of `name`'s length (rounded up), ties to the first.
  - `pub struct FilterFix { pub label: String, pub query: String }` — e.g. label `"Use #Work"`, query = the whole query with `#wrok` replaced by `#Work`.
  - `pub struct FilterReading { pub segments: Vec<TitleSegment>, pub problem: Option<String>, pub fix: Option<FilterFix> }`.
  - `pub fn read(query: &str, today: NaiveDate, projects: &[Project], sections: &[Section], labels: &[Label]) -> FilterReading` — segments from `spans` (date segments carry their due bucket, priorities their level, as `editor_view` does); a `#`/`##`, `/` or `@` segment naming nothing known has `unknown: true`; `problem` is the parse error's message, else the first unknown name's ("No project named “wrok”."); `fix` for that unknown name when `closest` finds one. A syntax error with `at` marks that range `unknown` too. wasm `filterReading`.
  - `pub fn unhighlight(query: &str, today: NaiveDate, end: usize) -> Option<String>` — the query with the span ending at byte `end` wrapped in quotes (`tomorrow` → `"tomorrow"`); `None` when no span ends there. wasm `filterUnhighlightAt(query, today, caretUtf16)` converts with `editor::byte_at_utf16`.
- [ ] **Step 1: Write the failing tests:**
  - `closest_name`: `closest("wrok", ["Work","Home"]) == Some("Work")`; `closest("xyz", ["Work"]) == None`; `closest("hme", ["Home","Work"]) == Some("Home")`.
  - `unknown_project_is_red_with_a_fix`: `read("printer ink & today & #wrok", ..)` with project Work → the `#wrok` segment `unknown`, `problem == Some("No project named “wrok”.")`, `fix == Some(FilterFix{label:"Use #Work", query:"printer ink & today & #Work"})`.
  - `known_names_are_fine`: `read("#work & @home & /next up", ..)` with those names → no `unknown`, `problem == None`.
  - `syntax_error_wins`: `read("today &", ..)` → `problem` is the parse error message.
  - `backspace_quotes_the_term`: `unhighlight("call mom tomorrow", today, 17) == Some("call mom \"tomorrow\"")`; at 8 → `None`.
  - `backspace_after_multibyte`: `filterUnhighlightAt`'s Rust core: `unhighlight("café tomorrow", today, 14) == Some("café \"tomorrow\"")`.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement**; add the wasm exports and TS wrappers; regenerate bindings.
- [ ] **Step 4: Run** `cargo test`, `scripts/check-bindings.sh`, `cd web && pnpm check` → PASS.
- [ ] **Step 5: Commit** `feat(core): filter reading for the editors: unknown names, a fix, backspace to quotes`

---

### Task 9: Web: yellow new names in the card

**Files:**
- Modify: `web/src/editor/titleMarks.ts` (+ `titleMarks.test.ts`), `web/src/editor/TaskEditor.tsx`, `web/src/styles/app.css`, `web/src/styles/contrast.test.ts`
- Test: `titleMarks.test.ts`, `editor.test.tsx`

**Interfaces:**
- Consumes: `TitleSegment.is_new` / `.unknown`, `EditorView.new_project` / `.new_labels` (Task 4).
- Produces: `markClass(s)` appends `" hl-new"` when `s.is_new` and `" hl-unknown"` when `s.unknown`; `markStyle` returns `undefined` for a new segment (yellow overrides the label's color). Pill class `pill-new`.
- [ ] **Step 1: Write the failing tests:**
  - `titleMarks.test.ts`: a `Label` segment with `is_new: true` → class contains `hl-new`, `markStyle` undefined; `unknown: true` → `hl-unknown`.
  - `editor.test.tsx` (mock backend): typing "Buy ink #wrok @errands" in the add card → the location pill has class `pill-new` and text "wrok"; the labels pill has `pill-new` (it holds a new label). Typing "@home" alone (known) → no `pill-new`.
- [ ] **Step 2: Run** `cd web && pnpm vitest run titleMarks editor` → FAIL.
- [ ] **Step 3: Implement**: `markClass`/`markStyle`; the location pill shows `view.new_project` with `pill-new` when it's set; the labels pill gets `pill-new` when `view.new_labels` is non-empty. CSS: `.hl-new` (yellow tint + dotted underline), `.hl-unknown` (dotted `var(--p1)` underline), `.chip-btn.pill-new` (fill, text, dotted border; wins over `.pill-set` and label colors), with dark-theme tokens. Add the yellow text on its fill to `contrast.test.ts`.
- [ ] **Step 4: Run** `cd web && pnpm check` → PASS.
- [ ] **Step 5: Commit** `feat(web): new projects and labels show yellow in the card`

---

### Task 10: Web: the save prompt

**Files:**
- Create: `web/src/editor/NewNamesPrompt.tsx`, `web/src/editor/newNames.ts` (+ `newNames.test.ts`)
- Modify: `web/src/editor/AddTask.tsx`, `web/src/taskview/SubtaskCard.tsx`, `web/src/taskview/TaskView.tsx`, `web/src/editor/save.ts` (+ `save.test.ts`), `web/src/ipc/actions.ts`, `web/src/ipc/mock.ts`, `web/src/styles/app.css`
- Test: `newNames.test.ts`, `save.test.ts`, `AddTask.test.tsx`, the task view's tests

**Interfaces:**
- Consumes: `newNames`, `keepAsText`, `addPlan`, `taskWithProject` (Task 4).
- Produces:
  - `newNames.ts` (screen rules): `type Ticks = { projects: Record<string, boolean>; labels: Record<string, boolean> }`; `allTicked(names: NewNames): Ticks`; `groupState(ticks: Record<string, boolean>): "all" | "some" | "none"`; `toggleGroup(ticks): Record<string, boolean>` (all ticked → all off, else all on); `unticked(t: Ticks): string[]`; `countText(ticks): string` ("1 of 2").
  - `<NewNamesPrompt names onBack onSave(unticked: string[]) />` — the prompt in the card's place, groups with header checks (`role="checkbox"`, `aria-checked` `"true"|"false"|"mixed"`), rows indented, Keel's `.check` circle (`.check.done` when ticked; header "some" shows a dash), yellow pills with `#`/`@`, copy from Global Constraints; a group is hidden when it has no names; the header reads "New project"/"New label" for one name.
  - `addProjectGetId(name: string): Promise<Id>` in `ipc/actions.ts` (like `addSectionGetId`).
  - `addFromEditor(v, …)` creates `plan.project` first (via `addProjectGetId`), sends `taskWithProject(plan.task, id)`, then labels and the rest as today.
- [ ] **Step 1: Write the failing tests:**
  - `newNames.test.ts`: `groupState({a:true,b:false}) === "some"`; `toggleGroup({a:true,b:false})` → both true; `toggleGroup({a:true,b:true})` → both false; `unticked` lists `office` for `{labels:{errands:true, office:false}}`; `countText({errands:true, office:false}) === "1 of 2"`.
  - `save.test.ts`: a title with `#wrok` sends `project_add` "wrok" before `task_add`, and the task's `project_id` is the new id.
  - `AddTask.test.tsx`: submitting "Buy ink #wrok @errands" shows "Create these with the task?" with both groups ticked; "Back to editing" returns to the card with the title intact; unticking `@errands` and "Save" adds a task titled "Buy ink @errands" with no label and creates project "wrok"; a title with no new names saves straight away with no prompt.
  - Task view: retyping a task's title to "Buy ink #wrok @errands" and leaving the field shows the prompt; "Save" creates project "wrok" and label "errands", moves the task to "wrok" and saves the title "Buy ink"; "Back to editing" keeps the typed title and saves nothing.
- [ ] **Step 2: Run** `cd web && pnpm vitest run newNames save AddTask TaskView` → FAIL.
- [ ] **Step 3: Implement**. On save: `newNames(v, …)`; empty → save as today; else show the prompt; "Save" runs `keepAsText(v, unticked)` then `addFromEditor`. Same flow in `SubtaskCard`'s add. In `TaskView`, `saveDrafts` checks `newNames` on the retyped title before `applyTitle`, and shows the prompt through the same pending path the streak-reset question uses (`setPendingMove`); "Save" applies `keepAsText`, creates the ticked project (`addProjectGetId`) and labels, applies `withProject` when a project was created, then saves as today.
- [ ] **Step 4: Run** `cd web && pnpm check` → PASS.
- [ ] **Step 5: Commit** `feat(web): confirm new projects and labels when saving a task`

---

### Task 11: Web: highlighted filter fields

**Files:**
- Modify: `web/src/shell/filterQuery.ts` (+ test), `web/src/shell/FilterForm.tsx`, `web/src/screens/PageEdit.tsx`, `web/src/editor/HighlightedTitle.tsx` (allow `autoFocus` off and a `className`), `web/src/styles/app.css`
- Test: `web/src/shell/Sidebar.test.tsx`, `filterQuery.test.ts`

**Interfaces:**
- Consumes: `filterReading`, `filterUnhighlightAt` (Task 8).
- Produces: `useQueryCheck(initialQuery, names: { projects; sections; labels })` returns `{ query, setQuery, segments, problem, fix, unhighlight(caret) }` computed with `filterReading` (no IPC round trip); `check_filter` IPC is still called on save as the backstop. `FILTER_HINT` becomes: `today · in 3 days · next friday · every monday · overdue · next 7 days · p1 · #Project · /Section · @label · no date · streak · "word" · due before: +3 days · deadline before: +7 days — combine with & | ! ( ) and , for separate lists`.
- [ ] **Step 1: Write the failing tests** (`Sidebar.test.tsx`, mock data with project Work):
  - typing `printer ink & today & #wrok` in the add-filter form → a `mark.hl-unknown` over `#wrok`, the line "No project named “wrok”." and a "Use #Work" button; Save disabled; clicking "Use #Work" makes the field `printer ink & today & #Work` and enables Save.
  - typing `call mom tomorrow` then Backspace with the caret after `tomorrow` → the field reads `call mom "tomorrow"` and no `mark` covers it.
  - `today` gets `mark.hl` with the today due class, as in the card.
- [ ] **Step 2: Run** `cd web && pnpm vitest run Sidebar filterQuery` → FAIL.
- [ ] **Step 3: Implement**: both filter fields render `HighlightedTitle` with the reading's segments (class `mono` kept), the error line (`.filter-problem`) with the fix as a link-style button, Backspace calling `unhighlight`.
- [ ] **Step 4: Run** `cd web && pnpm check` → PASS.
- [ ] **Step 5: Commit** `feat(web): filter fields highlight terms, flag unknown names and offer a fix`

---

### Task 12: Verify and document

**Files:**
- Modify: `CLAUDE.md` (the parser layout: `phrases`, filter language notes, new names, occurrences), `.claude/issue-roadmap.md`

- [ ] **Step 1: Run** `cargo test`, `cargo clippy --all-targets`, `cd web && pnpm check`, `cd web && pnpm build` → all pass.
- [ ] **Step 2: Browser check** (`cd web && pnpm dev`, mock backend, 375px and desktop, light and dark): the card with "Buy printer ink today #wrok @errands @office" shows yellow highlights and pills; Save shows the prompt matching the mockup (labels header dash after unticking `@office`); the filter form shows red `#wrok`, the fix, and backspace-to-quotes. Screenshot each.
- [ ] **Step 3: Update** CLAUDE.md and the roadmap row.
- [ ] **Step 4: Commit** `docs: shared phrase vocabulary in CLAUDE.md`
- [ ] **Step 5: Stop and ask** Noah about merging (and filing the issue); don't push.
