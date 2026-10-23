//! The native Liquid Glass card on iPhone (`tauri-plugin-glass`). Swift draws
//! it and reports what the user does; this module decides what it shows
//! (`card_state`) and what each action changes (`apply_event`), so typing,
//! chips and saving behave exactly like the web editor.


use chrono::{Datelike, NaiveDate, NaiveTime};
use keel_core::calendar::{self, MonthView};
use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::model::{Label, Project, Section, Task};
use keel_core::recurrence::Recurrence;
use keel_core::views;
use keel_core::quickadd::{strip_spans, QuickAdd, SpanKind};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::editor::{
    add_from_editor, labels_text, recurrence_for_at, unhighlight, with_title, location_text, parse_title_at, priority_text, repeat_chip_text, repeat_options,
    repeat_text, save_edit, EditorValues,
};
use crate::pickers::{date_chip, deadline_text, offset_text, quick_dates, reminders_text, ReminderDraft, REMINDER_PRESETS};

/// Adding parses the title as quick add; editing never does (as on the web).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CardMode {
    Add,
    Edit,
}

/// The chips, in the order the card shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChipId {
    Date,
    Priority,
    Labels,
    Reminders,
    Deadline,
    Repeat,
    Location,
}

/// A highlighted phrase in the title, in UTF-16 units (Swift's `NSRange`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GlassSpan {
    pub start: usize,
    pub end: usize,
    /// date | time | repeat | priority-1..3 | project | section | label | deadline
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChipState {
    pub id: ChipId,
    pub label: String,
    /// Has a value (drawn tinted).
    pub set: bool,
    /// A color other than Keel green: "p1".."p3" for the priority flags,
    /// "due-overdue" / "due-today" / "due-tomorrow" / "due-week" /
    /// "due-later" for the date (Keel's due colors).
    pub tone: Option<String>,
    /// The date repeats (drawn with a ⟳).
    pub repeats: bool,
}

/// What the pickers need to show the current values and the choices.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerData {
    pub date: String,
    pub time: String,
    pub priority: String,
    pub repeat: String,
    /// (value, label), the same options as the web Repeat menu.
    pub repeat_options: Vec<(String, String)>,
    pub project_id: Id,
    pub section_id: Option<Id>,
    pub projects: Vec<PickProject>,
    pub labels: Vec<String>,
    pub all_labels: Vec<String>,
    pub deadline: String,
    pub reminders: Vec<ReminderDraft>,
    /// What each reminder shows, in order.
    pub reminder_labels: Vec<String>,
    /// (minutes before, label)
    pub reminder_presets: Vec<(i64, String)>,
    /// Relative reminders need a due date.
    pub has_due: bool,
    pub quick_dates: Vec<QuickDate>,
    /// The calendar month shown (core's model; the card only draws it).
    pub calendar: MonthView,
    /// The month / year picker's choices.
    pub years: Vec<i32>,
    pub months: Vec<&'static str>,
    /// The chosen date's due color (fills the selected day).
    pub selected_tone: Option<String>,
}

/// A date-picker shortcut ("Tomorrow", "Next week").
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QuickDate {
    pub label: String,
    pub hint: String,
    /// `YYYY-MM-DD`
    pub date: String,
    /// The date's due color, like the chip.
    pub tone: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickProject {
    pub id: Id,
    pub name: String,
    pub is_inbox: bool,
    /// (id, name)
    pub sections: Vec<(Id, String)>,
}

/// Everything the Swift card shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardState {
    pub mode: CardMode,
    /// The text this state was computed for; Swift drops a state whose title
    /// no longer matches what's in the field (typing got ahead of it).
    pub title: String,
    pub description: String,
    pub spans: Vec<GlassSpan>,
    pub chips: Vec<ChipState>,
    pub can_submit: bool,
    pub picker: PickerData,
}

/// What the user did in the card, as Swift reports it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GlassEvent {
    Title { text: String },
    Description { text: String },
    Pick { pick: Pick },
    /// The calendar's arrows (`delta` months) and its month / year picker.
    CalendarShift { delta: i32 },
    CalendarMonth { year: i32, month: u32 },
    /// Backspace right after a highlight: that phrase becomes plain text.
    /// `at` is where the highlight ends, in UTF-16 units.
    Unhighlight { at: usize },
    Submit,
    Cancel,
}

/// A value chosen in a picker, in [`EditorValues`]' formats.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Pick {
    Date { date: String, time: String },
    Priority { value: String },
    Labels { names: Vec<String> },
    Reminders { drafts: Vec<ReminderDraft> },
    Deadline { date: String },
    Repeat { value: String },
    Location { project_id: Id, section_id: Option<Id> },
}

/// How the card is refreshed after an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refresh {
    /// Highlights and chips only; the field keeps what's typed.
    Update,
    /// The title changed under the field (a pick removed a typed phrase).
    Replace,
}

/// A pick that rewrote the title must replace the field's text: an update
/// would be dropped as stale, since the field still shows the old title.
pub fn refresh_for(ev: &GlassEvent, before: &str, after: &str) -> Refresh {
    match ev {
        GlassEvent::Title { .. } => Refresh::Update,
        _ if before != after => Refresh::Replace,
        _ => Refresh::Update,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Changed,
    Submit,
    Cancel,
}

/// The lists and date a card is shown against.
#[derive(Clone, Debug, PartialEq)]
pub struct CardContext {
    pub projects: Vec<Project>,
    pub sections: Vec<Section>,
    pub labels: Vec<Label>,
    /// Text for the "keep current rule" repeat option, when the rule is custom.
    pub keep_label: Option<String>,
    /// The edited task's recurrence (what "keep" means).
    pub existing: Option<Recurrence>,
    pub today: NaiveDate,
    /// The device's first day of the week (the calendar's first column).
    pub week_start: chrono::Weekday,
    /// The month the calendar shows, once navigated; None follows the
    /// chosen date (or today).
    pub calendar: Option<(i32, u32)>,
}

/// Keel's due color for a date (the task list's `.due.*` classes).
fn due_tone(bucket: views::DueBucket) -> &'static str {
    match bucket {
        views::DueBucket::Overdue => "due-overdue",
        views::DueBucket::Today => "due-today",
        views::DueBucket::Tomorrow => "due-tomorrow",
        views::DueBucket::ThisWeek => "due-week",
        views::DueBucket::Later => "due-later",
    }
}

fn kind_name(k: SpanKind) -> &'static str {
    match k {
        SpanKind::Date => "date",
        SpanKind::Time => "time",
        SpanKind::Repeat => "repeat",
        SpanKind::Priority => "priority",
        SpanKind::Project => "project",
        SpanKind::Section => "section",
        SpanKind::Label => "label",
        SpanKind::Deadline => "deadline",
    }
}

fn utf16_at(s: &str, byte: usize) -> usize {
    s[..byte].encode_utf16().count()
}

/// The byte offset of UTF-16 offset `at` in `s` (clamped to the end).
pub(crate) fn byte_at_utf16(s: &str, at: usize) -> usize {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        if units >= at {
            return i;
        }
        units += c.len_utf16();
    }
    s.len()
}

/// The typed phrases a pick replaces.
fn kinds_for(pick: &Pick) -> &'static [SpanKind] {
    match pick {
        Pick::Date { .. } => &[SpanKind::Date, SpanKind::Time],
        Pick::Priority { .. } => &[SpanKind::Priority],
        Pick::Labels { .. } => &[SpanKind::Label],
        Pick::Reminders { .. } => &[],
        Pick::Deadline { .. } => &[SpanKind::Deadline],
        Pick::Repeat { .. } => &[SpanKind::Repeat],
        Pick::Location { .. } => &[SpanKind::Project, SpanKind::Section],
    }
}

/// The values with what the title says applied on top, the same way
/// `editor::new_task_from` merges them when the task is saved.
fn merged(v: &EditorValues, qa: &QuickAdd) -> EditorValues {
    let mut m = v.clone();
    if let Some(d) = qa.date {
        m.date = d.format("%Y-%m-%d").to_string();
        m.time = qa.time.map(|t| t.format("%H:%M").to_string()).unwrap_or_default();
    }
    if let Some(p) = qa.priority {
        m.priority = (5 - p.to_api()).to_string();
    }
    for l in &qa.labels {
        if !m.labels.iter().any(|x| x.eq_ignore_ascii_case(l)) {
            m.labels.push(l.clone());
        }
    }
    if let Some(p) = &qa.project_id {
        m.project_id = p.clone();
        m.section_id = qa.section_id.clone();
    }
    if let Some(d) = qa.deadline {
        m.deadline = d.format("%Y-%m-%d").to_string();
    }
    m
}

pub fn card_state(v: &EditorValues, mode: CardMode, ctx: &CardContext) -> CardState {
    let qa = (mode == CardMode::Add).then(|| parse_title_at(v, &ctx.projects, &ctx.sections, ctx.today));
    let shown = qa.as_ref().map(|qa| merged(v, qa)).unwrap_or_else(|| v.clone());
    let spans = qa
        .as_ref()
        .map(|qa| {
            qa.spans
                .iter()
                .map(|s| GlassSpan {
                    start: utf16_at(&v.content, s.start),
                    end: utf16_at(&v.content, s.end),
                    kind: match (s.kind, qa.priority) {
                        (SpanKind::Priority, Some(p)) => format!("priority-{}", 5 - p.to_api()),
                        (k, _) => kind_name(k).to_string(),
                    },
                })
                .collect()
        })
        .unwrap_or_default();

    let date = NaiveDate::parse_from_str(&shown.date, "%Y-%m-%d").ok();
    let time = NaiveTime::parse_from_str(&shown.time, "%H:%M").ok();
    let deadline = NaiveDate::parse_from_str(&shown.deadline, "%Y-%m-%d").ok();
    let opts = repeat_options(&shown.date, ctx.keep_label.as_deref(), ctx.today);
    let typed_rule = qa.as_ref().and_then(|qa| qa.recurrence.as_ref());
    let repeat_label = typed_rule.map(repeat_text).unwrap_or_else(|| repeat_chip_text(&shown.repeat, &opts));
    let at_inbox = ctx.projects.iter().any(|p| p.is_inbox && p.id == shown.project_id) && shown.section_id.is_none();

    // The rule the task will repeat by: typed, or picked from the menu.
    let rule = typed_rule.cloned().or_else(|| recurrence_for_at(&shown.repeat, &shown.date, ctx.existing.clone(), ctx.today));
    let date_tone = date.map(|d| due_tone(views::due_bucket(d, ctx.today)).to_string());
    let chip = |id, label: String, set| ChipState { id, label, set, tone: None, repeats: false };
    let priority_tone = matches!(shown.priority.as_str(), "1" | "2" | "3").then(|| format!("p{}", shown.priority));
    let chips = vec![
        ChipState {
            tone: date_tone,
            repeats: rule.is_some(),
            ..chip(ChipId::Date, date_chip(date, time, ctx.today).0, date.is_some() || time.is_some())
        },
        ChipState { tone: priority_tone, ..chip(ChipId::Priority, priority_text(&shown.priority), shown.priority != "4") },
        chip(ChipId::Labels, labels_text(&shown.labels), !shown.labels.is_empty()),
        chip(ChipId::Reminders, reminders_text(&shown.reminders, ctx.today), !shown.reminders.is_empty()),
        chip(ChipId::Deadline, deadline_text(deadline, ctx.today), deadline.is_some()),
        chip(ChipId::Repeat, repeat_label, typed_rule.is_some() || shown.repeat != "none"),
        chip(
            ChipId::Location,
            location_text(&shown.project_id, shown.section_id.as_ref(), &ctx.projects, &ctx.sections),
            !at_inbox,
        ),
    ];

    let picker = PickerData {
        date: shown.date.clone(),
        time: shown.time.clone(),
        priority: shown.priority.clone(),
        repeat: shown.repeat.clone(),
        repeat_options: opts.into_iter().map(|(value, label)| (value.to_string(), label)).collect(),
        project_id: shown.project_id.clone(),
        section_id: shown.section_id.clone(),
        projects: ctx
            .projects
            .iter()
            .map(|p| PickProject {
                id: p.id.clone(),
                name: p.name.clone(),
                is_inbox: p.is_inbox,
                sections: ctx
                    .sections
                    .iter()
                    .filter(|s| s.project_id == p.id)
                    .map(|s| (s.id.clone(), s.name.clone()))
                    .collect(),
            })
            .collect(),
        labels: shown.labels.clone(),
        all_labels: ctx.labels.iter().map(|l| l.name.clone()).collect(),
        deadline: shown.deadline.clone(),
        reminders: shown.reminders.clone(),
        reminder_labels: shown.reminders.iter().map(|r| r.label(ctx.today)).collect(),
        reminder_presets: REMINDER_PRESETS.iter().map(|&m| (m, offset_text(m))).collect(),
        has_due: date.is_some() || time.is_some(),
        quick_dates: quick_dates(ctx.today)
            .into_iter()
            .map(|(label, hint, d)| QuickDate {
                label: label.to_string(),
                hint,
                date: d.format("%Y-%m-%d").to_string(),
                tone: due_tone(views::due_bucket(d, ctx.today)).to_string(),
            })
            .collect(),
        calendar: {
            let (y, m) = ctx.calendar.unwrap_or_else(|| {
                let d = date.unwrap_or(ctx.today);
                (d.year(), d.month())
            });
            calendar::month_view(y, m, ctx.week_start, ctx.today, date, rule.as_ref())
        },
        years: calendar::year_choices(ctx.today),
        months: calendar::MONTHS.to_vec(),
        selected_tone: date.map(|d| due_tone(views::due_bucket(d, ctx.today)).to_string()),
    };

    CardState {
        mode,
        title: v.content.clone(),
        description: v.description.clone(),
        spans,
        chips,
        can_submit: match &qa {
            Some(qa) => !qa.content.trim().is_empty(),
            None => !v.content.trim().is_empty(),
        },
        picker,
    }
}

/// Calendar navigation lives in the context, not the task's values: the
/// arrows and the month / year picker move it (true: the card needs a
/// refresh); picking a date resets it to follow that date.
pub fn apply_calendar(ctx: &mut CardContext, ev: &GlassEvent, v: &EditorValues) -> bool {
    let shown = || {
        ctx.calendar.unwrap_or_else(|| {
            let d = NaiveDate::parse_from_str(&v.date, "%Y-%m-%d").unwrap_or(ctx.today);
            (d.year(), d.month())
        })
    };
    match ev {
        GlassEvent::CalendarShift { delta } => {
            let (y, m) = shown();
            ctx.calendar = Some(calendar::shift_month(y, m, *delta));
            true
        }
        GlassEvent::CalendarMonth { year, month } => {
            ctx.calendar = Some((*year, (*month).clamp(1, 12)));
            true
        }
        GlassEvent::Pick { pick: Pick::Date { .. } } => {
            ctx.calendar = None;
            false
        }
        _ => false,
    }
}

/// Applies one action from the card to the values.
pub fn apply_event(v: &mut EditorValues, ev: &GlassEvent, mode: CardMode, ctx: &CardContext) -> Outcome {
    match ev {
        GlassEvent::Title { text } => *v = with_title(v, text.clone()),
        GlassEvent::Unhighlight { at } => {
            let end = byte_at_utf16(&v.content, *at);
            if mode != CardMode::Add || !unhighlight(v, &ctx.projects, &ctx.sections, ctx.today, end) {
                return Outcome::Changed;
            }
        }
        GlassEvent::Description { text } => v.description = text.clone(),
        GlassEvent::Submit => return Outcome::Submit,
        GlassEvent::CalendarShift { .. } | GlassEvent::CalendarMonth { .. } => {}
        GlassEvent::Cancel => return Outcome::Cancel,
        GlassEvent::Pick { pick } => {
            if mode == CardMode::Add {
                let qa = parse_title_at(v, &ctx.projects, &ctx.sections, ctx.today);
                v.content = strip_spans(&v.content, &qa.spans, kinds_for(pick));
            }
            match pick {
                Pick::Date { date, time } => (v.date, v.time) = (date.clone(), time.clone()),
                Pick::Priority { value } => v.priority = value.clone(),
                Pick::Labels { names } => v.labels = names.clone(),
                Pick::Reminders { drafts } => v.reminders = drafts.clone(),
                Pick::Deadline { date } => v.deadline = date.clone(),
                Pick::Repeat { value } => v.repeat = value.clone(),
                Pick::Location { project_id, section_id } => {
                    (v.project_id, v.section_id) = (project_id.clone(), section_id.clone())
                }
            }
        }
    }
    Outcome::Changed
}


/// What the values become after a submit: adding resets the card for the
/// next task (it stays open); editing closes it.
pub fn after_submit(mode: CardMode, initial: &EditorValues) -> Option<EditorValues> {
    (mode == CardMode::Add).then(|| initial.clone())
}

/// One open card: what it was opened with and what it holds now.
#[derive(Clone)]
struct CardSession {
    mode: CardMode,
    initial: EditorValues,
    values: EditorValues,
    /// The task being edited.
    task_id: Option<Id>,
    /// Its recurrence rule, kept unless Repeat changes.
    existing: Option<Recurrence>,
    ctx: CardContext,
    reload: Signal<u32>,
}

/// Forwards the plugin's `card` events (a `GlassEvent` as JSON) to Rust.
const EVENTS_JS: &str = r#"
await window.__TAURI__.core.addPluginListener('glass', 'card', (e) => dioxus.send(e.json));
await new Promise(() => {});
"#;

/// Forwards which floating button was tapped.
const BUTTONS_JS: &str = r#"
document.body.classList.add('native-glass');
await window.__TAURI__.core.addPluginListener('glass', 'button', (e) => dioxus.send(e.name));
await new Promise(() => {});
"#;

/// Opens the composer the way the view's own "Add task" row would, with
/// that row's defaults.
const ADD_JS: &str = "document.querySelector('.view-add')?.click();";

/// What a floating glass button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    AddTask,
    OpenMenu,
    Search,
    /// The drawer's theme switcher and settings (taps on the web buttons).
    Theme,
    Settings,
}

/// What a row's long-press menu can do (the desktop's hover actions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskAction {
    SubTask,
    Streak,
    Delete,
    /// Sidebar rows: rename a project or label, edit a filter.
    Rename,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ActionItem {
    pub id: TaskAction,
    pub label: String,
    pub destructive: bool,
}

/// The menu a long press on a task opens: the same actions as the row's
/// hover buttons on desktop.
pub fn task_actions(recurring: bool) -> Vec<ActionItem> {
    let item = |id, label: &str, destructive| ActionItem { id, label: label.to_string(), destructive };
    let mut items = vec![item(TaskAction::SubTask, "Add sub-task", false)];
    if recurring {
        items.push(item(TaskAction::Streak, "Streak", false));
    }
    items.push(item(TaskAction::Delete, "Delete task", true));
    items
}

/// The menu for a sidebar row (project, label or filter); Delete still
/// goes through the row's inline confirm.
pub fn nav_actions(rename_label: &str) -> Vec<ActionItem> {
    vec![
        ActionItem { id: TaskAction::Rename, label: rename_label.to_string(), destructive: false },
        ActionItem { id: TaskAction::Delete, label: "Delete".to_string(), destructive: true },
    ]
}

/// The menu choice Swift reports. `target` is the row's menu id: a task id,
/// or `project:<id>` / `label:<id>` / `filter:<id>` for sidebar rows.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChosenAction {
    pub target: String,
    pub action: TaskAction,
}

/// Gives a row a long-press menu on iPhone: when the row tagged
/// `data-menu-id={menu_id}` is long-pressed, Swift shows `items` titled
/// `title`, and the choice comes back to `on_choose`.
pub fn use_row_menu(
    menu_id: String,
    title: String,
    items: Vec<ActionItem>,
    mut on_choose: impl FnMut(TaskAction) + 'static,
) {
    let glass = use_glass();
    let id = menu_id.clone();
    use_effect(move || {
        let mut pressed = glass.pressed;
        if pressed().as_deref() == Some(id.as_str()) {
            pressed.set(None);
            let (id, title, items) = (id.clone(), title.clone(), items.clone());
            spawn(async move { api::glass_actions(&id, &title, items).await });
        }
    });
    use_effect(move || {
        let mut chosen = glass.chosen;
        let Some(c) = chosen().filter(|c| c.target == menu_id) else { return };
        chosen.set(None);
        on_choose(c.action);
    });
}

/// A native text prompt (a glass alert with one or two fields). On iPhone
/// it replaces the web forms that focus a field from code, which iOS
/// answers with no keyboard.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Prompt {
    /// Echoed in the answer, so the right caller handles it.
    pub id: String,
    pub title: String,
    /// Shown under the title (e.g. why a filter query was rejected).
    pub message: String,
    pub fields: Vec<PromptField>,
    /// The confirm button's label.
    pub confirm: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PromptField {
    pub placeholder: String,
    pub value: String,
}

impl Prompt {
    pub fn new(id: &str, title: &str, confirm: &str) -> Self {
        Prompt { id: id.into(), title: title.into(), message: String::new(), fields: Vec::new(), confirm: confirm.into() }
    }

    pub fn field(mut self, placeholder: &str, value: &str) -> Self {
        self.fields.push(PromptField { placeholder: placeholder.into(), value: value.into() });
        self
    }

    pub fn message(mut self, message: &str) -> Self {
        self.message = message.into();
        self
    }
}

/// What came back: the fields' values, or None when cancelled.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PromptAnswer {
    pub id: String,
    pub values: Option<Vec<String>>,
}

/// Field `i` of an answer, trimmed; None when missing or blank.
pub fn answer_text(values: &[String], i: usize) -> Option<String> {
    values.get(i).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Calls `on_answer` with the values when the prompt `id` is answered
/// (not when it's cancelled).
pub fn use_prompt_answer(id: String, mut on_answer: impl FnMut(Vec<String>) + 'static) {
    let glass = use_glass();
    use_effect(move || {
        let mut answer = glass.answer;
        let Some(a) = answer().filter(|a| a.id == id) else { return };
        answer.set(None);
        if let Some(values) = a.values {
            on_answer(values);
        }
    });
}

/// Forwards long presses on task rows (the task id), the action menu's
/// choices (a `ChosenAction` as JSON) and prompt answers to Rust.
const PRESS_JS: &str = r#"
document.addEventListener('keel-long-press', (e) => dioxus.send({ press: e.detail }));
await window.__TAURI__.core.addPluginListener('glass', 'action', (e) => dioxus.send({ chosen: e.json }));
await window.__TAURI__.core.addPluginListener('glass', 'prompt', (e) => dioxus.send({ answer: e.json }));
await new Promise(() => {});
"#;

#[derive(Deserialize)]
struct PressMsg {
    press: Option<String>,
    chosen: Option<String>,
    answer: Option<String>,
}

/// Which floating buttons show (`plugin:glass|buttons`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ButtonsState {
    pub visible: bool,
    /// The + (views with an "Add task" row; not Search).
    pub add: bool,
    /// In Search, the search button closes it instead.
    pub searching: bool,
    /// Glass stand-ins for the drawer's theme and settings buttons, while
    /// the drawer is open.
    pub drawer: Vec<DrawerButton>,
    /// Keel's theme ("system" / "light" / "dark"): the native views follow
    /// it rather than the phone's setting.
    pub theme: String,
}

pub fn floating_buttons(searching: bool, drawer_open: bool) -> ButtonsState {
    ButtonsState { visible: !drawer_open, add: !searching, searching, drawer: Vec::new(), theme: "system".to_string() }
}

/// Where the page drew one of the drawer's header buttons (CSS px, which
/// are points on iOS), as `DRAWER_JS` measures it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WebRect {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// A native glass button drawn over the matching web one.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DrawerButton {
    pub id: String,
    /// SF Symbol name.
    pub symbol: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The drawer's header buttons as glass: the theme switcher shows the
/// current preference's symbol, settings a gear.
pub fn drawer_buttons(rects: &[WebRect], theme: &str) -> Vec<DrawerButton> {
    rects
        .iter()
        .map(|r| {
            let symbol = match (r.id.as_str(), theme) {
                ("theme", "light") => "sun.max",
                ("theme", "dark") => "moon",
                ("theme", _) => "desktopcomputer",
                _ => "gearshape",
            };
            DrawerButton { id: r.id.clone(), symbol: symbol.to_string(), x: r.x, y: r.y, w: r.w, h: r.h }
        })
        .collect()
}

/// Measures the drawer's theme and settings buttons: once when asked (`go`
/// is sent after the drawer opens), and again whenever the drawer itself
/// scrolls. (An earlier version watched every DOM change and fed back into
/// rendering when the settings dialog opened, freezing the page.)
const MEASURE_FN: &str = r#"
const measure = () => [['theme', '.brand .theme-toggle'], ['settings', '.brand .settings-btn']].map(([id, sel]) => {
  const r = document.querySelector(sel)?.getBoundingClientRect();
  return r && { id, x: r.x, y: r.y, w: r.width, h: r.height };
}).filter(Boolean);
"#;

/// Re-measures whenever the drawer itself scrolls.
const DRAWER_SCROLL_JS: &str = r#"
let timer = null;
document.addEventListener('scroll', (ev) => {
  if (!ev.target.classList?.contains('sidebar')) return;
  clearTimeout(timer);
  timer = setTimeout(() => dioxus.send(measure()), 16);
}, true);
await new Promise(() => {});
"#;

/// One reading, once the drawer has finished sliding in.
const DRAWER_ONCE_JS: &str = r#"
await new Promise((r) => setTimeout(r, 320));
dioxus.send(measure());
"#;

/// Taps the web button a glass stand-in covers.
fn click_js(selector: &str) -> String {
    format!("document.querySelector('{selector}')?.click();")
}

pub fn button_action(name: &str) -> Option<ButtonAction> {
    match name {
        "fab" => Some(ButtonAction::AddTask),
        "menu" => Some(ButtonAction::OpenMenu),
        "search" => Some(ButtonAction::Search),
        "theme" => Some(ButtonAction::Theme),
        "settings" => Some(ButtonAction::Settings),
        _ => None,
    }
}

/// The native card on iPhone; elsewhere `native` stays false and callers
/// keep the web editor.
#[derive(Clone, Copy)]
pub struct Glass {
    pub native: Signal<bool>,
    session: Signal<Option<CardSession>>,
    /// The menu id of the row that was long-pressed (its row opens the menu).
    pub pressed: Signal<Option<String>>,
    /// The menu choice, for the row it belongs to.
    pub chosen: Signal<Option<ChosenAction>>,
    /// Where the drawer's header buttons sit (empty while it's closed).
    pub drawer_rects: Signal<Vec<WebRect>>,
    /// The settings dialog is open (the drawer's glass buttons step aside).
    pub settings_open: Signal<bool>,
    /// The latest prompt answer, for the caller whose id it carries.
    pub answer: Signal<Option<PromptAnswer>>,
}

/// Provides [`Glass`] (call once, in `App`): asks the plugin whether it's
/// there and, if so, starts handling its events. `on_button` gets the
/// floating buttons' taps, except Add, which is handled here.
pub fn provide(mut on_button: impl FnMut(ButtonAction) + 'static) -> Glass {
    let glass = use_context_provider(|| Glass {
        native: Signal::new(false),
        session: Signal::new(None),
        pressed: Signal::new(None),
        chosen: Signal::new(None),
        drawer_rects: Signal::new(Vec::new()),
        settings_open: Signal::new(false),
        answer: Signal::new(None),
    });
    use_hook(move || {
        spawn(async move {
            if !api::glass_available().await {
                return;
            }
            let mut native = glass.native;
            native.set(true);
            spawn(async move {
                let (mut pressed, mut chosen, mut answer) = (glass.pressed, glass.chosen, glass.answer);
                let mut presses = document::eval(PRESS_JS);
                while let Ok(msg) = presses.recv::<PressMsg>().await {
                    if let Some(id) = msg.press {
                        pressed.set(Some(id));
                    }
                    if let Some(c) = msg.chosen.and_then(|j| serde_json::from_str::<ChosenAction>(&j).ok()) {
                        chosen.set(Some(c));
                    }
                    if let Some(a) = msg.answer.and_then(|j| serde_json::from_str::<PromptAnswer>(&j).ok()) {
                        answer.set(Some(a));
                    }
                }
            });
            spawn(async move {
                let mut rects = glass.drawer_rects;
                let mut spots = document::eval(&format!("{MEASURE_FN}{DRAWER_SCROLL_JS}"));
                while let Ok(r) = spots.recv::<Vec<WebRect>>().await {
                    rects.set(r);
                }
            });
            spawn(async move {
                let mut taps = document::eval(BUTTONS_JS);
                while let Ok(name) = taps.recv::<String>().await {
                    match button_action(&name) {
                        Some(ButtonAction::AddTask) => {
                            let _ = document::eval(ADD_JS);
                        }
                        Some(ButtonAction::Theme) => {
                            let _ = document::eval(&click_js(".brand .theme-toggle"));
                        }
                        Some(ButtonAction::Settings) => {
                            let _ = document::eval(&click_js(".brand .settings-btn"));
                        }
                        Some(action) => on_button(action),
                        None => {}
                    }
                }
            });
            let mut events = document::eval(EVENTS_JS);
            while let Ok(json) = events.recv::<String>().await {
                if let Ok(ev) = serde_json::from_str::<GlassEvent>(&json) {
                    glass.handle(ev).await;
                }
            }
        });
    });
    glass
}

pub fn use_glass() -> Glass {
    use_context()
}

impl Glass {
    /// Opens the composer with `defaults` (project, section, date, parent).
    pub fn add(self, defaults: EditorValues, projects: Vec<Project>, sections: Vec<Section>, labels: Vec<Label>, reload: Signal<u32>) {
        let ctx = CardContext {
            projects,
            sections,
            labels,
            keep_label: None,
            existing: None,
            today: crate::today(),
            week_start: crate::pickers::week_start(),
            calendar: None,
        };
        self.open(CardSession {
            mode: CardMode::Add,
            initial: defaults.clone(),
            values: defaults,
            task_id: None,
            existing: None,
            ctx,
            reload,
        });
    }

    /// Opens the editor on `task`.
    pub fn edit(
        self,
        task: &Task,
        reminders: Vec<ReminderDraft>,
        projects: Vec<Project>,
        sections: Vec<Section>,
        labels: Vec<Label>,
        reload: Signal<u32>,
    ) {
        let mut before = EditorValues::from_task(task);
        before.reminders = reminders;
        let existing = task.due.as_ref().and_then(|d| d.recurrence.clone());
        let keep_label = (before.repeat == "keep").then(|| existing.as_ref().map(repeat_text).unwrap_or_default());
        let ctx = CardContext {
            projects,
            sections,
            labels,
            keep_label,
            existing: existing.clone(),
            today: crate::today(),
            week_start: crate::pickers::week_start(),
            calendar: None,
        };
        self.open(CardSession {
            mode: CardMode::Edit,
            initial: before.clone(),
            values: before,
            task_id: Some(task.id.clone()),
            existing,
            ctx,
            reload,
        });
    }

    /// Shows a native prompt; the answer arrives through `use_prompt_answer`.
    pub fn ask(self, prompt: Prompt) {
        spawn(async move { api::glass_prompt(&prompt).await });
    }

    /// Measures the drawer's header buttons once it has slid in (iPhone).
    pub fn measure_drawer(self) {
        if !*self.native.peek() {
            return;
        }
        let mut rects = self.drawer_rects;
        spawn(async move {
            let mut once = document::eval(&format!("{MEASURE_FN}{DRAWER_ONCE_JS}"));
            if let Ok(r) = once.recv::<Vec<WebRect>>().await {
                rects.set(r);
            }
        });
    }

    fn open(mut self, s: CardSession) {
        let state = card_state(&s.values, s.mode, &s.ctx);
        self.session.set(Some(s));
        spawn(async move { api::glass_show(&state).await });
    }

    async fn handle(mut self, ev: GlassEvent) {
        let Some(mut s) = self.session.peek().clone() else { return };
        if apply_calendar(&mut s.ctx, &ev, &s.values) {
            let state = card_state(&s.values, s.mode, &s.ctx);
            self.session.set(Some(s));
            api::glass_update(&state).await;
            return;
        }
        let before = s.values.content.clone();
        match apply_event(&mut s.values, &ev, s.mode, &s.ctx) {
            Outcome::Changed => {
                let state = card_state(&s.values, s.mode, &s.ctx);
                let refresh = refresh_for(&ev, &before, &s.values.content);
                self.session.set(Some(s));
                match refresh {
                    Refresh::Update => api::glass_update(&state).await,
                    Refresh::Replace => api::glass_show(&state).await,
                }
            }
            Outcome::Cancel => {
                self.session.set(None);
                api::glass_hide().await;
            }
            Outcome::Submit => {
                if !card_state(&s.values, s.mode, &s.ctx).can_submit {
                    return;
                }
                let mut reload = s.reload;
                match after_submit(s.mode, &s.initial) {
                    Some(next) => {
                        let saved = std::mem::replace(&mut s.values, next);
                        let state = card_state(&s.values, s.mode, &s.ctx);
                        let (p, sec, l) = (s.ctx.projects.clone(), s.ctx.sections.clone(), s.ctx.labels.clone());
                        self.session.set(Some(s));
                        api::glass_show(&state).await;
                        add_from_editor(saved, p, sec, l).await;
                    }
                    None => {
                        self.session.set(None);
                        api::glass_hide().await;
                        if let Some(id) = s.task_id.clone() {
                            save_edit(id, s.initial.clone(), s.values.clone(), s.existing.clone()).await;
                        }
                    }
                }
                reload += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::EditorValues;
    use chrono::NaiveDate;
    use keel_core::ids::Id;
    use keel_core::model::{Project, Task};
    use keel_core::views;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 19).unwrap() // Monday
    }

    fn ctx() -> CardContext {
        let mut inbox = Project::inbox();
        inbox.id = Id::from("P-IN");
        let mut work = Project::new("Work");
        work.id = Id::from("P-WORK");
        CardContext {
            projects: vec![inbox, work],
            sections: vec![],
            labels: vec![],
            keep_label: None,
            existing: None,
            today: today(),
            week_start: chrono::Weekday::Sun,
            calendar: None,
        }
    }

    fn blank(content: &str) -> EditorValues {
        let mut v = EditorValues::blank(Id::from("P-IN"), None, String::new(), Vec::new());
        v.content = content.to_string();
        v
    }

    fn chip(state: &CardState, id: ChipId) -> &ChipState {
        state.chips.iter().find(|c| c.id == id).unwrap()
    }

    #[test]
    fn spans_are_utf16() {
        let s = card_state(&blank("café 🎉 tomorrow"), CardMode::Add, &ctx());
        assert_eq!(s.spans, vec![GlassSpan { start: 8, end: 16, kind: "date".to_string() }]);
    }

    #[test]
    fn chips_come_in_the_fixed_order() {
        let s = card_state(&blank(""), CardMode::Add, &ctx());
        let ids: Vec<ChipId> = s.chips.iter().map(|c| c.id).collect();
        use ChipId::*;
        assert_eq!(ids, vec![Date, Priority, Labels, Reminders, Deadline, Repeat, Location]);
        assert_eq!(chip(&s, Date).label, "Date");
        assert!(!chip(&s, Date).set);
    }

    #[test]
    fn add_mode_chip_shows_typed_date() {
        let s = card_state(&blank("call mom tomorrow"), CardMode::Add, &ctx());
        let tomorrow = today().succ_opt().unwrap();
        assert!(chip(&s, ChipId::Date).set);
        assert_eq!(chip(&s, ChipId::Date).label, views::due_label(tomorrow, today()));
    }

    #[test]
    fn edit_mode_has_no_spans_and_shows_stored_values() {
        let mut task = Task::new("Buy p1 batteries #2", Id::from("P-IN"));
        task.id = Id::from("T-1");
        let v = EditorValues::from_task(&task);
        let s = card_state(&v, CardMode::Edit, &ctx());
        assert!(s.spans.is_empty());
        assert!(!chip(&s, ChipId::Priority).set);
        assert_eq!(s.title, "Buy p1 batteries #2");
    }

    #[test]
    fn title_is_echoed() {
        let v = blank("call mom tomorrow p1");
        assert_eq!(card_state(&v, CardMode::Add, &ctx()).title, v.content);
    }

    #[test]
    fn can_submit_needs_text() {
        assert!(!card_state(&blank("   "), CardMode::Add, &ctx()).can_submit);
        assert!(card_state(&blank("x"), CardMode::Add, &ctx()).can_submit);
    }

    #[test]
    fn picking_date_strips_typed_date() {
        let mut v = blank("call mom tomorrow 5pm");
        let ev = GlassEvent::Pick { pick: Pick::Date { date: "2026-10-23".to_string(), time: String::new() } };
        assert_eq!(apply_event(&mut v, &ev, CardMode::Add, &ctx()), Outcome::Changed);
        assert_eq!(v.content, "call mom");
        assert_eq!(v.date, "2026-10-23");
        assert_eq!(v.time, "");
    }

    #[test]
    fn picking_in_edit_mode_does_not_touch_title() {
        let mut v = blank("Buy p1 batteries");
        let ev = GlassEvent::Pick { pick: Pick::Priority { value: "1".to_string() } };
        apply_event(&mut v, &ev, CardMode::Edit, &ctx());
        assert_eq!(v.content, "Buy p1 batteries");
        assert_eq!(v.priority, "1");
    }

    #[test]
    fn typing_updates_the_values() {
        let mut v = blank("");
        assert_eq!(apply_event(&mut v, &GlassEvent::Title { text: "hi".into() }, CardMode::Add, &ctx()), Outcome::Changed);
        apply_event(&mut v, &GlassEvent::Description { text: "more".into() }, CardMode::Add, &ctx());
        assert_eq!((v.content.as_str(), v.description.as_str()), ("hi", "more"));
    }

    #[test]
    fn submit_and_cancel_outcomes() {
        let mut v = blank("x");
        assert_eq!(apply_event(&mut v, &GlassEvent::Submit, CardMode::Add, &ctx()), Outcome::Submit);
        assert_eq!(apply_event(&mut v, &GlassEvent::Cancel, CardMode::Add, &ctx()), Outcome::Cancel);
    }

    #[test]
    fn events_deserialize_from_swift_json() {
        let ev: GlassEvent =
            serde_json::from_str(r#"{"type":"pick","pick":{"type":"date","date":"2026-10-23","time":"09:00"}}"#).unwrap();
        assert_eq!(ev, GlassEvent::Pick { pick: Pick::Date { date: "2026-10-23".into(), time: "09:00".into() } });
        let ev: GlassEvent = serde_json::from_str(
            r#"{"type":"pick","pick":{"type":"location","projectId":"P-WORK","sectionId":null}}"#,
        )
        .unwrap();
        assert_eq!(ev, GlassEvent::Pick { pick: Pick::Location { project_id: Id::from("P-WORK"), section_id: None } });
        let ev: GlassEvent = serde_json::from_str(r#"{"type":"title","text":"hi"}"#).unwrap();
        assert_eq!(ev, GlassEvent::Title { text: "hi".into() });
    }

    #[test]
    fn add_submit_resets_to_defaults() {
        let initial = blank("");
        assert_eq!(after_submit(CardMode::Add, &initial), Some(initial.clone()));
        assert_eq!(after_submit(CardMode::Edit, &initial), None);
    }

    #[test]
    fn picker_offers_the_web_quick_dates_and_presets() {
        let p = card_state(&blank(""), CardMode::Add, &ctx()).picker;
        let quick: Vec<(&str, &str)> = p.quick_dates.iter().map(|q| (q.label.as_str(), q.date.as_str())).collect();
        assert_eq!(
            quick,
            vec![("Today", "2026-10-19"), ("Tomorrow", "2026-10-20"), ("This weekend", "2026-10-24"), ("Next week", "2026-10-26")]
        );
        let offsets: Vec<i64> = p.reminder_presets.iter().map(|(m, _)| *m).collect();
        assert_eq!(offsets, vec![0, 10, 30, 60, 1440]);
        assert_eq!(p.reminder_presets[4].1, crate::pickers::offset_text(1440));
        assert!(!p.has_due);
        assert!(card_state(&blank("x tomorrow"), CardMode::Add, &ctx()).picker.has_due);
    }

    #[test]
    fn picker_labels_each_reminder() {
        let mut v = blank("x");
        v.reminders = vec![ReminderDraft { id: None, minute_offset: Some(30), at: None }];
        let p = card_state(&v, CardMode::Add, &ctx()).picker;
        assert_eq!(p.reminder_labels, vec![v.reminders[0].label(today())]);
    }

    #[test]
    fn button_names_map_to_actions() {
        assert_eq!(button_action("fab"), Some(ButtonAction::AddTask));
        assert_eq!(button_action("menu"), Some(ButtonAction::OpenMenu));
        assert_eq!(button_action("search"), Some(ButtonAction::Search));
        assert_eq!(button_action("other"), None);
    }

    #[test]
    fn a_pick_that_rewrites_the_title_replaces_the_field_text() {
        let pick = GlassEvent::Pick { pick: Pick::Priority { value: "1".into() } };
        assert_eq!(refresh_for(&pick, "call p1", "call"), Refresh::Replace);
        assert_eq!(refresh_for(&pick, "call", "call"), Refresh::Update);
        // Typing never replaces the field (it's already showing the text).
        assert_eq!(refresh_for(&GlassEvent::Title { text: "call".into() }, "cal", "call"), Refresh::Update);
    }

    #[test]
    fn priority_spans_and_chip_carry_the_flag_level() {
        let s = card_state(&blank("call p2"), CardMode::Add, &ctx());
        assert_eq!(s.spans[0].kind, "priority-2");
        assert_eq!(chip(&s, ChipId::Priority).tone.as_deref(), Some("p2"));
        assert_eq!(chip(&s, ChipId::Date).tone, None);
        assert_eq!(chip(&card_state(&blank("x"), CardMode::Add, &ctx()), ChipId::Priority).tone, None);
    }

    #[test]
    fn search_swaps_the_search_button_for_close_and_drops_add() {
        let state =
            |visible, add, searching| ButtonsState { visible, add, searching, drawer: Vec::new(), theme: "system".into() };
        assert_eq!(floating_buttons(false, false), state(true, true, false));
        assert_eq!(floating_buttons(true, false), state(true, false, true));
        assert!(!floating_buttons(false, true).visible, "hidden while the drawer is open");
    }

    #[test]
    fn long_press_offers_the_row_actions() {
        let ids = |r| task_actions(r).into_iter().map(|a| a.id).collect::<Vec<_>>();
        assert_eq!(ids(false), vec![TaskAction::SubTask, TaskAction::Delete]);
        assert_eq!(ids(true), vec![TaskAction::SubTask, TaskAction::Streak, TaskAction::Delete]);
        assert!(task_actions(false).last().unwrap().destructive);
    }

    #[test]
    fn chosen_actions_deserialize_from_swift_json() {
        let c: ChosenAction = serde_json::from_str(r#"{"target":"T-1","action":"subTask"}"#).unwrap();
        assert_eq!(c, ChosenAction { target: "T-1".to_string(), action: TaskAction::SubTask });
    }

    #[test]
    fn sidebar_rows_offer_rename_and_delete() {
        let items = nav_actions("Rename");
        let ids: Vec<TaskAction> = items.iter().map(|a| a.id).collect();
        assert_eq!(ids, vec![TaskAction::Rename, TaskAction::Delete]);
        assert_eq!(items[0].label, "Rename");
        assert!(items[1].destructive && !items[0].destructive);
        assert_eq!(nav_actions("Edit filter")[0].label, "Edit filter");
    }

    #[test]
    fn drawer_buttons_take_the_measured_spots_and_theme_symbol() {
        let rects = vec![
            WebRect { id: "theme".into(), x: 250.0, y: 60.0, w: 36.0, h: 36.0 },
            WebRect { id: "settings".into(), x: 300.0, y: 60.0, w: 36.0, h: 36.0 },
        ];
        let b = drawer_buttons(&rects, "dark");
        assert_eq!(b[0].symbol, "moon");
        assert_eq!(b[1].symbol, "gearshape");
        assert_eq!((b[1].x, b[1].y, b[1].w), (300.0, 60.0, 36.0));
        assert_eq!(drawer_buttons(&rects, "light")[0].symbol, "sun.max");
        assert_eq!(drawer_buttons(&rects, "system")[0].symbol, "desktopcomputer");
        assert_eq!(button_action("theme"), Some(ButtonAction::Theme));
        assert_eq!(button_action("settings"), Some(ButtonAction::Settings));
    }

    #[test]
    fn buttons_carry_the_app_theme() {
        assert_eq!(floating_buttons(false, false).theme, "system");
        let mut s = floating_buttons(false, false);
        s.theme = "dark".into();
        assert_eq!(serde_json::to_value(&s).unwrap()["theme"], "dark");
    }

    #[test]
    fn prompts_serialize_for_swift_and_answers_come_back() {
        let p = Prompt::new("new-project", "New project", "Add").field("Project name", "");
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["id"], "new-project");
        assert_eq!(v["confirm"], "Add");
        assert_eq!(v["fields"][0]["placeholder"], "Project name");
        assert_eq!(v["message"], "");
        let a: PromptAnswer = serde_json::from_str(r#"{"id":"new-project","values":["Groceries"]}"#).unwrap();
        assert_eq!(a, PromptAnswer { id: "new-project".into(), values: Some(vec!["Groceries".into()]) });
        let cancel: PromptAnswer = serde_json::from_str(r#"{"id":"new-project","values":null}"#).unwrap();
        assert_eq!(cancel.values, None);
    }

    #[test]
    fn a_prompt_answer_is_trimmed_and_blank_means_nothing() {
        assert_eq!(answer_text(&["  Groceries ".to_string()], 0), Some("Groceries".to_string()));
        assert_eq!(answer_text(&["   ".to_string()], 0), None);
        assert_eq!(answer_text(&[], 0), None);
    }

    #[test]
    fn backspace_after_a_highlight_keeps_the_phrase_as_text() {
        let mut v = blank("call mom today");
        assert_eq!(card_state(&v, CardMode::Add, &ctx()).spans.len(), 1);
        let ev = GlassEvent::Unhighlight { at: 14 };
        assert_eq!(apply_event(&mut v, &ev, CardMode::Add, &ctx()), Outcome::Changed);
        assert_eq!(v.content, "call mom today", "nothing is deleted");
        let s = card_state(&v, CardMode::Add, &ctx());
        assert!(s.spans.is_empty());
        assert!(!chip(&s, ChipId::Date).set);
        // Typing on keeps it plain; new phrases still highlight.
        apply_event(&mut v, &GlassEvent::Title { text: "call mom today 5pm".into() }, CardMode::Add, &ctx());
        let s = card_state(&v, CardMode::Add, &ctx());
        assert_eq!(s.spans.len(), 1);
        assert_eq!((s.spans[0].start, s.spans[0].kind.as_str()), (15, "time"));
    }

    #[test]
    fn unhighlight_needs_a_highlight_ending_there() {
        let mut v = blank("call mom today");
        apply_event(&mut v, &GlassEvent::Unhighlight { at: 4 }, CardMode::Add, &ctx());
        assert!(v.plain.is_empty());
        let ev: GlassEvent = serde_json::from_str(r#"{"type":"unhighlight","at":14}"#).unwrap();
        assert_eq!(ev, GlassEvent::Unhighlight { at: 14 });
    }

    #[test]
    fn the_date_chip_takes_the_due_color_and_a_repeat_mark() {
        let s = card_state(&blank("x tomorrow"), CardMode::Add, &ctx());
        assert_eq!(chip(&s, ChipId::Date).tone.as_deref(), Some("due-tomorrow"));
        assert!(!chip(&s, ChipId::Date).repeats);
        let s = card_state(&blank("x every monday"), CardMode::Add, &ctx());
        assert_eq!(chip(&s, ChipId::Date).tone.as_deref(), Some("due-today"), "today is a Monday");
        assert!(chip(&s, ChipId::Date).repeats);
        assert_eq!(chip(&card_state(&blank("x"), CardMode::Add, &ctx()), ChipId::Date).tone, None);
    }

    #[test]
    fn the_calendar_rings_the_days_a_task_repeats_on() {
        let repeats = |v: &EditorValues| -> Vec<String> {
            let p = card_state(v, CardMode::Add, &ctx()).picker;
            p.calendar.weeks.iter().flatten().filter(|c| c.repeat).map(|c| c.date.format("%Y-%m-%d").to_string()).collect()
        };
        let r = repeats(&blank("x every monday"));
        assert_eq!(&r[..2], &["2026-10-19", "2026-10-26"]);
        assert!(!r.contains(&"2026-10-20".to_string()));
        // A repeat picked from the menu counts too.
        let mut v = blank("x");
        v.date = "2026-10-21".into();
        v.repeat = "weekly".into();
        assert_eq!(&repeats(&v)[..2], &["2026-10-21", "2026-10-28"]);
        assert!(repeats(&blank("x tomorrow")).is_empty());
    }

    #[test]
    fn a_phrase_alone_highlights_but_cant_be_sent() {
        let s = card_state(&blank("This Weekend"), CardMode::Add, &ctx());
        assert_eq!(s.spans.len(), 1);
        assert!(chip(&s, ChipId::Date).set);
        assert!(!s.can_submit, "no name yet");
        assert!(card_state(&blank("Clean This Weekend"), CardMode::Add, &ctx()).can_submit);
        // Editing doesn't parse, so a title like that is just a name.
        assert!(card_state(&blank("This Weekend"), CardMode::Edit, &ctx()).can_submit);
    }

    #[test]
    fn the_picker_calendar_follows_the_chosen_date_and_navigation() {
        let mut c = ctx();
        let mut v = blank("x");
        v.date = "2026-12-04".into();
        let p = card_state(&v, CardMode::Add, &c).picker;
        assert_eq!(p.calendar.title, "December 2026", "opens on the chosen date's month");
        assert_eq!(p.years.first(), Some(&2025));
        assert_eq!(p.months[0], "January");
        assert!(apply_calendar(&mut c, &GlassEvent::CalendarShift { delta: 1 }, &v));
        assert_eq!(card_state(&v, CardMode::Add, &c).picker.calendar.title, "January 2027");
        assert!(apply_calendar(&mut c, &GlassEvent::CalendarMonth { year: 2028, month: 3 }, &v));
        assert_eq!(card_state(&v, CardMode::Add, &c).picker.calendar.title, "March 2028");
        // Picking a date brings the calendar back to it.
        assert!(!apply_calendar(&mut c, &GlassEvent::Pick { pick: Pick::Date { date: "2026-10-24".into(), time: String::new() } }, &v));
        assert_eq!(c.calendar, None);
    }

    #[test]
    fn the_picker_colors_match_the_date_chip() {
        let p = card_state(&blank("x tomorrow"), CardMode::Add, &ctx()).picker;
        assert_eq!(p.selected_tone.as_deref(), Some("due-tomorrow"));
        let tones: Vec<&str> = p.quick_dates.iter().map(|q| q.tone.as_str()).collect();
        // Today (Mon 19), Tomorrow, This weekend (Sat 24), Next week (Mon 26).
        assert_eq!(tones, vec!["due-today", "due-tomorrow", "due-week", "due-later"]);
    }
}
