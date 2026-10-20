use keel_core::commands::{apply, Action, ApplyResult, Command};
use keel_core::ids::Id;
use keel_core::model::Task;
use keel_core::recurrence::Recurrence;
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

    fn section(&mut self, project: &Id, name: &str) -> Id {
        self.run(Action::SectionAdd { project_id: project.clone(), name: name.into() }).id_mapping["t"].clone()
    }

    fn task(&mut self, name: &str, project: &Id, section: Option<&Id>, parent: Option<&Id>, recurrence: Option<Recurrence>) -> Id {
        self.run(Action::TaskAdd {
            content: name.into(),
            project_id: project.clone(),
            description: None,
            priority: None,
            due: None,
            labels: None,
            parent_id: parent.cloned(),
            section_id: section.cloned(),
            recurrence,
        })
        .id_mapping["t"]
            .clone()
    }

    fn get(&self, id: &Id) -> Task {
        self.repo.get_task(id).unwrap().unwrap()
    }

    fn section_names(&self, project: &Id) -> Vec<String> {
        self.repo.list_sections(project).unwrap().into_iter().map(|s| s.name).collect()
    }
}

fn p(id: &str) -> Id {
    Id::from(id)
}

#[test]
fn add_rename_list_sections() {
    let mut h = H::new();
    let planning = h.section(&p("P"), "Planning");
    h.section(&p("P"), "Doing");
    h.section(&p("Q"), "Elsewhere");
    h.run(Action::SectionUpdate { id: planning, name: " Plan ".into() });
    assert_eq!(h.section_names(&p("P")), vec!["Plan", "Doing"], "creation order, scoped to project");
}

#[test]
fn delete_section_removes_its_tasks_only() {
    let mut h = H::new();
    let a = h.section(&p("P"), "A");
    let b = h.section(&p("P"), "B");
    h.task("in-a", &p("P"), Some(&a), None, None);
    h.task("in-b", &p("P"), Some(&b), None, None);
    h.task("loose", &p("P"), None, None, None);
    h.run(Action::SectionDelete { id: a });
    assert_eq!(h.section_names(&p("P")), vec!["B"]);
    let mut left: Vec<String> = h.repo.list_tasks(&p("P")).unwrap().into_iter().map(|t| t.content).collect();
    left.sort();
    assert_eq!(left, vec!["in-b", "loose"]);
}

#[test]
fn move_task_moves_subtree() {
    let mut h = H::new();
    let dest_section = h.section(&p("Q"), "Inbox-ish");
    let parent = h.task("parent", &p("P"), None, None, None);
    let child = h.task("child", &p("P"), None, Some(&parent), None);
    let grandchild = h.task("grandchild", &p("P"), None, Some(&child), None);
    h.run(Action::TaskMove { id: parent.clone(), project_id: p("Q"), section_id: Some(dest_section.clone()) });
    for id in [&parent, &child, &grandchild] {
        let t = h.get(id);
        assert_eq!(t.project_id, p("Q"), "{} moved project", t.content);
        assert_eq!(t.section_id, Some(dest_section.clone()), "{} moved section", t.content);
    }
    assert_eq!(h.get(&grandchild).parent_id, Some(child), "hierarchy kept");
}

#[test]
fn update_project_clears_section_and_moves_children() {
    let mut h = H::new();
    let sec = h.section(&p("P"), "S");
    let parent = h.task("parent", &p("P"), Some(&sec), None, None);
    let child = h.task("child", &p("P"), Some(&sec), Some(&parent), None);
    h.run(Action::TaskUpdate {
        id: parent.clone(),
        content: None,
        description: None,
        priority: None,
        project_id: Some(p("Q")),
        labels: None,
        due: None,
        is_collapsed: None,
    });
    for id in [&parent, &child] {
        let t = h.get(id);
        assert_eq!(t.project_id, p("Q"));
        assert_eq!(t.section_id, None);
    }
}

#[test]
fn complete_cascades_to_nonrecurring_children() {
    let mut h = H::new();
    let parent = h.task("parent", &p("P"), None, None, None);
    let child = h.task("child", &p("P"), None, Some(&parent), None);
    let habit = h.task("habit", &p("P"), None, Some(&parent), Some(Recurrence::Daily));
    let habit_due = h.get(&habit).due;
    h.run(Action::TaskComplete { id: parent.clone() });
    assert!(h.get(&parent).checked);
    assert!(h.get(&child).checked, "non-recurring child completes with its parent");
    let habit_after = h.get(&habit);
    assert!(!habit_after.checked, "recurring child is left alone");
    assert_eq!(habit_after.due, habit_due, "and not rescheduled");
}

#[test]
fn collapse_flag_persists() {
    let mut h = H::new();
    let id = h.task("parent", &p("P"), None, None, None);
    h.run(Action::TaskUpdate {
        id: id.clone(),
        content: None,
        description: None,
        priority: None,
        project_id: None,
        labels: None,
        due: None,
        is_collapsed: Some(true),
    });
    assert!(h.get(&id).is_collapsed);
}
