use todo_core::commands::{apply, Action, Command};
use todo_core::ids::Id;
use todo_core::repository::Repository;
use todo_storage::SqliteRepo;

fn run(repo: &SqliteRepo, uuid: &str, temp: Option<&str>, action: Action) -> todo_core::commands::ApplyResult {
    apply(repo, Command { uuid: uuid.into(), temp_id: temp.map(str::to_string), action }).unwrap()
}

fn label_names(repo: &SqliteRepo) -> Vec<String> {
    repo.list_labels().unwrap().into_iter().map(|l| l.name).collect()
}

#[test]
fn add_and_list_labels() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    run(&repo, "a", Some("x"), Action::LabelAdd { name: "work".into() });
    run(&repo, "b", Some("y"), Action::LabelAdd { name: "Home".into() });
    assert_eq!(label_names(&repo), vec!["Home", "work"], "sorted by name, case-insensitively");
}

#[test]
fn duplicate_name_case_insensitive_is_noop() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let first = run(&repo, "a", Some("x"), Action::LabelAdd { name: "Home".into() });
    let second = run(&repo, "b", Some("y"), Action::LabelAdd { name: "  home ".into() });
    assert_eq!(label_names(&repo), vec!["Home"]);
    assert_eq!(second.id_mapping["y"], first.id_mapping["x"], "temp id maps to the existing label");
}

#[test]
fn delete_label_strips_it_from_tasks() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let label = run(&repo, "a", Some("l"), Action::LabelAdd { name: "home".into() }).id_mapping["l"].clone();
    let task = run(
        &repo,
        "b",
        Some("t"),
        Action::TaskAdd {
            content: "dishes".into(),
            project_id: Id::new(),
            description: None,
            priority: None,
            due: None,
            labels: Some(vec!["home".into(), "chores".into()]),
            parent_id: None,
            section_id: None,
            recurrence: None,
        },
    )
    .id_mapping["t"]
        .clone();

    run(&repo, "c", None, Action::LabelDelete { id: label });

    assert!(label_names(&repo).is_empty());
    assert_eq!(repo.get_task(&task).unwrap().unwrap().labels, vec!["chores".to_string()]);
}
