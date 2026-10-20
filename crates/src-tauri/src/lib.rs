//! Tauri shell: thin IPC wrappers over `keel_service::AppService`.
//! All domain logic lives in the service/core crates; this layer only
//! bridges the webview to them.

mod notifications;

use tauri_plugin_notification::NotificationExt;
use keel_core::commands::{ApplyResult, Command};
use keel_core::filter::FilterGroup;
use keel_core::model::{Filter, Label, Project, Section, Task};
use keel_core::reminders::{PlannedReminder, ReminderSettings};
use keel_core::search::SearchResults;
use keel_service::AppService;

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
fn dispatch(app: tauri::AppHandle, state: tauri::State<'_, AppService>, cmd: Command) -> Result<ApplyResult, String> {
    let result = state.dispatch(cmd).map_err(|e| e.to_string())?;
    // Any change can move a due date: re-plan notifications.
    notifications::resync(&app);
    Ok(result)
}

#[tauri::command(rename_all = "snake_case")]
fn search(state: tauri::State<'_, AppService>, query: String) -> Result<SearchResults, String> {
    state.search(&query).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn get_reminder_settings(state: tauri::State<'_, AppService>) -> Result<ReminderSettings, String> {
    state.reminder_settings().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn set_reminder_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppService>,
    settings: ReminderSettings,
) -> Result<(), String> {
    state.set_reminder_settings(&settings).map_err(|e| e.to_string())?;
    notifications::resync(&app);
    Ok(())
}

/// The next few planned notifications (for the settings panel).
#[tauri::command(rename_all = "snake_case")]
fn upcoming_reminders(state: tauri::State<'_, AppService>) -> Result<Vec<PlannedReminder>, String> {
    let mut plan = state.reminder_plan().map_err(|e| e.to_string())?;
    plan.truncate(5);
    Ok(plan)
}

/// "granted" | "denied" | "prompt".
#[tauri::command(rename_all = "snake_case")]
fn notification_permission(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.notification().permission_state().map_err(|e| e.to_string())?;
    Ok(notifications::permission_text(state).to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn request_notification_permission(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.notification().request_permission().map_err(|e| e.to_string())?;
    notifications::resync(&app);
    Ok(notifications::permission_text(state).to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn test_notification(app: tauri::AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title("Keel")
        .body("Notifications are working.")
        .show()
        .map_err(|e| e.to_string())
}

/// The app was renamed from "todo-planner" (identifier `com.nshoe.todoplanner`,
/// store `todo.db`). On first launch as Keel, copy that store into the new
/// data dir so existing tasks carry over. The old copy is left in place.
fn migrate_legacy_store(dir: &std::path::Path, db_path: &std::path::Path) {
    if db_path.exists() {
        return;
    }
    let Some(legacy) = dir.parent().map(|p| p.join("com.nshoe.todoplanner").join("todo.db")) else {
        return;
    };
    if legacy.exists() {
        match std::fs::copy(&legacy, db_path) {
            Ok(_) => eprintln!("migrated store from {}", legacy.display()),
            Err(e) => eprintln!("could not migrate store from {}: {e}", legacy.display()),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            use tauri::Manager;
            let dir = app
                .path()
                .app_data_dir()
                .expect("resolve app data dir");
            std::fs::create_dir_all(&dir).ok();
            let db_path = dir.join("keel.db");
            migrate_legacy_store(&dir, &db_path);
            let svc = AppService::open(db_path.to_str().expect("utf-8 db path"))
                .expect("open local store");
            app.manage(svc);
            app.manage(notifications::Reminders::default());
            notifications::resync(app.handle());
            #[cfg(desktop)]
            notifications::start_ticker(app.handle().clone());
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
            dispatch,
            search,
            get_reminder_settings,
            set_reminder_settings,
            upcoming_reminders,
            notification_permission,
            request_notification_permission,
            test_notification
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
