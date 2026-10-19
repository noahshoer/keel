mod editor;
mod api;
mod icons;
mod sidebar;
mod style;
mod task_row;
mod theme;

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use todo_core::ids::Id;
use todo_core::model::{Label, Project, Task};
use todo_core::views;

use editor::{AddTask, EditorValues};
use icons::Icon;
use sidebar::Sidebar;
use task_row::{CompletedRow, TaskRow};

const UPCOMING_DAYS: i64 = 14;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: [&str; 7] = [
    "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
];

/// Which list the main pane shows.
#[derive(Clone, PartialEq, Debug)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Project(Id, String),
    Label(String),
}

/// The local date, from JS (no `SystemTime` on wasm32).
pub fn today() -> NaiveDate {
    let d = js_sys::Date::new_0();
    NaiveDate::from_ymd_opt(d.get_full_year() as i32, d.get_month() + 1, d.get_date())
        .expect("JS date is valid")
}

fn short_date(d: NaiveDate) -> String {
    format!("{} {}", d.day(), MONTHS[d.month0() as usize])
}

fn weekday(d: NaiveDate) -> &'static str {
    WEEKDAYS[d.weekday().num_days_from_monday() as usize]
}

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, PartialEq)]
struct Loaded {
    projects: Vec<Project>,
    labels: Vec<Label>,
    inbox: Project,
    today_count: usize,
    tasks: Vec<Task>,
    /// Empty unless "Show completed" is on for the current view.
    completed: Vec<Task>,
}

#[derive(Clone, PartialEq)]
struct Section {
    title: Option<String>,
    muted: String,
    tasks: Vec<Task>,
}

/// Splits a view's tasks into Todoist-style sections.
fn sections(view: &View, tasks: Vec<Task>, today: NaiveDate) -> Vec<Section> {
    match view {
        View::Today => {
            let (overdue, due_today): (Vec<Task>, Vec<Task>) = tasks
                .into_iter()
                .partition(|t| views::due_date_of(t).is_some_and(|d| d < today));
            let mut out = Vec::new();
            let split = !overdue.is_empty();
            if split {
                out.push(Section { title: Some("Overdue".into()), muted: String::new(), tasks: overdue });
            }
            if !due_today.is_empty() {
                out.push(Section {
                    title: split.then(|| "Today".to_string()),
                    muted: if split { format!("{} · {}", short_date(today), weekday(today)) } else { String::new() },
                    tasks: due_today,
                });
            }
            out
        }
        View::Upcoming => {
            // Overdue first (Todoist lists it at the top of Upcoming).
            let (overdue, upcoming): (Vec<Task>, Vec<Task>) = tasks
                .into_iter()
                .partition(|t| views::due_date_of(t).is_some_and(|d| d < today));
            let mut sections = Vec::new();
            if !overdue.is_empty() {
                sections.push(Section { title: Some("Overdue".into()), muted: String::new(), tasks: overdue });
            }
            let mut out: Vec<(NaiveDate, Vec<Task>)> = Vec::new();
            for t in upcoming {
                let Some(d) = views::due_date_of(&t) else { continue };
                match out.last_mut() {
                    Some((last, group)) if *last == d => group.push(t),
                    _ => out.push((d, vec![t])),
                }
            }
            sections.extend(out.into_iter().map(|(d, tasks)| {
                let rel = match (d - today).num_days() {
                    0 => "Today · ",
                    1 => "Tomorrow · ",
                    _ => "",
                };
                Section { title: Some(short_date(d)), muted: format!("{rel}{}", weekday(d)), tasks }
            }));
            sections
        }
        View::Inbox | View::Project(..) | View::Label(_) => {
            if tasks.is_empty() {
                Vec::new()
            } else {
                vec![Section { title: None, muted: String::new(), tasks }]
            }
        }
    }
}

#[component]
fn App() -> Element {
    let view = use_signal(|| View::Today);
    let reload = use_signal(|| 0u32);
    let theme = use_signal(theme::saved);
    let show_completed = use_signal(|| false);
    use_hook(|| theme::apply(&theme.peek()));

    let data = use_resource(move || async move {
        let _ = reload();
        let v = view();
        let projects = api::list_projects().await?;
        let inbox = projects
            .iter()
            .find(|p| p.is_inbox)
            .cloned()
            .ok_or_else(|| "no inbox project".to_string())?;
        let labels = api::list_labels().await?;
        let today_tasks = api::today_view().await?;
        let today_count = today_tasks.len();
        let tasks = match &v {
            View::Upcoming => {
                // Overdue (from the Today query) on top, then the window.
                let now = today();
                let mut tasks: Vec<Task> = today_tasks
                    .into_iter()
                    .filter(|t| views::due_date_of(t).is_some_and(|d| d < now))
                    .collect();
                tasks.extend(api::upcoming_view(UPCOMING_DAYS).await?);
                tasks
            }
            View::Today => today_tasks,
            View::Inbox => api::project_view(inbox.id.as_str()).await?,
            View::Project(id, _) => api::project_view(id.as_str()).await?,
            View::Label(name) => api::label_view(name).await?,
        };
        // Completed tasks are hidden unless this view's toggle is on.
        let completed = if show_completed() {
            match &v {
                View::Today => api::completed_today().await?,
                View::Inbox => api::completed_in_project(inbox.id.as_str()).await?,
                View::Project(id, _) => api::completed_in_project(id.as_str()).await?,
                View::Label(name) => api::completed_with_label(name).await?,
                View::Upcoming => Vec::new(),
            }
        } else {
            Vec::new()
        };
        Ok::<Loaded, String>(Loaded { projects, labels, inbox, today_count, tasks, completed })
    });

    let today = today();
    let current = view();
    let state = (*data.read()).clone();

    rsx! {
        style { dangerous_inner_html: style::css() }
        div { class: "app",
            match state {
                None => rsx! {
                    main { class: "main", div { class: "content", p { class: "view-sub", "Loading…" } } }
                },
                Some(Err(e)) => rsx! {
                    main { class: "main", div { class: "error", "Error: {e}" } }
                },
                Some(Ok(loaded)) => rsx! {
                    Sidebar {
                        view,
                        projects: loaded.projects.clone(),
                        labels: loaded.labels.clone(),
                        today_count: loaded.today_count,
                        today_day: today.day(),
                        theme,
                        reload,
                    }
                    MainPane { view: current, loaded, today, show_completed, reload }
                },
            }
        }
    }
}

#[component]
fn MainPane(
    view: View,
    loaded: Loaded,
    today: NaiveDate,
    show_completed: Signal<bool>,
    reload: Signal<u32>,
) -> Element {
    let mut show_completed = show_completed;
    // Upcoming has no completed list (Todoist hides that toggle there too).
    let can_show_completed = !matches!(view, View::Upcoming);
    let toggle_text = if show_completed() { "Hide completed" } else { "Show completed" };
    let names: HashMap<Id, String> =
        loaded.projects.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
    let show_project = matches!(view, View::Today | View::Upcoming | View::Label(_));
    let title = match &view {
        View::Today => "Today".to_string(),
        View::Upcoming => "Upcoming".to_string(),
        View::Inbox => "Inbox".to_string(),
        View::Project(_, name) | View::Label(name) => name.clone(),
    };
    let subtitle = match &view {
        View::Today => format!("{} {}", weekday(today), short_date(today)),
        View::Upcoming => format!("Next {UPCOMING_DAYS} days"),
        _ => String::new(),
    };
    let add_project = match &view {
        View::Project(id, _) => id.clone(),
        _ => loaded.inbox.id.clone(),
    };
    let add_date = match &view {
        View::Today | View::Upcoming => today.format("%Y-%m-%d").to_string(),
        _ => String::new(),
    };
    let add_labels = match &view {
        View::Label(name) => vec![name.clone()],
        _ => Vec::new(),
    };
    let defaults = EditorValues::blank(add_project, add_date, add_labels);
    let view_key = format!("{view:?}");
    let empty = loaded.tasks.is_empty();
    let secs = sections(&view, loaded.tasks.clone(), today);
    let empty_text = match &view {
        View::Today => ("You're all done for today!", "Enjoy the rest of your day."),
        View::Upcoming => ("Nothing scheduled.", "Tasks with dates in the next two weeks show up here."),
        _ => ("No tasks yet.", "Add one below."),
    };

    rsx! {
        main { class: "main",
            div { class: "content",
                div { class: "view-head",
                    h1 { class: "view-title", "{title}" }
                    if can_show_completed {
                        button {
                            class: "view-toggle",
                            title: "Completed tasks are hidden by default",
                            onclick: move |_| show_completed.set(!show_completed()),
                            Icon { svg: icons::check_circle() }
                            "{toggle_text}"
                        }
                    }
                }
                if !subtitle.is_empty() {
                    p { class: "view-sub", "{subtitle}" }
                }
                if empty {
                    div { class: "empty",
                        div { class: "big", "{empty_text.0}" }
                        div { "{empty_text.1}" }
                    }
                }
                for (i, section) in secs.into_iter().enumerate() {
                    div { key: "{i}",
                        if let Some(t) = section.title.clone() {
                            div { class: "section-title",
                                "{t}"
                                span { class: "muted", "{section.muted}" }
                            }
                        }
                        for task in section.tasks.clone() {
                            TaskRow {
                                key: "{task.id}",
                                project_name: if show_project { names.get(&task.project_id).cloned() } else { None },
                                task: task.clone(),
                                today,
                                projects: loaded.projects.clone(),
                                labels: loaded.labels.clone(),
                                reload,
                            }
                        }
                    }
                }
                AddTask {
                    key: "{view_key}",
                    defaults,
                    projects: loaded.projects.clone(),
                    labels: loaded.labels.clone(),
                    reload,
                }
                if show_completed() && can_show_completed {
                    div { class: "section-title",
                        "Completed"
                        span { class: "muted", "{loaded.completed.len()}" }
                    }
                    if loaded.completed.is_empty() {
                        p { class: "view-sub", "No completed tasks here yet." }
                    }
                    for task in loaded.completed.clone() {
                        CompletedRow {
                            key: "done-{task.id}",
                            project_name: if show_project { names.get(&task.project_id).cloned() } else { None },
                            task: task.clone(),
                            today,
                            reload,
                        }
                    }
                }
            }
        }
    }
}
