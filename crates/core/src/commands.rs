use std::collections::HashMap;

use chrono::{Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::model::{Due, Priority, Project, Task};
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
            Action::TaskComplete { .. } => "task_complete",
            Action::TaskDelete { .. } => "task_delete",
            Action::TaskSetStreak { .. } => "task_set_streak",
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
                (Some(mut d), Some(rule)) => {
                    d.is_recurring = true;
                    d.recurrence = Some(rule);
                    Some(d)
                }
                (None, Some(rule)) => {
                    let first = if rule.contains(today) { today } else { rule.next_after(today) };
                    Some(Due {
                        date: iso(first),
                        timezone: None,
                        string: describe(&rule),
                        lang: "en".to_string(),
                        is_recurring: true,
                        recurrence: Some(rule),
                    })
                }
                (due, None) => due,
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
                    // Recurring: reschedule to the next occurrence after
                    // max(due, today) and credit the streak; stays active.
                    Some((rule, current)) => {
                        let next = rule.next_after(current.max(today));
                        if let Some(d) = task.due.as_mut() {
                            let suffix = d.date.get(10..).unwrap_or("").to_string();
                            d.date = format!("{}{}", iso(next), suffix);
                        }
                        streak::credit_completion(&mut task, today);
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

    repo.record_command(&cmd.uuid, type_name, &args_json)?;
    Ok(result)
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
