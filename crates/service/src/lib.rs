//! Platform-free application service: owns the local store, seeds the Inbox,
//! and exposes dispatch/query operations. The Tauri layer is a thin wrapper
//! over this.

use chrono::{Local, NaiveDate};
use todo_core::commands::{apply_at, ApplyResult, Command};
use todo_core::ids::Id;
use todo_core::model::{Project, Task};
use todo_core::repository::{RepoError, Repository};
use todo_core::streak;
use todo_storage::SqliteRepo;

pub struct AppService {
    repo: SqliteRepo,
}

fn today() -> NaiveDate {
    Local::now().date_naive()
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
        self.dispatch_at(cmd, today())
    }

    /// `dispatch` with an explicit date (deterministic for tests).
    pub fn dispatch_at(&self, cmd: Command, today: NaiveDate) -> Result<ApplyResult, RepoError> {
        apply_at(&self.repo, cmd, today)
    }

    pub fn list_tasks(&self, project_id: &str) -> Result<Vec<Task>, RepoError> {
        self.list_tasks_at(project_id, today())
    }

    /// Lists a project's tasks, first applying lazy streak-miss detection to
    /// streak-enabled tasks and persisting any reset (design §7).
    pub fn list_tasks_at(&self, project_id: &str, today: NaiveDate) -> Result<Vec<Task>, RepoError> {
        let mut tasks = self.repo.list_tasks(&Id::from(project_id))?;
        for task in tasks.iter_mut().filter(|t| t.streak_enabled) {
            let before = task.streak_current;
            streak::recompute_miss(task, today);
            if task.streak_current != before {
                self.repo.upsert_task(task)?;
            }
        }
        Ok(tasks)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, RepoError> {
        self.repo.list_projects()
    }
}
