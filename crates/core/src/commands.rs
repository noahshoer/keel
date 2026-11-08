use std::collections::HashMap;

use chrono::{Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::model::{Due, Filter, Label, LastCompletion, Priority, Project, Reminder, Section, Task};
use crate::recurrence::Recurrence;
use crate::repository::{RepoError, Repository};
use crate::streak;
use crate::views;

/// The result of applying a command: any temp-id → real-id mappings created.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ApplyResult {
    pub id_mapping: HashMap<String, Id>,
    /// Parents a sub-task's completion completed with it, nearest first.
    #[serde(default)]
    pub auto_completed: Vec<Id>,
}

/// A single mutation. Carries a client `uuid` (idempotency) and, for creates,
/// a `temp_id` that later commands/UI can resolve via `ApplyResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields = nullable))]
pub struct Command {
    pub uuid: String,
    #[serde(default)]
    pub temp_id: Option<String>,
    pub action: Action,
}

/// A due-date edit. `date` is `YYYY-MM-DD` (or a datetime); with only a
/// `recurrence`, the first occurrence on/after today is used; both `None`
/// clears the due.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields = nullable))]
pub struct DueEdit {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub recurrence: Option<Recurrence>,
    /// The repeat's last day: `None` keeps the task's end while the rule stays
    /// the same, `Some("")` clears it.
    #[serde(default)]
    pub until: Option<String>,
}

/// The operations supported so far. (update/move/uncomplete land with their
/// features.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields = nullable))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    TaskAdd {
        content: String,
        project_id: Id,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        priority: Option<Priority>,
        #[serde(default)]
        due: Option<Due>,
        #[serde(default)]
        labels: Option<Vec<String>>,
        #[serde(default)]
        parent_id: Option<Id>,
        #[serde(default)]
        section_id: Option<Id>,
        /// Makes the task recurring. With no `due`, the first occurrence is
        /// the first scheduled day on or after today.
        #[serde(default)]
        recurrence: Option<Recurrence>,
        /// "Must be done by" date (`YYYY-MM-DD`), separate from the due.
        #[serde(default)]
        deadline: Option<String>,
    },
    /// Patch a task: every field is optional; omitted fields are unchanged.
    TaskUpdate {
        id: Id,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        priority: Option<Priority>,
        #[serde(default)]
        project_id: Option<Id>,
        #[serde(default)]
        labels: Option<Vec<String>>,
        /// `None` keeps the due; `Some(DueEdit { date: None, recurrence: None })` clears it.
        #[serde(default)]
        due: Option<DueEdit>,
        /// Collapse/expand the task's sub-tasks.
        #[serde(default)]
        is_collapsed: Option<bool>,
        /// `None` keeps the deadline; `Some("")` clears it.
        #[serde(default)]
        deadline: Option<String>,
    },
    /// Move a task (and its whole sub-tree) to a project, optionally a section.
    TaskMove {
        id: Id,
        project_id: Id,
        #[serde(default)]
        section_id: Option<Id>,
    },
    TaskComplete {
        id: Id,
    },
    SectionAdd {
        project_id: Id,
        name: String,
    },
    SectionUpdate {
        id: Id,
        name: String,
    },
    /// Soft-delete a section and the tasks in it.
    SectionDelete {
        id: Id,
    },
    /// Reopen a completed task.
    TaskUncomplete {
        id: Id,
    },
    TaskDelete {
        id: Id,
    },
    /// Opt a task in/out of streak tracking and set (or clear) its goal.
    TaskSetStreak {
        id: Id,
        enabled: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional = nullable, type = "number | null"))]
        goal: Option<i64>,
    },
    /// Create a label. Names are unique case-insensitively; re-adding an
    /// existing name maps `temp_id` to the existing label.
    LabelAdd {
        name: String,
    },
    /// Rename a label (and on every task). Renaming onto an existing label's
    /// name merges the two.
    LabelUpdate {
        id: Id,
        name: String,
    },
    /// Set a label's color. An unknown id does nothing.
    LabelSetColor {
        id: Id,
        color: String,
    },
    /// Soft-delete a label and strip its name from every task.
    LabelDelete {
        id: Id,
    },
    ProjectAdd {
        name: String,
    },
    /// Save a filter (name + query; the query is validated by the UI/service).
    FilterAdd {
        name: String,
        query: String,
    },
    FilterUpdate {
        id: Id,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        query: Option<String>,
    },
    FilterDelete {
        id: Id,
    },
    /// Rename a project. No-op for the Inbox.
    ProjectUpdate {
        id: Id,
        name: String,
    },
    /// Soft-delete a project and its tasks. No-op for the Inbox.
    ProjectDelete {
        id: Id,
    },
    /// A reminder on a task: `minute_offset` before its due time (relative),
    /// or at a fixed local time `at` = `YYYY-MM-DDTHH:MM` (absolute).
    ReminderAdd {
        task_id: Id,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional = nullable, type = "number | null"))]
        minute_offset: Option<i64>,
        #[serde(default)]
        at: Option<String>,
    },
    ReminderDelete {
        id: Id,
    },
    /// Turn on/off completing a task once all its sub-tasks are done.
    TaskSetDoneWithSubtasks {
        id: Id,
        enabled: bool,
    },
}

impl Action {
    fn type_name(&self) -> &'static str {
        match self {
            Action::TaskAdd { .. } => "task_add",
            Action::TaskUpdate { .. } => "task_update",
            Action::TaskComplete { .. } => "task_complete",
            Action::TaskUncomplete { .. } => "task_uncomplete",
            Action::TaskDelete { .. } => "task_delete",
            Action::TaskMove { .. } => "task_move",
            Action::SectionAdd { .. } => "section_add",
            Action::SectionUpdate { .. } => "section_update",
            Action::SectionDelete { .. } => "section_delete",
            Action::TaskSetStreak { .. } => "task_set_streak",
            Action::TaskSetDoneWithSubtasks { .. } => "task_set_done_with_subtasks",
            Action::LabelAdd { .. } => "label_add",
            Action::LabelUpdate { .. } => "label_update",
            Action::LabelSetColor { .. } => "label_set_color",
            Action::LabelDelete { .. } => "label_delete",
            Action::ProjectAdd { .. } => "project_add",
            Action::FilterAdd { .. } => "filter_add",
            Action::FilterUpdate { .. } => "filter_update",
            Action::FilterDelete { .. } => "filter_delete",
            Action::ProjectUpdate { .. } => "project_update",
            Action::ProjectDelete { .. } => "project_delete",
            Action::ReminderAdd { .. } => "reminder_add",
            Action::ReminderDelete { .. } => "reminder_delete",
        }
    }
}

/// A local date-time typed as `YYYY-MM-DDTHH:MM` (seconds optional).
fn parse_local_datetime(s: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%dT%H:%M"))
        .ok()
}

/// `YYYY-MM-DD` when `s` is a real date (anything else, e.g. "", gives `None`).
fn valid_date(s: &str) -> Option<String> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok().map(iso)
}

/// Human text for a rule, stored in `Due.string` (Todoist-style).
fn describe(rule: &Recurrence) -> String {
    rule.describe()
}

/// Builds a `Due` from an optional date and rule. With only a rule, the first
/// occurrence on/after `today` is used; with neither, there is no due. An end
/// (`until`) only goes with a rule.
fn build_due(date: Option<String>, rule: Option<Recurrence>, until: Option<String>, today: NaiveDate) -> Option<Due> {
    let until = until.filter(|_| rule.is_some()).as_deref().and_then(valid_date);
    let (date, string) = match (&date, &rule) {
        (None, None) => return None,
        (Some(d), None) => (d.clone(), d.clone()),
        (Some(d), Some(r)) => (d.clone(), describe(r)),
        (None, Some(r)) => {
            let first = if r.contains(today) { today } else { r.next_after(today) };
            (iso(first), describe(r))
        }
    };
    let string = match until.as_deref().and_then(|u| NaiveDate::parse_from_str(u, "%Y-%m-%d").ok()) {
        Some(end) => format!("{string} until {}", end.format("%b %-d")),
        None => string,
    };
    Some(Due {
        date,
        timezone: None,
        string,
        lang: "en".to_string(),
        is_recurring: rule.is_some(),
        recurrence: rule,
        until,
        moved_from: None,
    })
}

/// True when the occurrence a completion on `today` is for is the repeat's
/// last: the next one would fall past its end.
fn repeat_ends(due: &Due, rule: &Recurrence, current: NaiveDate, today: NaiveDate) -> bool {
    let Some(end) = due.until.as_deref().and_then(|u| NaiveDate::parse_from_str(u, "%Y-%m-%d").ok()) else {
        return false;
    };
    let occurrence = moved_from(due).unwrap_or_else(|| completed_occurrence(rule, current, today));
    rule.next_after(occurrence.max(current)) > end
}

/// The occurrence a moved due date stands in for.
fn moved_from(due: &Due) -> Option<NaiveDate> {
    due.moved_from.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

/// After an edit that kept the rule: the occurrence the new date stands in
/// for, or `None` when it falls on one of the repeat's own days.
fn moved_occurrence(old: &Due, new: &Due, rule: &Recurrence) -> Option<String> {
    let new_day = due_date(new)?;
    if rule.contains(new_day) {
        return None;
    }
    let old_day = due_date(old).filter(|d| rule.contains(*d));
    moved_from(old).or(old_day).map(iso)
}

fn iso(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// The date part of a Todoist-style due (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM…`).
fn due_date(due: &Due) -> Option<NaiveDate> {
    due.date
        .get(..10)
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}

/// Apply a command to the repository using the local date as "today".
pub fn apply<R: Repository>(repo: &R, cmd: Command) -> Result<ApplyResult, RepoError> {
    apply_at(repo, cmd, Local::now().date_naive())
}

/// Apply a command with an explicit `today` (deterministic for tests).
/// Idempotent by `uuid`: re-applying a recorded command is a no-op.
pub fn apply_at<R: Repository>(
    repo: &R,
    cmd: Command,
    today: NaiveDate,
) -> Result<ApplyResult, RepoError> {
    if repo.has_command(&cmd.uuid)? {
        return Ok(ApplyResult::default());
    }

    let mut result = ApplyResult::default();
    let args_json =
        serde_json::to_string(&cmd.action).map_err(|e| RepoError::Backend(e.to_string()))?;
    let type_name = cmd.action.type_name();

    match cmd.action {
        Action::TaskAdd {
            content,
            project_id,
            description,
            priority,
            due,
            labels,
            parent_id,
            section_id,
            recurrence,
            deadline,
        } => {
            let mut task = Task::new(content, project_id);
            task.deadline = deadline.as_deref().and_then(valid_date);
            if let Some(d) = description {
                task.description = d;
            }
            if let Some(p) = priority {
                task.priority = p;
            }
            task.due = match (due, recurrence) {
                (due, None) => due,
                (due, rule) => {
                    let until = due.as_ref().and_then(|d| d.until.clone());
                    build_due(due.map(|d| d.date), rule, until, today)
                }
            };
            if let Some(l) = labels {
                task.labels = l;
            }
            task.parent_id = parent_id;
            task.section_id = section_id;
            repo.upsert_task(&task)?;
            if let Some(temp) = cmd.temp_id {
                result.id_mapping.insert(temp, task.id.clone());
            }
        }
        Action::TaskComplete { id } => {
            if complete_task(repo, &id, today)? {
                result.auto_completed = complete_parents(repo, &id, today)?;
            }
        }
        Action::TaskMove { id, project_id, section_id } => {
            move_subtree(repo, &id, &project_id, section_id.as_ref())?;
        }
        Action::SectionAdd { project_id, name } => {
            let order = repo.list_sections(&project_id)?.len() as i64;
            let section = Section::new(project_id, name.trim(), order);
            repo.upsert_section(&section)?;
            if let Some(temp) = cmd.temp_id {
                result.id_mapping.insert(temp, section.id.clone());
            }
        }
        Action::SectionUpdate { id, name } => {
            let name = name.trim().to_string();
            if let (Some(mut section), false) = (repo.get_section(&id)?, name.is_empty()) {
                section.name = name;
                section.updated_at = Utc::now();
                repo.upsert_section(&section)?;
            }
        }
        Action::SectionDelete { id } => {
            if let Some(mut section) = repo.get_section(&id)? {
                for task in repo.list_tasks(&section.project_id)? {
                    if task.section_id.as_ref() == Some(&id) {
                        repo.soft_delete_task(&task.id)?;
                    }
                }
                section.is_deleted = true;
                section.updated_at = Utc::now();
                repo.upsert_section(&section)?;
            }
        }
        Action::TaskUpdate {
            id,
            content,
            description,
            priority,
            project_id,
            labels,
            due,
            is_collapsed,
            deadline,
        } => {
            // A project change is a move: the sub-tree follows and the
            // section (which belongs to the old project) is cleared.
            if let Some(p) = &project_id {
                if repo.get_task(&id)?.is_some_and(|t| &t.project_id != p) {
                    move_subtree(repo, &id, p, None)?;
                }
            }
            if let Some(mut task) = repo.get_task(&id)? {
                if let Some(c) = content {
                    task.content = c;
                }
                if let Some(d) = description {
                    task.description = d;
                }
                if let Some(p) = priority {
                    task.priority = p;
                }
                if let Some(c) = is_collapsed {
                    task.is_collapsed = c;
                }
                if let Some(l) = labels {
                    task.labels = l;
                }
                if let Some(d) = deadline {
                    task.deadline = valid_date(&d);
                }
                if let Some(edit) = due {
                    let old_due = task.due.clone();
                    let old_rule = old_due.as_ref().and_then(|d| d.recurrence.clone());
                    let until = match edit.until {
                        Some(u) => Some(u),
                        None if edit.recurrence == old_rule => task.due.as_ref().and_then(|d| d.until.clone()),
                        None => None,
                    };
                    task.due = build_due(edit.date, edit.recurrence, until, today);
                    // A new date under the same rule moves just this occurrence;
                    // a streak can't carry over a move.
                    if let (Some(old), Some(new), Some(rule)) = (old_due.as_ref(), task.due.as_mut(), old_rule.as_ref()) {
                        if new.recurrence.as_ref() == Some(rule) && due_date(old) != due_date(new) {
                            new.moved_from = moved_occurrence(old, new, rule);
                            task.streak_current = 0;
                            task.streak_last_period = None;
                            task.last_completion = None;
                        } else if new.recurrence.as_ref() == Some(rule) {
                            new.moved_from = old.moved_from.clone();
                        }
                    }
                    let new_rule = task.due.as_ref().and_then(|d| d.recurrence.clone());
                    // A different schedule invalidates the occurrence history.
                    if new_rule != old_rule {
                        task.streak_current = 0;
                        task.streak_last_period = None;
                        task.last_completion = None;
                    }
                }
                task.updated_at = Utc::now();
                repo.upsert_task(&task)?;
            }
        }
        Action::TaskDelete { id } => {
            // The task's reminders (and its sub-tasks') go with it.
            let mut gone: Vec<Id> = descendants(repo, &id)?.into_iter().map(|t| t.id).collect();
            gone.push(id.clone());
            for mut r in repo.list_reminders()? {
                if gone.contains(&r.task_id) {
                    r.is_deleted = true;
                    repo.upsert_reminder(&r)?;
                }
            }
            repo.soft_delete_task(&id)?;
        }
        Action::ReminderAdd { task_id, minute_offset, at } => {
            let task_exists = repo.get_task(&task_id)?.is_some_and(|t| !t.is_deleted);
            let reminder = match (minute_offset, at.as_deref().and_then(parse_local_datetime)) {
                _ if !task_exists => None,
                (Some(m), _) => Some(Reminder::relative(task_id, m)),
                (None, Some(at)) => Some(Reminder::absolute(task_id, at)),
                (None, None) => None,
            };
            if let Some(r) = reminder {
                repo.upsert_reminder(&r)?;
                if let Some(temp) = cmd.temp_id {
                    result.id_mapping.insert(temp, r.id.clone());
                }
            }
        }
        Action::ReminderDelete { id } => {
            if let Some(mut r) = repo.list_reminders()?.into_iter().find(|r| r.id == id) {
                r.is_deleted = true;
                repo.upsert_reminder(&r)?;
            }
        }
        Action::TaskSetDoneWithSubtasks { id, enabled } => {
            if let Some(mut task) = repo.get_task(&id)? {
                task.done_with_subtasks = enabled;
                task.updated_at = Utc::now();
                repo.upsert_task(&task)?;
            }
        }
        Action::TaskSetStreak { id, enabled, goal } => {
            if let Some(mut task) = repo.get_task(&id)? {
                task.streak_enabled = enabled;
                task.streak_goal = goal;
                task.last_completion = None;
                task.updated_at = Utc::now();
                repo.upsert_task(&task)?;
            }
        }
        Action::LabelAdd { name } => {
            let name = name.trim().to_string();
            let existing = repo
                .list_labels()?
                .into_iter()
                .find(|l| l.name.eq_ignore_ascii_case(&name));
            let id = match existing {
                Some(label) => label.id,
                None => {
                    let label = Label::new(name);
                    repo.upsert_label(&label)?;
                    label.id
                }
            };
            if let Some(temp) = cmd.temp_id {
                result.id_mapping.insert(temp, id);
            }
        }
        Action::LabelDelete { id } => {
            if let Some(mut label) = repo.list_labels()?.into_iter().find(|l| l.id == id) {
                for mut task in repo.list_all_tasks()? {
                    let before = task.labels.len();
                    task.labels.retain(|n| !n.eq_ignore_ascii_case(&label.name));
                    if task.labels.len() != before {
                        task.updated_at = Utc::now();
                        repo.upsert_task(&task)?;
                    }
                }
                label.is_deleted = true;
                repo.upsert_label(&label)?;
            }
        }
        Action::ProjectAdd { name } => {
            let project = Project::new(name);
            repo.upsert_project(&project)?;
            if let Some(temp) = cmd.temp_id {
                result.id_mapping.insert(temp, project.id.clone());
            }
        }
        Action::FilterAdd { name, query } => {
            let order = repo.list_filters()?.len() as i64;
            let filter = Filter::new(name.trim(), query.trim(), order);
            repo.upsert_filter(&filter)?;
            if let Some(temp) = cmd.temp_id {
                result.id_mapping.insert(temp, filter.id.clone());
            }
        }
        Action::FilterUpdate { id, name, query } => {
            if let Some(mut filter) = repo.list_filters()?.into_iter().find(|f| f.id == id) {
                if let Some(n) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
                    filter.name = n;
                }
                if let Some(q) = query.map(|q| q.trim().to_string()).filter(|q| !q.is_empty()) {
                    filter.query = q;
                }
                repo.upsert_filter(&filter)?;
            }
        }
        Action::FilterDelete { id } => {
            if let Some(mut filter) = repo.list_filters()?.into_iter().find(|f| f.id == id) {
                filter.is_deleted = true;
                repo.upsert_filter(&filter)?;
            }
        }
        Action::ProjectUpdate { id, name } => {
            let name = name.trim().to_string();
            if let Some(mut project) = repo.get_project(&id)? {
                if !project.is_inbox && !name.is_empty() {
                    project.name = name;
                    project.updated_at = Utc::now();
                    repo.upsert_project(&project)?;
                }
            }
        }
        Action::ProjectDelete { id } => {
            if let Some(mut project) = repo.get_project(&id)? {
                if project.is_inbox {
                    return finish(repo, &cmd.uuid, type_name, &args_json, result);
                }
                project.is_deleted = true;
                project.updated_at = Utc::now();
                repo.upsert_project(&project)?;
            }
            for task in repo.list_tasks(&id)? {
                repo.soft_delete_task(&task.id)?;
            }
        }
        Action::TaskUncomplete { id } => {
            let was_done = repo.get_task(&id)?.is_some_and(|t| views::done_for_now(&t, today));
            uncomplete_task(repo, &id, today)?;
            if !was_done {
                return finish(repo, &cmd.uuid, type_name, &args_json, result);
            }
            // A parent done today isn't any more.
            let mut child = id;
            while let Some(parent) = repo.get_task(&child)?.and_then(|t| t.parent_id) {
                match repo.get_task(&parent)? {
                    Some(p) if views::done_today(&p, today) => uncomplete_task(repo, &parent, today)?,
                    _ => break,
                }
                child = parent;
            }
        }
        Action::LabelSetColor { id, color } => {
            if let Some(mut label) = repo.list_labels()?.into_iter().find(|l| l.id == id) {
                label.color = color;
                repo.upsert_label(&label)?;
            }
        }
        Action::LabelUpdate { id, name } => {
            let name = name.trim().to_string();
            let labels = repo.list_labels()?;
            if let (Some(mut label), false) = (labels.iter().find(|l| l.id == id).cloned(), name.is_empty()) {
                let old = label.name.clone();
                // Renaming onto another label's name merges into that label.
                let target = labels
                    .iter()
                    .find(|l| l.id != id && l.name.eq_ignore_ascii_case(&name))
                    .map(|l| l.name.clone());
                let new_name = target.clone().unwrap_or_else(|| name.clone());
                for mut task in repo.list_all_tasks()? {
                    if !task.labels.iter().any(|n| n.eq_ignore_ascii_case(&old)) {
                        continue;
                    }
                    let mut renamed: Vec<String> = Vec::new();
                    for n in task.labels.drain(..) {
                        let n = if n.eq_ignore_ascii_case(&old) { new_name.clone() } else { n };
                        if !renamed.iter().any(|r| r.eq_ignore_ascii_case(&n)) {
                            renamed.push(n);
                        }
                    }
                    task.labels = renamed;
                    task.updated_at = Utc::now();
                    repo.upsert_task(&task)?;
                }
                if target.is_some() {
                    label.is_deleted = true;
                } else {
                    label.name = new_name;
                }
                repo.upsert_label(&label)?;
            }
        }
    }

    finish(repo, &cmd.uuid, type_name, &args_json, result)
}

/// Completes a task (see `Action::TaskComplete`). False when nothing changed:
/// no such task, or its occurrence was already credited.
fn complete_task<R: Repository>(repo: &R, id: &Id, today: NaiveDate) -> Result<bool, RepoError> {
    let Some(mut task) = repo.get_task(id)? else { return Ok(false) };
    let now = Utc::now();
    // A repeat past its last occurrence completes like a plain task.
    let recurring = task.due.as_ref().and_then(|d| {
        d.recurrence.clone().zip(due_date(d)).filter(|(rule, current)| !repeat_ends(d, rule, *current, today))
    });
    match recurring {
        // Recurring: the task stays active, past the occurrence
        // being completed. Re-completing an already-credited
        // occurrence is a no-op.
        Some((rule, current)) => {
            let Some(passed) = complete_occurrence(&mut task, &rule, current, today) else {
                return Ok(false);
            };
            // Sub-tasks complete with it. Plain ones stay checked until
            // its next date; repeating ones move on, unless they're
            // already past this occurrence.
            let reopens_on = task.due.as_ref().and_then(|d| d.date.get(..10)).map(str::to_string);
            for mut child in descendants(repo, &task.id)? {
                let child_recurring = child.due.as_ref().is_some_and(|d| d.is_recurring);
                if !child_recurring {
                    if !child.checked {
                        child.checked = true;
                        child.completed_at = Some(now);
                    }
                    child.reopens_on = reopens_on.clone();
                    child.updated_at = now;
                    repo.upsert_task(&child)?;
                }
                let rule = child.due.as_ref().and_then(|d| d.recurrence.clone().zip(due_date(d)));
                if let Some((rule, due)) = rule.filter(|(_, due)| !child.checked && *due <= passed) {
                    let ends = child.due.as_ref().is_some_and(|d| repeat_ends(d, &rule, due, today));
                    if ends {
                        child.checked = true;
                        child.completed_at = Some(now);
                        child.updated_at = now;
                        repo.upsert_task(&child)?;
                    } else if complete_occurrence(&mut child, &rule, due, today).is_some() {
                        child.updated_at = now;
                        repo.upsert_task(&child)?;
                    }
                }
            }
        }
        None => {
            task.checked = true;
            task.completed_at = Some(now);
            // Completed by hand: it no longer waits to reopen.
            task.reopens_on = None;
            // Sub-tasks complete with their parent, repeating ones too.
            for mut child in descendants(repo, &task.id)? {
                if !child.checked {
                    child.checked = true;
                    child.completed_at = Some(now);
                    child.updated_at = now;
                    repo.upsert_task(&child)?;
                }
            }
        }
    }
    task.updated_at = now;
    repo.upsert_task(&task)?;
    Ok(true)
}

/// After `id` completed: completes each parent above it that is done with
/// its sub-tasks and now has them all done. Returns those parents, nearest first.
fn complete_parents<R: Repository>(repo: &R, id: &Id, today: NaiveDate) -> Result<Vec<Id>, RepoError> {
    let mut completed = Vec::new();
    let mut child = id.clone();
    loop {
        let mut all = repo.list_all_tasks()?;
        // Sub-tasks whose day has come count as open, even before a read reopens them.
        for t in all.iter_mut() {
            t.reopen_if_due(today);
        }
        let Some(parent) = all.iter().find(|t| t.id == child).and_then(|c| c.parent_id.clone()) else { break };
        let Some(p) = all.iter().find(|t| t.id == parent) else { break };
        if !p.done_with_subtasks || views::done_for_now(p, today) || !views::all_done(&parent, &all, today) {
            break;
        }
        let ready = match p.due.as_ref().filter(|d| d.is_recurring) {
            // A repeating parent waits for its date.
            Some(d) => due_date(d).is_some_and(|date| date <= today),
            // Completing a one-off parent ends its sub-tasks' repeats: not while one still runs.
            None => !views::children(&parent, &all).iter().any(|c| views::repeats(c) && !views::is_done(c, today)),
        };
        if !ready || !complete_task(repo, &parent, today)? {
            break;
        }
        completed.push(parent.clone());
        child = parent;
    }
    Ok(completed)
}

/// Reopens a task (see `Action::TaskUncomplete`).
fn uncomplete_task<R: Repository>(repo: &R, id: &Id, today: NaiveDate) -> Result<(), RepoError> {
    let Some(mut task) = repo.get_task(id)? else { return Ok(()) };
    // A repeating task was never checked: put back what completing it changed.
    if let Some(undo) = streak::undoable_completion(&task, today) {
        if let Some(d) = task.due.as_mut() {
            d.date = undo.date;
            d.moved_from = undo.moved_from;
        }
        task.streak_current = undo.streak_current;
        task.streak_best = undo.streak_best;
        task.streak_last_period = undo.streak_last_period;
    }
    task.last_completion = None;
    task.reopens_on = None;
    task.checked = false;
    task.completed_at = None;
    task.updated_at = Utc::now();
    repo.upsert_task(&task)?;
    Ok(())
}

/// Every task below `id` (children, grandchildren, …), non-deleted.
fn descendants<R: Repository>(repo: &R, id: &Id) -> Result<Vec<Task>, RepoError> {
    let all = repo.list_all_tasks()?;
    let mut out = Vec::new();
    let mut frontier = vec![id.clone()];
    while let Some(parent) = frontier.pop() {
        for t in all.iter().filter(|t| t.parent_id.as_ref() == Some(&parent)) {
            if !out.iter().any(|o: &Task| o.id == t.id) && t.id != *id {
                frontier.push(t.id.clone());
                out.push(t.clone());
            }
        }
    }
    Ok(out)
}

/// Moves a task and its whole sub-tree to `project_id` / `section_id`.
fn move_subtree<R: Repository>(
    repo: &R,
    id: &Id,
    project_id: &Id,
    section_id: Option<&Id>,
) -> Result<(), RepoError> {
    let Some(root) = repo.get_task(id)? else { return Ok(()) };
    let now = Utc::now();
    for mut t in std::iter::once(root).chain(descendants(repo, id)?) {
        t.project_id = project_id.clone();
        t.section_id = section_id.cloned();
        t.updated_at = now;
        repo.upsert_task(&t)?;
    }
    Ok(())
}

fn finish<R: Repository>(
    repo: &R,
    uuid: &str,
    type_name: &str,
    args_json: &str,
    result: ApplyResult,
) -> Result<ApplyResult, RepoError> {
    repo.record_command(uuid, type_name, args_json)?;
    Ok(result)
}

/// Credits the occurrence being completed and reschedules past it. Returns the
/// last date that's now behind the task, or `None` when that occurrence was
/// already credited.
fn complete_occurrence(
    task: &mut Task,
    rule: &Recurrence,
    current: NaiveDate,
    today: NaiveDate,
) -> Option<NaiveDate> {
    let occurrence = task
        .due
        .as_ref()
        .and_then(moved_from)
        .unwrap_or_else(|| completed_occurrence(rule, current, today));
    if already_credited(task, occurrence) {
        return None;
    }
    let passed = occurrence.max(current);
    let mut undo = None;
    if let Some(d) = task.due.as_mut() {
        let suffix = d.date.get(10..).unwrap_or("").to_string();
        let before = std::mem::replace(&mut d.date, format!("{}{}", iso(rule.next_after(passed)), suffix));
        let was_moved = d.moved_from.take();
        undo = Some(LastCompletion {
            moved_to: d.date.clone(),
            date: before,
            streak_current: task.streak_current,
            streak_best: task.streak_best,
            streak_last_period: task.streak_last_period.clone(),
            moved_from: was_moved,
            completed_on: Some(iso(today)),
        });
    }
    task.last_completion = undo;
    streak::credit_completion(task, occurrence);
    Some(passed)
}

/// Which occurrence a completion on `today` is for: today's when today is a
/// scheduled day; the immediately upcoming one when completing early on an
/// unscheduled day; otherwise today (late — the missed one is not rescued).
fn completed_occurrence(rule: &Recurrence, due: NaiveDate, today: NaiveDate) -> NaiveDate {
    if rule.contains(today) {
        today
    } else if due == rule.next_after(today) {
        due
    } else {
        today
    }
}

/// True when a streak-tracked task has already been credited for `occurrence`
/// (or a later one), so a repeat completion must not reschedule or count again.
fn already_credited(task: &Task, occurrence: NaiveDate) -> bool {
    task.streak_enabled
        && task
            .streak_last_period
            .as_deref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .is_some_and(|last| occurrence <= last)
}

#[cfg(test)]
mod tests {
    use super::{Action, Command};
    use crate::ids::Id;

    /// Pins the IPC wire contract the frontend must produce for `dispatch`.
    #[test]
    fn task_add_command_wire_shape() {
        let cmd = Command {
            uuid: "u".to_string(),
            temp_id: Some("tmp-1".to_string()),
            action: Action::TaskAdd {
                content: "x".to_string(),
                project_id: Id::from("P1"),
                description: None,
                priority: None,
                due: None,
                labels: None,
                parent_id: None,
                section_id: None,
                recurrence: None,
                deadline: None,
            },
        };
        let v: serde_json::Value = serde_json::to_value(&cmd).unwrap();
        assert_eq!(v["uuid"], "u");
        assert_eq!(v["temp_id"], "tmp-1");
        assert_eq!(v["action"]["type"], "task_add");
        assert_eq!(v["action"]["content"], "x");
        assert_eq!(v["action"]["project_id"], "P1");

        // And it round-trips back to a TaskAdd.
        let back: Command = serde_json::from_value(v).unwrap();
        assert!(matches!(back.action, Action::TaskAdd { .. }));
    }
}
