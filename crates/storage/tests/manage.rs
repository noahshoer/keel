use keel_core::commands::{apply, Action, ApplyResult, Command};
use keel_core::ids::Id;
use keel_core::model::Project;
use keel_core::repository::Repository;
use keel_storage::SqliteRepo;

struct H {
    repo: SqliteRepo,
    n: u32,
}

impl H {
    fn new() -> Self {
        H { repo: SqliteRepo::open_in_memory().unwrap(), n: 0 }
    }

    fn run(&mut self, action: Action) -> ApplyResult {
        self.n += 1;
        apply(&self.repo, Command { uuid: format!("u{}", self.n), temp_id: Some("t".into()), action }).unwrap()
    }

    fn task(&mut self, labels: &[&str]) -> Id {
        self.run(Action::TaskAdd {
            content: "task".into(),
            project_id: Id::from("P"),
            description: None,
            priority: None,
            due: None,
            labels: Some(labels.iter().map(|s| s.to_string()).collect()),
            parent_id: None,
            section_id: None,
            recurrence: None,
            deadline: None,
        })
        .id_mapping["t"]
            .clone()
    }

    fn label(&mut self, name: &str) -> Id {
        self.run(Action::LabelAdd { name: name.into() }).id_mapping["t"].clone()
    }

    fn label_names(&self) -> Vec<String> {
        self.repo.list_labels().unwrap().into_iter().map(|l| l.name).collect()
    }

    fn labels_of(&self, id: &Id) -> Vec<String> {
        self.repo.get_task(id).unwrap().unwrap().labels
    }
}

#[test]
fn uncomplete_reopens_task() {
    let mut h = H::new();
    let id = h.task(&[]);
    h.run(Action::TaskComplete { id: id.clone() });
    assert!(h.repo.get_task(&id).unwrap().unwrap().checked);
    h.run(Action::TaskUncomplete { id: id.clone() });
    let t = h.repo.get_task(&id).unwrap().unwrap();
    assert!(!t.checked);
    assert_eq!(t.completed_at, None);
}

#[test]
fn rename_project() {
    let mut h = H::new();
    let id = h.run(Action::ProjectAdd { name: "Wrk".into() }).id_mapping["t"].clone();
    h.run(Action::ProjectUpdate { id: id.clone(), name: "  Work ".into() });
    assert_eq!(h.repo.get_project(&id).unwrap().unwrap().name, "Work");
}

#[test]
fn inbox_cannot_be_renamed_or_deleted() {
    let mut h = H::new();
    let inbox = Project::inbox();
    h.repo.upsert_project(&inbox).unwrap();
    h.run(Action::ProjectUpdate { id: inbox.id.clone(), name: "Junk".into() });
    h.run(Action::ProjectDelete { id: inbox.id.clone() });
    let projects = h.repo.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "Inbox");
}

#[test]
fn rename_label_updates_tasks() {
    let mut h = H::new();
    let label = h.label("hme");
    let task = h.task(&["hme", "errands"]);
    h.run(Action::LabelUpdate { id: label, name: "home".into() });
    assert_eq!(h.label_names(), vec!["home"]);
    assert_eq!(h.labels_of(&task), vec!["home", "errands"]);
}

#[test]
fn rename_label_onto_existing_merges() {
    let mut h = H::new();
    h.label("home");
    let house = h.label("house");
    let both = h.task(&["home", "house"]);
    let only_house = h.task(&["house"]);
    h.run(Action::LabelUpdate { id: house, name: "Home".into() });
    assert_eq!(h.label_names(), vec!["home"], "merged into the existing label");
    assert_eq!(h.labels_of(&both), vec!["home"], "no duplicate after merge");
    assert_eq!(h.labels_of(&only_house), vec!["home"]);
}
