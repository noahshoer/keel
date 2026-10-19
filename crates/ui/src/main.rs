mod api;

use dioxus::prelude::*;
use todo_core::model::{Project, Task};
use todo_core::recurrence::Recurrence;

fn main() {
    dioxus::launch(App);
}

/// Quick-add recurrence choices (full natural-language input comes later).
fn recurrence_for(choice: &str) -> Option<Recurrence> {
    match choice {
        "daily" => Some(Recurrence::Daily),
        "weekdays" => Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])),
        "monthly" => Some(Recurrence::MonthlyDay(1)),
        _ => None,
    }
}

#[component]
fn App() -> Element {
    let mut reload = use_signal(|| 0u32);
    let mut new_task = use_signal(String::new);
    let mut repeat = use_signal(|| "none".to_string());

    // Reloads whenever `reload` changes.
    let data = use_resource(move || async move {
        let _ = reload();
        let projects = api::list_projects().await?;
        let inbox = projects
            .into_iter()
            .find(|p| p.is_inbox)
            .ok_or_else(|| "no inbox project".to_string())?;
        let tasks = api::list_tasks(inbox.id.as_str()).await?;
        Ok::<(Project, Vec<Task>), String>((inbox, tasks))
    });

    rsx! {
        div {
            style: "max-width:720px;margin:0 auto;padding:24px;font-family:system-ui,sans-serif;",
            h1 { "Today" }
            match &*data.read_unchecked() {
                Some(Ok((inbox, tasks))) => {
                    let inbox_id = inbox.id.clone();
                    let tasks = tasks.clone();
                    rsx! {
                        form {
                            style: "display:flex;gap:8px;",
                            onsubmit: move |e| {
                                e.prevent_default();
                                let content = new_task().trim().to_string();
                                if content.is_empty() { return; }
                                let pid = inbox_id.clone();
                                let rec = recurrence_for(&repeat());
                                spawn(async move {
                                    let _ = api::add_task(content, pid, rec).await;
                                    new_task.set(String::new());
                                    reload += 1;
                                });
                            },
                            input {
                                style: "flex:1;padding:8px;",
                                value: "{new_task}",
                                placeholder: "Add a task…",
                                oninput: move |e| new_task.set(e.value()),
                            }
                            select {
                                style: "padding:8px;",
                                value: "{repeat}",
                                onchange: move |e| repeat.set(e.value()),
                                option { value: "none", "Doesn't repeat" }
                                option { value: "daily", "Every day" }
                                option { value: "weekdays", "Every weekday" }
                                option { value: "monthly", "Every month (1st)" }
                            }
                            button { r#type: "submit", style: "padding:8px 12px;", "Add" }
                        }
                        ul { style: "list-style:none;padding:0;",
                            for task in tasks {
                                TaskRow { key: "{task.id}", task: task.clone(), reload }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! { p { style: "color:#b00;", "Error: {e}" } },
                None => rsx! { p { "Loading…" } },
            }
        }
    }
}

#[component]
fn TaskRow(task: Task, reload: Signal<u32>) -> Element {
    let recurring = task.due.as_ref().is_some_and(|d| d.is_recurring);
    let due_label = task.due.as_ref().map(|d| {
        if d.is_recurring {
            format!("{} · next {}", d.string, d.date)
        } else {
            d.date.clone()
        }
    });

    let id_complete = task.id.clone();
    let id_delete = task.id.clone();
    let id_toggle = task.id.clone();
    let id_goal = task.id.clone();
    let streak_on = task.streak_enabled;
    let goal = task.streak_goal;
    let mut r1 = reload;
    let mut r2 = reload;
    let mut r3 = reload;
    let mut r4 = reload;

    let goal_text = goal.map(|g| format!(" · goal {g}")).unwrap_or_default();
    let badge = format!("🔥 {} (best {}){}", task.streak_current, task.streak_best, goal_text);
    let goal_value = goal.map(|g| g.to_string()).unwrap_or_default();

    rsx! {
        li {
            style: "padding:8px 0;border-bottom:1px solid #eee;",
            div {
                style: "display:flex;align-items:center;gap:8px;",
                input {
                    r#type: "checkbox",
                    checked: task.checked,
                    onclick: move |_| {
                        let id = id_complete.clone();
                        spawn(async move {
                            let _ = api::complete_task(id).await;
                            r1 += 1;
                        });
                    },
                }
                span {
                    style: if task.checked { "flex:1;text-decoration:line-through;color:#999;" } else { "flex:1;" },
                    "{task.content}"
                }
                if streak_on {
                    span { style: "font-size:13px;color:#c2410c;", "{badge}" }
                }
                button {
                    style: "border:none;background:none;color:#b00;cursor:pointer;",
                    onclick: move |_| {
                        let id = id_delete.clone();
                        spawn(async move {
                            let _ = api::delete_task(id).await;
                            r2 += 1;
                        });
                    },
                    "✕"
                }
            }
            if let Some(label) = due_label {
                div { style: "font-size:12px;color:#666;margin-left:28px;", "{label}" }
            }
            if recurring {
                div {
                    style: "display:flex;align-items:center;gap:8px;margin-left:28px;font-size:12px;",
                    button {
                        style: "font-size:12px;padding:2px 8px;",
                        onclick: move |_| {
                            let id = id_toggle.clone();
                            spawn(async move {
                                let _ = api::set_streak(id, !streak_on, goal).await;
                                r3 += 1;
                            });
                        },
                        if streak_on { "Stop tracking streak" } else { "Track streak" }
                    }
                    if streak_on {
                        label { "Goal " }
                        input {
                            r#type: "number",
                            min: "1",
                            style: "width:64px;",
                            value: "{goal_value}",
                            onchange: move |e| {
                                let id = id_goal.clone();
                                let new_goal = e.value().trim().parse::<i64>().ok().filter(|g| *g > 0);
                                spawn(async move {
                                    let _ = api::set_streak(id, true, new_goal).await;
                                    r4 += 1;
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}
