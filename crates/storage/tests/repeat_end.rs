use chrono::NaiveDate;
use keel_core::commands::{apply_at, Action, Command, DueEdit};
use keel_core::ids::Id;
use keel_core::model::{Due, Task};
use keel_core::recurrence::Recurrence;
use keel_core::repository::Repository;
use keel_storage::SqliteRepo;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

struct World {
    repo: SqliteRepo,
    n: u32,
}

impl World {
    fn new() -> Self {
        World { repo: SqliteRepo::open_in_memory().unwrap(), n: 0 }
    }

    fn run(&mut self, action: Action, today: NaiveDate) -> keel_core::commands::ApplyResult {
        self.n += 1;
        let cmd = Command { uuid: format!("u{}", self.n), temp_id: Some("tmp".to_string()), action };
        apply_at(&self.repo, cmd, today).unwrap()
    }

    /// A daily task from Oct 19 that ends on `until`.
    fn add_daily(&mut self, until: Option<&str>) -> Id {
        let due = Due {
            date: "2026-10-19".to_string(),
            timezone: None,
            string: String::new(),
            lang: "en".to_string(),
            is_recurring: true,
            recurrence: Some(Recurrence::Daily),
            until: until.map(str::to_string),
        };
        let action = Action::TaskAdd {
            content: "x".to_string(),
            project_id: Id::new(),
            description: None,
            priority: None,
            due: Some(due),
            labels: None,
            parent_id: None,
            section_id: None,
            recurrence: Some(Recurrence::Daily),
            deadline: None,
        };
        self.run(action, d(2026, 10, 19)).id_mapping["tmp"].clone()
    }

    fn get(&self, id: &Id) -> Task {
        self.repo.get_task(id).unwrap().unwrap()
    }

    fn complete(&mut self, id: &Id, today: NaiveDate) {
        self.run(Action::TaskComplete { id: id.clone() }, today);
    }

    fn edit_due(&mut self, id: &Id, edit: DueEdit) {
        let action = Action::TaskUpdate {
            id: id.clone(),
            content: None,
            description: None,
            priority: None,
            project_id: None,
            due: Some(edit),
            labels: None,
            is_collapsed: None,
            deadline: None,
        };
        self.run(action, d(2026, 10, 19));
    }
}

#[test]
fn the_end_is_saved_and_described() {
    let mut w = World::new();
    let id = w.add_daily(Some("2026-10-21"));
    let due = w.get(&id).due.unwrap();
    assert_eq!(due.until.as_deref(), Some("2026-10-21"));
    assert_eq!(due.string, "every day until Oct 21");
}

#[test]
fn completing_the_last_occurrence_finishes_the_task() {
    let mut w = World::new();
    let id = w.add_daily(Some("2026-10-21"));
    w.complete(&id, d(2026, 10, 19));
    w.complete(&id, d(2026, 10, 20));
    let t = w.get(&id);
    assert!(!t.checked);
    assert_eq!(t.due.unwrap().date, "2026-10-21");
    w.complete(&id, d(2026, 10, 21));
    let t = w.get(&id);
    assert!(t.checked, "no occurrence is left");
    assert_eq!(t.due.unwrap().date, "2026-10-21", "the date stays on the last one");
}

#[test]
fn a_repeat_without_an_end_goes_on() {
    let mut w = World::new();
    let id = w.add_daily(None);
    w.complete(&id, d(2026, 10, 19));
    assert!(!w.get(&id).checked);
}

#[test]
fn editing_keeps_sets_or_clears_the_end() {
    let mut w = World::new();
    let id = w.add_daily(Some("2026-10-21"));
    let edit = |date: &str, until: Option<&str>, rule: Recurrence| DueEdit {
        date: Some(date.to_string()),
        recurrence: Some(rule),
        until: until.map(str::to_string),
    };
    w.edit_due(&id, edit("2026-10-20", None, Recurrence::Daily));
    assert_eq!(w.get(&id).due.unwrap().until.as_deref(), Some("2026-10-21"), "same rule keeps it");
    w.edit_due(&id, edit("2026-10-20", Some("2026-11-30"), Recurrence::Daily));
    assert_eq!(w.get(&id).due.unwrap().until.as_deref(), Some("2026-11-30"));
    w.edit_due(&id, edit("2026-10-20", Some(""), Recurrence::Daily));
    assert_eq!(w.get(&id).due.unwrap().until, None, "empty clears it");
    w.edit_due(&id, edit("2026-10-20", Some("2026-11-30"), Recurrence::Daily));
    w.edit_due(&id, edit("2026-10-20", None, Recurrence::Weekly(vec![1])));
    assert_eq!(w.get(&id).due.unwrap().until, None, "a new rule starts without one");
}
