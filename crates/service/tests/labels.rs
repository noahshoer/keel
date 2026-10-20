use chrono::NaiveDate;
use keel_core::commands::{Action, Command};
use keel_core::ids::Id;
use keel_service::AppService;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()
}

fn add(svc: &AppService, n: &mut u32, content: &str, project: &Id, labels: &[&str]) -> Id {
    *n += 1;
    let res = svc
        .dispatch_at(
            Command {
                uuid: format!("u{n}"),
                temp_id: Some("t".into()),
                action: Action::TaskAdd {
                    content: content.into(),
                    project_id: project.clone(),
                    description: None,
                    priority: None,
                    due: None,
                    labels: Some(labels.iter().map(|s| s.to_string()).collect()),
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

#[test]
fn label_view_spans_projects_and_hides_completed() {
    let dir = tempfile::tempdir().unwrap();
    let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
    let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;
    let other = Id::from("OTHER");
    let mut n = 0;

    svc.dispatch_at(
        Command { uuid: "lbl".into(), temp_id: None, action: Action::LabelAdd { name: "home".into() } },
        today(),
    )
    .unwrap();
    assert_eq!(svc.list_labels().unwrap().len(), 1);

    add(&svc, &mut n, "dishes", &inbox, &["home"]);
    add(&svc, &mut n, "laundry", &other, &["Home"]);
    add(&svc, &mut n, "report", &inbox, &["work"]);
    let done = add(&svc, &mut n, "vacuum", &inbox, &["home"]);
    n += 1;
    svc.dispatch_at(
        Command { uuid: format!("u{n}"), temp_id: None, action: Action::TaskComplete { id: done } },
        today(),
    )
    .unwrap();

    let mut names: Vec<String> =
        svc.label_view_at("home", today()).unwrap().into_iter().map(|t| t.content).collect();
    names.sort();
    assert_eq!(names, vec!["dishes", "laundry"], "case-insensitive, all projects, no completed");
}
