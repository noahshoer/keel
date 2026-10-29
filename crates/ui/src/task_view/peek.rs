//! The view under a sub-task's, drawn read-only and out of sight until an
//! edge swipe pulls the sub-task's content aside (`gestures::SHEET_DRAG_JS`
//! shows it), so the parent slides in as the finger moves. It repeats the
//! layout of the real view and does nothing on a tap.

use chrono::NaiveDate;
use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::model::{Project, Reminder, Section};

use super::fields::{layout, subtask_header, Field};
use super::nav::TaskDetail;
use crate::editor::{flag_class, location_text, EditorValues};
use crate::icons::{self, Icon};
use crate::pickers::{date_chip, deadline_text, reminders_text, ReminderDraft};
use crate::task_row::{check_class, SubtaskCount};

/// One set field's row: its text, icon and the icon's color class.
fn row(f: Field, v: &EditorValues, today: NaiveDate, projects: &[Project], sections: &[Section]) -> (String, String, String) {
    let date = NaiveDate::parse_from_str(&v.date, "%Y-%m-%d").ok();
    let time = chrono::NaiveTime::parse_from_str(&v.time, "%H:%M").ok();
    match f {
        Field::Description => (v.description.clone(), icons::text_lines(), String::new()),
        Field::Date => {
            let (text, cls) = date_chip(date, time, today);
            (text, icons::calendar_filled(), cls.to_string())
        }
        Field::Deadline => (
            format!("Deadline {}", deadline_text(NaiveDate::parse_from_str(&v.deadline, "%Y-%m-%d").ok(), today)),
            icons::target_small(),
            String::new(),
        ),
        Field::Priority => (format!("Priority {}", v.priority), icons::flag_filled(), flag_class(&v.priority).to_string()),
        Field::Labels => (v.labels.join(", "), icons::tag_small(), String::new()),
        Field::Reminders => (reminders_text(&v.reminders, today), icons::bell_small(), String::new()),
        Field::Location => (
            location_text(&v.project_id, v.section_id.as_ref(), projects, sections),
            icons::hash_small(),
            String::new(),
        ),
    }
}

#[component]
pub fn TaskPeek(detail: TaskDetail, projects: Vec<Project>, sections: Vec<Section>, inbox_id: Id) -> Element {
    let today = crate::today();
    let reminders = use_context::<Signal<Vec<Reminder>>>();
    let task = detail.task.clone();
    let mut v = EditorValues::from_task(&task);
    v.reminders = reminders.read().iter().filter(|r| r.task_id == task.id).map(ReminderDraft::from_reminder).collect();
    let (fields, missing) = layout(&v, &inbox_id);
    let header = detail.progress.map(|p| (p, subtask_header(p, false)));
    let checked = task.checked;
    rsx! {
        div { class: "tv-peek", aria_hidden: "true",
            div { class: "tv-peek-in",
                div { class: if checked { "tv-titleline done" } else { "tv-titleline" },
                    span { class: if checked { "check done" } else { check_class(task.priority) }, Icon { svg: icons::check() } }
                    div { class: "tv-grow tv-title", div { class: "tv-peek-text", "{task.content}" } }
                }
                div { class: "tv-card tv-fields",
                    for f in fields.iter().copied() {
                        div { key: "{f:?}", class: "tv-row",
                            {
                                let (text, svg, cls) = row(f, &v, today, &projects, &sections);
                                if f == Field::Description {
                                    rsx! {
                                        span { class: "tv-row-icon", Icon { svg } }
                                        div { class: "tv-grow tv-desc", div { class: "tv-peek-text", "{text}" } }
                                    }
                                } else {
                                    rsx! { span { class: "tv-field-btn", span { class: "chip-icon {cls}", Icon { svg } } "{text}" } }
                                }
                            }
                        }
                    }
                    if !missing.is_empty() {
                        div { class: "tv-pills",
                            for m in missing.iter().copied() {
                                span { key: "{m:?}", class: "tv-pill-slot", span { class: "tv-pill", "{m.label()}" } }
                            }
                        }
                    }
                }
                div { class: "tv-card tv-subtasks",
                    if let Some((_, (count, toggle, _))) = header.clone() {
                        div { class: "tv-sub-head",
                            b { "Sub-tasks" }
                            span { class: "tv-sub-side", "{count} · " span { class: "tv-toggle", "{toggle}" } }
                        }
                    }
                    div { class: "tv-sub-list",
                        if let Some((p, _)) = header {
                            div { class: "tv-progress", i { style: "width: {p.done * 100 / p.total.max(1)}%" } }
                        }
                        for r in detail.visible.iter().cloned() {
                            div { key: "{r.task.id}", class: if r.task.checked { "tv-sub done" } else { "tv-sub" },
                                span { class: if r.task.checked { "check done" } else { check_class(r.task.priority) }, Icon { svg: icons::check() } }
                                div { class: "tv-sub-body",
                                    span { class: "task-content", "{r.task.content}" }
                                    if let Some(p) = r.progress {
                                        SubtaskCount { progress: p }
                                    }
                                    span { class: "card-chev", Icon { svg: icons::chevron_right() } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
