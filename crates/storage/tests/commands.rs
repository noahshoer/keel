use todo_core::commands::{apply, Action, Command};
use todo_core::ids::Id;
use todo_core::repository::Repository;
use todo_storage::SqliteRepo;

fn add_task_cmd(uuid: &str, temp: &str, content: &str, project: &Id) -> Command {
    Command {
        uuid: uuid.to_string(),
        temp_id: Some(temp.to_string()),
        action: Action::TaskAdd {
            content: content.to_string(),
            project_id: project.clone(),
            description: None,
            priority: None,
            due: None,
            labels: None,
            parent_id: None,
            section_id: None,
        },
    }
}

#[test]
fn task_add_creates_and_maps_temp_id() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let project = Id::new();
    let res = apply(&repo, add_task_cmd("u1", "tmp-1", "hello", &project)).unwrap();
    let new_id = res.id_mapping.get("tmp-1").expect("temp id mapped to real id");
    let task = repo.get_task(new_id).unwrap().expect("task persisted");
    assert_eq!(task.content, "hello");
    assert_eq!(task.project_id, project);
}

#[test]
fn duplicate_uuid_is_noop() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let project = Id::new();
    let cmd = add_task_cmd("dup", "tmp-x", "once", &project);
    apply(&repo, cmd.clone()).unwrap();
    apply(&repo, cmd).unwrap(); // same uuid again
    let tasks = repo.list_tasks(&project).unwrap();
    assert_eq!(tasks.len(), 1, "same command uuid must not add a second task");
}

#[test]
fn task_complete_sets_checked_and_completed_at() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let project = Id::new();
    let res = apply(&repo, add_task_cmd("a", "tmp-a", "do it", &project)).unwrap();
    let id = res.id_mapping.get("tmp-a").unwrap().clone();

    apply(
        &repo,
        Command {
            uuid: "c".to_string(),
            temp_id: None,
            action: Action::TaskComplete { id: id.clone() },
        },
    )
    .unwrap();

    let task = repo.get_task(&id).unwrap().unwrap();
    assert!(task.checked, "task should be checked");
    assert!(task.completed_at.is_some(), "completed_at should be set");
}

#[test]
fn delete_project_soft_deletes_its_tasks() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    // Create a project via command so we have its id.
    let res = apply(
        &repo,
        Command {
            uuid: "p1".to_string(),
            temp_id: Some("tmp-p".to_string()),
            action: Action::ProjectAdd {
                name: "Chores".to_string(),
            },
        },
    )
    .unwrap();
    let project = res.id_mapping.get("tmp-p").unwrap().clone();

    apply(&repo, add_task_cmd("t1", "tmp-1", "one", &project)).unwrap();
    apply(&repo, add_task_cmd("t2", "tmp-2", "two", &project)).unwrap();
    assert_eq!(repo.list_tasks(&project).unwrap().len(), 2);

    apply(
        &repo,
        Command {
            uuid: "d".to_string(),
            temp_id: None,
            action: Action::ProjectDelete {
                id: project.clone(),
            },
        },
    )
    .unwrap();

    assert!(
        repo.list_tasks(&project).unwrap().is_empty(),
        "project's tasks are soft-deleted"
    );
    assert!(
        !repo.list_projects().unwrap().iter().any(|p| p.id == project),
        "deleted project is excluded"
    );
}

#[test]
fn delete_task_soft_deletes_subtasks() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let project = Id::new();
    let res = apply(&repo, add_task_cmd("pa", "tmp-parent", "parent", &project)).unwrap();
    let parent = res.id_mapping.get("tmp-parent").unwrap().clone();

    // A sub-task under the parent.
    apply(
        &repo,
        Command {
            uuid: "ch".to_string(),
            temp_id: Some("tmp-child".to_string()),
            action: Action::TaskAdd {
                content: "child".to_string(),
                project_id: project.clone(),
                description: None,
                priority: None,
                due: None,
                labels: None,
                parent_id: Some(parent.clone()),
                section_id: None,
            },
        },
    )
    .unwrap();
    assert_eq!(repo.list_tasks(&project).unwrap().len(), 2);

    apply(
        &repo,
        Command {
            uuid: "dt".to_string(),
            temp_id: None,
            action: Action::TaskDelete { id: parent },
        },
    )
    .unwrap();

    assert!(
        repo.list_tasks(&project).unwrap().is_empty(),
        "parent and its sub-task are both soft-deleted"
    );
}
