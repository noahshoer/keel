mod editor;
mod filters;
mod gestures;
mod glass;
mod api;
mod icons;
mod pickers;
mod search;
mod settings;
mod sidebar;
mod style;
mod task_row;
mod theme;

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::filter::FilterGroup;
use keel_core::model::{Filter, Label, Project, Reminder, Section, Task};
use keel_core::views;

use editor::{AddTask, EditorValues};
use search::SearchPane;
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
    /// id, name, query
    Filter(Id, String, String),
    Search,
}

/// Sends `true` to Rust on "/" (outside text fields) or Ctrl/Cmd+K, then
/// focuses the search box once the Search view has rendered.
const SEARCH_SHORTCUT_JS: &str = r#"
document.addEventListener('keydown', (ev) => {
  const el = document.activeElement;
  const typing = el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable);
  const slash = ev.key === '/' && !typing && !ev.ctrlKey && !ev.metaKey && !ev.altKey;
  const ctrlK = (ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === 'k';
  if (slash || ctrlK) {
    ev.preventDefault();
    dioxus.send(true);
    setTimeout(() => document.querySelector('.search-input')?.focus(), 60);
  }
});
await new Promise(() => {});
"#;

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
    /// The view this data was loaded for. The pane shows this, not the
    /// view just picked, so a switch swaps title and tasks together instead
    /// of flashing the new title over the old (empty-looking) task list.
    view: View,
    projects: Vec<Project>,
    /// Sections of every project (for grouping and the editor's location picker).
    sections: Vec<Section>,
    labels: Vec<Label>,
    filters: Vec<Filter>,
    inbox: Project,
    today_count: usize,
    tasks: Vec<Task>,
    /// Empty unless "Show completed" is on for the current view.
    completed: Vec<Task>,
    /// Result lists for a filter view (one per comma group).
    filter_groups: Vec<FilterGroup>,
    /// Why a saved filter couldn't run (shown in the pane).
    filter_error: Option<String>,
}

/// A titled run of tasks in the date-based views.
#[derive(Clone, PartialEq)]
struct Group {
    title: Option<String>,
    muted: String,
    tasks: Vec<Task>,
}

/// Splits Today / Upcoming into Todoist-style date groups.
fn date_groups(view: &View, tasks: Vec<Task>, today: NaiveDate) -> Vec<Group> {
    let (overdue, rest): (Vec<Task>, Vec<Task>) = tasks
        .into_iter()
        .partition(|t| views::due_date_of(t).is_some_and(|d| d < today));
    let mut out = Vec::new();
    let split = !overdue.is_empty();
    if split {
        out.push(Group { title: Some("Overdue".into()), muted: String::new(), tasks: overdue });
    }
    match view {
        View::Today => {
            if !rest.is_empty() {
                out.push(Group {
                    title: split.then(|| "Today".to_string()),
                    muted: if split { format!("{} · {}", short_date(today), weekday(today)) } else { String::new() },
                    tasks: rest,
                });
            }
        }
        _ => {
            let mut days: Vec<(NaiveDate, Vec<Task>)> = Vec::new();
            for t in rest {
                let Some(d) = views::due_date_of(&t) else { continue };
                match days.last_mut() {
                    Some((last, group)) if *last == d => group.push(t),
                    _ => days.push((d, vec![t])),
                }
            }
            out.extend(days.into_iter().map(|(d, tasks)| {
                let rel = match (d - today).num_days() {
                    0 => "Today · ",
                    1 => "Tomorrow · ",
                    _ => "",
                };
                Group { title: Some(short_date(d)), muted: format!("{rel}{}", weekday(d)), tasks }
            }));
        }
    }
    out
}

#[component]
fn App() -> Element {
    let mut view = use_signal(|| View::Today);
    let mut last_view = use_signal(|| View::Today);
    // Kept across visits to the Search view.
    let search_query = use_signal(String::new);
    let reload = use_signal(|| 0u32);
    let theme = use_signal(theme::saved);
    let show_completed = use_signal(|| false);
    // Task reminders, read by every task row (refreshed with each load).
    let mut reminders = use_context_provider(|| Signal::new(Vec::<Reminder>::new()));
    pickers::provide_touch();
    // Narrow layouts: the sidebar is a slide-over drawer (CSS ignores this
    // on wide windows, where the sidebar is always shown).
    let mut drawer = use_signal(|| false);
    use_hook(|| theme::apply(&theme.peek()));
    // Forms here are handled in Rust; never let the browser submit one (a
    // native submit reloads the page and cuts off work still in flight).
    use_hook(|| {
        let _ = document::eval("document.addEventListener('submit', (e) => e.preventDefault(), true);");
    });
    // "/" (when not typing) or Ctrl/Cmd+K opens Search and focuses the box.
    use_hook(move || {
        spawn(async move {
            let mut keys = document::eval(SEARCH_SHORTCUT_JS);
            while keys.recv::<bool>().await.is_ok() {
                view.set(View::Search);
            }
        });
    });
    // iPhone: the native glass card and floating buttons.
    let glass = glass::provide(move |action| match action {
        glass::ButtonAction::OpenMenu => drawer.set(true),
        // In Search the button closes it, back to where you were.
        glass::ButtonAction::Search => {
            let next = if *view.peek() == View::Search { last_view.peek().clone() } else { View::Search };
            view.set(next);
        }
        glass::ButtonAction::AddTask => {}
    });
    // The floating buttons step aside while the drawer is open.
    use_effect(move || {
        let state = glass::floating_buttons(view() == View::Search, drawer());
        if (glass.native)() {
            spawn(async move { api::glass_buttons(state).await });
        }
    });
    // Where closing Search goes back to.
    use_effect(move || {
        let v = view();
        if v != View::Search {
            last_view.set(v);
        }
    });
    // Touch: swipe right opens the drawer, swipe left closes it.
    use_hook(move || {
        spawn(async move {
            let _ = document::eval(gestures::LONG_PRESS_JS);
            let mut swipes = document::eval(gestures::SWIPE_JS);
            while let Ok(s) = swipes.recv::<gestures::Swipe>().await {
                let open_now = *drawer.peek();
                if let Some(open) = gestures::drawer_swipe(&s, open_now) {
                    drawer.set(open);
                }
            }
        });
    });
    // Picking a view closes the drawer.
    use_effect(move || {
        let _ = view();
        drawer.set(false);
    });

    let data = use_resource(move || async move {
        let _ = reload();
        let v = view();
        let projects = api::list_projects().await?;
        let inbox = projects
            .iter()
            .find(|p| p.is_inbox)
            .cloned()
            .ok_or_else(|| "no inbox project".to_string())?;
        let mut sections = Vec::new();
        for p in &projects {
            sections.extend(api::list_sections(p.id.as_str()).await?);
        }
        let labels = api::list_labels().await?;
        reminders.set(api::list_reminders().await?);
        let filters = api::list_filters().await?;
        let today_tasks = api::today_view().await?;
        let today_count = today_tasks.len();
        let mut filter_groups = Vec::new();
        let mut filter_error = None;
        let tasks = match &v {
            View::Filter(_, _, query) => match api::filter_view(query).await {
                Ok(groups) => {
                    let tasks = groups.iter().flat_map(|g| g.tasks.clone()).collect();
                    filter_groups = groups;
                    tasks
                }
                Err(e) => {
                    filter_error = Some(e);
                    Vec::new()
                }
            },
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
            // The Search view loads its own results.
            View::Search => Vec::new(),
        };
        // Completed tasks are hidden unless the toggle is on.
        let completed = if show_completed() {
            match &v {
                View::Today => api::completed_today().await?,
                View::Inbox => api::completed_in_project(inbox.id.as_str()).await?,
                View::Project(id, _) => api::completed_in_project(id.as_str()).await?,
                View::Label(name) => api::completed_with_label(name).await?,
                View::Upcoming | View::Filter(..) | View::Search => Vec::new(),
            }
        } else {
            Vec::new()
        };
        Ok::<Loaded, String>(Loaded {
            view: v,
            projects,
            sections,
            labels,
            filters,
            inbox,
            today_count,
            tasks,
            completed,
            filter_groups,
            filter_error,
        })
    });

    let today = today();
    let state = (*data.read()).clone();

    rsx! {
        style { dangerous_inner_html: style::css() }
        div { class: if drawer() { "app drawer-open" } else { "app" },
            if drawer() {
                div { class: "drawer-scrim", onclick: move |_| drawer.set(false) }
            }
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
                        filters: loaded.filters.clone(),
                        today_count: loaded.today_count,
                        today_day: today.day(),
                        theme,
                        reload,
                    }
                    MainPane { view: loaded.view.clone(), loaded, today, show_completed, drawer, search_query, reload }
                },
            }
        }
    }
}

/// What a task row shows on the right: "Project" or "Project / Section".
fn location_names(loaded: &Loaded) -> HashMap<Id, String> {
    let projects: HashMap<&Id, &str> = loaded.projects.iter().map(|p| (&p.id, p.name.as_str())).collect();
    let sections: HashMap<&Id, &str> = loaded.sections.iter().map(|s| (&s.id, s.name.as_str())).collect();
    let all = loaded.tasks.iter().chain(loaded.completed.iter());
    all.map(|t| {
        let project = projects.get(&t.project_id).copied().unwrap_or("");
        let label = match t.section_id.as_ref().and_then(|s| sections.get(s)) {
            Some(section) => format!("{project} / {section}"),
            None => project.to_string(),
        };
        (t.id.clone(), label)
    })
    .collect()
}

#[component]
fn MainPane(
    view: View,
    loaded: Loaded,
    today: NaiveDate,
    show_completed: Signal<bool>,
    drawer: Signal<bool>,
    search_query: Signal<String>,
    reload: Signal<u32>,
) -> Element {
    let mut show_completed = show_completed;
    let mut drawer = drawer;
    // Upcoming and filters have no completed list (as in Todoist).
    let can_show_completed = !matches!(view, View::Upcoming | View::Filter(..));
    let toggle_text = if show_completed() { "Hide completed" } else { "Show completed" };
    let names = location_names(&loaded);
    let show_project = matches!(view, View::Today | View::Upcoming | View::Label(_) | View::Filter(..));
    let title = match &view {
        View::Today => "Today".to_string(),
        View::Upcoming => "Upcoming".to_string(),
        View::Inbox => "Inbox".to_string(),
        View::Project(_, name) | View::Label(name) | View::Filter(_, name, _) => name.clone(),
        View::Search => "Search".to_string(),
    };
    let subtitle = match &view {
        View::Today => format!("{} {}", weekday(today), short_date(today)),
        View::Upcoming => format!("Next {UPCOMING_DAYS} days"),
        View::Filter(_, _, query) => query.clone(),
        _ => String::new(),
    };
    let filter_view = matches!(view, View::Filter(..));
    let search_view = view == View::Search;
    let multi_group = loaded.filter_groups.len() > 1;
    // The project a project-style view shows (Inbox or a project).
    let project_id = match &view {
        View::Project(id, _) => Some(id.clone()),
        View::Inbox => Some(loaded.inbox.id.clone()),
        _ => None,
    };
    let add_date = match &view {
        View::Today | View::Upcoming => today.format("%Y-%m-%d").to_string(),
        _ => String::new(),
    };
    let add_labels = match &view {
        View::Label(name) => vec![name.clone()],
        _ => Vec::new(),
    };
    let defaults = EditorValues::blank(
        project_id.clone().unwrap_or_else(|| loaded.inbox.id.clone()),
        None,
        add_date,
        add_labels,
    );
    let view_key = format!("{view:?}");
    let empty = loaded.tasks.is_empty();
    let empty_text = match &view {
        View::Today => ("You're all done for today!", "Enjoy the rest of your day."),
        View::Upcoming => ("Nothing scheduled.", "Tasks with dates in the next two weeks show up here."),
        View::Filter(..) => ("No tasks match this filter.", "Edit it from the sidebar to widen the query."),
        _ => ("No tasks yet.", "Add one below."),
    };

    // Project views: unsectioned tasks first, then each section's tasks.
    let project_sections: Vec<Section> = match &project_id {
        Some(pid) => loaded.sections.iter().filter(|s| &s.project_id == pid).cloned().collect(),
        None => Vec::new(),
    };
    let in_section = |t: &Task, s: &Section| t.section_id.as_ref() == Some(&s.id);
    let unsectioned: Vec<Task> = loaded
        .tasks
        .iter()
        .filter(|t| !project_sections.iter().any(|s| in_section(t, s)))
        .cloned()
        .collect();
    let date_view = matches!(view, View::Today | View::Upcoming);
    let groups = if date_view { date_groups(&view, loaded.tasks.clone(), today) } else { Vec::new() };

    rsx! {
        main { class: "main",
            div { key: "{view_key}", class: "content",
                div { class: "view-head",
                    button {
                        class: "icon-btn menu-btn",
                        title: "Open menu",
                        aria_label: "Open menu",
                        onclick: move |_| drawer.set(true),
                        Icon { svg: icons::menu() }
                    }
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
                    p { class: if filter_view { "view-sub mono" } else { "view-sub" }, "{subtitle}" }
                }
                if let Some(problem) = loaded.filter_error.clone() {
                    div { class: "filter-problem big-problem", "This filter can’t run: {problem}" }
                } else if empty && project_sections.is_empty() && !search_view {
                    div { class: "empty",
                        div { class: "big", "{empty_text.0}" }
                        div { "{empty_text.1}" }
                    }
                }
                if search_view {
                    SearchPane {
                        query: search_query,
                        today,
                        show_completed: show_completed(),
                        projects: loaded.projects.clone(),
                        sections: loaded.sections.clone(),
                        labels: loaded.labels.clone(),
                        reload,
                    }
                } else if filter_view {
                    for (i, group) in loaded.filter_groups.iter().enumerate() {
                        div { key: "fg-{i}",
                            if multi_group {
                                div { class: "section-title",
                                    span { class: "mono", "{group.query}" }
                                    span { class: "muted", "{group.tasks.len()}" }
                                }
                            }
                            TaskList {
                                tasks: group.tasks.clone(),
                                nested: false,
                                today,
                                names: names.clone(),
                                show_project,
                                projects: loaded.projects.clone(),
                                sections: loaded.sections.clone(),
                                labels: loaded.labels.clone(),
                                reload,
                            }
                        }
                    }
                    AddTask {
                        key: "{view_key}",
                        main: true,
                        defaults: defaults.clone(),
                        projects: loaded.projects.clone(),
                        sections: loaded.sections.clone(),
                        labels: loaded.labels.clone(),
                        reload,
                    }
                } else if date_view {
                    for (i, group) in groups.into_iter().enumerate() {
                        div { key: "{i}",
                            if let Some(t) = group.title.clone() {
                                div { class: "section-title",
                                    "{t}"
                                    span { class: "muted", "{group.muted}" }
                                }
                            }
                            TaskList {
                                tasks: group.tasks.clone(),
                                nested: false,
                                today,
                                names: names.clone(),
                                show_project,
                                projects: loaded.projects.clone(),
                                sections: loaded.sections.clone(),
                                labels: loaded.labels.clone(),
                                reload,
                            }
                        }
                    }
                    AddTask {
                        key: "{view_key}",
                        main: true,
                        defaults: defaults.clone(),
                        projects: loaded.projects.clone(),
                        sections: loaded.sections.clone(),
                        labels: loaded.labels.clone(),
                        reload,
                    }
                } else {
                    TaskList {
                        tasks: unsectioned,
                        nested: true,
                        today,
                        names: names.clone(),
                        show_project,
                        projects: loaded.projects.clone(),
                        sections: loaded.sections.clone(),
                        labels: loaded.labels.clone(),
                        reload,
                    }
                    AddTask {
                        key: "{view_key}",
                        main: true,
                        defaults: defaults.clone(),
                        projects: loaded.projects.clone(),
                        sections: loaded.sections.clone(),
                        labels: loaded.labels.clone(),
                        reload,
                    }
                    for section in project_sections.iter() {
                        SectionBlock {
                            key: "{section.id}",
                            section: section.clone(),
                            tasks: loaded.tasks.iter().filter(|t| in_section(t, section)).cloned().collect::<Vec<Task>>(),
                            today,
                            names: names.clone(),
                            projects: loaded.projects.clone(),
                            sections: loaded.sections.clone(),
                            labels: loaded.labels.clone(),
                            reload,
                        }
                    }
                    if let Some(pid) = project_id.clone() {
                        AddSection { key: "add-section-{pid}", project_id: pid, reload }
                    }
                }
                if show_completed() && can_show_completed && !search_view {
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
                            project_name: if show_project { names.get(&task.id).cloned() } else { None },
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

/// Task rows; with `nested`, sub-tasks indent under their parents and
/// collapsed parents hide them.
#[component]
fn TaskList(
    tasks: Vec<Task>,
    nested: bool,
    today: NaiveDate,
    names: HashMap<Id, String>,
    show_project: bool,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    reload: Signal<u32>,
) -> Element {
    let rows: Vec<views::Nested> = if nested {
        views::nest(tasks)
    } else {
        tasks
            .into_iter()
            .map(|task| views::Nested { task, depth: 0, has_children: false })
            .collect()
    };
    rsx! {
        for row in rows {
            TaskRow {
                key: "{row.task.id}",
                project_name: if show_project { names.get(&row.task.id).cloned() } else { None },
                depth: row.depth,
                has_children: row.has_children,
                task: row.task.clone(),
                today,
                projects: projects.clone(),
                sections: sections.clone(),
                labels: labels.clone(),
                reload,
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum HeadMode {
    Normal,
    Renaming,
    ConfirmDelete,
}

/// A project section: header (rename / delete on hover), its tasks, and its
/// own "+ Add task".
#[component]
fn SectionBlock(
    section: Section,
    tasks: Vec<Task>,
    today: NaiveDate,
    names: HashMap<Id, String>,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    reload: Signal<u32>,
) -> Element {
    let mut reload = reload;
    let mut mode = use_signal(|| HeadMode::Normal);
    let mut draft = use_signal(String::new);
    let count = tasks.len();
    let name = section.name.clone();
    let id_rename = section.id.clone();
    let id_delete = section.id.clone();
    let defaults = EditorValues::blank(section.project_id.clone(), Some(section.id.clone()), String::new(), Vec::new());
    let task_word = if count == 1 { "task" } else { "tasks" };

    rsx! {
        div { class: "section-block",
            match mode() {
                HeadMode::Renaming => rsx! {
                    form {
                        class: "section-rename",
                        onsubmit: move |e| {
                            e.prevent_default();
                            let n = draft().trim().to_string();
                            mode.set(HeadMode::Normal);
                            if n.is_empty() {
                                return;
                            }
                            let id = id_rename.clone();
                            spawn(async move {
                                let _ = api::rename_section(id, n).await;
                                reload += 1;
                            });
                        },
                        input {
                            class: "project-input",
                            autofocus: true,
                            value: "{draft}",
                            oninput: move |e| draft.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Escape {
                                    mode.set(HeadMode::Normal);
                                }
                            },
                        }
                    }
                },
                HeadMode::ConfirmDelete => rsx! {
                    div { class: "confirm-box",
                        div { class: "confirm-text", "Delete section “{name}” and its {count} {task_word}?" }
                        div { class: "confirm-actions",
                            button { class: "btn btn-cancel", onclick: move |_| mode.set(HeadMode::Normal), "Cancel" }
                            button {
                                class: "btn btn-add",
                                onclick: move |_| {
                                    mode.set(HeadMode::Normal);
                                    let id = id_delete.clone();
                                    spawn(async move {
                                        let _ = api::delete_section(id).await;
                                        reload += 1;
                                    });
                                },
                                "Delete"
                            }
                        }
                    }
                },
                HeadMode::Normal => rsx! {
                    div { class: "section-title section-row",
                        "{name}"
                        span { class: "muted", "{count}" }
                        span { class: "section-actions",
                            button {
                                class: "icon-btn",
                                title: "Rename section",
                                onclick: move |_| {
                                    draft.set(section.name.clone());
                                    mode.set(HeadMode::Renaming);
                                },
                                Icon { svg: icons::pencil() }
                            }
                            button {
                                class: "icon-btn",
                                title: "Delete section",
                                onclick: move |_| mode.set(HeadMode::ConfirmDelete),
                                Icon { svg: icons::trash_small() }
                            }
                        }
                    }
                },
            }
            TaskList {
                tasks,
                nested: true,
                today,
                names,
                show_project: false,
                projects: projects.clone(),
                sections: sections.clone(),
                labels: labels.clone(),
                reload,
            }
            AddTask { defaults, projects, sections, labels, reload }
        }
    }
}

/// "+ Add section" at the bottom of a project.
#[component]
fn AddSection(project_id: Id, reload: Signal<u32>) -> Element {
    let mut reload = reload;
    let mut open = use_signal(|| false);
    let mut name = use_signal(String::new);
    if !open() {
        return rsx! {
            div { class: "add-section-row", onclick: move |_| open.set(true),
                span { class: "add-section-line" }
                span { class: "add-section-label", "Add section" }
                span { class: "add-section-line" }
            }
        };
    }
    rsx! {
        form {
            class: "section-rename",
            onsubmit: move |e| {
                e.prevent_default();
                let n = name().trim().to_string();
                if n.is_empty() {
                    return;
                }
                let pid = project_id.clone();
                spawn(async move {
                    let _ = api::add_section(pid, n).await;
                    name.set(String::new());
                    open.set(false);
                    reload += 1;
                });
            },
            input {
                class: "project-input",
                placeholder: "Name this section, then Enter",
                autofocus: true,
                value: "{name}",
                oninput: move |e| name.set(e.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Escape {
                        open.set(false);
                    }
                },
            }
        }
    }
}
