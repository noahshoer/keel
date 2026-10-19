use std::fmt;

use crate::ids::Id;
use crate::model::{Label, Project, Section, Task};

/// Errors a `Repository` can return.
#[derive(Debug)]
pub enum RepoError {
    Backend(String),
}

impl fmt::Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoError::Backend(m) => write!(f, "storage backend error: {m}"),
        }
    }
}

impl std::error::Error for RepoError {}

/// Storage-agnostic persistence boundary. The domain and command layers
/// depend only on this; the concrete store (SQLite now, libSQL at sync time)
/// lives behind it.
pub trait Repository {
    fn upsert_task(&self, task: &Task) -> Result<(), RepoError>;
    fn get_task(&self, id: &Id) -> Result<Option<Task>, RepoError>;
    /// Active (non-deleted) tasks in a project.
    fn list_tasks(&self, project: &Id) -> Result<Vec<Task>, RepoError>;
    /// Active (non-deleted) tasks across every project.
    fn list_all_tasks(&self) -> Result<Vec<Task>, RepoError>;
    /// Soft-delete a task and (soft-delete) its sub-tasks.
    fn soft_delete_task(&self, id: &Id) -> Result<(), RepoError>;

    fn upsert_project(&self, project: &Project) -> Result<(), RepoError>;
    fn get_project(&self, id: &Id) -> Result<Option<Project>, RepoError>;
    /// Active (non-deleted) projects.
    fn list_projects(&self) -> Result<Vec<Project>, RepoError>;

    fn upsert_section(&self, section: &Section) -> Result<(), RepoError>;
    fn get_section(&self, id: &Id) -> Result<Option<Section>, RepoError>;
    /// Active sections of a project, in display order.
    fn list_sections(&self, project: &Id) -> Result<Vec<Section>, RepoError>;

    fn upsert_label(&self, label: &Label) -> Result<(), RepoError>;
    /// Active (non-deleted) labels, sorted by name case-insensitively.
    fn list_labels(&self) -> Result<Vec<Label>, RepoError>;

    /// Command-log idempotency support.
    fn has_command(&self, uuid: &str) -> Result<bool, RepoError>;
    fn record_command(&self, uuid: &str, ctype: &str, args_json: &str) -> Result<(), RepoError>;
}
