use keel_core::arrange::{Arrangement, OrderLevel, SortField, ViewPlace};
use keel_core::commands::{apply, Action, Command};
use keel_core::ids::Id;
use keel_core::repository::Repository;
use keel_storage::SqliteRepo;

fn arrange(repo: &SqliteRepo, uuid: &str, place: ViewPlace, arrangement: Arrangement) {
    apply(
        repo,
        Command {
            uuid: uuid.into(),
            temp_id: None,
            action: Action::ViewArrange { place, arrangement },
        },
    )
    .unwrap();
}

fn level(field: SortField) -> OrderLevel {
    OrderLevel {
        field,
        reversed: false,
    }
}

fn by_priority() -> Arrangement {
    Arrangement {
        order: vec![level(SortField::Priority)],
        ..Arrangement::default()
    }
}

#[test]
fn view_arrange_stores_and_lists() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let groups = vec![SortField::Project, SortField::Section, SortField::Label];
    let order = vec![level(SortField::DueDate), level(SortField::Priority)];
    arrange(
        &repo,
        "a",
        ViewPlace::Today,
        Arrangement {
            filter: " p1 ".into(),
            groups: groups.clone(),
            order: order.clone(),
        },
    );
    let all = repo.list_arrangements().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].place, ViewPlace::Today);
    assert_eq!(all[0].arrangement.filter, "p1");
    assert_eq!(all[0].arrangement.groups, groups);
    assert_eq!(all[0].arrangement.order, order);
}

#[test]
fn view_arrange_replaces() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    arrange(&repo, "a", ViewPlace::Today, by_priority());
    let newer = Arrangement {
        filter: "overdue".into(),
        ..Arrangement::default()
    };
    arrange(&repo, "b", ViewPlace::Today, newer.clone());
    let all = repo.list_arrangements().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].arrangement, newer);
}

#[test]
fn default_arrangement_removes_the_row() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    arrange(&repo, "a", ViewPlace::Today, by_priority());
    arrange(&repo, "b", ViewPlace::Today, Arrangement::default());
    assert!(repo.list_arrangements().unwrap().is_empty());
}

#[test]
fn replayed_command_is_ignored() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let first = Command {
        uuid: "same".into(),
        temp_id: None,
        action: Action::ViewArrange {
            place: ViewPlace::Today,
            arrangement: by_priority(),
        },
    };
    apply(&repo, first.clone()).unwrap();
    let between = Arrangement {
        filter: "p1".into(),
        ..Arrangement::default()
    };
    arrange(&repo, "other", ViewPlace::Today, between.clone());
    apply(&repo, first).unwrap();
    let all = repo.list_arrangements().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].arrangement, between);
}

#[test]
fn places_are_separate() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let id = Id::from("a".to_string());
    arrange(
        &repo,
        "a",
        ViewPlace::Project { id: id.clone() },
        by_priority(),
    );
    arrange(&repo, "b", ViewPlace::Label { id }, by_priority());
    assert_eq!(repo.list_arrangements().unwrap().len(), 2);
}

#[test]
fn repeated_fields_keep_the_first() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    arrange(
        &repo,
        "a",
        ViewPlace::Upcoming,
        Arrangement {
            groups: vec![SortField::Label, SortField::Project, SortField::Label],
            order: vec![
                level(SortField::Priority),
                OrderLevel {
                    field: SortField::Priority,
                    reversed: true,
                },
            ],
            ..Arrangement::default()
        },
    );
    let a = &repo.list_arrangements().unwrap()[0].arrangement;
    assert_eq!(a.groups, vec![SortField::Label, SortField::Project]);
    assert_eq!(a.order, vec![level(SortField::Priority)]);
}

#[test]
fn unreadable_row_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keel.db");
    let path = path.to_str().unwrap();
    let repo = SqliteRepo::open(path).unwrap();
    arrange(&repo, "a", ViewPlace::Today, by_priority());
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "INSERT INTO view_arrangement (place_key, place_json, arrangement_json) \
             VALUES ('upcoming', '{\"kind\":\"upcoming\"}', '{\"groups\":[\"size\"]}')",
            [],
        )
        .unwrap();
    let all = repo.list_arrangements().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].place, ViewPlace::Today);
}

fn create(repo: &SqliteRepo, uuid: &str, action: Action) -> Id {
    let result = apply(
        repo,
        Command {
            uuid: uuid.into(),
            temp_id: Some("t".into()),
            action,
        },
    )
    .unwrap();
    result.id_mapping["t"].clone()
}

fn run(repo: &SqliteRepo, uuid: &str, action: Action) {
    apply(
        repo,
        Command {
            uuid: uuid.into(),
            temp_id: None,
            action,
        },
    )
    .unwrap();
}

#[test]
fn deleting_a_page_drops_its_arrangement() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let project = create(
        &repo,
        "p",
        Action::ProjectAdd {
            name: "Home".into(),
        },
    );
    let label = create(
        &repo,
        "l",
        Action::LabelAdd {
            name: "errand".into(),
        },
    );
    let filter = create(
        &repo,
        "f",
        Action::FilterAdd {
            name: "Urgent".into(),
            query: "p1".into(),
        },
    );
    arrange(
        &repo,
        "a1",
        ViewPlace::Project {
            id: project.clone(),
        },
        by_priority(),
    );
    arrange(
        &repo,
        "a2",
        ViewPlace::Label { id: label.clone() },
        by_priority(),
    );
    arrange(
        &repo,
        "a3",
        ViewPlace::Filter { id: filter.clone() },
        by_priority(),
    );
    arrange(&repo, "a4", ViewPlace::Today, by_priority());

    run(&repo, "d1", Action::ProjectDelete { id: project });
    run(&repo, "d2", Action::LabelDelete { id: label });
    run(&repo, "d3", Action::FilterDelete { id: filter });

    let left: Vec<ViewPlace> = repo
        .list_arrangements()
        .unwrap()
        .into_iter()
        .map(|p| p.place)
        .collect();
    assert_eq!(left, vec![ViewPlace::Today]);
}
