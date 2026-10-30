//! The task editor's data rules, shared by every UI: the values it edits,
//! the chip texts and menus, how a picked value or a typed title changes
//! the values, and what adding or saving sends. The Dioxus editor, the glass
//! card and the React editor (through `core-wasm`) all run this code.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::commands::{Action, DueEdit};
use crate::ids::Id;
use crate::model::{Due, Label, Priority, Project, Reminder, ReminderType, Section, Task};
use crate::quickadd::{self, QuickAdd, SpanKind};
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
}

impl EditorValues {
    pub fn blank(project_id: Id, section_id: Option<Id>, date: String, labels: Vec<String>) -> Self {
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
        }
    }

    pub fn from_task(t: &Task) -> Self {
        let rule = t.due.as_ref().and_then(|d| d.recurrence.clone());
        EditorValues {
            content: t.content.clone(),
            description: t.description.clone(),
            date: t.due.as_ref().and_then(|d| d.date.get(..10)).unwrap_or("").to_string(),
            time: t.due.as_ref().and_then(|d| d.date.get(11..16)).unwrap_or("").to_string(),
            priority: (5 - t.priority.to_api()).to_string(),
            repeat: repeat_choice(&rule).to_string(),
            project_id: t.project_id.clone(),
            section_id: t.section_id.clone(),
            labels: t.labels.clone(),
            parent_id: t.parent_id.clone(),
            deadline: t.deadline.clone().unwrap_or_default(),
            reminders: Vec::new(),
            plain: Vec::new(),
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
            (true, false) => format!("{}T{}:00", today.format("%Y-%m-%d"), self.time),
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
            at: if relative { None } else { r.at().map(|a| a.format("%Y-%m-%dT%H:%M").to_string()) },
        }
    }

    /// "At time of task", "30 min before", "Tuesday 9 AM".
    pub fn label(&self, today: NaiveDate) -> String {
        if let Some(m) = self.minute_offset {
            return offset_text(m);
        }
        match self.at.as_deref().and_then(|a| chrono::NaiveDateTime::parse_from_str(a, "%Y-%m-%dT%H:%M").ok()) {
            Some(at) => format!("{} {}", views::due_label(at.date(), today), views::time_label(at.time())),
            None => "Reminder".to_string(),
        }
    }
}

/// Relative reminder choices, in minutes before the due time.
pub const REMINDER_PRESETS: [i64; 5] = [0, 10, 30, 60, 1440];

/// The presets with their text ("At time of task", "10 min before", …).
pub fn reminder_presets() -> Vec<(i64, String)> {
    REMINDER_PRESETS.iter().map(|&m| (m, offset_text(m))).collect()
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

/// A value chosen in a picker, in [`EditorValues`]' formats.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Pick {
    Date { date: String, time: String },
    Priority { value: String },
    Labels { names: Vec<String> },
    Reminders { drafts: Vec<ReminderDraft> },
    Deadline { date: String },
    Repeat { value: String },
    Location { project_id: Id, section_id: Option<Id> },
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
}

impl NewTask {
    /// The `TaskAdd` that creates this task.
    pub fn into_action(self) -> Action {
        let due = (!self.date.is_empty()).then(|| Due {
            date: self.date.clone(),
            timezone: None,
            string: self.date.clone(),
            lang: "en".to_string(),
            is_recurring: false,
            recurrence: None,
        });
        let description = (!self.description.trim().is_empty()).then(|| self.description.trim().to_string());
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
pub fn recurrence_for(choice: &str, date: &str, existing: Option<Recurrence>, today: NaiveDate) -> Option<Recurrence> {
    let anchor = NaiveDate::parse_from_str(date.get(..10).unwrap_or(date), "%Y-%m-%d").unwrap_or(today);
    match choice {
        "daily" => Some(Recurrence::Daily),
        "weekdays" => Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])),
        "weekly" => Some(Recurrence::Weekly(vec![anchor.weekday().num_days_from_monday() as u8])),
        "monthly" => Some(Recurrence::MonthlyDay(anchor.day() as u8)),
        "keep" => existing,
        _ => None,
    }
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
    if labels.is_empty() { "Labels".to_string() } else { labels.join(", ") }
}

/// The Priority chip's text ("4" is no priority).
pub fn priority_text(priority: &str) -> String {
    if priority == "4" { "Priority".to_string() } else { format!("P{priority}") }
}

/// The repeat choices as (value, label); week/month options name the day
/// they repeat on, and "keep" appears when the task has a custom rule.
pub fn repeat_options(date: &str, keep_label: Option<&str>, today: NaiveDate) -> Vec<(String, String)> {
    let anchor = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap_or(today);
    let mut opts = vec![
        ("none".to_string(), "No repeat".to_string()),
        ("daily".to_string(), "Every day".to_string()),
        ("weekdays".to_string(), "Every weekday (Mon – Fri)".to_string()),
        ("weekly".to_string(), format!("Every week on {}", anchor.format("%A"))),
        ("monthly".to_string(), format!("Every month on the {}", recurrence::ordinal(anchor.day()))),
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

/// "Project" or "Project / Section" for the location chip.
pub fn location_text(project_id: &Id, section_id: Option<&Id>, projects: &[Project], sections: &[Section]) -> String {
    let Some(p) = projects.iter().find(|p| &p.id == project_id) else { return "Inbox".to_string() };
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
        let kind = if p.is_inbox { LocationKind::Inbox } else { LocationKind::Project };
        items.push(LocationChoice { value: p.id.to_string(), label: p.name.clone(), kind });
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
        Some((p, s)) => Pick::Location { project_id: Id::from(p), section_id: Some(Id::from(s)) },
        None => Pick::Location { project_id: Id::from(value), section_id: None },
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
    let due_changed = before.date != after.date || before.time != after.time || before.repeat != after.repeat;
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
            recurrence: recurrence_for(&after.repeat, &due, existing, today),
        }),
    };
    let moved = (after.project_id != before.project_id || after.section_id != before.section_id)
        .then(|| (after.project_id.clone(), after.section_id.clone()));
    (patch, moved)
}

/// Parses the editor's title as quick add, leaving the kept-plain phrases.
pub fn parse_title(v: &EditorValues, projects: &[Project], sections: &[Section], today: NaiveDate) -> QuickAdd {
    let mut qa = quickadd::parse_keeping(&v.content, today, projects, sections, &v.plain);
    // A bare /Section resolves within the Inbox; carry its project along.
    if qa.project_id.is_none() {
        if let Some(s) = qa.section_id.as_ref().and_then(|id| sections.iter().find(|s| &s.id == id)) {
            qa.project_id = Some(s.project_id.clone());
        }
    }
    qa
}

/// Whether the title has a name to save: with quick add, something besides
/// the recognized phrases ("This weekend" alone highlights but can't be
/// added yet).
pub fn has_name(v: &EditorValues, quick_add: bool, projects: &[Project], sections: &[Section], today: NaiveDate) -> bool {
    if quick_add {
        !parse_title(v, projects, sections, today).content.trim().is_empty()
    } else {
        !v.content.trim().is_empty()
    }
}

/// The typed phrases a pick replaces.
fn kinds_for(pick: &Pick) -> &'static [SpanKind] {
    match pick {
        Pick::Date { .. } => &[SpanKind::Date, SpanKind::Time],
        Pick::Priority { .. } => &[SpanKind::Priority],
        Pick::Labels { .. } => &[SpanKind::Label],
        Pick::Reminders { .. } => &[],
        Pick::Deadline { .. } => &[SpanKind::Deadline],
        Pick::Repeat { .. } => &[SpanKind::Repeat],
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
    if quick_add {
        let qa = parse_title(v, projects, sections, today);
        let mut kinds = kinds_for(pick).to_vec();
        if let Pick::Date { date, .. } = pick {
            let shown = qa.date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| v.date.clone());
            if *date != shown {
                kinds.push(SpanKind::Repeat);
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
        Pick::Repeat { value } => v.repeat = value.clone(),
        Pick::Location { project_id, section_id } => (v.project_id, v.section_id) = (project_id.clone(), section_id.clone()),
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
    apply_pick(v, &Pick::Labels { names }, quick_add, projects, sections, today);
}

/// The values after the title is retyped: kept-plain phrases follow the
/// text, or are dropped when the edit touches them.
pub fn with_title(v: &EditorValues, text: String) -> EditorValues {
    EditorValues { plain: quickadd::shift_plain(&v.content, &text, &v.plain), content: text, ..v.clone() }
}

/// Turns the highlighted phrase ending at byte `end` back into plain text
/// (Todoist's backspace). False when no highlight ends there.
pub fn unhighlight(v: &mut EditorValues, projects: &[Project], sections: &[Section], today: NaiveDate, end: usize) -> bool {
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
            Ok(t) => format!("{}T{}", d.format("%Y-%m-%d"), t.format("%H:%M:00")),
            Err(_) => d.format("%Y-%m-%d").to_string(),
        },
        _ => qa.due_string().unwrap_or_else(|| v.due_string(today)),
    };
    let recurrence = qa.recurrence.clone().or_else(|| recurrence_for(&v.repeat, &date, None, today));
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
        deadline: qa.deadline.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| v.deadline.clone()),
    }
}

/// The labels a pick added by a name no label has yet (any case), so they
/// can be created: names already on the task aren't.
pub fn labels_to_create(before: &[String], after: &[String], known: &[Label]) -> Vec<String> {
    after
        .iter()
        .filter(|n| !before.iter().any(|b| b.eq_ignore_ascii_case(n)))
        .filter(|n| !known.iter().any(|l| l.name.eq_ignore_ascii_case(n)))
        .cloned()
        .collect()
}

/// The Date chip's text and due bucket (`None` when no date is set; a time
/// alone means today).
pub fn date_chip(date: Option<NaiveDate>, time: Option<NaiveTime>, today: NaiveDate) -> (String, Option<DueBucket>) {
    match (date, time) {
        (Some(d), Some(t)) => (format!("{} {}", views::due_label(d, today), views::time_label(t)), Some(views::due_bucket(d, today))),
        (Some(d), None) => (views::due_label(d, today), Some(views::due_bucket(d, today))),
        (None, Some(t)) => (format!("Today {}", views::time_label(t)), Some(DueBucket::Today)),
        (None, None) => ("Date".to_string(), None),
    }
}

/// The Deadline chip's text.
pub fn deadline_text(deadline: Option<NaiveDate>, today: NaiveDate) -> String {
    deadline.map(|d| views::due_label(d, today)).unwrap_or_else(|| "Deadline".to_string())
}

/// The Reminders chip's text.
pub fn reminders_text(reminders: &[ReminderDraft], today: NaiveDate) -> String {
    match reminders.len() {
        0 => "Reminders".to_string(),
        1 => reminders[0].label(today),
        n => format!("{n} reminders"),
    }
}

const DOW: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// The date picker's shortcuts: (label, hint, date).
pub fn quick_dates(today: NaiveDate) -> Vec<(&'static str, String, NaiveDate)> {
    let saturday = today + Duration::days(((5 - today.weekday().num_days_from_monday() as i64) + 7) % 7);
    let next_monday = today + Duration::days(7 - today.weekday().num_days_from_monday() as i64);
    let tomorrow = today + Duration::days(1);
    vec![
        ("Today", DOW[today.weekday().num_days_from_monday() as usize].to_string(), today),
        ("Tomorrow", DOW[tomorrow.weekday().num_days_from_monday() as usize].to_string(), tomorrow),
        ("This weekend", DOW[5].to_string(), saturday),
        (
            "Next week",
            format!("Mon {} {}", next_monday.day(), &crate::calendar::MONTHS[next_monday.month0() as usize][..3]),
            next_monday,
        ),
    ]
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
    fn a_label_picked_by_a_new_name_is_created_once() {
        let known = vec![Label::new("home")];
        let before: Vec<String> = vec!["home".into()];
        let after: Vec<String> = vec!["home".into(), "Travel".into()];
        assert_eq!(labels_to_create(&before, &after, &known), ["Travel"]);
        // Known names (any case) and names already on the task aren't created.
        let after: Vec<String> = vec!["HOME".into()];
        assert!(labels_to_create(&before, &after, &known).is_empty());
        let before: Vec<String> = vec!["Travel".into()];
        let after: Vec<String> = vec!["travel".into()];
        assert!(labels_to_create(&before, &after, &known).is_empty());
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
        let pick = Pick::Date { date: "2026-10-23".into(), time: String::new() };
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
        assert_eq!(custom_reminder_at("tomorrow 9am", day30()), Some("2026-10-31T09:00".to_string()));
        assert_eq!(custom_reminder_at("tomorrow", day30()), None);
    }

    #[test]
    fn location_choices_nest_sections() {
        let mut inbox = Project::new("Inbox");
        inbox.is_inbox = true;
        let home = Project::new("Home");
        let garden = Section::new(home.id.clone(), "Garden", 0);
        let items = location_choices(&[inbox.clone(), home.clone()], &[garden.clone()]);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].kind, LocationKind::Inbox);
        assert_eq!(items[1], LocationChoice { value: home.id.to_string(), label: "Home".into(), kind: LocationKind::Project });
        assert_eq!(items[2].value, format!("{}|{}", home.id, garden.id));
        assert_eq!(items[2].kind, LocationKind::Section);
        assert_eq!(
            location_pick(&items[2].value),
            Pick::Location { project_id: home.id.clone(), section_id: Some(garden.id.clone()) }
        );
    }

    #[test]
    fn date_chip_texts() {
        let t = NaiveTime::from_hms_opt(17, 0, 0);
        let tomorrow = NaiveDate::from_ymd_opt(2026, 10, 31);
        assert_eq!(date_chip(tomorrow, t, day30()), ("Tomorrow 5 PM".to_string(), Some(DueBucket::Tomorrow)));
        assert_eq!(date_chip(tomorrow, None, day30()), ("Tomorrow".to_string(), Some(DueBucket::Tomorrow)));
        assert_eq!(date_chip(None, t, day30()), ("Today 5 PM".to_string(), Some(DueBucket::Today)));
        assert_eq!(date_chip(None, None, day30()), ("Date".to_string(), None));
    }

    #[test]
    fn pick_keeps_its_camel_case_shape() {
        let p: Pick = serde_json::from_str(r#"{"type":"location","projectId":"P","sectionId":null}"#).unwrap();
        assert_eq!(p, Pick::Location { project_id: Id::from("P"), section_id: None });
        let d: ReminderDraft = serde_json::from_str(r#"{"id":null,"minuteOffset":30,"at":null}"#).unwrap();
        assert_eq!(d.minute_offset, Some(30));
    }
}
