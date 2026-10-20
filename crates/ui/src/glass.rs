//! The native Liquid Glass card on iPhone (`tauri-plugin-glass`). Swift draws
//! it and reports what the user does; this module decides what it shows
//! (`card_state`) and what each action changes (`apply_event`), so typing,
//! chips and saving behave exactly like the web editor.


use chrono::{NaiveDate, NaiveTime};
use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::model::{Label, Project, Section, Task};
use keel_core::recurrence::Recurrence;
use keel_core::quickadd::{strip_spans, QuickAdd, SpanKind};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::editor::{
    add_from_editor, labels_text, location_text, parse_title_at, priority_text, repeat_chip_text, repeat_options,
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
    /// date | time | repeat | priority | project | section | label | deadline
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChipState {
    pub id: ChipId,
    pub label: String,
    /// Has a value (drawn tinted).
    pub set: bool,
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
}

/// A date-picker shortcut ("Tomorrow", "Next week").
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QuickDate {
    pub label: String,
    pub hint: String,
    /// `YYYY-MM-DD`
    pub date: String,
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
    pub today: NaiveDate,
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
                    kind: kind_name(s.kind).to_string(),
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

    let chip = |id, label: String, set| ChipState { id, label, set };
    let chips = vec![
        chip(ChipId::Date, date_chip(date, time, ctx.today).0, date.is_some() || time.is_some()),
        chip(ChipId::Priority, priority_text(&shown.priority), shown.priority != "4"),
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
            .map(|(label, hint, d)| QuickDate { label: label.to_string(), hint, date: d.format("%Y-%m-%d").to_string() })
            .collect(),
    };

    CardState {
        mode,
        title: v.content.clone(),
        description: v.description.clone(),
        spans,
        chips,
        can_submit: !v.content.trim().is_empty(),
        picker,
    }
}

/// Applies one action from the card to the values.
pub fn apply_event(v: &mut EditorValues, ev: &GlassEvent, mode: CardMode, ctx: &CardContext) -> Outcome {
    match ev {
        GlassEvent::Title { text } => v.content = text.clone(),
        GlassEvent::Description { text } => v.description = text.clone(),
        GlassEvent::Submit => return Outcome::Submit,
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
}

pub fn button_action(name: &str) -> Option<ButtonAction> {
    match name {
        "fab" => Some(ButtonAction::AddTask),
        "menu" => Some(ButtonAction::OpenMenu),
        "search" => Some(ButtonAction::Search),
        _ => None,
    }
}

/// The native card on iPhone; elsewhere `native` stays false and callers
/// keep the web editor.
#[derive(Clone, Copy)]
pub struct Glass {
    pub native: Signal<bool>,
    session: Signal<Option<CardSession>>,
}

/// Provides [`Glass`] (call once, in `App`): asks the plugin whether it's
/// there and, if so, starts handling its events. `on_button` gets the
/// floating buttons' taps, except Add, which is handled here.
pub fn provide(mut on_button: impl FnMut(ButtonAction) + 'static) -> Glass {
    let glass = use_context_provider(|| Glass { native: Signal::new(false), session: Signal::new(None) });
    use_hook(move || {
        spawn(async move {
            if !api::glass_available().await {
                return;
            }
            let mut native = glass.native;
            native.set(true);
            spawn(async move {
                let mut taps = document::eval(BUTTONS_JS);
                while let Ok(name) = taps.recv::<String>().await {
                    match button_action(&name) {
                        Some(ButtonAction::AddTask) => {
                            let _ = document::eval(ADD_JS);
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
        let ctx = CardContext { projects, sections, labels, keep_label: None, today: crate::today() };
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
        let ctx = CardContext { projects, sections, labels, keep_label, today: crate::today() };
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

    fn open(mut self, s: CardSession) {
        let state = card_state(&s.values, s.mode, &s.ctx);
        self.session.set(Some(s));
        spawn(async move { api::glass_show(&state).await });
    }

    async fn handle(mut self, ev: GlassEvent) {
        let Some(mut s) = self.session.peek().clone() else { return };
        match apply_event(&mut s.values, &ev, s.mode, &s.ctx) {
            Outcome::Changed => {
                let state = card_state(&s.values, s.mode, &s.ctx);
                self.session.set(Some(s));
                api::glass_update(&state).await;
            }
            Outcome::Cancel => {
                self.session.set(None);
                api::glass_hide().await;
            }
            Outcome::Submit => {
                if s.values.content.trim().is_empty() {
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
        CardContext { projects: vec![inbox, work], sections: vec![], labels: vec![], keep_label: None, today: today() }
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
}
