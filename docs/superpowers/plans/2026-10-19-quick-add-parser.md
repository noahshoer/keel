# Natural-Language Quick Add — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Typing a task like `Call mom tomorrow at 5pm p1 #Family @phone` fills in date, time, repeat, priority, project/section and labels — previewed live, applied on add — Todoist-style.

**Architecture:** `core::quickadd::parse(input, today, projects, sections) -> QuickAdd` is a pure, host-tested tokenizer/matcher (compiled into the wasm UI, so no IPC). It strips recognized phrases from the title. `core::views::time_label` formats due times. The UI previews the parse under the title as chips, applies it when adding, and gains a time field in the editor (times are stored as `YYYY-MM-DDTHH:MM:00` in `Due.date`, which completion already preserves).

**Spec:** `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md` (§8; this is the core tier — recurrence limited to the existing rule types)

## Supported phrases (English, case-insensitive)
- **Dates:** `today`/`tod`, `tomorrow`/`tmr`/`tom`/`tmrw`, weekdays (`mon`, `monday`, …; today if it's that day, else next), `next <weekday>` (next calendar week), `next week` (Monday), `next month` (the 1st), `weekend`/`this weekend` (Saturday), `next weekend`, `in N day(s)/week(s)/month(s)`, `oct 20`, `october 20`, `20 oct`, `oct 20th`, optional year (`oct 20 2027`; without a year a past date rolls to next year), ISO `2026-10-20`. Optional leading `on`.
- **Times:** `5pm`, `5 pm`, `5:30pm`, `17:00`, `noon`, `midnight`, optional leading `at`. A time alone means today.
- **Repeats:** `every day`/`daily`/`everyday`, `every weekday`, `every week`/`weekly`, `every month`/`monthly`, `every mon`, `every mon, wed and fri`, `every 15th`, `every month on the 15th`. Week/month rules anchor on the parsed date (else today). Unsupported repeats (`every 2 weeks`, `every year`) stay in the title.
- **Priority:** `p1`–`p4`. **Project:** `#Name` (multi-word names matched greedily, case-insensitive; unknown stays text). **Section:** `/Name` within the chosen project (else Inbox); unknown stays text. **Labels:** `@name` (new labels are created on add).

## Global Constraints
- Only the first date, first time and first repeat are taken; later ones stay in the title.
- If stripping leaves an empty title, the original text is kept as the title.
- Parsing applies when **adding** tasks (editing existing titles is never reparsed).
- Chips the user set explicitly in the editor are kept unless the text specifies that field.

## Review Focus
- **Words inside the title that look like dates** only match as whole words (e.g. "today's" is text) — Task 1.
- **Month-day without year in the past** rolls to next year — Task 1.
- **Unknown #project / /section** stays in the title rather than vanishing — Task 1.
- **Repeat + time** (`every weekday at 9am`) yields first occurrence with the time — Task 1.
- **12-hour edge cases** (`12am` = 00:00, `12pm` = 12:00) — Task 1/2.

---

## Task 1: `core::quickadd`
- [ ] Tests (today = Mon 2026-10-19): relative dates; weekdays incl. same-day; next weekday/week/month/weekend; in N units; month-day forms + rollover + year; ISO; times incl. 12am/12pm/noon; time-only → today; repeats (all forms) incl. anchoring and with time; unsupported repeat stays text; priority; project greedy + unknown; section in project + unknown; labels; first-wins; empty-title fallback; whole-word only. RED → GREEN → commit.

## Task 2: `views::time_label`
- [ ] `fn due_time(task) -> Option<NaiveTime>`, `fn time_label(t: NaiveTime) -> String` ("5 PM", "5:30 PM", "12 AM"); tests. Commit.

## Task 3: UI
- [ ] Editor: time field; add-mode live preview chips from `quickadd::parse`; apply on add (create unknown labels first); rows show time in the due chip. `dx build` clean; stage; commit.
