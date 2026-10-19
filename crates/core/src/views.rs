//! Pure date helpers shared by the service views and the UI (Todoist-style
//! due chips and grouping). `today` is always injected.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::ids::Id;
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

/// A task positioned in a nested (sub-task) display.
#[derive(Clone, PartialEq, Debug)]
pub struct Nested {
    pub task: Task,
    pub depth: usize,
    /// True when the task has sub-tasks in this list (shown or collapsed).
    pub has_children: bool,
}

/// Orders a flat list for display: each sub-task follows its parent (depth+1),
/// siblings keep their input order, collapsed tasks hide their descendants, and
/// a sub-task whose parent isn't in the list shows at top level.
pub fn nest(tasks: Vec<Task>) -> Vec<Nested> {
    use std::collections::{HashMap, HashSet};

    let present: HashSet<&Id> = tasks.iter().map(|t| &t.id).collect();
    let mut children: HashMap<&Id, Vec<usize>> = HashMap::new();
    let mut roots = Vec::new();
    for (i, t) in tasks.iter().enumerate() {
        match &t.parent_id {
            Some(p) if p != &t.id && present.contains(p) => children.entry(p).or_default().push(i),
            _ => roots.push(i),
        }
    }

    fn visit<'a>(
        i: usize,
        depth: usize,
        tasks: &'a [Task],
        children: &HashMap<&'a Id, Vec<usize>>,
        seen: &mut HashSet<usize>,
        out: &mut Vec<Nested>,
    ) {
        if !seen.insert(i) {
            return; // cycle guard
        }
        let task = &tasks[i];
        let kids = children.get(&task.id).map(Vec::as_slice).unwrap_or(&[]);
        out.push(Nested { task: task.clone(), depth, has_children: !kids.is_empty() });
        if !task.is_collapsed {
            for &k in kids {
                visit(k, depth + 1, tasks, children, seen, out);
            }
        }
    }

    let mut out = Vec::with_capacity(tasks.len());
    let mut seen = HashSet::new();
    for &r in &roots {
        visit(r, 0, &tasks, &children, &mut seen, &mut out);
    }
    out
}

/// The time of day of a task's due, when it has one (`…THH:MM:SS`).
pub fn due_time(task: &Task) -> Option<chrono::NaiveTime> {
    task.due
        .as_ref()
        .and_then(|d| d.date.get(11..16))
        .and_then(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M").ok())
}

/// 12-hour label: "5 PM", "5:30 PM", "12 AM".
pub fn time_label(t: chrono::NaiveTime) -> String {
    use chrono::Timelike;
    let (pm, h12) = t.hour12();
    let suffix = if pm { "PM" } else { "AM" };
    if t.minute() == 0 {
        format!("{h12} {suffix}")
    } else {
        format!("{h12}:{:02} {suffix}", t.minute())
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

#[cfg(test)]
mod nest_tests {
    use super::nest;
    use crate::ids::Id;
    use crate::model::Task;

    fn t(name: &str, parent: Option<&Task>) -> Task {
        let mut task = Task::new(name, Id::from("P"));
        task.parent_id = parent.map(|p| p.id.clone());
        task
    }

    fn shape(tasks: Vec<Task>) -> Vec<(String, usize, bool)> {
        nest(tasks).into_iter().map(|n| (n.task.content, n.depth, n.has_children)).collect()
    }

    #[test]
    fn nests_children_under_parents_in_order() {
        let a = t("a", None);
        let b = t("b", None);
        let a1 = t("a1", Some(&a));
        let a2 = t("a2", Some(&a));
        // Input is flat and interleaved; children follow their parent, in input order.
        let got = shape(vec![a1.clone(), b.clone(), a.clone(), a2.clone()]);
        assert_eq!(
            got,
            vec![
                ("b".into(), 0, false),
                ("a".into(), 0, true),
                ("a1".into(), 1, false),
                ("a2".into(), 1, false),
            ]
        );
    }

    #[test]
    fn grandchildren_depth_2() {
        let a = t("a", None);
        let a1 = t("a1", Some(&a));
        let a1x = t("a1x", Some(&a1));
        let got = shape(vec![a, a1, a1x]);
        assert_eq!(got[2], ("a1x".into(), 2, false));
        assert!(got[1].2, "a1 has children");
    }

    #[test]
    fn collapsed_hides_descendants() {
        let mut a = t("a", None);
        let a1 = t("a1", Some(&a));
        let a1x = t("a1x", Some(&a1));
        let b = t("b", None);
        a.is_collapsed = true;
        let got = shape(vec![a, a1, a1x, b]);
        assert_eq!(got, vec![("a".into(), 0, true), ("b".into(), 0, false)]);
    }

    #[test]
    fn orphan_child_is_top_level() {
        let missing_parent = t("gone", None);
        let orphan = t("orphan", Some(&missing_parent));
        assert_eq!(shape(vec![orphan]), vec![("orphan".into(), 0, false)]);
    }
}

#[cfg(test)]
mod time_tests {
    use super::{due_time, time_label};
    use crate::ids::Id;
    use crate::model::{Due, Task};
    use chrono::NaiveTime;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn labels_12_hour() {
        assert_eq!(time_label(t(17, 0)), "5 PM");
        assert_eq!(time_label(t(17, 30)), "5:30 PM");
        assert_eq!(time_label(t(0, 0)), "12 AM");
        assert_eq!(time_label(t(12, 0)), "12 PM");
        assert_eq!(time_label(t(9, 5)), "9:05 AM");
    }

    #[test]
    fn reads_time_from_due() {
        let mut task = Task::new("x", Id::from("P"));
        assert_eq!(due_time(&task), None);
        let due = |date: &str| Due {
            date: date.into(),
            timezone: None,
            string: String::new(),
            lang: "en".into(),
            is_recurring: false,
            recurrence: None,
        };
        task.due = Some(due("2026-10-19"));
        assert_eq!(due_time(&task), None, "date-only due has no time");
        task.due = Some(due("2026-10-19T17:30:00"));
        assert_eq!(due_time(&task), Some(t(17, 30)));
    }
}
