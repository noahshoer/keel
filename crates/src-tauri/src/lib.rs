//! Tauri shell: thin IPC wrappers over `keel_service::AppService`.
//! All domain logic lives in the service/core crates; this layer only
//! bridges the webview to them.

pub mod args;
mod diag;
mod notifications;

use tauri_plugin_notification::NotificationExt;
use keel_core::commands::ApplyResult;
use keel_core::filter::FilterGroup;
use keel_core::model::{Filter, Label, Project, Reminder, Section, Task};
use keel_core::reminders::{PlannedReminder, ReminderSettings};
use keel_core::import::todoist::ImportSummary;
use keel_core::search::SearchResults;
use keel_service::AppService;

use args::*;

// `rename_all = "snake_case"` so top-level command arg names stay snake_case
// end-to-end (Tauri v2 camelCases them by default). Frontend sends snake_case.
#[tauri::command(rename_all = "snake_case")]
fn list_projects(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Project>, String> {
    state.list_projects().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn list_tasks(args: ListTasksArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.list_tasks(&args.project_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn today_view(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.today_view().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn all_tasks(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.all_tasks().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn upcoming_view(args: UpcomingViewArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.upcoming_view(args.days).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn project_view(args: ProjectViewArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.project_view(&args.project_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn list_labels(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Label>, String> {
    state.list_labels().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn label_view(args: LabelViewArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.label_view(&args.name).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn completed_today(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.completed_today().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn completed_in_project(args: CompletedInProjectArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.completed_in_project(&args.project_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn completed_with_label(args: CompletedWithLabelArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Task>, String> {
    state.completed_with_label(&args.name).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn list_sections(args: ListSectionsArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Section>, String> {
    state.list_sections(&args.project_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn list_filters(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Filter>, String> {
    state.list_filters().map_err(|e| e.to_string())
}

/// Ok when the query parses; otherwise the human-readable problem.
#[tauri::command(rename_all = "snake_case")]
fn check_filter(args: CheckFilterArgs, state: tauri::State<'_, AppService>) -> Result<(), String> {
    state.check_filter(&args.query).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn filter_view(args: FilterViewArgs, state: tauri::State<'_, AppService>) -> Result<Vec<FilterGroup>, String> {
    state.filter_view(&args.query).map_err(|e| e.to_string())
}

// Commands that reach a mobile plugin (notifications) are async: a sync
// command runs on the main thread and blocks it until the plugin's Swift
// answers, which deadlocked with any plugin call (e.g. the glass buttons)
// that needed the main thread to deliver its own result.
#[tauri::command(rename_all = "snake_case")]
async fn dispatch(args: DispatchArgs, app: tauri::AppHandle, state: tauri::State<'_, AppService>) -> Result<ApplyResult, String> {
    let result = state.dispatch(args.cmd).map_err(|e| e.to_string())?;
    // Any change can move a due date: re-plan notifications.
    notifications::resync(&app);
    Ok(result)
}

#[tauri::command(rename_all = "snake_case")]
fn search(args: SearchArgs, state: tauri::State<'_, AppService>) -> Result<SearchResults, String> {
    state.search(&args.query).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
async fn import_todoist_csv(
    args: ImportTodoistCsvArgs,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppService>,
) -> Result<ImportSummary, String> {
    let summary = state.import_todoist_csv(&args.project_name, &args.text)?;
    notifications::resync(&app);
    Ok(summary)
}

#[tauri::command(rename_all = "snake_case")]
fn list_reminders(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<Vec<Reminder>, String> {
    state.list_reminders().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn get_reminder_settings(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<ReminderSettings, String> {
    state.reminder_settings().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
async fn set_reminder_settings(
    args: SetReminderSettingsArgs,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppService>,
) -> Result<(), String> {
    state.set_reminder_settings(&args.settings).map_err(|e| e.to_string())?;
    notifications::resync(&app);
    Ok(())
}

/// The next few planned notifications (for the settings panel).
#[tauri::command(rename_all = "snake_case")]
fn upcoming_reminders(_args: NoArgs, state: tauri::State<'_, AppService>) -> Result<Vec<PlannedReminder>, String> {
    let mut plan = state.reminder_plan().map_err(|e| e.to_string())?;
    plan.truncate(5);
    Ok(plan)
}

/// "granted" | "denied" | "prompt".
#[tauri::command(rename_all = "snake_case")]
async fn notification_permission(_args: NoArgs, app: tauri::AppHandle) -> Result<String, String> {
    let state = app.notification().permission_state().map_err(|e| e.to_string())?;
    Ok(notifications::permission_text(state).to_string())
}

/// Mobile schedules with the OS, so reminders arrive while the app is
/// closed; the desktop ticker only runs while it's open.
#[tauri::command(rename_all = "snake_case")]
fn reminders_when_closed(_args: NoArgs) -> bool {
    cfg!(mobile)
}

#[tauri::command(rename_all = "snake_case")]
async fn request_notification_permission(_args: NoArgs, app: tauri::AppHandle) -> Result<String, String> {
    let state = app.notification().request_permission().map_err(|e| e.to_string())?;
    notifications::resync(&app);
    Ok(notifications::permission_text(state).to_string())
}

#[tauri::command(rename_all = "snake_case")]
async fn test_notification(_args: NoArgs, app: tauri::AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title("Keel")
        .body("Notifications are working.")
        .show()
        .map_err(|e| e.to_string())
}

/// Holds the page's scroll while the task view is open (iOS; see
/// `tauri_plugin_keel_ios::hold_page_scroll`). Async: it calls into Swift.
#[tauri::command(rename_all = "snake_case")]
async fn hold_page_scroll(args: HoldPageScrollArgs, app: tauri::AppHandle) -> Result<(), String> {
    tauri_plugin_keel_ios::hold_page_scroll(&app, args.on)
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
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_keel_ios::init());
    // Liquid Glass is parked: only a build with the `glass` feature loads it.
    #[cfg(feature = "glass")]
    let builder = builder.plugin(tauri_plugin_glass::init());
    builder
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
            diag::init(app.handle(), &app.state::<AppService>(), &dir);
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
            all_tasks,
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
            list_reminders,
            import_todoist_csv,
            get_reminder_settings,
            set_reminder_settings,
            upcoming_reminders,
            notification_permission,
            reminders_when_closed,
            diag::diag_enabled,
            diag::set_diag_enabled,
            diag::diag_log,
            request_notification_permission,
            test_notification,
            hold_page_scroll
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
