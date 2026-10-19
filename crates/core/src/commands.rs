use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::model::{Due, Priority, Project, Task};
use crate::repository::{RepoError, Repository};

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

/// The operations supported in v1's foundation. (update/move/uncomplete land
/// with their features — see ledger R8.)
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
    },
    TaskComplete {
        id: Id,
    },
    TaskDelete {
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
            Action::TaskComplete { .. } => "task_complete",
            Action::TaskDelete { .. } => "task_delete",
            Action::ProjectAdd { .. } => "project_add",
            Action::ProjectDelete { .. } => "project_delete",
        }
    }
}

/// Apply a command to the repository. Idempotent by `uuid`: re-applying a
/// command whose uuid is already recorded is a no-op.
pub fn apply<R: Repository>(repo: &R, cmd: Command) -> Result<ApplyResult, RepoError> {
    if repo.has_command(&cmd.uuid)? {
        return Ok(ApplyResult::default());
    }

    let mut result = ApplyResult::default();
    let args_json = serde_json::to_string(&cmd.action).map_err(|e| RepoError::Backend(e.to_string()))?;
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
        } => {
            let mut task = Task::new(content, project_id);
            if let Some(d) = description {
                task.description = d;
            }
            if let Some(p) = priority {
                task.priority = p;
            }
            task.due = due;
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
                task.checked = true;
                task.completed_at = Some(now);
                task.updated_at = now;
                repo.upsert_task(&task)?;
            }
        }
        Action::TaskDelete { id } => {
            repo.soft_delete_task(&id)?;
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
