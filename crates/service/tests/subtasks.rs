use chrono::NaiveDate;
use keel_core::commands::{Action, Command};
use keel_core::ids::Id;
use keel_core::model::Task;
use keel_core::recurrence::Recurrence;
use keel_service::AppService;

const FRIDAY: u8 = 4;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()
}

struct Fx {
    _dir: tempfile::TempDir,
    svc: AppService,
    inbox: Id,
    n: u32,
}

impl Fx {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
        let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;
        Fx { _dir: dir, svc, inbox, n: 0 }
    }

    fn run_on(&mut self, action: Action, day: NaiveDate) -> Option<Id> {
        self.n += 1;
        let cmd = Command { uuid: format!("u{}", self.n), temp_id: Some("t".into()), action };
        self.svc.dispatch_at(cmd, day).unwrap().id_mapping.get("t").cloned()
    }

    fn add(&mut self, parent: Option<&Id>, recurrence: Option<Recurrence>) -> Id {
        let action = Action::TaskAdd {
            content: "t".to_string(),
            project_id: self.inbox.clone(),
            description: None,
            priority: None,
            due: None,
            labels: None,
            parent_id: parent.cloned(),
            section_id: None,
            recurrence,
            deadline: None,
        };
        self.run_on(action, today()).unwrap()
    }

    fn streak(&mut self, id: &Id) {
        self.run_on(Action::TaskSetStreak { id: id.clone(), enabled: true, goal: None }, today());
    }

    fn complete(&mut self, id: &Id) {
        self.run_on(Action::TaskComplete { id: id.clone() }, today());
    }

    fn get(&self, id: &Id) -> Task {
        self.svc.all_tasks_at(today()).unwrap().into_iter().find(|t| &t.id == id).unwrap()
    }

    fn date(&self, id: &Id) -> String {
        self.get(id).due.unwrap().date
    }
}

#[test]
fn completing_a_parent_completes_plain_and_repeating_sub_tasks() {
    let mut f = Fx::new();
    let parent = f.add(None, None);
    let plain = f.add(Some(&parent), None);
    let repeating = f.add(Some(&parent), Some(Recurrence::Daily));
    let grandchild = f.add(Some(&plain), Some(Recurrence::Daily));

    f.complete(&parent);

    for id in [&parent, &plain, &repeating, &grandchild] {
        let t = f.get(id);
        assert!(t.checked, "{id:?}");
        assert!(t.completed_at.is_some(), "{id:?}");
    }
}

#[test]
fn completing_a_repeating_parent_completes_its_repeating_sub_tasks() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let child = f.add(Some(&parent), Some(Recurrence::Daily));
    f.streak(&child);

    f.complete(&parent);

    assert_eq!(f.date(&parent), "2026-01-06");
    assert_eq!(f.date(&child), "2026-01-06");
    let t = f.get(&child);
    assert!(!t.checked, "a repeating sub-task of a repeating parent stays active");
    assert_eq!(t.streak_current, 1);
}

#[test]
fn a_sub_task_done_earlier_is_not_completed_twice() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let child = f.add(Some(&parent), Some(Recurrence::Daily));
    f.streak(&child);
    f.complete(&child);
    assert_eq!(f.date(&child), "2026-01-06");

    f.complete(&parent);

    assert_eq!(f.date(&child), "2026-01-06");
    assert_eq!(f.get(&child).streak_current, 1);
}

#[test]
fn a_repeating_parent_does_not_pull_a_later_sub_task_forward() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    // Weekly on Fridays: next due Jan 9, after the parent's Jan 5 occurrence.
    let weekly = Recurrence::Weekly(vec![FRIDAY]);
    let child = f.add(Some(&parent), Some(weekly));
    let before = f.date(&child);
    assert!(before.as_str() > "2026-01-05", "{before}");

    f.complete(&parent);

    assert_eq!(f.date(&child), before);
}

#[test]
fn completing_a_repeating_parent_still_resets_plain_sub_tasks() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let plain = f.add(Some(&parent), None);
    f.complete(&plain);
    assert!(f.get(&plain).checked);

    f.complete(&parent);

    assert!(!f.get(&plain).checked);
}

#[test]
fn undoing_a_repeating_completion_restores_its_date_and_streak() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let child = f.add(Some(&parent), Some(Recurrence::Daily));
    f.streak(&child);
    // Two days running, so the undo has a streak to step back to.
    let tomorrow = NaiveDate::from_ymd_opt(2026, 1, 6).unwrap();
    f.complete(&child);
    f.run_on(Action::TaskComplete { id: child.clone() }, tomorrow);
    assert_eq!(f.date(&child), "2026-01-07");
    let t = f.get(&child);
    assert_eq!((t.streak_current, t.streak_best), (2, 2));

    f.run_on(Action::TaskUncomplete { id: child.clone() }, tomorrow);

    let t = f.get(&child);
    assert_eq!(f.date(&child), "2026-01-06");
    assert_eq!((t.streak_current, t.streak_best), (1, 1));
    assert_eq!(t.streak_last_period.as_deref(), Some("2026-01-05"));
    assert!(!t.checked);
}

#[test]
fn undo_returns_an_overdue_task_to_its_overdue_date() {
    let mut f = Fx::new();
    let task = f.add(None, Some(Recurrence::Weekly(vec![FRIDAY])));
    let first = f.date(&task);
    let later = NaiveDate::parse_from_str(&first, "%Y-%m-%d").unwrap() + chrono::Duration::days(10);
    f.run_on(Action::TaskComplete { id: task.clone() }, later);
    assert_ne!(f.date(&task), first);

    f.run_on(Action::TaskUncomplete { id: task.clone() }, later);

    assert_eq!(f.date(&task), first);
}

#[test]
fn undo_leaves_a_task_whose_date_was_changed_since() {
    let mut f = Fx::new();
    let task = f.add(None, Some(Recurrence::Daily));
    f.complete(&task);
    let due = keel_core::commands::DueEdit { date: Some("2026-02-01".into()), recurrence: Some(Recurrence::Daily) };
    f.run_on(
        Action::TaskUpdate {
            id: task.clone(),
            content: None,
            description: None,
            priority: None,
            project_id: None,
            labels: None,
            due: Some(due),
            is_collapsed: None,
            deadline: None,
        },
        today(),
    );

    f.run_on(Action::TaskUncomplete { id: task.clone() }, today());

    assert_eq!(f.date(&task), "2026-02-01");
}

#[test]
fn undoing_twice_steps_back_only_once() {
    let mut f = Fx::new();
    let task = f.add(None, Some(Recurrence::Daily));
    f.complete(&task);
    f.run_on(Action::TaskUncomplete { id: task.clone() }, today());
    f.run_on(Action::TaskUncomplete { id: task.clone() }, today());
    assert_eq!(f.date(&task), "2026-01-05");
}
