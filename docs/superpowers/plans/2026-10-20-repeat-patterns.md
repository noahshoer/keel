# Extra Repeat Patterns — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The NL long tail from spec §8 that quick add currently leaves in the title: intervals (`every 2 weeks`, `every other day`, `every 3 months`), yearly (`every year`, `every jan 5`), and month-relative days (`every last day`, `every last workday`, `every first monday`, `every 3rd of the month`).

**Architecture:** New `Recurrence` variants in `core::recurrence`. Every consumer (completion rescheduling, streaks, reminders, the quick-add first occurrence) only calls `contains` / `next_after` / `prev_before`, so they work unchanged once the engine does. Interval rules carry an **anchor** date (their phase), set from the parsed date or today. Variants are added, never changed, so existing stored rules (JSON in `Due`) still deserialize. One `Recurrence::describe()` replaces the two copies of rule-to-text (commands' `Due.string` and the editor's chip).

## New variants
- `EveryNDays { n, anchor }` — every N days from the anchor.
- `EveryNWeeks { n, days, anchor }` — on `days` (Mon=0…) in every Nth week counted from the anchor's week.
- `EveryNMonths { n, day, anchor }` — the `day`-th (clamped) of every Nth month from the anchor's month.
- `Yearly { month, day }` — once a year (Feb 29 → Feb 28 in non-leap years).
- `MonthlyLastDay`
- `MonthlyWorkday { nth }` — first (`1`) or last (`-1`) Mon–Fri of the month.
- `MonthlyNthWeekday { nth, weekday }` — `nth` 1–4, or `-1` = last.

## Phrases (in addition to today's)
- `every N days|weeks|months|years`, `every other day|week|month|year`, `every 2 weeks on mon and thu`, `every other monday`
- `every year`, `yearly`, `annually`, `every jan 5`, `every 5 january`, `every january 5th`
- `every last day [of the month]`, `every month on the last day`
- `every workday` (= every weekday), `every first workday`, `every last workday`
- `every first|second|third|fourth|last|1st|2nd|3rd|4th monday`
- `every 3rd of the month`, `every 15th of every month`
- `every N years` → every 12·N months on the anchor's day.

## Global Constraints
- No silent misreads: anything not fully understood stays in the title (e.g. `every 2` alone, `every 0 days`, `every 5th monday`).
- `n` is 1–99 (`every 1 week` = weekly).
- Editor repeat menu keeps its five presets; any other rule shows as a sixth "keep" option with its description.

## Review Focus
- **Anchor phase**: `every 2 weeks` from a Wednesday fires on alternate Wednesdays; `prev_before` and `next_after` agree with `contains` — Task 1.
- **Clamping**: monthly 31st, yearly Feb 29, last workday when the month ends on a weekend — Task 1.
- **Parser precedence**: `every 3rd monday` (nth weekday) vs `every 3rd` (day of month) vs `every 3 days` (interval) — Task 2.

---

## Task 1: Engine
- [x] Variants + `contains`/`next_after`/`prev_before` (bounded scans for the complex ones) + `describe()`. Property test: for every variant and many dates, `next_after(d)` is the first `contains` day after `d` and `prev_before` mirrors it. Commit.

## Task 2: Parser
- [x] `match_repeat` forms above; anchors from the parsed date (else today); first occurrence on/after the anchor. Tests per phrase + no-misread cases + spans. Commit.

## Task 3: Wire-up
- [x] `commands::describe` → `Recurrence::describe`; editor chip text + "keep" option label from `describe`. `dx build` clean; verify in preview. Commit, merge, push.
