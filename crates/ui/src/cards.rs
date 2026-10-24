//! Lists as cards: each top-level task is a card, and in dated views the
//! sub-tasks that match sit inside their parent's card.

use std::collections::HashMap;

use chrono::NaiveDate;
use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::model::{Label, Priority, Project, Section, Task};
use keel_core::views::{Card, Progress};

use crate::icons::{self, Icon};
use crate::task_row::{due_text, SubtaskCount, TaskRow};
use crate::task_view::nav::use_nav;

#[component]
pub fn CardList(
    cards: Vec<Card>,
    today: NaiveDate,
    names: HashMap<Id, String>,
    show_project: bool,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    reload: Signal<u32>,
) -> Element {
    let name_of = |t: &Task| if show_project { names.get(&t.id).cloned() } else { None };
    rsx! {
        for card in cards {
            div { key: "{card.head.task.id}", class: "task-card",
                if card.head_matches {
                    TaskRow {
                        key: "{card.head.task.id}",
                        project_name: name_of(&card.head.task),
                        task: card.head.task.clone(),
                        progress: card.head.progress,
                        today,
                        projects: projects.clone(),
                        sections: sections.clone(),
                        labels: labels.clone(),
                        reload,
                    }
                } else {
                    CardHead {
                        key: "head-{card.head.task.id}",
                        task: card.head.task.clone(),
                        progress: card.head.progress,
                        today,
                    }
                }
                for row in card.rows {
                    TaskRow {
                        key: "{row.task.id}",
                        project_name: name_of(&row.task),
                        task: row.task.clone(),
                        progress: row.progress,
                        today,
                        projects: projects.clone(),
                        sections: sections.clone(),
                        labels: labels.clone(),
                        reload,
                    }
                }
            }
        }
    }
}

fn flag_class(p: Priority) -> Option<&'static str> {
    match p {
        Priority::P1 => Some("card-flag p1"),
        Priority::P2 => Some("card-flag p2"),
        Priority::P3 => Some("card-flag p3"),
        Priority::None => None,
    }
}

/// A card's header when its task doesn't match the view: context only, with
/// no circle. Tapping it opens the task.
#[component]
pub fn CardHead(task: Task, progress: Option<Progress>, today: NaiveDate) -> Element {
    let nav = use_nav();
    let due = due_text(&task, today);
    let id = task.id.clone();
    rsx! {
        div { class: "card-head", title: "Open task", onclick: move |_| nav.open(id.clone()),
            if let Some(cls) = flag_class(task.priority) {
                span { class: cls, Icon { svg: icons::flag_small() } }
            }
            span { class: "pname", "{task.content}" }
            span { class: "card-head-side",
                if let Some((cls, label)) = due {
                    span { class: cls, "{label}" }
                }
                if let Some(p) = progress {
                    SubtaskCount { progress: p }
                }
                span { class: "card-chev", Icon { svg: icons::chevron_right() } }
            }
        }
    }
}
