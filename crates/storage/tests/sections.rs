use chrono::NaiveDate;
use keel_core::commands::{apply, apply_at, Action, ApplyResult, Command};
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

    /// Like `run`, but on a fixed day (recurring tasks start due that day).
    fn run_on(&mut self, day: &str, action: Action) -> ApplyResult {
        self.n += 1;
        let today = NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap();
        apply_at(&self.repo, Command { uuid: format!("u{}", self.n), temp_id: Some("t".into()), action }, today).unwrap()
    }

    fn task_on(&mut self, day: &str, name: &str, parent: Option<&Id>, recurrence: Option<Recurrence>) -> Id {
        self.run_on(
            day,
            Action::TaskAdd {
                content: name.into(),
                project_id: p("P"),
                description: None,
                priority: None,
                due: None,
                labels: None,
                parent_id: parent.cloned(),
                section_id: None,
                recurrence,
                deadline: None,
            },
        )
        .id_mapping["t"]
            .clone()
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
            deadline: None,
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
        deadline: None,
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
        deadline: None,
    });
    assert!(h.get(&id).is_collapsed);
}

const DAY: &str = "2026-10-23";

/// Repeating parent P (daily, due DAY) with children A and B, and A1 under A,
/// all three checked.
fn checked_tree(h: &mut H) -> (Id, Id, Id, Id) {
    let parent = h.task_on(DAY, "P", None, Some(Recurrence::Daily));
    let a = h.task_on(DAY, "A", Some(&parent), None);
    let b = h.task_on(DAY, "B", Some(&parent), None);
    let a1 = h.task_on(DAY, "A1", Some(&a), None);
    for id in [&a, &b, &a1] {
        h.run_on(DAY, Action::TaskComplete { id: id.clone() });
        assert!(h.get(id).checked);
    }
    (parent, a, b, a1)
}

#[test]
fn repeating_parent_resets_all_nonrepeating_descendants() {
    let mut h = H::new();
    let (parent, a, b, a1) = checked_tree(&mut h);
    h.run_on(DAY, Action::TaskSetStreak { id: parent.clone(), enabled: true, goal: None });
    h.run_on(DAY, Action::TaskComplete { id: parent.clone() });
    for id in [&a, &b, &a1] {
        let t = h.get(id);
        assert!(!t.checked);
        assert_eq!(t.completed_at, None);
    }
    let pt = h.get(&parent);
    assert_eq!(pt.due.unwrap().date, "2026-10-24");
    assert!(!pt.checked);
    assert_eq!(pt.streak_current, 1);
}

#[test]
fn reset_leaves_repeating_descendants() {
    let mut h = H::new();
    let parent = h.task_on(DAY, "P", None, Some(Recurrence::Daily));
    let habit = h.task_on(DAY, "H", Some(&parent), Some(Recurrence::Weekly(vec![4])));
    h.run_on(DAY, Action::TaskSetStreak { id: habit.clone(), enabled: true, goal: None });
    h.run_on(DAY, Action::TaskComplete { id: habit.clone() });
    let before = h.get(&habit);
    h.run_on(DAY, Action::TaskComplete { id: parent });
    let after = h.get(&habit);
    assert_eq!(after.due, before.due);
    assert_eq!(after.streak_current, before.streak_current);
    assert_eq!(after.streak_last_period, before.streak_last_period);
}

#[test]
fn reset_ignores_other_trees() {
    let mut h = H::new();
    let (parent, ..) = checked_tree(&mut h);
    let q = h.task_on(DAY, "Q", None, Some(Recurrence::Daily));
    let other = h.task_on(DAY, "other", Some(&q), None);
    h.run_on(DAY, Action::TaskComplete { id: other.clone() });
    h.run_on(DAY, Action::TaskComplete { id: parent });
    assert!(h.get(&other).checked);
}

#[test]
fn already_credited_does_not_reset() {
    let mut h = H::new();
    let (parent, a, ..) = checked_tree(&mut h);
    h.run_on(DAY, Action::TaskSetStreak { id: parent.clone(), enabled: true, goal: None });
    h.run_on(DAY, Action::TaskComplete { id: parent.clone() });
    assert!(!h.get(&a).checked);
    h.run_on(DAY, Action::TaskComplete { id: a.clone() });
    assert!(h.get(&a).checked);
    // Same occurrence again: a no-op, so A stays checked.
    h.run_on(DAY, Action::TaskComplete { id: parent.clone() });
    assert!(h.get(&a).checked);
    assert_eq!(h.get(&parent).streak_current, 1);
}

#[test]
fn one_off_parent_still_completes_children() {
    let mut h = H::new();
    let parent = h.task_on(DAY, "P", None, None);
    let child = h.task_on(DAY, "C", Some(&parent), None);
    let grandchild = h.task_on(DAY, "G", Some(&child), None);
    h.run_on(DAY, Action::TaskComplete { id: parent.clone() });
    for id in [&parent, &child, &grandchild] {
        assert!(h.get(id).checked);
    }
}

#[test]
fn uncomplete_child_leaves_parent_completed() {
    let mut h = H::new();
    let parent = h.task_on(DAY, "P", None, None);
    let child = h.task_on(DAY, "C", Some(&parent), None);
    h.run_on(DAY, Action::TaskComplete { id: parent.clone() });
    assert!(h.get(&child).checked);
    h.run_on(DAY, Action::TaskUncomplete { id: child.clone() });
    assert!(h.get(&parent).checked);
    assert!(!h.get(&child).checked);
}

#[test]
fn delete_parent_deletes_whole_subtree_and_reminders() {
    let mut h = H::new();
    let parent = h.task_on(DAY, "P", None, None);
    let a = h.task_on(DAY, "A", Some(&parent), None);
    let a1 = h.task_on(DAY, "A1", Some(&a), None);
    h.run_on(DAY, Action::ReminderAdd { task_id: a.clone(), minute_offset: Some(10), at: None });
    h.run_on(DAY, Action::ReminderAdd { task_id: a1.clone(), minute_offset: Some(10), at: None });
    assert_eq!(h.repo.list_reminders().unwrap().len(), 2);
    h.run_on(DAY, Action::TaskDelete { id: parent.clone() });
    for id in [&parent, &a, &a1] {
        assert!(h.get(id).is_deleted);
    }
    assert!(h.repo.list_reminders().unwrap().is_empty());
}
