//! The Search view: a query box with live results (open tasks, plus matching
//! completed ones when "Show completed" is on). Results load here rather than
//! in the app-wide loader, so typing doesn't reload everything.

use std::collections::HashMap;

use chrono::NaiveDate;
use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::model::{Label, Project, Section, Task};

use crate::api;
use crate::icons::{self, Icon};
use crate::task_row::{CompletedRow, TaskRow};

/// "Project" or "Project / Section" for each task.
fn locations<'a>(tasks: impl Iterator<Item = &'a Task>, projects: &[Project], sections: &[Section]) -> HashMap<Id, String> {
    tasks
        .map(|t| {
            let project = projects.iter().find(|p| p.id == t.project_id).map(|p| p.name.as_str()).unwrap_or("");
            let label = match t.section_id.as_ref().and_then(|s| sections.iter().find(|x| &x.id == s)) {
                Some(section) => format!("{project} / {}", section.name),
                None => project.to_string(),
            };
            (t.id.clone(), label)
        })
        .collect()
}

#[component]
pub fn SearchPane(
    query: Signal<String>,
    today: NaiveDate,
    show_completed: bool,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    reload: Signal<u32>,
) -> Element {
    let mut query = query;
    let results = use_resource(move || async move {
        let _ = reload();
        let q = query();
        if q.trim().is_empty() {
            return Ok(Default::default());
        }
        api::search(&q).await
    });
    let q = query();
    let found = results.read().clone();

    rsx! {
        div { class: "search-box",
            span { class: "search-icon", Icon { svg: icons::search() } }
            input {
                class: "search-input",
                r#type: "search",
                placeholder: "Search tasks, descriptions and labels",
                autofocus: true,
                spellcheck: "false",
                value: "{q}",
                oninput: move |e| query.set(e.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Escape {
                        query.set(String::new());
                    }
                },
            }
            if !q.is_empty() {
                button {
                    r#type: "button",
                    class: "search-clear",
                    title: "Clear",
                    aria_label: "Clear search",
                    // Keep focus (and the keyboard) in the field.
                    onmousedown: move |e| e.prevent_default(),
                    onclick: move |_| query.set(String::new()),
                    Icon { svg: icons::x_small() }
                }
            }
        }
        if q.trim().is_empty() {
            div { class: "empty",
                div { class: "big", "Search your tasks" }
                div { {crate::platform::search_hint(crate::platform::current())} }
            }
        } else {
            match found {
                None => rsx! { p { class: "view-sub", "Searching…" } },
                Some(Err(e)) => rsx! { div { class: "error", "Search failed: {e}" } },
                Some(Ok(r)) => {
                    let names = locations(r.open.iter().chain(r.completed.iter()), &projects, &sections);
                    let none = r.open.is_empty() && (!show_completed || r.completed.is_empty());
                    rsx! {
                        if none {
                            div { class: "empty",
                                div { class: "big", "No tasks match “{q}”" }
                                if !show_completed && !r.completed.is_empty() {
                                    div { "{r.completed.len()} completed task(s) match — turn on Show completed to see them." }
                                } else {
                                    div { "Try fewer or shorter words." }
                                }
                            }
                        } else {
                            p { class: "view-sub",
                                if r.open.len() == 1 { "1 task" } else { "{r.open.len()} tasks" }
                            }
                        }
                        for task in r.open.iter().cloned() {
                            TaskRow {
                                key: "{task.id}",
                                project_name: names.get(&task.id).cloned(),
                                task,
                                today,
                                projects: projects.clone(),
                                sections: sections.clone(),
                                labels: labels.clone(),
                                reload,
                            }
                        }
                        if show_completed && !r.completed.is_empty() {
                            div { class: "section-title",
                                "Completed"
                                span { class: "muted", "{r.completed.len()}" }
                            }
                            for task in r.completed.iter().cloned() {
                                CompletedRow {
                                    key: "done-{task.id}",
                                    project_name: names.get(&task.id).cloned(),
                                    task,
                                    today,
                                    reload,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
