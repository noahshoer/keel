use std::collections::HashMap;

use chrono::{Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::model::{Due, Label, Priority, Project, Task};
use crate::recurrence::Recurrence;
use crate::repository::{RepoError, Repository};
use crate::streak;

/// The result of applying a command: any temp-id → real-id mappings created.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplyResult {
    pub id_mapping: HashMap<String, Id>,
}

/// A single mutation. Carries a client `uuid` (idempotency) and, for creates,
/// a `temp_id` that later commands/UI can resolve via `ApplyResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct DueEdit {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub recurrence: Option<Recurrence>,
}

/// The operations supported so far. (update/move/uncomplete land with their
/// features.)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    },
    TaskComplete {
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
        goal: Option<i64>,
    },
    /// Create a label. Names are unique case-insensitively; re-adding an
    /// existing name maps `temp_id` to the existing label.
    LabelAdd {
        name: String,
    },
    /// Soft-delete a label and strip its name from every task.
    LabelDelete {
        id: Id,
    },
    ProjectAdd {
        name: String,
    },
    ProjectDelete {
        id: Id,
    },
}

impl Action {
    fn type_name(&self) -> &'static str {
        match self {
            Action::TaskAdd { .. } => "task_add",
            Action::TaskUpdate { .. } => "task_update",
            Action::TaskComplete { .. } => "task_complete",
            Action::TaskDelete { .. } => "task_delete",
            Action::TaskSetStreak { .. } => "task_set_streak",
            Action::LabelAdd { .. } => "label_add",
            Action::LabelDelete { .. } => "label_delete",
            Action::ProjectAdd { .. } => "project_add",
            Action::ProjectDelete { .. } => "project_delete",
        }
    }
}

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Human text for a rule, stored in `Due.string` (Todoist-style).
fn describe(rule: &Recurrence) -> String {
    match rule {
        Recurrence::Daily => "every day".to_string(),
        Recurrence::Weekly(days) => {
            let names: Vec<&str> = days
                .iter()
                .filter_map(|d| WEEKDAYS.get(*d as usize).copied())
                .collect();
            format!("every {}", names.join(", "))
        }
        Recurrence::MonthlyDay(day) => format!("every month on the {day}"),
    }
}

/// Builds a `Due` from an optional date and rule. With only a rule, the first
/// occurrence on/after `today` is used; with neither, there is no due.
fn build_due(date: Option<String>, rule: Option<Recurrence>, today: NaiveDate) -> Option<Due> {
    let (date, string) = match (&date, &rule) {
        (None, None) => return None,
        (Some(d), None) => (d.clone(), d.clone()),
        (Some(d), Some(r)) => (d.clone(), describe(r)),
        (None, Some(r)) => {
            let first = if r.contains(today) { today } else { r.next_after(today) };
            (iso(first), describe(r))
        }
    };
    Some(Due {
        date,
        timezone: None,
        string,
        lang: "en".to_string(),
        is_recurring: rule.is_some(),
        recurrence: rule,
    })
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
        } => {
            let mut task = Task::new(content, project_id);
            if let Some(d) = description {
                task.description = d;
            }
            if let Some(p) = priority {
                task.priority = p;
            }
            task.due = match (due, recurrence) {
                (due, None) => due,
                (due, rule) => build_due(due.map(|d| d.date), rule, today),
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
            if let Some(mut task) = repo.get_task(&id)? {
                let now = Utc::now();
                let recurring = task
                    .due
                    .as_ref()
                    .and_then(|d| d.recurrence.clone().zip(due_date(d)));
                match recurring {
                    // Recurring: credit the occurrence being completed, then
                    // reschedule past it; the task stays active. Re-completing
                    // an already-credited occurrence is a no-op.
                    Some((rule, current)) => {
                        let occurrence = completed_occurrence(&rule, current, today);
                        if already_credited(&task, occurrence) {
                            return finish(repo, &cmd.uuid, type_name, &args_json, result);
                        }
                        let next = rule.next_after(occurrence.max(current));
                        if let Some(d) = task.due.as_mut() {
                            let suffix = d.date.get(10..).unwrap_or("").to_string();
                            d.date = format!("{}{}", iso(next), suffix);
                        }
                        streak::credit_completion(&mut task, occurrence);
                    }
                    None => {
                        task.checked = true;
                        task.completed_at = Some(now);
                    }
                }
                task.updated_at = now;
                repo.upsert_task(&task)?;
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
        } => {
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
                if let Some(p) = project_id {
                    task.project_id = p;
                }
                if let Some(l) = labels {
                    task.labels = l;
                }
                if let Some(edit) = due {
                    let old_rule = task.due.as_ref().and_then(|d| d.recurrence.clone());
                    task.due = build_due(edit.date, edit.recurrence, today);
                    let new_rule = task.due.as_ref().and_then(|d| d.recurrence.clone());
                    // A different schedule invalidates the occurrence history.
                    if new_rule != old_rule {
                        task.streak_current = 0;
                        task.streak_last_period = None;
                    }
                }
                task.updated_at = Utc::now();
                repo.upsert_task(&task)?;
            }
        }
        Action::TaskDelete { id } => {
            repo.soft_delete_task(&id)?;
        }
        Action::TaskSetStreak { id, enabled, goal } => {
            if let Some(mut task) = repo.get_task(&id)? {
                task.streak_enabled = enabled;
                task.streak_goal = goal;
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
        Action::ProjectDelete { id } => {
            if let Some(mut project) = repo.get_project(&id)? {
                project.is_deleted = true;
                project.updated_at = Utc::now();
                repo.upsert_project(&project)?;
            }
            for task in repo.list_tasks(&id)? {
                repo.soft_delete_task(&task.id)?;
            }
        }
    }

    finish(repo, &cmd.uuid, type_name, &args_json, result)
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
