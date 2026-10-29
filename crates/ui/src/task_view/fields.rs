//! What the task view's fields card shows: set fields as rows in a fixed
//! order, missing ones as dotted "+ …" pills in the same order.

use keel_core::ids::Id;
use keel_core::views::Progress;

use crate::editor::EditorValues;

/// The fields card's rows, in their fixed order. Repeat lives in Date's row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

/// The set fields in order, then the missing ones in the same order, with
/// Repeat right after Date.
pub fn layout(v: &EditorValues, inbox_id: &Id) -> (Vec<Field>, Vec<Missing>) {
    let mut fields = Vec::new();
    let mut missing = Vec::new();
    for f in ORDER {
        if is_set(f, v, inbox_id) {
            fields.push(f);
        } else {
            missing.push(Missing::Field(f));
        }
        if f == Field::Date && v.repeat == "none" {
            missing.push(Missing::Repeat);
        }
    }
    (fields, missing)
}

/// The sub-task card's header: ("2 of 5", the toggle's text, all done).
pub fn subtask_header(progress: Progress, show_completed: bool) -> (String, &'static str, bool) {
    let toggle = if show_completed { "Hide completed" } else { "Show completed" };
    (progress.long(), toggle, progress.all_done())
}

/// Whether the title or description draft has changes to save before the
/// view moves to another task or closes.
pub fn pending_save_before_nav(before: &EditorValues, draft: &EditorValues) -> bool {
    before.content != draft.content || before.description != draft.description
}

/// Whether a draft takes the stored text after a reload: only when the
/// stored text changed (`seen` is the last stored text) and isn't just our
/// own save coming back (`saved`), so a draft typed since stays.
pub fn takes_stored_text(stored: &str, seen: &str, saved: &str) -> bool {
    stored != seen && stored != saved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inbox() -> Id {
        Id::from("P-IN")
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
        let (fields, missing) = layout(&fixture(), &inbox());
        assert_eq!(fields, vec![Field::Description, Field::Date, Field::Priority, Field::Labels]);
        assert_eq!(
            missing,
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
        let (fields, missing) = layout(&v, &inbox());
        assert_eq!(fields.last(), Some(&Field::Location));
        assert!(!missing.contains(&Missing::Field(Field::Location)));
        // A section in the Inbox counts too.
        let mut s = fixture();
        s.section_id = Some(Id::from("S"));
        assert!(layout(&s, &inbox()).0.contains(&Field::Location));
    }

    #[test]
    fn layout_repeat_inside_date() {
        let mut v = fixture();
        v.repeat = "weekly".into();
        let (fields, missing) = layout(&v, &inbox());
        assert!(fields.contains(&Field::Date));
        assert!(!missing.contains(&Missing::Repeat));
    }

    #[test]
    fn layout_repeat_counts_as_date() {
        let mut v = fixture();
        v.date = String::new();
        v.repeat = "weekly".into();
        let (fields, missing) = layout(&v, &inbox());
        assert!(fields.contains(&Field::Date));
        assert!(!missing.contains(&Missing::Field(Field::Date)));
        assert!(!missing.contains(&Missing::Repeat));
    }

    #[test]
    fn newly_set_field_lands_in_order() {
        let mut v = fixture();
        v.deadline = "2026-10-30".into();
        let (fields, missing) = layout(&v, &inbox());
        assert_eq!(fields, vec![Field::Description, Field::Date, Field::Deadline, Field::Priority, Field::Labels]);
        assert!(!missing.contains(&Missing::Field(Field::Deadline)));
    }

    #[test]
    fn empty_description_whitespace_is_missing() {
        let mut v = fixture();
        v.description = "  ".into();
        let (fields, missing) = layout(&v, &inbox());
        assert!(!fields.contains(&Field::Description));
        assert_eq!(missing.first(), Some(&Missing::Field(Field::Description)));
    }

    #[test]
    fn header_text() {
        assert_eq!(
            subtask_header(Progress { done: 2, total: 5 }, false),
            ("2 of 5".to_string(), "Show completed", false)
        );
        assert_eq!(
            subtask_header(Progress { done: 5, total: 5 }, true),
            ("5 of 5".to_string(), "Hide completed", true)
        );
    }

    #[test]
    fn pending_save_detects_title_and_description() {
        let before = fixture();
        let mut date = before.clone();
        date.date = "2026-10-31".into();
        assert!(!pending_save_before_nav(&before, &date));
        let mut title = before.clone();
        title.content = "Pack for NYC".into();
        assert!(pending_save_before_nav(&before, &title));
        let mut desc = before.clone();
        desc.description.push_str("  ");
        assert!(pending_save_before_nav(&before, &desc));
    }

    #[test]
    fn our_own_save_coming_back_keeps_the_draft() {
        // Saved "Pack", then typed "Pack bag": the reload brings back "Pack".
        assert!(!takes_stored_text("Pack", "Old", "Pack"));
        // An edit from elsewhere replaces the draft.
        assert!(takes_stored_text("Moved", "Old", "Pack"));
        // Nothing changed.
        assert!(!takes_stored_text("Old", "Old", "Old"));
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
}
