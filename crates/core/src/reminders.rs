//! Local notification planning: which reminders should exist right now.
//! Pure (host-tested); the app re-plans after every change and hands the
//! result to the OS (mobile) or a timer (desktop).

use std::collections::HashMap;

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::model::{Reminder, ReminderType, Task};
use crate::views;

/// Device-local notification preferences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct ReminderSettings {
    pub enabled: bool,
    /// Minutes before a timed due (0 = at the due time).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub offset_minutes: i64,
    /// When to remind about date-only tasks (`None` = don't).
    pub all_day_time: Option<NaiveTime>,
    /// A daily "N tasks due today" summary at this time (`None` = off).
    pub daily_summary: Option<NaiveTime>,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        ReminderSettings {
            enabled: false,
            offset_minutes: 0,
            all_day_time: NaiveTime::from_hms_opt(9, 0, 0),
            daily_summary: None,
        }
    }
}

/// One notification to deliver at local time `at`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PlannedReminder {
    /// Stable for the same task (or summary day) and fire time.
    pub id: i32,
    /// `None` for the daily summary.
    pub task_id: Option<Id>,
    pub at: NaiveDateTime,
    pub title: String,
    pub body: String,
}

/// How far ahead reminders are planned.
pub const HORIZON_DAYS: i64 = 14;
/// iOS keeps at most 64 pending local notifications; leave headroom.
pub const MAX_REMINDERS: usize = 60;

/// Every reminder that should be pending at `now` (local time), soonest first:
/// the global rule, each task's own reminders, and "deadline today".
pub fn plan(
    tasks: &[Task],
    reminders: &[Reminder],
    settings: &ReminderSettings,
    now: NaiveDateTime,
) -> Vec<PlannedReminder> {
    if !settings.enabled {
        return Vec::new();
    }
    let horizon = now + Duration::days(HORIZON_DAYS);
    let open = || tasks.iter().filter(|t| !t.checked && !t.is_deleted);
    let by_id: HashMap<&Id, &Task> = open().map(|t| (&t.id, t)).collect();
    let mut out: Vec<PlannedReminder> = Vec::new();
    // One notification per task and time; a deadline on the same moment is
    // folded into the existing text.
    let mut push = |task: &Task, fire: NaiveDateTime, body: String| {
        if fire <= now || fire > horizon {
            return;
        }
        if let Some(existing) = out.iter_mut().find(|r| r.task_id.as_ref() == Some(&task.id) && r.at == fire) {
            if body == DEADLINE_BODY && !existing.body.contains(DEADLINE_BODY) {
                existing.body = format!("{} · {DEADLINE_BODY}", existing.body);
            }
            return;
        }
        out.push(PlannedReminder {
            id: stable_id(&format!("{}|{fire}", task.id)),
            task_id: Some(task.id.clone()),
            at: fire,
            title: task.content.clone(),
            body,
        });
    };

    // The global rule: at (or N minutes before) the due time; date-only
    // tasks at the all-day time.
    for task in open() {
        let Some((base, timed)) = due_base(task, settings) else { continue };
        let fire = if timed { base - Duration::minutes(settings.offset_minutes.max(0)) } else { base };
        push(task, fire, due_body(task, fire));
    }

    // Each task's own reminders.
    for r in reminders.iter().filter(|r| !r.is_deleted) {
        let Some(task) = by_id.get(&r.task_id).copied() else { continue };
        let fire = match r.kind {
            ReminderType::Relative => match due_base(task, settings) {
                Some((base, _)) => base - Duration::minutes(r.minute_offset.unwrap_or(0).max(0)),
                None => continue,
            },
            ReminderType::Absolute => match r.at() {
                Some(at) => at,
                None => continue,
            },
        };
        push(task, fire, due_body(task, fire));
    }

    // "Deadline today", at the all-day time.
    if let Some(time) = settings.all_day_time {
        for task in open() {
            let deadline = task.deadline.as_deref().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
            if let Some(day) = deadline {
                push(task, day.and_time(time), DEADLINE_BODY.to_string());
            }
        }
    }

    if let Some(time) = settings.daily_summary {
        let mut day: NaiveDate = now.date();
        while day.and_time(time) <= horizon {
            let fire = day.and_time(time);
            let count = open().filter(|t| views::due_date_of(t) == Some(day)).count();
            if fire > now && count > 0 {
                let noun = if count == 1 { "task" } else { "tasks" };
                out.push(PlannedReminder {
                    id: stable_id(&format!("summary|{fire}")),
                    task_id: None,
                    at: fire,
                    title: "Today".to_string(),
                    body: format!("{count} {noun} due today"),
                });
            }
            day += Duration::days(1);
        }
    }

    out.sort_by(|a, b| a.at.cmp(&b.at).then(a.id.cmp(&b.id)));
    out.truncate(MAX_REMINDERS);
    out
}

const DEADLINE_BODY: &str = "Deadline today";

/// What reminders count back from: the due date and time, or for a
/// date-only task the all-day time. `bool` = the task has a time.
fn due_base(task: &Task, settings: &ReminderSettings) -> Option<(NaiveDateTime, bool)> {
    let date = views::due_date_of(task)?;
    match views::due_time(task) {
        Some(time) => Some((date.and_time(time), true)),
        None => settings.all_day_time.map(|t| (date.and_time(t), false)),
    }
}

/// Notification text relative to when it fires: "Due at 5 PM",
/// "Due in 30 min (5 PM)", "Due tomorrow at 5 PM", "Due today", "Reminder".
fn due_body(task: &Task, fire: NaiveDateTime) -> String {
    let Some(date) = views::due_date_of(task) else { return "Reminder".to_string() };
    let day = fire.date();
    let when = || {
        let label = views::due_label(date, day);
        match label.as_str() {
            "Today" | "Tomorrow" | "Yesterday" => label.to_lowercase(),
            _ => label,
        }
    };
    match views::due_time(task) {
        Some(time) if date == day => {
            let due = date.and_time(time);
            if fire >= due {
                format!("Due at {}", views::time_label(time))
            } else {
                format!("Due in {} ({})", minutes_text((due - fire).num_minutes()), views::time_label(time))
            }
        }
        Some(time) => format!("Due {} at {}", when(), views::time_label(time)),
        None if date == day => "Due today".to_string(),
        None => format!("Due {}", when()),
    }
}

/// "30 min", "1 hr", "1 hr 30 min".
fn minutes_text(m: i64) -> String {
    match (m / 60, m % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} hr"),
        (h, m) => format!("{h} hr {m} min"),
    }
}

/// A positive i32 from FNV-1a (notification ids must be i32 on mobile).
fn stable_id(key: &str) -> i32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in key.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    ((h & 0x7fff_ffff) as i32).max(1)
}

/// A relative reminder's text: "At time of task", "10 min before",
/// "1 hour before", "2 days before" (the largest whole unit).
pub fn offset_text(minutes: i64) -> String {
    match minutes {
        0 => "At time of task".to_string(),
        m if m % 1440 == 0 => {
            let d = m / 1440;
            format!("{d} day{} before", if d == 1 { "" } else { "s" })
        }
        m if m % 60 == 0 => {
            let h = m / 60;
            format!("{h} hour{} before", if h == 1 { "" } else { "s" })
        }
        m => format!("{m} min before"),
    }
}

/// A reminder's text: its offset ("30 min before") or its time ("Tuesday
/// 9 AM"); "Reminder" when an absolute one has no readable time.
pub fn reminder_label(r: &Reminder, today: NaiveDate) -> String {
    if r.kind == ReminderType::Relative {
        if let Some(m) = r.minute_offset {
            return offset_text(m);
        }
    }
    match r.at() {
        Some(at) => format!("{} {}", views::due_label(at.date(), today), views::time_label(at.time())),
        None => "Reminder".to_string(),
    }
}

#[cfg(test)]
mod label_tests {
    use super::{offset_text, reminder_label};
    use crate::ids::Id;
    use crate::model::Reminder;
    use chrono::NaiveDate;

    #[test]
    fn offsets_read_in_the_largest_whole_unit() {
        assert_eq!(offset_text(0), "At time of task");
        assert_eq!(offset_text(10), "10 min before");
        assert_eq!(offset_text(60), "1 hour before");
        assert_eq!(offset_text(120), "2 hours before");
        assert_eq!(offset_text(1440), "1 day before");
        assert_eq!(offset_text(2880), "2 days before");
        assert_eq!(offset_text(90), "90 min before");
    }

    #[test]
    fn reminders_label_by_offset_or_time() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        assert_eq!(reminder_label(&Reminder::relative(Id::from("t"), 30), today), "30 min before");
        let tue = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap().and_hms_opt(9, 0, 0).unwrap();
        assert_eq!(reminder_label(&Reminder::absolute(Id::from("t"), tue), today), "Tuesday 9 AM");
        let mut broken = Reminder::absolute(Id::from("t"), tue);
        broken.due = None;
        assert_eq!(reminder_label(&broken, today), "Reminder");
    }
}

#[cfg(test)]
mod tests {
    use super::{plan, PlannedReminder, ReminderSettings, MAX_REMINDERS};
    use crate::model::Reminder;
    use crate::ids::Id;
    use crate::model::{Due, Task};
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, min, 0).unwrap()
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    /// Mon 2026-10-19 08:00.
    fn now() -> NaiveDateTime {
        at(2026, 10, 19, 8, 0)
    }

    fn task(name: &str, due: Option<&str>) -> Task {
        let mut task = Task::new(name, Id::from("P"));
        task.due = due.map(|date| Due {
            date: date.into(),
            timezone: None,
            string: String::new(),
            lang: "en".into(),
            is_recurring: false,
            recurrence: None,
        });
        task
    }

    fn on() -> ReminderSettings {
        ReminderSettings { enabled: true, ..ReminderSettings::default() }
    }

    fn times(r: &[PlannedReminder]) -> Vec<NaiveDateTime> {
        r.iter().map(|r| r.at).collect()
    }

    #[test]
    fn disabled_plans_nothing() {
        let tasks = [task("a", Some("2026-10-19T17:00:00"))];
        assert!(plan(&tasks, &[], &ReminderSettings::default(), now()).is_empty());
    }

    #[test]
    fn timed_task_at_due_time() {
        let tasks = [task("Call mom", Some("2026-10-19T17:00:00"))];
        let r = plan(&tasks, &[], &on(), now());
        assert_eq!(times(&r), vec![at(2026, 10, 19, 17, 0)]);
        assert_eq!(r[0].title, "Call mom");
        assert_eq!(r[0].body, "Due at 5 PM");
        assert_eq!(r[0].task_id, Some(tasks[0].id.clone()));
    }

    #[test]
    fn offset_before_and_crossing_midnight() {
        let s = ReminderSettings { offset_minutes: 30, ..on() };
        let tasks = [task("a", Some("2026-10-19T17:00:00")), task("b", Some("2026-10-21T00:15:00"))];
        let r = plan(&tasks, &[], &s, now());
        assert_eq!(times(&r), vec![at(2026, 10, 19, 16, 30), at(2026, 10, 20, 23, 45)]);
        assert_eq!(r[0].body, "Due in 30 min (5 PM)");
    }

    #[test]
    fn date_only_uses_all_day_time_or_skips() {
        let tasks = [task("a", Some("2026-10-20"))];
        let r = plan(&tasks, &[], &on(), now());
        assert_eq!(times(&r), vec![at(2026, 10, 20, 9, 0)]);
        assert_eq!(r[0].body, "Due today", "fires on the due day");
        let none = ReminderSettings { all_day_time: None, ..on() };
        assert!(plan(&tasks, &[], &none, now()).is_empty());
    }

    #[test]
    fn offset_does_not_apply_to_date_only() {
        let s = ReminderSettings { offset_minutes: 60, ..on() };
        let r = plan(&[task("a", Some("2026-10-20"))], &[], &s, now());
        assert_eq!(times(&r), vec![at(2026, 10, 20, 9, 0)]);
    }

    #[test]
    fn skips_past_completed_deleted_and_undated() {
        let mut done = task("done", Some("2026-10-19T17:00:00"));
        done.checked = true;
        let mut gone = task("gone", Some("2026-10-19T17:00:00"));
        gone.is_deleted = true;
        let tasks = [
            task("past", Some("2026-10-19T07:00:00")),
            task("now exactly", Some("2026-10-19T08:00:00")),
            task("overdue day", Some("2026-10-18")),
            task("today's 9am already gone?", Some("2026-10-19")), // 09:00 > 08:00 → kept
            done,
            gone,
            task("undated", None),
        ];
        let r = plan(&tasks, &[], &on(), now());
        let names: Vec<&str> = r.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(names, vec!["today's 9am already gone?"]);
    }

    #[test]
    fn horizon_cap_and_sorting() {
        let tasks: Vec<Task> = (0..80)
            .map(|i| task(&format!("t{i}"), Some(&format!("2026-10-{:02}T{:02}:00:00", 20 + i % 10, 8 + i / 10))))
            .chain([task("far", Some("2026-11-20T10:00:00"))])
            .collect();
        let r = plan(&tasks, &[], &on(), now());
        assert_eq!(r.len(), MAX_REMINDERS);
        assert!(r.windows(2).all(|w| w[0].at <= w[1].at), "sorted");
        assert!(r.iter().all(|r| r.title != "far"), "beyond the 14-day horizon");
    }

    #[test]
    fn ids_are_stable_and_distinct() {
        let tasks = [task("a", Some("2026-10-19T17:00:00")), task("b", Some("2026-10-19T17:00:00"))];
        let first = plan(&tasks, &[], &on(), now());
        let again = plan(&tasks, &[], &on(), at(2026, 10, 19, 9, 0));
        assert_eq!(first[0].id, again[0].id);
        assert_ne!(first[0].id, first[1].id);
        assert!(first.iter().all(|r| r.id > 0));
    }

    #[test]
    fn daily_summary_counts_tasks_per_day() {
        let s = ReminderSettings { daily_summary: Some(t(7, 30)), all_day_time: None, ..on() };
        let tasks = [
            task("a", Some("2026-10-20")),
            task("b", Some("2026-10-20T17:00:00")),
            task("c", Some("2026-10-22")),
        ];
        let r = plan(&tasks, &[], &s, now());
        let summaries: Vec<(&NaiveDateTime, &str)> =
            r.iter().filter(|r| r.task_id.is_none()).map(|r| (&r.at, r.body.as_str())).collect();
        assert_eq!(
            summaries,
            vec![
                (&at(2026, 10, 20, 7, 30), "2 tasks due today"),
                (&at(2026, 10, 22, 7, 30), "1 task due today"),
            ],
            "no summary today (07:30 already passed) or on empty days"
        );
        assert_eq!(r.iter().find(|r| r.task_id.is_none()).unwrap().title, "Today");
    }
    #[test]
    fn explicit_relative_reminders() {
        let t = task("Call mom", Some("2026-10-20T17:00:00"));
        let rs = [Reminder::relative(t.id.clone(), 60), Reminder::relative(t.id.clone(), 24 * 60)];
        let s = ReminderSettings { all_day_time: None, ..on() };
        let r = plan(&[t.clone()], &rs, &s, now());
        assert_eq!(
            r.iter().map(|r| (r.at, r.body.as_str())).collect::<Vec<_>>(),
            vec![
                (at(2026, 10, 19, 17, 0), "Due tomorrow at 5 PM"),
                (at(2026, 10, 20, 16, 0), "Due in 1 hr (5 PM)"),
                (at(2026, 10, 20, 17, 0), "Due at 5 PM"),
            ],
            "explicit reminders on top of the global one"
        );
    }

    #[test]
    fn relative_on_date_only_uses_all_day_time() {
        let t = task("Bins", Some("2026-10-21"));
        let rs = [Reminder::relative(t.id.clone(), 24 * 60)];
        let r = plan(&[t.clone()], &rs, &on(), now());
        assert_eq!(times(&r), vec![at(2026, 10, 20, 9, 0), at(2026, 10, 21, 9, 0)]);
        assert_eq!(r[0].body, "Due tomorrow");
        let off = ReminderSettings { all_day_time: None, ..on() };
        assert!(plan(&[t], &rs, &off, now()).is_empty(), "no all-day time, no base to count back from");
    }

    #[test]
    fn absolute_reminders_fire_once_even_without_due() {
        let t = task("Water plants", None);
        let rs = [Reminder::absolute(t.id.clone(), at(2026, 10, 19, 18, 30)), Reminder::absolute(t.id.clone(), at(2026, 10, 18, 9, 0))];
        let r = plan(&[t], &rs, &on(), now());
        assert_eq!(r.len(), 1, "the past one is skipped");
        assert_eq!((r[0].at, r[0].body.as_str()), (at(2026, 10, 19, 18, 30), "Reminder"));
    }

    #[test]
    fn duplicates_fire_once_and_deleted_or_orphaned_are_ignored() {
        let t = task("a", Some("2026-10-19T17:00:00"));
        let mut gone = Reminder::relative(t.id.clone(), 30);
        gone.is_deleted = true;
        let rs = [
            Reminder::relative(t.id.clone(), 0), // same as the global rule
            gone,
            Reminder::relative(crate::ids::Id::from("no-such-task"), 0),
        ];
        assert_eq!(times(&plan(&[t], &rs, &on(), now())), vec![at(2026, 10, 19, 17, 0)]);
    }

    #[test]
    fn deadline_reminder_on_the_day() {
        let mut t = task("Report", None);
        t.deadline = Some("2026-10-22".into());
        let r = plan(&[t.clone()], &[], &on(), now());
        assert_eq!((r[0].at, r[0].body.as_str()), (at(2026, 10, 22, 9, 0), "Deadline today"));
        // Due the same day too: one notification that says both.
        t.due = task("x", Some("2026-10-22")).due;
        let r = plan(&[t], &[], &on(), now());
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].body, "Due today · Deadline today");
    }
}
