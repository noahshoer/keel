use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::recurrence::Recurrence;

/// Task priority. Stored API-style (4 = highest / UI "P1" … 1 = none).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Priority {
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

impl Default for Priority {
    fn default() -> Self {
        Priority::None
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
}

impl Task {
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
            due: Some(Due {
                date: date.clone(),
                timezone: None,
                string: date,
                lang: "en".to_string(),
                is_recurring: false,
                recurrence: None,
            }),
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
    use super::{Due, Priority, Task};
    use crate::ids::Id;

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
        let due = Due {
            date: "2026-10-19".to_string(),
            timezone: None,
            string: "today".to_string(),
            lang: "en".to_string(),
            is_recurring: false,
            recurrence: None,
        };
        let json = serde_json::to_string(&due).expect("serialize");
        let back: Due = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.timezone, None);
        assert_eq!(due, back);
    }
}
