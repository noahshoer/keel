//! Platform-free application service: owns the local store, seeds the Inbox,
//! and exposes dispatch/query operations. The Tauri layer is a thin wrapper
//! over this.

use todo_core::commands::{apply, ApplyResult, Command};
use todo_core::ids::Id;
use todo_core::model::{Project, Task};
use todo_core::repository::{RepoError, Repository};
use todo_storage::SqliteRepo;

pub struct AppService {
    repo: SqliteRepo,
}

impl AppService {
    /// Open (and migrate) the store at `path`, seeding a default Inbox on an
    /// empty database. Idempotent across reopen (seeds only when no projects).
    pub fn open(path: &str) -> Result<Self, RepoError> {
        let repo = SqliteRepo::open(path)?;
        let svc = AppService { repo };
        svc.seed_inbox_if_empty()?;
        Ok(svc)
    }

    fn seed_inbox_if_empty(&self) -> Result<(), RepoError> {
        if self.repo.list_projects()?.is_empty() {
            self.repo.upsert_project(&Project::inbox())?;
        }
        Ok(())
    }

    pub fn dispatch(&self, cmd: Command) -> Result<ApplyResult, RepoError> {
        apply(&self.repo, cmd)
    }

    pub fn list_tasks(&self, project_id: &str) -> Result<Vec<Task>, RepoError> {
        self.repo.list_tasks(&Id::from(project_id))
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, RepoError> {
        self.repo.list_projects()
    }
}
