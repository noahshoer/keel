//! What the task view's fields card shows: set fields as rows in a fixed
//! order, missing ones as dotted "+ …" pills in the same order.

use keel_core::views::Progress;

use crate::editor::EditorValues;

pub use keel_core::task_view::{field_layout, Field, FieldLayout, Missing};

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
    use keel_core::ids::Id;

    /// Title and description set; an Inbox task.
    fn fixture() -> EditorValues {
        let mut v = EditorValues::blank(Id::from("P-IN"), None, "2026-10-24".into(), vec!["travel".into()]);
        v.content = "Pack for Boston".into();
        v.description = "Leave Friday after work".into();
        v
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
}
