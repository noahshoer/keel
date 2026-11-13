use keel_core::commands::Action;

mod common;
use common::Fx;

#[test]
fn lists_sections_for_project() {
    let mut f = Fx::new();
    for name in ["Next", "Later"] {
        f.run(Action::SectionAdd {
            project_id: f.inbox.clone(),
            name: name.to_string(),
        });
    }
    let names: Vec<String> = f
        .svc
        .list_sections(f.inbox.as_str())
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, vec!["Next", "Later"]);
}
