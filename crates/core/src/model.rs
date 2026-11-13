use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::recurrence::Recurrence;

/// Task priority. Stored API-style (4 = highest / UI "P1" … 1 = none).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Priority {
    #[default]
    None,
    P3,
    P2,
    P1,
}

impl Priority {
    /// Map to the Todoist API integer (4 = highest … 1 = none).
    pub fn to_api(self) -> u8 {
        match self {
            Priority::None => 1,
            Priority::P3 => 2,
            Priority::P2 => 3,
            Priority::P1 => 4,
        }
    }

    /// Map from the API integer; anything out of 1..=4 clamps to `None`.
    pub fn from_api(value: u8) -> Priority {
        match value {
            4 => Priority::P1,
            3 => Priority::P2,
            2 => Priority::P3,
            _ => Priority::None,
        }
    }
}

/// How a project renders its tasks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ViewStyle {
    List,
    Board,
    Calendar,
}

/// A scheduled due date/time. `date` holds a Todoist-style date or datetime
/// string; `timezone` is `None` for a floating due.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Due {
    pub date: String,
    pub timezone: Option<String>,
    pub string: String,
    pub lang: String,
    pub is_recurring: bool,
    /// Structured recurrence rule (NL parsing lands in a later plan).
    /// Invariant: `is_recurring == recurrence.is_some()`.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub recurrence: Option<Recurrence>,
    /// The last day a repeat may fall on (`YYYY-MM-DD`), from "for 10 days"
    /// or "until nov 30"; completing the last occurrence finishes the task.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub until: Option<String>,
    /// The occurrence (`YYYY-MM-DD`) this due date stands in for, when just
    /// this one was moved off the repeat's days.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub moved_from: Option<String>,
}

impl Due {
    /// A one-off due on `date` (`YYYY-MM-DD` or a datetime), described by
    /// `string`: floating, in English, with no repeat.
    pub fn new(date: impl Into<String>, string: impl Into<String>) -> Due {
        Due {
            date: date.into(),
            timezone: None,
            string: string.into(),
            lang: "en".to_string(),
            is_recurring: false,
            recurrence: None,
            until: None,
            moved_from: None,
        }
    }

    /// [`Due::new`] repeating by `rule`, with no end.
    pub fn repeating(date: impl Into<String>, string: impl Into<String>, rule: Recurrence) -> Due {
        Due {
            is_recurring: true,
            recurrence: Some(rule),
            ..Due::new(date, string)
        }
    }

    /// The repeat's end ([`Due::until`]) as a date, when it is one.
    pub fn until_date(&self) -> Option<NaiveDate> {
        self.until.as_deref().and_then(parse_ymd)
    }
}

/// A `YYYY-MM-DD` date: the whole of `s`, untrimmed.
pub fn parse_ymd(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// The `YYYY-MM-DD` date in the first ten bytes of `s`, so a datetime
/// (`YYYY-MM-DDTHH:MM…`) gives its day. `None` when `s` is shorter.
pub fn parse_ymd_prefix(s: &str) -> Option<NaiveDate> {
    s.get(..10).and_then(parse_ymd)
}

/// Whether `name` is in `names`, ignoring ASCII case (how label names on a
/// task compare). Neither side is trimmed.
pub fn contains_name(names: &[String], name: &str) -> bool {
    names.iter().any(|n| n.eq_ignore_ascii_case(name))
}

/// `d` as `YYYY-MM-DD`, the format every stored date uses.
pub fn ymd(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum DurationUnit {
    Minute,
    Day,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Duration {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub amount: i64,
    pub unit: DurationUnit,
}

/// A task (item).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Task {
    pub id: Id,
    pub project_id: Id,
    pub section_id: Option<Id>,
    pub parent_id: Option<Id>,
    pub content: String,
    pub description: String,
    pub priority: Priority,
    pub due: Option<Due>,
    pub deadline: Option<String>,
    pub duration: Option<Duration>,
    pub labels: Vec<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub child_order: i64,
    pub order_key: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub day_order: i64,
    pub is_collapsed: bool,
    pub checked: bool,
    pub is_deleted: bool,
    pub added_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    // Streak fields (see design §7). Dormant unless streak_enabled.
    pub streak_enabled: bool,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub streak_current: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub streak_best: i64,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub streak_goal: Option<i64>,
    pub streak_last_period: Option<String>,
    /// What undoing the last completion of a repeating task puts back.
    #[serde(default)]
    pub last_completion: Option<LastCompletion>,
    /// When a plain sub-task completed with its repeating parent reopens: the
    /// parent's next date (`YYYY-MM-DD`).
    #[serde(default)]
    pub reopens_on: Option<String>,
    /// Completes itself once its sub-tasks are all done (issue #11).
    #[serde(default)]
    pub done_with_subtasks: bool,
}

/// What a repeating task's completion changed. It stays active, so this is all
/// there is to undo it with.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct LastCompletion {
    /// The date it moved to. Once the due date is anything else, it can't be undone.
    pub moved_to: String,
    pub date: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub streak_current: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub streak_best: i64,
    pub streak_last_period: Option<String>,
    /// The occurrence the due stood in for, when it had been moved.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub moved_from: Option<String>,
    /// The day it was completed (`YYYY-MM-DD`); missing on older saves.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub completed_on: Option<String>,
}

impl Task {
    /// Reopens the task if its `reopens_on` has come. True when it changed.
    pub fn reopen_if_due(&mut self, today: NaiveDate) -> bool {
        let date = self.reopens_on.as_deref().and_then(parse_ymd);
        if !date.is_some_and(|d| d <= today) {
            return false;
        }
        self.checked = false;
        self.completed_at = None;
        self.reopens_on = None;
        self.updated_at = Utc::now();
        true
    }

    /// Forgets the current run and the completion undo would put back (a new
    /// schedule starts the streak over); the best run stays.
    pub fn reset_streak(&mut self) {
        self.streak_current = 0;
        self.streak_last_period = None;
        self.last_completion = None;
    }

    /// A new task in `project_id` with sensible defaults (streaks off).
    pub fn new(content: impl Into<String>, project_id: Id) -> Task {
        let now = Utc::now();
        Task {
            id: Id::new(),
            project_id,
            section_id: None,
            parent_id: None,
            content: content.into(),
            description: String::new(),
            priority: Priority::None,
            due: None,
            deadline: None,
            duration: None,
            labels: Vec::new(),
            child_order: 0,
            order_key: None,
            day_order: 0,
            is_collapsed: false,
            checked: false,
            is_deleted: false,
            added_at: now,
            completed_at: None,
            updated_at: now,
            streak_enabled: false,
            streak_current: 0,
            streak_best: 0,
            streak_goal: None,
            streak_last_period: None,
            last_completion: None,
            reopens_on: None,
            done_with_subtasks: false,
        }
    }
}

/// A project (list of tasks).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Project {
    pub id: Id,
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
    pub parent_id: Option<Id>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub child_order: i64,
    pub order_key: Option<String>,
    pub view_style: ViewStyle,
    pub is_favorite: bool,
    pub is_collapsed: bool,
    pub is_archived: bool,
    pub is_inbox: bool,
    pub is_deleted: bool,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Project {
        let now = Utc::now();
        Project {
            id: Id::new(),
            name: name.into(),
            color: "charcoal".to_string(),
            icon: None,
            parent_id: None,
            child_order: 0,
            order_key: None,
            view_style: ViewStyle::List,
            is_favorite: false,
            is_collapsed: false,
            is_archived: false,
            is_inbox: false,
            is_deleted: false,
            added_at: now,
            updated_at: now,
        }
    }

    /// The special seeded Inbox project.
    pub fn inbox() -> Project {
        let mut p = Project::new("Inbox");
        p.is_inbox = true;
        p
    }
}

/// A section within a project.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Section {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub description: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub section_order: i64,
    pub order_key: Option<String>,
    pub is_collapsed: bool,
    pub is_archived: bool,
    pub is_deleted: bool,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A label.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Label {
    pub id: Id,
    pub name: String,
    pub color: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub item_order: i64,
    pub is_favorite: bool,
    pub is_deleted: bool,
}

impl Section {
    pub fn new(project_id: Id, name: impl Into<String>, section_order: i64) -> Section {
        let now = Utc::now();
        Section {
            id: Id::new(),
            project_id,
            name: name.into(),
            description: None,
            section_order,
            order_key: None,
            is_collapsed: false,
            is_archived: false,
            is_deleted: false,
            added_at: now,
            updated_at: now,
        }
    }
}

impl Filter {
    pub fn new(name: impl Into<String>, query: impl Into<String>, item_order: i64) -> Filter {
        Filter {
            id: Id::new(),
            name: name.into(),
            query: query.into(),
            color: "charcoal".to_string(),
            item_order,
            is_favorite: false,
            is_deleted: false,
        }
    }
}

impl Label {
    pub fn new(name: impl Into<String>) -> Label {
        Label {
            id: Id::new(),
            name: name.into(),
            color: "charcoal".to_string(),
            item_order: 0,
            is_favorite: false,
            is_deleted: false,
        }
    }
}

/// A saved filter (query over tasks).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Filter {
    pub id: Id,
    pub name: String,
    pub query: String,
    pub color: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub item_order: i64,
    pub is_favorite: bool,
    pub is_deleted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ReminderType {
    Relative,
    Absolute,
}

/// A time-based reminder attached to a task. (Location reminders deferred.)
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Reminder {
    pub id: Id,
    pub task_id: Id,
    pub kind: ReminderType,
    pub due: Option<Due>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub minute_offset: Option<i64>,
    pub is_deleted: bool,
}

impl Reminder {
    /// `minutes` before the task's due time (date-only tasks: before the
    /// all-day reminder time).
    pub fn relative(task_id: Id, minutes: i64) -> Reminder {
        Reminder {
            id: Id::new(),
            task_id,
            kind: ReminderType::Relative,
            due: None,
            minute_offset: Some(minutes.max(0)),
            is_deleted: false,
        }
    }

    /// At a fixed local date and time.
    pub fn absolute(task_id: Id, at: chrono::NaiveDateTime) -> Reminder {
        let date = at.format("%Y-%m-%dT%H:%M:00").to_string();
        Reminder {
            id: Id::new(),
            task_id,
            kind: ReminderType::Absolute,
            due: Some(Due::new(date.clone(), date)),
            minute_offset: None,
            is_deleted: false,
        }
    }

    /// The fixed time of an absolute reminder.
    pub fn at(&self) -> Option<chrono::NaiveDateTime> {
        let date = &self.due.as_ref()?.date;
        chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M"))
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{contains_name, parse_ymd, parse_ymd_prefix, ymd, Due, Priority, Task};
    use crate::ids::Id;
    use chrono::NaiveDate;

    #[test]
    fn contains_name_ignores_ascii_case_only() {
        let names = vec!["Home".to_string(), "Éclair".to_string()];
        assert!(contains_name(&names, "home"));
        assert!(contains_name(&names, "HOME"));
        assert!(!contains_name(&names, "home "), "no trimming");
        assert!(!contains_name(&names, "éclair"), "ASCII case only");
        assert!(!contains_name(&[], "home"));
    }

    #[test]
    fn ymd_round_trips_a_date() {
        let d = NaiveDate::from_ymd_opt(2026, 3, 7).unwrap();
        assert_eq!(ymd(d), "2026-03-07");
        assert_eq!(parse_ymd("2026-03-07"), Some(d));
    }

    #[test]
    fn parse_ymd_takes_the_whole_string() {
        assert_eq!(parse_ymd(""), None);
        assert_eq!(parse_ymd("2026-02-30"), None);
        assert_eq!(parse_ymd("2026-03-07T09:00:00"), None);
    }

    #[test]
    fn parse_ymd_prefix_reads_the_date_of_a_datetime() {
        let d = NaiveDate::from_ymd_opt(2026, 3, 7).unwrap();
        assert_eq!(parse_ymd_prefix("2026-03-07"), Some(d));
        assert_eq!(parse_ymd_prefix("2026-03-07T09:00:00"), Some(d));
        // Shorter than ten bytes is not a date, even one chrono would read.
        assert_eq!(parse_ymd_prefix("2026-3-7"), None);
        assert_eq!(parse_ymd_prefix(""), None);
    }

    #[test]
    fn due_constructors_fill_the_plain_fields() {
        use crate::recurrence::Recurrence;
        let plain = Due {
            date: "2026-10-19".to_string(),
            timezone: None,
            string: "today".to_string(),
            lang: "en".to_string(),
            is_recurring: false,
            recurrence: None,
            until: None,
            moved_from: None,
        };
        assert_eq!(Due::new("2026-10-19", "today"), plain);
        let repeating = Due {
            is_recurring: true,
            recurrence: Some(Recurrence::Daily),
            string: "every day".to_string(),
            ..plain
        };
        assert_eq!(
            Due::repeating("2026-10-19", "every day", Recurrence::Daily),
            repeating
        );
    }

    #[test]
    fn until_date_reads_the_repeat_end() {
        let mut due = Due::new("2026-10-19", "every day");
        assert_eq!(due.until_date(), None);
        due.until = Some("2026-11-30".to_string());
        assert_eq!(due.until_date(), NaiveDate::from_ymd_opt(2026, 11, 30));
        due.until = Some("soon".to_string());
        assert_eq!(due.until_date(), None);
    }

    #[test]
    fn id_new_is_unique_and_sortable() {
        let a = Id::new();
        // A 2ms gap guarantees a later timestamp, so the ULID sorts strictly
        // after — this pins the time-sortability property, not same-ms luck.
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = Id::new();
        assert_ne!(a, b, "two fresh ids must differ");
        assert!(b.as_str() >= a.as_str(), "later id should sort >= earlier");
        assert_eq!(a.as_str().len(), 26, "ULID string is 26 chars");
    }

    #[test]
    fn priority_api_roundtrip() {
        assert_eq!(Priority::P1.to_api(), 4);
        assert_eq!(Priority::None.to_api(), 1);
        assert_eq!(Priority::from_api(4), Priority::P1);
        assert_eq!(Priority::from_api(1), Priority::None);
        assert_eq!(Priority::from_api(0), Priority::None);
        assert_eq!(Priority::from_api(9), Priority::None);
    }

    #[test]
    fn reset_streak_keeps_only_the_best() {
        let mut task = Task::new("run", Id::new());
        task.streak_current = 4;
        task.streak_best = 9;
        task.streak_last_period = Some("2026-10-19".to_string());
        task.last_completion = Some(super::LastCompletion {
            moved_to: "2026-10-20".to_string(),
            date: "2026-10-19".to_string(),
            streak_current: 3,
            streak_best: 9,
            streak_last_period: None,
            moved_from: None,
            completed_on: None,
        });
        task.reset_streak();
        assert_eq!(task.streak_current, 0);
        assert_eq!(task.streak_best, 9);
        assert_eq!(task.streak_last_period, None);
        assert_eq!(task.last_completion, None);
    }

    #[test]
    fn task_serde_roundtrip() {
        let project = Id::new();
        let mut task = Task::new("write the plan", project.clone());
        task.priority = Priority::P1;
        task.labels = vec!["home".to_string(), "focus".to_string()];
        let json = serde_json::to_string(&task).expect("serialize");
        let back: Task = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(task, back);
    }

    #[test]
    fn floating_due_keeps_none_timezone() {
        let due = Due::new("2026-10-19", "today");
        let json = serde_json::to_string(&due).expect("serialize");
        let back: Due = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.timezone, None);
        assert_eq!(due, back);
    }
}
