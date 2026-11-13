//! What a list page remembers about how it is arranged: a filter query, the
//! fields it is headed by, and the order of its tasks.

use serde::{Deserialize, Serialize};

use chrono::NaiveDate;

use crate::filter::{self, FilterContext};
use crate::ids::Id;
use crate::model::{parse_ymd_prefix, Label, Priority, Project, Section, Task};
use crate::views::{due_date_of, due_time, short_date, weekday_long};

/// A field a page can be grouped or ordered by. Append-only: stored as JSON.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Project,
    Section,
    Label,
    Priority,
    DueDate,
    Deadline,
}

/// One level of a page's order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct OrderLevel {
    pub field: SortField,
    #[serde(default)]
    pub reversed: bool,
}

/// A page's filter, heading fields and order levels.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Arrangement {
    #[serde(default)]
    pub filter: String,
    #[serde(default)]
    pub groups: Vec<SortField>,
    #[serde(default)]
    pub order: Vec<OrderLevel>,
}

/// What an arrangement does to its page.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ArrangementFlags {
    /// It has a filter (a blank one doesn't count).
    pub filtered: bool,
    /// It groups or orders the page.
    pub sorted: bool,
}

impl Arrangement {
    /// What it does to the page.
    pub fn flags(&self) -> ArrangementFlags {
        ArrangementFlags {
            filtered: !self.filter.trim().is_empty(),
            sorted: !self.groups.is_empty() || !self.order.is_empty(),
        }
    }

    /// True when the page is arranged as it is without any setting.
    pub fn is_default(&self) -> bool {
        let flags = self.flags();
        !flags.filtered && !flags.sorted
    }

    /// The filter trimmed; a field repeated in `groups` or in `order` keeps
    /// its first entry.
    pub fn normalized(mut self) -> Arrangement {
        self.filter = self.filter.trim().to_string();
        let mut seen = Vec::new();
        self.groups.retain(|f| {
            let new = !seen.contains(f);
            seen.push(*f);
            new
        });
        let mut seen = Vec::new();
        self.order.retain(|l| {
            let new = !seen.contains(&l.field);
            seen.push(l.field);
            new
        });
        self
    }
}

/// Which page an arrangement belongs to.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ViewPlace {
    Today,
    Upcoming,
    Project { id: Id },
    Label { id: Id },
    Filter { id: Id },
}

impl ViewPlace {
    /// The storage key: "today", "upcoming", "project:<id>", "label:<id>",
    /// "filter:<id>".
    pub fn key(&self) -> String {
        match self {
            ViewPlace::Today => "today".into(),
            ViewPlace::Upcoming => "upcoming".into(),
            ViewPlace::Project { id } => format!("project:{}", id.as_str()),
            ViewPlace::Label { id } => format!("label:{}", id.as_str()),
            ViewPlace::Filter { id } => format!("filter:{}", id.as_str()),
        }
    }
}

/// An arrangement with the page it belongs to.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PlacedArrangement {
    pub place: ViewPlace,
    pub arrangement: Arrangement,
}

/// Orders two optional sort values: a missing value is always after a present
/// one; `reversed` flips only the order between two present values.
fn cmp_missing_last<T: Ord>(a: Option<T>, b: Option<T>, reversed: bool) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Some(a), Some(b)) if reversed => b.cmp(&a),
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn field_name(f: SortField) -> &'static str {
    match f {
        SortField::Project => "Project",
        SortField::Section => "Section",
        SortField::Label => "Label",
        SortField::Priority => "Priority",
        SortField::DueDate => "Due date",
        SortField::Deadline => "Deadline",
    }
}

/// One field's row in the sort menu.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SortChoice {
    pub field: SortField,
    pub name: String,
    /// What the normal direction is called.
    pub order_label: String,
    /// What the reversed direction is called.
    pub reversed_label: String,
}

/// The six fields in menu order, with their direction labels.
pub fn sort_choices() -> Vec<SortChoice> {
    let rows = [
        (SortField::Project, "Sidebar order", "Reversed"),
        (SortField::Section, "Section order", "Reversed"),
        (SortField::Label, "Label order", "Reversed"),
        (SortField::Priority, "Highest first", "Lowest first"),
        (SortField::DueDate, "Soonest first", "Latest first"),
        (SortField::Deadline, "Soonest first", "Latest first"),
    ];
    rows.into_iter()
        .map(|(field, order, reversed)| SortChoice {
            field,
            name: field_name(field).into(),
            order_label: order.into(),
            reversed_label: reversed.into(),
        })
        .collect()
}

/// "Project, Section, Label · Due date, Priority"; one half alone has no dot;
/// empty with no sort. A reversed level reads with its flipped label in
/// brackets: "Due date (latest first)".
pub fn summary(arrangement: &Arrangement) -> String {
    let groups = arrangement
        .groups
        .iter()
        .map(|f| field_name(*f).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let choices = sort_choices();
    let order = arrangement
        .order
        .iter()
        .map(|l| {
            if l.reversed {
                let label = choices
                    .iter()
                    .find(|c| c.field == l.field)
                    .map_or("", |c| c.reversed_label.as_str());
                format!("{} ({})", field_name(l.field), label.to_lowercase())
            } else {
                field_name(l.field).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    match (groups.is_empty(), order.is_empty()) {
        (true, true) => String::new(),
        (false, true) => groups,
        (true, false) => order,
        (false, false) => format!("{groups} \u{b7} {order}"),
    }
}

/// What `arrange` reads names and positions from.
pub struct ArrangeNames<'a> {
    pub projects: &'a [Project],
    pub sections: &'a [Section],
    pub labels: &'a [Label],
}

/// The innermost heading of a group, drawn under its title line.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SubHeading {
    pub text: String,
    /// A label heading's color name.
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub label_color: Option<String>,
}

/// One run of tasks under the same heading values.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ArrangedGroup {
    /// Unique on the page and stable across reloads (built from the heading
    /// values' ids).
    pub key: String,
    /// Every heading level but the innermost, joined with " › "; with one
    /// level, that level. `None` with no headings.
    pub title: Option<String>,
    /// This group opens a new title line.
    pub starts_title: bool,
    /// The innermost level when there are two or more.
    pub sub: Option<SubHeading>,
    pub tasks: Vec<Task>,
}

/// A page's tasks, grouped and ordered.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Arranged {
    pub groups: Vec<ArrangedGroup>,
    pub filter_problem: Option<String>,
    /// Distinct tasks in `groups`.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub shown: usize,
    /// Distinct tasks given.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub total: usize,
}

type Rank = (i64, i64, String);

/// One heading value of one entry.
struct Heading {
    key: String,
    rank: Rank,
    text: String,
    color: Option<String>,
}

/// A task as it sits in one group (once per label when headed by Label).
struct Entry {
    task: Task,
    headings: Vec<Heading>,
    /// The task's value at each order level, worked out once (not per comparison).
    order: Vec<Option<Rank>>,
}

fn known_label<'a>(names: &ArrangeNames<'a>, name: &str) -> Option<(usize, &'a Label)> {
    names
        .labels
        .iter()
        .enumerate()
        .find(|(_, l)| !l.is_deleted && l.name.eq_ignore_ascii_case(name))
}

/// Where a project sits in the sidebar; `None` when it isn't there.
fn project_pos(names: &ArrangeNames, id: &Id) -> Option<usize> {
    names.projects.iter().position(|p| &p.id == id)
}

/// A project's sidebar position as a rank; an unknown project sorts after every known one.
fn project_rank(names: &ArrangeNames, id: &Id) -> i64 {
    project_pos(names, id).map_or(i64::MAX, |p| p as i64)
}

fn section_of<'a>(names: &'a ArrangeNames, task: &Task) -> Option<&'a Section> {
    let id = task.section_id.as_ref()?;
    names.sections.iter().find(|s| &s.id == id)
}

fn parse_day(s: &str) -> Option<NaiveDate> {
    parse_ymd_prefix(s)
}

fn day_heading(
    prefix: &str,
    date: Option<NaiveDate>,
    none_text: &str,
    today: NaiveDate,
) -> (String, Rank, String) {
    match date {
        Some(d) if d < today => (
            format!("{prefix}:overdue"),
            (0, 0, String::new()),
            "Overdue".into(),
        ),
        Some(d) => (
            format!("{prefix}:{d}"),
            (1, day_rank(d), String::new()),
            match (d - today).num_days() {
                0 => format!("{} · Today · {}", short_date(d), weekday_long(d)),
                1 => format!("{} · Tomorrow · {}", short_date(d), weekday_long(d)),
                _ => format!("{} · {}", short_date(d), weekday_long(d)),
            },
        ),
        None => (
            format!("{prefix}:none"),
            (2, 0, String::new()),
            none_text.into(),
        ),
    }
}

fn priority_number(p: Priority) -> i64 {
    match p {
        Priority::P1 => 1,
        Priority::P2 => 2,
        Priority::P3 => 3,
        Priority::None => 4,
    }
}

fn heading_for(
    field: SortField,
    entry_task: &Task,
    label: Option<&String>,
    names: &ArrangeNames,
    today: NaiveDate,
    section_with_project: bool,
) -> Heading {
    match field {
        SortField::Project => {
            let text = project_pos(names, &entry_task.project_id)
                .map_or("No project", |p| names.projects[p].name.as_str());
            Heading {
                key: format!("project:{}", entry_task.project_id.as_str()),
                rank: (
                    project_rank(names, &entry_task.project_id),
                    0,
                    String::new(),
                ),
                text: text.into(),
                color: None,
            }
        }
        SortField::Section => {
            let ppos = project_rank(names, &entry_task.project_id);
            let project =
                project_pos(names, &entry_task.project_id).map(|p| names.projects[p].name.as_str());
            let (key, order, name) = match section_of(names, entry_task) {
                Some(s) => (
                    format!("section:{}", s.id.as_str()),
                    s.section_order,
                    s.name.as_str(),
                ),
                None => (
                    format!("section:none:{}", entry_task.project_id.as_str()),
                    i64::MIN,
                    "No section",
                ),
            };
            let text = match project {
                Some(p) if section_with_project => format!("{p} › {name}"),
                _ => name.to_string(),
            };
            Heading {
                key,
                rank: (ppos, order, String::new()),
                text,
                color: None,
            }
        }
        SortField::Label => match label {
            Some(name) => match known_label(names, name) {
                Some((pos, l)) => Heading {
                    key: format!("label:{}", l.id.as_str()),
                    rank: (0, pos as i64, String::new()),
                    text: l.name.clone(),
                    color: Some(l.color.clone()),
                },
                None => Heading {
                    key: format!("label:name:{}", name.to_lowercase()),
                    rank: (1, 0, name.to_lowercase()),
                    text: name.clone(),
                    color: None,
                },
            },
            None => Heading {
                key: "label:none".into(),
                rank: (2, 0, String::new()),
                text: "No label".into(),
                color: None,
            },
        },
        SortField::Priority => {
            let n = priority_number(entry_task.priority);
            Heading {
                key: format!("priority:{n}"),
                rank: (n, 0, String::new()),
                text: format!("Priority {n}"),
                color: None,
            }
        }
        SortField::DueDate => {
            let (key, rank, text) = day_heading("due", due_date_of(entry_task), "No date", today);
            Heading {
                key,
                rank,
                text,
                color: None,
            }
        }
        SortField::Deadline => {
            let date = entry_task.deadline.as_deref().and_then(parse_day);
            let (key, rank, text) = day_heading("deadline", date, "No deadline", today);
            Heading {
                key,
                rank,
                text,
                color: None,
            }
        }
    }
}

/// A day as a number that sorts in date order.
fn day_rank(d: NaiveDate) -> i64 {
    i64::from(chrono::Datelike::num_days_from_ce(&d))
}

/// The value one order level sorts a task by; `None` is a missing value.
fn order_rank(field: SortField, task: &Task, names: &ArrangeNames) -> Option<Rank> {
    match field {
        SortField::Project => Some((project_rank(names, &task.project_id), 0, String::new())),
        SortField::Section => Some((
            project_rank(names, &task.project_id),
            section_of(names, task).map_or(i64::MIN, |s| s.section_order),
            String::new(),
        )),
        SortField::Label => task
            .labels
            .iter()
            .map(|n| match known_label(names, n) {
                Some((pos, _)) => (0, pos as i64, String::new()),
                None => (1, 0, n.to_lowercase()),
            })
            .min(),
        SortField::Priority => Some((priority_number(task.priority), 0, String::new())),
        SortField::DueDate => due_date_of(task).map(|d| {
            let time = due_time(task).map_or(i64::MAX, |t| {
                i64::from(chrono::Timelike::num_seconds_from_midnight(&t))
            });
            (day_rank(d), time, String::new())
        }),
        SortField::Deadline => task
            .deadline
            .as_deref()
            .and_then(parse_day)
            .map(|d| (day_rank(d), 0, String::new())),
    }
}

/// Keeps the tasks the arrangement's filter matches. A blank filter keeps
/// everything; a filter that doesn't read (a syntax error or an unknown name)
/// keeps everything too and returns what is wrong with it.
fn apply_filter(
    tasks: Vec<Task>,
    query: &str,
    names: &ArrangeNames,
    today: NaiveDate,
) -> (Vec<Task>, Option<String>) {
    if query.trim().is_empty() {
        return (tasks, None);
    }
    let (reading, parsed) =
        filter::read_parsed(query, today, names.projects, names.sections, names.labels);
    let groups = match (reading.problem, parsed) {
        (Some(problem), _) => return (tasks, Some(problem)),
        (None, Err(e)) => return (tasks, Some(e.message)),
        (None, Ok(groups)) => groups,
    };
    let ctx = FilterContext {
        today,
        projects: names
            .projects
            .iter()
            .map(|p| (p.id.clone(), p.name.clone()))
            .collect(),
        sections: names
            .sections
            .iter()
            .map(|s| (s.id.clone(), s.name.clone()))
            .collect(),
    };
    let kept = tasks
        .into_iter()
        .filter(|t| groups.iter().any(|g| filter::matches(g, t, &ctx)))
        .collect();
    (kept, None)
}

/// Filters a page's tasks, then groups them under the arrangement's headings
/// and orders them.
pub fn arrange(
    tasks: Vec<Task>,
    arrangement: &Arrangement,
    names: &ArrangeNames,
    today: NaiveDate,
) -> Arranged {
    let mut seen = std::collections::HashSet::new();
    let tasks: Vec<Task> = tasks
        .into_iter()
        .filter(|t| seen.insert(t.id.clone()))
        .collect();
    let total = tasks.len();
    let (tasks, filter_problem) = apply_filter(tasks, &arrangement.filter, names, today);
    let fields = &arrangement.groups;

    let project_heading_at = fields.iter().position(|f| *f == SortField::Project);
    let section_at = fields.iter().position(|f| *f == SortField::Section);
    let many_projects = tasks
        .iter()
        .map(|t| &t.project_id)
        .collect::<std::collections::HashSet<_>>()
        .len()
        > 1;
    let section_with_project = many_projects
        && match (section_at, project_heading_at) {
            (Some(s), Some(p)) => p > s,
            (Some(_), None) => true,
            _ => false,
        };

    let by_label = fields.contains(&SortField::Label);
    let mut entries: Vec<Entry> = Vec::new();
    for task in tasks {
        let labels: Vec<Option<String>> = if by_label {
            let mut out: Vec<Option<String>> = Vec::new();
            for l in &task.labels {
                if !out.iter().flatten().any(|o| o.eq_ignore_ascii_case(l)) {
                    out.push(Some(l.clone()));
                }
            }
            if out.is_empty() {
                out.push(None);
            }
            out
        } else {
            vec![None]
        };
        let order: Vec<Option<Rank>> = arrangement
            .order
            .iter()
            .map(|level| order_rank(level.field, &task, names))
            .collect();
        for label in labels {
            let headings = fields
                .iter()
                .map(|f| {
                    heading_for(
                        *f,
                        &task,
                        label.as_ref(),
                        names,
                        today,
                        section_with_project,
                    )
                })
                .collect();
            entries.push(Entry {
                task: task.clone(),
                headings,
                order: order.clone(),
            });
        }
    }

    entries.sort_by(|a, b| {
        for (x, y) in a.headings.iter().zip(&b.headings) {
            let c = x.rank.cmp(&y.rank).then_with(|| x.key.cmp(&y.key));
            if c.is_ne() {
                return c;
            }
        }
        for ((x, y), level) in a.order.iter().zip(&b.order).zip(&arrangement.order) {
            let c = cmp_missing_last(x.as_ref(), y.as_ref(), level.reversed);
            if c.is_ne() {
                return c;
            }
        }
        std::cmp::Ordering::Equal
    });

    let mut groups: Vec<ArrangedGroup> = Vec::new();
    let mut group_keys: Vec<Vec<String>> = Vec::new();
    for e in entries {
        let keys: Vec<String> = e.headings.iter().map(|h| h.key.clone()).collect();
        if group_keys.last() == Some(&keys) && !groups.is_empty() {
            groups.last_mut().expect("a group").tasks.push(e.task);
            continue;
        }
        let n = e.headings.len();
        let (title, sub, key) = if n == 0 {
            (None, None, "all".to_string())
        } else {
            if n == 1 {
                (Some(e.headings[0].text.clone()), None, keys.join("/"))
            } else {
                let title = e.headings[..n - 1]
                    .iter()
                    .map(|h| h.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" › ");
                let last = &e.headings[n - 1];
                (
                    Some(title),
                    Some(SubHeading {
                        text: last.text.clone(),
                        label_color: last.color.clone(),
                    }),
                    keys.join("/"),
                )
            }
        };
        groups.push(ArrangedGroup {
            key,
            title,
            starts_title: false,
            sub,
            tasks: vec![e.task],
        });
        group_keys.push(keys);
    }
    if groups.is_empty() && fields.is_empty() {
        groups.push(ArrangedGroup {
            key: "all".into(),
            title: None,
            starts_title: false,
            sub: None,
            tasks: Vec::new(),
        });
    }

    // Title lines: a run of groups sharing every key but the innermost.
    let prefix_len = fields
        .len()
        .saturating_sub(1)
        .max(usize::from(fields.len() == 1));
    let prefixes: Vec<&[String]> = group_keys
        .iter()
        .map(|k| &k[..prefix_len.min(k.len())])
        .collect();
    if !fields.is_empty() {
        for (i, group) in groups.iter_mut().enumerate() {
            group.starts_title = i == 0 || prefixes[i] != prefixes[i - 1];
        }
    }

    let shown = groups
        .iter()
        .flat_map(|g| g.tasks.iter().map(|t| &t.id))
        .collect::<std::collections::HashSet<_>>()
        .len();
    Arranged {
        groups,
        filter_problem,
        shown,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_keys() {
        let id = Id::from("a".to_string());
        assert_eq!(ViewPlace::Today.key(), "today");
        assert_eq!(ViewPlace::Upcoming.key(), "upcoming");
        assert_eq!(ViewPlace::Project { id: id.clone() }.key(), "project:a");
        assert_eq!(ViewPlace::Label { id: id.clone() }.key(), "label:a");
        assert_eq!(ViewPlace::Filter { id }.key(), "filter:a");
    }

    #[test]
    fn flags_say_what_an_arrangement_does() {
        let blank = Arrangement {
            filter: "  ".into(),
            ..Arrangement::default()
        };
        assert_eq!(
            blank.flags(),
            ArrangementFlags {
                filtered: false,
                sorted: false
            }
        );
        let filtered = Arrangement {
            filter: "p1".into(),
            ..Arrangement::default()
        };
        assert!(filtered.flags().filtered && !filtered.flags().sorted);
        let grouped = Arrangement {
            groups: vec![SortField::Label],
            ..Arrangement::default()
        };
        assert!(grouped.flags().sorted && !grouped.flags().filtered);
        let ordered = Arrangement {
            order: vec![OrderLevel {
                field: SortField::Priority,
                reversed: false,
            }],
            ..Arrangement::default()
        };
        assert!(ordered.flags().sorted);
        for a in [blank, filtered, grouped, ordered] {
            let f = a.flags();
            assert_eq!(a.is_default(), !f.filtered && !f.sorted);
        }
    }

    #[test]
    fn arrangement_reads_from_empty_object() {
        let a: Arrangement = serde_json::from_str("{}").unwrap();
        assert!(a.is_default());
    }

    use crate::model::{Due, Label, Priority, Project, Section, Task};
    use chrono::NaiveDate;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    struct World {
        projects: Vec<Project>,
        sections: Vec<Section>,
        labels: Vec<Label>,
    }

    fn world() -> World {
        let mut home = Project::new("Home");
        home.id = Id::from("home".to_string());
        let mut work = Project::new("Work");
        work.id = Id::from("work".to_string());
        let mut kitchen = Section::new(home.id.clone(), "Kitchen", 1);
        kitchen.id = Id::from("kitchen".to_string());
        let mut desk = Section::new(work.id.clone(), "Kitchen", 1);
        desk.id = Id::from("kitchen2".to_string());
        World {
            projects: vec![home, work],
            sections: vec![kitchen, desk],
            labels: vec![Label::new("errand"), Label::new("phone")],
        }
    }

    impl World {
        fn names(&self) -> ArrangeNames<'_> {
            ArrangeNames {
                projects: &self.projects,
                sections: &self.sections,
                labels: &self.labels,
            }
        }
    }

    fn task(id: &str) -> Task {
        let mut t = Task::new(id, Id::from("home".to_string()));
        t.id = Id::from(id.to_string());
        t
    }
    fn in_project(mut t: Task, p: &str) -> Task {
        t.project_id = Id::from(p.to_string());
        t
    }
    fn in_section(mut t: Task, s: &str) -> Task {
        t.section_id = Some(Id::from(s.to_string()));
        t
    }
    fn labelled(mut t: Task, l: &[&str]) -> Task {
        t.labels = l.iter().map(|s| s.to_string()).collect();
        t
    }
    fn prio(mut t: Task, p: Priority) -> Task {
        t.priority = p;
        t
    }
    fn due(mut t: Task, d: &str) -> Task {
        t.due = Some(Due {
            date: d.into(),
            timezone: None,
            string: d.into(),
            lang: "en".into(),
            is_recurring: false,
            recurrence: None,
            until: None,
            moved_from: None,
        });
        t
    }
    fn deadline(mut t: Task, d: &str) -> Task {
        t.deadline = Some(d.into());
        t
    }
    fn arr(groups: &[SortField], order: &[(SortField, bool)]) -> Arrangement {
        Arrangement {
            filter: String::new(),
            groups: groups.to_vec(),
            order: order
                .iter()
                .map(|&(field, reversed)| OrderLevel { field, reversed })
                .collect(),
        }
    }
    fn ids(g: &ArrangedGroup) -> Vec<String> {
        g.tasks.iter().map(|t| t.id.as_str().to_string()).collect()
    }
    fn run(w: &World, tasks: Vec<Task>, a: &Arrangement) -> Arranged {
        arrange(tasks, a, &w.names(), today())
    }

    #[test]
    fn no_headings_is_one_group_in_input_order() {
        let w = world();
        let r = run(&w, vec![task("b"), task("a")], &arr(&[], &[]));
        assert_eq!(r.groups.len(), 1);
        assert_eq!(r.groups[0].key, "all");
        assert_eq!(r.groups[0].title, None);
        assert!(r.groups[0].sub.is_none());
        assert_eq!(ids(&r.groups[0]), ["b", "a"]);
        let empty = run(&w, vec![], &arr(&[], &[]));
        assert_eq!(empty.groups.len(), 1);
        assert!(empty.groups[0].tasks.is_empty());
        assert_eq!((r.shown, r.total), (2, 2));
    }

    #[test]
    fn noahs_example() {
        let w = world();
        let tasks = vec![
            prio(
                due(
                    in_section(labelled(task("k-today-p3"), &["errand"]), "kitchen"),
                    "2026-11-09",
                ),
                Priority::P3,
            ),
            prio(
                due(
                    in_section(labelled(task("k-today-p1"), &["errand"]), "kitchen"),
                    "2026-11-09",
                ),
                Priority::P1,
            ),
            prio(
                due(
                    in_section(labelled(task("k-yest-p3"), &["errand"]), "kitchen"),
                    "2026-11-08",
                ),
                Priority::P3,
            ),
            in_section(labelled(task("k-phone"), &["phone"]), "kitchen"),
            labelled(task("h-errand"), &["errand"]),
            in_project(task("w-none"), "work"),
        ];
        let a = arr(
            &[SortField::Project, SortField::Section, SortField::Label],
            &[(SortField::DueDate, false), (SortField::Priority, false)],
        );
        let r = run(&w, tasks, &a);
        let seq: Vec<(String, String, Vec<String>)> = r
            .groups
            .iter()
            .map(|g| {
                (
                    g.title.clone().unwrap(),
                    g.sub.clone().unwrap().text,
                    ids(g),
                )
            })
            .collect();
        let s = |a: &str, b: &str, c: &[&str]| {
            (
                a.to_string(),
                b.to_string(),
                c.iter().map(|x| x.to_string()).collect::<Vec<_>>(),
            )
        };
        assert_eq!(
            seq,
            vec![
                s("Home › No section", "errand", &["h-errand"]),
                s(
                    "Home › Kitchen",
                    "errand",
                    &["k-yest-p3", "k-today-p1", "k-today-p3"]
                ),
                s("Home › Kitchen", "phone", &["k-phone"]),
                s("Work › No section", "No label", &["w-none"]),
            ]
        );
        let starts: Vec<bool> = r.groups.iter().map(|g| g.starts_title).collect();
        assert_eq!(starts, [true, true, false, true]);
    }

    #[test]
    fn one_heading_has_no_sub() {
        let w = world();
        let tasks = vec![task("none"), prio(task("p1"), Priority::P1)];
        let r = run(&w, tasks, &arr(&[SortField::Priority], &[]));
        let titles: Vec<_> = r.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(titles, ["Priority 1", "Priority 4"]);
        assert!(r.groups.iter().all(|g| g.sub.is_none()));
    }

    #[test]
    fn two_labels_two_groups() {
        let w = world();
        let r = run(
            &w,
            vec![labelled(task("t"), &["errand", "phone"])],
            &arr(&[SortField::Label], &[]),
        );
        assert_eq!(r.groups.len(), 2);
        assert_eq!(r.shown, 1);
        assert_eq!(r.total, 1);
    }

    #[test]
    fn missing_values_go_last_both_directions() {
        let w = world();
        for (field, set) in [
            (SortField::DueDate, due as fn(Task, &str) -> Task),
            (SortField::Deadline, deadline),
        ] {
            for reversed in [false, true] {
                let tasks = vec![
                    task("none"),
                    set(task("a"), "2026-11-10"),
                    set(task("b"), "2026-11-12"),
                ];
                let r = run(&w, tasks, &arr(&[], &[(field, reversed)]));
                let got = ids(&r.groups[0]);
                assert_eq!(got.last().unwrap(), "none");
                assert_eq!(got[0], if reversed { "b" } else { "a" });
            }
        }
    }

    #[test]
    fn due_date_headings() {
        let w = world();
        let tasks = vec![
            task("n"),
            due(task("t"), "2026-11-09"),
            due(task("o"), "2026-11-01"),
        ];
        let r = run(&w, tasks, &arr(&[SortField::DueDate], &[]));
        let titles: Vec<_> = r.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(titles, ["Overdue", "9 Nov · Today · Monday", "No date"]);
        let r = run(
            &w,
            vec![
                due(task("a"), "2026-11-10"),
                due(task("b"), "2026-11-13"),
                due(task("t"), "2026-11-09"),
            ],
            &arr(&[SortField::DueDate], &[]),
        );
        let titles: Vec<_> = r.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(
            titles,
            [
                "9 Nov · Today · Monday",
                "10 Nov · Tomorrow · Tuesday",
                "13 Nov · Friday"
            ]
        );
        let r = run(
            &w,
            vec![task("n"), deadline(task("d"), "2026-11-09")],
            &arr(&[SortField::Deadline], &[]),
        );
        let titles: Vec<_> = r.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(titles, ["9 Nov · Today · Monday", "No deadline"]);
    }

    #[test]
    fn timed_before_untimed_on_the_same_day() {
        let w = world();
        let tasks = vec![
            due(task("untimed"), "2026-11-10"),
            due(task("late"), "2026-11-10T15:00:00"),
            due(task("early"), "2026-11-10T08:30:00"),
        ];
        let r = run(&w, tasks, &arr(&[], &[(SortField::DueDate, false)]));
        assert_eq!(ids(&r.groups[0]), ["early", "late", "untimed"]);
    }

    #[test]
    fn label_as_an_order_uses_the_first_label() {
        let w = world();
        // Label order is the labels' own order: errand, then phone.
        let tasks = vec![
            labelled(task("none"), &[]),
            labelled(task("phone"), &["phone"]),
            labelled(task("both"), &["phone", "errand"]),
            labelled(task("errand"), &["errand"]),
        ];
        let r = run(&w, tasks, &arr(&[], &[(SortField::Label, false)]));
        assert_eq!(ids(&r.groups[0]), ["both", "errand", "phone", "none"]);
    }

    #[test]
    fn section_names_its_project_across_projects() {
        let w = world();
        let tasks = vec![
            in_section(task("a"), "kitchen"),
            in_project(in_section(task("b"), "kitchen2"), "work"),
        ];
        let r = run(&w, tasks, &arr(&[SortField::Section], &[]));
        let titles: Vec<_> = r.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(titles, ["Home › Kitchen", "Work › Kitchen"]);
    }

    #[test]
    fn section_alone_in_one_project() {
        let w = world();
        let r = run(
            &w,
            vec![task("n"), in_section(task("a"), "kitchen")],
            &arr(&[SortField::Section], &[]),
        );
        let titles: Vec<_> = r.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(titles, ["No section", "Kitchen"]);
    }

    #[test]
    fn ties_keep_input_order() {
        let w = world();
        let tasks = vec![task("c"), task("a"), task("b")];
        let r = run(&w, tasks, &arr(&[], &[(SortField::Priority, false)]));
        assert_eq!(ids(&r.groups[0]), ["c", "a", "b"]);
    }

    #[test]
    fn shared_task_appears_once() {
        let w = world();
        let r = run(
            &w,
            vec![task("x"), task("x")],
            &arr(&[SortField::Priority], &[]),
        );
        assert_eq!(r.groups.len(), 1);
        assert_eq!(ids(&r.groups[0]), ["x"]);
        assert_eq!((r.shown, r.total), (1, 1));
    }

    #[test]
    fn group_keys_are_unique_and_stable() {
        let w = world();
        let tasks = vec![
            in_section(task("a"), "kitchen"),
            in_project(in_section(task("b"), "kitchen2"), "work"),
        ];
        let a = arr(&[SortField::Section], &[]);
        let r1 = run(&w, tasks.clone(), &a);
        let r2 = run(&w, tasks, &a);
        let k1: Vec<_> = r1.groups.iter().map(|g| g.key.clone()).collect();
        let k2: Vec<_> = r2.groups.iter().map(|g| g.key.clone()).collect();
        assert_eq!(k1, k2);
        assert_ne!(k1[0], k1[1]);
    }

    fn with_filter(f: &str) -> Arrangement {
        Arrangement {
            filter: f.into(),
            ..Arrangement::default()
        }
    }

    fn filter_tasks() -> Vec<Task> {
        let mut v = vec![
            prio(labelled(task("e1"), &["errand"]), Priority::P1),
            prio(labelled(task("e2"), &["errand"]), Priority::P1),
            prio(labelled(task("e3"), &["errand"]), Priority::P2),
            labelled(task("ph"), &["phone"]),
            prio(task("p1only"), Priority::P1),
        ];
        for i in 0..9 {
            v.push(task(&format!("x{i}")));
        }
        v
    }

    #[test]
    fn filter_narrows() {
        let w = world();
        let r = run(&w, filter_tasks(), &with_filter("@errand & p1"));
        assert_eq!(ids(&r.groups[0]), ["e1", "e2"]);
        assert_eq!((r.shown, r.total), (2, 14));
        assert!(r.filter_problem.is_none());
    }

    #[test]
    fn comma_groups_match_any() {
        let w = world();
        let r = run(&w, filter_tasks(), &with_filter("p1, @phone"));
        assert_eq!(ids(&r.groups[0]), ["e1", "e2", "ph", "p1only"]);
        assert_eq!(r.shown, 4);
    }

    #[test]
    fn blank_filter_keeps_everything() {
        let w = world();
        let r = run(&w, filter_tasks(), &with_filter("   "));
        assert_eq!((r.shown, r.total), (14, 14));
        assert!(r.filter_problem.is_none());
    }

    #[test]
    fn broken_filter_shows_everything() {
        let w = world();
        for q in ["#Gone & p1", "p1 &"] {
            let r = run(&w, filter_tasks(), &with_filter(q));
            assert!(r.filter_problem.is_some(), "{q}");
            assert_eq!(r.shown, r.total, "{q}");
        }
    }

    #[test]
    fn relative_dates_follow_today() {
        let w = world();
        let tasks = vec![due(task("a"), "2026-11-09"), due(task("b"), "2026-11-10")];
        let a = with_filter("today");
        let r1 = arrange(tasks.clone(), &a, &w.names(), today());
        let r2 = arrange(
            tasks,
            &a,
            &w.names(),
            NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
        );
        assert_eq!(ids(&r1.groups[0]), ["a"]);
        assert_eq!(ids(&r2.groups[0]), ["b"]);
    }

    #[test]
    fn filter_then_headings() {
        let w = world();
        let mut a = with_filter("p1");
        a.groups = vec![SortField::Label];
        let r = run(&w, filter_tasks(), &a);
        let titles: Vec<_> = r.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(titles, ["errand", "No label"]);
    }

    #[test]
    fn summary_texts() {
        let a = arr(
            &[SortField::Project, SortField::Section, SortField::Label],
            &[(SortField::DueDate, false), (SortField::Priority, false)],
        );
        assert_eq!(
            summary(&a),
            "Project, Section, Label \u{b7} Due date, Priority"
        );
        assert_eq!(summary(&arr(&[SortField::Project], &[])), "Project");
        assert_eq!(
            summary(&arr(&[], &[(SortField::Priority, false)])),
            "Priority"
        );
        assert_eq!(summary(&arr(&[], &[])), "");
        assert_eq!(
            summary(&arr(
                &[SortField::Label],
                &[(SortField::DueDate, true), (SortField::Priority, false)]
            )),
            "Label \u{b7} Due date (latest first), Priority"
        );
    }

    #[test]
    fn sort_choices_cover_every_field() {
        let c = sort_choices();
        let fields: Vec<_> = c.iter().map(|x| x.field).collect();
        assert_eq!(
            fields,
            [
                SortField::Project,
                SortField::Section,
                SortField::Label,
                SortField::Priority,
                SortField::DueDate,
                SortField::Deadline
            ]
        );
        let due = &c[4];
        assert_eq!(
            (
                due.name.as_str(),
                due.order_label.as_str(),
                due.reversed_label.as_str()
            ),
            ("Due date", "Soonest first", "Latest first")
        );
        assert_eq!(c[0].order_label, "Sidebar order");
        assert_eq!(c[3].reversed_label, "Lowest first");
    }
}
