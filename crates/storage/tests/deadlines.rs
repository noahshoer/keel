use chrono::NaiveDate;
use keel_core::commands::{apply_at, Action, Command};
use keel_core::ids::Id;
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

    fn add(&mut self, deadline: Option<&str>) -> Id {
        self.run(Action::TaskAdd {
            content: "Report".into(),
            project_id: Id::from("P1"),
            description: None,
            priority: None,
            due: None,
            labels: None,
            parent_id: None,
            section_id: None,
            recurrence: None,
            deadline: deadline.map(String::from),
        })
        .id_mapping["t"]
            .clone()
    }

    fn set_deadline(&mut self, id: &Id, deadline: Option<&str>) {
        self.run(Action::TaskUpdate {
            id: id.clone(),
            content: None,
            description: None,
            priority: None,
            project_id: None,
            labels: None,
            due: None,
            is_collapsed: None,
            deadline: deadline.map(String::from),
        });
    }

    fn deadline(&self, id: &Id) -> Option<String> {
        self.repo.get_task(id).unwrap().unwrap().deadline
    }
}

#[test]
fn add_with_deadline() {
    let mut h = H::new();
    let id = h.add(Some("2026-10-30"));
    assert_eq!(h.deadline(&id).as_deref(), Some("2026-10-30"));
    let none = h.add(None);
    assert_eq!(h.deadline(&none), None);
}

#[test]
fn update_sets_keeps_and_clears() {
    let mut h = H::new();
    let id = h.add(None);
    h.set_deadline(&id, Some("2026-11-02"));
    assert_eq!(h.deadline(&id).as_deref(), Some("2026-11-02"));
    h.set_deadline(&id, None);
    assert_eq!(h.deadline(&id).as_deref(), Some("2026-11-02"), "None leaves it alone");
    h.set_deadline(&id, Some(""));
    assert_eq!(h.deadline(&id), None, "empty string clears it");
}

#[test]
fn invalid_deadline_is_ignored() {
    let mut h = H::new();
    let id = h.add(Some("someday"));
    assert_eq!(h.deadline(&id), None);
}
