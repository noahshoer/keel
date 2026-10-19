mod api;

use dioxus::prelude::*;
use todo_core::commands::{Action, Command};
use todo_core::model::{Project, Task};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut reload = use_signal(|| 0u32);
    let mut new_task = use_signal(String::new);

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
                            onsubmit: move |e| {
                                e.prevent_default();
                                let content = new_task().trim().to_string();
                                if content.is_empty() { return; }
                                let pid = inbox_id.clone();
                                spawn(async move {
                                    let cmd = Command {
                                        uuid: api::uuid(),
                                        temp_id: Some(format!("tmp-{}", api::uuid())),
                                        action: Action::TaskAdd {
                                            content,
                                            project_id: pid,
                                            description: None,
                                            priority: None,
                                            due: None,
                                            labels: None,
                                            parent_id: None,
                                            section_id: None,
                                            recurrence: None,
                                        },
                                    };
                                    let _ = api::dispatch(&cmd).await;
                                    new_task.set(String::new());
                                    reload += 1;
                                });
                            },
                            input {
                                style: "width:70%;padding:8px;",
                                value: "{new_task}",
                                placeholder: "Add a task…",
                                oninput: move |e| new_task.set(e.value()),
                            }
                            button { r#type: "submit", style: "padding:8px 12px;margin-left:8px;", "Add" }
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
    let complete_id = task.id.clone();
    let delete_id = task.id.clone();
    let mut reload_c = reload;
    let mut reload_d = reload;
    rsx! {
        li {
            style: "display:flex;align-items:center;gap:8px;padding:6px 0;border-bottom:1px solid #eee;",
            input {
                r#type: "checkbox",
                checked: task.checked,
                onclick: move |_| {
                    let id = complete_id.clone();
                    spawn(async move {
                        let cmd = Command { uuid: api::uuid(), temp_id: None, action: Action::TaskComplete { id } };
                        let _ = api::dispatch(&cmd).await;
                        reload_c += 1;
                    });
                },
            }
            span {
                style: if task.checked { "flex:1;text-decoration:line-through;color:#999;" } else { "flex:1;" },
                "{task.content}"
            }
            button {
                style: "border:none;background:none;color:#b00;cursor:pointer;",
                onclick: move |_| {
                    let id = delete_id.clone();
                    spawn(async move {
                        let cmd = Command { uuid: api::uuid(), temp_id: None, action: Action::TaskDelete { id } };
                        let _ = api::dispatch(&cmd).await;
                        reload_d += 1;
                    });
                },
                "✕"
            }
        }
    }
}
