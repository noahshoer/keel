//! Platform-free application service: owns the local store, seeds the Inbox,
//! and exposes dispatch/query operations. The Tauri layer is a thin wrapper
//! over this.

pub mod diag;

use chrono::{Local, NaiveDate, NaiveDateTime};
use keel_core::commands::{apply_at, Action, ApplyResult, Command};
use keel_core::import::todoist::{self, ImportOp, ImportSummary};
use keel_core::ids::Id;
use keel_core::filter::{self, FilterContext, FilterError, FilterGroup};
use keel_core::model::{Due, Filter, Label, Project, Reminder, Section, Task};
use keel_core::reminders::{self, PlannedReminder, ReminderSettings};
use keel_core::repository::{RepoError, Repository};
use keel_core::search::{self, SearchResults};
use keel_core::streak;
use keel_core::views;
use keel_storage::SqliteRepo;

const REMINDER_SETTINGS_KEY: &str = "reminders";
const SCHEDULED_IDS_KEY: &str = "reminders_scheduled_ids";
const DIAG_KEY: &str = "diag_enabled";

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

    /// Every non-deleted task, open and checked, after streak recompute.
    pub fn all_tasks(&self) -> Result<Vec<Task>, RepoError> {
        self.all_tasks_at(today())
    }

    pub fn all_tasks_at(&self, today: NaiveDate) -> Result<Vec<Task>, RepoError> {
        let tasks = self.repo.list_all_tasks()?;
        self.recompute_streaks(tasks, today)
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

    /// Text search over open tasks (title matches first, then due date,
    /// then priority) plus the newest matching completed tasks.
    pub fn search(&self, query: &str) -> Result<SearchResults, RepoError> {
        self.search_at(query, today())
    }

    pub fn search_at(&self, query: &str, today: NaiveDate) -> Result<SearchResults, RepoError> {
        let words = search::words(query);
        if words.is_empty() {
            return Ok(SearchResults::default());
        }
        let mut open: Vec<Task> = self.open_tasks(today)?.into_iter().filter(|t| search::matches(t, &words)).collect();
        open.sort_by(|a, b| {
            let (da, db) = (views::due_date_of(a), views::due_date_of(b));
            search::rank(a, &words)
                .cmp(&search::rank(b, &words))
                .then(da.is_none().cmp(&db.is_none()))
                .then(da.cmp(&db))
                .then(b.priority.to_api().cmp(&a.priority.to_api()))
                .then(a.child_order.cmp(&b.child_order))
        });
        let mut completed = self.completed(|t| search::matches(t, &words))?;
        completed.truncate(50);
        Ok(SearchResults { open, completed })
    }

    /// Imports a Todoist project CSV export as a new project named
    /// `project_name`, through normal commands (so it lands in the command
    /// log). Nothing is created when the file isn't a Todoist export.
    pub fn import_todoist_csv(&self, project_name: &str, text: &str) -> Result<ImportSummary, String> {
        let items = todoist::parse(text).map_err(|e| e.to_string())?;
        let plan = todoist::plan(&items, today());
        let run = |action: Action| -> Result<Option<Id>, String> {
            let cmd = Command { uuid: Id::new().to_string(), temp_id: Some("new".to_string()), action };
            let result = self.dispatch(cmd).map_err(|e| e.to_string())?;
            Ok(result.id_mapping.get("new").cloned())
        };

        let name = match project_name.trim() {
            "" => "Imported from Todoist".to_string(),
            n => n.to_string(),
        };
        let project_id = run(Action::ProjectAdd { name: name.clone() })?.ok_or("The project wasn't created.")?;

        let existing = self.list_labels().map_err(|e| e.to_string())?;
        let mut labels_created = 0;
        for label in &plan.labels {
            if !existing.iter().any(|l| l.name.eq_ignore_ascii_case(label)) {
                run(Action::LabelAdd { name: label.clone() })?;
                labels_created += 1;
            }
        }

        // Ids of what each op created, so later ops can point at them.
        let mut ids: Vec<Option<Id>> = Vec::with_capacity(plan.ops.len());
        let (mut tasks, mut sections) = (0, 0);
        for op in &plan.ops {
            let id = match op {
                ImportOp::Section { name } => {
                    sections += 1;
                    run(Action::SectionAdd { project_id: project_id.clone(), name: name.clone() })?
                }
                ImportOp::Task { content, .. } if content.trim().is_empty() => None,
                ImportOp::Task { parent, section, content, description, priority, labels, due, recurrence, deadline } => {
                    tasks += 1;
                    let lookup = |key: &Option<usize>| key.and_then(|k| ids.get(k).cloned().flatten());
                    run(Action::TaskAdd {
                        content: content.clone(),
                        project_id: project_id.clone(),
                        description: (!description.is_empty()).then(|| description.clone()),
                        priority: Some(*priority),
                        due: due.as_ref().map(|date| Due {
                            date: date.clone(),
                            timezone: None,
                            string: date.clone(),
                            lang: "en".to_string(),
                            is_recurring: false,
                            recurrence: None,
                        }),
                        labels: (!labels.is_empty()).then(|| labels.clone()),
                        parent_id: lookup(parent),
                        section_id: lookup(section),
                        recurrence: recurrence.clone(),
                        deadline: deadline.clone(),
                    })?
                }
            };
            ids.push(id);
        }
        Ok(ImportSummary {
            project_id,
            project_name: name,
            tasks,
            sections,
            labels_created,
            comments: plan.comments,
            kept_as_text: plan.kept_as_text,
        })
    }

    pub fn list_filters(&self) -> Result<Vec<Filter>, RepoError> {
        self.repo.list_filters()
    }

    /// Validates a filter query without running it.
    pub fn check_filter(&self, query: &str) -> Result<(), FilterError> {
        filter::parse(query).map(|_| ())
    }

    pub fn filter_view(&self, query: &str) -> Result<Vec<FilterGroup>, RepoError> {
        self.filter_view_at(query, today())
    }

    /// Open tasks matching each comma group of `query`, sorted by due date
    /// (undated last) then priority.
    pub fn filter_view_at(&self, query: &str, today: NaiveDate) -> Result<Vec<FilterGroup>, RepoError> {
        let groups = filter::parse(query).map_err(|e| RepoError::Backend(format!("Invalid filter: {e}")))?;
        let projects = self.repo.list_projects()?;
        let mut sections = Vec::new();
        for p in &projects {
            sections.extend(self.repo.list_sections(&p.id)?);
        }
        let ctx = FilterContext {
            today,
            projects: projects.into_iter().map(|p| (p.id, p.name)).collect(),
            sections: sections.into_iter().map(|s| (s.id, s.name)).collect(),
        };
        let mut open = self.open_tasks(today)?;
        open.sort_by(|a, b| {
            let (da, db) = (views::due_date_of(a), views::due_date_of(b));
            // Dated before undated, earlier first.
            da.is_none()
                .cmp(&db.is_none())
                .then(da.cmp(&db))
                .then(b.priority.to_api().cmp(&a.priority.to_api()))
                .then(a.child_order.cmp(&b.child_order))
        });
        let texts = filter::group_texts(query);
        Ok(groups
            .iter()
            .enumerate()
            .map(|(i, expr)| FilterGroup {
                query: texts.get(i).cloned().unwrap_or_default(),
                tasks: open.iter().filter(|t| filter::matches(expr, t, &ctx)).cloned().collect(),
            })
            .collect())
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

    /// Notification preferences for this device (defaults when unset).
    pub fn reminder_settings(&self) -> Result<ReminderSettings, RepoError> {
        Ok(self
            .repo
            .get_setting(REMINDER_SETTINGS_KEY)?
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default())
    }

    pub fn set_reminder_settings(&self, settings: &ReminderSettings) -> Result<(), RepoError> {
        let json = serde_json::to_string(settings).map_err(|e| RepoError::Backend(e.to_string()))?;
        self.repo.set_setting(REMINDER_SETTINGS_KEY, &json)
    }

    /// Whether diagnostics logging is on for this device (off by default).
    pub fn diag_enabled(&self) -> Result<bool, RepoError> {
        Ok(self.repo.get_setting(DIAG_KEY)?.as_deref() == Some("1"))
    }

    pub fn set_diag_enabled(&self, on: bool) -> Result<(), RepoError> {
        self.repo.set_setting(DIAG_KEY, if on { "1" } else { "0" })
    }

    /// Ids of the notifications last handed to the OS (mobile), so the next
    /// re-plan can cancel them, even after a restart.
    pub fn scheduled_reminder_ids(&self) -> Result<Vec<i32>, RepoError> {
        Ok(self
            .repo
            .get_setting(SCHEDULED_IDS_KEY)?
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default())
    }

    pub fn set_scheduled_reminder_ids(&self, ids: &[i32]) -> Result<(), RepoError> {
        let json = serde_json::to_string(ids).map_err(|e| RepoError::Backend(e.to_string()))?;
        self.repo.set_setting(SCHEDULED_IDS_KEY, &json)
    }

    /// The notifications that should be pending now.
    pub fn reminder_plan(&self) -> Result<Vec<PlannedReminder>, RepoError> {
        self.reminder_plan_at(Local::now().naive_local())
    }

    pub fn reminder_plan_at(&self, now: NaiveDateTime) -> Result<Vec<PlannedReminder>, RepoError> {
        let tasks = self.open_tasks(now.date())?;
        Ok(reminders::plan(&tasks, &self.repo.list_reminders()?, &self.reminder_settings()?, now))
    }

    /// Every active task reminder (the UI shows them per task).
    pub fn list_reminders(&self) -> Result<Vec<Reminder>, RepoError> {
        self.repo.list_reminders()
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, RepoError> {
        self.repo.list_projects()
    }
}
