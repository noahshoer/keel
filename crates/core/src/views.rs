//! Pure date helpers shared by the service views and the UI (Todoist-style
//! due chips and grouping). `today` is always injected.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::model::Task;

/// Which colored due chip a date gets, relative to today.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DueBucket {
    Overdue,
    Today,
    Tomorrow,
    /// 2–6 days ahead.
    ThisWeek,
    Later,
}

pub fn due_bucket(due: NaiveDate, today: NaiveDate) -> DueBucket {
    match (due - today).num_days() {
        n if n < 0 => DueBucket::Overdue,
        0 => DueBucket::Today,
        1 => DueBucket::Tomorrow,
        2..=6 => DueBucket::ThisWeek,
        _ => DueBucket::Later,
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const WEEKDAYS_LONG: [&str; 7] = [
    "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
];

/// Todoist-style chip text: Today / Tomorrow / Yesterday / weekday / "3 Nov"
/// (with the year when it differs from today's).
pub fn due_label(due: NaiveDate, today: NaiveDate) -> String {
    match (due - today).num_days() {
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        -1 => "Yesterday".to_string(),
        2..=6 => WEEKDAYS_LONG[due.weekday().num_days_from_monday() as usize].to_string(),
        _ => {
            let month = MONTHS[due.month0() as usize];
            if due.year() == today.year() {
                format!("{} {}", due.day(), month)
            } else {
                format!("{} {} {}", due.day(), month, due.year())
            }
        }
    }
}

/// The calendar date of a task's due (`YYYY-MM-DD` or a datetime string).
pub fn due_date_of(task: &Task) -> Option<NaiveDate> {
    task.due
        .as_ref()
        .and_then(|d| d.date.get(..10))
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}

#[cfg(test)]
mod tests {
    use super::{due_bucket, due_date_of, due_label, DueBucket};
    use crate::ids::Id;
    use crate::model::{Due, Task};
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn buckets_around_today() {
        let today = d(2026, 10, 19); // Monday
        assert_eq!(due_bucket(d(2026, 10, 18), today), DueBucket::Overdue);
        assert_eq!(due_bucket(today, today), DueBucket::Today);
        assert_eq!(due_bucket(d(2026, 10, 20), today), DueBucket::Tomorrow);
        assert_eq!(due_bucket(d(2026, 10, 21), today), DueBucket::ThisWeek);
        assert_eq!(due_bucket(d(2026, 10, 25), today), DueBucket::ThisWeek);
        assert_eq!(due_bucket(d(2026, 10, 26), today), DueBucket::Later);
    }

    #[test]
    fn buckets_cross_year_boundary() {
        assert_eq!(due_bucket(d(2027, 1, 1), d(2026, 12, 31)), DueBucket::Tomorrow);
        assert_eq!(due_bucket(d(2026, 12, 31), d(2027, 1, 1)), DueBucket::Overdue);
    }

    #[test]
    fn labels() {
        let today = d(2026, 10, 19);
        assert_eq!(due_label(today, today), "Today");
        assert_eq!(due_label(d(2026, 10, 20), today), "Tomorrow");
        assert_eq!(due_label(d(2026, 10, 18), today), "Yesterday");
        assert_eq!(due_label(d(2026, 10, 23), today), "Friday");
        assert_eq!(due_label(d(2026, 11, 3), today), "3 Nov");
        assert_eq!(due_label(d(2026, 9, 1), today), "1 Sep");
        assert_eq!(due_label(d(2027, 3, 4), today), "4 Mar 2027");
    }

    #[test]
    fn due_date_of_reads_date_and_datetime() {
        let mut t = Task::new("x", Id::new());
        assert_eq!(due_date_of(&t), None);
        t.due = Some(Due {
            date: "2026-10-19T09:30:00".to_string(),
            timezone: None,
            string: "today at 9:30".to_string(),
            lang: "en".to_string(),
            is_recurring: false,
            recurrence: None,
        });
        assert_eq!(due_date_of(&t), Some(d(2026, 10, 19)));
    }
}
