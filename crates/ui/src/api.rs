//! Typed wrappers over the Tauri IPC `invoke` bridge.
//! Requires `app.withGlobalTauri = true` in tauri.conf.json.

use serde::Serialize;
use todo_core::commands::{Action, ApplyResult, Command};
use todo_core::ids::Id;
use todo_core::model::{Due, Priority, Project, Task};
use todo_core::recurrence::Recurrence;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
export async function __invoke(cmd, args) {
  return await window.__TAURI__.core.invoke(cmd, args);
}
export function __uuid() { return crypto.randomUUID(); }
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn __invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
    fn __uuid() -> String;
}

/// A fresh client-side identifier for command `uuid`/`temp_id`.
pub fn uuid() -> String {
    __uuid()
}

async fn invoke<A: Serialize, R: for<'de> serde::Deserialize<'de>>(
    cmd: &str,
    args: &A,
) -> Result<R, String> {
    let args = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    let out = __invoke(cmd, args).await.map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(out).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct NoArgs {}

#[derive(Serialize)]
struct ProjectArg<'a> {
    project_id: &'a str,
}

#[derive(Serialize)]
struct DaysArg {
    days: i64,
}

#[derive(Serialize)]
struct CmdArg<'a> {
    cmd: &'a Command,
}

pub async fn list_projects() -> Result<Vec<Project>, String> {
    invoke("list_projects", &NoArgs {}).await
}

pub async fn today_view() -> Result<Vec<Task>, String> {
    invoke("today_view", &NoArgs {}).await
}

pub async fn upcoming_view(days: i64) -> Result<Vec<Task>, String> {
    invoke("upcoming_view", &DaysArg { days }).await
}

pub async fn project_view(project_id: &str) -> Result<Vec<Task>, String> {
    invoke("project_view", &ProjectArg { project_id }).await
}

pub async fn dispatch(cmd: &Command) -> Result<ApplyResult, String> {
    invoke("dispatch", &CmdArg { cmd }).await
}

fn command(action: Action, temp: bool) -> Command {
    Command {
        uuid: uuid(),
        temp_id: temp.then(|| format!("tmp-{}", uuid())),
        action,
    }
}

/// Fields from the add-task editor.
pub struct NewTask {
    pub content: String,
    pub description: String,
    pub project_id: Id,
    /// `YYYY-MM-DD`, or empty for no date.
    pub date: String,
    pub priority: Priority,
    pub recurrence: Option<Recurrence>,
}

pub async fn add_task(t: NewTask) -> Result<ApplyResult, String> {
    let due = (!t.date.is_empty()).then(|| Due {
        date: t.date.clone(),
        timezone: None,
        string: t.date.clone(),
        lang: "en".to_string(),
        is_recurring: false,
        recurrence: None,
    });
    let description = (!t.description.trim().is_empty()).then(|| t.description.trim().to_string());
    dispatch(&command(
        Action::TaskAdd {
            content: t.content,
            project_id: t.project_id,
            description,
            priority: Some(t.priority),
            due,
            labels: None,
            parent_id: None,
            section_id: None,
            recurrence: t.recurrence,
        },
        true,
    ))
    .await
}

pub async fn add_project(name: String) -> Result<ApplyResult, String> {
    dispatch(&command(Action::ProjectAdd { name }, true)).await
}

pub async fn complete_task(id: Id) -> Result<ApplyResult, String> {
    dispatch(&command(Action::TaskComplete { id }, false)).await
}

pub async fn delete_task(id: Id) -> Result<ApplyResult, String> {
    dispatch(&command(Action::TaskDelete { id }, false)).await
}

pub async fn set_streak(id: Id, enabled: bool, goal: Option<i64>) -> Result<ApplyResult, String> {
    dispatch(&command(Action::TaskSetStreak { id, enabled, goal }, false)).await
}
