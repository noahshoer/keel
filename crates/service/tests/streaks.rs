use chrono::NaiveDate;
use keel_core::ids::Id;
use keel_core::recurrence::Recurrence;

mod common;
use common::{d, Fx, NewTask};

/// Opens a temp store, adds a Daily streak task to the Inbox on `today`, and
/// completes it once on `today`. Returns (fixture, task id).
fn completed_streak_task(today: NaiveDate) -> (Fx, Id) {
    let mut f = Fx::on(today);
    let id = f.add_task(NewTask::new("read").recurrence(Recurrence::Daily));
    f.streak(&id);
    f.complete(&id);
    (f, id)
}

fn streak_of(f: &Fx, id: &Id, today: NaiveDate) -> i64 {
    f.svc
        .list_tasks_at(f.inbox.as_str(), today)
        .unwrap()
        .into_iter()
        .find(|t| &t.id == id)
        .unwrap()
        .streak_current
}

#[test]
fn completion_then_list_shows_streak() {
    let (f, id) = completed_streak_task(d(2026, 1, 5));
    assert_eq!(streak_of(&f, &id, d(2026, 1, 5)), 1);
    // The next day the occurrence is due but not missed yet.
    assert_eq!(streak_of(&f, &id, d(2026, 1, 6)), 1);
}

#[test]
fn list_resets_streak_after_missed_period_and_persists() {
    let (mut f, id) = completed_streak_task(d(2026, 1, 5));
    // Jan 6 came and went uncompleted.
    assert_eq!(streak_of(&f, &id, d(2026, 1, 7)), 0);

    // Reopen: the reset was written, not just computed on the fly.
    f.reopen();
    let stored = f
        .svc
        .list_tasks_at(f.inbox.as_str(), d(2026, 1, 5)) // a date that wouldn't itself reset
        .unwrap()
        .into_iter()
        .find(|t| t.id == id)
        .unwrap();
    assert_eq!(stored.streak_current, 0);
    assert_eq!(stored.streak_best, 1, "best record kept");
}
