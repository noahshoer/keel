use chrono::{Duration, Local};
use keel_core::ids::Id;

mod common;
use common::{Fx, NewTask};

impl Fx {
    fn task(&mut self, content: &str, project: &Id, labels: &[&str]) -> Id {
        self.add_task(NewTask::new(content).project(project).labels(labels))
    }

    /// Completes with a pause first, so completion timestamps are distinct
    /// and newest-first ordering is deterministic.
    fn complete_later(&mut self, id: &Id) {
        std::thread::sleep(std::time::Duration::from_millis(3));
        self.complete(id);
    }
}

fn names(tasks: Vec<keel_core::model::Task>) -> Vec<String> {
    tasks.into_iter().map(|t| t.content).collect()
}

#[test]
fn project_completed_newest_first() {
    let mut f = Fx::new();
    let inbox = f.inbox.clone();
    let a = f.task("a", &inbox, &[]);
    f.task("b-open", &inbox, &[]);
    let c = f.task("c", &inbox, &[]);
    f.complete_later(&a);
    f.complete_later(&c);
    assert_eq!(
        names(f.svc.completed_in_project(inbox.as_str()).unwrap()),
        vec!["c", "a"]
    );
}

#[test]
fn label_completed() {
    let mut f = Fx::new();
    let inbox = f.inbox.clone();
    let other = Id::from("OTHER");
    let x = f.task("x", &inbox, &["home"]);
    let y = f.task("y", &other, &["Home"]);
    f.task("z-open", &inbox, &["home"]);
    f.complete_later(&x);
    f.complete_later(&y);
    let mut got = names(f.svc.completed_with_label("HOME").unwrap());
    got.sort();
    assert_eq!(got, vec!["x", "y"]);
}

#[test]
fn completed_today_only_today() {
    let mut f = Fx::new();
    let inbox = f.inbox.clone();
    let id = f.task("done", &inbox, &[]);
    f.complete_later(&id);
    let today = Local::now().date_naive();
    assert_eq!(
        names(f.svc.completed_today_at(today).unwrap()),
        vec!["done"]
    );
    assert!(f
        .svc
        .completed_today_at(today + Duration::days(1))
        .unwrap()
        .is_empty());
}
