use chrono::NaiveDate;
use dioxus::prelude::*;
use keel_core::model::{Label, Priority, Project, Reminder, Section, Task};
use keel_core::views::{self, DueBucket};

use crate::api;
use crate::editor::{add_from_editor, repeat_text, save_edit, EditorValues, TaskEditor};
use crate::pickers::ReminderDraft;

/// Pixels of indentation per sub-task level.
const INDENT: usize = 28;
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
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    /// Sub-task nesting level (0 = top level).
    #[props(default)]
    depth: usize,
    /// Shows the collapse chevron.
    #[props(default)]
    has_children: bool,
    reload: Signal<u32>,
) -> Element {
    let mut reload = reload;
    let mut panel = use_signal(|| false);
    let mut editing = use_signal(|| false);
    let mut adding_sub = use_signal(|| false);
    let glass = crate::glass::use_glass();
    // All task reminders, shared app-wide (see `App`).
    let all_reminders = use_context::<Signal<Vec<Reminder>>>();
    let my_reminders: Vec<ReminderDraft> = all_reminders
        .read()
        .iter()
        .filter(|r| r.task_id == task.id)
        .map(ReminderDraft::from_reminder)
        .collect();
    let indent = format!("margin-left: {}px;", depth * INDENT);
    // For the native card (iPhone): what editing and "Add sub-task" open with.
    let native_lists = (projects.clone(), sections.clone(), labels.clone());
    let mut native_sub = EditorValues::blank(task.project_id.clone(), task.section_id.clone(), String::new(), Vec::new());
    native_sub.parent_id = Some(task.id.clone());

    if editing() {
        let mut before = EditorValues::from_task(&task);
        before.reminders = my_reminders.clone();
        let existing_rule = task.due.as_ref().and_then(|d| d.recurrence.clone());
        let keep_label = (before.repeat == "keep")
            .then(|| existing_rule.as_ref().map(repeat_text).unwrap_or_default());
        let id = task.id.clone();
        let initial = before.clone();
        return rsx! {
            div { style: "{indent}",
                TaskEditor {
                    initial,
                    projects,
                    sections,
                    labels,
                    keep_label,
                    submit_text: "Save".to_string(),
                    on_submit: move |after: EditorValues| {
                        let (id, before, rule) = (id.clone(), before.clone(), existing_rule.clone());
                        editing.set(false);
                        spawn(async move {
                            save_edit(id, before, after, rule).await;
                            reload += 1;
                        });
                    },
                    on_cancel: move |_| editing.set(false),
                    reload,
                }
            }
        };
    }

    // Defaults for "Add sub-task": same project/section, child of this task.
    let mut sub_defaults =
        EditorValues::blank(task.project_id.clone(), task.section_id.clone(), String::new(), Vec::new());
    sub_defaults.parent_id = Some(task.id.clone());
    let (sub_projects, sub_sections, sub_labels) = (projects.clone(), sections.clone(), labels.clone());
    let sub_indent = format!("margin-left: {}px;", (depth + 1) * INDENT);
    let collapsed = task.is_collapsed;
    let id_collapse = task.id.clone();

    let deadline = task
        .deadline
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| (if d <= today { "urgent" } else { "set" }, views::due_label(d, today)));
    let has_reminders = !my_reminders.is_empty();
    let reminder_title = my_reminders.iter().map(|r| r.label(today)).collect::<Vec<_>>().join(", ");
    let due = views::due_date_of(&task)
        .map(|d| {
            let label = views::due_label(d, today);
            let label = match views::due_time(&task) {
                Some(t) => format!("{label} {}", views::time_label(t)),
                None => label,
            };
            (due_class(views::due_bucket(d, today)), label)
        });
    let recurring = task.due.as_ref().is_some_and(|d| d.is_recurring);
    let streak_on = task.streak_enabled;
    let goal = task.streak_goal;
    let goal_value = goal.map(|g| g.to_string()).unwrap_or_default();
    let show_meta = due.is_some() || streak_on || !task.labels.is_empty() || deadline.is_some() || has_reminders;

    let id_complete = task.id.clone();
    let id_delete = task.id.clone();
    let id_toggle = task.id.clone();
    let id_goal = task.id.clone();

    // One root per row: keyed rows with several roots (the row plus the
    // sub-task editor's placeholder) broke Dioxus' moves when a priority
    // change reordered the list, and the page went blank.
    rsx! {
        div { class: "task-slot",
            div { class: "task", style: "{indent}",
                div { class: "task-row",
                    if has_children {
                        button {
                            class: if collapsed { "chevron collapsed" } else { "chevron" },
                            title: if collapsed { "Show sub-tasks" } else { "Hide sub-tasks" },
                            onclick: move |_| {
                                let id = id_collapse.clone();
                                spawn(async move {
                                    let _ = api::set_collapsed(id, !collapsed).await;
                                    reload += 1;
                                });
                            },
                            Icon { svg: icons::chevron() }
                        }
                    }
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
                    div {
                        class: "task-body",
                        title: "Edit task",
                        onclick: {
                            let (task, mine) = (task.clone(), my_reminders.clone());
                            let (p, s, l) = native_lists.clone();
                            move |_| {
                                if (glass.native)() {
                                    glass.edit(&task, mine.clone(), p.clone(), s.clone(), l.clone(), reload);
                                } else {
                                    editing.set(true);
                                }
                            }
                        },
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
                                if let Some((cls, label)) = deadline.clone() {
                                    span { class: "deadline {cls}", title: "Deadline",
                                        Icon { svg: icons::target_small() }
                                        "{label}"
                                    }
                                }
                                if has_reminders {
                                    span { class: "reminder-tag", title: "{reminder_title}",
                                        Icon { svg: icons::bell_small() }
                                    }
                                }
                                for name in task.labels.iter() {
                                    span { key: "{name}", class: "label-tag",
                                        Icon { svg: icons::tag_small() }
                                        "{name}"
                                    }
                                }
                                if streak_on {
                                    span { class: "streak", title: "Current streak", "🔥 {task.streak_current}" }
                                }
                            }
                        }
                    }
                    div { class: "task-actions",
                        button {
                            class: "icon-btn",
                            title: "Add sub-task",
                            onclick: {
                                let defaults = native_sub.clone();
                                let (p, s, l) = native_lists.clone();
                                move |_| {
                                    if (glass.native)() {
                                        glass.add(defaults.clone(), p.clone(), s.clone(), l.clone(), reload);
                                    } else {
                                        adding_sub.set(true);
                                    }
                                }
                            },
                            Icon { svg: icons::subtask() }
                        }
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
            if adding_sub() {
                div { style: "{sub_indent}",
                    TaskEditor {
                        initial: sub_defaults,
                        projects,
                        sections,
                        labels,
                        keep_label: None,
                        submit_text: "Add sub-task".to_string(),
                        quick_add: true,
                        on_submit: move |v: EditorValues| {
                            let (p, s, l) = (sub_projects.clone(), sub_sections.clone(), sub_labels.clone());
                            adding_sub.set(false);
                            spawn(async move {
                                add_from_editor(v, p, s, l).await;
                                reload += 1;
                            });
                        },
                        on_cancel: move |_| adding_sub.set(false),
                        reload,
                    }
                }
            }
        }
    }
}

/// A completed task in the "Completed" section: click the filled check to
/// reopen it.
#[component]
pub fn CompletedRow(task: Task, today: NaiveDate, project_name: Option<String>, reload: Signal<u32>) -> Element {
    let mut reload = reload;
    let when = task
        .completed_at
        .map(|c| views::due_label(c.with_timezone(&chrono::Local).date_naive(), today))
        .unwrap_or_default();
    let id_reopen = task.id.clone();
    let id_delete = task.id.clone();
    rsx! {
        div { class: "task completed",
            div { class: "task-row",
                button {
                    class: "check done",
                    title: "Mark as not done",
                    onclick: move |_| {
                        let id = id_reopen.clone();
                        spawn(async move {
                            let _ = api::uncomplete_task(id).await;
                            reload += 1;
                        });
                    },
                    Icon { svg: icons::check() }
                }
                div { class: "task-body",
                    div { class: "task-content", "{task.content}" }
                    div { class: "task-meta",
                        span { class: "done-when", "Completed · {when}" }
                    }
                }
                div { class: "task-actions",
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
        }
    }
}
