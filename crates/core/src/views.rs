//! Pure date helpers shared by the service views and the UI (Todoist-style
//! due chips and grouping). `today` is always injected.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::model::Task;

/// Which colored due chip a date gets, relative to today.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
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

/// Sub-task completion counts for one parent.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

impl Progress {
    /// True when there is at least one counted sub-task and all are checked.
    pub fn all_done(&self) -> bool {
        self.total > 0 && self.done == self.total
    }

    /// Compact count for rows: "2/5".
    pub fn short(&self) -> String {
        format!("{}/{}", self.done, self.total)
    }

    /// Spelled-out count for the task view header: "2 of 5".
    pub fn long(&self) -> String {
        format!("{} of {}", self.done, self.total)
    }
}

/// Counts a task's direct, non-deleted sub-tasks. Repeating sub-tasks are
/// left out (they never stay complete). `None` when nothing is counted.
pub fn progress(parent: &Id, all: &[Task]) -> Option<Progress> {
    let mut p = Progress { done: 0, total: 0 };
    for t in all {
        if t.is_deleted || t.parent_id.as_ref() != Some(parent) || t.due.as_ref().is_some_and(|d| d.is_recurring) {
            continue;
        }
        p.total += 1;
        if t.checked {
            p.done += 1;
        }
    }
    (p.total > 0).then_some(p)
}

/// A task's direct, non-deleted sub-tasks ordered by `child_order`, then `added_at`.
pub fn children<'a>(parent: &Id, all: &'a [Task]) -> Vec<&'a Task> {
    let mut kids: Vec<&Task> =
        all.iter().filter(|t| !t.is_deleted && t.parent_id.as_ref() == Some(parent)).collect();
    kids.sort_by(|a, b| a.child_order.cmp(&b.child_order).then(a.added_at.cmp(&b.added_at)));
    kids
}

/// One task in a card, with its sub-task counts.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CardRow {
    pub task: Task,
    pub progress: Option<Progress>,
}

/// A head task and the matched rows grouped under it. `head_matches == false`
/// means the head is context only (no circle; tapping it opens the head task).
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Card {
    pub head: CardRow,
    pub head_matches: bool,
    pub rows: Vec<CardRow>,
}

/// How [`cards`] groups a view's matches.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum CardMode {
    /// Only top-level matches, one card each. A task whose parent is in
    /// another project counts as top-level here.
    TopLevel,
    /// Every match, grouped under its top-level ancestor.
    Grouped,
}

/// The top-level ancestor of `task`. The walk stops at a parent that is
/// missing or deleted, and a `parent_id` cycle makes the task its own root.
pub fn root<'a>(task: &'a Task, all: &'a [Task]) -> &'a Task {
    root_where(task, all, |_| true)
}

/// [`root`], walking only through parents that pass `keep`.
fn root_where<'a>(task: &'a Task, all: &'a [Task], keep: impl Fn(&Task) -> bool) -> &'a Task {
    let mut cur = task;
    let mut seen = vec![&task.id];
    while let Some(pid) = cur.parent_id.as_ref() {
        let Some(parent) = all.iter().find(|t| !t.is_deleted && &t.id == pid && keep(t)) else {
            break;
        };
        if seen.contains(&&parent.id) {
            return task;
        }
        seen.push(&parent.id);
        cur = parent;
    }
    cur
}

/// Groups a view's matched tasks into cards (see `CardMode`). Cards keep the
/// order of their first matched member; rows keep the input order.
pub fn cards(matched: Vec<Task>, all: &[Task], mode: CardMode) -> Vec<Card> {
    let row = |task: Task| {
        let progress = progress(&task.id, all);
        CardRow { task, progress }
    };
    if mode == CardMode::TopLevel {
        // A sub-task moved alone to another project heads its own card there:
        // the walk goes only through tasks in the same project.
        let tops: Vec<Task> = matched
            .into_iter()
            .filter(|t| root_where(t, all, |p| p.project_id == t.project_id).id == t.id)
            .collect();
        return tops
            .into_iter()
            .map(|t| Card { head: row(t), head_matches: true, rows: Vec::new() })
            .collect();
    }
    let mut out: Vec<(Id, Card)> = Vec::new();
    for task in matched {
        let root_task = root(&task, all);
        let root_id = root_task.id.clone();
        let is_root = root_id == task.id;
        let unmatched_head = (!is_root).then(|| root_task.clone());
        let idx = match out.iter().position(|(id, _)| *id == root_id) {
            Some(i) => i,
            None => {
                let head = unmatched_head.unwrap_or_else(|| task.clone());
                out.push((root_id, Card { head: row(head), head_matches: is_root, rows: Vec::new() }));
                out.len() - 1
            }
        };
        let card = &mut out[idx].1;
        if is_root {
            card.head = row(task);
            card.head_matches = true;
        } else {
            card.rows.push(row(task));
        }
    }
    out.into_iter().map(|(_, c)| c).collect()
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

/// A titled run of tasks in the dated views (Today, Upcoming).
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DateGroup {
    /// "Overdue", "Today" or a short date; `None` for Today's single group.
    pub title: Option<String>,
    /// The grey text after the title ("30 Oct · Friday", "Tomorrow · Saturday").
    pub muted: String,
    pub tasks: Vec<Task>,
}

/// Which dated view [`date_groups`] splits for.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum DateGroupMode {
    /// Overdue, then today (titled only when there is something overdue).
    Today,
    /// Overdue, then one group per day.
    Upcoming,
}

/// "30 Oct".
fn short_date(d: NaiveDate) -> String {
    format!("{} {}", d.day(), MONTHS[d.month0() as usize])
}

fn weekday_long(d: NaiveDate) -> &'static str {
    WEEKDAYS_LONG[d.weekday().num_days_from_monday() as usize]
}

/// The Today view's subtitle: "Friday 30 Oct".
pub fn day_subtitle(today: NaiveDate) -> String {
    format!("{} {}", weekday_long(today), short_date(today))
}

/// Splits Today / Upcoming into Todoist-style date groups, keeping the
/// tasks' order within each. Tasks without a date are left out of
/// Upcoming's day groups.
pub fn date_groups(mode: DateGroupMode, tasks: Vec<Task>, today: NaiveDate) -> Vec<DateGroup> {
    let (overdue, rest): (Vec<Task>, Vec<Task>) =
        tasks.into_iter().partition(|t| due_date_of(t).is_some_and(|d| d < today));
    let mut out = Vec::new();
    let split = !overdue.is_empty();
    if split {
        out.push(DateGroup { title: Some("Overdue".into()), muted: String::new(), tasks: overdue });
    }
    match mode {
        DateGroupMode::Today => {
            if !rest.is_empty() {
                out.push(DateGroup {
                    title: split.then(|| "Today".to_string()),
                    muted: if split { format!("{} · {}", short_date(today), weekday_long(today)) } else { String::new() },
                    tasks: rest,
                });
            }
        }
        DateGroupMode::Upcoming => {
            let mut days: Vec<(NaiveDate, Vec<Task>)> = Vec::new();
            for t in rest {
                let Some(d) = due_date_of(&t) else { continue };
                match days.last_mut() {
                    Some((last, group)) if *last == d => group.push(t),
                    _ => days.push((d, vec![t])),
                }
            }
            out.extend(days.into_iter().map(|(d, tasks)| {
                let rel = match (d - today).num_days() {
                    0 => "Today · ",
                    1 => "Tomorrow · ",
                    _ => "",
                };
                DateGroup { title: Some(short_date(d)), muted: format!("{rel}{}", weekday_long(d)), tasks }
            }));
        }
    }
    out
}

/// The Upcoming view's tasks: what's overdue (from the Today query, which
/// has it) on top, then the upcoming window.
pub fn upcoming_tasks(today_tasks: Vec<Task>, window: Vec<Task>, today: NaiveDate) -> Vec<Task> {
    let mut out: Vec<Task> =
        today_tasks.into_iter().filter(|t| due_date_of(t).is_some_and(|d| d < today)).collect();
    out.extend(window);
    out
}

/// A deadline chip turns urgent on the day itself and after.
pub fn deadline_urgent(deadline: NaiveDate, today: NaiveDate) -> bool {
    deadline <= today
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

#[cfg(test)]
mod progress_tests {
    use super::{children, progress, Progress};
    use crate::ids::Id;
    use crate::model::{Due, Task};

    fn t(name: &str, parent: Option<&Task>, checked: bool, recurring: bool) -> Task {
        let mut task = Task::new(name, Id::from("P"));
        task.parent_id = parent.map(|p| p.id.clone());
        task.checked = checked;
        if recurring {
            task.due = Some(Due {
                date: "2026-10-23".into(),
                timezone: None,
                string: "every day".into(),
                lang: "en".into(),
                is_recurring: true,
                recurrence: None,
            });
        }
        task
    }

    #[test]
    fn progress_none_without_children() {
        let p = t("p", None, false, false);
        assert_eq!(progress(&p.id, &[p.clone()]), None);
    }

    #[test]
    fn progress_counts_direct_children_only() {
        let p = t("p", None, false, false);
        let a = t("a", Some(&p), true, false);
        let b = t("b", Some(&p), false, false);
        let c = t("c", Some(&p), false, false);
        let a1 = t("a1", Some(&a), false, false);
        let a2 = t("a2", Some(&a), false, false);
        let all = vec![p.clone(), a.clone(), b, c, a1, a2];
        assert_eq!(progress(&p.id, &all), Some(Progress { done: 1, total: 3 }));
        assert_eq!(progress(&a.id, &all), Some(Progress { done: 0, total: 2 }));
    }

    #[test]
    fn progress_skips_repeating_children() {
        let p = t("p", None, false, false);
        let a = t("a", Some(&p), true, false);
        let h = t("h", Some(&p), false, true);
        let pr = progress(&p.id, &[p.clone(), a, h]).unwrap();
        assert_eq!(pr, Progress { done: 1, total: 1 });
        assert!(pr.all_done());
    }

    #[test]
    fn progress_none_when_only_repeating_children() {
        let p = t("p", None, false, false);
        let h = t("h", Some(&p), false, true);
        assert_eq!(progress(&p.id, &[p.clone(), h]), None);
    }

    #[test]
    fn progress_skips_deleted_children() {
        let p = t("p", None, false, false);
        let a = t("a", Some(&p), false, false);
        let mut d = t("d", Some(&p), true, false);
        d.is_deleted = true;
        assert_eq!(progress(&p.id, &[p.clone(), a, d]), Some(Progress { done: 0, total: 1 }));
    }

    #[test]
    fn all_done_needs_every_child() {
        assert!(Progress { done: 5, total: 5 }.all_done());
        assert!(!Progress { done: 4, total: 5 }.all_done());
        assert!(!Progress { done: 0, total: 0 }.all_done());
    }

    #[test]
    fn progress_text() {
        assert_eq!(Progress { done: 2, total: 5 }.short(), "2/5");
        assert_eq!(Progress { done: 5, total: 5 }.long(), "5 of 5");
    }

    #[test]
    fn children_in_child_order() {
        let p = t("p", None, false, false);
        let mut a = t("a", Some(&p), false, false);
        let mut b = t("b", Some(&p), false, false);
        let mut c = t("c", Some(&p), false, false);
        let mut d = t("d", Some(&p), false, false);
        a.child_order = 2;
        b.child_order = 0;
        c.child_order = 1;
        d.child_order = 3;
        d.is_deleted = true;
        let all = vec![p.clone(), a, b, c, d];
        let names: Vec<&str> = children(&p.id, &all).iter().map(|t| t.content.as_str()).collect();
        assert_eq!(names, vec!["b", "c", "a"]);
    }
}

#[cfg(test)]
mod card_tests {
    use super::{cards, progress, root, Card, CardMode, Progress};
    use crate::ids::Id;
    use crate::model::{Due, Priority, Task};

    struct Fx {
        pack: Task,
        chargers: Task,
        passport: Task,
        clothes: Task,
        call: Task,
    }

    fn task(name: &str, parent: Option<&Task>) -> Task {
        let mut t = Task::new(name, Id::from("P"));
        t.parent_id = parent.map(|p| p.id.clone());
        t
    }

    fn fx() -> Fx {
        let mut pack = task("Pack", None);
        pack.priority = Priority::P2;
        Fx {
            chargers: task("Chargers", Some(&pack)),
            passport: task("Passport", Some(&pack)),
            clothes: task("Clothes", Some(&pack)),
            call: task("Call", None),
            pack,
        }
    }

    fn all(f: &Fx) -> Vec<Task> {
        vec![f.pack.clone(), f.chargers.clone(), f.passport.clone(), f.clothes.clone(), f.call.clone()]
    }

    fn heads(cs: &[Card]) -> Vec<String> {
        cs.iter().map(|c| c.head.task.content.clone()).collect()
    }

    fn rows(c: &Card) -> Vec<String> {
        c.rows.iter().map(|r| r.task.content.clone()).collect()
    }

    #[test]
    fn toplevel_hides_subtasks() {
        let f = fx();
        let m = vec![f.call.clone(), f.pack.clone(), f.chargers.clone(), f.passport.clone(), f.clothes.clone()];
        let cs = cards(m, &all(&f), CardMode::TopLevel);
        assert_eq!(heads(&cs), ["Call", "Pack"]);
        assert!(cs.iter().all(|c| c.head_matches && c.rows.is_empty()));
    }

    #[test]
    fn toplevel_keeps_orphans() {
        let f = fx();
        let mut orphan = task("Orphan", None);
        orphan.parent_id = Some(Id::from("gone"));
        let mut a = all(&f);
        a.push(orphan.clone());
        let cs = cards(vec![orphan], &a, CardMode::TopLevel);
        assert_eq!(heads(&cs), ["Orphan"]);
    }

    #[test]
    fn grouped_subtask_under_unmatched_parent() {
        let f = fx();
        let cs = cards(vec![f.call.clone(), f.chargers.clone()], &all(&f), CardMode::Grouped);
        assert_eq!(heads(&cs), ["Call", "Pack"]);
        assert!(cs[0].head_matches);
        assert!(!cs[1].head_matches);
        assert_eq!(rows(&cs[1]), ["Chargers"]);
    }

    #[test]
    fn grouped_siblings_share_card() {
        let f = fx();
        let m = vec![f.chargers.clone(), f.call.clone(), f.passport.clone()];
        let cs = cards(m, &all(&f), CardMode::Grouped);
        assert_eq!(heads(&cs), ["Pack", "Call"]);
        assert_eq!(rows(&cs[0]), ["Chargers", "Passport"]);
    }

    #[test]
    fn grouped_parent_also_matches() {
        let f = fx();
        let m = vec![f.chargers.clone(), f.pack.clone(), f.passport.clone()];
        let cs = cards(m, &all(&f), CardMode::Grouped);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].head.task.content, "Pack");
        assert!(cs[0].head_matches);
        assert_eq!(rows(&cs[0]), ["Chargers", "Passport"]);
    }

    #[test]
    fn grouped_grandchild_goes_to_root() {
        let f = fx();
        let tooth = task("Toothbrush", Some(&f.chargers));
        let mut a = all(&f);
        a.push(tooth.clone());
        let cs = cards(vec![tooth], &a, CardMode::Grouped);
        assert_eq!(heads(&cs), ["Pack"]);
        assert_eq!(rows(&cs[0]), ["Toothbrush"]);
    }

    #[test]
    fn grouped_completed_parent_is_context() {
        let mut f = fx();
        f.pack.checked = true;
        let mut h = task("H", Some(&f.pack));
        h.due = Some(Due {
            date: "2026-10-23".into(),
            timezone: None,
            string: "every day".into(),
            lang: "en".into(),
            is_recurring: true,
            recurrence: None,
        });
        let mut a = all(&f);
        a.push(h.clone());
        let cs = cards(vec![h], &a, CardMode::Grouped);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].head.task.content, "Pack");
        assert!(!cs[0].head_matches);
        assert_eq!(rows(&cs[0]), ["H"]);
    }

    #[test]
    fn cards_group_across_projects() {
        let mut f = fx();
        f.chargers.project_id = Id::from("other");
        let cs = cards(vec![f.chargers.clone()], &all(&f), CardMode::Grouped);
        assert_eq!(heads(&cs), ["Pack"]);
        assert_eq!(rows(&cs[0]), ["Chargers"]);
    }

    #[test]
    fn toplevel_shows_subtask_moved_to_other_project() {
        let mut f = fx();
        f.chargers.project_id = Id::from("other");
        let mut tooth = task("Toothbrush", Some(&f.chargers));
        tooth.project_id = Id::from("other");
        let mut a = all(&f);
        a.push(tooth.clone());
        // The other project's list: Chargers heads it, its child stays under it.
        let cs = cards(vec![f.chargers.clone(), tooth], &a, CardMode::TopLevel);
        assert_eq!(heads(&cs), ["Chargers"]);
        // Pack's own project still hides its same-project sub-tasks.
        let cs = cards(vec![f.pack.clone(), f.passport.clone()], &a, CardMode::TopLevel);
        assert_eq!(heads(&cs), ["Pack"]);
    }

    #[test]
    fn cycle_does_not_hang() {
        let mut a = task("A", None);
        let mut b = task("B", None);
        a.parent_id = Some(b.id.clone());
        b.parent_id = Some(a.id.clone());
        let all = vec![a.clone(), b.clone()];
        assert_eq!(root(&a, &all).content, "A");
        for mode in [CardMode::TopLevel, CardMode::Grouped] {
            let cs = cards(vec![a.clone(), b.clone()], &all, mode);
            let mut seen: Vec<String> = Vec::new();
            for c in &cs {
                seen.push(c.head.task.content.clone());
                seen.extend(rows(c));
            }
            seen.sort();
            assert_eq!(seen, ["A", "B"]);
        }
    }

    #[test]
    fn card_rows_carry_progress() {
        let mut f = fx();
        let cs = cards(vec![f.pack.clone()], &all(&f), CardMode::Grouped);
        assert_eq!(cs[0].head.progress, Some(Progress { done: 0, total: 3 }));
        f.chargers.checked = true;
        f.passport.checked = true;
        let a = all(&f);
        let cs = cards(vec![f.pack.clone(), f.clothes.clone()], &a, CardMode::Grouped);
        assert_eq!(cs[0].head.progress, Some(Progress { done: 2, total: 3 }));
        assert_eq!(cs[0].rows[0].progress, progress(&f.clothes.id, &a));
    }

    #[test]
    fn all_done_head() {
        let mut f = fx();
        f.chargers.checked = true;
        f.passport.checked = true;
        f.clothes.checked = true;
        let cs = cards(vec![f.pack.clone()], &all(&f), CardMode::Grouped);
        assert!(cs[0].head.progress.unwrap().all_done());
        assert!(cs[0].rows.is_empty());
    }
}

#[cfg(test)]
mod date_group_tests {
    use super::{cards, date_groups, deadline_urgent, upcoming_tasks, CardMode, DateGroupMode, Progress};
    use crate::ids::Id;
    use crate::model::{Due, Task};
    use chrono::{Datelike, NaiveDate};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn task(name: &str, parent: Option<&Task>, due: NaiveDate) -> Task {
        let mut t = Task::new(name, Id::from("proj"));
        t.id = Id::from(name);
        t.parent_id = parent.map(|p| p.id.clone());
        t.due = Some(Due {
            date: due.format("%Y-%m-%d").to_string(),
            timezone: None,
            string: String::new(),
            lang: "en".to_string(),
            is_recurring: false,
            recurrence: None,
        });
        t
    }

    fn titles(groups: &[super::DateGroup]) -> Vec<(Option<String>, String)> {
        groups.iter().map(|g| (g.title.clone(), g.muted.clone())).collect()
    }

    #[test]
    fn today_splits_overdue_only_when_there_is_some() {
        let today = d(2026, 10, 30);
        let due_today = task("Laundry", None, today);
        let groups = date_groups(DateGroupMode::Today, vec![due_today.clone()], today);
        assert_eq!(titles(&groups), [(None, String::new())]);

        let late = task("Taxes", None, d(2026, 10, 28));
        let groups = date_groups(DateGroupMode::Today, vec![due_today, late], today);
        assert_eq!(
            titles(&groups),
            [(Some("Overdue".to_string()), String::new()), (Some("Today".to_string()), "30 Oct · Friday".to_string())]
        );
        assert_eq!(groups[0].tasks[0].content, "Taxes");
        assert_eq!(groups[1].tasks[0].content, "Laundry");
    }

    #[test]
    fn upcoming_labels_today_and_tomorrow() {
        let today = d(2026, 10, 30);
        let tasks = vec![task("A", None, today), task("B", None, d(2026, 10, 31)), task("C", None, d(2026, 11, 1))];
        let groups = date_groups(DateGroupMode::Upcoming, tasks, today);
        assert_eq!(
            titles(&groups),
            [
                (Some("30 Oct".to_string()), "Today · Friday".to_string()),
                (Some("31 Oct".to_string()), "Tomorrow · Saturday".to_string()),
                (Some("1 Nov".to_string()), "Sunday".to_string()),
            ]
        );
    }

    #[test]
    fn upcoming_groups_cards_per_day() {
        let (today, thu, fri) = (d(2026, 10, 19), d(2026, 10, 22), d(2026, 10, 23));
        assert_eq!(thu.weekday(), chrono::Weekday::Thu);
        let parent = task("Pack for Boston", None, fri);
        let sub = task("Chargers", Some(&parent), thu);
        let all = vec![parent.clone(), sub.clone()];
        let groups = date_groups(DateGroupMode::Upcoming, vec![sub.clone(), parent.clone()], today);
        assert_eq!(groups.len(), 2);

        let thursday = cards(groups[0].tasks.clone(), &all, CardMode::Grouped);
        assert_eq!(groups[0].title.as_deref(), Some("22 Oct"));
        assert_eq!(thursday.len(), 1);
        assert_eq!(thursday[0].head.task.id, parent.id);
        assert!(!thursday[0].head_matches, "the parent is a context-only head on Thursday");
        let rows: Vec<_> = thursday[0].rows.iter().map(|r| r.task.id.clone()).collect();
        assert_eq!(rows, vec![sub.id.clone()]);
        assert_eq!(thursday[0].head.progress, Some(Progress { done: 0, total: 1 }));

        let friday = cards(groups[1].tasks.clone(), &all, CardMode::Grouped);
        assert_eq!(groups[1].title.as_deref(), Some("23 Oct"));
        assert_eq!(friday.len(), 1);
        assert_eq!(friday[0].head.task.id, parent.id);
        assert!(friday[0].head_matches, "on Friday the parent is its own completable card");
        assert!(friday[0].rows.is_empty());
    }

    #[test]
    fn upcoming_tasks_puts_overdue_first_and_drops_today() {
        let today = d(2026, 10, 30);
        let late = task("Late", None, d(2026, 10, 29));
        let due_today = task("Now", None, today);
        let soon = task("Soon", None, d(2026, 11, 2));
        let window = vec![due_today.clone(), soon.clone()];
        let out = upcoming_tasks(vec![due_today, late], window, today);
        let names: Vec<_> = out.iter().map(|t| t.content.as_str()).collect();
        assert_eq!(names, ["Late", "Now", "Soon"]);
    }

    #[test]
    fn deadline_today_is_urgent() {
        let today = d(2026, 10, 30);
        assert!(deadline_urgent(d(2026, 10, 29), today));
        assert!(deadline_urgent(today, today));
        assert!(!deadline_urgent(d(2026, 10, 31), today));
    }
}

#[cfg(test)]
mod subtitle_tests {
    use super::day_subtitle;
    use chrono::NaiveDate;

    #[test]
    fn the_today_subtitle_names_the_weekday_and_date() {
        assert_eq!(day_subtitle(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()), "Friday 30 Oct");
        assert_eq!(day_subtitle(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()), "Sunday 1 Nov");
    }
}
