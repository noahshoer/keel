//! The Todoist-style task editor card, shared by "+ Add task" and in-place
//! editing, plus the collapsed "+ Add task" row.

use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use todo_core::commands::DueEdit;
use todo_core::ids::Id;
use todo_core::model::{Label, Priority, Project, Task};
use todo_core::recurrence::Recurrence;

use crate::api::{self, NewTask, TaskPatch};
use crate::icons::{self, Icon};

/// Everything the editor edits, as form-friendly strings.
#[derive(Clone, PartialEq, Debug)]
pub struct EditorValues {
    pub content: String,
    pub description: String,
    /// `YYYY-MM-DD` or empty.
    pub date: String,
    /// "1".."4".
    pub priority: String,
    /// none | daily | weekdays | weekly | monthly | keep (an existing custom rule).
    pub repeat: String,
    pub project_id: Id,
    pub labels: Vec<String>,
}

impl EditorValues {
    pub fn blank(project_id: Id, date: String, labels: Vec<String>) -> Self {
        EditorValues {
            content: String::new(),
            description: String::new(),
            date,
            priority: "4".to_string(),
            repeat: "none".to_string(),
            project_id,
            labels,
        }
    }

    pub fn from_task(t: &Task) -> Self {
        let rule = t.due.as_ref().and_then(|d| d.recurrence.clone());
        EditorValues {
            content: t.content.clone(),
            description: t.description.clone(),
            date: t.due.as_ref().and_then(|d| d.date.get(..10)).unwrap_or("").to_string(),
            priority: t.priority.to_api_ui().to_string(),
            repeat: repeat_choice(&rule).to_string(),
            project_id: t.project_id.clone(),
            labels: t.labels.clone(),
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
}

/// UI priority number ("P1" = 1 … none = 4) for a stored priority.
trait UiPriority {
    fn to_api_ui(self) -> u8;
}

impl UiPriority for Priority {
    fn to_api_ui(self) -> u8 {
        5 - self.to_api()
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
    let anchor = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap_or_else(|_| crate::today());
    match choice {
        "daily" => Some(Recurrence::Daily),
        "weekdays" => Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])),
        "weekly" => Some(Recurrence::Weekly(vec![anchor.weekday().num_days_from_monday() as u8])),
        "monthly" => Some(Recurrence::MonthlyDay(anchor.day() as u8)),
        "keep" => existing,
        _ => None,
    }
}

/// Builds a patch containing only what changed between `before` and `after`.
pub fn patch_for(before: &EditorValues, after: &EditorValues, existing: Option<Recurrence>) -> TaskPatch {
    let due_changed = before.date != after.date || before.repeat != after.repeat;
    TaskPatch {
        content: (after.content.trim() != before.content).then(|| after.content.trim().to_string()),
        description: (after.description != before.description).then(|| after.description.clone()),
        priority: (after.priority != before.priority).then(|| after.priority()),
        project_id: (after.project_id != before.project_id).then(|| after.project_id.clone()),
        labels: (after.labels != before.labels).then(|| after.labels.clone()),
        due: due_changed.then(|| DueEdit {
            date: (!after.date.is_empty()).then(|| after.date.clone()),
            recurrence: recurrence_for(&after.repeat, &after.date, existing),
        }),
    }
}

/// The new task described by `v`.
pub fn new_task(v: &EditorValues) -> NewTask {
    NewTask {
        content: v.content.trim().to_string(),
        description: v.description.clone(),
        project_id: v.project_id.clone(),
        recurrence: recurrence_for(&v.repeat, &v.date, None),
        date: v.date.clone(),
        priority: v.priority(),
    }
}

#[component]
pub fn TaskEditor(
    initial: EditorValues,
    projects: Vec<Project>,
    labels: Vec<Label>,
    /// Text for the "keep current rule" repeat option, when the rule is custom.
    keep_label: Option<String>,
    submit_text: String,
    on_submit: EventHandler<EditorValues>,
    on_cancel: EventHandler<()>,
    reload: Signal<u32>,
) -> Element {
    let mut reload = reload;
    let start = initial.clone();
    let mut vals = use_signal(move || start);
    let mut picker = use_signal(|| false);
    let mut new_label = use_signal(String::new);
    let v = vals();
    let can_submit = !v.content.trim().is_empty();
    let label_text = if v.labels.is_empty() { "Labels".to_string() } else { v.labels.join(", ") };

    rsx! {
        form {
            class: "editor",
            onsubmit: move |e| {
                e.prevent_default();
                let v = vals();
                if !v.content.trim().is_empty() {
                    on_submit.call(v);
                }
            },
            input {
                class: "name",
                placeholder: "Task name",
                autofocus: true,
                value: "{v.content}",
                oninput: move |e| {
                    let val = e.value();
                    vals.with_mut(|v| v.content = val);
                },
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
                label { class: "chip", title: "Date",
                    Icon { svg: icons::calendar_small() }
                    input {
                        r#type: "date",
                        value: "{v.date}",
                        oninput: move |e| {
                            let val = e.value();
                            vals.with_mut(|v| v.date = val);
                        },
                    }
                }
                label { class: "chip", title: "Priority",
                    Icon { svg: icons::flag_small() }
                    select {
                        value: "{v.priority}",
                        onchange: move |e| {
                            let val = e.value();
                            vals.with_mut(|v| v.priority = val);
                        },
                        option { value: "1", "Priority 1" }
                        option { value: "2", "Priority 2" }
                        option { value: "3", "Priority 3" }
                        option { value: "4", "Priority 4" }
                    }
                }
                label { class: "chip", title: "Repeat",
                    Icon { svg: icons::repeat_small() }
                    select {
                        value: "{v.repeat}",
                        onchange: move |e| {
                            let val = e.value();
                            vals.with_mut(|v| v.repeat = val);
                        },
                        option { value: "none", "No repeat" }
                        option { value: "daily", "Every day" }
                        option { value: "weekdays", "Every weekday" }
                        option { value: "weekly", "Every week" }
                        option { value: "monthly", "Every month" }
                        if let Some(text) = keep_label.clone() {
                            option { value: "keep", "{text}" }
                        }
                    }
                }
                label { class: "chip", title: "Project",
                    span { class: "chip-hash", "#" }
                    select {
                        value: "{v.project_id}",
                        onchange: move |e| {
                            let val = e.value();
                            vals.with_mut(|v| v.project_id = Id::from(val));
                        },
                        for p in projects.iter() {
                            option { key: "{p.id}", value: "{p.id}", "{p.name}" }
                        }
                    }
                }
                div { class: "chip-wrap",
                    button {
                        r#type: "button",
                        class: "chip chip-btn",
                        title: "Labels",
                        onclick: move |_| picker.set(!picker()),
                        Icon { svg: icons::tag_small() }
                        "{label_text}"
                    }
                    if picker() {
                        div { class: "label-pop",
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
pub fn AddTask(defaults: EditorValues, projects: Vec<Project>, labels: Vec<Label>, reload: Signal<u32>) -> Element {
    let mut reload = reload;
    let mut open = use_signal(|| false);
    let mut generation = use_signal(|| 0u32);

    if !open() {
        return rsx! {
            div { class: "add-task-row", onclick: move |_| open.set(true),
                span { class: "plus", Icon { svg: icons::plus(14) } }
                "Add task"
            }
        };
    }

    rsx! {
        TaskEditor {
            key: "{generation}",
            initial: defaults.clone(),
            projects,
            labels,
            keep_label: None,
            submit_text: "Add task".to_string(),
            on_submit: move |v: EditorValues| {
                spawn(async move {
                    let _ = api::add_task(new_task(&v)).await;
                    // Remount a fresh editor (Todoist keeps it open for the next task).
                    generation += 1;
                    reload += 1;
                });
            },
            on_cancel: move |_| open.set(false),
            reload,
        }
    }
}
