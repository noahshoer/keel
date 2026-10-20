use chrono::NaiveDate;
use keel_core::commands::{Action, Command};
use keel_core::model::{Due, Priority};
use keel_service::AppService;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()
}

struct F {
    _dir: tempfile::TempDir,
    svc: AppService,
    n: u32,
}

impl F {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
        F { _dir: dir, svc, n: 0 }
    }

    fn add(&mut self, name: &str, description: &str, due: Option<&str>, priority: Priority) -> String {
        self.n += 1;
        let inbox = self.svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;
        let action = Action::TaskAdd {
            content: name.into(),
            project_id: inbox,
            description: Some(description.into()),
            priority: Some(priority),
            due: due.map(|d| Due {
                date: d.into(),
                timezone: None,
                string: d.into(),
                lang: "en".into(),
                is_recurring: false,
                recurrence: None,
            }),
            labels: None,
            parent_id: None,
            section_id: None,
            recurrence: None,
            deadline: None,
        };
        let cmd = Command { uuid: format!("u{}", self.n), temp_id: Some("t".into()), action };
        self.svc.dispatch_at(cmd, today()).unwrap().id_mapping["t"].to_string()
    }

    fn complete(&mut self, id: &str) {
        self.n += 1;
        let cmd = Command {
            uuid: format!("u{}", self.n),
            temp_id: None,
            action: Action::TaskComplete { id: id.into() },
        };
        self.svc.dispatch_at(cmd, today()).unwrap();
    }
}

fn names(tasks: &[keel_core::model::Task]) -> Vec<&str> {
    tasks.iter().map(|t| t.content.as_str()).collect()
}

#[test]
fn title_matches_first_then_due_then_priority() {
    let mut f = F::new();
    f.add("Groceries", "milk and eggs", Some("2026-10-19"), Priority::None);
    f.add("Milk run later", "", Some("2026-10-25"), Priority::None);
    f.add("Buy milk", "", Some("2026-10-20"), Priority::None);
    f.add("Milk the goat", "", None, Priority::P1);
    f.add("Bread", "", Some("2026-10-19"), Priority::None);
    let r = f.svc.search_at("milk", today()).unwrap();
    assert_eq!(names(&r.open), vec!["Buy milk", "Milk run later", "Milk the goat", "Groceries"]);
    assert!(r.completed.is_empty());
}

#[test]
fn completed_matches_are_separate_and_empty_query_finds_nothing() {
    let mut f = F::new();
    let done = f.add("Call the bank", "", None, Priority::None);
    f.add("Bank statement", "", None, Priority::None);
    f.complete(&done);
    let r = f.svc.search_at("bank", today()).unwrap();
    assert_eq!(names(&r.open), vec!["Bank statement"]);
    assert_eq!(names(&r.completed), vec!["Call the bank"]);
    assert_eq!(f.svc.search_at("   ", today()).unwrap(), Default::default());
}
