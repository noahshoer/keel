//! Typed wrappers over the Tauri IPC `invoke` bridge.
//! Requires `app.withGlobalTauri = true` in tauri.conf.json.

use serde::Serialize;
use todo_core::commands::{ApplyResult, Command};
use todo_core::model::{Project, Task};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
export async function __invoke(cmd, args) {
  return await window.__TAURI__.core.invoke(cmd, args);
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn __invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen(inline_js = r#"
export function __uuid() { return crypto.randomUUID(); }
"#)]
extern "C" {
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
struct CmdArg<'a> {
    cmd: &'a Command,
}

pub async fn list_projects() -> Result<Vec<Project>, String> {
    invoke("list_projects", &NoArgs {}).await
}

pub async fn list_tasks(project_id: &str) -> Result<Vec<Task>, String> {
    invoke("list_tasks", &ProjectArg { project_id }).await
}

pub async fn dispatch(cmd: &Command) -> Result<ApplyResult, String> {
    invoke("dispatch", &CmdArg { cmd }).await
}
