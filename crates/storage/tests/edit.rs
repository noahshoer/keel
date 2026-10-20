use chrono::NaiveDate;
use keel_core::commands::{apply_at, Action, Command, DueEdit};
use keel_core::ids::Id;
use keel_core::model::{Priority, Task};
use keel_core::recurrence::Recurrence;
use keel_core::repository::Repository;
use keel_storage::SqliteRepo;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

const MON: (i32, u32, u32) = (2026, 1, 5);

fn mon() -> NaiveDate {
    d(MON.0, MON.1, MON.2)
}

struct Harness {
    repo: SqliteRepo,
    n: u32,
}

impl Harness {
    fn new() -> Self {
        Harness { repo: SqliteRepo::open_in_memory().unwrap(), n: 0 }
    }

    fn run(&mut self, action: Action, today: NaiveDate) -> keel_core::commands::ApplyResult {
        self.n += 1;
        apply_at(
            &self.repo,
            Command { uuid: format!("u{}", self.n), temp_id: Some("t".into()), action },
            today,
        )
        .unwrap()
    }

    fn add(&mut self, labels: Option<Vec<String>>, priority: Option<Priority>, recurrence: Option<Recurrence>) -> Id {
        let res = self.run(
            Action::TaskAdd {
                content: "original".into(),
                project_id: Id::from("P1"),
                description: Some("desc".into()),
                priority,
                due: None,
                labels,
                parent_id: None,
                section_id: None,
                recurrence,
                deadline: None,
            },
            mon(),
        );
        res.id_mapping["t"].clone()
    }

    fn get(&self, id: &Id) -> Task {
        self.repo.get_task(id).unwrap().unwrap()
    }
}

/// A TaskUpdate that changes nothing; tests fill in the fields they care about.
fn noop(id: &Id) -> Action {
    Action::TaskUpdate {
        id: id.clone(),
        content: None,
        description: None,
        priority: None,
        project_id: None,
        labels: None,
        due: None,
        is_collapsed: None,
        deadline: None,
    }
}

#[test]
fn update_content_only_keeps_everything_else() {
    let mut h = Harness::new();
    let id = h.add(Some(vec!["home".into()]), Some(Priority::P1), Some(Recurrence::Daily));
    h.run(Action::TaskSetStreak { id: id.clone(), enabled: true, goal: Some(10) }, mon());
    h.run(Action::TaskComplete { id: id.clone() }, mon());
    let before = h.get(&id);

    h.run(
        Action::TaskUpdate {
            id: id.clone(),
            content: Some("renamed".into()),
            description: None,
            priority: None,
            project_id: None,
            labels: None,
            due: None,
            is_collapsed: None,
            deadline: None,
        },
        mon(),
    );

    let after = h.get(&id);
    assert_eq!(after.content, "renamed");
    assert_eq!(after.description, before.description);
    assert_eq!(after.labels, vec!["home".to_string()]);
    assert_eq!(after.priority, Priority::P1);
    assert_eq!(after.due, before.due, "due + recurrence untouched");
    assert_eq!(after.streak_current, 1);
    assert_eq!(after.streak_goal, Some(10));
}

#[test]
fn update_sets_date_priority_project_labels() {
    let mut h = Harness::new();
    let id = h.add(None, None, None);
    let other = Id::from("P2");
    h.run(
        Action::TaskUpdate {
            id: id.clone(),
            content: None,
            description: Some(String::new()),
            priority: Some(Priority::P2),
            project_id: Some(other.clone()),
            labels: Some(vec!["work".into(), "deep".into()]),
            due: Some(DueEdit { date: Some("2026-02-01".into()), recurrence: None }),
            is_collapsed: None,
            deadline: None,
        },
        mon(),
    );
    let t = h.get(&id);
    assert_eq!(t.priority, Priority::P2);
    assert_eq!(t.project_id, other);
    assert_eq!(t.labels, vec!["work".to_string(), "deep".to_string()]);
    assert_eq!(t.description, "");
    let due = t.due.unwrap();
    assert_eq!(due.date, "2026-02-01");
    assert!(!due.is_recurring);
}

#[test]
fn clear_due_removes_recurrence() {
    let mut h = Harness::new();
    let id = h.add(None, None, Some(Recurrence::Daily));
    assert!(h.get(&id).due.is_some());
    let mut action = noop(&id);
    if let Action::TaskUpdate { due, .. } = &mut action {
        *due = Some(DueEdit { date: None, recurrence: None });
    }
    h.run(action, mon());
    assert_eq!(h.get(&id).due, None);
}

#[test]
fn set_recurrence_without_date_uses_first_occurrence() {
    let mut h = Harness::new();
    let id = h.add(None, None, None);
    let mut action = noop(&id);
    if let Action::TaskUpdate { due, .. } = &mut action {
        *due = Some(DueEdit { date: None, recurrence: Some(Recurrence::Weekly(vec![2])) });
    }
    h.run(action, mon());
    let due = h.get(&id).due.unwrap();
    assert_eq!(due.date, "2026-01-07", "first Wednesday on/after Monday");
    assert!(due.is_recurring);
    assert_eq!(due.string, "every Wed");
}

#[test]
fn changing_recurrence_resets_streak_keeps_best() {
    let mut h = Harness::new();
    let id = h.add(None, None, Some(Recurrence::Daily));
    h.run(Action::TaskSetStreak { id: id.clone(), enabled: true, goal: None }, mon());
    h.run(Action::TaskComplete { id: id.clone() }, mon());
    h.run(Action::TaskComplete { id: id.clone() }, d(2026, 1, 6));
    assert_eq!(h.get(&id).streak_current, 2);

    let mut action = noop(&id);
    if let Action::TaskUpdate { due, .. } = &mut action {
        *due = Some(DueEdit { date: Some("2026-01-12".into()), recurrence: Some(Recurrence::Weekly(vec![0])) });
    }
    h.run(action, d(2026, 1, 6));
    let t = h.get(&id);
    assert_eq!(t.streak_current, 0);
    assert_eq!(t.streak_last_period, None);
    assert_eq!(t.streak_best, 2);
    assert_eq!(t.due.unwrap().string, "every Mon");
}
