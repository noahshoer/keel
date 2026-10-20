use chrono::NaiveDate;
use keel_core::commands::{Action, Command};
use keel_core::ids::Id;
use keel_core::recurrence::Recurrence;
use keel_service::AppService;

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

/// Opens a temp store, adds a Daily streak task to the Inbox on `today`, and
/// completes it once on `today`. Returns (dir guard, service, inbox id, task id).
fn completed_streak_task(today: NaiveDate) -> (tempfile::TempDir, AppService, Id, Id) {
    let dir = tempfile::tempdir().unwrap();
    let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
    let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;

    let res = svc
        .dispatch_at(
            cmd(
                "add",
                Some("tmp"),
                Action::TaskAdd {
                    content: "read".to_string(),
                    project_id: inbox.clone(),
                    description: None,
                    priority: None,
                    due: None,
                    labels: None,
                    parent_id: None,
                    section_id: None,
                    recurrence: Some(Recurrence::Daily),
                    deadline: None,
                },
            ),
            today,
        )
        .unwrap();
    let id = res.id_mapping["tmp"].clone();
    svc.dispatch_at(
        cmd("on", None, Action::TaskSetStreak { id: id.clone(), enabled: true, goal: None }),
        today,
    )
    .unwrap();
    svc.dispatch_at(cmd("done", None, Action::TaskComplete { id: id.clone() }), today).unwrap();
    (dir, svc, inbox, id)
}

fn streak_of(svc: &AppService, inbox: &Id, id: &Id, today: NaiveDate) -> i64 {
    svc.list_tasks_at(inbox.as_str(), today)
        .unwrap()
        .into_iter()
        .find(|t| &t.id == id)
        .unwrap()
        .streak_current
}

#[test]
fn completion_then_list_shows_streak() {
    let (_dir, svc, inbox, id) = completed_streak_task(d(2026, 1, 5));
    assert_eq!(streak_of(&svc, &inbox, &id, d(2026, 1, 5)), 1);
    // The next day the occurrence is due but not missed yet.
    assert_eq!(streak_of(&svc, &inbox, &id, d(2026, 1, 6)), 1);
}

#[test]
fn list_resets_streak_after_missed_period_and_persists() {
    let (dir, svc, inbox, id) = completed_streak_task(d(2026, 1, 5));
    // Jan 6 came and went uncompleted.
    assert_eq!(streak_of(&svc, &inbox, &id, d(2026, 1, 7)), 0);
    drop(svc);

    // Reopen: the reset was written, not just computed on the fly.
    let svc2 = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
    let stored = svc2
        .list_tasks_at(inbox.as_str(), d(2026, 1, 5)) // a date that wouldn't itself reset
        .unwrap()
        .into_iter()
        .find(|t| t.id == id)
        .unwrap();
    assert_eq!(stored.streak_current, 0);
    assert_eq!(stored.streak_best, 1, "best record kept");
}
