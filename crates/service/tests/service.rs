use todo_core::commands::{Action, Command};
use todo_service::AppService;

fn temp_service() -> (tempfile::TempDir, AppService) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.db");
    let svc = AppService::open(path.to_str().unwrap()).expect("open service");
    (dir, svc)
}

#[test]
fn first_run_seeds_inbox() {
    let (_d, svc) = temp_service();
    let projects = svc.list_projects().unwrap();
    assert_eq!(projects.len(), 1, "fresh db seeds exactly one project");
    assert!(projects[0].is_inbox, "the seeded project is the Inbox");
}

#[test]
fn dispatch_task_add_then_list_returns_it() {
    let (_d, svc) = temp_service();
    let inbox = svc
        .list_projects()
        .unwrap()
        .into_iter()
        .find(|p| p.is_inbox)
        .unwrap();

    svc.dispatch(Command {
        uuid: "u1".to_string(),
        temp_id: Some("tmp-1".to_string()),
        action: Action::TaskAdd {
            content: "milk".to_string(),
            project_id: inbox.id.clone(),
            description: None,
            priority: None,
            due: None,
            labels: None,
            parent_id: None,
            section_id: None,
        },
    })
    .unwrap();

    let tasks = svc.list_tasks(inbox.id.as_str()).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].content, "milk");
}

#[test]
fn seed_inbox_is_idempotent_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.db");
    let p = path.to_str().unwrap();
    {
        let svc = AppService::open(p).unwrap();
        assert_eq!(svc.list_projects().unwrap().len(), 1);
    }
    // Reopening must not seed a second Inbox.
    let svc2 = AppService::open(p).unwrap();
    assert_eq!(svc2.list_projects().unwrap().len(), 1, "no duplicate Inbox on reopen");
}
