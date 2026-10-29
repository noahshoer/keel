//! Settings window (gear in the sidebar header): notification preferences.
//! Notifications are per device, so these never sync.

use chrono::{NaiveTime, Timelike};
use dioxus::prelude::*;
use keel_core::reminders::{PlannedReminder, ReminderSettings};
use keel_core::views;

use crate::api;
use crate::icons::{self, Icon};
use crate::pickers::{Menu, MenuItem, OpenPop};

fn hm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).expect("valid time")
}

/// Menu value for an optional time ("off" or "HH:MM").
fn time_value(t: Option<NaiveTime>) -> String {
    t.map(|t| format!("{:02}:{:02}", t.hour(), t.minute())).unwrap_or_else(|| "off".to_string())
}

fn parse_time_value(v: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(v, "%H:%M").ok()
}

fn time_items(off_label: &str, times: &[NaiveTime]) -> Vec<MenuItem> {
    std::iter::once(MenuItem::new("off", off_label))
        .chain(times.iter().map(|t| MenuItem::new(time_value(Some(*t)), views::time_label(*t))))
        .collect()
}

fn offset_label(m: i64) -> String {
    match m {
        0 => "At the due time".to_string(),
        60 => "1 hour before".to_string(),
        m => format!("{m} minutes before"),
    }
}

/// "Travel: 12 tasks, 2 sections, 3 new labels, 1 date kept as text".
fn import_line(s: &keel_core::import::todoist::ImportSummary) -> String {
    let plural = |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut parts = vec![plural(s.tasks, "task", "tasks")];
    if s.sections > 0 {
        parts.push(plural(s.sections, "section", "sections"));
    }
    if s.labels_created > 0 {
        parts.push(plural(s.labels_created, "new label", "new labels"));
    }
    if s.comments > 0 {
        parts.push(plural(s.comments, "comment", "comments"));
    }
    if s.kept_as_text > 0 {
        parts.push(format!("{} kept as text in the description", plural(s.kept_as_text, "date", "dates")));
    }
    format!("{}: {}", s.project_name, parts.join(", "))
}

/// "Today 5 PM", "Tue 9 AM", "3 Nov 9 AM".
fn when_text(r: &PlannedReminder) -> String {
    let today = crate::today();
    let date = r.at.date();
    let day = match (date - today).num_days() {
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        2..=6 => date.format("%a").to_string(),
        _ => views::due_label(date, today),
    };
    format!("{day} {}", views::time_label(r.at.time()))
}

/// How reminders reach this device: the OS holds them on iPhone; the
/// desktop app shows them itself.
fn delivery_note(when_closed: bool) -> &'static str {
    if when_closed {
        "Reminders arrive even when Keel is closed."
    } else {
        "Reminders appear while Keel is open on this computer."
    }
}

/// Which mark fills a task's circle when it's completed (this device).
#[component]
fn CompletionSetting() -> Element {
    use crate::completion::{self, CompletionIcon};
    let mut chosen = use_context::<Signal<CompletionIcon>>();
    rsx! {
        div { class: "setting-row completion-setting",
            div { class: "setting-text",
                div { class: "setting-name", "Completion mark" }
                div { class: "setting-hint", "Fills the circle when you complete a task" }
            }
            div { class: "mark-choices",
                for icon in CompletionIcon::ALL {
                    button {
                        key: "{icon.key()}",
                        class: if chosen() == icon { "mark-choice chosen" } else { "mark-choice" },
                        title: "{icon.label()}",
                        aria_pressed: "{chosen() == icon}",
                        onclick: move |_| {
                            completion::save(icon);
                            chosen.set(icon);
                        },
                        span { class: "mark-dot", dangerous_inner_html: "{icon.svg()}" }
                        span { class: "mark-name", "{icon.label()}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SettingsDialog(on_close: EventHandler<()>, reload: Signal<u32>) -> Element {
    let mut reload = reload;
    let mut settings = use_signal(|| None::<ReminderSettings>);
    // One line per imported file: (ok, text).
    let mut imports = use_signal(Vec::<(bool, String)>::new);
    let mut importing = use_signal(|| false);
    let mut permission = use_signal(String::new);
    let mut when_closed = use_signal(|| None::<bool>);
    let mut diag_on = use_signal(|| false);
    let mut upcoming = use_signal(Vec::<PlannedReminder>::new);
    let mut message = use_signal(|| None::<String>);
    let mut open: OpenPop = use_signal(|| None);

    let refresh = move || {
        spawn(async move {
            if let Ok(s) = api::get_reminder_settings().await {
                settings.set(Some(s));
            }
            if let Ok(p) = api::notification_permission().await {
                permission.set(p);
            }
            if let Ok(c) = api::reminders_when_closed().await {
                when_closed.set(Some(c));
            }
            if let Ok(d) = api::diag_enabled().await {
                diag_on.set(d);
            }
            upcoming.set(api::upcoming_reminders().await.unwrap_or_default());
        });
    };
    use_hook(move || refresh());

    // Saves a changed copy, then reloads the preview.
    let mut save = move |next: ReminderSettings| {
        settings.set(Some(next.clone()));
        spawn(async move {
            if let Err(e) = api::set_reminder_settings(&next).await {
                message.set(Some(e));
            }
            upcoming.set(api::upcoming_reminders().await.unwrap_or_default());
        });
    };

    let Some(s) = settings() else {
        return rsx! {
            div { class: "modal-scrim", onclick: move |_| on_close.call(()),
                div { class: "modal", onclick: move |e| e.stop_propagation(), p { class: "view-sub", "Loading…" } }
            }
        };
    };

    let toggle = {
        let s = s.clone();
        move |_| {
            let s = s.clone();
            message.set(None);
            if s.enabled {
                save(ReminderSettings { enabled: false, ..s });
                return;
            }
            // Turning on asks the OS for permission first.
            spawn(async move {
                let state = if permission() == "granted" {
                    "granted".to_string()
                } else {
                    api::request_notification_permission().await.unwrap_or_else(|_| "denied".to_string())
                };
                permission.set(state.clone());
                if state == "granted" {
                    save(ReminderSettings { enabled: true, ..s });
                } else {
                    message.set(Some(
                        "Notifications are blocked. Allow them for Keel in your system settings, then try again."
                            .to_string(),
                    ));
                }
            });
        }
    };

    let offsets = [0i64, 5, 10, 15, 30, 60];
    let offset_items: Vec<MenuItem> = offsets.iter().map(|m| MenuItem::new(m.to_string(), offset_label(*m))).collect();
    let all_day_items = time_items("No reminder", &[hm(7, 0), hm(8, 0), hm(9, 0), hm(10, 0), hm(12, 0), hm(18, 0)]);
    let summary_items = time_items("Off", &[hm(6, 30), hm(7, 0), hm(7, 30), hm(8, 0), hm(9, 0)]);
    let all_day_text = s.all_day_time.map(views::time_label).unwrap_or_else(|| "No reminder".to_string());
    let summary_text = s.daily_summary.map(views::time_label).unwrap_or_else(|| "Off".to_string());
    let (s1, s2, s3) = (s.clone(), s.clone(), s.clone());
    let enabled = s.enabled;

    rsx! {
        div {
            class: "modal-scrim",
            onclick: move |_| on_close.call(()),
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    if open().is_some() { open.set(None) } else { on_close.call(()) }
                }
            },
            div {
                class: "modal",
                tabindex: "-1",
                onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                onclick: move |e| e.stop_propagation(),
                if open().is_some() {
                    div { class: "pop-backdrop", onclick: move |_| open.set(None) }
                }
                div { class: "modal-head",
                    h2 { "Settings" }
                    button { class: "icon-btn", title: "Close", onclick: move |_| on_close.call(()), "×" }
                }
                h3 { class: "modal-section", "Notifications" }
                div { class: "setting-row",
                    div { class: "setting-text",
                        div { class: "setting-name", "Task reminders" }
                        div { class: "setting-hint",
                            match permission().as_str() {
                                "denied" => "Blocked by the system",
                                _ if enabled => "On",
                                _ => "Off",
                            }
                        }
                    }
                    button {
                        class: if enabled { "switch on" } else { "switch" },
                        role: "switch",
                        aria_checked: "{enabled}",
                        title: "Task reminders",
                        onclick: toggle,
                        span { class: "knob" }
                    }
                }
                if let Some(m) = message() {
                    div { class: "setting-warning", "{m}" }
                }
                div { class: if enabled { "setting-group" } else { "setting-group disabled" },
                    div { class: "setting-row",
                        div { class: "setting-name", "Remind me" }
                        Menu {
                            id: "offset",
                            title: "Remind me",
                            icon: icons::clock_small(),
                            text: offset_label(s.offset_minutes),
                            items: offset_items,
                            selected: s.offset_minutes.to_string(),
                            open,
                            on_select: move |v: String| save(ReminderSettings { offset_minutes: v.parse().unwrap_or(0), ..s1.clone() }),
                        }
                    }
                    div { class: "setting-row",
                        div { class: "setting-name", "Tasks without a time" }
                        Menu {
                            id: "allday",
                            title: "Tasks without a time",
                            icon: icons::calendar_small(),
                            text: all_day_text,
                            items: all_day_items,
                            selected: time_value(s.all_day_time),
                            open,
                            on_select: move |v: String| save(ReminderSettings { all_day_time: parse_time_value(&v), ..s2.clone() }),
                        }
                    }
                    div { class: "setting-row",
                        div { class: "setting-text",
                            div { class: "setting-name", "Daily summary" }
                            div { class: "setting-hint", "How many tasks are due that day" }
                        }
                        Menu {
                            id: "summary",
                            title: "Daily summary",
                            icon: icons::sun(),
                            text: summary_text,
                            items: summary_items,
                            selected: time_value(s.daily_summary),
                            open,
                            on_select: move |v: String| save(ReminderSettings { daily_summary: parse_time_value(&v), ..s3.clone() }),
                        }
                    }
                    div { class: "setting-row",
                        div { class: "setting-name", "Coming up" }
                        button {
                            class: "btn btn-cancel",
                            onclick: move |_| {
                                spawn(async move {
                                    if let Err(e) = api::test_notification().await {
                                        message.set(Some(e));
                                    }
                                });
                            },
                            "Send test notification"
                        }
                    }
                    if upcoming().is_empty() {
                        div { class: "setting-hint upcoming-empty",
                            if enabled { "No reminders in the next two weeks." } else { "Turn on reminders to see what's coming up." }
                        }
                    } else {
                        ul { class: "upcoming-list",
                            for r in upcoming().iter() {
                                li { key: "{r.id}",
                                    span { class: "upcoming-when", "{when_text(r)}" }
                                    span { class: "upcoming-title",
                                        if r.task_id.is_some() { "{r.title}" } else { "Daily summary · {r.body}" }
                                    }
                                }
                            }
                        }
                    }
                }
                h3 { class: "modal-section", "Appearance" }
                CompletionSetting {}
                h3 { class: "modal-section", "Diagnostics" }
                div { class: "setting-row",
                    div { class: "setting-text",
                        div { class: "setting-name", "Diagnostics logging" }
                        div { class: "setting-hint", "Writes a log for bug reports. Off by default." }
                    }
                    button {
                        class: if diag_on() { "switch on" } else { "switch" },
                        role: "switch",
                        aria_checked: "{diag_on()}",
                        title: "Diagnostics logging",
                        onclick: move |_| {
                            let next = !diag_on();
                            diag_on.set(next);
                            spawn(async move {
                                if api::set_diag_enabled(next).await.is_ok() {
                                    crate::diag::set_active(next);
                                } else {
                                    diag_on.set(!next);
                                }
                            });
                        },
                        span { class: "knob" }
                    }
                }
                h3 { class: "modal-section", "Import" }
                div { class: "setting-row",
                    div { class: "setting-text",
                        div { class: "setting-name", "From Todoist" }
                        div { class: "setting-hint",
                            "In Todoist, open a project → ⋯ → Manage data → Export as CSV. Each file becomes a new project."
                        }
                    }
                    label { class: if importing() { "btn btn-cancel file-btn busy" } else { "btn btn-cancel file-btn" },
                        if importing() { "Importing…" } else { "Choose CSV files" }
                        input {
                            r#type: "file",
                            accept: ".csv,text/csv",
                            multiple: true,
                            disabled: importing(),
                            onchange: move |evt| {
                                let Some(files) = evt.files() else { return };
                                importing.set(true);
                                spawn(async move {
                                    for name in files.files() {
                                        let project = name
                                            .rsplit(['/', '\\'])
                                            .next()
                                            .unwrap_or(&name)
                                            .trim_end_matches(".csv")
                                            .trim_end_matches(".CSV")
                                            .to_string();
                                        let line = match files.read_file_to_string(&name).await {
                                            None => (false, format!("{project}: couldn't read the file")),
                                            Some(text) => match api::import_todoist_csv(&project, &text).await {
                                                Ok(s) => (true, import_line(&s)),
                                                Err(e) => (false, format!("{project}: {e}")),
                                            },
                                        };
                                        imports.with_mut(|l| l.push(line));
                                    }
                                    importing.set(false);
                                    reload += 1;
                                });
                            },
                        }
                    }
                }
                if !imports().is_empty() {
                    ul { class: "import-results",
                        for (i, (ok, text)) in imports().iter().enumerate() {
                            li { key: "{i}", class: if *ok { "ok" } else { "bad" },
                                Icon { svg: if *ok { icons::check_small() } else { icons::no_date() } }
                                span { "{text}" }
                            }
                        }
                    }
                }
                if let Some(closed) = when_closed() {
                    p { class: "modal-foot",
                        Icon { svg: if closed { icons::bell_small() } else { icons::monitor() } }
                        "{delivery_note(closed)}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_note_describes_only_this_device() {
        let phone = delivery_note(true);
        assert!(phone.contains("closed") && !phone.contains("computer"), "{phone}");
        let desktop = delivery_note(false);
        assert!(desktop.contains("open") && !desktop.contains("iPhone"), "{desktop}");
    }
}
