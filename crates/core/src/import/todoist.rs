//! Todoist's project CSV export ("Manage data → Export as CSV") → Keel.
//!
//! `parse` reads the rows into items; `plan` turns them into an ordered list
//! of operations (sections, tasks with parent/section references) that the
//! service executes as normal commands. Both are pure.

use chrono::NaiveDate;

use crate::model::Priority;
use crate::quickadd;
use crate::recurrence::Recurrence;

/// A row we understood.
#[derive(Clone, Debug, PartialEq)]
pub enum TodoistItem {
    Section { name: String },
    Task(TodoistTask),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TodoistTask {
    pub content: String,
    pub description: String,
    /// Todoist's 1 (p1, highest) … 4 (p4); `None` when empty/invalid.
    pub priority: Option<u8>,
    /// 1 = top level, 2 = sub-task, …
    pub indent: usize,
    pub date: String,
    pub date_lang: String,
    pub deadline: String,
    pub deadline_lang: String,
    /// `note` rows that followed this task.
    pub comments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportError {
    /// The file has no `TYPE` / `CONTENT` header.
    NotTodoistCsv,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::NotTodoistCsv => {
                write!(f, "This doesn't look like a Todoist CSV export (no TYPE and CONTENT columns).")
            }
        }
    }
}

/// One step of an import. `key`s are positions in `ImportPlan::ops`, so later
/// ops can point at an earlier section or parent task.
#[derive(Clone, Debug, PartialEq)]
pub enum ImportOp {
    Section {
        name: String,
    },
    Task {
        /// Op index of the parent task.
        parent: Option<usize>,
        /// Op index of the section.
        section: Option<usize>,
        content: String,
        description: String,
        priority: Priority,
        labels: Vec<String>,
        /// `Due.date` format (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:00`).
        due: Option<String>,
        recurrence: Option<Recurrence>,
        /// `YYYY-MM-DD`.
        deadline: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ImportPlan {
    pub ops: Vec<ImportOp>,
    /// Every label mentioned (first spelling wins, case-insensitive).
    pub labels: Vec<String>,
    pub comments: usize,
    /// Dates or deadlines we couldn't read exactly, kept in the description.
    pub kept_as_text: usize,
}

/// What an import created (shown to the user).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ImportSummary {
    pub project_id: crate::ids::Id,
    pub project_name: String,
    pub tasks: usize,
    pub sections: usize,
    pub labels_created: usize,
    pub comments: usize,
    pub kept_as_text: usize,
}

/// Reads an export into items.
pub fn parse(text: &str) -> Result<Vec<TodoistItem>, ImportError> {
    let rows = super::csv::parse(text);
    let mut rows = rows.into_iter().skip_while(|r| r.iter().all(|f| f.trim().is_empty()));
    let header: Vec<String> =
        rows.next().ok_or(ImportError::NotTodoistCsv)?.iter().map(|h| h.trim().to_lowercase()).collect();
    let col = |name: &str| header.iter().position(|h| h == name);
    let (Some(type_col), Some(content_col)) = (col("type"), col("content")) else {
        return Err(ImportError::NotTodoistCsv);
    };
    let cols = [
        col("description"),
        col("priority"),
        col("indent"),
        col("date"),
        col("date_lang"),
        col("deadline"),
        col("deadline_lang"),
    ];
    let mut items = Vec::new();
    let mut last_task: Option<usize> = None;
    for row in rows {
        let get = |i: Option<usize>| i.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).unwrap_or_default();
        let content = get(Some(content_col));
        match get(Some(type_col)).as_str() {
            "section" => {
                items.push(TodoistItem::Section { name: content });
                last_task = None;
            }
            "task" => {
                let [description, priority, indent, date, date_lang, deadline, deadline_lang] = cols.map(get);
                items.push(TodoistItem::Task(TodoistTask {
                    content,
                    description,
                    priority: priority.parse::<u8>().ok().filter(|p| (1..=4).contains(p)),
                    indent: indent.parse::<usize>().unwrap_or(1).max(1),
                    date,
                    date_lang,
                    deadline,
                    deadline_lang,
                    comments: Vec::new(),
                }));
                last_task = Some(items.len() - 1);
            }
            "note" => {
                if let Some(TodoistItem::Task(t)) = last_task.and_then(|i| items.get_mut(i)) {
                    if !content.is_empty() {
                        t.comments.push(content);
                    }
                }
            }
            // Blank separator rows, `meta`, and anything unknown.
            _ => {}
        }
    }
    Ok(items)
}

/// Turns items into import operations. Dates are read with the quick-add
/// grammar and only accepted when the whole text is understood.
pub fn plan(items: &[TodoistItem], today: NaiveDate) -> ImportPlan {
    let mut out = ImportPlan::default();
    let mut section: Option<usize> = None;
    // (indent, op index) of the open ancestors.
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for item in items {
        match item {
            TodoistItem::Section { name } => {
                out.ops.push(ImportOp::Section { name: name.clone() });
                section = Some(out.ops.len() - 1);
                stack.clear();
            }
            TodoistItem::Task(t) => {
                while stack.last().is_some_and(|&(indent, _)| indent >= t.indent) {
                    stack.pop();
                }
                let parent = stack.last().map(|&(_, key)| key);
                let (content, labels) = split_labels(&t.content);
                for l in &labels {
                    if !out.labels.iter().any(|x| x.eq_ignore_ascii_case(l)) {
                        out.labels.push(l.clone());
                    }
                }
                let mut notes: Vec<String> = Vec::new();
                if !t.description.is_empty() {
                    notes.push(t.description.clone());
                }
                let (due, recurrence) = match read_date(&t.date, &t.date_lang, today) {
                    DateRead::Empty => (None, None),
                    DateRead::Read(due, rule) => (Some(due), rule),
                    DateRead::Unknown => {
                        out.kept_as_text += 1;
                        notes.push(format!("Todoist date: {}", t.date));
                        (None, None)
                    }
                };
                let deadline = match read_date(&t.deadline, &t.deadline_lang, today) {
                    DateRead::Empty => None,
                    DateRead::Read(d, None) => Some(d[..10].to_string()),
                    DateRead::Read(_, Some(_)) | DateRead::Unknown => {
                        out.kept_as_text += 1;
                        notes.push(format!("Todoist deadline: {}", t.deadline));
                        None
                    }
                };
                if !t.comments.is_empty() {
                    out.comments += t.comments.len();
                    let list: Vec<String> = t.comments.iter().map(|c| format!("- {c}")).collect();
                    notes.push(format!("Comments:\n{}", list.join("\n")));
                }
                let priority = match t.priority {
                    Some(1) => Priority::P1,
                    Some(2) => Priority::P2,
                    Some(3) => Priority::P3,
                    _ => Priority::None,
                };
                out.ops.push(ImportOp::Task {
                    parent,
                    section,
                    content,
                    description: notes.join("\n\n"),
                    priority,
                    labels,
                    due,
                    recurrence,
                    deadline,
                });
                stack.push((t.indent, out.ops.len() - 1));
            }
        }
    }
    out
}

enum DateRead {
    Empty,
    /// `Due.date` string and an optional repeat rule.
    Read(String, Option<Recurrence>),
    /// Not understood exactly; keep the text.
    Unknown,
}

/// A Todoist date cell, read only when the whole text is understood.
fn read_date(text: &str, lang: &str, today: NaiveDate) -> DateRead {
    let text = text.trim();
    if text.is_empty() {
        return DateRead::Empty;
    }
    if !lang.is_empty() && !lang.eq_ignore_ascii_case("en") {
        return DateRead::Unknown;
    }
    // "every!" repeats from the completion date, which Keel doesn't do.
    if text.to_lowercase().contains("every!") {
        return DateRead::Unknown;
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(text, fmt) {
            return DateRead::Read(dt.format("%Y-%m-%dT%H:%M:00").to_string(), None);
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return DateRead::Read(d.format("%Y-%m-%d").to_string(), None);
    }
    let q = quickadd::parse(&format!("x {text}"), today, &[], &[]);
    let only_a_date = q.content == "x" && q.priority.is_none() && q.labels.is_empty() && q.deadline.is_none();
    match (only_a_date, q.due_string()) {
        (true, Some(due)) => DateRead::Read(due, q.recurrence),
        _ => DateRead::Unknown,
    }
}

/// Pulls `@label` words out of a title: (title without them, labels).
fn split_labels(content: &str) -> (String, Vec<String>) {
    let mut words = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    for word in content.split_whitespace() {
        let name = word.strip_prefix('@').map(|n| n.trim_end_matches([',', '.', ';', '!', '?']));
        match name {
            Some(n) if !n.is_empty() => {
                if !labels.iter().any(|l| l.eq_ignore_ascii_case(n)) {
                    labels.push(n.to_string());
                }
            }
            _ => words.push(word),
        }
    }
    (words.join(" "), labels)
}

#[cfg(test)]
mod tests {
    use super::{parse, plan, ImportError, ImportOp, TodoistItem};
    use crate::model::Priority;
    use crate::recurrence::Recurrence;
    use chrono::NaiveDate;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 20).unwrap() // Tuesday
    }

    /// Shaped like a real export: all columns, blank separator rows, a BOM.
    const EXPORT: &str = "\u{feff}TYPE,CONTENT,DESCRIPTION,PRIORITY,INDENT,AUTHOR,RESPONSIBLE,DATE,DATE_LANG,TIMEZONE,DURATION,DURATION_UNIT,DEADLINE,DEADLINE_LANG\r\n\
task,Pay rent @home @bills,,1,1,Noah (123),,every month on the 1st,en,US/Eastern,,None,,\r\n\
,,,,,,,,,,,,,\r\n\
task,Plan trip,\"Flights, hotel\",2,1,Noah (123),,oct 30,en,US/Eastern,,None,2026-11-15,en\r\n\
note,Ask Sam about dates,,,,Noah (123),,,,,,,,\r\n\
note,\"Budget: $800\",,,,Noah (123),,,,,,,,\r\n\
task,Book flights,,4,2,Noah (123),,tomorrow at 5pm,en,US/Eastern,,None,,\r\n\
task,Compare airlines,,4,3,Noah (123),,,,US/Eastern,,None,,\r\n\
task,Pack,,4,2,Noah (123),,,,US/Eastern,,None,,\r\n\
,,,,,,,,,,,,,\r\n\
section,Someday,,,,,,,,,,,,\r\n\
task,Learn piano,,,1,Noah (123),,every! 3 weeks,en,US/Eastern,,None,,\r\n\
task,Read tomorrow's paper p1,,4,1,Noah (123),,,,US/Eastern,,None,,\r\n\
meta,view_style=list,,,,,,,,,,,,\r\n";

    #[test]
    fn parses_items_notes_and_skips_blank_and_meta_rows() {
        let items = parse(EXPORT).unwrap();
        assert_eq!(items.len(), 8);
        let TodoistItem::Task(trip) = &items[1] else { panic!("task") };
        assert_eq!(trip.content, "Plan trip");
        assert_eq!(trip.description, "Flights, hotel");
        assert_eq!(trip.priority, Some(2));
        assert_eq!(trip.comments, vec!["Ask Sam about dates", "Budget: $800"]);
        assert_eq!(trip.deadline, "2026-11-15");
        assert_eq!(items[5], TodoistItem::Section { name: "Someday".into() });
    }

    #[test]
    fn rejects_files_without_the_columns() {
        assert_eq!(parse("name,due\nx,y\n"), Err(ImportError::NotTodoistCsv));
        assert_eq!(parse(""), Err(ImportError::NotTodoistCsv));
    }

    #[test]
    fn columns_in_any_order_and_case() {
        let items = parse("content,Priority,type\nHello,3,task\n").unwrap();
        let TodoistItem::Task(t) = &items[0] else { panic!() };
        assert_eq!((t.content.as_str(), t.priority, t.indent), ("Hello", Some(3), 1));
    }

    fn task(op: &ImportOp) -> (&str, Option<usize>, Option<usize>) {
        match op {
            ImportOp::Task { content, parent, section, .. } => (content.as_str(), *parent, *section),
            ImportOp::Section { .. } => panic!("expected a task"),
        }
    }

    #[test]
    fn nesting_sections_and_order() {
        let p = plan(&parse(EXPORT).unwrap(), today());
        let shape: Vec<String> = p
            .ops
            .iter()
            .map(|op| match op {
                ImportOp::Section { name } => format!("§{name}"),
                ImportOp::Task { content, parent, section, .. } => format!("{content} p={parent:?} s={section:?}"),
            })
            .collect();
        assert_eq!(
            shape,
            vec![
                "Pay rent p=None s=None",
                "Plan trip p=None s=None",
                "Book flights p=Some(1) s=None",
                "Compare airlines p=Some(2) s=None",
                "Pack p=Some(1) s=None",
                "§Someday",
                "Learn piano p=None s=Some(5)",
                "Read tomorrow's paper p1 p=None s=Some(5)",
            ]
        );
    }

    #[test]
    fn labels_priority_dates_deadlines_and_comments() {
        let p = plan(&parse(EXPORT).unwrap(), today());
        assert_eq!(p.labels, vec!["home", "bills"]);
        assert_eq!(p.comments, 2);
        let ImportOp::Task { content, labels, priority, due, recurrence, .. } = &p.ops[0] else { panic!() };
        assert_eq!(content, "Pay rent", "labels come out of the title");
        assert_eq!(labels, &vec!["home".to_string(), "bills".to_string()]);
        assert_eq!(*priority, Priority::P1);
        assert_eq!(due.as_deref(), Some("2026-11-01"));
        assert_eq!(*recurrence, Some(Recurrence::MonthlyDay(1)));

        let ImportOp::Task { description, due, deadline, priority, .. } = &p.ops[1] else { panic!() };
        assert_eq!(*priority, Priority::P2);
        assert_eq!(due.as_deref(), Some("2026-10-30"));
        assert_eq!(deadline.as_deref(), Some("2026-11-15"));
        assert_eq!(description, "Flights, hotel\n\nComments:\n- Ask Sam about dates\n- Budget: $800");

        let ImportOp::Task { due, .. } = &p.ops[2] else { panic!() };
        assert_eq!(due.as_deref(), Some("2026-10-21T17:00:00"));
    }

    #[test]
    fn unknown_dates_are_kept_as_text_not_guessed() {
        let p = plan(&parse(EXPORT).unwrap(), today());
        assert_eq!(p.kept_as_text, 1);
        let ImportOp::Task { due, recurrence, description, priority, .. } = &p.ops[6] else { panic!() };
        assert_eq!((due, recurrence), (&None, &None));
        assert_eq!(description, "Todoist date: every! 3 weeks");
        assert_eq!(*priority, Priority::None, "empty priority is p4");
    }

    #[test]
    fn title_words_are_not_parsed() {
        let p = plan(&parse(EXPORT).unwrap(), today());
        let ImportOp::Task { content, due, priority, .. } = &p.ops[7] else { panic!() };
        assert_eq!(content, "Read tomorrow's paper p1");
        assert_eq!(due, &None);
        assert_eq!(*priority, Priority::None, "PRIORITY 4 column, not the 'p1' word");
    }

    #[test]
    fn other_languages_and_iso_datetimes() {
        let items = parse("TYPE,CONTENT,DATE,DATE_LANG,DEADLINE\ntask,a,mañana,es,\ntask,b,2026-10-25T08:30,en,next friday\n").unwrap();
        let p = plan(&items, today());
        let ImportOp::Task { due, description, .. } = &p.ops[0] else { panic!() };
        assert_eq!((due.as_deref(), description.as_str()), (None, "Todoist date: mañana"));
        let ImportOp::Task { due, deadline, .. } = &p.ops[1] else { panic!() };
        assert_eq!(due.as_deref(), Some("2026-10-25T08:30:00"));
        assert_eq!(deadline.as_deref(), Some("2026-10-30"));
        assert_eq!(p.kept_as_text, 1);
    }

    #[test]
    fn deep_indent_attaches_to_nearest_parent() {
        let items = parse("TYPE,CONTENT,INDENT\ntask,a,1\ntask,b,3\ntask,c,1\n").unwrap();
        let p = plan(&items, today());
        assert_eq!(task(&p.ops[1]), ("b", Some(0), None));
        assert_eq!(task(&p.ops[2]), ("c", None, None));
    }
}
