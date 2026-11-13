use chrono::NaiveDate;
use keel_core::commands::Action;
use keel_core::ids::Id;
use keel_core::model::{Due, Priority};

mod common;
use common::{d, Fx, NewTask};

fn today() -> NaiveDate {
    d(2026, 10, 19)
}

impl Fx {
    fn task(&mut self, name: &str, due: Option<&str>, priority: Priority, labels: &[&str]) -> Id {
        let due = due.map(|d| Due {
            date: d.into(),
            timezone: None,
            string: d.into(),
            lang: "en".into(),
            is_recurring: false,
            recurrence: None,
            until: None,
            moved_from: None,
        });
        self.add_task(
            NewTask::new(name)
                .priority(priority)
                .due(due)
                .labels(labels),
        )
    }
}

fn names(tasks: &[keel_core::model::Task]) -> Vec<String> {
    tasks.iter().map(|t| t.content.clone()).collect()
}

#[test]
fn groups_in_order_and_open_only() {
    let mut f = Fx::on(today());
    f.task("a-today", Some("2026-10-19"), Priority::P1, &[]);
    f.task("b-overdue", Some("2026-10-10"), Priority::None, &[]);
    f.task("c-home", None, Priority::None, &["home"]);
    let done = f.task("d-done-today", Some("2026-10-19"), Priority::None, &[]);
    f.run(Action::TaskComplete { id: done });

    let groups = f
        .svc
        .filter_view_at("today, overdue | @home", today())
        .unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].query, "today");
    assert_eq!(
        names(&groups[0].tasks),
        vec!["a-today"],
        "completed tasks never appear"
    );
    assert_eq!(groups[1].query, "overdue | @home");
    assert_eq!(names(&groups[1].tasks), vec!["b-overdue", "c-home"]);
}

#[test]
fn invalid_query_errors() {
    let f = Fx::on(today());
    assert!(f.svc.filter_view_at("today &", today()).is_err());
    assert!(f.svc.check_filter("today &").is_err());
    assert!(f.svc.check_filter("#Inbox & p1").is_ok());
}

#[test]
fn sorted_by_due_then_priority() {
    let mut f = Fx::on(today());
    f.task("x-20-p4", Some("2026-10-20"), Priority::None, &[]);
    f.task("y-20-p1", Some("2026-10-20"), Priority::P1, &[]);
    f.task("z-19", Some("2026-10-19"), Priority::None, &[]);
    f.task("w-undated", None, Priority::P1, &[]);
    let groups = f.svc.filter_view_at("all", today()).unwrap();
    assert_eq!(
        names(&groups[0].tasks),
        vec!["z-19", "y-20-p1", "x-20-p4", "w-undated"]
    );
}

#[test]
fn saved_filter_with_deleted_project_still_runs() {
    let mut f = Fx::on(today());
    f.task("a-today", Some("2026-10-19"), Priority::None, &[]);
    f.task("b-today", Some("2026-10-19"), Priority::None, &[]);
    let groups = f.svc.filter_view_at("#Gone, today", today()).unwrap();
    assert_eq!(groups.len(), 2);
    assert!(groups[0].tasks.is_empty());
    assert_eq!(groups[1].tasks.len(), 2);
}
