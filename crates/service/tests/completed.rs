use chrono::{Duration, Local};
use todo_core::commands::{Action, Command};
use todo_core::ids::Id;
use todo_service::AppService;

struct F {
    _dir: tempfile::TempDir,
    svc: AppService,
    inbox: Id,
    n: u32,
}

impl F {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let svc = AppService::open(dir.path().join("todo.db").to_str().unwrap()).unwrap();
        let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;
        F { _dir: dir, svc, inbox, n: 0 }
    }

    fn run(&mut self, action: Action) -> todo_core::commands::ApplyResult {
        self.n += 1;
        self.svc
            .dispatch(Command { uuid: format!("u{}", self.n), temp_id: Some("t".into()), action })
            .unwrap()
    }

    fn task(&mut self, content: &str, project: &Id, labels: &[&str]) -> Id {
        self.run(Action::TaskAdd {
            content: content.into(),
            project_id: project.clone(),
            description: None,
            priority: None,
            due: None,
            labels: Some(labels.iter().map(|s| s.to_string()).collect()),
            parent_id: None,
            section_id: None,
            recurrence: None,
        })
        .id_mapping["t"]
            .clone()
    }

    fn complete(&mut self, id: &Id) {
        // Distinct completion timestamps so newest-first ordering is deterministic.
        std::thread::sleep(std::time::Duration::from_millis(3));
        self.run(Action::TaskComplete { id: id.clone() });
    }
}

fn names(tasks: Vec<todo_core::model::Task>) -> Vec<String> {
    tasks.into_iter().map(|t| t.content).collect()
}

#[test]
fn project_completed_newest_first() {
    let mut f = F::new();
    let inbox = f.inbox.clone();
    let a = f.task("a", &inbox, &[]);
    f.task("b-open", &inbox, &[]);
    let c = f.task("c", &inbox, &[]);
    f.complete(&a);
    f.complete(&c);
    assert_eq!(names(f.svc.completed_in_project(inbox.as_str()).unwrap()), vec!["c", "a"]);
}

#[test]
fn label_completed() {
    let mut f = F::new();
    let inbox = f.inbox.clone();
    let other = Id::from("OTHER");
    let x = f.task("x", &inbox, &["home"]);
    let y = f.task("y", &other, &["Home"]);
    f.task("z-open", &inbox, &["home"]);
    f.complete(&x);
    f.complete(&y);
    let mut got = names(f.svc.completed_with_label("HOME").unwrap());
    got.sort();
    assert_eq!(got, vec!["x", "y"]);
}

#[test]
fn completed_today_only_today() {
    let mut f = F::new();
    let inbox = f.inbox.clone();
    let id = f.task("done", &inbox, &[]);
    f.complete(&id);
    let today = Local::now().date_naive();
    assert_eq!(names(f.svc.completed_today_at(today).unwrap()), vec!["done"]);
    assert!(f.svc.completed_today_at(today + Duration::days(1)).unwrap().is_empty());
}
