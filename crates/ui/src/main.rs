mod add_task;
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
use todo_core::model::{Project, Task};
use todo_core::views;

use add_task::AddTask;
use sidebar::Sidebar;
use task_row::TaskRow;

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
    inbox: Project,
    today_count: usize,
    tasks: Vec<Task>,
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
            let mut out: Vec<(NaiveDate, Vec<Task>)> = Vec::new();
            for t in tasks {
                let Some(d) = views::due_date_of(&t) else { continue };
                match out.last_mut() {
                    Some((last, group)) if *last == d => group.push(t),
                    _ => out.push((d, vec![t])),
                }
            }
            out.into_iter()
                .map(|(d, tasks)| {
                    let rel = match (d - today).num_days() {
                        0 => "Today · ",
                        1 => "Tomorrow · ",
                        _ => "",
                    };
                    Section { title: Some(short_date(d)), muted: format!("{rel}{}", weekday(d)), tasks }
                })
                .collect()
        }
        View::Inbox | View::Project(..) => {
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
        let today_tasks = api::today_view().await?;
        let today_count = today_tasks.len();
        let tasks = match &v {
            View::Today => today_tasks,
            View::Upcoming => api::upcoming_view(UPCOMING_DAYS).await?,
            View::Inbox => api::project_view(inbox.id.as_str()).await?,
            View::Project(id, _) => api::project_view(id.as_str()).await?,
        };
        Ok::<Loaded, String>(Loaded { projects, inbox, today_count, tasks })
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
                        today_count: loaded.today_count,
                        today_day: today.day(),
                        theme,
                        reload,
                    }
                    MainPane { view: current, loaded, today, reload }
                },
            }
        }
    }
}

#[component]
fn MainPane(view: View, loaded: Loaded, today: NaiveDate, reload: Signal<u32>) -> Element {
    let names: HashMap<Id, String> =
        loaded.projects.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
    let show_project = matches!(view, View::Today | View::Upcoming);
    let title = match &view {
        View::Today => "Today".to_string(),
        View::Upcoming => "Upcoming".to_string(),
        View::Inbox => "Inbox".to_string(),
        View::Project(_, name) => name.clone(),
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
                h1 { class: "view-title", "{title}" }
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
                                reload,
                            }
                        }
                    }
                }
                AddTask { key: "{view_key}", project_id: add_project, default_date: add_date, reload }
            }
        }
    }
}
