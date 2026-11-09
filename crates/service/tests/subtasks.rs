use chrono::NaiveDate;
use keel_core::commands::{Action, Command};
use keel_core::ids::Id;
use keel_core::model::Task;
use keel_core::recurrence::Recurrence;
use keel_core::views::{is_done, progress, Progress};
use keel_service::AppService;
use keel_storage::SqliteRepo;

const FRIDAY: u8 = 4;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()
}

fn tomorrow() -> NaiveDate {
    today().succ_opt().unwrap()
}

struct Fx {
    _dir: tempfile::TempDir,
    path: String,
    svc: AppService,
    inbox: Id,
    n: u32,
}

impl Fx {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keel.db").to_str().unwrap().to_string();
        let svc = AppService::open(&path).unwrap();
        let inbox = svc
            .list_projects()
            .unwrap()
            .into_iter()
            .find(|p| p.is_inbox)
            .unwrap()
            .id;
        Fx {
            _dir: dir,
            path,
            svc,
            inbox,
            n: 0,
        }
    }

    fn run_on(&mut self, action: Action, day: NaiveDate) -> Option<Id> {
        self.n += 1;
        let cmd = Command {
            uuid: format!("u{}", self.n),
            temp_id: Some("t".into()),
            action,
        };
        self.svc
            .dispatch_at(cmd, day)
            .unwrap()
            .id_mapping
            .get("t")
            .cloned()
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
        self.run_on(
            Action::TaskSetStreak {
                id: id.clone(),
                enabled: true,
                goal: None,
            },
            today(),
        );
    }

    fn complete(&mut self, id: &Id) {
        self.run_on(Action::TaskComplete { id: id.clone() }, today());
    }

    fn get(&self, id: &Id) -> Task {
        self.svc
            .all_tasks_at(today())
            .unwrap()
            .into_iter()
            .find(|t| &t.id == id)
            .unwrap()
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
    assert!(
        !t.checked,
        "a repeating sub-task of a repeating parent stays active"
    );
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
fn completing_a_repeating_parent_keeps_plain_sub_tasks_completed_until_its_next_date() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let done_first = f.add(Some(&parent), None);
    let still_open = f.add(Some(&parent), None);
    let grandchild = f.add(Some(&still_open), None);
    f.complete(&done_first);

    f.complete(&parent);

    for id in [&done_first, &still_open, &grandchild] {
        assert!(f.get(id).checked, "{id:?} completes with its parent");
    }
    let today_all = f.svc.all_tasks_at(today()).unwrap();
    assert_eq!(
        progress(&parent, &today_all, today()),
        Some(Progress { done: 2, total: 2 })
    );

    let next_all = f.svc.all_tasks_at(tomorrow()).unwrap();
    for id in [&done_first, &still_open, &grandchild] {
        let t = next_all.iter().find(|t| &t.id == id).unwrap();
        assert!(!t.checked, "{id:?} reopens for the next round");
        assert_eq!(t.completed_at, None);
    }
    assert_eq!(
        progress(&parent, &next_all, tomorrow()),
        Some(Progress { done: 0, total: 2 })
    );
    // And it's saved, not just how that read looked.
    assert!(!f
        .svc
        .project_view_at(f.inbox.as_str(), tomorrow())
        .unwrap()
        .iter()
        .any(|t| t.checked));
}

#[test]
fn a_plain_sub_task_reopened_by_hand_stays_open_and_the_rest_stay_completed() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let a = f.add(Some(&parent), None);
    let b = f.add(Some(&parent), None);
    f.complete(&parent);

    f.run_on(Action::TaskUncomplete { id: a.clone() }, today());

    assert!(!f.get(&a).checked);
    assert!(f.get(&b).checked);
}

#[test]
fn a_plain_sub_task_completed_by_hand_does_not_reopen_on_its_own() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let a = f.add(Some(&parent), None);
    f.complete(&a);
    let next_all = f.svc.all_tasks_at(tomorrow()).unwrap();
    assert!(next_all.iter().find(|t| t.id == a).unwrap().checked);
}

#[test]
fn a_sub_task_of_a_parent_that_does_not_repeat_never_reopens() {
    let mut f = Fx::new();
    let parent = f.add(None, None);
    let a = f.add(Some(&parent), None);
    f.complete(&parent);
    let next_all = f.svc.all_tasks_at(tomorrow()).unwrap();
    assert!(next_all.iter().find(|t| t.id == a).unwrap().checked);
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
    let due = keel_core::commands::DueEdit {
        date: Some("2026-02-01".into()),
        recurrence: Some(Recurrence::Daily),
        until: None,
    };
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

#[test]
fn a_completed_sub_task_stays_completed_until_its_next_date() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let child = f.add(Some(&parent), Some(Recurrence::Daily));
    f.complete(&child);
    assert!(is_done(&f.get(&child), today()));
    assert!(!is_done(&f.get(&child), tomorrow()));
}

#[test]
fn completing_a_repeating_parent_leaves_its_sub_tasks_completed_for_today() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let a = f.add(Some(&parent), Some(Recurrence::Daily));
    let b = f.add(Some(&parent), Some(Recurrence::Daily));
    f.streak(&a);
    f.complete(&parent);

    let all = f.svc.all_tasks_at(today()).unwrap();
    assert_eq!(
        progress(&parent, &all, today()),
        Some(Progress { done: 2, total: 2 })
    );
    assert_eq!(
        progress(&parent, &all, tomorrow()),
        Some(Progress { done: 0, total: 2 })
    );

    // And tapping one reopens just it, for today.
    f.run_on(Action::TaskUncomplete { id: a.clone() }, today());
    assert_eq!(f.date(&a), "2026-01-05");
    assert!(!is_done(&f.get(&a), today()));
    assert!(is_done(&f.get(&b), today()));
}

#[test]
fn a_sub_task_just_not_due_yet_is_not_completed() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let child = f.add(Some(&parent), Some(Recurrence::Weekly(vec![FRIDAY])));
    assert!(!is_done(&f.get(&child), today()));
}

#[test]
fn a_sub_task_completed_before_completions_were_saved_shows_done_and_undoes() {
    let mut f = Fx::new();
    let parent = f.add(None, Some(Recurrence::Daily));
    let child = f.add(Some(&parent), Some(Recurrence::Daily));
    f.streak(&child);
    f.complete(&child);
    // As the older build left it: moved and credited, nothing saved.
    let mut old = f.get(&child);
    old.last_completion = None;
    keel_core::repository::Repository::upsert_task(&SqliteRepo::open(&f.path).unwrap(), &old)
        .unwrap();

    assert!(is_done(&f.get(&child), today()));
    f.run_on(Action::TaskUncomplete { id: child.clone() }, today());

    let t = f.get(&child);
    assert_eq!(f.date(&child), "2026-01-05");
    assert!(!is_done(&t, today()));
    assert_eq!(
        (t.streak_current, t.streak_best, t.streak_last_period),
        (0, 0, None)
    );
}
