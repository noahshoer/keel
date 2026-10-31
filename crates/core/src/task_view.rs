//! What the task view shows, as data: the fields card's layout, the data one
//! level renders, for the React UI.

use serde::Serialize;

use crate::editor::EditorValues;
use crate::ids::Id;
use crate::model::Task;
use crate::views::{self, CardRow, Progress};

/// The fields card's rows, in their fixed order. Repeat lives in Date's row.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Field {
    Description,
    Date,
    Deadline,
    Priority,
    Labels,
    Reminders,
    Location,
}

const ORDER: [Field; 7] =
    [Field::Description, Field::Date, Field::Deadline, Field::Priority, Field::Labels, Field::Reminders, Field::Location];

/// A dotted pill for something without a value.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "kind", content = "field")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Missing {
    Field(Field),
    /// Only while the task doesn't repeat; a rule shows in Date's row.
    Repeat,
}

impl Missing {
    pub fn label(&self) -> &'static str {
        match self {
            Missing::Field(Field::Description) => "+ Description",
            Missing::Field(Field::Date) => "+ Date",
            Missing::Field(Field::Deadline) => "+ Deadline",
            Missing::Field(Field::Priority) => "+ Priority",
            Missing::Field(Field::Labels) => "+ Labels",
            Missing::Field(Field::Reminders) => "+ Reminders",
            Missing::Field(Field::Location) => "+ Location",
            Missing::Repeat => "+ Repeat",
        }
    }
}

fn is_set(f: Field, v: &EditorValues, inbox_id: &Id) -> bool {
    match f {
        Field::Description => !v.description.trim().is_empty(),
        // A repeat with no date yet still belongs to the date row.
        Field::Date => !v.date.is_empty() || !v.time.is_empty() || v.repeat != "none",
        Field::Deadline => !v.deadline.is_empty(),
        Field::Priority => v.priority != "4",
        Field::Labels => !v.labels.is_empty(),
        Field::Reminders => !v.reminders.is_empty(),
        // An Inbox task with no section has nowhere to show; anywhere else does.
        Field::Location => &v.project_id != inbox_id || v.section_id.is_some(),
    }
}

/// A missing field's pill: what it adds and the text it shows.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MissingPill {
    pub missing: Missing,
    pub label: String,
}

/// The fields card: set fields as rows, missing ones as pills.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct FieldLayout {
    pub fields: Vec<Field>,
    pub missing: Vec<MissingPill>,
}

/// The set fields in order, then the missing ones in the same order, with
/// Repeat right after Date.
pub fn field_layout(v: &EditorValues, inbox_id: &Id) -> FieldLayout {
    let mut fields = Vec::new();
    let mut missing = Vec::new();
    let mut pill = |m: Missing| missing.push(MissingPill { missing: m, label: m.label().to_string() });
    for f in ORDER {
        if is_set(f, v, inbox_id) {
            fields.push(f);
        } else {
            pill(Missing::Field(f));
        }
        if f == Field::Date && v.repeat == "none" {
            pill(Missing::Repeat);
        }
    }
    FieldLayout { fields, missing }
}

/// Everything one level of the view renders.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TaskDetail {
    pub task: Task,
    pub parent: Option<Task>,
    pub progress: Option<Progress>,
    /// Direct sub-tasks in child order; checked ones only when `show_completed`.
    pub visible: Vec<CardRow>,
}

pub fn detail(id: &Id, all: &[Task], show_completed: bool) -> Option<TaskDetail> {
    let task = all.iter().find(|t| &t.id == id)?.clone();
    let parent = task.parent_id.as_ref().and_then(|p| all.iter().find(|t| &t.id == p)).cloned();
    let visible = views::children(id, all)
        .into_iter()
        .filter(|c| show_completed || !c.checked)
        .map(|c| CardRow { task: c.clone(), progress: views::progress(&c.id, all) })
        .collect();
    Some(TaskDetail { progress: views::progress(id, all), task, parent, visible })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> Id {
        Id::from(s)
    }

    fn task(s: &str, parent: Option<&str>, order: i64) -> Task {
        let mut t = Task::new(s, id("proj"));
        t.id = id(s);
        t.parent_id = parent.map(id);
        t.child_order = order;
        t
    }

    fn inbox() -> Id {
        Id::from("P-IN")
    }

    fn kinds(pills: &[MissingPill]) -> Vec<Missing> {
        pills.iter().map(|p| p.missing).collect()
    }

    /// Description, date, priority and labels set; an Inbox task with no section.
    fn fixture() -> EditorValues {
        let mut v = EditorValues::blank(inbox(), None, "2026-10-24".into(), vec!["travel".into()]);
        v.content = "Pack for Boston".into();
        v.description = "Leave Friday after work".into();
        v.priority = "2".into();
        v
    }

    #[test]
    fn layout_fixed_order() {
        let FieldLayout { fields, missing } = field_layout(&fixture(), &inbox());
        assert_eq!(fields, vec![Field::Description, Field::Date, Field::Priority, Field::Labels]);
        assert_eq!(
            kinds(&missing),
            vec![
                Missing::Repeat,
                Missing::Field(Field::Deadline),
                Missing::Field(Field::Reminders),
                Missing::Field(Field::Location),
            ]
        );
    }

    #[test]
    fn layout_location_row_outside_inbox() {
        let mut v = fixture();
        v.project_id = Id::from("X");
        let FieldLayout { fields, missing } = field_layout(&v, &inbox());
        assert_eq!(fields.last(), Some(&Field::Location));
        assert!(!kinds(&missing).contains(&Missing::Field(Field::Location)));
        // A section in the Inbox counts too.
        let mut s = fixture();
        s.section_id = Some(Id::from("S"));
        assert!(field_layout(&s, &inbox()).fields.contains(&Field::Location));
    }

    #[test]
    fn layout_repeat_inside_date() {
        let mut v = fixture();
        v.repeat = "weekly".into();
        let FieldLayout { fields, missing } = field_layout(&v, &inbox());
        assert!(fields.contains(&Field::Date));
        assert!(!kinds(&missing).contains(&Missing::Repeat));
    }

    #[test]
    fn layout_repeat_counts_as_date() {
        let mut v = fixture();
        v.date = String::new();
        v.repeat = "weekly".into();
        let FieldLayout { fields, missing } = field_layout(&v, &inbox());
        assert!(fields.contains(&Field::Date));
        assert!(!kinds(&missing).contains(&Missing::Field(Field::Date)));
        assert!(!kinds(&missing).contains(&Missing::Repeat));
    }

    #[test]
    fn newly_set_field_lands_in_order() {
        let mut v = fixture();
        v.deadline = "2026-10-30".into();
        let FieldLayout { fields, missing } = field_layout(&v, &inbox());
        assert_eq!(fields, vec![Field::Description, Field::Date, Field::Deadline, Field::Priority, Field::Labels]);
        assert!(!kinds(&missing).contains(&Missing::Field(Field::Deadline)));
    }

    #[test]
    fn empty_description_whitespace_is_missing() {
        let mut v = fixture();
        v.description = "  ".into();
        let FieldLayout { fields, missing } = field_layout(&v, &inbox());
        assert!(!fields.contains(&Field::Description));
        assert_eq!(kinds(&missing).first(), Some(&Missing::Field(Field::Description)));
    }

    #[test]
    fn pill_labels() {
        let all = [
            (Missing::Field(Field::Description), "+ Description"),
            (Missing::Field(Field::Date), "+ Date"),
            (Missing::Field(Field::Deadline), "+ Deadline"),
            (Missing::Field(Field::Priority), "+ Priority"),
            (Missing::Field(Field::Labels), "+ Labels"),
            (Missing::Field(Field::Reminders), "+ Reminders"),
            (Missing::Repeat, "+ Repeat"),
            (Missing::Field(Field::Location), "+ Location"),
        ];
        for (m, text) in all {
            assert!(m.label().starts_with("+ "));
            assert_eq!(m.label(), text);
        }
    }

    #[test]
    fn detail_hides_completed_unless_shown() {
        let mut c = task("C", Some("P"), 0);
        c.checked = true;
        let all = vec![task("P", None, 0), task("S", Some("P"), 1), c];
        let hidden = detail(&id("P"), &all, false).unwrap();
        let ids: Vec<_> = hidden.visible.iter().map(|r| r.task.id.clone()).collect();
        assert_eq!(ids, vec![id("S")]);
        let shown = detail(&id("P"), &all, true).unwrap();
        let ids: Vec<_> = shown.visible.iter().map(|r| r.task.id.clone()).collect();
        assert_eq!(ids, vec![id("C"), id("S")]);
    }

    #[test]
    fn detail_reflects_reloaded_tasks() {
        let before = vec![task("P", None, 0), task("C", Some("P"), 0), task("S", Some("P"), 1)];
        assert_eq!(detail(&id("P"), &before, false).unwrap().progress, Some(Progress { done: 0, total: 2 }));
        let mut after = before.clone();
        after[1].checked = true;
        assert_eq!(detail(&id("P"), &after, false).unwrap().progress, Some(Progress { done: 1, total: 2 }));
    }

    #[test]
    fn detail_none_for_missing() {
        assert!(detail(&id("X"), &[task("A", None, 0)], false).is_none());
    }

    #[test]
    fn detail_has_parent() {
        let all = vec![task("P", None, 0), task("C", Some("P"), 0)];
        let d = detail(&id("C"), &all, false).unwrap();
        assert_eq!(d.parent.unwrap().id, id("P"));
        assert!(detail(&id("P"), &all, false).unwrap().parent.is_none());
    }

    #[test]
    fn visible_rows_carry_their_own_progress() {
        let all = vec![task("P", None, 0), task("C", Some("P"), 0), task("G", Some("C"), 0)];
        let d = detail(&id("P"), &all, false).unwrap();
        assert_eq!(d.visible[0].progress, Some(Progress { done: 0, total: 1 }));
    }

    #[test]
    fn missing_serializes_tagged() {
        assert_eq!(serde_json::to_string(&Missing::Repeat).unwrap(), r#"{"kind":"Repeat"}"#);
        assert_eq!(
            serde_json::to_string(&Missing::Field(Field::Deadline)).unwrap(),
            r#"{"kind":"Field","field":"Deadline"}"#
        );
    }

    #[test]
    fn completed_elsewhere_still_has_detail() {
        let mut a = task("A", None, 0);
        a.checked = true;
        assert!(detail(&id("A"), &[a], false).unwrap().task.checked);
    }
}
