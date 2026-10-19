use todo_core::commands::{apply, Action, ApplyResult, Command};
use todo_core::repository::Repository;
use todo_storage::SqliteRepo;

fn run(repo: &SqliteRepo, uuid: &str, action: Action) -> ApplyResult {
    apply(repo, Command { uuid: uuid.into(), temp_id: Some("t".into()), action }).unwrap()
}

#[test]
fn add_update_delete_filters() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let first = run(&repo, "a", Action::FilterAdd { name: "Focus".into(), query: "today & p1".into() }).id_mapping["t"].clone();
    let second = run(&repo, "b", Action::FilterAdd { name: "Errands".into(), query: "@errands".into() }).id_mapping["t"].clone();

    run(&repo, "c", Action::FilterUpdate { id: first.clone(), name: Some(" Deep work ".into()), query: None });
    run(&repo, "d", Action::FilterUpdate { id: first.clone(), name: None, query: Some("today | overdue".into()) });
    let filters = repo.list_filters().unwrap();
    assert_eq!(filters.len(), 2);
    assert_eq!(filters[0].id, first, "creation order kept");
    assert_eq!(filters[0].name, "Deep work");
    assert_eq!(filters[0].query, "today | overdue");

    run(&repo, "e", Action::FilterDelete { id: second });
    let filters = repo.list_filters().unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0].id, first);
}
