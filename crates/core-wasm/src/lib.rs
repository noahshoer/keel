//! `keel-core` for the web UI (`web/src/core`), over wasm-bindgen.
//!
//! Each export takes and returns the ts-rs binding shapes (`web/src/bindings`):
//! inputs are deserialized with serde-wasm-bindgen, outputs serialized with
//! `Serializer::json_compatible()` so `Option` is `null`, maps are plain
//! objects and `i64` is a `number`, exactly as over IPC. The
//! `unchecked_*_type` attributes name those bindings, so the generated
//! `.d.ts` has no `any`. The paths are relative to `web/src/core/pkg/`.
//!
//! Only data rules live here, and only as thin calls into `keel_core`: date
//! strings are parsed here, everything else is `keel_core`'s.

use chrono::{NaiveDate, NaiveTime};
use keel_core::ids::Id;
use keel_core::model::{Reminder, Task};
use keel_core::reminders;
use keel_core::views::{self, CardMode, DateGroupMode, Progress};
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Parses a strict `YYYY-MM-DD` date (conversion only: which date a `Due`
/// falls on is `views::due_date_of`, exported as `dueDateOf`).
fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| format!("invalid date \"{s}\": expected YYYY-MM-DD"))
}

/// Parses an `HH:MM` or `HH:MM:SS` time of day.
fn parse_time(s: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(s, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M"))
        .map_err(|_| format!("invalid time \"{s}\": expected HH:MM or HH:MM:SS"))
}

fn input<T: DeserializeOwned>(name: &str, value: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&format!("invalid {name}: {e}")))
}

fn output<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&format!("serialize: {e}")))
}

/// `views::due_label` for two `YYYY-MM-DD` date strings.
#[wasm_bindgen(js_name = dueLabel)]
pub fn due_label(due: &str, today: &str) -> Result<String, JsError> {
    let due = parse_date(due).map_err(|e| JsError::new(&e))?;
    let today = parse_date(today).map_err(|e| JsError::new(&e))?;
    Ok(views::due_label(due, today))
}

/// `views::due_date_of`: the calendar date of a task's due as `YYYY-MM-DD`,
/// `null` without one.
#[wasm_bindgen(js_name = dueDateOf, unchecked_return_type = "string | null")]
pub fn due_date_of(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task")] task: JsValue,
) -> Result<JsValue, JsError> {
    let task: Task = input("task", task)?;
    output(&views::due_date_of(&task).map(|d| d.format("%Y-%m-%d").to_string()))
}

/// `views::due_time`: the time of day of a task's due as `HH:MM:SS`, `null`
/// for a date-only due or none.
#[wasm_bindgen(js_name = dueTime, unchecked_return_type = "string | null")]
pub fn due_time(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task")] task: JsValue,
) -> Result<JsValue, JsError> {
    let task: Task = input("task", task)?;
    output(&views::due_time(&task).map(|t| t.format("%H:%M:%S").to_string()))
}

/// `views::time_label` for an `HH:MM[:SS]` string.
#[wasm_bindgen(js_name = timeLabel)]
pub fn time_label(time: &str) -> Result<String, JsError> {
    Ok(views::time_label(parse_time(time).map_err(|e| JsError::new(&e))?))
}

/// `views::progress`: a parent's sub-task counts, `null` when it has none.
#[wasm_bindgen(js_name = progress, unchecked_return_type = "import('../../bindings').Progress | null")]
pub fn progress(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Id")] parent: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] all: JsValue,
) -> Result<JsValue, JsError> {
    let parent: Id = input("parent", parent)?;
    let all: Vec<Task> = input("all", all)?;
    output(&views::progress(&parent, &all))
}

/// `views::cards`: groups a view's matched tasks into cards.
#[wasm_bindgen(js_name = cards, unchecked_return_type = "import('../../bindings').Card[]")]
pub fn cards(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] matched: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] all: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').CardMode")] mode: JsValue,
) -> Result<JsValue, JsError> {
    let matched: Vec<Task> = input("matched", matched)?;
    let all: Vec<Task> = input("all", all)?;
    let mode: CardMode = input("mode", mode)?;
    output(&views::cards(matched, &all, mode))
}

/// `views::due_bucket`: which coloured chip a `YYYY-MM-DD` due gets.
#[wasm_bindgen(js_name = dueBucket, unchecked_return_type = "import('../../bindings').DueBucket")]
pub fn due_bucket(due: &str, today: &str) -> Result<JsValue, JsError> {
    let due = parse_date(due).map_err(|e| JsError::new(&e))?;
    let today = parse_date(today).map_err(|e| JsError::new(&e))?;
    output(&views::due_bucket(due, today))
}

/// `Progress::all_done`: at least one counted sub-task, and all checked.
#[wasm_bindgen(js_name = allDone)]
pub fn all_done(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Progress")] progress: JsValue,
) -> Result<bool, JsError> {
    let progress: Progress = input("progress", progress)?;
    Ok(progress.all_done())
}

/// `views::date_groups`: Today / Upcoming split into titled date groups.
#[wasm_bindgen(js_name = dateGroups, unchecked_return_type = "import('../../bindings').DateGroup[]")]
pub fn date_groups(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').DateGroupMode")] mode: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] tasks: JsValue,
    today: &str,
) -> Result<JsValue, JsError> {
    let mode: DateGroupMode = input("mode", mode)?;
    let tasks: Vec<Task> = input("tasks", tasks)?;
    let today = parse_date(today).map_err(|e| JsError::new(&e))?;
    output(&views::date_groups(mode, tasks, today))
}

/// `views::upcoming_tasks`: the overdue ones from Today's tasks, then the window.
#[wasm_bindgen(js_name = upcomingTasks, unchecked_return_type = "import('../../bindings').Task[]")]
pub fn upcoming_tasks(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] today_tasks: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] window: JsValue,
    today: &str,
) -> Result<JsValue, JsError> {
    let today_tasks: Vec<Task> = input("todayTasks", today_tasks)?;
    let window: Vec<Task> = input("window", window)?;
    let today = parse_date(today).map_err(|e| JsError::new(&e))?;
    output(&views::upcoming_tasks(today_tasks, window, today))
}

/// `views::deadline_urgent` for two `YYYY-MM-DD` dates.
#[wasm_bindgen(js_name = deadlineUrgent)]
pub fn deadline_urgent(deadline: &str, today: &str) -> Result<bool, JsError> {
    let deadline = parse_date(deadline).map_err(|e| JsError::new(&e))?;
    let today = parse_date(today).map_err(|e| JsError::new(&e))?;
    Ok(views::deadline_urgent(deadline, today))
}

/// `reminders::reminder_label`: "30 min before", "Tuesday 9 AM".
#[wasm_bindgen(js_name = reminderLabel)]
pub fn reminder_label(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Reminder")] reminder: JsValue,
    today: &str,
) -> Result<String, JsError> {
    let reminder: Reminder = input("reminder", reminder)?;
    let today = parse_date(today).map_err(|e| JsError::new(&e))?;
    Ok(reminders::reminder_label(&reminder, today))
}

/// `views::day_subtitle` for a `YYYY-MM-DD` date: "Friday 30 Oct".
#[wasm_bindgen(js_name = daySubtitle)]
pub fn day_subtitle(today: &str) -> Result<String, JsError> {
    Ok(views::day_subtitle(parse_date(today).map_err(|e| JsError::new(&e))?))
}

/// `Progress::short`: "2/5".
#[wasm_bindgen(js_name = progressShort)]
pub fn progress_short(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Progress")] progress: JsValue,
) -> Result<String, JsError> {
    let progress: Progress = input("progress", progress)?;
    Ok(progress.short())
}

#[cfg(test)]
mod tests {
    use super::{parse_date, parse_time};
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn parses_a_date() {
        assert_eq!(parse_date("2026-10-30"), Ok(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()));
    }

    #[test]
    fn rejects_malformed_dates() {
        // Strict: a datetime is not a date (use dueDateOf for a Due).
        for bad in [
            "", "30/10/2026", "2026-13-01", "2026-10-30T17:00:00", "2026-10-30 17:00", "2026-10-30x",
        ] {
            assert_eq!(parse_date(bad), Err(format!("invalid date \"{bad}\": expected YYYY-MM-DD")), "{bad}");
        }
    }

    #[test]
    fn parses_times() {
        let t = NaiveTime::from_hms_opt(17, 30, 0).unwrap();
        assert_eq!(parse_time("17:30"), Ok(t));
        assert_eq!(parse_time("17:30:00"), Ok(t));
    }

    #[test]
    fn rejects_malformed_times() {
        assert_eq!(parse_time("5pm"), Err("invalid time \"5pm\": expected HH:MM or HH:MM:SS".to_string()));
        assert_eq!(parse_time("25:00"), Err("invalid time \"25:00\": expected HH:MM or HH:MM:SS".to_string()));
    }
}
