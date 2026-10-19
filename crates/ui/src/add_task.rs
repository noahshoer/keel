use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use todo_core::ids::Id;
use todo_core::model::Priority;
use todo_core::recurrence::Recurrence;

use crate::api::{self, NewTask};
use crate::icons::{self, Icon};

fn priority_from(s: &str) -> Priority {
    match s {
        "1" => Priority::P1,
        "2" => Priority::P2,
        "3" => Priority::P3,
        _ => Priority::None,
    }
}

/// "Every week"/"Every month" anchor on the chosen date (or today).
fn recurrence_from(choice: &str, date: &str) -> Option<Recurrence> {
    let anchor = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap_or_else(|_| crate::today());
    match choice {
        "daily" => Some(Recurrence::Daily),
        "weekdays" => Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])),
        "weekly" => Some(Recurrence::Weekly(vec![anchor.weekday().num_days_from_monday() as u8])),
        "monthly" => Some(Recurrence::MonthlyDay(anchor.day() as u8)),
        _ => None,
    }
}

/// Todoist-style "+ Add task" row that expands into an inline editor card.
#[component]
pub fn AddTask(project_id: Id, default_date: String, reload: Signal<u32>) -> Element {
    let mut reload = reload;
    let mut open = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut desc = use_signal(String::new);
    let initial_date = default_date.clone();
    let mut date = use_signal(move || initial_date);
    let mut priority = use_signal(|| "4".to_string());
    let mut repeat = use_signal(|| "none".to_string());
    let can_add = !name().trim().is_empty();

    if !open() {
        return rsx! {
            div { class: "add-task-row", onclick: move |_| open.set(true),
                span { class: "plus", Icon { svg: icons::plus(14) } }
                "Add task"
            }
        };
    }

    let reset_date = default_date.clone();
    rsx! {
        form {
            class: "editor",
            onsubmit: move |e| {
                e.prevent_default();
                let content = name().trim().to_string();
                if content.is_empty() {
                    return;
                }
                let new_task = NewTask {
                    content,
                    description: desc(),
                    project_id: project_id.clone(),
                    recurrence: recurrence_from(&repeat(), &date()),
                    date: date(),
                    priority: priority_from(&priority()),
                };
                let reset = reset_date.clone();
                spawn(async move {
                    let _ = api::add_task(new_task).await;
                    name.set(String::new());
                    desc.set(String::new());
                    date.set(reset);
                    priority.set("4".to_string());
                    repeat.set("none".to_string());
                    reload += 1;
                });
            },
            input {
                class: "name",
                placeholder: "Task name",
                autofocus: true,
                value: "{name}",
                oninput: move |e| name.set(e.value()),
            }
            input {
                class: "desc",
                placeholder: "Description",
                value: "{desc}",
                oninput: move |e| desc.set(e.value()),
            }
            div { class: "chips",
                label { class: "chip", title: "Date",
                    Icon { svg: icons::calendar_small() }
                    input { r#type: "date", value: "{date}", oninput: move |e| date.set(e.value()) }
                }
                label { class: "chip", title: "Priority",
                    Icon { svg: icons::flag_small() }
                    select { value: "{priority}", onchange: move |e| priority.set(e.value()),
                        option { value: "1", "Priority 1" }
                        option { value: "2", "Priority 2" }
                        option { value: "3", "Priority 3" }
                        option { value: "4", "Priority 4" }
                    }
                }
                label { class: "chip", title: "Repeat",
                    Icon { svg: icons::repeat_small() }
                    select { value: "{repeat}", onchange: move |e| repeat.set(e.value()),
                        option { value: "none", "No repeat" }
                        option { value: "daily", "Every day" }
                        option { value: "weekdays", "Every weekday" }
                        option { value: "weekly", "Every week" }
                        option { value: "monthly", "Every month" }
                    }
                }
            }
            div { class: "editor-footer",
                button {
                    r#type: "button",
                    class: "btn btn-cancel",
                    onclick: move |_| {
                        open.set(false);
                        name.set(String::new());
                        desc.set(String::new());
                    },
                    "Cancel"
                }
                button { r#type: "submit", class: "btn btn-add", disabled: !can_add, "Add task" }
            }
        }
    }
}
