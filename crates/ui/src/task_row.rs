use chrono::NaiveDate;
use dioxus::prelude::*;
use todo_core::model::{Priority, Task};
use todo_core::views::{self, DueBucket};

use crate::api;
use crate::icons::{self, Icon};

fn check_class(p: Priority) -> &'static str {
    match p {
        Priority::P1 => "check p1",
        Priority::P2 => "check p2",
        Priority::P3 => "check p3",
        Priority::None => "check",
    }
}

fn due_class(b: DueBucket) -> &'static str {
    match b {
        DueBucket::Overdue => "due overdue",
        DueBucket::Today => "due today",
        DueBucket::Tomorrow => "due tomorrow",
        DueBucket::ThisWeek => "due week",
        DueBucket::Later => "due later",
    }
}

#[component]
pub fn TaskRow(
    task: Task,
    today: NaiveDate,
    project_name: Option<String>,
    reload: Signal<u32>,
) -> Element {
    let mut reload = reload;
    let mut panel = use_signal(|| false);

    let due = views::due_date_of(&task)
        .map(|d| (due_class(views::due_bucket(d, today)), views::due_label(d, today)));
    let recurring = task.due.as_ref().is_some_and(|d| d.is_recurring);
    let streak_on = task.streak_enabled;
    let goal = task.streak_goal;
    let goal_value = goal.map(|g| g.to_string()).unwrap_or_default();
    let show_meta = due.is_some() || streak_on;

    let id_complete = task.id.clone();
    let id_delete = task.id.clone();
    let id_toggle = task.id.clone();
    let id_goal = task.id.clone();

    rsx! {
        div { class: "task",
            div { class: "task-row",
                button {
                    class: check_class(task.priority),
                    title: "Complete task",
                    onclick: move |_| {
                        let id = id_complete.clone();
                        spawn(async move {
                            let _ = api::complete_task(id).await;
                            reload += 1;
                        });
                    },
                    Icon { svg: icons::check() }
                }
                div { class: "task-body",
                    div { class: "task-content", "{task.content}" }
                    if !task.description.is_empty() {
                        div { class: "task-desc", "{task.description}" }
                    }
                    if show_meta {
                        div { class: "task-meta",
                            if let Some((cls, label)) = due.clone() {
                                span { class: cls,
                                    Icon { svg: icons::calendar_small() }
                                    "{label}"
                                    if recurring {
                                        Icon { svg: icons::repeat_small() }
                                    }
                                }
                            }
                            if streak_on {
                                span { class: "streak", title: "Current streak", "🔥 {task.streak_current}" }
                            }
                        }
                    }
                }
                div { class: "task-actions",
                    if recurring {
                        button {
                            class: "icon-btn",
                            title: "Streak",
                            onclick: move |_| panel.set(!panel()),
                            "🔥"
                        }
                    }
                    button {
                        class: "icon-btn",
                        title: "Delete task",
                        onclick: move |_| {
                            let id = id_delete.clone();
                            spawn(async move {
                                let _ = api::delete_task(id).await;
                                reload += 1;
                            });
                        },
                        Icon { svg: icons::trash() }
                    }
                }
                if let Some(name) = project_name.clone() {
                    span { class: "task-project", "{name}" }
                }
            }
            if panel() && recurring {
                div { class: "streak-panel",
                    if streak_on {
                        span { class: "stat", "Current " b { "🔥 {task.streak_current}" } }
                        span { class: "stat", "Best " b { "{task.streak_best}" } }
                        label {
                            "Goal "
                            input {
                                r#type: "number",
                                min: "1",
                                value: "{goal_value}",
                                onchange: move |e| {
                                    let id = id_goal.clone();
                                    let new_goal = e.value().trim().parse::<i64>().ok().filter(|g| *g > 0);
                                    spawn(async move {
                                        let _ = api::set_streak(id, true, new_goal).await;
                                        reload += 1;
                                    });
                                },
                            }
                        }
                    } else {
                        span { "Track how many times in a row you complete this." }
                    }
                    button {
                        class: if streak_on { "btn btn-cancel" } else { "btn btn-add" },
                        onclick: move |_| {
                            let id = id_toggle.clone();
                            spawn(async move {
                                let _ = api::set_streak(id, !streak_on, goal).await;
                                reload += 1;
                            });
                        },
                        if streak_on { "Stop tracking" } else { "Track streak" }
                    }
                }
            }
        }
    }
}
