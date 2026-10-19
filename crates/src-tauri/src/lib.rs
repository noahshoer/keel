//! Tauri shell: thin IPC wrappers over `todo_service::AppService`.
//! All domain logic lives in the service/core crates; this layer only
//! bridges the webview to them.

use todo_core::commands::{ApplyResult, Command};
use todo_core::filter::FilterGroup;
use todo_core::model::{Filter, Label, Project, Section, Task};
use todo_service::AppService;

// `rename_all = "snake_case"` so top-level command arg names stay snake_case
// end-to-end (Tauri v2 camelCases them by default). Frontend sends snake_case.
#[tauri::command(rename_all = "snake_case")]
fn list_projects(state: tauri::State<'_, AppService>) -> Result<Vec<Project>, String> {
    state.list_projects().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn list_tasks(state: tauri::State<'_, AppService>, project_id: String) -> Result<Vec<Task>, String> {
    state.list_tasks(&project_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn today_view(state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.today_view().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn upcoming_view(state: tauri::State<'_, AppService>, days: i64) -> Result<Vec<Task>, String> {
    state.upcoming_view(days).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn project_view(state: tauri::State<'_, AppService>, project_id: String) -> Result<Vec<Task>, String> {
    state.project_view(&project_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn list_labels(state: tauri::State<'_, AppService>) -> Result<Vec<Label>, String> {
    state.list_labels().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn label_view(state: tauri::State<'_, AppService>, name: String) -> Result<Vec<Task>, String> {
    state.label_view(&name).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn completed_today(state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.completed_today().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn completed_in_project(state: tauri::State<'_, AppService>, project_id: String) -> Result<Vec<Task>, String> {
    state.completed_in_project(&project_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn completed_with_label(state: tauri::State<'_, AppService>, name: String) -> Result<Vec<Task>, String> {
    state.completed_with_label(&name).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn list_sections(state: tauri::State<'_, AppService>, project_id: String) -> Result<Vec<Section>, String> {
    state.list_sections(&project_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn list_filters(state: tauri::State<'_, AppService>) -> Result<Vec<Filter>, String> {
    state.list_filters().map_err(|e| e.to_string())
}

/// Ok when the query parses; otherwise the human-readable problem.
#[tauri::command(rename_all = "snake_case")]
fn check_filter(state: tauri::State<'_, AppService>, query: String) -> Result<(), String> {
    state.check_filter(&query).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn filter_view(state: tauri::State<'_, AppService>, query: String) -> Result<Vec<FilterGroup>, String> {
    state.filter_view(&query).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn dispatch(state: tauri::State<'_, AppService>, cmd: Command) -> Result<ApplyResult, String> {
    state.dispatch(cmd).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let dir = app
                .path()
                .app_data_dir()
                .expect("resolve app data dir");
            std::fs::create_dir_all(&dir).ok();
            let db_path = dir.join("todo.db");
            let svc = AppService::open(db_path.to_str().expect("utf-8 db path"))
                .expect("open local store");
            app.manage(svc);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_projects,
            list_tasks,
            today_view,
            upcoming_view,
            project_view,
            list_labels,
            label_view,
            completed_today,
            completed_in_project,
            completed_with_label,
            list_sections,
            list_filters,
            check_filter,
            filter_view,
            dispatch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
