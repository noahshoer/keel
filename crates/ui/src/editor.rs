//! The Todoist-style task editor card, shared by "+ Add task", "Add sub-task"
//! and in-place editing, plus the collapsed "+ Add task" row. When adding, the
//! title is parsed as natural language (dates, times, repeats, p1, #project,
//! /section, @labels) with a live preview.

use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use keel_core::commands::DueEdit;
use keel_core::ids::Id;
use keel_core::model::{Label, Priority, Project, Section, Task};
use keel_core::quickadd::{self, QuickAdd, Span, SpanKind};
use keel_core::recurrence::Recurrence;
use keel_core::views;

use crate::api::{self, NewTask, TaskPatch};
use crate::icons::{self, Icon};
use crate::pickers::{DatePicker, DeadlinePicker, Menu, MenuItem, OpenPop, ReminderDraft, ReminderPicker};

/// Everything the editor edits, as form-friendly values.
#[derive(Clone, PartialEq, Debug)]
pub struct EditorValues {
    pub content: String,
    pub description: String,
    /// `YYYY-MM-DD` or empty.
    pub date: String,
    /// `HH:MM` or empty.
    pub time: String,
    /// "1".."4".
    pub priority: String,
    /// none | daily | weekdays | weekly | monthly | keep (an existing custom rule).
    pub repeat: String,
    pub project_id: Id,
    pub section_id: Option<Id>,
    pub labels: Vec<String>,
    /// Only used when adding a sub-task.
    pub parent_id: Option<Id>,
    /// `YYYY-MM-DD` or empty.
    pub deadline: String,
    pub reminders: Vec<ReminderDraft>,
}

impl EditorValues {
    pub fn blank(project_id: Id, section_id: Option<Id>, date: String, labels: Vec<String>) -> Self {
        EditorValues {
            content: String::new(),
            description: String::new(),
            date,
            time: String::new(),
            priority: "4".to_string(),
            repeat: "none".to_string(),
            project_id,
            section_id,
            labels,
            parent_id: None,
            deadline: String::new(),
            reminders: Vec::new(),
        }
    }

    pub fn from_task(t: &Task) -> Self {
        let rule = t.due.as_ref().and_then(|d| d.recurrence.clone());
        EditorValues {
            content: t.content.clone(),
            description: t.description.clone(),
            date: t.due.as_ref().and_then(|d| d.date.get(..10)).unwrap_or("").to_string(),
            time: t.due.as_ref().and_then(|d| d.date.get(11..16)).unwrap_or("").to_string(),
            priority: (5 - t.priority.to_api()).to_string(),
            repeat: repeat_choice(&rule).to_string(),
            project_id: t.project_id.clone(),
            section_id: t.section_id.clone(),
            labels: t.labels.clone(),
            parent_id: t.parent_id.clone(),
            deadline: t.deadline.clone().unwrap_or_default(),
            reminders: Vec::new(),
        }
    }

    fn priority(&self) -> Priority {
        match self.priority.as_str() {
            "1" => Priority::P1,
            "2" => Priority::P2,
            "3" => Priority::P3,
            _ => Priority::None,
        }
    }

    /// `project` or `project|section`, for the location select.
    fn location(&self) -> String {
        match &self.section_id {
            Some(s) => format!("{}|{}", self.project_id, s),
            None => self.project_id.to_string(),
        }
    }

    /// The `Due.date` string for the chosen date/time ("" = no due). A time
    /// without a date means today.
    fn due_string(&self) -> String {
        match (self.date.is_empty(), self.time.is_empty()) {
            (true, true) => String::new(),
            (false, true) => self.date.clone(),
            (true, false) => format!("{}T{}:00", crate::today().format("%Y-%m-%d"), self.time),
            (false, false) => format!("{}T{}:00", self.date, self.time),
        }
    }
}

/// Which repeat option represents an existing rule.
fn repeat_choice(rule: &Option<Recurrence>) -> &'static str {
    match rule {
        None => "none",
        Some(Recurrence::Daily) => "daily",
        Some(Recurrence::Weekly(days)) if days == &vec![0, 1, 2, 3, 4] => "weekdays",
        Some(Recurrence::Weekly(days)) if days.len() == 1 => "weekly",
        Some(Recurrence::MonthlyDay(_)) => "monthly",
        Some(_) => "keep",
    }
}

/// The rule for a repeat choice; "Every week/month" anchor on `date` (or today).
fn recurrence_for(choice: &str, date: &str, existing: Option<Recurrence>) -> Option<Recurrence> {
    let anchor = NaiveDate::parse_from_str(date.get(..10).unwrap_or(date), "%Y-%m-%d")
        .unwrap_or_else(|_| crate::today());
    match choice {
        "daily" => Some(Recurrence::Daily),
        "weekdays" => Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])),
        "weekly" => Some(Recurrence::Weekly(vec![anchor.weekday().num_days_from_monday() as u8])),
        "monthly" => Some(Recurrence::MonthlyDay(anchor.day() as u8)),
        "keep" => existing,
        _ => None,
    }
}

fn ordinal(n: u8) -> String {
    keel_core::recurrence::ordinal(n as u32)
}

/// Human text for a repeat rule ("Every other week on Wed").
pub fn repeat_text(rule: &Recurrence) -> String {
    let text = rule.describe();
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

/// The Labels chip's text.
pub(crate) fn labels_text(labels: &[String]) -> String {
    if labels.is_empty() { "Labels".to_string() } else { labels.join(", ") }
}

/// The Priority chip's text ("4" is no priority).
pub(crate) fn priority_text(priority: &str) -> String {
    if priority == "4" { "Priority".to_string() } else { format!("P{priority}") }
}

/// The repeat choices as (value, label); week/month options name the day
/// they repeat on, and "keep" appears when the task has a custom rule.
pub(crate) fn repeat_options(date: &str, keep_label: Option<&str>, today: NaiveDate) -> Vec<(&'static str, String)> {
    let anchor = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap_or(today);
    let mut opts = vec![
        ("none", "No repeat".to_string()),
        ("daily", "Every day".to_string()),
        ("weekdays", "Every weekday (Mon – Fri)".to_string()),
        ("weekly", format!("Every week on {}", anchor.format("%A"))),
        ("monthly", format!("Every month on the {}", ordinal(anchor.day() as u8))),
    ];
    if let Some(text) = keep_label {
        opts.push(("keep", text.to_string()));
    }
    opts
}

/// The Repeat chip's text for the chosen option.
pub(crate) fn repeat_chip_text(repeat: &str, opts: &[(&'static str, String)]) -> String {
    opts.iter()
        .find(|(value, _)| *value == repeat && *value != "none")
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| "Repeat".to_string())
}

/// "Project" or "Project / Section" for the location chip.
pub(crate) fn location_text(project_id: &Id, section_id: Option<&Id>, projects: &[Project], sections: &[Section]) -> String {
    let Some(p) = projects.iter().find(|p| &p.id == project_id) else { return "Inbox".to_string() };
    match section_id.and_then(|s| sections.iter().find(|x| &x.id == s && x.project_id == p.id)) {
        Some(s) => format!("{} / {}", p.name, s.name),
        None => p.name.clone(),
    }
}

/// Field changes between `before` and `after`, plus a move when the
/// project/section changed (moves are a separate command that also carries
/// the sub-tree).
pub fn changes(
    before: &EditorValues,
    after: &EditorValues,
    existing: Option<Recurrence>,
) -> (TaskPatch, Option<(Id, Option<Id>)>) {
    let due_changed = before.date != after.date || before.time != after.time || before.repeat != after.repeat;
    let due = after.due_string();
    let patch = TaskPatch {
        content: (after.content.trim() != before.content).then(|| after.content.trim().to_string()),
        description: (after.description != before.description).then(|| after.description.clone()),
        priority: (after.priority != before.priority).then(|| after.priority()),
        project_id: None,
        labels: (after.labels != before.labels).then(|| after.labels.clone()),
        deadline: (after.deadline != before.deadline).then(|| after.deadline.clone()),
        due: due_changed.then(|| DueEdit {
            date: (!due.is_empty()).then(|| due.clone()),
            recurrence: recurrence_for(&after.repeat, &due, existing),
        }),
    };
    let moved = (after.project_id != before.project_id || after.section_id != before.section_id)
        .then(|| (after.project_id.clone(), after.section_id.clone()));
    (patch, moved)
}

/// Parses the editor's title as quick add.
fn parse_title(v: &EditorValues, projects: &[Project], sections: &[Section]) -> QuickAdd {
    parse_title_at(v, projects, sections, crate::today())
}

/// [`parse_title`] relative to `today` (testable off the browser).
pub(crate) fn parse_title_at(v: &EditorValues, projects: &[Project], sections: &[Section], today: NaiveDate) -> QuickAdd {
    let mut qa = quickadd::parse(&v.content, today, projects, sections);
    // A bare /Section resolves within the Inbox; carry its project along.
    if qa.project_id.is_none() {
        if let Some(s) = qa.section_id.as_ref().and_then(|id| sections.iter().find(|s| &s.id == id)) {
            qa.project_id = Some(s.project_id.clone());
        }
    }
    qa
}

/// The new task described by the editor values with the quick-add parse
/// applied on top (text wins over chips for the fields it mentions).
fn new_task_from(v: &EditorValues, qa: &QuickAdd) -> NewTask {
    let date = qa.due_string().unwrap_or_else(|| v.due_string());
    let recurrence = qa.recurrence.clone().or_else(|| recurrence_for(&v.repeat, &date, None));
    let mut labels = v.labels.clone();
    for l in &qa.labels {
        if !labels.iter().any(|x| x.eq_ignore_ascii_case(l)) {
            labels.push(l.clone());
        }
    }
    let (project_id, section_id) = match (&qa.project_id, &qa.section_id) {
        (Some(p), s) => (p.clone(), s.clone()),
        (None, _) => (v.project_id.clone(), v.section_id.clone()),
    };
    NewTask {
        content: qa.content.trim().to_string(),
        description: v.description.clone(),
        project_id,
        recurrence,
        date,
        priority: qa.priority.unwrap_or_else(|| v.priority()),
        labels,
        section_id,
        parent_id: v.parent_id.clone(),
        deadline: qa.deadline.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| v.deadline.clone()),
    }
}

/// Adds the task the editor describes (quick add applied), creating any
/// labels it mentions that don't exist yet.
pub async fn add_from_editor(v: EditorValues, projects: Vec<Project>, sections: Vec<Section>, labels: Vec<Label>) {
    let qa = parse_title(&v, &projects, &sections);
    let task = new_task_from(&v, &qa);
    for name in &qa.labels {
        if !labels.iter().any(|l| l.name.eq_ignore_ascii_case(name)) {
            let _ = api::add_label(name.clone()).await;
        }
    }
    if let Ok(result) = api::add_task(task).await {
        if let Some(id) = result.id_mapping.values().next() {
            sync_reminders(id.clone(), &[], &v.reminders).await;
        }
    }
}

/// Saves an edited task: the move (with its sub-tree), reminder changes,
/// then the field changes.
pub async fn save_edit(id: Id, before: EditorValues, after: EditorValues, existing: Option<Recurrence>) {
    let (patch, moved) = changes(&before, &after, existing);
    if let Some((project, section)) = moved {
        let _ = api::move_task(id.clone(), project, section).await;
    }
    sync_reminders(id.clone(), &before.reminders, &after.reminders).await;
    if !patch.is_empty() {
        let _ = api::update_task(id, patch).await;
    }
}

/// Saves reminder edits: removes the ones taken out, adds the new ones.
pub async fn sync_reminders(task_id: Id, before: &[ReminderDraft], after: &[ReminderDraft]) {
    for old in before {
        if let Some(id) = &old.id {
            if !after.iter().any(|a| a.id.as_ref() == Some(id)) {
                let _ = api::delete_reminder(id.clone()).await;
            }
        }
    }
    for new in after.iter().filter(|a| a.id.is_none()) {
        let _ = api::add_reminder(task_id.clone(), new.minute_offset, new.at.clone()).await;
    }
}

#[component]
pub fn TaskEditor(
    initial: EditorValues,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    /// Text for the "keep current rule" repeat option, when the rule is custom.
    keep_label: Option<String>,
    submit_text: String,
    /// Parse the title as natural language (add mode) and preview it.
    #[props(default)]
    quick_add: bool,
    /// Stay open after submitting, reset to `initial` (Todoist's add flow).
    #[props(default)]
    keep_open: bool,
    on_submit: EventHandler<EditorValues>,
    on_cancel: EventHandler<()>,
    reload: Signal<u32>,
) -> Element {
    let mut reload = reload;
    let start = initial.clone();
    let mut vals = use_signal(move || start);
    let blank = initial.clone();
    let blank = use_signal(move || blank);
    let mut open: OpenPop = use_signal(|| None);
    let mut new_label = use_signal(String::new);
    let v = vals();
    let can_submit = !v.content.trim().is_empty();
    let label_text = labels_text(&v.labels);
    let today = crate::today();

    // Project / section menu: each project, then its sections indented.
    let location = v.location();
    let mut location_items = Vec::new();
    let location_text = location_text(&v.project_id, v.section_id.as_ref(), &projects, &sections);
    for p in projects.iter() {
        let icon = if p.is_inbox { icons::inbox() } else { icons::hash_small() };
        let class = if p.is_inbox { "loc-inbox" } else { "loc-project" };
        location_items.push(MenuItem::new(p.id.to_string(), p.name.clone()).icon(icon, class));
        for s in sections.iter().filter(|s| s.project_id == p.id) {
            let value = format!("{}|{}", p.id, s.id);
            location_items.push(MenuItem::new(value, s.name.clone()).icon(icons::section_small(), "loc-section").indented());
        }
    }

    // Priority menu with colored flags.
    let flag_class = |p: &str| match p {
        "1" => "pri-1",
        "2" => "pri-2",
        "3" => "pri-3",
        _ => "pri-4",
    };
    let priority_items: Vec<MenuItem> = ["1", "2", "3", "4"]
        .iter()
        .map(|p| MenuItem::new(*p, format!("Priority {p}")).icon(icons::flag_small(), flag_class(p)))
        .collect();
    let priority_text = priority_text(&v.priority);

    let repeat_opts = repeat_options(&v.date, keep_label.as_deref(), today);
    let repeat_text_chip = repeat_chip_text(&v.repeat, &repeat_opts);
    let repeat_items: Vec<MenuItem> = repeat_opts.into_iter().map(|(value, label)| MenuItem::new(value, label)).collect();

    // Live quick-add parse: highlighted spans in the title + preview chips.
    let qa = quick_add.then(|| parse_title(&v, &projects, &sections));
    let spans = qa.as_ref().map(|q| q.spans.clone()).unwrap_or_default();
    let preview: Vec<(&'static str, String)> = match &qa {
        Some(qa) => {
            let mut chips = Vec::new();
            if let Some(d) = qa.date {
                let mut text = views::due_label(d, today);
                if let Some(t) = qa.time {
                    text = format!("{text} {}", views::time_label(t));
                }
                chips.push(("chip-date", text));
            }
            if let Some(r) = &qa.recurrence {
                chips.push(("chip-repeat", repeat_text(r)));
            }
            if let Some(p) = qa.priority {
                chips.push(("chip-priority", format!("P{}", 5 - p.to_api())));
            }
            if let Some(pid) = &qa.project_id {
                let project = projects.iter().find(|p| &p.id == pid).map(|p| p.name.clone()).unwrap_or_default();
                let text = match qa.section_id.as_ref().and_then(|s| sections.iter().find(|x| &x.id == s)) {
                    Some(section) => format!("#{project} / {}", section.name),
                    None => format!("#{project}"),
                };
                chips.push(("chip-project", text));
            }
            for l in &qa.labels {
                chips.push(("chip-label", format!("@{l}")));
            }
            if let Some(d) = qa.deadline {
                chips.push(("chip-deadline", format!("Deadline {}", views::due_label(d, today))));
            }
            chips
        }
        None => Vec::new(),
    };

    // Relative reminders count back from a due date (typed or picked).
    let has_due = !v.date.is_empty() || !v.time.is_empty() || qa.as_ref().is_some_and(|q| q.date.is_some());
    let title_placeholder: &'static str =
        if quick_add { "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”" } else { "Task name" };
    // Submits once: a fast second Enter (before the editor closes) is ignored.
    // In keep-open mode the form resets to blank instead (an empty title
    // can't be submitted, so a second Enter is a no-op there too).
    let mut submitted = use_signal(|| false);
    let mut submit = move || {
        let v = vals();
        if submitted() || v.content.trim().is_empty() {
            return;
        }
        if keep_open {
            vals.set(blank());
            open.set(None);
        } else {
            submitted.set(true);
        }
        on_submit.call(v);
    };

    rsx! {
        form {
            class: "editor",
            onsubmit: move |e| {
                e.prevent_default();
                submit();
            },
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    e.prevent_default();
                    if open().is_some() {
                        open.set(None);
                    } else {
                        on_cancel.call(());
                    }
                }
            },
            if open().is_some() {
                div { class: "pop-backdrop", onclick: move |_| open.set(None), onwheel: move |_| open.set(None) }
            }
            HighlightedTitle {
                value: v.content.clone(),
                spans,
                placeholder: title_placeholder,
                on_input: move |val: String| vals.with_mut(|v| v.content = val),
                on_enter: move |_| submit(),
            }
            if !preview.is_empty() {
                div { class: "quick-preview",
                    for (i, (class, text)) in preview.iter().enumerate() {
                        span { key: "{i}", class: "quick-chip {class}", "{text}" }
                    }
                }
            }
            input {
                class: "desc",
                placeholder: "Description",
                value: "{v.description}",
                oninput: move |e| {
                    let val = e.value();
                    vals.with_mut(|v| v.description = val);
                },
            }
            div { class: "chips",
                DatePicker {
                    date: v.date.clone(),
                    time: v.time.clone(),
                    open,
                    on_change: move |(date, time): (String, String)| vals.with_mut(|v| {
                        v.date = date;
                        v.time = time;
                    }),
                }
                DeadlinePicker {
                    deadline: v.deadline.clone(),
                    open,
                    on_change: move |d: String| vals.with_mut(|v| v.deadline = d),
                }
                ReminderPicker {
                    reminders: v.reminders.clone(),
                    has_due: has_due,
                    open,
                    on_change: move |rs: Vec<ReminderDraft>| vals.with_mut(|v| v.reminders = rs),
                }
                Menu {
                    id: "priority",
                    title: "Priority",
                    icon: icons::flag_small(),
                    icon_class: flag_class(&v.priority),
                    text: priority_text,
                    items: priority_items,
                    selected: v.priority.clone(),
                    open,
                    on_select: move |p: String| vals.with_mut(|v| v.priority = p),
                }
                Menu {
                    id: "repeat",
                    title: "Repeat",
                    icon: icons::repeat_small(),
                    text: repeat_text_chip,
                    items: repeat_items,
                    selected: v.repeat.clone(),
                    open,
                    on_select: move |r: String| vals.with_mut(|v| v.repeat = r),
                }
                Menu {
                    id: "location",
                    title: "Project / section",
                    icon: icons::hash_small(),
                    text: location_text,
                    items: location_items,
                    selected: location.clone(),
                    open,
                    on_select: move |val: String| vals.with_mut(|v| match val.split_once('|') {
                        Some((p, s)) => {
                            v.project_id = Id::from(p);
                            v.section_id = Some(Id::from(s));
                        }
                        None => {
                            v.project_id = Id::from(val.as_str());
                            v.section_id = None;
                        }
                    }),
                }
                div { class: "chip-wrap",
                    button {
                        r#type: "button",
                        class: "chip chip-btn",
                        class: if open() == Some("labels") { "chip-open" },
                        title: "Labels",
                        onclick: move |_| open.set(if open() == Some("labels") { None } else { Some("labels") }),
                        span { class: "chip-icon", Icon { svg: icons::tag_small() } }
                        "{label_text}"
                    }
                    if open() == Some("labels") {
                        div { class: "popover menu label-pop",
                            onmounted: move |_| crate::pickers::place_popover(),
                            if labels.is_empty() {
                                div { class: "label-empty", "No labels yet" }
                            }
                            for l in labels.iter() {
                                LabelOption {
                                    key: "{l.id}",
                                    name: l.name.clone(),
                                    checked: v.labels.iter().any(|n| n.eq_ignore_ascii_case(&l.name)),
                                    vals,
                                }
                            }
                            input {
                                class: "label-new",
                                placeholder: "New label, then Enter",
                                value: "{new_label}",
                                oninput: move |e| new_label.set(e.value()),
                                onkeydown: move |e| {
                                    if e.key() == Key::Enter {
                                        e.prevent_default();
                                        let name = new_label().trim().to_string();
                                        if name.is_empty() {
                                            return;
                                        }
                                        vals.with_mut(|v| {
                                            if !v.labels.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                                                v.labels.push(name.clone());
                                            }
                                        });
                                        new_label.set(String::new());
                                        spawn(async move {
                                            let _ = api::add_label(name).await;
                                            reload += 1;
                                        });
                                    }
                                },
                            }
                        }
                    }
                }
            }
            div { class: "editor-footer",
                button {
                    r#type: "button",
                    class: "btn btn-cancel",
                    onclick: move |_| on_cancel.call(()),
                    "Cancel"
                }
                button { r#type: "submit", class: "btn btn-add", disabled: !can_submit, "{submit_text}" }
            }
        }
    }
}

/// The task title: an auto-growing textarea over a mirror that paints the
/// quick-add phrases it recognized. Enter submits (titles are one line).
#[component]
fn HighlightedTitle(
    value: String,
    spans: Vec<Span>,
    placeholder: &'static str,
    on_input: EventHandler<String>,
    on_enter: EventHandler<()>,
) -> Element {
    // (text, highlight class) segments covering the whole value.
    let mut segments: Vec<(String, &'static str)> = Vec::new();
    let mut at = 0;
    for s in spans.iter().filter(|s| s.end <= value.len() && value.is_char_boundary(s.start) && value.is_char_boundary(s.end)) {
        if s.start < at {
            continue;
        }
        if s.start > at {
            segments.push((value[at..s.start].to_string(), ""));
        }
        let class = match s.kind {
            SpanKind::Date | SpanKind::Time => "hl hl-date",
            SpanKind::Repeat => "hl hl-repeat",
            SpanKind::Priority => "hl hl-priority",
            SpanKind::Project | SpanKind::Section => "hl hl-project",
            SpanKind::Label => "hl hl-label",
            SpanKind::Deadline => "hl hl-deadline",
        };
        segments.push((value[s.start..s.end].to_string(), class));
        at = s.end;
    }
    if at < value.len() {
        segments.push((value[at..].to_string(), ""));
    }

    rsx! {
        div { class: "title-wrap",
            div { class: "title-mirror", aria_hidden: "true",
                for (i, (text, class)) in segments.iter().enumerate() {
                    if class.is_empty() {
                        span { key: "{i}", "{text}" }
                    } else {
                        mark { key: "{i}", class: "{class}", "{text}" }
                    }
                }
                // Keeps the last line's height when the value ends in a space.
                "\u{200b}"
            }
            textarea {
                class: "name title-input",
                rows: "1",
                placeholder,
                autofocus: true,
                spellcheck: "false",
                value: "{value}",
                oninput: move |e| on_input.call(e.value().replace(['\n', '\r'], " ")),
                onkeydown: move |e| {
                    if e.key() == Key::Enter {
                        e.prevent_default();
                        on_enter.call(());
                    }
                },
            }
        }
    }
}

#[component]
fn LabelOption(name: String, checked: bool, vals: Signal<EditorValues>) -> Element {
    let mut vals = vals;
    rsx! {
        label { class: "label-opt",
            input {
                r#type: "checkbox",
                checked,
                onchange: move |_| {
                    let name = name.clone();
                    vals.with_mut(|v| {
                        if let Some(i) = v.labels.iter().position(|n| n.eq_ignore_ascii_case(&name)) {
                            v.labels.remove(i);
                        } else {
                            v.labels.push(name);
                        }
                    });
                },
            }
            Icon { svg: icons::tag_small() }
            "{name}"
        }
    }
}

/// The collapsed "+ Add task" row; expands into a [`TaskEditor`].
#[component]
pub fn AddTask(
    defaults: EditorValues,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    reload: Signal<u32>,
    /// The view's own row (the floating + opens this one).
    #[props(default)]
    main: bool,
) -> Element {
    let mut reload = reload;
    let mut open = use_signal(|| false);
    let glass = crate::glass::use_glass();

    if !open() {
        let (d, p, s, l) = (defaults.clone(), projects.clone(), sections.clone(), labels.clone());
        return rsx! {
            div {
                class: "add-task-row",
                class: if main { "view-add" },
                onclick: move |_| {
                    if (glass.native)() {
                        glass.add(d.clone(), p.clone(), s.clone(), l.clone(), reload);
                    } else {
                        open.set(true);
                    }
                },
                span { class: "plus", Icon { svg: icons::plus(14) } }
                "Add task"
            }
        };
    }

    let (p2, s2, l2) = (projects.clone(), sections.clone(), labels.clone());
    rsx! {
        TaskEditor {
            initial: defaults.clone(),
            projects,
            sections,
            labels,
            keep_label: None,
            submit_text: "Add task".to_string(),
            quick_add: true,
            keep_open: true,
            on_submit: move |v: EditorValues| {
                let (p, s, l) = (p2.clone(), s2.clone(), l2.clone());
                spawn(async move {
                    add_from_editor(v, p, s, l).await;
                    reload += 1;
                });
            },
            on_cancel: move |_| open.set(false),
            reload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_changes_leave_quick_add_text_alone() {
        let mut task = Task::new("Buy p1 batteries #2", Id::from("P-IN"));
        task.id = Id::from("T-1");
        let before = EditorValues::from_task(&task);
        let mut after = before.clone();
        after.priority = "1".to_string();
        let (patch, moved) = changes(&before, &after, None);
        assert_eq!(patch.content, None);
        assert_eq!(patch.priority, Some(Priority::P1));
        assert_eq!(moved, None);
    }
}
