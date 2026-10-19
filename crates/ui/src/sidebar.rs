use dioxus::prelude::*;
use todo_core::model::{Label, Project};

use crate::icons::{self, Icon};
use crate::{api, theme, View};

fn nav_class(active: bool) -> &'static str {
    if active {
        "nav-item active"
    } else {
        "nav-item"
    }
}

#[component]
pub fn Sidebar(
    view: Signal<View>,
    projects: Vec<Project>,
    labels: Vec<Label>,
    today_count: usize,
    today_day: u32,
    theme: Signal<String>,
    reload: Signal<u32>,
) -> Element {
    let mut view = view;
    let mut reload = reload;
    let mut theme_pref = theme;
    let theme_icon = match theme_pref().as_str() {
        "light" => icons::sun(),
        "dark" => icons::moon(),
        _ => icons::monitor(),
    };
    let theme_title = theme::label(&theme_pref());
    let mut adding = use_signal(|| false);
    let mut adding_label = use_signal(|| false);
    let mut label_name = use_signal(String::new);
    let mut name = use_signal(String::new);
    let current = view();
    let count = if today_count > 0 { today_count.to_string() } else { String::new() };
    let user_projects: Vec<Project> = projects.into_iter().filter(|p| !p.is_inbox).collect();

    rsx! {
        aside { class: "sidebar",
            div { class: "brand",
                div { class: "brand-mark", "✓" }
                "Todo Planner"
                button {
                    class: "icon-btn theme-toggle",
                    title: "{theme_title}",
                    onclick: move |_| {
                        let next = theme::next(&theme_pref()).to_string();
                        theme::apply(&next);
                        theme_pref.set(next);
                    },
                    Icon { svg: theme_icon.clone() }
                }
            }
            div {
                class: nav_class(current == View::Inbox),
                onclick: move |_| view.set(View::Inbox),
                span { class: "nav-icon inbox", Icon { svg: icons::inbox() } }
                "Inbox"
            }
            div {
                class: nav_class(current == View::Today),
                onclick: move |_| view.set(View::Today),
                span { class: "nav-icon today", Icon { svg: icons::today(today_day) } }
                "Today"
                span { class: "count", "{count}" }
            }
            div {
                class: nav_class(current == View::Upcoming),
                onclick: move |_| view.set(View::Upcoming),
                span { class: "nav-icon upcoming", Icon { svg: icons::upcoming() } }
                "Upcoming"
            }
            div { class: "section-head",
                "My Projects"
                button {
                    class: "icon-btn",
                    title: "Add project",
                    onclick: move |_| adding.set(!adding()),
                    Icon { svg: icons::plus(16) }
                }
            }
            if adding() {
                form {
                    class: "project-form",
                    onsubmit: move |e| {
                        e.prevent_default();
                        let n = name().trim().to_string();
                        if n.is_empty() {
                            return;
                        }
                        spawn(async move {
                            let _ = api::add_project(n).await;
                            name.set(String::new());
                            adding.set(false);
                            reload += 1;
                        });
                    },
                    input {
                        class: "project-input",
                        placeholder: "Project name, then Enter",
                        autofocus: true,
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
            }
            for p in user_projects {
                NavProject { key: "{p.id}", active: current == View::Project(p.id.clone(), p.name.clone()), project: p, view }
            }
            div { class: "section-head",
                "Labels"
                button {
                    class: "icon-btn",
                    title: "Add label",
                    onclick: move |_| adding_label.set(!adding_label()),
                    Icon { svg: icons::plus(16) }
                }
            }
            if adding_label() {
                form {
                    class: "project-form",
                    onsubmit: move |e| {
                        e.prevent_default();
                        let n = label_name().trim().to_string();
                        if n.is_empty() {
                            return;
                        }
                        spawn(async move {
                            let _ = api::add_label(n).await;
                            label_name.set(String::new());
                            adding_label.set(false);
                            reload += 1;
                        });
                    },
                    input {
                        class: "project-input",
                        placeholder: "Label name, then Enter",
                        autofocus: true,
                        value: "{label_name}",
                        oninput: move |e| label_name.set(e.value()),
                    }
                }
            }
            for l in labels {
                NavLabel { key: "{l.id}", active: current == View::Label(l.name.clone()), name: l.name, view }
            }
        }
    }
}

#[component]
fn NavProject(project: Project, active: bool, view: Signal<View>) -> Element {
    let mut view = view;
    let target = View::Project(project.id.clone(), project.name.clone());
    rsx! {
        div {
            class: nav_class(active),
            onclick: move |_| view.set(target.clone()),
            span { class: "nav-icon hash", "#" }
            "{project.name}"
        }
    }
}

#[component]
fn NavLabel(name: String, active: bool, view: Signal<View>) -> Element {
    let mut view = view;
    let target = View::Label(name.clone());
    rsx! {
        div {
            class: nav_class(active),
            onclick: move |_| view.set(target.clone()),
            span { class: "nav-icon label", Icon { svg: icons::tag() } }
            "{name}"
        }
    }
}
