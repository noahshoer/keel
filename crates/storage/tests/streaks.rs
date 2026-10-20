use chrono::NaiveDate;
use keel_core::commands::{apply, apply_at, Action, Command};
use keel_core::ids::Id;
use keel_core::recurrence::Recurrence;
use keel_core::repository::Repository;
use keel_storage::SqliteRepo;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn cmd(uuid: &str, temp: Option<&str>, action: Action) -> Command {
    Command {
        uuid: uuid.to_string(),
        temp_id: temp.map(str::to_string),
        action,
    }
}

fn add(content: &str, project: &Id, recurrence: Option<Recurrence>) -> Action {
    Action::TaskAdd {
        content: content.to_string(),
        project_id: project.clone(),
        description: None,
        priority: None,
        due: None,
        labels: None,
        parent_id: None,
        section_id: None,
        recurrence,
        deadline: None,
    }
}

/// Adds a Daily recurring task on `today` with streak tracking on; returns its id.
fn daily_streak_task(repo: &SqliteRepo, today: NaiveDate) -> Id {
    let project = Id::new();
    let res = apply_at(
        repo,
        cmd("add", Some("tmp"), add("read", &project, Some(Recurrence::Daily))),
        today,
    )
    .unwrap();
    let id = res.id_mapping["tmp"].clone();
    apply_at(
        repo,
        cmd(
            "streak-on",
            None,
            Action::TaskSetStreak {
                id: id.clone(),
                enabled: true,
                goal: None,
            },
        ),
        today,
    )
    .unwrap();
    id
}

#[test]
fn complete_recurring_advances_due_and_increments_streak() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let id = daily_streak_task(&repo, d(2026, 1, 5));

    let before = repo.get_task(&id).unwrap().unwrap();
    let due = before.due.as_ref().expect("recurrence implies a due date");
    assert_eq!(due.date, "2026-01-05");
    assert!(due.is_recurring);
    assert_eq!(due.string, "every day");

    apply_at(&repo, cmd("c1", None, Action::TaskComplete { id: id.clone() }), d(2026, 1, 5)).unwrap();

    let after = repo.get_task(&id).unwrap().unwrap();
    assert_eq!(after.due.unwrap().date, "2026-01-06", "rescheduled to next occurrence");
    assert_eq!(after.streak_current, 1);
    assert!(!after.checked, "recurring tasks stay active");
}

#[test]
fn complete_recurring_twice_same_day_no_double() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let id = daily_streak_task(&repo, d(2026, 1, 5));
    apply_at(&repo, cmd("c1", None, Action::TaskComplete { id: id.clone() }), d(2026, 1, 5)).unwrap();
    apply_at(&repo, cmd("c2", None, Action::TaskComplete { id: id.clone() }), d(2026, 1, 5)).unwrap();
    let t = repo.get_task(&id).unwrap().unwrap();
    assert_eq!(t.streak_current, 1);
    assert_eq!(t.due.unwrap().date, "2026-01-06", "a repeat click must not skip tomorrow's occurrence");
}

#[test]
fn complete_nonrecurring_checks_it() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let project = Id::new();
    let res = apply(&repo, cmd("a", Some("t"), add("once", &project, None))).unwrap();
    let id = res.id_mapping["t"].clone();
    apply(&repo, cmd("c", None, Action::TaskComplete { id: id.clone() })).unwrap();
    let t = repo.get_task(&id).unwrap().unwrap();
    assert!(t.checked);
    assert_eq!(t.streak_current, 0);
}

#[test]
fn set_streak_enables_and_sets_goal() {
    let repo = SqliteRepo::open_in_memory().unwrap();
    let project = Id::new();
    let res = apply(&repo, cmd("a", Some("t"), add("gym", &project, Some(Recurrence::Weekly(vec![0, 3]))))).unwrap();
    let id = res.id_mapping["t"].clone();
    apply(
        &repo,
        cmd(
            "s",
            None,
            Action::TaskSetStreak {
                id: id.clone(),
                enabled: true,
                goal: Some(30),
            },
        ),
    )
    .unwrap();
    let t = repo.get_task(&id).unwrap().unwrap();
    assert!(t.streak_enabled);
    assert_eq!(t.streak_goal, Some(30));
}

#[test]
fn early_completion_credits_the_upcoming_occurrence() {
    // Mon/Wed habit. 2026-01-05 is a Monday.
    let repo = SqliteRepo::open_in_memory().unwrap();
    let project = Id::new();
    let mon = d(2026, 1, 5);
    let res = apply_at(&repo, cmd("add", Some("t"), add("gym", &project, Some(Recurrence::Weekly(vec![0, 2])))), mon).unwrap();
    let id = res.id_mapping["t"].clone();
    apply_at(&repo, cmd("on", None, Action::TaskSetStreak { id: id.clone(), enabled: true, goal: None }), mon).unwrap();

    apply_at(&repo, cmd("c1", None, Action::TaskComplete { id: id.clone() }), mon).unwrap(); // Mon on time
    apply_at(&repo, cmd("c2", None, Action::TaskComplete { id: id.clone() }), d(2026, 1, 6)).unwrap(); // Wed's, done Tue
    let t = repo.get_task(&id).unwrap().unwrap();
    assert_eq!(t.streak_current, 2);
    assert_eq!(t.due.as_ref().unwrap().date, "2026-01-12", "next occurrence after Wed is Mon");

    apply_at(&repo, cmd("c3", None, Action::TaskComplete { id: id.clone() }), d(2026, 1, 12)).unwrap(); // next Mon
    assert_eq!(repo.get_task(&id).unwrap().unwrap().streak_current, 3, "early completion must not break the streak");
}
