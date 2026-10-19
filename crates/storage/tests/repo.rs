use todo_core::ids::Id;
use todo_core::model::{Due, Task};
use todo_core::repository::Repository;
use todo_storage::SqliteRepo;

fn temp_db() -> (tempfile::TempDir, SqliteRepo) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.db");
    let repo = SqliteRepo::open(path.to_str().unwrap()).expect("open");
    (dir, repo)
}

#[test]
fn migrations_are_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.db");
    let p = path.to_str().unwrap();
    // Opening twice on the same file must both succeed (migrations re-run safely).
    let r1 = SqliteRepo::open(p).expect("first open runs migrations");
    drop(r1);
    let r2 = SqliteRepo::open(p).expect("second open re-runs migrations without error");
    // And the schema is usable after a re-run.
    assert!(r2.list_projects().unwrap().is_empty());
}

#[test]
fn task_roundtrip() {
    let (_d, repo) = temp_db();
    let project = Id::new();
    let task = Task::new("buy milk", project.clone());
    repo.upsert_task(&task).unwrap();
    let got = repo.get_task(&task.id).unwrap().expect("task present");
    assert_eq!(got, task);
}

#[test]
fn long_content_roundtrips() {
    let (_d, repo) = temp_db();
    let project = Id::new();
    let big = "x".repeat(10_000);
    let task = Task::new(big.clone(), project);
    repo.upsert_task(&task).unwrap();
    let got = repo.get_task(&task.id).unwrap().unwrap();
    assert_eq!(got.content, big);
    assert_eq!(got.content.len(), 10_000);
}

#[test]
fn floating_due_stored_as_null_tz() {
    let (_d, repo) = temp_db();
    let project = Id::new();
    let mut task = Task::new("float", project);
    task.due = Some(Due {
        date: "2026-10-19".to_string(),
        timezone: None,
        string: "today".to_string(),
        lang: "en".to_string(),
        is_recurring: false,
        rrule: None,
    });
    repo.upsert_task(&task).unwrap();
    let got = repo.get_task(&task.id).unwrap().unwrap();
    assert_eq!(got.due.unwrap().timezone, None);
}

#[test]
fn list_tasks_filters_by_project_and_excludes_deleted() {
    let (_d, repo) = temp_db();
    let proj_a = Id::new();
    let proj_b = Id::new();
    let t1 = Task::new("a1", proj_a.clone());
    let t2 = Task::new("a2", proj_a.clone());
    let t3 = Task::new("b1", proj_b.clone());
    repo.upsert_task(&t1).unwrap();
    repo.upsert_task(&t2).unwrap();
    repo.upsert_task(&t3).unwrap();

    repo.soft_delete_task(&t2.id).unwrap();

    let a = repo.list_tasks(&proj_a).unwrap();
    assert_eq!(a.len(), 1, "only the non-deleted task in project A");
    assert_eq!(a[0].id, t1.id);

    let b = repo.list_tasks(&proj_b).unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].id, t3.id);
}
