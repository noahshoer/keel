use chrono::NaiveDate;
use keel_core::commands::{apply_at, Action, Command};
use keel_core::ids::Id;
use keel_core::model::ReminderType;
use keel_core::repository::Repository;
use keel_storage::SqliteRepo;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()
}

struct H {
    repo: SqliteRepo,
    n: u32,
}

impl H {
    fn new() -> Self {
        H { repo: SqliteRepo::open_in_memory().unwrap(), n: 0 }
    }

    fn run(&mut self, action: Action) -> keel_core::commands::ApplyResult {
        self.n += 1;
        apply_at(&self.repo, Command { uuid: format!("u{}", self.n), temp_id: Some("t".into()), action }, today()).unwrap()
    }

    fn task(&mut self, parent: Option<&Id>) -> Id {
        self.run(Action::TaskAdd {
            content: "x".into(),
            project_id: Id::from("P1"),
            description: None,
            priority: None,
            due: None,
            labels: None,
            parent_id: parent.cloned(),
            section_id: None,
            recurrence: None,
            deadline: None,
        })
        .id_mapping["t"]
            .clone()
    }

    fn remind(&mut self, task: &Id, minute_offset: Option<i64>, at: Option<&str>) -> Option<Id> {
        self.run(Action::ReminderAdd { task_id: task.clone(), minute_offset, at: at.map(String::from) })
            .id_mapping
            .get("t")
            .cloned()
    }
}

#[test]
fn add_relative_and_absolute_then_delete() {
    let mut h = H::new();
    let t = h.task(None);
    let rel = h.remind(&t, Some(30), None).unwrap();
    let abs = h.remind(&t, None, Some("2026-10-21T09:00")).unwrap();
    let all = h.repo.list_reminders().unwrap();
    assert_eq!(all.len(), 2);
    let r = all.iter().find(|r| r.id == rel).unwrap();
    assert_eq!((r.kind, r.minute_offset), (ReminderType::Relative, Some(30)));
    let a = all.iter().find(|r| r.id == abs).unwrap();
    assert_eq!(a.kind, ReminderType::Absolute);
    assert_eq!(a.at().unwrap().to_string(), "2026-10-21 09:00:00");

    h.run(Action::ReminderDelete { id: rel });
    let left: Vec<Id> = h.repo.list_reminders().unwrap().into_iter().map(|r| r.id).collect();
    assert_eq!(left, vec![abs]);
}

#[test]
fn invalid_reminders_are_not_added() {
    let mut h = H::new();
    let t = h.task(None);
    assert_eq!(h.remind(&t, None, None), None, "neither offset nor time");
    assert_eq!(h.remind(&t, None, Some("tomorrow-ish")), None);
    assert_eq!(h.remind(&Id::from("missing"), Some(10), None), None, "unknown task");
    assert!(h.repo.list_reminders().unwrap().is_empty());
}

#[test]
fn deleting_a_task_deletes_its_and_subtask_reminders() {
    let mut h = H::new();
    let parent = h.task(None);
    let child = h.task(Some(&parent));
    let other = h.task(None);
    h.remind(&parent, Some(0), None);
    h.remind(&child, Some(10), None);
    let keep = h.remind(&other, Some(5), None).unwrap();
    h.run(Action::TaskDelete { id: parent });
    let left: Vec<Id> = h.repo.list_reminders().unwrap().into_iter().map(|r| r.id).collect();
    assert_eq!(left, vec![keep]);
}
