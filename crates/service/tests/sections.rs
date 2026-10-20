use keel_core::commands::{Action, Command};
use keel_service::AppService;

#[test]
fn lists_sections_for_project() {
    let dir = tempfile::tempdir().unwrap();
    let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
    let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;
    for (i, name) in ["Next", "Later"].iter().enumerate() {
        svc.dispatch(Command {
            uuid: format!("s{i}"),
            temp_id: None,
            action: Action::SectionAdd { project_id: inbox.clone(), name: name.to_string() },
        })
        .unwrap();
    }
    let names: Vec<String> = svc.list_sections(inbox.as_str()).unwrap().into_iter().map(|s| s.name).collect();
    assert_eq!(names, vec!["Next", "Later"]);
}
