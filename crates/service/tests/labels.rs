use chrono::NaiveDate;
use keel_core::commands::Action;
use keel_core::ids::Id;

mod common;
use common::{d, Fx, NewTask};

fn today() -> NaiveDate {
    d(2026, 10, 19)
}

#[test]
fn label_view_spans_projects_and_hides_completed() {
    let mut f = Fx::on(today());
    let other = Id::from("OTHER");

    f.run(Action::LabelAdd {
        name: "home".into(),
    });
    assert_eq!(f.svc.list_labels().unwrap().len(), 1);

    f.add_task(NewTask::new("dishes").labels(&["home"]));
    f.add_task(NewTask::new("laundry").project(&other).labels(&["Home"]));
    f.add_task(NewTask::new("report").labels(&["work"]));
    let done = f.add_task(NewTask::new("vacuum").labels(&["home"]));
    f.complete(&done);

    let mut names: Vec<String> = f
        .svc
        .label_view_at("home", today())
        .unwrap()
        .into_iter()
        .map(|t| t.content)
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["dishes", "laundry"],
        "case-insensitive, all projects, no completed"
    );
}
