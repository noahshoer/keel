use chrono::NaiveDate;
use dioxus::prelude::*;
use keel_core::model::{Label, Priority, Project, Reminder, Section, Task};
use keel_core::views::{self, DueBucket, Progress};

use crate::api;
use crate::editor::EditorValues;
use crate::icons::{self, Icon};
use crate::pickers::ReminderDraft;
use crate::task_view::nav::use_nav;

pub(crate) fn check_class(p: Priority) -> &'static str {
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

/// A task's due label with its color class: "Fri 18:00", "Today".
pub fn due_text(task: &Task, today: NaiveDate) -> Option<(&'static str, String)> {
    views::due_date_of(task).map(|d| {
        let label = views::due_label(d, today);
        let label = match views::due_time(task) {
            Some(t) => format!("{label} {}", views::time_label(t)),
            None => label,
        };
        (due_class(views::due_bucket(d, today)), label)
    })
}

/// "x/y" for a task's sub-tasks, then a round green check when all are done.
#[component]
pub fn SubtaskCount(progress: Progress) -> Element {
    rsx! {
        span { class: "task-count", "{progress.short()}" }
        if progress.all_done() {
            span { class: "done-check", title: "All sub-tasks done", Icon { svg: icons::check_small() } }
        }
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
    /// Sub-task counts, shown as "x/y" next to the name.
    #[props(default)]
    progress: Option<Progress>,
    reload: Signal<u32>,
) -> Element {
    let mut reload = reload;
    let mut panel = use_signal(|| false);
    let glass = crate::glass::use_glass();
    let nav = use_nav();
    // Completing: 0 idle, 1 circle filled, 2 switching off. A tap while
    // filled cancels (`gen` invalidates the pending steps).
    let completion_icon = use_context::<Signal<crate::completion::CompletionIcon>>();
    let mut phase = use_signal(|| 0u8);
    let mut gen = use_signal(|| 0u32);
    // After completing, the row stays switched off until the reload drops
    // it (resetting first flashed it back in). A repeating task stays in
    // project views with a new due date; that new date brings it back.
    let mut done_due = use_signal(|| None::<String>);
    let due_now = task.due.as_ref().map(|d| d.date.clone()).unwrap_or_default();
    use_effect(use_reactive!(|due_now| {
        if done_due.peek().as_ref().is_some_and(|d| *d != due_now) {
            done_due.set(None);
            phase.set(0);
        }
    }));
    // All task reminders, shared app-wide (see `App`).
    let all_reminders = use_context::<Signal<Vec<Reminder>>>();
    let my_reminders: Vec<ReminderDraft> = all_reminders
        .read()
        .iter()
        .filter(|r| r.task_id == task.id)
        .map(ReminderDraft::from_reminder)
        .collect();
    // For the native card (iPhone): what "Add sub-task" opens with.
    let native_lists = (projects.clone(), sections.clone(), labels.clone());
    let mut native_sub = EditorValues::blank(task.project_id.clone(), task.section_id.clone(), String::new(), Vec::new());
    native_sub.parent_id = Some(task.id.clone());
    // iPhone: a long press opens the native action menu (the desktop's
    // hover buttons).
    {
        let (task_id, defaults, (p, s, l)) = (task.id.clone(), native_sub.clone(), native_lists.clone());
        let recurring = task.due.as_ref().is_some_and(|d| d.is_recurring);
        let items = crate::glass::task_actions(recurring);
        crate::glass::use_row_menu(task.id.to_string(), task.content.clone(), items, move |action| match action {
            crate::glass::TaskAction::SubTask => glass.add(defaults.clone(), p.clone(), s.clone(), l.clone(), reload),
            crate::glass::TaskAction::Streak => {
                let open = *panel.peek();
                panel.set(!open);
            }
            crate::glass::TaskAction::Delete => {
                let id = task_id.clone();
                spawn(async move {
                    let _ = api::delete_task(id).await;
                    reload += 1;
                });
            }
            crate::glass::TaskAction::Rename => {}
        });
    }

    let deadline = task
        .deadline
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| (if d <= today { "urgent" } else { "set" }, views::due_label(d, today)));
    let has_reminders = !my_reminders.is_empty();
    let reminder_title = my_reminders.iter().map(|r| r.label(today)).collect::<Vec<_>>().join(", ");
    let due = due_text(&task, today);
    let recurring = task.due.as_ref().is_some_and(|d| d.is_recurring);
    let streak_on = task.streak_enabled;
    let show_meta = due.is_some() || streak_on || !task.labels.is_empty() || deadline.is_some() || has_reminders;

    let id_complete = task.id.clone();
    let id_delete = task.id.clone();
    let id_open = task.id.clone();

    // One root per row: keyed rows with several roots broke Dioxus' moves
    // when a priority change reordered the list, and the page went blank.
    rsx! {
        div {
            class: match phase() { 1 => "task completing", 2 => "task completing switch-off", _ => "task" },
            "data-menu-id": "{task.id}",
            div { class: "task-row",
                button {
                    class: check_class(task.priority),
                    title: "Complete task",
                    onclick: move |_| {
                        use crate::completion::{sleep, FILL_MS, OFF_MS};
                        match phase() {
                            1 => {
                                gen += 1;
                                phase.set(0);
                                return;
                            }
                            2 => return,
                            _ => {}
                        }
                        gen += 1;
                        let mine = gen();
                        phase.set(1);
                        let id = id_complete.clone();
                        let due = due_now.clone();
                        spawn(async move {
                            sleep(FILL_MS).await;
                            if gen() != mine {
                                return;
                            }
                            phase.set(2);
                            sleep(OFF_MS).await;
                            if gen() != mine {
                                return;
                            }
                            // Hidden before the save so the reload can't flash it
                            // back; if the save fails, the row returns unchecked.
                            done_due.set(Some(due));
                            if api::complete_task(id).await.is_err() {
                                done_due.set(None);
                                phase.set(0);
                            }
                            reload += 1;
                        });
                    },
                    Icon { svg: completion_icon().svg() }
                }
                div {
                    class: "task-body",
                    title: "Open task",
                    onclick: move |_| nav.open_chain(id_open.clone(), &nav.all.read()),
                    div { class: "task-title",
                        div { class: "task-content", "{task.content}" }
                        if let Some(p) = progress {
                            SubtaskCount { progress: p }
                        }
                    }
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
                            let id = task.id.clone();
                            move |_| nav.open_adding(id.clone(), &nav.all.read())
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
                StreakPanel { task: task.clone(), reload }
            }
        }
    }
}

/// A repeating task's streak: current and best, a goal, and tracking on or off.
#[component]
pub fn StreakPanel(task: Task, reload: Signal<u32>) -> Element {
    let mut reload = reload;
    let streak_on = task.streak_enabled;
    let goal = task.streak_goal;
    let goal_value = goal.map(|g| g.to_string()).unwrap_or_default();
    let id_toggle = task.id.clone();
    let id_goal = task.id.clone();
    rsx! {
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

/// A completed task in the "Completed" section: click the filled check to
/// reopen it, or the rest of the row to open its view.
#[component]
pub fn CompletedRow(task: Task, today: NaiveDate, project_name: Option<String>, reload: Signal<u32>) -> Element {
    let mut reload = reload;
    let when = task
        .completed_at
        .map(|c| views::due_label(c.with_timezone(&chrono::Local).date_naive(), today))
        .unwrap_or_default();
    let id_reopen = task.id.clone();
    let id_delete = task.id.clone();
    let id_open = task.id.clone();
    let nav = use_nav();
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
                div {
                    class: "task-body",
                    title: "Open task",
                    onclick: move |_| nav.open_chain(id_open.clone(), &nav.all.read()),
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
