# Notifications — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Basic local notifications (spec §9): a reminder for each open dated task (at its due time, or N minutes before; date-only tasks at a chosen time of day), plus an optional daily "due today" summary. Works on Windows now (for development) and on iOS once built on the Mac.

**Architecture:** One pure planner, `core::reminders::plan(tasks, settings, now) -> Vec<PlannedReminder>`, decides every notification that should exist (stable i32 ids, sorted, capped below iOS's 64 pending limit). The Tauri layer re-plans after every change (dispatch, settings, startup):
- **iOS/Android:** cancel our pending notifications and schedule the plan with `tauri-plugin-notification`'s `Schedule::At` — the OS delivers them while the app is closed.
- **Desktop:** the plugin can only show immediately, so a background thread fires due reminders from the current plan while the app is open (each id once).

Settings are device-local (notifications are per device), stored as JSON in a new `local_setting` key/value table that sync will never touch.

**Spec:** `docs/superpowers/specs/2026-10-19-todoist-tauri-port-design.md` (§9 basic notifications)

## Settings (defaults)
- `enabled` (false until the user turns it on — that's when permission is requested)
- `offset_minutes` (0 = at due time; 5/10/15/30/60 before)
- `all_day_time` (`Some(09:00)`; `None` = no reminder for date-only tasks)
- `daily_summary` (`None`; e.g. `Some(08:00)` — "3 tasks due today", only when > 0)

## Global Constraints
- Only open, non-deleted tasks with a due; only fire times strictly after `now`; within 14 days; at most 60 reminders (soonest first).
- Ids are stable for the same task + fire time (re-planning doesn't duplicate on desktop).
- Recurring tasks: only the current occurrence (completing re-plans the next).
- No per-task custom reminders yet (the `reminder` table stays unused).

## Review Focus
- **Date-only vs timed** dues and the offset crossing midnight — Task 1.
- **Past fire times** (overdue, or offset puts it in the past) are skipped — Task 1.
- **Desktop timer** fires each id once and never fires stale reminders after startup — Task 3.
- **Permission flow**: enabling requests permission; denied keeps it off with a message — Task 4.

---

## Task 1: `core::reminders`
- [x] `ReminderSettings` (serde, Default), `PlannedReminder { id: i32, task_id: Option<Id>, at: NaiveDateTime, title, body }`, `plan(...)`. Tests: timed at due; offset before; offset crossing midnight; date-only uses all_day_time; all_day None skips; past skipped; completed/deleted/undated skipped; horizon + cap + sort; stable ids; disabled → empty; daily summary counts per day and skips zero days. Commit.

## Task 2: Storage + service
- [x] `local_setting` table; `SqliteRepo::{get_setting, set_setting}`; `AppService::{reminder_settings, set_reminder_settings, reminder_plan_at(now)}`. Tests. Commit.

## Task 3: Tauri wiring
- [x] Register the plugin; `notifications.rs`: `resync` (mobile schedule / desktop plan), desktop ticker thread; resync after dispatch + settings + startup. IPC: `get_reminder_settings`, `set_reminder_settings`, `notification_permission`, `request_notification_permission`, `test_notification`. Commit.

## Task 4: UI
- [x] Settings panel (gear in the sidebar header): Notifications toggle (requests permission), "Remind me" (at due time / N min before), "Tasks without a time" (none / time), daily summary (off / time), "Send test notification", permission status. Commit.

## Task 5: Mac checklist
- [x] `docs/notes/ios-notifications-checklist.md`: `cargo tauri ios init`, run, permission prompt, closed-app delivery, tap opens app.
