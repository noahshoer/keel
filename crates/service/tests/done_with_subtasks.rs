//! "Done with sub-tasks" (issue #11): finishing a parent's last sub-task
//! completes the parent, and reopening a sub-task reopens a done parent.
use chrono::NaiveDate;
use keel_core::commands::{Action, ApplyResult};
use keel_core::ids::Id;
use keel_core::model::Task;
use keel_core::recurrence::Recurrence;

mod common;
use common::Fx;

const FRIDAY: u8 = 4;

fn day(n: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, n).unwrap()
}

/// Monday 2026-01-05.
fn today() -> NaiveDate {
    day(5)
}

impl Fx {
    fn done_with(&mut self, id: &Id) {
        self.run(Action::TaskSetDoneWithSubtasks {
            id: id.clone(),
            enabled: true,
        });
    }

    fn complete_on(&mut self, id: &Id, on: NaiveDate) -> ApplyResult {
        self.run_on(Action::TaskComplete { id: id.clone() }, on)
    }

    fn reopen_on(&mut self, id: &Id, on: NaiveDate) {
        self.run_on(Action::TaskUncomplete { id: id.clone() }, on);
    }

    fn get_on(&self, id: &Id, on: NaiveDate) -> Task {
        self.svc
            .all_tasks_at(on)
            .unwrap()
            .into_iter()
            .find(|t| &t.id == id)
            .unwrap()
    }

    /// "Sharpen the Physical Saw": a daily parent with a streak and the
    /// setting, and three daily sub-tasks with streaks.
    fn sharpen(&mut self) -> (Id, [Id; 3]) {
        let parent = self.add_under(None, Some(Recurrence::Daily));
        self.streak(&parent);
        self.done_with(&parent);
        let subs = [(); 3].map(|_| self.add_under(Some(&parent), Some(Recurrence::Daily)));
        for s in &subs {
            self.streak(s);
        }
        (parent, subs)
    }
}

#[test]
fn sharpen_case() {
    let mut f = Fx::on(today());
    let (parent, [a, b, c]) = f.sharpen();

    assert!(f.complete(&a).auto_completed.is_empty());
    assert!(f.complete(&b).auto_completed.is_empty());
    assert_eq!(f.date(&parent), "2026-01-05");
    assert_eq!(f.get(&parent).streak_current, 0);

    let res = f.complete(&c);

    assert_eq!(res.auto_completed, vec![parent.clone()]);
    assert_eq!(f.date(&parent), "2026-01-06");
    assert_eq!(f.get(&parent).streak_current, 1);
    for s in [&a, &b, &c] {
        assert_eq!(f.get(s).streak_current, 1, "{s:?} counted once");
    }
}

#[test]
fn flag_off_does_nothing() {
    let mut f = Fx::on(today());
    let parent = f.add_under(None, Some(Recurrence::Daily));
    let subs = [(); 3].map(|_| f.add_under(Some(&parent), Some(Recurrence::Daily)));

    for s in &subs {
        assert!(f.complete(s).auto_completed.is_empty());
    }

    assert_eq!(f.date(&parent), "2026-01-05");
}

#[test]
fn one_off_parent_is_checked() {
    let mut f = Fx::on(today());
    let parent = f.add_under(None, None);
    f.done_with(&parent);
    let a = f.add_under(Some(&parent), None);
    let b = f.add_under(Some(&parent), None);

    f.complete(&a);
    assert!(!f.get(&parent).checked);
    let res = f.complete(&b);

    assert_eq!(res.auto_completed, vec![parent.clone()]);
    assert!(f.get(&parent).checked);
}

#[test]
fn two_levels() {
    let mut f = Fx::on(today());
    let grandparent = f.add_under(None, None);
    f.done_with(&grandparent);
    let parent = f.add_under(Some(&grandparent), None);
    f.done_with(&parent);
    let sub = f.add_under(Some(&parent), None);

    let res = f.complete(&sub);

    assert_eq!(
        res.auto_completed,
        vec![parent.clone(), grandparent.clone()]
    );
    assert!(f.get(&parent).checked);
    assert!(f.get(&grandparent).checked);
}

#[test]
fn repeating_parent_waits_for_its_date() {
    let mut f = Fx::on(today());
    let parent = f.add_under(None, Some(Recurrence::Weekly(vec![FRIDAY])));
    f.done_with(&parent);
    assert_eq!(f.date(&parent), "2026-01-09");
    let sub = f.add_under(Some(&parent), Some(Recurrence::Daily));

    let res = f.complete(&sub);

    assert!(res.auto_completed.is_empty());
    assert_eq!(f.date(&parent), "2026-01-09");
}

#[test]
fn one_off_parent_waits_for_open_repeating_sub_task() {
    let mut f = Fx::on(today());
    let parent = f.add_under(None, None);
    f.done_with(&parent);
    let plain = f.add_under(Some(&parent), None);
    let daily = f.add_under(Some(&parent), Some(Recurrence::Daily));

    f.complete(&plain);

    assert!(!f.get(&parent).checked);
    assert!(!f.get(&daily).checked, "its repeat goes on");
}

#[test]
fn already_done_parent_is_left_alone() {
    let mut f = Fx::on(today());
    let parent = f.add_under(None, Some(Recurrence::Daily));
    f.streak(&parent);
    f.done_with(&parent);
    let sub = f.add_under(Some(&parent), Some(Recurrence::Weekly(vec![FRIDAY])));
    f.complete(&parent);
    assert_eq!(f.date(&parent), "2026-01-06");

    // Its weekly sub-task wasn't due, so completing the parent left it; done early now.
    let res = f.complete(&sub);

    assert!(res.auto_completed.is_empty());
    assert_eq!(f.date(&parent), "2026-01-06");
    assert_eq!(f.get(&parent).streak_current, 1);
}

#[test]
fn stale_reopen_counts_as_open() {
    let mut f = Fx::on(today());
    let parent = f.add_under(None, Some(Recurrence::Daily));
    f.done_with(&parent);
    let a = f.add_under(Some(&parent), None);
    let _b = f.add_under(Some(&parent), None);
    f.complete(&parent);
    assert_eq!(f.date(&parent), "2026-01-06");

    // The next day, before anything has read the tasks (which reopens them).
    let res = f.complete_on(&a, day(6));

    assert!(res.auto_completed.is_empty(), "B is open again today");
    assert_eq!(f.get_on(&parent, day(6)).due.unwrap().date, "2026-01-06");
    assert!(
        f.get_on(&a, day(6)).checked,
        "A was completed today and stays so"
    );
}

#[test]
fn replayed_complete_does_not_cascade_again() {
    let mut f = Fx::on(today());
    let (parent, [a, b, c]) = f.sharpen();
    f.complete(&a);
    f.complete(&b);

    f.send("same", Action::TaskComplete { id: c.clone() }, today());
    f.send("same", Action::TaskComplete { id: c.clone() }, today());

    assert_eq!(f.date(&parent), "2026-01-06");
    assert_eq!(f.get(&parent).streak_current, 1);
}

#[test]
fn reopening_a_sub_task_reopens_the_parent() {
    let mut f = Fx::on(today());
    let (parent, [a, b, c]) = f.sharpen();
    for s in [&a, &b, &c] {
        f.complete(s);
    }
    assert_eq!(f.date(&parent), "2026-01-06");

    f.reopen_on(&a, today());

    assert_eq!(f.date(&parent), "2026-01-05");
    assert_eq!(f.get(&parent).streak_current, 0);
    let all = f.svc.all_tasks_at(today()).unwrap();
    for s in [&b, &c] {
        assert!(
            keel_core::views::is_done(all.iter().find(|t| &t.id == s).unwrap(), today()),
            "{s:?} stays done"
        );
    }
}

#[test]
fn reopening_reopens_a_parent_without_the_flag() {
    let mut f = Fx::on(today());
    let parent = f.add_under(None, None);
    let a = f.add_under(Some(&parent), None);
    let b = f.add_under(Some(&parent), None);
    f.complete(&parent);

    f.reopen_on(&a, today());

    assert!(!f.get(&parent).checked);
    assert!(f.get(&b).checked);
}

#[test]
fn reopening_today_leaves_yesterdays_parent_alone() {
    let mut f = Fx::on(today());
    let (parent, [a, b, c]) = f.sharpen();
    for s in [&a, &b, &c] {
        f.complete(s);
    }

    f.complete_on(&a, day(6));
    f.reopen_on(&a, day(6));

    assert_eq!(f.get_on(&parent, day(6)).due.unwrap().date, "2026-01-06");
    assert_eq!(f.get_on(&parent, day(6)).streak_current, 1);
}

#[test]
fn reopening_climbs_two_levels() {
    let mut f = Fx::on(today());
    let grandparent = f.add_under(None, None);
    f.done_with(&grandparent);
    let parent = f.add_under(Some(&grandparent), None);
    f.done_with(&parent);
    let sub = f.add_under(Some(&parent), None);
    f.complete(&sub);

    f.reopen_on(&sub, today());

    assert!(!f.get(&parent).checked);
    assert!(!f.get(&grandparent).checked);
}

#[test]
fn reopening_days_later_leaves_a_weekly_parent_alone() {
    let mut f = Fx::on(today());
    let friday = Recurrence::Weekly(vec![FRIDAY]);
    let parent = f.add_under(None, Some(friday.clone()));
    f.streak(&parent);
    f.done_with(&parent);
    let sub = f.add_under(Some(&parent), Some(friday));
    f.complete_on(&sub, day(9));
    assert_eq!(f.get_on(&parent, day(9)).due.unwrap().date, "2026-01-16");

    // Saturday: a mis-tap reopens the sub-task done yesterday.
    f.reopen_on(&sub, day(10));

    let p = f.get_on(&parent, day(10));
    assert_eq!(
        p.due.unwrap().date,
        "2026-01-16",
        "Friday's completion stands"
    );
    assert_eq!(p.streak_current, 1);
}

#[test]
fn reopening_an_open_sub_task_leaves_the_parent_alone() {
    let mut f = Fx::on(today());
    let parent = f.add_under(None, Some(Recurrence::Daily));
    // Due Friday, so completing the parent today leaves it open.
    let sub = f.add_under(Some(&parent), Some(Recurrence::Weekly(vec![FRIDAY])));
    f.complete(&parent);
    assert_eq!(f.date(&parent), "2026-01-06");

    f.reopen_on(&sub, today());

    assert_eq!(f.date(&parent), "2026-01-06");
}
