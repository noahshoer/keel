//! Shared setup for the integration tests: one temp store, the service on it,
//! and the few ways the tests add and read tasks.
//!
//! Each test file is its own binary and uses only part of this, so unused
//! items are expected; this is the one place that allows `dead_code`.
#![allow(dead_code)]

use chrono::NaiveDate;
use keel_core::commands::{Action, ApplyResult, Command};
use keel_core::ids::Id;
use keel_core::model::{Due, Priority, Task};
use keel_core::recurrence::Recurrence;
use keel_service::AppService;

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// A temp store with the service open on it. The directory lives as long as
/// the fixture. Commands go in through `run` (on `today`) or `run_on` (on a
/// given day); each gets its own uuid and the temp id "t", so a created id
/// is `result.id_mapping["t"]`.
pub struct Fx {
    pub dir: tempfile::TempDir,
    pub path: String,
    pub svc: AppService,
    pub inbox: Id,
    /// The day `run`, `add_task`, `complete`, `get` and friends act on.
    pub today: NaiveDate,
    n: u32,
}

impl Fx {
    /// A fresh store whose `today` is the real local day.
    pub fn new() -> Self {
        Self::on(chrono::Local::now().date_naive())
    }

    /// A fresh store whose `today` is `today`.
    pub fn on(today: NaiveDate) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keel.db").to_str().unwrap().to_string();
        let svc = AppService::open(&path).unwrap();
        let inbox = svc
            .list_projects()
            .unwrap()
            .into_iter()
            .find(|p| p.is_inbox)
            .unwrap()
            .id;
        Fx {
            dir,
            path,
            svc,
            inbox,
            today,
            n: 0,
        }
    }

    /// Opens the store again, so a test can check what reached the file.
    pub fn reopen(&mut self) {
        let old = std::mem::replace(&mut self.svc, AppService::open(&self.path).unwrap());
        drop(old);
    }

    /// Dispatches with an explicit command uuid (send the same one twice to
    /// test idempotency).
    pub fn send(&mut self, uuid: &str, action: Action, on: NaiveDate) -> ApplyResult {
        let cmd = Command {
            uuid: uuid.to_string(),
            temp_id: Some("t".into()),
            action,
        };
        self.svc.dispatch_at(cmd, on).unwrap()
    }

    pub fn run_on(&mut self, action: Action, on: NaiveDate) -> ApplyResult {
        self.n += 1;
        let uuid = format!("u{}", self.n);
        self.send(&uuid, action, on)
    }

    pub fn run(&mut self, action: Action) -> ApplyResult {
        self.run_on(action, self.today)
    }

    pub fn add_task(&mut self, task: NewTask) -> Id {
        let action = Action::TaskAdd {
            content: task.content,
            project_id: task.project.unwrap_or_else(|| self.inbox.clone()),
            description: task.description,
            priority: task.priority,
            due: task.due,
            labels: task.labels,
            parent_id: task.parent,
            section_id: None,
            recurrence: task.recurrence,
            deadline: None,
        };
        self.run(action).id_mapping["t"].clone()
    }

    /// An Inbox task named "t", under `parent` if given, repeating if given.
    pub fn add_under(&mut self, parent: Option<&Id>, recurrence: Option<Recurrence>) -> Id {
        let mut task = NewTask::new("t");
        task.parent = parent.cloned();
        task.recurrence = recurrence;
        self.add_task(task)
    }

    pub fn streak(&mut self, id: &Id) {
        self.run(Action::TaskSetStreak {
            id: id.clone(),
            enabled: true,
            goal: None,
        });
    }

    pub fn complete(&mut self, id: &Id) -> ApplyResult {
        self.run(Action::TaskComplete { id: id.clone() })
    }

    /// The task as `all_tasks_at(today)` reads it.
    pub fn get(&self, id: &Id) -> Task {
        self.svc
            .all_tasks_at(self.today)
            .unwrap()
            .into_iter()
            .find(|t| &t.id == id)
            .unwrap()
    }

    /// The task's due date, as the text it is stored in.
    pub fn date(&self, id: &Id) -> String {
        self.get(id).due.unwrap().date
    }
}

/// What a test adds: an Inbox task with just a name, plus whatever the
/// builder methods set.
pub struct NewTask {
    content: String,
    project: Option<Id>,
    parent: Option<Id>,
    recurrence: Option<Recurrence>,
    due: Option<Due>,
    labels: Option<Vec<String>>,
    priority: Option<Priority>,
    description: Option<String>,
}

impl NewTask {
    pub fn new(content: &str) -> Self {
        NewTask {
            content: content.to_string(),
            project: None,
            parent: None,
            recurrence: None,
            due: None,
            labels: None,
            priority: None,
            description: None,
        }
    }

    pub fn project(mut self, project: &Id) -> Self {
        self.project = Some(project.clone());
        self
    }

    pub fn parent(mut self, parent: &Id) -> Self {
        self.parent = Some(parent.clone());
        self
    }

    pub fn recurrence(mut self, recurrence: Recurrence) -> Self {
        self.recurrence = Some(recurrence);
        self
    }

    pub fn due(mut self, due: Option<Due>) -> Self {
        self.due = due;
        self
    }

    pub fn labels(mut self, labels: &[&str]) -> Self {
        self.labels = Some(labels.iter().map(|l| l.to_string()).collect());
        self
    }

    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
}
