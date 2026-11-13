use keel_core::commands::Action;

mod common;
use common::{Fx, NewTask};

#[test]
fn first_run_seeds_inbox() {
    let f = Fx::new();
    let projects = f.svc.list_projects().unwrap();
    assert_eq!(projects.len(), 1, "fresh db seeds exactly one project");
    assert!(projects[0].is_inbox, "the seeded project is the Inbox");
}

#[test]
fn dispatch_task_add_then_list_returns_it() {
    let mut f = Fx::new();
    f.add_task(NewTask::new("milk"));

    let tasks = f.svc.list_tasks(f.inbox.as_str()).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].content, "milk");
}

#[test]
fn seed_inbox_is_idempotent_across_reopen() {
    let mut f = Fx::new();
    assert_eq!(f.svc.list_projects().unwrap().len(), 1);
    // Reopening must not seed a second Inbox.
    f.reopen();
    assert_eq!(
        f.svc.list_projects().unwrap().len(),
        1,
        "no duplicate Inbox on reopen"
    );
}

#[test]
fn list_arrangements_returns_what_was_dispatched() {
    use keel_core::arrange::{Arrangement, PlacedArrangement, SortField, ViewPlace};
    let mut f = Fx::new();
    assert!(f.svc.list_arrangements().unwrap().is_empty());
    let arrangement = Arrangement {
        filter: "p1".into(),
        groups: vec![SortField::Priority],
        order: vec![],
    };
    f.run(Action::ViewArrange {
        place: ViewPlace::Today,
        arrangement: arrangement.clone(),
    });
    assert_eq!(
        f.svc.list_arrangements().unwrap(),
        vec![PlacedArrangement {
            place: ViewPlace::Today,
            arrangement
        }]
    );
}
