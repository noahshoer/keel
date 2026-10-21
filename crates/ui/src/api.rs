//! Typed wrappers over the Tauri IPC `invoke` bridge.
//! Requires `app.withGlobalTauri = true` in tauri.conf.json.

use serde::Serialize;
use keel_core::commands::{Action, ApplyResult, Command, DueEdit};
use keel_core::ids::Id;
use keel_core::filter::FilterGroup;
use keel_core::model::{Due, Filter, Label, Priority, Project, Reminder, Section, Task};
use keel_core::recurrence::Recurrence;
use keel_core::reminders::{PlannedReminder, ReminderSettings};
use keel_core::import::todoist::ImportSummary;
use keel_core::search::SearchResults;
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
    // Backend errors arrive as plain strings; show them as-is.
    let out = __invoke(cmd, args)
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
    serde_wasm_bindgen::from_value(out).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct NoArgs {}

#[derive(Serialize)]
struct ProjectArg<'a> {
    project_id: &'a str,
}

#[derive(Serialize)]
struct NameArg<'a> {
    name: &'a str,
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

pub async fn list_labels() -> Result<Vec<Label>, String> {
    invoke("list_labels", &NoArgs {}).await
}

pub async fn label_view(name: &str) -> Result<Vec<Task>, String> {
    invoke("label_view", &NameArg { name }).await
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
    pub labels: Vec<String>,
    pub section_id: Option<Id>,
    /// Set when adding a sub-task.
    pub parent_id: Option<Id>,
    /// `YYYY-MM-DD`, or empty for no deadline.
    pub deadline: String,
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
            labels: (!t.labels.is_empty()).then_some(t.labels),
            parent_id: t.parent_id,
            section_id: t.section_id,
            recurrence: t.recurrence,
            deadline: (!t.deadline.is_empty()).then_some(t.deadline),
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

pub async fn add_label(name: String) -> Result<ApplyResult, String> {
    dispatch(&command(Action::LabelAdd { name }, true)).await
}

/// Only the fields that changed; `None` leaves a field as it is.
#[derive(Default)]
pub struct TaskPatch {
    pub content: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub project_id: Option<Id>,
    pub labels: Option<Vec<String>>,
    pub due: Option<DueEdit>,
    /// `Some("")` clears the deadline.
    pub deadline: Option<String>,
}

impl TaskPatch {
    pub fn is_empty(&self) -> bool {
        self.content.is_none()
            && self.description.is_none()
            && self.priority.is_none()
            && self.project_id.is_none()
            && self.labels.is_none()
            && self.due.is_none()
            && self.deadline.is_none()
    }
}

pub async fn update_task(id: Id, p: TaskPatch) -> Result<ApplyResult, String> {
    dispatch(&command(
        Action::TaskUpdate {
            id,
            content: p.content,
            description: p.description,
            priority: p.priority,
            project_id: p.project_id,
            labels: p.labels,
            is_collapsed: None,
            due: p.due,
            deadline: p.deadline,
        },
        false,
    ))
    .await
}

pub async fn completed_today() -> Result<Vec<Task>, String> {
    invoke("completed_today", &NoArgs {}).await
}

pub async fn completed_in_project(project_id: &str) -> Result<Vec<Task>, String> {
    invoke("completed_in_project", &ProjectArg { project_id }).await
}

pub async fn completed_with_label(name: &str) -> Result<Vec<Task>, String> {
    invoke("completed_with_label", &NameArg { name }).await
}

pub async fn uncomplete_task(id: Id) -> Result<ApplyResult, String> {
    dispatch(&command(Action::TaskUncomplete { id }, false)).await
}

pub async fn rename_project(id: Id, name: String) -> Result<ApplyResult, String> {
    dispatch(&command(Action::ProjectUpdate { id, name }, false)).await
}

pub async fn delete_project(id: Id) -> Result<ApplyResult, String> {
    dispatch(&command(Action::ProjectDelete { id }, false)).await
}

pub async fn rename_label(id: Id, name: String) -> Result<ApplyResult, String> {
    dispatch(&command(Action::LabelUpdate { id, name }, false)).await
}

pub async fn delete_label(id: Id) -> Result<ApplyResult, String> {
    dispatch(&command(Action::LabelDelete { id }, false)).await
}

pub async fn list_sections(project_id: &str) -> Result<Vec<Section>, String> {
    invoke("list_sections", &ProjectArg { project_id }).await
}

pub async fn add_section(project_id: Id, name: String) -> Result<ApplyResult, String> {
    dispatch(&command(Action::SectionAdd { project_id, name }, true)).await
}

pub async fn rename_section(id: Id, name: String) -> Result<ApplyResult, String> {
    dispatch(&command(Action::SectionUpdate { id, name }, false)).await
}

pub async fn delete_section(id: Id) -> Result<ApplyResult, String> {
    dispatch(&command(Action::SectionDelete { id }, false)).await
}

/// Move a task (and its sub-tasks) to a project and optional section.
pub async fn move_task(id: Id, project_id: Id, section_id: Option<Id>) -> Result<ApplyResult, String> {
    dispatch(&command(Action::TaskMove { id, project_id, section_id }, false)).await
}

pub async fn set_collapsed(id: Id, collapsed: bool) -> Result<ApplyResult, String> {
    dispatch(&command(
        Action::TaskUpdate {
            id,
            content: None,
            description: None,
            priority: None,
            project_id: None,
            labels: None,
            due: None,
            is_collapsed: Some(collapsed),
            deadline: None,
        },
        false,
    ))
    .await
}

#[derive(Serialize)]
struct QueryArg<'a> {
    query: &'a str,
}

pub async fn list_filters() -> Result<Vec<Filter>, String> {
    invoke("list_filters", &NoArgs {}).await
}

/// Ok when `query` is valid; otherwise the problem, in words.
pub async fn check_filter(query: &str) -> Result<(), String> {
    invoke::<_, ()>("check_filter", &QueryArg { query }).await
}

pub async fn filter_view(query: &str) -> Result<Vec<FilterGroup>, String> {
    invoke("filter_view", &QueryArg { query }).await
}

pub async fn add_filter(name: String, query: String) -> Result<ApplyResult, String> {
    dispatch(&command(Action::FilterAdd { name, query }, true)).await
}

pub async fn update_filter(id: Id, name: String, query: String) -> Result<ApplyResult, String> {
    dispatch(&command(Action::FilterUpdate { id, name: Some(name), query: Some(query) }, false)).await
}

pub async fn delete_filter(id: Id) -> Result<ApplyResult, String> {
    dispatch(&command(Action::FilterDelete { id }, false)).await
}

#[derive(Serialize)]
struct SettingsArg<'a> {
    settings: &'a ReminderSettings,
}

pub async fn get_reminder_settings() -> Result<ReminderSettings, String> {
    invoke("get_reminder_settings", &NoArgs {}).await
}

pub async fn set_reminder_settings(settings: &ReminderSettings) -> Result<(), String> {
    invoke::<_, ()>("set_reminder_settings", &SettingsArg { settings }).await
}

pub async fn upcoming_reminders() -> Result<Vec<PlannedReminder>, String> {
    invoke("upcoming_reminders", &NoArgs {}).await
}

/// "granted" | "denied" | "prompt".
pub async fn notification_permission() -> Result<String, String> {
    invoke("notification_permission", &NoArgs {}).await
}

/// True where the OS delivers reminders while the app is closed (iOS).
pub async fn reminders_when_closed() -> Result<bool, String> {
    invoke("reminders_when_closed", &NoArgs {}).await
}

pub async fn request_notification_permission() -> Result<String, String> {
    invoke("request_notification_permission", &NoArgs {}).await
}

pub async fn test_notification() -> Result<(), String> {
    invoke::<_, ()>("test_notification", &NoArgs {}).await
}

pub async fn search(query: &str) -> Result<SearchResults, String> {
    invoke("search", &QueryArg { query }).await
}

pub async fn list_reminders() -> Result<Vec<Reminder>, String> {
    invoke("list_reminders", &NoArgs {}).await
}

/// `minute_offset` before the due time, or at `at` (`YYYY-MM-DDTHH:MM`).
pub async fn add_reminder(task_id: Id, minute_offset: Option<i64>, at: Option<String>) -> Result<ApplyResult, String> {
    dispatch(&command(Action::ReminderAdd { task_id, minute_offset, at }, true)).await
}

pub async fn delete_reminder(id: Id) -> Result<ApplyResult, String> {
    dispatch(&command(Action::ReminderDelete { id }, false)).await
}

#[derive(Serialize)]
struct ImportArgs<'a> {
    project_name: &'a str,
    text: &'a str,
}

/// Imports one Todoist project CSV export as a new project.
pub async fn import_todoist_csv(project_name: &str, text: &str) -> Result<ImportSummary, String> {
    invoke("import_todoist_csv", &ImportArgs { project_name, text }).await
}

#[derive(Serialize)]
struct GlassStateArg<'a> {
    state: &'a crate::glass::CardState,
}

/// True on iOS, where `tauri-plugin-glass` draws the native card and buttons.
pub async fn glass_available() -> bool {
    invoke("plugin:glass|is_available", &NoArgs {}).await.unwrap_or(false)
}

/// Opens the native card, replacing whatever its fields hold.
pub async fn glass_show(state: &crate::glass::CardState) {
    let _ = invoke::<_, ()>("plugin:glass|show", &GlassStateArg { state }).await;
}

/// Refreshes the open card (highlights, chips) after an edit.
pub async fn glass_update(state: &crate::glass::CardState) {
    let _ = invoke::<_, ()>("plugin:glass|update", &GlassStateArg { state }).await;
}

pub async fn glass_hide() {
    let _ = invoke::<_, ()>("plugin:glass|hide", &NoArgs {}).await;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionsArg<'a> {
    task_id: &'a Id,
    title: &'a str,
    items: Vec<crate::glass::ActionItem>,
}

/// Opens the native action menu for a long-pressed task.
pub async fn glass_actions(task_id: &Id, title: &str, items: Vec<crate::glass::ActionItem>) {
    let _ = invoke::<_, ()>("plugin:glass|actions", &ActionsArg { task_id, title, items }).await;
}

/// Shows or hides the floating glass buttons.
pub async fn glass_buttons(state: crate::glass::ButtonsState) {
    let _ = invoke::<_, ()>("plugin:glass|buttons", &state).await;
}
