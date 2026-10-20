use keel_service::AppService;

const EXPORT: &str = "TYPE,CONTENT,DESCRIPTION,PRIORITY,INDENT,AUTHOR,RESPONSIBLE,DATE,DATE_LANG,TIMEZONE,DURATION,DURATION_UNIT,DEADLINE,DEADLINE_LANG\n\
task,Pay rent @home,,1,1,Noah (1),,every month on the 1st,en,,,None,,\n\
task,Plan trip,Flights,2,1,Noah (1),,2026-10-30,en,,,None,2026-11-15,en\n\
note,Ask Sam,,,,Noah (1),,,,,,,,\n\
task,Book flights @travel,,4,2,Noah (1),,,,,,None,,\n\
,,,,,,,,,,,,,\n\
section,Someday,,,,,,,,,,,,\n\
task,Learn piano,,4,1,Noah (1),,every! 3 weeks,en,,,None,,\n";

fn open() -> (tempfile::TempDir, AppService) {
    let dir = tempfile::tempdir().unwrap();
    let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
    (dir, svc)
}

#[test]
fn imports_a_project_with_sections_nesting_and_labels() {
    let (_dir, svc) = open();
    svc.dispatch(keel_core::commands::Command {
        uuid: "pre".into(),
        temp_id: None,
        action: keel_core::commands::Action::LabelAdd { name: "Home".into() },
    })
    .unwrap();

    let summary = svc.import_todoist_csv("Travel", EXPORT).unwrap();
    assert_eq!(summary.project_name, "Travel");
    assert_eq!((summary.tasks, summary.sections, summary.comments, summary.kept_as_text), (4, 1, 1, 1));
    assert_eq!(summary.labels_created, 1, "@home already exists (case-insensitive); @travel is new");

    let project = svc.list_projects().unwrap().into_iter().find(|p| p.name == "Travel").unwrap();
    assert_eq!(summary.project_id, project.id);
    let tasks = svc.project_view(project.id.as_str()).unwrap();
    let get = |name: &str| tasks.iter().find(|t| t.content == name).unwrap_or_else(|| panic!("{name}"));
    let trip = get("Plan trip");
    assert_eq!(get("Book flights").parent_id.as_ref(), Some(&trip.id));
    assert_eq!(get("Book flights").labels, vec!["travel"]);
    assert_eq!(trip.deadline.as_deref(), Some("2026-11-15"));
    assert_eq!(trip.description, "Flights\n\nComments:\n- Ask Sam");
    assert!(get("Pay rent").due.as_ref().is_some_and(|d| d.is_recurring));

    let sections = svc.list_sections(project.id.as_str()).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(get("Learn piano").section_id.as_ref(), Some(&sections[0].id));
    assert_eq!(get("Learn piano").description, "Todoist date: every! 3 weeks");
}

#[test]
fn not_a_todoist_file_creates_nothing() {
    let (_dir, svc) = open();
    let before = svc.list_projects().unwrap().len();
    let err = svc.import_todoist_csv("Junk", "a,b\n1,2\n").unwrap_err();
    assert!(err.contains("Todoist"), "{err}");
    assert_eq!(svc.list_projects().unwrap().len(), before);
}
