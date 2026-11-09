//! What a list page remembers about how it is arranged: a filter query, the
//! fields it is headed by, and the order of its tasks.

use serde::{Deserialize, Serialize};

use crate::ids::Id;

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

impl Arrangement {
    /// True when the page is arranged as it is without any setting.
    pub fn is_default(&self) -> bool {
        self.filter.trim().is_empty() && self.groups.is_empty() && self.order.is_empty()
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
    fn arrangement_reads_from_empty_object() {
        let a: Arrangement = serde_json::from_str("{}").unwrap();
        assert!(a.is_default());
    }
}
