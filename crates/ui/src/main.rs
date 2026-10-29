mod diag;
mod editor;
mod filters;
mod gestures;
mod glass;
mod api;
mod cards;
mod completion;
mod icons;
mod pickers;
mod platform;
mod search;
mod settings;
mod sidebar;
mod style;
mod task_row;
mod task_view;
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
use cards::CardList;
use task_row::CompletedRow;
use views::CardMode;

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

/// How the page changes when the view does: Search slides in over the page
/// and back out, like an iOS push / pop; everything else fades.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Motion {
    Fade,
    Push,
    Pop,
}

/// How a view groups its tasks into cards: project views show top-level
/// tasks only; the others show every match under its top-level ancestor.
pub fn view_mode(view: &View) -> CardMode {
    match view {
        View::Inbox | View::Project(..) => CardMode::TopLevel,
        View::Today | View::Upcoming | View::Label(_) | View::Filter(..) | View::Search => CardMode::Grouped,
    }
}

pub fn view_motion(from: &View, to: &View) -> Motion {
    match (from == &View::Search, to == &View::Search) {
        (false, true) => Motion::Push,
        (true, false) => Motion::Pop,
        _ => Motion::Fade,
    }
}

/// The page left behind by a push / pop: `keelGhost(dir)` takes a still copy
/// of the page as the view changes (it's still showing then), laid over the
/// main pane — under it for a push, over it for a pop. `keelGhostGo()` sets
/// it moving once the new view lands, and it removes itself after.
const GHOST_JS: &str = r#"
window.keelGhost = (dir) => {
  document.querySelectorAll('.view-ghost').forEach((g) => g.remove());
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const app = document.querySelector('.app'), main = app?.querySelector('.main');
  const content = main?.querySelector('.content');
  if (!content) return;
  const a = app.getBoundingClientRect(), m = main.getBoundingClientRect(), c = content.getBoundingClientRect();
  const ghost = document.createElement('div');
  ghost.className = 'view-ghost ' + dir;
  ghost.style.cssText = `left:${m.left - a.left}px;top:${m.top - a.top}px;width:${m.width}px;height:${m.height}px;` +
    `border-radius:${getComputedStyle(main).borderRadius}`;
  const copy = content.cloneNode(true);
  copy.classList.remove('view-in-a', 'view-in-b', 'view-pop');
  copy.style.cssText = `position:absolute;margin:0;top:${c.top - m.top}px;left:${c.left - m.left}px;width:${c.width}px`;
  ghost.appendChild(copy);
  app.appendChild(ghost);
  // In case the new view never lands.
  setTimeout(() => ghost.remove(), 2000);
};
window.keelGhostGo = () => {
  document.querySelectorAll('.view-ghost:not(.go)').forEach((g) => {
    g.classList.add('go');
    g.addEventListener('animationend', () => g.remove());
    setTimeout(() => g.remove(), 1500);
  });
};
"#;

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
    /// Every non-deleted task, open and checked (for card heads and counts).
    all: Vec<Task>,
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
    // The task view's stack; rows open tasks into it.
    let mut nav = task_view::nav::provide_nav();
    pickers::provide_touch();
    pickers::provide_week_start();
    // The mark a completed circle fills with (a per-device setting).
    use_context_provider(|| Signal::new(completion::saved()));
    // Narrow layouts: the sidebar is a slide-over drawer (CSS ignores this
    // on wide windows, where the sidebar is always shown).
    let mut drawer = use_signal(|| false);
    use_hook(|| theme::apply(&theme.peek()));
    // Diagnostics logging (off unless the setting or KEEL_DIAG turns it on).
    use_hook(|| {
        spawn(diag::init());
    });
    // Quick add: Backspace right after a highlight un-highlights it.
    use_hook(|| {
        let _ = document::eval(editor::UNHIGHLIGHT_KEY_JS);
    });
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
        glass::ButtonAction::AddTask
        | glass::ButtonAction::Theme
        | glass::ButtonAction::Settings => {}
    });
    // Measure the drawer's header buttons each time it opens.
    use_effect(move || {
        if drawer() {
            glass.measure_drawer();
        }
    });
    // iPhone: the task sheet's glass X and ••• appear with it as it opens
    // (and ride its slide-up), and go when it closes.
    let mut sheet_was_open = use_signal(|| false);
    use_effect(move || {
        let open = !nav.stack.read().0.is_empty();
        if open == *sheet_was_open.peek() || !(glass.native)() {
            return;
        }
        sheet_was_open.set(open);
        if open {
            let _ = document::eval("window.keelSheetOpened?.();");
        } else {
            spawn(async move { api::glass_sheet_hide().await });
        }
    });
    // The floating buttons step aside while the drawer or the task view is open.
    use_effect(move || {
        let view_open = !nav.stack.read().0.is_empty();
        let mut state = glass::floating_buttons(view() == View::Search, drawer(), view_open);
        state.theme = theme();
        if drawer() && !(glass.settings_open)() {
            state.drawer = glass::drawer_buttons(&(glass.drawer_rects)(), &theme());
        }
        if (glass.native)() {
            spawn(async move { api::glass_buttons(state).await });
        }
    });
    // Search slides in and out: a still copy of the page being left is
    // taken as soon as the view changes (before the new one loads).
    use_hook(|| {
        let _ = document::eval(GHOST_JS);
    });
    let mut requested = use_signal(|| None::<View>);
    use_effect(move || {
        let v = view();
        let before = requested.peek().clone();
        requested.set(Some(v.clone()));
        let dir = match before.map(|b| view_motion(&b, &v)) {
            Some(Motion::Push) => "push",
            Some(Motion::Pop) => "pop",
            _ => return,
        };
        let _ = document::eval(&format!("window.keelGhost && keelGhost('{dir}');"));
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
    // Touch: the task sheet follows the finger between its two heights and
    // closes past a threshold.
    use_hook(move || {
        gestures::install_sheet_rules();
        spawn(async move {
            let mut drags = document::eval(gestures::SHEET_DRAG_JS);
            while let Ok(m) = drags.recv::<gestures::SheetMsg>().await {
                if m.close {
                    nav.ask_close();
                }
            }
        });
    });
    // Picking a view closes the drawer once that view's data has loaded:
    // the page swaps under the drawer, then the drawer slides away to reveal
    // it (closing first showed the old page snap to the new one).
    let mut closing_for = use_signal(|| None::<View>);
    use_effect(move || {
        let v = view();
        closing_for.set(Some(v));
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
        let all = api::all_tasks().await?;
        nav.all.set(all.clone());
        // A deleted task's view closes, and every view above it.
        let mut stack = nav.stack.peek().clone();
        stack.prune(&all);
        if stack != *nav.stack.peek() {
            nav.stack.set(stack);
        }
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
            all,
            completed,
            filter_groups,
            filter_error,
        })
    });

    // Each time a different view's data lands, the page fades in again
    // (`swaps` alternates the animation so it restarts).
    let mut swaps = use_signal(|| (None::<View>, 0u32, Motion::Fade));
    use_effect(move || {
        let loaded_view = match &*data.read() {
            Some(Ok(l)) => Some(l.view.clone()),
            Some(Err(_)) => closing_for.peek().clone(),
            None => None,
        };
        let Some(lv) = loaded_view else { return };
        if closing_for.peek().as_ref() == Some(&lv) {
            closing_for.set(None);
            drawer.set(false);
        }
        if swaps.peek().0.as_ref() != Some(&lv) {
            let (before, n, _) = swaps.peek().clone();
            let motion = before.as_ref().map_or(Motion::Fade, |b| view_motion(b, &lv));
            swaps.set((Some(lv), n + 1, motion));
            if motion != Motion::Fade {
                // After the new page renders: the left-behind copy moves off.
                let _ = document::eval("requestAnimationFrame(() => window.keelGhostGo && keelGhostGo());");
            }
        }
    });

    let today = today();
    let state = (*data.read()).clone();
    // The task view on top of the stack, if one is open.
    let open_task = nav
        .stack
        .read()
        .top()
        .and_then(|id| task_view::nav::detail(id, &nav.all.read(), (nav.show_completed)()));

    rsx! {
        style { dangerous_inner_html: style::css() }
        div { class: format!("app{}{}", if drawer() { " drawer-open" } else { "" }, if open_task.is_some() { " view-open" } else { "" }),
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
                    // The scrim and sheet stay mounted while views push and
                    // pop inside, so the entrance plays once per open.
                    if let Some(d) = open_task.clone() {
                        div {
                            class: "tv-scrim",
                            onclick: move |_| {
                                spawn(async move {
                                    if !gestures::slide_close().await {
                                        nav.ask_close();
                                    }
                                });
                            },
                        }
                        div { class: "tv-sheet",
                            div { class: "tv-grabber", aria_hidden: "true" }
                            task_view::view::TaskView {
                                key: "{d.task.id}",
                                detail: d,
                                depth: nav.stack.read().0.len(),
                                projects: loaded.projects.clone(),
                                sections: loaded.sections.clone(),
                                labels: loaded.labels.clone(),
                                inbox_id: loaded.inbox.id.clone(),
                                reload,
                            }
                        }
                    }
                    MainPane { view: loaded.view.clone(), loaded, today, show_completed, drawer, search_query, reload, swap: swaps().1, motion: swaps().2 }
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
    /// Bumped when a different view lands; alternates the fade-in.
    #[props(default)]
    swap: u32,
    motion: Motion,
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
    let mode = view_mode(&view);

    rsx! {
        main { class: if motion == Motion::Push { "main view-push" } else { "main" },
            div {
                class: match motion {
                    Motion::Pop => "content view-pop",
                    Motion::Push => "content",
                    Motion::Fade if swap % 2 == 0 => "content view-in-a",
                    Motion::Fade => "content view-in-b",
                },
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
                            CardList {
                                cards: views::cards(group.tasks.clone(), &loaded.all, mode),
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
                            CardList {
                                cards: views::cards(group.tasks.clone(), &loaded.all, mode),
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
                    CardList {
                        cards: views::cards(unsectioned, &loaded.all, mode),
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
                            all: loaded.all.clone(),
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
                        div { key: "done-{task.id}", class: "task-card",
                            CompletedRow {
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
}

#[derive(Clone, Copy, PartialEq)]
enum HeadMode {
    Normal,
    Renaming,
    ConfirmDelete,
}

/// A section's cards: top-level tasks only, one card each.
fn section_cards(tasks: Vec<Task>, all: &[Task]) -> Vec<views::Card> {
    views::cards(tasks, all, CardMode::TopLevel)
}

/// A project section: header (rename / delete on hover), its tasks, and its
/// own "+ Add task".
#[component]
fn SectionBlock(
    section: Section,
    tasks: Vec<Task>,
    /// Every task, for the cards' sub-task counts.
    all: Vec<Task>,
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
    let cards = section_cards(tasks, &all);
    // What the section shows: its top-level tasks (sub-tasks go with them).
    let count = cards.len();
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
            CardList {
                cards,
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

#[cfg(test)]
mod motion_tests {
    use super::*;

    #[test]
    fn search_pushes_in_and_pops_out() {
        assert_eq!(view_motion(&View::Today, &View::Search), Motion::Push);
        assert_eq!(view_motion(&View::Search, &View::Inbox), Motion::Pop);
        assert_eq!(view_motion(&View::Today, &View::Inbox), Motion::Fade);
        assert_eq!(view_motion(&View::Search, &View::Search), Motion::Fade);
    }
}

#[cfg(test)]
mod card_mode_tests {
    use super::*;
    use keel_core::model::Due;
    use views::CardMode;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn task(name: &str, parent: Option<&Task>, due: NaiveDate) -> Task {
        let mut t = Task::new(name, Id::from("proj"));
        t.id = Id::from(name);
        t.parent_id = parent.map(|p| p.id.clone());
        t.due = Some(Due {
            date: due.format("%Y-%m-%d").to_string(),
            timezone: None,
            string: String::new(),
            lang: "en".to_string(),
            is_recurring: false,
            recurrence: None,
        });
        t
    }

    #[test]
    fn project_views_are_top_level() {
        assert_eq!(view_mode(&View::Inbox), CardMode::TopLevel);
        assert_eq!(view_mode(&View::Project(Id::from("p"), "P".into())), CardMode::TopLevel);
    }

    #[test]
    fn dated_views_group() {
        assert_eq!(view_mode(&View::Today), CardMode::Grouped);
        assert_eq!(view_mode(&View::Upcoming), CardMode::Grouped);
        assert_eq!(view_mode(&View::Label("home".into())), CardMode::Grouped);
        assert_eq!(view_mode(&View::Filter(Id::from("f"), "F".into(), "today".into())), CardMode::Grouped);
    }

    #[test]
    fn section_count_is_top_level_tasks() {
        let day = d(2026, 10, 23);
        let p = task("Pack", None, day);
        let c = task("Chargers", Some(&p), day);
        let g = task("Cable", Some(&c), day);
        let other = task("Gym", None, day);
        let in_section = vec![p.clone(), c.clone(), g.clone(), other.clone()];
        let all = in_section.clone();
        assert_eq!(section_cards(in_section, &all).len(), 2);
    }

    #[test]
    fn upcoming_groups_cards_per_day() {
        let (today, thu, fri) = (d(2026, 10, 19), d(2026, 10, 22), d(2026, 10, 23));
        assert_eq!(thu.weekday(), chrono::Weekday::Thu);
        let parent = task("Pack for Boston", None, fri);
        let sub = task("Chargers", Some(&parent), thu);
        let all = vec![parent.clone(), sub.clone()];
        let groups = date_groups(&View::Upcoming, vec![sub.clone(), parent.clone()], today);
        assert_eq!(groups.len(), 2);
        let mode = view_mode(&View::Upcoming);

        let thursday = views::cards(groups[0].tasks.clone(), &all, mode);
        assert_eq!(groups[0].title.as_deref(), Some("22 Oct"));
        assert_eq!(thursday.len(), 1);
        assert_eq!(thursday[0].head.task.id, parent.id);
        assert!(!thursday[0].head_matches, "the parent is a context-only head on Thursday");
        let rows: Vec<_> = thursday[0].rows.iter().map(|r| r.task.id.clone()).collect();
        assert_eq!(rows, vec![sub.id.clone()]);
        assert_eq!(thursday[0].head.progress, Some(views::Progress { done: 0, total: 1 }));

        let friday = views::cards(groups[1].tasks.clone(), &all, mode);
        assert_eq!(groups[1].title.as_deref(), Some("23 Oct"));
        assert_eq!(friday.len(), 1);
        assert_eq!(friday[0].head.task.id, parent.id);
        assert!(friday[0].head_matches, "on Friday the parent is its own completable card");
        assert!(friday[0].rows.is_empty());
    }
}
