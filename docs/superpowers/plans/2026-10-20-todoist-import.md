# Todoist CSV Import — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Import projects exported from Todoist ("Manage data → Export as CSV", one CSV per project) into Keel: sections, tasks with sub-task nesting, priorities, labels, due dates (incl. recurring), deadlines, descriptions and comments.

**Format** (Todoist help, "Import or export a project as a CSV file"): header row, then one row per item. Columns, matched by name in any order: `TYPE` (`task` | `section` | `note` | `meta`), `CONTENT`, `DESCRIPTION`, `PRIORITY` (1 = p1 … 4 = p4), `INDENT` (1 = top level, 2 = sub-task …), `AUTHOR`, `RESPONSIBLE`, `DATE` (natural language, may recur), `DATE_LANG`, `TIMEZONE`, `DURATION`, `DURATION_UNIT`, `DEADLINE`, `DEADLINE_LANG`, `IS_COLLAPSED`. Labels are inline `@name` in `CONTENT`. A `note` is a comment on the task above. Exports contain blank separator rows and may start with a UTF-8 BOM.

**Architecture:**
- `core::import::csv` — RFC 4180 reader (quotes, escaped quotes, embedded newlines, CRLF, BOM).
- `core::import::todoist::{parse, plan}` — rows → `TodoistItem`s → an ordered list of `ImportOp`s with symbolic keys (sections and parent tasks referenced by index), all pure and host-tested. Dates go through `quickadd::parse` ("x " + DATE) and are accepted only when the *whole* text is understood; deadlines likewise (date only). ISO `YYYY-MM-DD[THH:MM]` is accepted directly.
- `AppService::import_todoist_csv(project_name, text)` executes the plan through normal commands (so it lands in the command log): `ProjectAdd`, `LabelAdd` for unknown labels, `SectionAdd`, `TaskAdd` (parent/section resolved from earlier results). Returns `ImportSummary { project_id, project_name, tasks, sections, labels, comments, kept_as_text }`.
- UI: Settings → "Import from Todoist" → file picker (multiple `.csv`), one new project per file named after the file; shows the summary.

## Rulings
- **No silent misreads:** a DATE or DEADLINE we can't fully parse (including non-English `DATE_LANG`) is not guessed — the task keeps no due/deadline and the original text is appended to its description ("Todoist date: …"), and it's counted in `kept_as_text`.
- **Empty PRIORITY → p4.** (Todoist's doc says an empty cell means p1 on *their* import; exports always fill it, and hand-made CSVs shouldn't turn every task urgent.)
- **Comments** (no comments feature in v1) are appended to the task's description under "Comments:" so nothing is lost.
- AUTHOR / RESPONSIBLE / TIMEZONE / DURATION / `meta` rows are ignored. Section descriptions and `IS_COLLAPSED` are ignored (no section descriptions yet).
- An INDENT deeper than the previous task + 1 attaches to the nearest valid parent (Todoist does the same).
- Header names are case-insensitive; a file without `TYPE` and `CONTENT` columns is rejected with a clear error.

## Review Focus
- CSV edge cases (quoted commas/newlines, `""`, BOM, CRLF, trailing blank rows) — Task 1.
- Indent → parent mapping across sections (a section resets nesting) — Task 2.
- Labels only from `@word` tokens; title words like "tomorrow" or "p1" are NOT parsed — Task 2.

---

## Task 1: CSV reader
- [ ] `core::import::csv::parse(text) -> Vec<Vec<String>>`. Tests. Commit.

## Task 2: Todoist mapping
- [ ] `parse` (header mapping, rows → items, notes attach to previous task) and `plan` (ops with parent/section keys; dates/deadlines/priority/labels/descriptions). Tests incl. a realistic export. Commit.

## Task 3: Service + IPC
- [ ] `AppService::import_todoist_csv` + IPC `import_todoist_csv(project_name, text)`. Service test end to end. Commit.

## Task 4: UI
- [ ] Settings → Import section: file picker, per-file result lines, reload. Verify in the real app over the WebView debug port with a sample export. Merge, push.
