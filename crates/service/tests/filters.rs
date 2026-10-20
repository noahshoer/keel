use chrono::NaiveDate;
use keel_core::commands::{Action, Command};
use keel_core::ids::Id;
use keel_core::model::{Due, Priority};
use keel_service::AppService;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()
}

struct F {
    _dir: tempfile::TempDir,
    svc: AppService,
    inbox: Id,
    n: u32,
}

impl F {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
        let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;
        F { _dir: dir, svc, inbox, n: 0 }
    }

    fn run(&mut self, action: Action) -> keel_core::commands::ApplyResult {
        self.n += 1;
        self.svc
            .dispatch_at(Command { uuid: format!("u{}", self.n), temp_id: Some("t".into()), action }, today())
            .unwrap()
    }

    fn task(&mut self, name: &str, due: Option<&str>, priority: Priority, labels: &[&str]) -> Id {
        let inbox = self.inbox.clone();
        self.run(Action::TaskAdd {
            content: name.into(),
            project_id: inbox,
            description: None,
            priority: Some(priority),
            due: due.map(|d| Due {
                date: d.into(),
                timezone: None,
                string: d.into(),
                lang: "en".into(),
                is_recurring: false,
                recurrence: None,
            }),
            labels: Some(labels.iter().map(|s| s.to_string()).collect()),
            parent_id: None,
            section_id: None,
            recurrence: None,
        })
        .id_mapping["t"]
            .clone()
    }
}

fn names(tasks: &[keel_core::model::Task]) -> Vec<String> {
    tasks.iter().map(|t| t.content.clone()).collect()
}

#[test]
fn groups_in_order_and_open_only() {
    let mut f = F::new();
    f.task("a-today", Some("2026-10-19"), Priority::P1, &[]);
    f.task("b-overdue", Some("2026-10-10"), Priority::None, &[]);
    f.task("c-home", None, Priority::None, &["home"]);
    let done = f.task("d-done-today", Some("2026-10-19"), Priority::None, &[]);
    f.run(Action::TaskComplete { id: done });

    let groups = f.svc.filter_view_at("today, overdue | @home", today()).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].query, "today");
    assert_eq!(names(&groups[0].tasks), vec!["a-today"], "completed tasks never appear");
    assert_eq!(groups[1].query, "overdue | @home");
    assert_eq!(names(&groups[1].tasks), vec!["b-overdue", "c-home"]);
}

#[test]
fn invalid_query_errors() {
    let f = F::new();
    assert!(f.svc.filter_view_at("today &", today()).is_err());
    assert!(f.svc.check_filter("today &").is_err());
    assert!(f.svc.check_filter("#Inbox & p1").is_ok());
}

#[test]
fn sorted_by_due_then_priority() {
    let mut f = F::new();
    f.task("x-20-p4", Some("2026-10-20"), Priority::None, &[]);
    f.task("y-20-p1", Some("2026-10-20"), Priority::P1, &[]);
    f.task("z-19", Some("2026-10-19"), Priority::None, &[]);
    f.task("w-undated", None, Priority::P1, &[]);
    let groups = f.svc.filter_view_at("all", today()).unwrap();
    assert_eq!(names(&groups[0].tasks), vec!["z-19", "y-20-p1", "x-20-p4", "w-undated"]);
}
