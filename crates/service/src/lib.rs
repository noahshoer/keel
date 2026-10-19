//! Platform-free application service: owns the local store, seeds the Inbox,
//! and exposes dispatch/query operations. The Tauri layer is a thin wrapper
//! over this.

use chrono::{Local, NaiveDate};
use todo_core::commands::{apply_at, ApplyResult, Command};
use todo_core::ids::Id;
use todo_core::model::{Label, Project, Section, Task};
use todo_core::repository::{RepoError, Repository};
use todo_core::streak;
use todo_core::views;
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
        let tasks = self.repo.list_tasks(&Id::from(project_id))?;
        self.recompute_streaks(tasks, today)
    }

    fn recompute_streaks(&self, mut tasks: Vec<Task>, today: NaiveDate) -> Result<Vec<Task>, RepoError> {
        for task in tasks.iter_mut().filter(|t| t.streak_enabled) {
            let before = task.streak_current;
            streak::recompute_miss(task, today);
            if task.streak_current != before {
                self.repo.upsert_task(task)?;
            }
        }
        Ok(tasks)
    }

    /// Open (unchecked) tasks across all projects, after streak recompute.
    fn open_tasks(&self, today: NaiveDate) -> Result<Vec<Task>, RepoError> {
        let tasks = self.repo.list_all_tasks()?;
        Ok(self
            .recompute_streaks(tasks, today)?
            .into_iter()
            .filter(|t| !t.checked)
            .collect())
    }

    /// Dated tasks matching `keep`, sorted by due date then priority (high first).
    fn dated_view(
        &self,
        today: NaiveDate,
        keep: impl Fn(NaiveDate) -> bool,
    ) -> Result<Vec<Task>, RepoError> {
        let mut tasks: Vec<(NaiveDate, Task)> = self
            .open_tasks(today)?
            .into_iter()
            .filter_map(|t| views::due_date_of(&t).map(|d| (d, t)))
            .filter(|(d, _)| keep(*d))
            .collect();
        tasks.sort_by(|(da, a), (db, b)| {
            da.cmp(db)
                .then(b.priority.to_api().cmp(&a.priority.to_api()))
                .then(a.child_order.cmp(&b.child_order))
        });
        Ok(tasks.into_iter().map(|(_, t)| t).collect())
    }

    /// Today: open tasks due today or overdue, across every project.
    pub fn today_view(&self) -> Result<Vec<Task>, RepoError> {
        self.today_view_at(today())
    }

    pub fn today_view_at(&self, today: NaiveDate) -> Result<Vec<Task>, RepoError> {
        self.dated_view(today, |d| d <= today)
    }

    /// Upcoming: open tasks due in `[today, today + days)`.
    pub fn upcoming_view(&self, days: i64) -> Result<Vec<Task>, RepoError> {
        self.upcoming_view_at(today(), days)
    }

    pub fn upcoming_view_at(&self, today: NaiveDate, days: i64) -> Result<Vec<Task>, RepoError> {
        let end = today + chrono::Duration::days(days);
        self.dated_view(today, |d| d >= today && d < end)
    }

    /// A project's open tasks (dated or not), in manual order.
    pub fn project_view(&self, project_id: &str) -> Result<Vec<Task>, RepoError> {
        self.project_view_at(project_id, today())
    }

    pub fn project_view_at(&self, project_id: &str, today: NaiveDate) -> Result<Vec<Task>, RepoError> {
        Ok(self
            .list_tasks_at(project_id, today)?
            .into_iter()
            .filter(|t| !t.checked)
            .collect())
    }

    /// Completed (checked) tasks matching `keep`, newest first.
    fn completed(&self, keep: impl Fn(&Task) -> bool) -> Result<Vec<Task>, RepoError> {
        let mut tasks: Vec<Task> = self
            .repo
            .list_all_tasks()?
            .into_iter()
            .filter(|t| t.checked && keep(t))
            .collect();
        tasks.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
        Ok(tasks)
    }

    pub fn completed_in_project(&self, project_id: &str) -> Result<Vec<Task>, RepoError> {
        self.completed(|t| t.project_id.as_str() == project_id)
    }

    pub fn completed_with_label(&self, name: &str) -> Result<Vec<Task>, RepoError> {
        self.completed(|t| t.labels.iter().any(|l| l.eq_ignore_ascii_case(name)))
    }

    pub fn completed_today(&self) -> Result<Vec<Task>, RepoError> {
        self.completed_today_at(today())
    }

    /// Tasks completed on `today`'s local calendar day.
    pub fn completed_today_at(&self, today: NaiveDate) -> Result<Vec<Task>, RepoError> {
        self.completed(|t| {
            t.completed_at
                .is_some_and(|c| c.with_timezone(&Local).date_naive() == today)
        })
    }

    pub fn list_sections(&self, project_id: &str) -> Result<Vec<Section>, RepoError> {
        self.repo.list_sections(&Id::from(project_id))
    }

    pub fn list_labels(&self) -> Result<Vec<Label>, RepoError> {
        self.repo.list_labels()
    }

    /// Open tasks carrying `name` (case-insensitive), across every project.
    pub fn label_view(&self, name: &str) -> Result<Vec<Task>, RepoError> {
        self.label_view_at(name, today())
    }

    pub fn label_view_at(&self, name: &str, today: NaiveDate) -> Result<Vec<Task>, RepoError> {
        Ok(self
            .open_tasks(today)?
            .into_iter()
            .filter(|t| t.labels.iter().any(|l| l.eq_ignore_ascii_case(name)))
            .collect())
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, RepoError> {
        self.repo.list_projects()
    }
}
