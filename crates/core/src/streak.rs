//! Counter-based streaks (design §7). State lives on the task:
//! `streak_current`, `streak_best`, and `streak_last_period` (the ISO date of
//! the last credited occurrence). `today` is always injected.

use chrono::NaiveDate;

use crate::model::Task;
use crate::recurrence::Recurrence;

fn rule(task: &Task) -> Option<Recurrence> {
    task.due.as_ref().and_then(|d| d.recurrence.clone())
}

fn last_period(task: &Task) -> Option<NaiveDate> {
    task.streak_last_period
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
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
    task.streak_last_period = Some(today.format("%Y-%m-%d").to_string());
}

/// Hard reset to 0 once the occurrence after the last credited one is strictly
/// in the past (i.e. it came and went uncompleted).
pub fn recompute_miss(task: &mut Task, today: NaiveDate) {
    if !task.streak_enabled {
        return;
    }
    let (Some(rule), Some(last)) = (rule(task), last_period(task)) else { return };
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
