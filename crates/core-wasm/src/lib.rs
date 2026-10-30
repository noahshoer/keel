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

use chrono::{NaiveDate, NaiveTime, Weekday};
use keel_core::calendar;
use keel_core::editor::{self, EditorValues, Pick, ReminderDraft};
use keel_core::ids::Id;
use keel_core::model::{Label, Project, Reminder, Section, Task};
use keel_core::quickadd;
use keel_core::recurrence::Recurrence;
use keel_core::reminders;
use keel_core::task_view;
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

/// `Progress::long`: "2 of 5".
#[wasm_bindgen(js_name = progressLong)]
pub fn progress_long(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Progress")] progress: JsValue,
) -> Result<String, JsError> {
    let progress: Progress = input("progress", progress)?;
    Ok(progress.long())
}

/// `task_view::field_layout`: the fields card's rows and pills.
#[wasm_bindgen(js_name = fieldLayout, unchecked_return_type = "import('../../bindings').FieldLayout")]
pub fn field_layout(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] values: JsValue,
    inbox_id: String,
) -> Result<JsValue, JsError> {
    let v: EditorValues = input("values", values)?;
    output(&task_view::field_layout(&v, &Id::from(inbox_id)))
}

/// `task_view::detail`: what one level of the task view renders; `null` for an unknown id.
#[wasm_bindgen(js_name = taskDetail, unchecked_return_type = "import('../../bindings').TaskDetail | null")]
pub fn task_detail(
    id: String,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] all: JsValue,
    show_completed: bool,
) -> Result<JsValue, JsError> {
    let all: Vec<Task> = input("all", all)?;
    output(&task_view::detail(&Id::from(id), &all, show_completed))
}

/// `views::chain`: a task's ancestors from the top, then the task itself.
#[wasm_bindgen(js_name = chain, unchecked_return_type = "string[]")]
pub fn chain(
    id: String,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] all: JsValue,
) -> Result<JsValue, JsError> {
    let all: Vec<Task> = input("all", all)?;
    output(&views::chain(&Id::from(id), &all))
}

/// A browser `<input type="time">` value: `""` (cleared) is `None`,
/// `HH:MM[:SS]` is `Some(HH:MM)`; anything else isn't the input's format.
fn native_time(v: &str) -> Option<Option<String>> {
    if v.is_empty() {
        return Some(None);
    }
    NaiveTime::parse_from_str(v, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(v, "%H:%M:%S"))
        .ok()
        .map(|t| Some(t.format("%H:%M").to_string()))
}

/// An optional `YYYY-MM-DD` (`null` or `""` is none).
fn opt_date(name: &str, value: JsValue) -> Result<Option<NaiveDate>, JsError> {
    let s: Option<String> = input(name, value)?;
    match s.filter(|s| !s.is_empty()) {
        Some(s) => Ok(Some(parse_date(&s).map_err(|e| JsError::new(&e))?)),
        None => Ok(None),
    }
}

fn date(s: &str) -> Result<NaiveDate, JsError> {
    parse_date(s).map_err(|e| JsError::new(&e))
}

/// `EditorValues::blank`.
#[wasm_bindgen(js_name = blankValues, unchecked_return_type = "import('../../bindings').EditorValues")]
pub fn blank_values(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Id")] project_id: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Id | null")] section_id: JsValue,
    date: String,
    #[wasm_bindgen(unchecked_param_type = "string[]")] labels: JsValue,
) -> Result<JsValue, JsError> {
    output(&EditorValues::blank(input("projectId", project_id)?, input("sectionId", section_id)?, date, input("labels", labels)?))
}

/// `EditorValues::from_task`.
#[wasm_bindgen(js_name = valuesFromTask, unchecked_return_type = "import('../../bindings').EditorValues")]
pub fn values_from_task(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task")] task: JsValue,
) -> Result<JsValue, JsError> {
    let task: Task = input("task", task)?;
    output(&EditorValues::from_task(&task))
}

/// `editor::editor_view`: everything the editor draws for its values.
#[wasm_bindgen(js_name = editorView, unchecked_return_type = "import('../../bindings').EditorView")]
pub fn editor_view(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] values: JsValue,
    quick_add: bool,
    #[wasm_bindgen(unchecked_param_type = "string | null")] keep_label: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Recurrence | null")] existing: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Project[]")] projects: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Section[]")] sections: JsValue,
    today: &str,
) -> Result<JsValue, JsError> {
    let v: EditorValues = input("values", values)?;
    let keep: Option<String> = input("keepLabel", keep_label)?;
    let existing: Option<Recurrence> = input("existing", existing)?;
    let projects: Vec<Project> = input("projects", projects)?;
    let sections: Vec<Section> = input("sections", sections)?;
    output(&editor::editor_view(&v, quick_add, keep.as_deref(), existing, &projects, &sections, date(today)?))
}

/// `editor::with_title`: the values after the title is retyped.
#[wasm_bindgen(js_name = withTitle, unchecked_return_type = "import('../../bindings').EditorValues")]
pub fn with_title(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] values: JsValue,
    text: String,
) -> Result<JsValue, JsError> {
    let v: EditorValues = input("values", values)?;
    output(&editor::with_title(&v, text))
}

/// `editor::apply_pick`.
#[wasm_bindgen(js_name = applyPick, unchecked_return_type = "import('../../bindings').EditorValues")]
pub fn apply_pick(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] values: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorPick")] pick: JsValue,
    quick_add: bool,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Project[]")] projects: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Section[]")] sections: JsValue,
    today: &str,
) -> Result<JsValue, JsError> {
    let mut v: EditorValues = input("values", values)?;
    let pick: Pick = input("pick", pick)?;
    let projects: Vec<Project> = input("projects", projects)?;
    let sections: Vec<Section> = input("sections", sections)?;
    editor::apply_pick(&mut v, &pick, quick_add, &projects, &sections, date(today)?);
    output(&v)
}

/// `editor::toggle_label`.
#[wasm_bindgen(js_name = toggleLabel, unchecked_return_type = "import('../../bindings').EditorValues")]
pub fn toggle_label(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] values: JsValue,
    name: &str,
    quick_add: bool,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Project[]")] projects: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Section[]")] sections: JsValue,
    today: &str,
) -> Result<JsValue, JsError> {
    let mut v: EditorValues = input("values", values)?;
    let projects: Vec<Project> = input("projects", projects)?;
    let sections: Vec<Section> = input("sections", sections)?;
    editor::toggle_label(&mut v, name, quick_add, &projects, &sections, date(today)?);
    output(&v)
}

/// `editor::unhighlight_at`: the values with the phrase ending at the caret
/// (UTF-16 units) turned plain; `null` when no highlight ends there.
#[wasm_bindgen(js_name = unhighlightAt, unchecked_return_type = "import('../../bindings').EditorValues | null")]
pub fn unhighlight_at(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] values: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Project[]")] projects: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Section[]")] sections: JsValue,
    today: &str,
    caret: usize,
) -> Result<JsValue, JsError> {
    let mut v: EditorValues = input("values", values)?;
    let projects: Vec<Project> = input("projects", projects)?;
    let sections: Vec<Section> = input("sections", sections)?;
    let changed = editor::unhighlight_at(&mut v, &projects, &sections, date(today)?, caret);
    output(&changed.then_some(v))
}

/// `editor::location_pick`.
#[wasm_bindgen(js_name = locationPick, unchecked_return_type = "import('../../bindings').EditorPick")]
pub fn location_pick(value: &str) -> Result<JsValue, JsError> {
    output(&editor::location_pick(value))
}

/// `editor::add_plan`: what adding the editor's task sends.
#[wasm_bindgen(js_name = addPlan, unchecked_return_type = "import('../../bindings').AddPlan")]
pub fn add_plan(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] values: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Project[]")] projects: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Section[]")] sections: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Label[]")] labels: JsValue,
    today: &str,
) -> Result<JsValue, JsError> {
    let v: EditorValues = input("values", values)?;
    let projects: Vec<Project> = input("projects", projects)?;
    let sections: Vec<Section> = input("sections", sections)?;
    let labels: Vec<Label> = input("labels", labels)?;
    output(&editor::add_plan(&v, &projects, &sections, &labels, date(today)?))
}

/// `editor::edit_plan`: what saving an edit sends.
#[wasm_bindgen(js_name = editPlan, unchecked_return_type = "import('../../bindings').EditPlan")]
pub fn edit_plan(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Id")] id: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] before: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').EditorValues")] after: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Recurrence | null")] existing: JsValue,
    today: &str,
) -> Result<JsValue, JsError> {
    let id: Id = input("id", id)?;
    let before: EditorValues = input("before", before)?;
    let after: EditorValues = input("after", after)?;
    let existing: Option<Recurrence> = input("existing", existing)?;
    output(&editor::edit_plan(&id, &before, &after, existing, date(today)?))
}

/// `editor::reminder_actions`.
#[wasm_bindgen(js_name = reminderActions, unchecked_return_type = "import('../../bindings').Action[]")]
pub fn reminder_actions(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Id")] task_id: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').ReminderDraft[]")] before: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').ReminderDraft[]")] after: JsValue,
) -> Result<JsValue, JsError> {
    let task_id: Id = input("taskId", task_id)?;
    let before: Vec<ReminderDraft> = input("before", before)?;
    let after: Vec<ReminderDraft> = input("after", after)?;
    output(&editor::reminder_actions(&task_id, &before, &after))
}

/// `ReminderDraft::label`: "30 min before", "Tuesday 9 AM".
#[wasm_bindgen(js_name = reminderDraftLabel)]
pub fn reminder_draft_label(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').ReminderDraft")] draft: JsValue,
    today: &str,
) -> Result<String, JsError> {
    let draft: ReminderDraft = input("draft", draft)?;
    Ok(draft.label(date(today)?))
}

/// `editor::reminder_presets`: (minutes before, text).
#[wasm_bindgen(js_name = reminderPresets, unchecked_return_type = "Array<[number, string]>")]
pub fn reminder_presets() -> Result<JsValue, JsError> {
    output(&editor::reminder_presets())
}

/// `editor::custom_reminder_at`: `YYYY-MM-DDTHH:MM`, `null` without a date and a time.
#[wasm_bindgen(js_name = customReminderAt, unchecked_return_type = "string | null")]
pub fn custom_reminder_at(text: &str, today: &str) -> Result<JsValue, JsError> {
    output(&editor::custom_reminder_at(text, date(today)?))
}

/// `editor::quick_dates`: (label, hint, `YYYY-MM-DD`).
#[wasm_bindgen(js_name = quickDates, unchecked_return_type = "Array<[string, string, string]>")]
pub fn quick_dates(today: &str) -> Result<JsValue, JsError> {
    let out: Vec<(&str, String, String)> =
        editor::quick_dates(date(today)?).into_iter().map(|(l, h, d)| (l, h, d.format("%Y-%m-%d").to_string())).collect();
    output(&out)
}

/// `calendar::month_view`. `week_start` is 1 = Monday … 7 = Sunday
/// (`Intl.Locale` week info's `firstDay`); 6 is Saturday, anything else Monday.
#[wasm_bindgen(js_name = monthView, unchecked_return_type = "import('../../bindings').MonthView")]
pub fn month_view(
    year: i32,
    month: u32,
    week_start: u8,
    today: &str,
    #[wasm_bindgen(unchecked_param_type = "string | null")] selected: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Recurrence | null")] rule: JsValue,
) -> Result<JsValue, JsError> {
    let start = match week_start {
        7 => Weekday::Sun,
        6 => Weekday::Sat,
        _ => Weekday::Mon,
    };
    let selected = opt_date("selected", selected)?;
    let rule: Option<Recurrence> = input("rule", rule)?;
    output(&calendar::month_view(year, month, start, date(today)?, selected, rule.as_ref()))
}

/// `calendar::shift_month`: `[year, month]`.
#[wasm_bindgen(js_name = shiftMonth, unchecked_return_type = "[number, number]")]
pub fn shift_month(year: i32, month: u32, delta: i32) -> Result<JsValue, JsError> {
    output(&calendar::shift_month(year, month, delta))
}

/// `calendar::year_choices`.
#[wasm_bindgen(js_name = yearChoices, unchecked_return_type = "number[]")]
pub fn year_choices(today: &str, shown: i32) -> Result<JsValue, JsError> {
    output(&calendar::year_choices(date(today)?, shown))
}

/// `calendar::MONTHS`.
#[wasm_bindgen(js_name = monthNames, unchecked_return_type = "string[]")]
pub fn month_names() -> Result<JsValue, JsError> {
    output(&calendar::MONTHS)
}

/// A typed time ("5pm", "17:30") as `HH:MM`; `null` when it isn't one.
#[wasm_bindgen(js_name = typedTime, unchecked_return_type = "string | null")]
pub fn typed_time(text: &str) -> Result<JsValue, JsError> {
    output(&quickadd::parse_time(text).map(|t| t.format("%H:%M").to_string()))
}

/// A `<input type="time">` value as `HH:MM`; `null` when cleared,
/// `undefined` when it isn't the input's format.
#[wasm_bindgen(js_name = nativeTime, unchecked_return_type = "string | null | undefined")]
pub fn native_time_js(value: &str) -> Result<JsValue, JsError> {
    match native_time(value) {
        Some(t) => output(&t),
        None => Ok(JsValue::UNDEFINED),
    }
}

#[cfg(test)]
mod tests {
    use super::{native_time, parse_date, parse_time};
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

    #[test]
    fn native_time_reads_the_time_inputs_value() {
        assert_eq!(native_time("17:30"), Some(Some("17:30".to_string())));
        assert_eq!(native_time("07:05:00"), Some(Some("07:05".to_string())), "with seconds");
        assert_eq!(native_time(""), Some(None), "cleared");
        assert_eq!(native_time("5pm"), None, "not the input's format");
    }
}
