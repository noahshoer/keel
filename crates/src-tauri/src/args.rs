//! One argument type per IPC command: each command takes `args: <Name>Args`
//! (or `NoArgs`), so the frontend sends `{ args: { … } }`. Field names are
//! the snake_case names the commands took before, with the same types, so
//! serde accepts the same values. `--features ts` exports them to
//! `web/src/bindings` (`tests/commands_list.rs`).

use keel_core::commands::Command;
use keel_core::reminders::ReminderSettings;
use serde::Deserialize;

/// For commands that take nothing; the frontend sends `{ args: {} }`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct NoArgs {}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ListTasksArgs {
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct UpcomingViewArgs {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub days: i64,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ProjectViewArgs {
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct LabelViewArgs {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CompletedInProjectArgs {
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CompletedWithLabelArgs {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ListSectionsArgs {
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CheckFilterArgs {
    pub query: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct FilterViewArgs {
    pub query: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DispatchArgs {
    pub cmd: Command,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SearchArgs {
    pub query: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ImportTodoistCsvArgs {
    pub project_name: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SetReminderSettingsArgs {
    pub settings: ReminderSettings,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SetDiagEnabledArgs {
    pub on: bool,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DiagLogArgs {
    pub source: String,
    pub msg: String,
}
