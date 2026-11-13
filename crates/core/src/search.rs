//! Basic text search (spec §6): every word of the query must appear in the
//! task's title, description or labels — any order, case-insensitive.

use serde::{Deserialize, Serialize};

use crate::ids::Id;
use crate::model::{contains_name, Task};

/// Search results for the Search view.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SearchResults {
    /// Open tasks: title matches first, then by due date (undated last), then priority.
    pub open: Vec<Task>,
    /// Completed tasks, newest first (capped).
    pub completed: Vec<Task>,
}

/// What a search is limited to, on top of the query (the page Search was
/// opened from). It holds for every comma group of the query.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum SearchScope {
    /// Tasks in this project (by id), sections and sub-tasks included.
    Project { id: Id },
    /// Tasks carrying this label (by name, case-insensitive).
    Label { name: String },
}

impl SearchScope {
    pub fn contains(&self, task: &Task) -> bool {
        match self {
            SearchScope::Project { id } => task.project_id == *id,
            SearchScope::Label { name } => contains_name(&task.labels, name),
        }
    }
}

/// The query's words, lower-cased (a leading `@` on a word is ignored).
pub fn words(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|w| w.trim_start_matches('@').to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Whether every query word appears in the task. An empty query matches nothing.
pub fn matches(task: &Task, words: &[String]) -> bool {
    if words.is_empty() {
        return false;
    }
    let haystack = [
        task.content.as_str(),
        task.description.as_str(),
        &task.labels.join(" "),
    ]
    .join("\n")
    .to_lowercase();
    words.iter().all(|w| haystack.contains(w.as_str()))
}

/// 0 when every word is in the title, else 1 (for ordering).
pub fn rank(task: &Task, words: &[String]) -> u8 {
    let title = task.content.to_lowercase();
    if words.iter().all(|w| title.contains(w.as_str())) {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::{matches, rank, words, SearchScope};
    use crate::ids::Id;
    use crate::model::Task;

    fn task(content: &str, description: &str, labels: &[&str]) -> Task {
        let mut t = Task::new(content, Id::from("P"));
        t.description = description.into();
        t.labels = labels.iter().map(|l| l.to_string()).collect();
        t
    }

    fn m(query: &str, t: &Task) -> bool {
        matches(t, &words(query))
    }

    #[test]
    fn words_lowercase_and_strip_at() {
        assert_eq!(
            words("  Buy  @Errands milk "),
            vec!["buy", "errands", "milk"]
        );
        assert!(words("   ").is_empty());
    }

    #[test]
    fn all_words_any_order_any_field() {
        let t = task("Buy milk", "from the corner shop", &["errands"]);
        assert!(m("milk", &t));
        assert!(m("MILK buy", &t), "order and case don't matter");
        assert!(m("milk corner", &t), "title + description");
        assert!(m("milk errands", &t), "labels count");
        assert!(m("@errands", &t));
        assert!(m("mil", &t), "prefix/substring");
        assert!(!m("milk bread", &t), "every word must match");
    }

    #[test]
    fn empty_query_matches_nothing() {
        assert!(!m("", &task("Anything", "", &[])));
    }

    #[test]
    fn title_matches_rank_first() {
        let w = words("milk");
        assert_eq!(rank(&task("Buy milk", "", &[]), &w), 0);
        assert_eq!(rank(&task("Groceries", "milk, eggs", &[]), &w), 1);
    }

    #[test]
    fn scopes_limit_by_project_id_or_label_name() {
        let t = task("Buy milk", "", &["Errands"]);
        assert!(SearchScope::Project { id: Id::from("P") }.contains(&t));
        assert!(!SearchScope::Project { id: Id::from("Q") }.contains(&t));
        assert!(SearchScope::Label {
            name: "errands".into()
        }
        .contains(&t));
        assert!(!SearchScope::Label {
            name: "home".into()
        }
        .contains(&t));
    }

    #[test]
    fn scope_json_is_tagged_by_kind() {
        let json = serde_json::to_string(&SearchScope::Label {
            name: "home".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"label","name":"home"}"#);
    }
}
