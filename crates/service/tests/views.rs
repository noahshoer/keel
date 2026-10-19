use chrono::NaiveDate;
use todo_core::commands::{Action, Command};
use todo_core::ids::Id;
use todo_core::model::{Due, Priority};
use todo_service::AppService;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

const TODAY: (i32, u32, u32) = (2026, 10, 19);

fn today() -> NaiveDate {
    d(TODAY.0, TODAY.1, TODAY.2)
}

struct Fixture {
    _dir: tempfile::TempDir,
    svc: AppService,
    inbox: Id,
    n: u32,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let svc = AppService::open(dir.path().join("todo.db").to_str().unwrap()).unwrap();
        let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;
        Fixture { _dir: dir, svc, inbox, n: 0 }
    }

    fn uuid(&mut self) -> String {
        self.n += 1;
        format!("u{}", self.n)
    }

    fn project(&mut self, name: &str) -> Id {
        let u = self.uuid();
        let res = self
            .svc
            .dispatch_at(
                Command { uuid: u, temp_id: Some("p".into()), action: Action::ProjectAdd { name: name.into() } },
                today(),
            )
            .unwrap();
        res.id_mapping["p"].clone()
    }

    fn task(&mut self, content: &str, project: &Id, due: Option<&str>, priority: Priority) -> Id {
        let u = self.uuid();
        let due = due.map(|date| Due {
            date: date.to_string(),
            timezone: None,
            string: date.to_string(),
            lang: "en".into(),
            is_recurring: false,
            recurrence: None,
        });
        let res = self
            .svc
            .dispatch_at(
                Command {
                    uuid: u,
                    temp_id: Some("t".into()),
                    action: Action::TaskAdd {
                        content: content.into(),
                        project_id: project.clone(),
                        description: None,
                        priority: Some(priority),
                        due,
                        labels: None,
                        parent_id: None,
                        section_id: None,
                        recurrence: None,
                    },
                },
                today(),
            )
            .unwrap();
        res.id_mapping["t"].clone()
    }

    fn complete(&mut self, id: &Id) {
        let u = self.uuid();
        self.svc
            .dispatch_at(Command { uuid: u, temp_id: None, action: Action::TaskComplete { id: id.clone() } }, today())
            .unwrap();
    }
}

fn names(tasks: Vec<todo_core::model::Task>) -> Vec<String> {
    tasks.into_iter().map(|t| t.content).collect()
}

#[test]
fn today_spans_projects_and_includes_overdue() {
    let mut f = Fixture::new();
    let work = f.project("Work");
    let inbox = f.inbox.clone();
    f.task("today-low", &inbox, Some("2026-10-19"), Priority::None);
    f.task("today-p1", &work, Some("2026-10-19"), Priority::P1);
    f.task("overdue", &work, Some("2026-10-17"), Priority::None);
    f.task("tomorrow", &inbox, Some("2026-10-20"), Priority::P1);

    assert_eq!(
        names(f.svc.today_view_at(today()).unwrap()),
        vec!["overdue", "today-p1", "today-low"],
        "overdue first, then by priority; spans projects; excludes tomorrow"
    );
}

#[test]
fn undated_only_in_project_view() {
    let mut f = Fixture::new();
    let inbox = f.inbox.clone();
    f.task("someday", &inbox, None, Priority::None);
    assert!(f.svc.today_view_at(today()).unwrap().is_empty());
    assert!(f.svc.upcoming_view_at(today(), 7).unwrap().is_empty());
    assert_eq!(names(f.svc.project_view_at(inbox.as_str(), today()).unwrap()), vec!["someday"]);
}

#[test]
fn completed_hidden_everywhere() {
    let mut f = Fixture::new();
    let inbox = f.inbox.clone();
    let id = f.task("done", &inbox, Some("2026-10-19"), Priority::None);
    f.complete(&id);
    assert!(f.svc.today_view_at(today()).unwrap().is_empty());
    assert!(f.svc.upcoming_view_at(today(), 7).unwrap().is_empty());
    assert!(f.svc.project_view_at(inbox.as_str(), today()).unwrap().is_empty());
}

#[test]
fn upcoming_window_bounds() {
    let mut f = Fixture::new();
    let inbox = f.inbox.clone();
    f.task("yesterday", &inbox, Some("2026-10-18"), Priority::None);
    f.task("today", &inbox, Some("2026-10-19"), Priority::None);
    f.task("day7", &inbox, Some("2026-10-25"), Priority::None);
    f.task("day8", &inbox, Some("2026-10-26"), Priority::None);
    assert_eq!(names(f.svc.upcoming_view_at(today(), 7).unwrap()), vec!["today", "day7"]);
}
