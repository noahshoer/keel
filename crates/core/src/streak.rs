//! Counter-based streaks (design §7). State lives on the task:
//! `streak_current`, `streak_best`, and `streak_last_period` (the ISO date of
//! the last credited occurrence). `today` is always injected.

use chrono::NaiveDate;

use crate::model::{parse_ymd, parse_ymd_prefix, ymd, LastCompletion, Task};
use crate::recurrence::Recurrence;

fn rule(task: &Task) -> Option<Recurrence> {
    task.due.as_ref().and_then(|d| d.recurrence.clone())
}

fn last_period(task: &Task) -> Option<NaiveDate> {
    task.streak_last_period.as_deref().and_then(parse_ymd)
}

/// A streak in words, in the unit its repeat counts: "4 day streak", or "Streak
/// of 4" when one occurrence isn't one day, week, month or year.
pub fn streak_text(task: &Task) -> String {
    let unit = match rule(task) {
        Some(Recurrence::Daily) => Some("day"),
        Some(Recurrence::Weekly(days)) if days.len() == 1 => Some("week"),
        // Every weekday: each occurrence is the next working day.
        Some(Recurrence::Weekly(days)) if days == [0, 1, 2, 3, 4] => Some("day"),
        Some(
            Recurrence::MonthlyDay(_)
            | Recurrence::MonthlyLastDay
            | Recurrence::MonthlyWorkday { .. }
            | Recurrence::MonthlyNthWeekday { .. },
        ) => Some("month"),
        Some(Recurrence::Yearly { .. }) => Some("year"),
        _ => None,
    };
    match unit {
        Some(unit) => format!("{} {unit} streak", task.streak_current),
        None => format!("Streak of {}", task.streak_current),
    }
}

/// Credit a completion on `today`. Consecutive with the previous occurrence →
/// +1; same period again → no change; any gap → restart at 1.
pub fn credit_completion(task: &mut Task, today: NaiveDate) {
    if !task.streak_enabled {
        return;
    }
    let Some(rule) = rule(task) else { return };
    match last_period(task) {
        Some(last) if last >= today => return,
        Some(last) if last == rule.prev_before(today) => task.streak_current += 1,
        _ => task.streak_current = 1,
    }
    task.streak_best = task.streak_best.max(task.streak_current);
    task.streak_last_period = Some(ymd(today));
}

/// The completion that undoing would put back, if there is one. One from before
/// completions were saved is worked out from its streak, while it's still today's.
pub fn undoable_completion(task: &Task, today: NaiveDate) -> Option<LastCompletion> {
    let due = task.due.as_ref().filter(|_| !task.checked)?;
    if let Some(saved) = task.last_completion.as_ref() {
        if saved.moved_to == due.date {
            return Some(saved.clone());
        }
    }
    let rule = due.recurrence.as_ref()?;
    let moved_to = parse_ymd_prefix(&due.date)?;
    if !task.streak_enabled
        || last_period(task) != Some(today)
        || rule.next_after(today) != moved_to
    {
        return None;
    }
    let streak_current = (task.streak_current - 1).max(0);
    Some(LastCompletion {
        moved_to: due.date.clone(),
        date: format!("{}{}", ymd(today), due.date.get(10..).unwrap_or("")),
        streak_current,
        streak_best: if task.streak_best == task.streak_current {
            streak_current
        } else {
            task.streak_best
        },
        streak_last_period: (streak_current > 0).then(|| ymd(rule.prev_before(today))),
        moved_from: None,
        completed_on: Some(ymd(today)),
    })
}

/// Hard reset to 0 once the occurrence after the last credited one is strictly
/// in the past (i.e. it came and went uncompleted).
pub fn recompute_miss(task: &mut Task, today: NaiveDate) {
    if !task.streak_enabled {
        return;
    }
    let (Some(rule), Some(last)) = (rule(task), last_period(task)) else {
        return;
    };
    if rule.next_after(last) < today {
        task.streak_current = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::{credit_completion, recompute_miss};
    use crate::ids::Id;
    use crate::model::{Due, Task};
    use crate::recurrence::Recurrence;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn streak_task(rule: Recurrence) -> Task {
        let mut t = Task::new("habit", Id::new());
        t.due = Some(Due {
            date: "2026-01-05".to_string(),
            timezone: None,
            string: "every day".to_string(),
            lang: "en".to_string(),
            is_recurring: true,
            recurrence: Some(rule),
            until: None,
            moved_from: None,
        });
        t.streak_enabled = true;
        t
    }

    #[test]
    fn first_completion_sets_current_1() {
        let mut t = streak_task(Recurrence::Daily);
        credit_completion(&mut t, d(2026, 1, 5));
        assert_eq!(t.streak_current, 1);
        assert_eq!(t.streak_best, 1);
        assert_eq!(t.streak_last_period.as_deref(), Some("2026-01-05"));
    }

    #[test]
    fn consecutive_completions_increment() {
        let mut t = streak_task(Recurrence::Daily);
        credit_completion(&mut t, d(2026, 1, 5));
        credit_completion(&mut t, d(2026, 1, 6));
        assert_eq!(t.streak_current, 2);
        assert_eq!(t.streak_best, 2);
    }

    #[test]
    fn same_period_completion_no_double_count() {
        let mut t = streak_task(Recurrence::Daily);
        credit_completion(&mut t, d(2026, 1, 5));
        credit_completion(&mut t, d(2026, 1, 5));
        assert_eq!(t.streak_current, 1);
    }

    #[test]
    fn gap_completion_restarts_at_1_and_keeps_best() {
        let mut t = streak_task(Recurrence::Daily);
        credit_completion(&mut t, d(2026, 1, 5));
        credit_completion(&mut t, d(2026, 1, 6));
        credit_completion(&mut t, d(2026, 1, 9));
        assert_eq!(t.streak_current, 1);
        assert_eq!(t.streak_best, 2, "best record survives the restart");
    }

    #[test]
    fn miss_resets_to_zero_only_after_due_day_passes() {
        let mut t = streak_task(Recurrence::Daily);
        credit_completion(&mut t, d(2026, 1, 5));
        recompute_miss(&mut t, d(2026, 1, 6)); // the next occurrence is today: intact
        assert_eq!(t.streak_current, 1);
        recompute_miss(&mut t, d(2026, 1, 7)); // Jan 6 elapsed uncompleted: miss
        assert_eq!(t.streak_current, 0);
        assert_eq!(t.streak_best, 1);
    }

    #[test]
    fn weekly_consecutive_across_weekdays() {
        // 0=Mon, 2=Wed. 2026-01-05 is a Monday.
        let mut t = streak_task(Recurrence::Weekly(vec![0, 2]));
        credit_completion(&mut t, d(2026, 1, 5));
        credit_completion(&mut t, d(2026, 1, 7));
        assert_eq!(t.streak_current, 2, "Mon then Wed is consecutive");
        recompute_miss(&mut t, d(2026, 1, 10)); // next is Mon Jan 12: still intact
        assert_eq!(t.streak_current, 2);
    }

    #[test]
    fn disabled_streak_is_untouched() {
        let mut t = streak_task(Recurrence::Daily);
        t.streak_enabled = false;
        credit_completion(&mut t, d(2026, 1, 5));
        assert_eq!(t.streak_current, 0);
    }
}

#[cfg(test)]
mod streak_text_tests {
    use super::streak_text;
    use crate::ids::Id;
    use crate::model::{Due, Task};
    use crate::recurrence::Recurrence;
    use chrono::NaiveDate;

    fn task(rule: Option<Recurrence>, current: i64) -> Task {
        let mut t = Task::new("t", Id::from("p"));
        t.streak_current = current as _;
        t.due = rule.map(|r| Due {
            date: "2026-11-01".into(),
            is_recurring: true,
            string: String::new(),
            lang: "en".into(),
            timezone: None,
            recurrence: Some(r),
            until: None,
            moved_from: None,
        });
        t
    }

    fn anchor() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
    }

    #[test]
    fn names_the_unit_a_repeat_counts_in() {
        assert_eq!(
            streak_text(&task(Some(Recurrence::Daily), 4)),
            "4 day streak"
        );
        assert_eq!(
            streak_text(&task(Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])), 4)),
            "4 day streak"
        );
        assert_eq!(
            streak_text(&task(Some(Recurrence::Weekly(vec![2])), 3)),
            "3 week streak"
        );
        assert_eq!(
            streak_text(&task(Some(Recurrence::MonthlyDay(1)), 2)),
            "2 month streak"
        );
        assert_eq!(
            streak_text(&task(Some(Recurrence::MonthlyLastDay), 2)),
            "2 month streak"
        );
        assert_eq!(
            streak_text(&task(Some(Recurrence::MonthlyWorkday { nth: 1 }), 2)),
            "2 month streak"
        );
        assert_eq!(
            streak_text(&task(
                Some(Recurrence::MonthlyNthWeekday { nth: 1, weekday: 0 }),
                2
            )),
            "2 month streak"
        );
        assert_eq!(
            streak_text(&task(Some(Recurrence::Yearly { month: 1, day: 1 }), 5)),
            "5 year streak"
        );
    }

    #[test]
    fn is_generic_when_a_repeat_has_no_single_unit() {
        let generic = "Streak of 3";
        assert_eq!(
            streak_text(&task(Some(Recurrence::Weekly(vec![0, 2])), 3)),
            generic
        );
        assert_eq!(
            streak_text(&task(
                Some(Recurrence::EveryNDays {
                    n: 3,
                    anchor: anchor()
                }),
                3
            )),
            generic
        );
        assert_eq!(
            streak_text(&task(
                Some(Recurrence::EveryNWeeks {
                    n: 2,
                    days: vec![1],
                    anchor: anchor()
                }),
                3
            )),
            generic
        );
        assert_eq!(
            streak_text(&task(
                Some(Recurrence::EveryNMonths {
                    n: 2,
                    day: 1,
                    anchor: anchor()
                }),
                3
            )),
            generic
        );
        assert_eq!(streak_text(&task(None, 3)), generic);
    }
}
