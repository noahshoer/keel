//! The task editor's data rules, shared by every UI: the values it edits,
//! the chip texts and menus, how a picked value or a typed title changes
//! the values, and what adding or saving sends. The React editor (through
//! `core-wasm`) and the parked glass card run this code.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::calendar::{MONTHS_SHORT, WEEKDAYS_SHORT};
use crate::commands::{Action, DueEdit};
use crate::ids::Id;
use crate::model::{
    parse_ymd, ymd, Due, Label, Priority, Project, Reminder, ReminderType, Section, Task,
};
use crate::phrases::{phrase_name, same_name};
use crate::quickadd::{self, QuickAdd, Span, SpanKind};
use crate::recurrence::{self, Recurrence};
use crate::reminders::offset_text;
use crate::views::{self, DueBucket};

/// Everything the editor edits, as form-friendly values.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EditorValues {
    pub content: String,
    pub description: String,
    /// `YYYY-MM-DD` or empty.
    pub date: String,
    /// `HH:MM` or empty.
    pub time: String,
    /// "1".."4".
    pub priority: String,
    /// none | daily | weekdays | weekly | monthly | keep (an existing custom rule).
    pub repeat: String,
    pub project_id: Id,
    pub section_id: Option<Id>,
    pub labels: Vec<String>,
    /// Only used when adding a sub-task.
    pub parent_id: Option<Id>,
    /// `YYYY-MM-DD` or empty.
    pub deadline: String,
    pub reminders: Vec<ReminderDraft>,
    /// Byte ranges of `content` turned back into plain text (backspace
    /// right after a highlight); quick add leaves them alone.
    pub plain: Vec<(usize, usize)>,
    /// Track a streak, from the Streak pill. Only used when adding, and only
    /// with a repeat.
    #[serde(default)]
    pub streak: bool,
    /// Its goal, carried over from a typed "streak 30" when a pick takes the
    /// phrase out of the title.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub streak_goal: Option<i64>,
    /// The repeat's last day, `YYYY-MM-DD` or empty (from "for 10 days" /
    /// "until nov 30").
    #[serde(default)]
    pub until: String,
    /// The occurrence (`YYYY-MM-DD`) a moved date stands in for, or empty.
    #[serde(default)]
    pub moved_from: String,
    /// The rule typed into the Repeat menu, while `repeat` is "custom".
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub rule: Option<Recurrence>,
}

impl EditorValues {
    pub fn blank(
        project_id: Id,
        section_id: Option<Id>,
        date: String,
        labels: Vec<String>,
    ) -> Self {
        EditorValues {
            content: String::new(),
            description: String::new(),
            date,
            time: String::new(),
            priority: "4".to_string(),
            repeat: "none".to_string(),
            project_id,
            section_id,
            labels,
            parent_id: None,
            deadline: String::new(),
            reminders: Vec::new(),
            plain: Vec::new(),
            streak: false,
            streak_goal: None,
            until: String::new(),
            moved_from: String::new(),
            rule: None,
        }
    }

    pub fn from_task(t: &Task) -> Self {
        let rule = t.due.as_ref().and_then(|d| d.recurrence.clone());
        EditorValues {
            content: t.content.clone(),
            description: t.description.clone(),
            date: t
                .due
                .as_ref()
                .and_then(|d| d.date.get(..10))
                .unwrap_or("")
                .to_string(),
            time: t
                .due
                .as_ref()
                .and_then(|d| d.date.get(11..16))
                .unwrap_or("")
                .to_string(),
            priority: (5 - t.priority.to_api()).to_string(),
            // The task's own rule, not a preset: presets follow the date, and
            // a new date only moves one occurrence.
            repeat: if rule.is_some() { "keep" } else { "none" }.to_string(),
            project_id: t.project_id.clone(),
            section_id: t.section_id.clone(),
            labels: t.labels.clone(),
            parent_id: t.parent_id.clone(),
            deadline: t.deadline.clone().unwrap_or_default(),
            reminders: Vec::new(),
            plain: Vec::new(),
            streak: false,
            streak_goal: None,
            until: t
                .due
                .as_ref()
                .and_then(|d| d.until.clone())
                .unwrap_or_default(),
            moved_from: t
                .due
                .as_ref()
                .and_then(|d| d.moved_from.clone())
                .unwrap_or_default(),
            rule: None,
        }
    }

    pub fn priority(&self) -> Priority {
        match self.priority.as_str() {
            "1" => Priority::P1,
            "2" => Priority::P2,
            "3" => Priority::P3,
            _ => Priority::None,
        }
    }

    /// `project` or `project|section`, for the location select.
    pub fn location(&self) -> String {
        match &self.section_id {
            Some(s) => format!("{}|{}", self.project_id, s),
            None => self.project_id.to_string(),
        }
    }

    /// The `Due.date` string for the chosen date/time ("" = no due). A time
    /// without a date means today.
    pub fn due_string(&self, today: NaiveDate) -> String {
        match (self.date.is_empty(), self.time.is_empty()) {
            (true, true) => String::new(),
            (false, true) => self.date.clone(),
            (true, false) => format!("{}T{}:00", ymd(today), self.time),
            (false, false) => format!("{}T{}:00", self.date, self.time),
        }
    }
}

/// A reminder being edited: saved ones have an `id`; new ones don't yet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ReminderDraft {
    pub id: Option<Id>,
    /// Minutes before the due time (relative).
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub minute_offset: Option<i64>,
    /// `YYYY-MM-DDTHH:MM` (absolute).
    pub at: Option<String>,
}

impl ReminderDraft {
    pub fn from_reminder(r: &Reminder) -> Self {
        let relative = r.kind == ReminderType::Relative;
        ReminderDraft {
            id: Some(r.id.clone()),
            minute_offset: if relative { r.minute_offset } else { None },
            at: if relative {
                None
            } else {
                r.at().map(|a| a.format("%Y-%m-%dT%H:%M").to_string())
            },
        }
    }

    /// "At time of task", "30 min before", "Tuesday 9 AM".
    pub fn label(&self, today: NaiveDate) -> String {
        if let Some(m) = self.minute_offset {
            return offset_text(m);
        }
        match self
            .at
            .as_deref()
            .and_then(|a| chrono::NaiveDateTime::parse_from_str(a, "%Y-%m-%dT%H:%M").ok())
        {
            Some(at) => format!(
                "{} {}",
                views::due_label(at.date(), today),
                views::time_label(at.time())
            ),
            None => "Reminder".to_string(),
        }
    }
}

/// Relative reminder choices, in minutes before the due time.
pub const REMINDER_PRESETS: [i64; 5] = [0, 10, 30, 60, 1440];

/// The presets with their text ("At time of task", "10 min before", …).
pub fn reminder_presets() -> Vec<(i64, String)> {
    REMINDER_PRESETS
        .iter()
        .map(|&m| (m, offset_text(m)))
        .collect()
}

/// A custom reminder typed in plain words ("tomorrow 9am") as
/// `YYYY-MM-DDTHH:MM`; `None` without both a date and a time. Parsed as if
/// after a title ("x …"): quick add applies nothing to a phrase that would
/// leave the title empty.
pub fn custom_reminder_at(text: &str, today: NaiveDate) -> Option<String> {
    let q = quickadd::parse(&format!("x {text}"), today, &[], &[]);
    match (q.date, q.time) {
        (Some(d), Some(t)) => Some(d.and_time(t).format("%Y-%m-%dT%H:%M").to_string()),
        _ => None,
    }
}

/// A value chosen in a picker, in [`EditorValues`]' formats. (TypeScript
/// calls it `EditorPick`: `Pick` is a built-in type there.)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, rename = "EditorPick"))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Pick {
    Date {
        date: String,
        time: String,
    },
    Priority {
        value: String,
    },
    Labels {
        names: Vec<String>,
    },
    Reminders {
        drafts: Vec<ReminderDraft>,
    },
    Deadline {
        date: String,
    },
    Repeat {
        value: String,
    },
    /// A repeat typed into the Repeat menu ("every 3 tuesdays").
    RepeatText {
        text: String,
    },
    Location {
        project_id: Id,
        section_id: Option<Id>,
    },
    Streak {
        on: bool,
        /// Its goal, from the pill's popover. Ignored unless above zero.
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        goal: Option<i64>,
    },
}

/// Only the fields that changed; `None` leaves a field as it is.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TaskPatch {
    pub content: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub project_id: Option<Id>,
    pub labels: Option<Vec<String>>,
    pub due: Option<DueEdit>,
    /// `Some("")` clears the deadline.
    pub deadline: Option<String>,
}

impl TaskPatch {
    pub fn is_empty(&self) -> bool {
        self.content.is_none()
            && self.description.is_none()
            && self.priority.is_none()
            && self.project_id.is_none()
            && self.labels.is_none()
            && self.due.is_none()
            && self.deadline.is_none()
    }

    /// The `TaskUpdate` that applies this patch to `id`.
    pub fn into_action(self, id: Id) -> Action {
        Action::TaskUpdate {
            id,
            content: self.content,
            description: self.description,
            priority: self.priority,
            project_id: self.project_id,
            labels: self.labels,
            is_collapsed: None,
            due: self.due,
            deadline: self.deadline,
        }
    }
}

/// Fields from the add-task editor.
#[derive(Clone, Debug, PartialEq)]
pub struct NewTask {
    pub content: String,
    pub description: String,
    pub project_id: Id,
    /// `YYYY-MM-DD`, or empty for no date.
    pub date: String,
    pub priority: Priority,
    pub recurrence: Option<Recurrence>,
    pub labels: Vec<String>,
    pub section_id: Option<Id>,
    /// Set when adding a sub-task.
    pub parent_id: Option<Id>,
    /// `YYYY-MM-DD`, or empty for no deadline.
    pub deadline: String,
    /// The repeat's last day, `YYYY-MM-DD`, or empty.
    pub until: String,
}

impl NewTask {
    /// The `TaskAdd` that creates this task.
    pub fn into_action(self) -> Action {
        let due = (!self.date.is_empty()).then(|| Due {
            until: (!self.until.is_empty()).then(|| self.until.clone()),
            ..Due::new(self.date.clone(), self.date.clone())
        });
        let description =
            (!self.description.trim().is_empty()).then(|| self.description.trim().to_string());
        Action::TaskAdd {
            content: self.content,
            project_id: self.project_id,
            description,
            priority: Some(self.priority),
            due,
            labels: (!self.labels.is_empty()).then_some(self.labels),
            parent_id: self.parent_id,
            section_id: self.section_id,
            recurrence: self.recurrence,
            deadline: (!self.deadline.is_empty()).then_some(self.deadline),
        }
    }
}

/// Which repeat option represents an existing rule.
pub fn repeat_choice(rule: &Option<Recurrence>) -> &'static str {
    match rule {
        None => "none",
        Some(Recurrence::Daily) => "daily",
        Some(Recurrence::Weekly(days)) if days == &vec![0, 1, 2, 3, 4] => "weekdays",
        Some(Recurrence::Weekly(days)) if days.len() == 1 => "weekly",
        Some(Recurrence::MonthlyDay(_)) => "monthly",
        Some(_) => "keep",
    }
}

/// The rule for a repeat choice; "Every week/month" anchor on `date` (or today).
pub fn recurrence_for(
    choice: &str,
    date: &str,
    existing: Option<Recurrence>,
    today: NaiveDate,
) -> Option<Recurrence> {
    let anchor = parse_ymd(date.get(..10).unwrap_or(date)).unwrap_or(today);
    match choice {
        "daily" => Some(Recurrence::Daily),
        "weekdays" => Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])),
        "weekly" => Some(Recurrence::Weekly(vec![
            anchor.weekday().num_days_from_monday() as u8,
        ])),
        "monthly" => Some(Recurrence::MonthlyDay(anchor.day() as u8)),
        "keep" => existing,
        _ => None,
    }
}

/// The rule the values repeat by: the typed one, or the chosen option's.
fn rule_of(
    v: &EditorValues,
    date: &str,
    existing: Option<Recurrence>,
    today: NaiveDate,
) -> Option<Recurrence> {
    if v.repeat == "custom" {
        v.rule.clone()
    } else {
        recurrence_for(&v.repeat, date, existing, today)
    }
}

fn same_repeat(before: &EditorValues, after: &EditorValues) -> bool {
    before.repeat == after.repeat && before.rule == after.rule && before.until == after.until
}

/// Whether saving `after` moves one occurrence of the repeating task: a new
/// day under the same repeat. A streak can't carry over a move.
pub fn moves_occurrence(
    before: &EditorValues,
    after: &EditorValues,
    existing: Option<&Recurrence>,
) -> bool {
    existing.is_some()
        && before.repeat != "none"
        && same_repeat(before, after)
        && before.date != after.date
}

/// What the Repeat menu's field reads, live.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RepeatReading {
    pub rule: Recurrence,
    /// "Every 3 weeks on Tue until Dec 31".
    pub text: String,
    /// The first occurrence, `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM` when a time was typed, or empty.
    pub time: String,
    /// The last day, `YYYY-MM-DD`, or empty.
    pub until: String,
}

/// Reads a typed repeat, anchored on `date` (today when it's empty or past).
/// `None` when the text isn't a repeat alone.
pub fn read_repeat(text: &str, date: &str, today: NaiveDate) -> Option<RepeatReading> {
    let start = parse_ymd(date.get(..10).unwrap_or(date)).map_or(today, |d| d.max(today));
    let q = quickadd::parse_repeat(text, today, start)?;
    let rule = q.recurrence?;
    let until = q.until.map(ymd).unwrap_or_default();
    Some(RepeatReading {
        text: with_end(repeat_text(&rule), Some(&until)),
        date: ymd(q.date.unwrap_or(start)),
        time: q
            .time
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_default(),
        until,
        rule,
    })
}

/// Human text for a repeat rule ("Every other week on Wed").
pub fn repeat_text(rule: &Recurrence) -> String {
    let text = rule.describe();
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

/// The Labels chip's text.
pub fn labels_text(labels: &[String]) -> String {
    if labels.is_empty() {
        "Labels".to_string()
    } else {
        labels.join(", ")
    }
}

/// The Priority chip's text ("4" is no priority).
pub fn priority_text(priority: &str) -> String {
    if priority == "4" {
        "Priority".to_string()
    } else {
        format!("P{priority}")
    }
}

/// The repeat choices as (value, label); week/month options name the day
/// they repeat on, and "keep" appears when the task has a custom rule.
pub fn repeat_options(
    date: &str,
    keep_label: Option<&str>,
    today: NaiveDate,
) -> Vec<(String, String)> {
    let anchor = parse_ymd(date).unwrap_or(today);
    let mut opts = vec![
        ("none".to_string(), "No repeat".to_string()),
        ("daily".to_string(), "Every day".to_string()),
        (
            "weekdays".to_string(),
            "Every weekday (Mon – Fri)".to_string(),
        ),
        (
            "weekly".to_string(),
            format!("Every week on {}", anchor.format("%A")),
        ),
        (
            "monthly".to_string(),
            format!("Every month on the {}", recurrence::ordinal(anchor.day())),
        ),
    ];
    if let Some(text) = keep_label {
        opts.push(("keep".to_string(), text.to_string()));
    }
    opts
}

/// The Repeat chip's text for the chosen option.
pub fn repeat_chip_text(repeat: &str, opts: &[(String, String)]) -> String {
    opts.iter()
        .find(|(value, _)| value == repeat && value != "none")
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| "Repeat".to_string())
}

/// A repeat's text with its end: "Every day until Oct 21".
fn with_end(text: String, until: Option<&str>) -> String {
    match until.and_then(parse_ymd) {
        Some(end) => format!("{text} until {}", end.format("%b %-d")),
        None => text,
    }
}

/// "Project" or "Project / Section" for the location chip.
pub fn location_text(
    project_id: &Id,
    section_id: Option<&Id>,
    projects: &[Project],
    sections: &[Section],
) -> String {
    let Some(p) = projects.iter().find(|p| &p.id == project_id) else {
        return "Inbox".to_string();
    };
    match section_id.and_then(|s| sections.iter().find(|x| &x.id == s && x.project_id == p.id)) {
        Some(s) => format!("{} / {}", p.name, s.name),
        None => p.name.clone(),
    }
}

/// What a location menu entry is (its icon).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum LocationKind {
    Inbox,
    Project,
    Section,
}

/// One entry of the project / section menu.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct LocationChoice {
    /// `project` or `project|section` ([`location_pick`] reads it).
    pub value: String,
    pub label: String,
    pub kind: LocationKind,
}

/// The project / section menu: each project, then its sections (nested).
pub fn location_choices(projects: &[Project], sections: &[Section]) -> Vec<LocationChoice> {
    let mut items = Vec::new();
    for p in projects {
        let kind = if p.is_inbox {
            LocationKind::Inbox
        } else {
            LocationKind::Project
        };
        items.push(LocationChoice {
            value: p.id.to_string(),
            label: p.name.clone(),
            kind,
        });
        for s in sections.iter().filter(|s| s.project_id == p.id) {
            items.push(LocationChoice {
                value: format!("{}|{}", p.id, s.id),
                label: s.name.clone(),
                kind: LocationKind::Section,
            });
        }
    }
    items
}

/// The location menu's choice as a pick.
pub fn location_pick(value: &str) -> Pick {
    match value.split_once('|') {
        Some((p, s)) => Pick::Location {
            project_id: Id::from(p),
            section_id: Some(Id::from(s)),
        },
        None => Pick::Location {
            project_id: Id::from(value),
            section_id: None,
        },
    }
}

/// Field changes between `before` and `after`, plus a move when the
/// project/section changed (moves are a separate command that also carries
/// the sub-tree).
pub fn changes(
    before: &EditorValues,
    after: &EditorValues,
    existing: Option<Recurrence>,
    today: NaiveDate,
) -> (TaskPatch, Option<(Id, Option<Id>)>) {
    let due_changed = before.date != after.date
        || before.time != after.time
        || before.repeat != after.repeat
        || before.until != after.until
        || before.rule != after.rule;
    let due = after.due_string(today);
    let patch = TaskPatch {
        content: (after.content.trim() != before.content).then(|| after.content.trim().to_string()),
        description: (after.description != before.description).then(|| after.description.clone()),
        priority: (after.priority != before.priority).then(|| after.priority()),
        project_id: None,
        labels: (after.labels != before.labels).then(|| after.labels.clone()),
        deadline: (after.deadline != before.deadline).then(|| after.deadline.clone()),
        due: due_changed.then(|| DueEdit {
            date: (!due.is_empty()).then(|| due.clone()),
            // Same repeat, new date: the task's own rule, never one
            // re-anchored on the new day (that's a move of one occurrence).
            recurrence: if same_repeat(before, after) && existing.is_some() {
                existing
            } else {
                rule_of(after, &due, existing, today)
            },
            until: (after.until != before.until).then(|| after.until.clone()),
        }),
    };
    let moved = (after.project_id != before.project_id || after.section_id != before.section_id)
        .then(|| (after.project_id.clone(), after.section_id.clone()));
    (patch, moved)
}

/// Parses the editor's title as quick add, leaving the kept-plain phrases.
pub fn parse_title(
    v: &EditorValues,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) -> QuickAdd {
    let mut qa = quickadd::parse_keeping(&v.content, today, projects, sections, &v.plain);
    // A bare /Section resolves within the Inbox; carry its project along.
    if qa.project_id.is_none() {
        if let Some(s) = qa
            .section_id
            .as_ref()
            .and_then(|id| sections.iter().find(|s| &s.id == id))
        {
            qa.project_id = Some(s.project_id.clone());
        }
    }
    qa
}

/// Whether the title has a name to save: with quick add, something besides
/// the recognized phrases ("This weekend" alone highlights but can't be
/// added yet).
pub fn has_name(
    v: &EditorValues,
    quick_add: bool,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) -> bool {
    let qa = quick_add.then(|| parse_title(v, projects, sections, today));
    title_has_name(v, qa.as_ref())
}

/// [`has_name`] given the title's quick-add parse (`None` without quick add).
fn title_has_name(v: &EditorValues, qa: Option<&QuickAdd>) -> bool {
    let name = qa.map_or(v.content.as_str(), |q| q.content.as_str());
    !name.trim().is_empty()
}

/// The typed phrases a pick replaces.
fn kinds_for(pick: &Pick) -> &'static [SpanKind] {
    match pick {
        Pick::Date { .. } => &[SpanKind::Date, SpanKind::Time],
        Pick::Priority { .. } => &[SpanKind::Priority],
        Pick::Labels { .. } => &[SpanKind::Label],
        Pick::Reminders { .. } => &[],
        Pick::Deadline { .. } => &[SpanKind::Deadline],
        // A typed "streak" goes with the typed repeat it needs (and carries over).
        Pick::Repeat { .. } | Pick::RepeatText { .. } => &[SpanKind::Repeat, SpanKind::Streak],
        Pick::Streak { .. } => &[SpanKind::Streak],
        Pick::Location { .. } => &[SpanKind::Project, SpanKind::Section],
    }
}

/// Applies a chip pick. With quick add, the typed phrase the pick replaces
/// comes out of the title (Todoist-style), so every editor saves the same
/// input the same way. A different day also replaces a typed repeat (it set
/// the date); a time for the same day keeps it.
pub fn apply_pick(
    v: &mut EditorValues,
    pick: &Pick,
    quick_add: bool,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) {
    // The title's own repeat, picked again, stays as it is.
    if matches!(pick, Pick::Repeat { value } if value == "typed") {
        return;
    }
    // Typed text that isn't a repeat changes nothing.
    let reading = match pick {
        Pick::RepeatText { text } => match read_repeat(text, &v.date, today) {
            Some(r) => Some(r),
            None => return,
        },
        _ => None,
    };
    let mut typed_streak = false;
    let mut typed_goal = None;
    if quick_add {
        let qa = parse_title(v, projects, sections, today);
        typed_streak = qa.streak;
        typed_goal = qa.streak_goal;
        let mut kinds = kinds_for(pick).to_vec();
        if let Pick::Date { date, .. } = pick {
            let shown = qa.date.map(ymd).unwrap_or_else(|| v.date.clone());
            if *date != shown {
                kinds.extend([SpanKind::Repeat, SpanKind::Streak]);
            }
        }
        // Through `with_title`, so un-highlighted ranges move with the text.
        *v = with_title(v, quickadd::strip_spans(&v.content, &qa.spans, &kinds));
    }
    match pick {
        Pick::Date { date, time } => (v.date, v.time) = (date.clone(), time.clone()),
        Pick::Priority { value } => v.priority = value.clone(),
        Pick::Labels { names } => v.labels = names.clone(),
        Pick::Reminders { drafts } => v.reminders = drafts.clone(),
        Pick::Deadline { date } => v.deadline = date.clone(),
        Pick::Repeat { value } => {
            // Another choice drops the end and any typed rule; the task's own keeps its end.
            if *value != v.repeat && value != "keep" {
                v.until = String::new();
            }
            v.repeat = value.clone();
            v.rule = None;
            // No repeat, no streak.
            v.streak = value != "none" && (v.streak || typed_streak);
            v.streak_goal = if v.streak {
                typed_goal.or(v.streak_goal)
            } else {
                None
            };
        }
        Pick::RepeatText { .. } => {
            if let Some(r) = reading {
                v.repeat = "custom".to_string();
                v.rule = Some(r.rule);
                v.date = r.date;
                if !r.time.is_empty() {
                    v.time = r.time;
                }
                v.until = r.until;
                v.streak = v.streak || typed_streak;
                v.streak_goal = if v.streak {
                    typed_goal.or(v.streak_goal)
                } else {
                    None
                };
            }
        }
        Pick::Streak { on, goal } => {
            v.streak = *on;
            v.streak_goal = if *on { goal.filter(|g| *g > 0) } else { None };
        }
        Pick::Location {
            project_id,
            section_id,
        } => (v.project_id, v.section_id) = (project_id.clone(), section_id.clone()),
    }
    // A repeat always has a date: today, by default.
    if v.repeat != "none" && v.date.is_empty() {
        v.date = ymd(today);
    }
}

/// Toggles one label from the labels menu: the typed @labels count as picked
/// (so taking them out of the title doesn't drop them), then it's applied
/// like a labels pick.
pub fn toggle_label(
    v: &mut EditorValues,
    name: &str,
    quick_add: bool,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) {
    let mut names = v.labels.clone();
    if quick_add {
        for l in parse_title(v, projects, sections, today).labels {
            if !names.iter().any(|n| n.eq_ignore_ascii_case(&l)) {
                names.push(l);
            }
        }
    }
    match names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
        Some(i) => {
            names.remove(i);
        }
        None => names.push(name.to_string()),
    }
    apply_pick(
        v,
        &Pick::Labels { names },
        quick_add,
        projects,
        sections,
        today,
    );
}

/// The values after the title is retyped: kept-plain phrases follow the
/// text, or are dropped when the edit touches them.
pub fn with_title(v: &EditorValues, text: String) -> EditorValues {
    EditorValues {
        plain: quickadd::shift_plain(&v.content, &text, &v.plain),
        content: text,
        ..v.clone()
    }
}

/// Turns the highlighted phrase ending at byte `end` back into plain text
/// (Todoist's backspace). False when no highlight ends there.
pub fn unhighlight(
    v: &mut EditorValues,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
    end: usize,
) -> bool {
    let qa = parse_title(v, projects, sections, today);
    match qa.spans.iter().find(|s| s.end == end) {
        Some(s) => {
            v.plain.push((s.start, s.end));
            true
        }
        None => false,
    }
}

/// The byte offset in `s` of a position given in UTF-16 units (a browser
/// caret); past the end gives `s.len()`.
pub fn byte_at_utf16(s: &str, at: usize) -> usize {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        if units >= at {
            return i;
        }
        units += c.len_utf16();
    }
    s.len()
}

/// The new task described by the editor values with the quick-add parse
/// applied on top (text wins over chips for the fields it mentions).
pub fn new_task_from(v: &EditorValues, qa: &QuickAdd, today: NaiveDate) -> NewTask {
    // A typed date wins; a picked time goes with it unless a time was typed.
    let date = match (qa.date, qa.time) {
        (Some(d), None) => match NaiveTime::parse_from_str(&v.time, "%H:%M") {
            Ok(t) => format!("{}T{}", ymd(d), t.format("%H:%M:00")),
            Err(_) => ymd(d),
        },
        _ => qa.due_string().unwrap_or_else(|| v.due_string(today)),
    };
    let recurrence = qa
        .recurrence
        .clone()
        .or_else(|| rule_of(v, &date, None, today));
    let mut labels = v.labels.clone();
    for l in &qa.labels {
        if !labels.iter().any(|x| x.eq_ignore_ascii_case(l)) {
            labels.push(l.clone());
        }
    }
    let (project_id, section_id) = match (&qa.project_id, &qa.section_id) {
        (Some(p), s) => (p.clone(), s.clone()),
        (None, _) => (v.project_id.clone(), v.section_id.clone()),
    };
    NewTask {
        content: qa.content.trim().to_string(),
        description: v.description.clone(),
        project_id,
        recurrence,
        date,
        priority: qa.priority.unwrap_or_else(|| v.priority()),
        labels,
        section_id,
        parent_id: v.parent_id.clone(),
        deadline: qa.deadline.map(ymd).unwrap_or_else(|| v.deadline.clone()),
        until: shown_until(v, qa),
    }
}

/// The repeat's end: a typed repeat brings its own (or none).
fn shown_until(v: &EditorValues, qa: &QuickAdd) -> String {
    match (&qa.recurrence, qa.until) {
        (Some(_), Some(end)) => ymd(end),
        (Some(_), None) => String::new(),
        (None, _) => v.until.clone(),
    }
}

/// The Date chip's text and due bucket (`None` when no date is set; a time
/// alone means today).
pub fn date_chip(
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
    today: NaiveDate,
) -> (String, Option<DueBucket>) {
    match (date, time) {
        (Some(d), Some(t)) => (
            format!("{} {}", views::due_label(d, today), views::time_label(t)),
            Some(views::due_bucket(d, today)),
        ),
        (Some(d), None) => (
            views::due_label(d, today),
            Some(views::due_bucket(d, today)),
        ),
        (None, Some(t)) => (
            format!("Today {}", views::time_label(t)),
            Some(DueBucket::Today),
        ),
        (None, None) => ("Date".to_string(), None),
    }
}

/// The Deadline chip's text.
pub fn deadline_text(deadline: Option<NaiveDate>, today: NaiveDate) -> String {
    deadline
        .map(|d| views::due_label(d, today))
        .unwrap_or_else(|| "Deadline".to_string())
}

/// The Reminders chip's text.
pub fn reminders_text(reminders: &[ReminderDraft], today: NaiveDate) -> String {
    match reminders.len() {
        0 => "Reminders".to_string(),
        1 => reminders[0].label(today),
        n => format!("{n} reminders"),
    }
}

/// The date picker's shortcuts: (label, hint, date).
pub fn quick_dates(today: NaiveDate) -> Vec<(&'static str, String, NaiveDate)> {
    let saturday =
        today + Duration::days(((5 - today.weekday().num_days_from_monday() as i64) + 7) % 7);
    let next_monday = today + Duration::days(7 - today.weekday().num_days_from_monday() as i64);
    let tomorrow = today + Duration::days(1);
    vec![
        (
            "Today",
            WEEKDAYS_SHORT[today.weekday().num_days_from_monday() as usize].to_string(),
            today,
        ),
        (
            "Tomorrow",
            WEEKDAYS_SHORT[tomorrow.weekday().num_days_from_monday() as usize].to_string(),
            tomorrow,
        ),
        ("This weekend", WEEKDAYS_SHORT[5].to_string(), saturday),
        (
            "Next week",
            format!(
                "Mon {} {}",
                next_monday.day(),
                MONTHS_SHORT[next_monday.month0() as usize]
            ),
            next_monday,
        ),
    ]
}

/// One stretch of the title: plain text, or a phrase quick add recognized.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TitleSegment {
    pub text: String,
    /// The phrase's kind; `None` for plain text.
    pub kind: Option<SpanKind>,
    /// A priority phrase's level (its last digit: "p2", "!!2"), 4 when it
    /// has none; 0 for anything else.
    pub level: u8,
    /// A date or time phrase's due bucket (the date chip's): its color.
    /// `None` for anything else.
    pub bucket: Option<DueBucket>,
    /// A deadline phrase falls on today or earlier (see [`views::deadline_urgent`]).
    pub urgent: bool,
    /// A `#project` or `@label` naming nothing yet: saving offers to create it.
    pub is_new: bool,
    /// A phrase the title can't resolve (set by a later parser step).
    pub unknown: bool,
}

/// The title cut into plain and recognized stretches, covering all of it
/// (overlapping or misaligned spans are skipped).
pub fn title_segments(value: &str, spans: &[Span]) -> Vec<TitleSegment> {
    let plain = |text: &str| TitleSegment {
        text: text.to_string(),
        kind: None,
        level: 0,
        bucket: None,
        urgent: false,
        is_new: false,
        unknown: false,
    };
    let mut segments = Vec::new();
    let mut at = 0;
    for s in spans.iter().filter(|s| {
        s.end <= value.len() && value.is_char_boundary(s.start) && value.is_char_boundary(s.end)
    }) {
        if s.start < at {
            continue;
        }
        if s.start > at {
            segments.push(plain(&value[at..s.start]));
        }
        let text = &value[s.start..s.end];
        let level = match s.kind {
            SpanKind::Priority => text
                .chars()
                .rev()
                .find_map(|c| c.to_digit(10))
                .filter(|d| (1..=3).contains(d))
                .unwrap_or(4) as u8,
            _ => 0,
        };
        segments.push(TitleSegment {
            text: text.to_string(),
            kind: Some(s.kind),
            level,
            bucket: None,
            urgent: false,
            is_new: false,
            unknown: false,
        });
        at = s.end;
    }
    if at < value.len() {
        segments.push(plain(&value[at..]));
    }
    segments
}

/// `v` with the parse applied on top, as saving does (see [`new_task_from`]):
/// what the editor's pills show. Text wins for the fields it mentions.
fn shown_values(v: &EditorValues, qa: &QuickAdd) -> EditorValues {
    let mut labels = v.labels.clone();
    for l in &qa.labels {
        if !labels.iter().any(|n| n.eq_ignore_ascii_case(l)) {
            labels.push(l.clone());
        }
    }
    let (project_id, section_id) = match (&qa.project_id, &qa.section_id) {
        (Some(p), s) => (p.clone(), s.clone()),
        (None, _) => (v.project_id.clone(), v.section_id.clone()),
    };
    EditorValues {
        date: qa.date.map(ymd).unwrap_or_else(|| v.date.clone()),
        time: qa
            .time
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_else(|| v.time.clone()),
        priority: qa
            .priority
            .map(|p| (5 - p.to_api()).to_string())
            .unwrap_or_else(|| v.priority.clone()),
        deadline: qa.deadline.map(ymd).unwrap_or_else(|| v.deadline.clone()),
        until: shown_until(v, qa),
        labels,
        project_id,
        section_id,
        ..v.clone()
    }
}

/// Everything the editor draws for its values, from one parse of the title.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EditorView {
    /// The title has a name to save (see [`has_name`]).
    pub can_submit: bool,
    pub segments: Vec<TitleSegment>,
    /// The values with what the title says applied: what the pills show.
    pub shown: EditorValues,
    /// The days it will repeat on: the typed rule, or the Repeat menu's.
    pub repeat_rule: Option<Recurrence>,
    /// The Streak pill shows: adding a task that repeats. `shown.streak` is
    /// whether it's on, and is never true without this.
    pub streak_offered: bool,
    /// Relative reminders count back from a due date (typed or picked).
    pub has_due: bool,
    /// The location menu's selected value.
    pub location: String,
    pub location_text: String,
    pub locations: Vec<LocationChoice>,
    pub priority_text: String,
    pub labels_text: String,
    pub repeat_options: Vec<(String, String)>,
    /// The menu's checked row: "typed" for a repeat typed in the title, or
    /// the chosen option.
    pub repeat_selected: String,
    pub repeat_chip_text: String,
    pub date_chip: (String, Option<DueBucket>),
    pub deadline_text: String,
    /// `None` without a deadline.
    pub deadline_urgent: Option<bool>,
    pub reminders_text: String,
    /// Each reminder's text, in order.
    pub reminder_labels: Vec<String>,
    /// The typed `#project` that names no project yet.
    pub new_project: Option<String>,
    /// The typed `@labels` that name no label yet, in title order (picked
    /// ones excluded).
    pub new_labels: Vec<String>,
}

/// What the editor shows for `v`. `quick_add` parses the title (adding);
/// `keep_label` names an existing custom rule and `existing` is that rule
/// (editing).
#[allow(clippy::too_many_arguments)]
pub fn editor_view(
    v: &EditorValues,
    quick_add: bool,
    keep_label: Option<&str>,
    existing: Option<Recurrence>,
    projects: &[Project],
    sections: &[Section],
    labels: &[Label],
    today: NaiveDate,
) -> EditorView {
    let qa = quick_add.then(|| parse_title(v, projects, sections, today));
    let can_submit = title_has_name(v, qa.as_ref());
    let spans = qa.as_ref().map(|q| q.spans.as_slice()).unwrap_or(&[]);
    let mut shown = qa
        .as_ref()
        .map(|q| shown_values(v, q))
        .unwrap_or_else(|| v.clone());
    let typed_repeat = qa.as_ref().and_then(|q| q.recurrence.clone());
    let repeat_rule = typed_repeat
        .clone()
        .or_else(|| rule_of(v, &shown.date, existing.clone(), today));
    let streak_offered = quick_add && repeat_rule.is_some();
    shown.streak = streak_offered && (v.streak || qa.as_ref().is_some_and(|q| q.streak));
    shown.streak_goal = if shown.streak {
        qa.as_ref().and_then(|q| q.streak_goal).or(v.streak_goal)
    } else {
        None
    };
    let has_due = !shown.date.is_empty() || !shown.time.is_empty();
    // The task's own rule names itself when the caller doesn't.
    let own_label = keep_label.map(str::to_string).or_else(|| {
        existing
            .as_ref()
            .filter(|_| v.repeat == "keep")
            .map(repeat_text)
    });
    let mut opts = repeat_options(&shown.date, own_label.as_deref(), today);
    // A preset that is the task's own rule would be the same row twice.
    if let Some(rule) = existing.as_ref().filter(|_| own_label.is_some()) {
        opts.retain(|(value, _)| {
            value == "keep"
                || recurrence_for(value, &shown.date, None, today).as_ref() != Some(rule)
        });
    }
    if let (Some(rule), "custom") = (&v.rule, v.repeat.as_str()) {
        opts.push(("custom".to_string(), repeat_text(rule)));
    }
    if let Some(rule) = &typed_repeat {
        opts.push(("typed".to_string(), repeat_text(rule)));
    }
    let repeat_selected = if typed_repeat.is_some() {
        "typed".to_string()
    } else {
        v.repeat.clone()
    };
    let date = parse_ymd(&shown.date);
    let time = NaiveTime::parse_from_str(&shown.time, "%H:%M").ok();
    let deadline = parse_ymd(&shown.deadline);
    let mut date_chip = date_chip(date, time, today);
    // A moved occurrence says which day it stands in for, while it's off the repeat.
    let moved = parse_ymd(&shown.moved_from);
    if let (Some(from), Some(day), Some(rule)) = (moved, date, &repeat_rule) {
        if let Some(note) = views::moved_text(from, day, rule) {
            date_chip.0 = format!("{} · {}", date_chip.0, note.replacen("Moved", "moved", 1));
        }
    }
    let deadline_urgent = deadline.map(|d| views::deadline_urgent(d, today));
    let mut segments = title_segments(&v.content, spans);
    for s in &mut segments {
        match s.kind {
            Some(SpanKind::Date | SpanKind::Time) => s.bucket = date_chip.1,
            Some(SpanKind::Deadline) => s.urgent = deadline_urgent == Some(true),
            _ => {}
        }
    }
    let new_project = qa.as_ref().and_then(|q| q.new_project.clone());
    for s in &mut segments {
        let Some(name) = phrase_name(&s.text) else {
            continue;
        };
        s.is_new = match s.kind {
            Some(SpanKind::Project) => new_project.as_deref().is_some_and(|p| same_name(p, &name)),
            // A picked label is created with the task, not asked about.
            Some(SpanKind::Label) => {
                !labels.iter().any(|l| same_name(&l.name, &name))
                    && !v.labels.iter().any(|n| same_name(n, &name))
            }
            _ => false,
        };
    }
    let new_labels = qa
        .as_ref()
        .map(|q| typed_new_labels(q, v, labels))
        .unwrap_or_default();
    let repeat_chip_text = with_end(
        typed_repeat
            .as_ref()
            .map(repeat_text)
            .unwrap_or_else(|| repeat_chip_text(&v.repeat, &opts)),
        repeat_rule.is_some().then_some(shown.until.as_str()),
    );
    EditorView {
        can_submit,
        segments,
        repeat_rule,
        streak_offered,
        has_due,
        location: shown.location(),
        location_text: location_text(
            &shown.project_id,
            shown.section_id.as_ref(),
            projects,
            sections,
        ),
        locations: location_choices(projects, sections),
        priority_text: priority_text(&shown.priority),
        labels_text: labels_text(&shown.labels),
        repeat_chip_text,
        repeat_options: opts,
        repeat_selected,
        date_chip,
        deadline_text: deadline_text(deadline, today),
        deadline_urgent,
        reminders_text: reminders_text(&v.reminders, today),
        reminder_labels: v.reminders.iter().map(|r| r.label(today)).collect(),
        new_project,
        new_labels,
        shown,
    }
}

/// The typed `@labels` no label has (any case), once each, in title order;
/// labels already picked are left out.
fn typed_new_labels(qa: &QuickAdd, v: &EditorValues, labels: &[Label]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in &qa.labels {
        let known = labels.iter().any(|l| same_name(&l.name, name));
        let picked = v.labels.iter().any(|n| same_name(n, name));
        if !known && !picked && !out.iter().any(|n| same_name(n, name)) {
            out.push(name.clone());
        }
    }
    out
}

/// What the save prompt lists: names typed with `#` / `@` that nothing has
/// yet. Empty when nothing is new.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct NewNames {
    pub projects: Vec<String>,
    pub labels: Vec<String>,
}

pub fn new_names(
    v: &EditorValues,
    projects: &[Project],
    sections: &[Section],
    labels: &[Label],
    today: NaiveDate,
) -> NewNames {
    let qa = parse_title(v, projects, sections, today);
    NewNames {
        projects: qa.new_project.iter().cloned().collect(),
        labels: typed_new_labels(&qa, v, labels),
    }
}

/// `v` with the typed `#name` phrases for `names.projects` and `@name`
/// phrases for `names.labels` (any case) left as plain text in the title.
/// Each list only touches its own kind of phrase. A `#` or `@` phrase that
/// only reads once those are text (a later `#name`, now the first) stays
/// text too: keeping a name never applies one the title didn't before.
pub fn keep_as_text(
    v: &EditorValues,
    names: &NewNames,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) -> EditorValues {
    let named = |s: &Span| matches!(s.kind, SpanKind::Project | SpanKind::Label);
    let applied: Vec<Span> = parse_title(v, projects, sections, today)
        .spans
        .into_iter()
        .filter(named)
        .collect();
    let before: Vec<(usize, usize)> = applied.iter().map(|s| (s.start, s.end)).collect();
    let mut v = v.clone();
    for s in &applied {
        let list = if s.kind == SpanKind::Project {
            &names.projects
        } else {
            &names.labels
        };
        let name = v.content.get(s.start..s.end).and_then(phrase_name);
        if name.is_some_and(|name| list.iter().any(|n| same_name(n, &name))) {
            v.plain.push((s.start, s.end));
        }
    }
    // Each pass plains at least one new range, so this ends.
    loop {
        let fresh: Vec<(usize, usize)> = parse_title(&v, projects, sections, today)
            .spans
            .iter()
            .filter(|s| {
                named(s)
                    && !before.contains(&(s.start, s.end))
                    && !v.plain.contains(&(s.start, s.end))
            })
            .map(|s| (s.start, s.end))
            .collect();
        if fresh.is_empty() {
            return v;
        }
        v.plain.extend(fresh);
    }
}

/// `v` moved to a just-created project (its section no longer applies).
pub fn with_project(v: &EditorValues, project_id: Id) -> EditorValues {
    EditorValues {
        project_id,
        section_id: None,
        ..v.clone()
    }
}

/// A `TaskAdd` moved to a just-created project; other actions are unchanged.
pub fn task_with_project(task: Action, project_id: Id) -> Action {
    match task {
        Action::TaskAdd {
            content,
            description,
            priority,
            due,
            labels,
            parent_id,
            recurrence,
            deadline,
            ..
        } => Action::TaskAdd {
            content,
            project_id,
            description,
            priority,
            due,
            labels,
            parent_id,
            section_id: None,
            recurrence,
            deadline,
        },
        other => other,
    }
}

/// `v` with every phrase its title says marked plain: a stored title's words
/// stay text until they're retyped.
pub fn with_stored_plain(
    v: &EditorValues,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) -> EditorValues {
    let spans = parse_title(v, projects, sections, today).spans;
    let mut v = v.clone();
    v.plain.extend(spans.iter().map(|s| (s.start, s.end)));
    v
}

/// An edited title applied, for saving it: the values with what the title
/// says on top (text wins, as in quick add) and the phrases cut out of the
/// name, plus the rule the task repeats by. `None` when the title has no name
/// besides its phrases.
pub fn apply_title(
    v: &EditorValues,
    existing: Option<Recurrence>,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) -> Option<(EditorValues, Option<Recurrence>)> {
    let qa = parse_title(v, projects, sections, today);
    let name = qa.content.trim();
    if name.is_empty() {
        return None;
    }
    let mut after = shown_values(v, &qa);
    after.content = name.to_string();
    after.plain = Vec::new();
    let rule = match qa.recurrence {
        Some(r) => {
            after.repeat = repeat_choice(&Some(r.clone())).to_string();
            Some(r)
        }
        None => existing,
    };
    Some((after, rule))
}

/// [`unhighlight`] at a browser caret (UTF-16 units).
pub fn unhighlight_at(
    v: &mut EditorValues,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
    caret: usize,
) -> bool {
    let end = byte_at_utf16(&v.content, caret);
    unhighlight(v, projects, sections, today, end)
}

/// The reminder changes as actions: removals first, then additions.
pub fn reminder_actions(
    task_id: &Id,
    before: &[ReminderDraft],
    after: &[ReminderDraft],
) -> Vec<Action> {
    let mut out = Vec::new();
    for old in before {
        if let Some(id) = &old.id {
            if !after.iter().any(|a| a.id.as_ref() == Some(id)) {
                out.push(Action::ReminderDelete { id: id.clone() });
            }
        }
    }
    for new in after.iter().filter(|a| a.id.is_none()) {
        out.push(Action::ReminderAdd {
            task_id: task_id.clone(),
            minute_offset: new.minute_offset,
            at: new.at.clone(),
        });
    }
    out
}

/// What adding the editor's task sends, in order: the labels it mentions
/// that don't exist yet, the task, then (once the task has its id) its
/// reminders through [`reminder_actions`].
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct AddPlan {
    /// The new project to create first (the task then moves into it, see
    /// [`task_with_project`]).
    pub project: Option<String>,
    pub labels: Vec<String>,
    pub task: Action,
    pub reminders: Vec<ReminderDraft>,
    /// Track a streak, once the task has its id. Never without a repeat.
    pub streak: bool,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub streak_goal: Option<i64>,
}

pub fn add_plan(
    v: &EditorValues,
    projects: &[Project],
    sections: &[Section],
    labels: &[Label],
    today: NaiveDate,
) -> AddPlan {
    let qa = parse_title(v, projects, sections, today);
    // Typed labels, and picked ones: ticking a label moves the typed
    // @labels into the picks (and out of the title).
    let mut new_labels: Vec<String> = Vec::new();
    for name in qa.labels.iter().chain(&v.labels) {
        let known = labels.iter().any(|l| l.name.eq_ignore_ascii_case(name));
        if !known && !new_labels.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            new_labels.push(name.clone());
        }
    }
    let task = new_task_from(v, &qa, today);
    let streak = (v.streak || qa.streak) && task.recurrence.is_some();
    let streak_goal = if streak {
        qa.streak_goal.or(v.streak_goal)
    } else {
        None
    };
    AddPlan {
        project: qa.new_project.clone(),
        labels: new_labels,
        task: task.into_action(),
        reminders: v.reminders.clone(),
        streak,
        streak_goal,
    }
}

/// What saving an edit sends, in order: the move (it carries the sub-tree),
/// the reminder changes, then the field changes (`None` when none changed).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EditPlan {
    pub moved: Option<Action>,
    pub reminders: Vec<Action>,
    pub update: Option<Action>,
}

pub fn edit_plan(
    id: &Id,
    before: &EditorValues,
    after: &EditorValues,
    existing: Option<Recurrence>,
    today: NaiveDate,
) -> EditPlan {
    let (patch, moved) = changes(before, after, existing, today);
    EditPlan {
        moved: moved.map(|(project_id, section_id)| Action::TaskMove {
            id: id.clone(),
            project_id,
            section_id,
        }),
        reminders: reminder_actions(id, &before.reminders, &after.reminders),
        update: (!patch.is_empty()).then(|| patch.into_action(id.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day19() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()
    }

    fn day30() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
    }

    fn typed(content: &str) -> EditorValues {
        let mut v = EditorValues::blank(Id::from("P-IN"), None, String::new(), Vec::new());
        v.content = content.into();
        v
    }

    #[test]
    fn edit_changes_leave_quick_add_text_alone() {
        let mut task = Task::new("Buy p1 batteries #2", Id::from("P-IN"));
        task.id = Id::from("T-1");
        let before = EditorValues::from_task(&task);
        let mut after = before.clone();
        after.priority = "1".to_string();
        let (patch, moved) = changes(&before, &after, None, day19());
        assert_eq!(patch.content, None);
        assert_eq!(patch.priority, Some(Priority::P1));
        assert_eq!(moved, None);
    }

    #[test]
    fn a_web_pick_replaces_the_typed_phrase_like_the_card() {
        let mut v = typed("call mom tomorrow");
        let pick = Pick::Date {
            date: "2026-10-23".into(),
            time: String::new(),
        };
        apply_pick(&mut v, &pick, true, &[], &[], day19());
        assert_eq!(v.content, "call mom");
        let task = new_task_from(&v, &parse_title(&v, &[], &[], day19()), day19());
        assert_eq!(task.date, "2026-10-23", "the pick is what's saved");
        // Editing doesn't parse the title, so nothing is stripped.
        let mut e = typed("call mom tomorrow");
        apply_pick(&mut e, &pick, false, &[], &[], day19());
        assert_eq!(e.content, "call mom tomorrow");
    }

    #[test]
    fn toggling_a_label_keeps_the_typed_ones() {
        let mut v = typed("x @home");
        toggle_label(&mut v, "work", true, &[], &[], day19());
        assert_eq!(v.content, "x");
        assert_eq!(v.labels, vec!["home".to_string(), "work".to_string()]);
        toggle_label(&mut v, "HOME", true, &[], &[], day19());
        assert_eq!(v.labels, vec!["work".to_string()]);
    }

    #[test]
    fn a_picked_time_is_saved_with_a_typed_date() {
        let mut v = typed("water plants every monday");
        v.date = "2026-10-19".into();
        v.time = "17:00".into();
        let task = new_task_from(&v, &parse_title(&v, &[], &[], day19()), day19());
        assert_eq!(task.date, "2026-10-19T17:00:00");
        assert!(task.recurrence.is_some(), "the repeat is kept");
    }

    #[test]
    fn custom_reminder_needs_a_time() {
        assert_eq!(
            custom_reminder_at("tomorrow 9am", day30()),
            Some("2026-10-31T09:00".to_string())
        );
        assert_eq!(custom_reminder_at("tomorrow", day30()), None);
    }

    #[test]
    fn location_choices_nest_sections() {
        let mut inbox = Project::new("Inbox");
        inbox.is_inbox = true;
        let home = Project::new("Home");
        let garden = Section::new(home.id.clone(), "Garden", 0);
        let items = location_choices(
            &[inbox.clone(), home.clone()],
            std::slice::from_ref(&garden),
        );
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].kind, LocationKind::Inbox);
        assert_eq!(
            items[1],
            LocationChoice {
                value: home.id.to_string(),
                label: "Home".into(),
                kind: LocationKind::Project
            }
        );
        assert_eq!(items[2].value, format!("{}|{}", home.id, garden.id));
        assert_eq!(items[2].kind, LocationKind::Section);
        assert_eq!(
            location_pick(&items[2].value),
            Pick::Location {
                project_id: home.id.clone(),
                section_id: Some(garden.id.clone())
            }
        );
    }

    #[test]
    fn date_chip_texts() {
        let t = NaiveTime::from_hms_opt(17, 0, 0);
        let tomorrow = NaiveDate::from_ymd_opt(2026, 10, 31);
        assert_eq!(
            date_chip(tomorrow, t, day30()),
            ("Tomorrow 5 PM".to_string(), Some(DueBucket::Tomorrow))
        );
        assert_eq!(
            date_chip(tomorrow, None, day30()),
            ("Tomorrow".to_string(), Some(DueBucket::Tomorrow))
        );
        assert_eq!(
            date_chip(None, t, day30()),
            ("Today 5 PM".to_string(), Some(DueBucket::Today))
        );
        assert_eq!(date_chip(None, None, day30()), ("Date".to_string(), None));
    }

    #[test]
    fn pick_keeps_its_camel_case_shape() {
        let p: Pick =
            serde_json::from_str(r#"{"type":"location","projectId":"P","sectionId":null}"#)
                .unwrap();
        assert_eq!(
            p,
            Pick::Location {
                project_id: Id::from("P"),
                section_id: None
            }
        );
        let d: ReminderDraft =
            serde_json::from_str(r#"{"id":null,"minuteOffset":30,"at":null}"#).unwrap();
        assert_eq!(d.minute_offset, Some(30));
    }
    fn kinds(view: &EditorView) -> Vec<Option<SpanKind>> {
        view.segments.iter().map(|s| s.kind).collect()
    }

    fn view_of(v: &EditorValues, quick_add: bool) -> EditorView {
        editor_view(v, quick_add, None, None, &[], &[], &[], day30())
    }

    #[test]
    fn segments_cover_the_title() {
        let v = typed("Call mom tomorrow p2");
        let qa = parse_title(&v, &[], &[], day30());
        let segs = title_segments(&v.content, &qa.spans);
        let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["Call mom ", "tomorrow", " ", "p2"]);
        assert_eq!(segs[1].kind, Some(SpanKind::Date));
        assert_eq!((segs[3].kind, segs[3].level), (Some(SpanKind::Priority), 2));
        assert_eq!((segs[0].kind, segs[0].level), (None, 0));
    }

    #[test]
    fn date_and_time_segments_carry_the_chips_bucket() {
        let view = view_of(&typed("Call mom tomorrow 5pm"), true);
        let buckets: Vec<_> = view
            .segments
            .iter()
            .filter(|s| s.kind.is_some())
            .map(|s| (s.kind, s.bucket))
            .collect();
        assert_eq!(
            buckets,
            [
                (Some(SpanKind::Date), Some(DueBucket::Tomorrow)),
                (Some(SpanKind::Time), Some(DueBucket::Tomorrow))
            ]
        );
        let view = view_of(&typed("Pay rent next week p1"), true);
        let date = view
            .segments
            .iter()
            .find(|s| s.kind == Some(SpanKind::Date))
            .unwrap();
        assert_eq!(date.bucket, Some(DueBucket::ThisWeek));
        // Other phrases and plain text have no bucket.
        assert!(view
            .segments
            .iter()
            .filter(|s| s.kind != Some(SpanKind::Date))
            .all(|s| s.bucket.is_none()));
    }

    #[test]
    fn deadline_segments_say_whether_they_are_urgent() {
        let urgent = view_of(&typed("File taxes {oct 30}"), true);
        let seg = |view: &EditorView| {
            view.segments
                .iter()
                .find(|s| s.kind == Some(SpanKind::Deadline))
                .map(|s| s.urgent)
        };
        assert_eq!(seg(&urgent), Some(true));
        let later = view_of(&typed("File taxes {nov 20}"), true);
        assert_eq!(seg(&later), Some(false));
    }

    #[test]
    fn segments_and_caret_survive_non_ascii() {
        let mut v = typed("Café tomorrow p1 🍰");
        assert!(kinds(&view_of(&v, true)).contains(&Some(SpanKind::Date)));
        let mut miss = v.clone();
        assert!(!unhighlight_at(&mut miss, &[], &[], day30(), 5));
        // "Café tomorrow" is 13 UTF-16 units (14 bytes).
        assert!(unhighlight_at(&mut v, &[], &[], day30(), 13));
        let after = view_of(&v, true);
        assert!(!kinds(&after).contains(&Some(SpanKind::Date)));
        assert!(kinds(&after).contains(&Some(SpanKind::Priority)));
        assert_eq!(
            after
                .segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<String>(),
            "Café tomorrow p1 🍰"
        );
    }

    #[test]
    fn pills_show_what_was_typed() {
        let mut home = Project::new("Home");
        home.id = Id::from("P-HOME");
        let v = typed("x tomorrow 5pm every day p1 #Home @phone {nov 2}");
        let view = editor_view(&v, true, None, None, &[home], &[], &[], day30());
        assert_eq!(view.date_chip.0, "Tomorrow 5 PM");
        assert_eq!(view.repeat_chip_text, "Every day");
        assert_eq!(view.priority_text, "P1");
        assert_eq!(view.location_text, "Home");
        assert_eq!(view.labels_text, "phone");
        assert_eq!(view.deadline_text, "Monday");
        assert_eq!(view.deadline_urgent, Some(false));
        assert_eq!(view.location, "P-HOME");
        assert_eq!(view.shown.priority, "1");
        assert_eq!(
            (view.shown.date.as_str(), view.shown.time.as_str()),
            ("2026-10-31", "17:00")
        );
        assert_eq!(view.shown.deadline, "2026-11-02");
    }

    #[test]
    fn typed_text_wins_over_a_pick_on_the_pills_as_it_does_when_saving() {
        let mut v = typed("x p2 tomorrow");
        v.priority = "3".into();
        v.date = "2026-11-06".into();
        v.time = "09:00".into();
        let view = editor_view(&v, true, None, None, &[], &[], &[], day30());
        assert_eq!(view.priority_text, "P2");
        assert_eq!(view.shown.date, "2026-10-31");
        assert_eq!(
            view.shown.time, "09:00",
            "a picked time goes with a typed date"
        );
        let saved = new_task_from(&v, &parse_title(&v, &[], &[], day30()), day30());
        assert_eq!(saved.priority.to_api(), 3);
        assert_eq!(saved.date, "2026-10-31T09:00:00");
    }

    #[test]
    fn picks_show_when_nothing_is_typed_and_come_back_when_the_phrase_goes() {
        let mut v = typed("x p2");
        v.priority = "3".into();
        assert_eq!(
            editor_view(&v, true, None, None, &[], &[], &[], day30()).priority_text,
            "P2"
        );
        v.content = "x".into();
        assert_eq!(
            editor_view(&v, true, None, None, &[], &[], &[], day30()).priority_text,
            "P3"
        );
    }

    #[test]
    fn editing_never_reads_the_title_for_the_pills() {
        let v = typed("x p2 tomorrow");
        let view = editor_view(&v, false, None, None, &[], &[], &[], day30());
        assert_eq!(view.priority_text, "Priority");
        assert_eq!(view.shown.date, v.date);
    }

    #[test]
    fn a_phrase_turned_back_into_text_leaves_the_pill() {
        let mut v = typed("x p2");
        assert!(unhighlight(&mut v, &[], &[], day30(), 4));
        assert_eq!(
            editor_view(&v, true, None, None, &[], &[], &[], day30()).priority_text,
            "Priority"
        );
    }

    #[test]
    fn editor_view_cannot_submit_a_phrase_alone() {
        for (text, quick_add) in [
            ("tomorrow", true),
            ("x tomorrow", true),
            ("tomorrow", false),
        ] {
            let v = typed(text);
            assert_eq!(
                has_name(&v, quick_add, &[], &[], day30()),
                view_of(&v, quick_add).can_submit,
                "{text}"
            );
        }
        assert!(!view_of(&typed("tomorrow"), true).can_submit);
        assert!(view_of(&typed("x tomorrow"), true).can_submit);
        assert!(
            view_of(&typed("tomorrow"), false).can_submit,
            "editing doesn't parse"
        );
        assert!(view_of(&typed("x tomorrow p1"), false)
            .segments
            .iter()
            .all(|s| s.kind.is_none()));
    }

    #[test]
    fn editor_view_chip_texts() {
        let mut v = typed("x");
        v.date = "2026-10-31".into();
        v.deadline = "2026-10-30".into();
        v.priority = "2".into();
        v.repeat = "weekly".into();
        v.reminders = vec![ReminderDraft {
            id: None,
            minute_offset: Some(30),
            at: None,
        }];
        let view = view_of(&v, true);
        assert_eq!(
            view.date_chip,
            ("Tomorrow".to_string(), Some(DueBucket::Tomorrow))
        );
        assert_eq!(
            (view.deadline_text.as_str(), view.deadline_urgent),
            ("Today", Some(true))
        );
        assert_eq!(view.priority_text, "P2");
        assert_eq!(view.repeat_chip_text, "Every week on Saturday");
        assert_eq!(view.repeat_rule, Some(Recurrence::Weekly(vec![5])));
        assert_eq!(view.reminders_text, "30 min before");
        assert!(view.has_due);
        assert_eq!(view.location_text, "Inbox");
    }

    #[test]
    fn shown_labels_merge_typed_and_picked() {
        let mut v = typed("x @Home @work");
        v.labels = vec!["home".into(), "errands".into()];
        let view = view_of(&v, true);
        assert_eq!(view.shown.labels, ["home", "errands", "work"]);
        assert_eq!(view.labels_text, "home, errands, work");
    }

    #[test]
    fn add_plan_creates_unknown_labels_once() {
        let known = vec![Label::new("Home")];
        let v = typed("x @home @Errands @errands");
        let plan = add_plan(&v, &[], &[], &known, day30());
        assert_eq!(plan.labels, ["Errands"]);
        match plan.task {
            Action::TaskAdd { labels, .. } => {
                let labels = labels.unwrap();
                assert!(labels.iter().any(|l| l.eq_ignore_ascii_case("home")));
                assert!(labels.iter().any(|l| l.eq_ignore_ascii_case("errands")));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn add_plan_creates_a_typed_label_moved_into_the_picks() {
        let known = vec![Label::new("home")];
        let mut v = typed("Buy milk @groceries");
        // Ticking "home" moves the typed @groceries into the picked labels.
        toggle_label(&mut v, "home", true, &[], &[], day30());
        assert_eq!(v.content, "Buy milk");
        let plan = add_plan(&v, &[], &[], &known, day30());
        assert_eq!(plan.labels, ["groceries"]);
    }

    #[test]
    fn add_plan_task_is_a_task_add() {
        let mut v = typed("Pay rent tomorrow p1 {nov 2}");
        v.description = "  online  ".into();
        v.parent_id = Some(Id::from("T-P"));
        v.reminders = vec![ReminderDraft {
            id: None,
            minute_offset: Some(0),
            at: None,
        }];
        let plan = add_plan(&v, &[], &[], &[], day30());
        assert_eq!(plan.reminders.len(), 1);
        match plan.task {
            Action::TaskAdd {
                content,
                description,
                priority,
                due,
                parent_id,
                deadline,
                project_id,
                ..
            } => {
                assert_eq!(content, "Pay rent");
                assert_eq!(description.as_deref(), Some("online"));
                assert_eq!(priority, Some(Priority::P1));
                assert_eq!(due.unwrap().date, "2026-10-31");
                assert_eq!(parent_id, Some(Id::from("T-P")));
                assert_eq!(deadline.as_deref(), Some("2026-11-02"));
                assert_eq!(project_id, Id::from("P-IN"));
            }
            other => panic!("{other:?}"),
        }
        let blank = add_plan(&typed("x"), &[], &[], &[], day30());
        assert!(matches!(
            blank.task,
            Action::TaskAdd {
                description: None,
                due: None,
                ..
            }
        ));
    }

    #[test]
    fn a_typed_streak_shows_and_is_planned() {
        let v = typed("Run every day streak");
        let view = editor_view(&v, true, None, None, &[], &[], &[], day30());
        assert!(view.streak_offered);
        assert!(view.shown.streak);
        assert!(view.can_submit);
        assert!(add_plan(&v, &[], &[], &[], day30()).streak);
    }

    #[test]
    fn a_typed_goal_shows_is_planned_and_survives_a_picked_repeat() {
        let mut v = typed("Run every day streak 30");
        assert_eq!(
            editor_view(&v, true, None, None, &[], &[], &[], day30())
                .shown
                .streak_goal,
            Some(30)
        );
        assert_eq!(add_plan(&v, &[], &[], &[], day30()).streak_goal, Some(30));

        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "weekly".into(),
            },
            true,
            &[],
            &[],
            day30(),
        );
        assert_eq!(v.content, "Run");
        assert_eq!(add_plan(&v, &[], &[], &[], day30()).streak_goal, Some(30));

        // Off clears it, and coming back on doesn't bring it back.
        apply_pick(
            &mut v,
            &Pick::Streak {
                on: false,
                goal: None,
            },
            true,
            &[],
            &[],
            day30(),
        );
        apply_pick(
            &mut v,
            &Pick::Streak {
                on: true,
                goal: None,
            },
            true,
            &[],
            &[],
            day30(),
        );
        assert_eq!(add_plan(&v, &[], &[], &[], day30()).streak_goal, None);
    }

    #[test]
    fn a_goal_from_the_popover_replaces_what_was_typed() {
        let mut v = typed("Run every day streak 30");
        let on = |goal| Pick::Streak { on: true, goal };

        apply_pick(&mut v, &on(Some(40)), true, &[], &[], day30());
        // The word goes, and the repeat it went with stays.
        assert_eq!(v.content, "Run every day");
        let view = editor_view(&v, true, None, None, &[], &[], &[], day30());
        assert!(view.shown.streak);
        assert_eq!(view.shown.streak_goal, Some(40));
        assert_eq!(add_plan(&v, &[], &[], &[], day30()).streak_goal, Some(40));

        // An emptied field, or a zero, is no goal; the streak stays on.
        for empty in [None, Some(0)] {
            apply_pick(&mut v, &on(empty), true, &[], &[], day30());
            let plan = add_plan(&v, &[], &[], &[], day30());
            assert!(plan.streak);
            assert_eq!(plan.streak_goal, None);
        }

        apply_pick(
            &mut v,
            &Pick::Streak {
                on: false,
                goal: Some(40),
            },
            true,
            &[],
            &[],
            day30(),
        );
        assert!(!add_plan(&v, &[], &[], &[], day30()).streak);
        assert_eq!(v.streak_goal, None);
    }

    #[test]
    fn the_streak_pill_shows_only_with_a_repeat() {
        let v = typed("Run");
        assert!(!editor_view(&v, true, None, None, &[], &[], &[], day30()).streak_offered);

        let mut v = typed("Run");
        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "daily".into(),
            },
            true,
            &[],
            &[],
            day30(),
        );
        let view = editor_view(&v, true, None, None, &[], &[], &[], day30());
        assert!(view.streak_offered);
        assert!(!view.shown.streak, "offered, not on");

        apply_pick(
            &mut v,
            &Pick::Streak {
                on: true,
                goal: None,
            },
            true,
            &[],
            &[],
            day30(),
        );
        assert!(
            editor_view(&v, true, None, None, &[], &[], &[], day30())
                .shown
                .streak
        );
        assert!(add_plan(&v, &[], &[], &[], day30()).streak);

        // Not when editing: the task view has its own.
        assert!(!editor_view(&v, false, None, None, &[], &[], &[], day30()).streak_offered);
    }

    #[test]
    fn removing_the_repeat_removes_the_streak() {
        let mut v = typed("Run");
        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "daily".into(),
            },
            true,
            &[],
            &[],
            day30(),
        );
        apply_pick(
            &mut v,
            &Pick::Streak {
                on: true,
                goal: None,
            },
            true,
            &[],
            &[],
            day30(),
        );
        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "none".into(),
            },
            true,
            &[],
            &[],
            day30(),
        );
        assert!(!v.streak);
        assert!(!editor_view(&v, true, None, None, &[], &[], &[], day30()).streak_offered);
        assert!(!add_plan(&v, &[], &[], &[], day30()).streak);
    }

    #[test]
    fn a_streak_never_saves_without_a_repeat() {
        // On from the pill, then the typed repeat is deleted from the title.
        let mut v = typed("Run");
        v.streak = true;
        assert!(
            !editor_view(&v, true, None, None, &[], &[], &[], day30())
                .shown
                .streak
        );
        assert!(!add_plan(&v, &[], &[], &[], day30()).streak);
    }

    #[test]
    fn a_picked_repeat_keeps_a_typed_streak_and_takes_its_word() {
        let mut v = typed("Run every day streak");
        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "weekly".into(),
            },
            true,
            &[],
            &[],
            day30(),
        );
        assert_eq!(v.content, "Run", "no stray word left in the title");
        assert!(v.streak);
        assert!(add_plan(&v, &[], &[], &[], day30()).streak);
    }

    #[test]
    fn turning_a_typed_streak_off_takes_its_word_and_keeps_the_repeat() {
        let mut v = typed("Run every day streak");
        apply_pick(
            &mut v,
            &Pick::Streak {
                on: false,
                goal: None,
            },
            true,
            &[],
            &[],
            day30(),
        );
        assert_eq!(v.content, "Run every day");
        let view = editor_view(&v, true, None, None, &[], &[], &[], day30());
        assert!(view.streak_offered);
        assert!(!view.shown.streak);
    }

    #[test]
    fn a_new_day_takes_a_typed_repeat_and_its_streak() {
        let mut v = typed("Run every day streak");
        apply_pick(
            &mut v,
            &Pick::Date {
                date: "2026-11-05".into(),
                time: String::new(),
            },
            true,
            &[],
            &[],
            day30(),
        );
        assert_eq!(v.content, "Run");
        assert!(!add_plan(&v, &[], &[], &[], day30()).streak);
    }

    #[test]
    fn a_repeat_always_has_a_date_today_by_default() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut v = EditorValues::blank(Id::from("P"), None, String::new(), vec![]);
        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "weekly".into(),
            },
            false,
            &[],
            &[],
            today,
        );
        assert_eq!(v.date, "2026-10-30");
        // Clearing the date while it repeats goes back to today.
        v.date = "2026-11-05".into();
        apply_pick(
            &mut v,
            &Pick::Date {
                date: String::new(),
                time: String::new(),
            },
            false,
            &[],
            &[],
            today,
        );
        assert_eq!(
            (v.date.as_str(), v.repeat.as_str()),
            ("2026-10-30", "weekly")
        );
        // An existing date stays, and no repeat means no date is fine.
        v.date = "2026-11-05".into();
        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "daily".into(),
            },
            false,
            &[],
            &[],
            today,
        );
        assert_eq!(v.date, "2026-11-05");
        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "none".into(),
            },
            false,
            &[],
            &[],
            today,
        );
        apply_pick(
            &mut v,
            &Pick::Date {
                date: String::new(),
                time: String::new(),
            },
            false,
            &[],
            &[],
            today,
        );
        assert_eq!(v.date, "");
    }

    #[test]
    fn edit_plan_orders_move_reminders_update() {
        let id = Id::from("T-1");
        let mut before = typed("x");
        before.reminders = vec![ReminderDraft {
            id: Some(Id::from("R-1")),
            minute_offset: Some(10),
            at: None,
        }];
        let mut after = before.clone();
        after.section_id = Some(Id::from("S-1"));
        after.priority = "1".into();
        after.reminders = vec![ReminderDraft {
            id: None,
            minute_offset: Some(60),
            at: None,
        }];
        let plan = edit_plan(&id, &before, &after, None, day30());
        assert_eq!(
            plan.moved,
            Some(Action::TaskMove {
                id: id.clone(),
                project_id: Id::from("P-IN"),
                section_id: Some(Id::from("S-1"))
            })
        );
        assert_eq!(
            plan.reminders,
            [
                Action::ReminderDelete {
                    id: Id::from("R-1")
                },
                Action::ReminderAdd {
                    task_id: id.clone(),
                    minute_offset: Some(60),
                    at: None
                },
            ]
        );
        match plan.update {
            Some(Action::TaskUpdate {
                priority,
                content,
                labels,
                due,
                ..
            }) => {
                assert_eq!(priority, Some(Priority::P1));
                assert_eq!((content, labels, due), (None, None, None));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn edit_plan_without_changes_is_empty() {
        let v = typed("x");
        let plan = edit_plan(&Id::from("T-1"), &v, &v, None, day30());
        assert_eq!(
            plan,
            EditPlan {
                moved: None,
                reminders: Vec::new(),
                update: None
            }
        );
    }

    #[test]
    fn an_edited_title_applies_what_it_says() {
        let mut v = typed("Call mom tomorrow p1");
        v.date = "2026-10-23".into();
        v.priority = "4".into();
        let (after, rule) = apply_title(&v, None, &[], &[], day19()).unwrap();
        assert_eq!(
            (
                after.content.as_str(),
                after.date.as_str(),
                after.priority.as_str()
            ),
            ("Call mom", "2026-10-20", "1")
        );
        assert!(after.plain.is_empty());
        assert_eq!(rule, None);
    }

    #[test]
    fn a_phrase_backspaced_to_plain_leaves_the_stored_values() {
        let mut v = typed("Call mom fri");
        v.date = "2026-10-23".into();
        assert_eq!(
            parse_title(&v, &[], &[], day19()).date,
            Some(NaiveDate::from_ymd_opt(2026, 10, 23).unwrap())
        );
        v.date = "2026-11-02".into();
        assert!(unhighlight(&mut v, &[], &[], day19(), "Call mom fri".len()));
        let (after, _) = apply_title(&v, None, &[], &[], day19()).unwrap();
        assert_eq!(
            (after.content.as_str(), after.date.as_str()),
            ("Call mom fri", "2026-11-02")
        );
    }

    #[test]
    fn phrases_already_in_a_stored_title_stay_text_until_retyped() {
        let v = with_stored_plain(&typed("Review Friday report"), &[], &[], day19());
        let (after, _) = apply_title(&v, None, &[], &[], day19()).unwrap();
        assert_eq!(
            (after.content.as_str(), after.date.as_str()),
            ("Review Friday report", "")
        );
        // Editing elsewhere in the title keeps the phrase plain.
        let v = with_title(&v, "Review the Friday report".into());
        let (after, _) = apply_title(&v, None, &[], &[], day19()).unwrap();
        assert_eq!(after.date, "");
        // Retyping the phrase itself parses it.
        let v = with_title(&v, "Review the Saturday report".into());
        let (after, _) = apply_title(&v, None, &[], &[], day19()).unwrap();
        assert_eq!(
            (after.content.as_str(), after.date.as_str()),
            ("Review the report", "2026-10-24")
        );
    }

    #[test]
    fn a_title_that_is_only_a_phrase_has_no_name_to_save() {
        assert_eq!(
            apply_title(&typed("tomorrow"), None, &[], &[], day19()),
            None
        );
    }

    #[test]
    fn a_typed_repeat_becomes_the_tasks_rule() {
        let (after, rule) = apply_title(
            &typed("Water plants every 3 months"),
            None,
            &[],
            &[],
            day19(),
        )
        .unwrap();
        assert_eq!(after.content, "Water plants");
        assert!(matches!(rule, Some(Recurrence::EveryNMonths { n: 3, .. })));
        assert_eq!(after.repeat, "keep");
    }

    #[test]
    fn a_typed_repeat_reads_live() {
        let r = read_repeat("every 3 tuesdays", "2026-10-24", day19()).unwrap();
        assert_eq!(r.text, "Every 3 weeks on Tue");
        assert_eq!(r.date, "2026-10-27");
        let r = read_repeat("every day for 3 days", "", day19()).unwrap();
        assert_eq!(
            (r.text.as_str(), r.date.as_str()),
            ("Every day until Oct 21", "2026-10-19")
        );
        // A past date starts from today.
        assert_eq!(
            read_repeat("every week", "2026-10-01", day19())
                .unwrap()
                .date,
            "2026-10-19"
        );
        assert_eq!(read_repeat("every week buy milk", "", day19()), None);
    }

    #[test]
    fn a_typed_repeat_pick_sets_the_rule() {
        let mut v = typed("Gym");
        v.date = "2026-10-24".into();
        v.time = "07:00".into();
        apply_pick(
            &mut v,
            &Pick::RepeatText {
                text: "every 3 tuesdays until dec 31".into(),
            },
            false,
            &[],
            &[],
            day19(),
        );
        assert_eq!(v.repeat, "custom");
        assert_eq!(
            v.rule,
            Some(Recurrence::EveryNWeeks {
                n: 3,
                days: vec![1],
                anchor: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap()
            })
        );
        assert_eq!(
            (v.date.as_str(), v.time.as_str(), v.until.as_str()),
            ("2026-10-27", "07:00", "2026-12-31")
        );
        let view = editor_view(&v, false, None, None, &[], &[], &[], day19());
        assert_eq!(view.repeat_chip_text, "Every 3 weeks on Tue until Dec 31");
        assert_eq!(view.repeat_rule, v.rule);
        assert!(view
            .repeat_options
            .iter()
            .any(|(value, label)| value == "custom" && label == "Every 3 weeks on Tue"));

        // Unreadable text changes nothing.
        let before = v.clone();
        apply_pick(
            &mut v,
            &Pick::RepeatText {
                text: "whenever".into(),
            },
            false,
            &[],
            &[],
            day19(),
        );
        assert_eq!(v, before);

        // A preset afterwards replaces it, end included.
        apply_pick(
            &mut v,
            &Pick::Repeat {
                value: "daily".into(),
            },
            false,
            &[],
            &[],
            day19(),
        );
        assert_eq!(
            (v.repeat.as_str(), v.rule.clone(), v.until.as_str()),
            ("daily", None, "")
        );
    }

    #[test]
    fn a_repeat_typed_in_the_title_is_the_menus_checked_row() {
        let v = typed("Gym every third fri");
        let view = editor_view(&v, true, None, None, &[], &[], &[], day19());
        assert_eq!(view.repeat_selected, "typed");
        assert!(view
            .repeat_options
            .iter()
            .any(|(value, label)| value == "typed" && label == "Every third Fri"));
        // Picking it again keeps the title as it is.
        let mut after = v.clone();
        apply_pick(
            &mut after,
            &Pick::Repeat {
                value: "typed".into(),
            },
            true,
            &[],
            &[],
            day19(),
        );
        assert_eq!(after, v);
        // Without one, the checked row is the chosen option.
        let view = editor_view(&typed("Gym"), true, None, None, &[], &[], &[], day19());
        assert_eq!(view.repeat_selected, "none");
        assert!(!view
            .repeat_options
            .iter()
            .any(|(value, _)| value == "typed"));
    }

    /// A task due Wed Oct 21 that repeats every Wednesday.
    fn weekly_wed(moved_from: Option<&str>, date: &str) -> Task {
        let mut task = Task::new("Swim", Id::from("P-IN"));
        task.id = Id::from("T-1");
        task.due = Some(Due {
            moved_from: moved_from.map(str::to_string),
            ..Due::repeating(date, "", Recurrence::Weekly(vec![2]))
        });
        task
    }

    #[test]
    fn a_date_pick_keeps_the_tasks_rule() {
        let wed = Some(Recurrence::Weekly(vec![2]));
        let before = EditorValues::from_task(&weekly_wed(None, "2026-10-21"));
        let mut after = before.clone();
        apply_pick(
            &mut after,
            &Pick::Date {
                date: "2026-10-22".into(),
                time: String::new(),
            },
            false,
            &[],
            &[],
            day19(),
        );
        let (patch, _) = changes(&before, &after, wed.clone(), day19());
        let due = patch.due.unwrap();
        assert_eq!(due.recurrence, wed, "not re-anchored on Thursday");
        assert_eq!(due.date.as_deref(), Some("2026-10-22"));
        assert!(moves_occurrence(&before, &after, wed.as_ref()));

        // Choosing a repeat isn't a move.
        let mut repeat = before.clone();
        apply_pick(
            &mut repeat,
            &Pick::Repeat {
                value: "daily".into(),
            },
            false,
            &[],
            &[],
            day19(),
        );
        assert!(!moves_occurrence(&before, &repeat, wed.as_ref()));
        // Nor is a date on a task that doesn't repeat.
        let plain = typed("Swim");
        let mut moved = plain.clone();
        moved.date = "2026-10-22".into();
        assert!(!moves_occurrence(&plain, &moved, None));
        // A time alone isn't one either.
        let mut timed = before.clone();
        timed.time = "09:00".into();
        assert!(!moves_occurrence(&before, &timed, wed.as_ref()));
    }

    #[test]
    fn the_date_chip_says_what_it_was_moved_from() {
        let wed = Some(Recurrence::Weekly(vec![2]));
        let v = EditorValues::from_task(&weekly_wed(Some("2026-10-21"), "2026-10-22"));
        assert_eq!(v.moved_from, "2026-10-21");
        let view = editor_view(&v, false, None, wed.clone(), &[], &[], &[], day19());
        assert_eq!(view.date_chip.0, "Thursday · moved from Wed 21 Oct");
        // Back on the repeat's own day, there's no move to show.
        let mut back = v.clone();
        back.date = "2026-10-21".into();
        let view = editor_view(&back, false, None, wed, &[], &[], &[], day19());
        assert_eq!(view.date_chip.0, "Wednesday");
    }

    #[test]
    fn a_typed_repeat_saves() {
        // Adding: the typed rule and its end go with the task.
        let mut v = typed("Gym");
        apply_pick(
            &mut v,
            &Pick::RepeatText {
                text: "every other friday for 2 months".into(),
            },
            true,
            &[],
            &[],
            day19(),
        );
        let plan = add_plan(&v, &[], &[], &[], day19());
        let Action::TaskAdd {
            due, recurrence, ..
        } = plan.task
        else {
            panic!("a task add")
        };
        assert!(matches!(
            recurrence,
            Some(Recurrence::EveryNWeeks { n: 2, .. })
        ));
        assert_eq!(due.and_then(|d| d.until).as_deref(), Some("2026-12-22"));

        // Editing: a new typed rule replaces the task's.
        let mut task = Task::new("Gym", Id::from("P-IN"));
        task.id = Id::from("T-1");
        let before = EditorValues::from_task(&task);
        let mut after = before.clone();
        apply_pick(
            &mut after,
            &Pick::RepeatText {
                text: "every 2 weeks".into(),
            },
            false,
            &[],
            &[],
            day19(),
        );
        let (patch, _) = changes(&before, &after, None, day19());
        let due = patch.due.unwrap();
        assert!(matches!(
            due.recurrence,
            Some(Recurrence::EveryNWeeks { n: 2, .. })
        ));
        // A different typed rule over a typed rule counts as a change.
        let mut again = after.clone();
        apply_pick(
            &mut again,
            &Pick::RepeatText {
                text: "every 3 weeks".into(),
            },
            false,
            &[],
            &[],
            day19(),
        );
        let (patch, _) = changes(&after, &again, None, day19());
        assert!(matches!(
            patch.due.unwrap().recurrence,
            Some(Recurrence::EveryNWeeks { n: 3, .. })
        ));
    }

    #[test]
    fn a_typed_end_goes_with_the_new_task() {
        let plan = add_plan(
            &typed("Stretch every day for 3 days"),
            &[],
            &[],
            &[],
            day19(),
        );
        let Action::TaskAdd {
            due, recurrence, ..
        } = plan.task
        else {
            panic!("a task add")
        };
        assert_eq!(recurrence, Some(Recurrence::Daily));
        assert_eq!(due.and_then(|d| d.until).as_deref(), Some("2026-10-21"));
        let view = editor_view(
            &typed("Stretch every day for 3 days"),
            true,
            None,
            None,
            &[],
            &[],
            &[],
            day19(),
        );
        assert_eq!(view.repeat_chip_text, "Every day until Oct 21");
    }

    #[test]
    fn a_tasks_end_shows_and_edits() {
        let mut task = Task::new("Stretch", Id::from("P-IN"));
        task.id = Id::from("T-1");
        task.due = Some(Due {
            until: Some("2026-10-21".into()),
            ..Due::repeating("2026-10-19", "", Recurrence::Daily)
        });
        let before = EditorValues::from_task(&task);
        assert_eq!(before.until, "2026-10-21");
        let view = editor_view(
            &before,
            false,
            None,
            Some(Recurrence::Daily),
            &[],
            &[],
            &[],
            day19(),
        );
        assert_eq!(view.repeat_chip_text, "Every day until Oct 21");

        // Untouched, the end isn't sent (the task keeps it).
        let mut after = before.clone();
        after.date = "2026-10-20".into();
        let (patch, _) = changes(&before, &after, Some(Recurrence::Daily), day19());
        assert_eq!(patch.due.unwrap().until, None);

        // A typed end replaces it; a typed repeat without one clears it.
        let (after, rule) = apply_title(
            &with_title(&before, "Stretch every day until nov 30".into()),
            Some(Recurrence::Daily),
            &[],
            &[],
            day19(),
        )
        .unwrap();
        let (patch, _) = changes(&before, &after, rule, day19());
        assert_eq!(patch.due.unwrap().until.as_deref(), Some("2026-11-30"));
        let (after, rule) = apply_title(
            &with_title(&before, "Stretch every day".into()),
            Some(Recurrence::Daily),
            &[],
            &[],
            day19(),
        )
        .unwrap();
        let (patch, _) = changes(&before, &after, rule, day19());
        assert_eq!(patch.due.unwrap().until.as_deref(), Some(""));
    }

    fn known() -> (Vec<Project>, Vec<Label>) {
        let mut work = Project::new("Work");
        work.id = Id::from("P-WORK");
        (vec![work], vec![Label::new("home")])
    }

    const NEW_TITLE: &str = "Buy ink #wrok @errands @Home @office";

    #[test]
    fn new_names_lists_typed_projects_and_labels() {
        let (projects, labels) = known();
        let n = new_names(&typed(NEW_TITLE), &projects, &[], &labels, day30());
        assert_eq!(
            n,
            NewNames {
                projects: vec!["wrok".into()],
                labels: vec!["errands".into(), "office".into()]
            }
        );
        assert_eq!(
            new_names(
                &typed("Buy ink #work @home"),
                &projects,
                &[],
                &labels,
                day30()
            ),
            NewNames {
                projects: vec![],
                labels: vec![]
            }
        );
    }

    #[test]
    fn repeated_new_label_is_listed_once() {
        let (projects, labels) = known();
        let n = new_names(
            &typed("Buy ink @errands @Errands @errands"),
            &projects,
            &[],
            &labels,
            day30(),
        );
        assert_eq!(
            n,
            NewNames {
                projects: vec![],
                labels: vec!["errands".into()]
            }
        );
    }

    #[test]
    fn picked_label_is_not_offered() {
        let (projects, labels) = known();
        let mut v = typed(NEW_TITLE);
        v.labels = vec!["errands".into()];
        let n = new_names(&v, &projects, &[], &labels, day30());
        assert_eq!(n.labels, ["office"]);
        let view = editor_view(&v, true, None, None, &projects, &[], &labels, day30());
        assert_eq!(view.new_labels, ["office"]);
    }

    #[test]
    fn segments_mark_new_names() {
        let (projects, labels) = known();
        let view = editor_view(
            &typed(NEW_TITLE),
            true,
            None,
            None,
            &projects,
            &[],
            &labels,
            day30(),
        );
        let flag = |t: &str| {
            view.segments
                .iter()
                .find(|s| s.text == t)
                .unwrap_or_else(|| panic!("{t}"))
                .is_new
        };
        assert!(flag("#wrok") && flag("@errands") && flag("@office"));
        assert!(!flag("@Home"));
        assert_eq!(view.new_project.as_deref(), Some("wrok"));
        assert!(view.segments.iter().all(|s| !s.unknown));
    }

    #[test]
    fn kept_names_become_text() {
        let (projects, labels) = known();
        let v = keep_as_text(
            &typed(NEW_TITLE),
            &NewNames {
                projects: vec!["wrok".into()],
                labels: vec!["office".into()],
            },
            &projects,
            &[],
            day30(),
        );
        let plan = add_plan(&v, &projects, &[], &labels, day30());
        assert_eq!(plan.project, None);
        assert_eq!(plan.labels, ["errands"]);
        match plan.task {
            Action::TaskAdd { content, .. } => assert_eq!(content, "Buy ink #wrok @office"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn kept_label_leaves_same_named_project() {
        let (projects, _) = known();
        let names = NewNames {
            projects: vec![],
            labels: vec!["work".into()],
        };
        let v = keep_as_text(
            &typed("Buy ink #Work @work"),
            &names,
            &projects,
            &[],
            day30(),
        );
        let plan = add_plan(&v, &projects, &[], &[], day30());
        assert!(plan.labels.is_empty());
        assert_eq!(plan.project, None);
        match plan.task {
            Action::TaskAdd {
                project_id,
                content,
                ..
            } => assert_eq!(
                (project_id, content.as_str()),
                (Id::from("P-WORK"), "Buy ink @work")
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn kept_names_ignore_trailing_punctuation() {
        let (projects, labels) = known();
        let names = NewNames {
            projects: vec!["wrok".into()],
            labels: vec!["errands".into()],
        };
        let v = keep_as_text(
            &typed("Buy ink #wrok, then @errands."),
            &names,
            &projects,
            &[],
            day30(),
        );
        let plan = add_plan(&v, &projects, &[], &labels, day30());
        assert_eq!(plan.project, None);
        assert!(plan.labels.is_empty());
        match plan.task {
            Action::TaskAdd { content, .. } => assert_eq!(content, "Buy ink #wrok, then @errands."),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn known_label_with_punctuation_is_not_new() {
        let (projects, labels) = known();
        let view = editor_view(
            &typed("call @home,"),
            true,
            None,
            None,
            &projects,
            &[],
            &labels,
            day30(),
        );
        let seg = view
            .segments
            .iter()
            .find(|s| s.text == "@home,")
            .expect("@home,");
        assert!(!seg.is_new);
    }

    #[test]
    fn picked_typed_label_is_not_new() {
        let (projects, labels) = known();
        let mut v = typed("Buy ink @errands");
        v.labels = vec!["errands".into()];
        let view = editor_view(&v, true, None, None, &projects, &[], &labels, day30());
        assert!(view.new_labels.is_empty());
        assert!(view.segments.iter().all(|s| !s.is_new));
    }

    #[test]
    fn kept_project_does_not_promote_the_next() {
        let (projects, labels) = known();
        let names = NewNames {
            projects: vec!["foo".into()],
            labels: vec![],
        };
        let v = keep_as_text(&typed("Buy #foo #bar"), &names, &projects, &[], day30());
        let plan = add_plan(&v, &projects, &[], &labels, day30());
        assert_eq!(plan.project, None);
        match plan.task {
            Action::TaskAdd { content, .. } => assert_eq!(content, "Buy #foo #bar"),
            other => panic!("{other:?}"),
        }
        // A known project behind it stays text too.
        let v = keep_as_text(&typed("Buy #foo #work"), &names, &projects, &[], day30());
        let plan = add_plan(&v, &projects, &[], &labels, day30());
        assert_eq!(plan.project, None);
        match plan.task {
            Action::TaskAdd {
                content,
                project_id,
                ..
            } => assert_eq!(
                (project_id, content.as_str()),
                (Id::from("P-IN"), "Buy #foo #work")
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn kept_name_after_multibyte_text() {
        let v = keep_as_text(
            &typed("café @office"),
            &NewNames {
                projects: vec![],
                labels: vec!["office".into()],
            },
            &[],
            &[],
            day30(),
        );
        assert_eq!(v.plain, vec![(6, 13)]);
    }

    #[test]
    fn add_plan_names_the_new_project() {
        let (projects, labels) = known();
        let plan = add_plan(&typed("Buy ink #wrok"), &projects, &[], &labels, day30());
        assert_eq!(plan.project.as_deref(), Some("wrok"));
        match plan.task {
            Action::TaskAdd {
                project_id,
                content,
                ..
            } => assert_eq!(
                (project_id, content.as_str()),
                (Id::from("P-IN"), "Buy ink")
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn task_with_project_moves_the_add() {
        let add = Action::TaskAdd {
            content: "x".into(),
            project_id: Id::from("P-IN"),
            description: None,
            priority: None,
            due: None,
            labels: None,
            parent_id: None,
            section_id: Some(Id::from("S")),
            recurrence: None,
            deadline: None,
        };
        match task_with_project(add, Id::from("P-NEW")) {
            Action::TaskAdd {
                project_id,
                section_id,
                ..
            } => assert_eq!((project_id, section_id), (Id::from("P-NEW"), None)),
            other => panic!("{other:?}"),
        }
        let other = Action::TaskDelete { id: Id::from("T") };
        assert_eq!(task_with_project(other.clone(), Id::from("P-NEW")), other);
    }

    #[test]
    fn retyped_title_with_new_project() {
        let mut home = Project::new("Home");
        home.id = Id::from("P-HOME");
        let projects = vec![home];
        let mut task = Task::new("Old", Id::from("P-HOME"));
        task.id = Id::from("T-1");
        let before = EditorValues::from_task(&task);
        let retyped = with_title(&before, "Buy ink #wrok".into());
        assert_eq!(
            new_names(&retyped, &projects, &[], &[], day30()).projects,
            ["wrok"]
        );
        let (after, _) = apply_title(&retyped, None, &projects, &[], day30()).unwrap();
        assert_eq!(after.project_id, Id::from("P-HOME"));
        let after = with_project(&after, Id::from("P-NEW"));
        let plan = edit_plan(&Id::from("T-1"), &before, &after, None, day30());
        assert_eq!(
            plan.moved,
            Some(Action::TaskMove {
                id: Id::from("T-1"),
                project_id: Id::from("P-NEW"),
                section_id: None
            })
        );
        match plan.update {
            Some(Action::TaskUpdate { content, .. }) => {
                assert_eq!(content.as_deref(), Some("Buy ink"))
            }
            other => panic!("{other:?}"),
        }
    }
}
