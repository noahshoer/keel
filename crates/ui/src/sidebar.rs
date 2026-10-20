use dioxus::prelude::*;
use keel_core::model::{Filter, Label, Project};

use crate::filters::{FilterForm, FilterRow};
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
    filters: Vec<Filter>,
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
    let mut show_settings = use_signal(|| false);
    let mut adding = use_signal(|| false);
    let mut adding_label = use_signal(|| false);
    let mut adding_filter = use_signal(|| false);
    let mut label_name = use_signal(String::new);
    let mut name = use_signal(String::new);
    let current = view();
    let count = if today_count > 0 { today_count.to_string() } else { String::new() };
    let user_projects: Vec<Project> = projects.into_iter().filter(|p| !p.is_inbox).collect();

    rsx! {
        aside { class: "sidebar",
            div { class: "brand",
                div { class: "brand-mark", "✓" }
                "Keel"
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
                button {
                    class: "icon-btn",
                    title: "Settings",
                    onclick: move |_| show_settings.set(true),
                    Icon { svg: icons::gear() }
                }
            }
            if show_settings() {
                crate::settings::SettingsDialog { on_close: move |_| show_settings.set(false) }
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
                NavProject { key: "{p.id}", active: current == View::Project(p.id.clone(), p.name.clone()), project: p, view, reload }
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
                NavLabel { key: "{l.id}", active: current == View::Label(l.name.clone()), label: l, view, reload }
            }
            div { class: "section-head",
                "Filters"
                button {
                    class: "icon-btn",
                    title: "Add filter",
                    onclick: move |_| adding_filter.set(!adding_filter()),
                    Icon { svg: icons::plus(16) }
                }
            }
            if adding_filter() {
                FilterForm {
                    initial_name: String::new(),
                    initial_query: String::new(),
                    submit_text: "Add filter".to_string(),
                    on_save: move |(name, query): (String, String)| {
                        adding_filter.set(false);
                        spawn(async move {
                            if let Ok(res) = api::add_filter(name.clone(), query.clone()).await {
                                if let Some(id) = res.id_mapping.values().next().cloned() {
                                    view.set(View::Filter(id, name, query));
                                }
                            }
                            reload += 1;
                        });
                    },
                    on_cancel: move |_| adding_filter.set(false),
                }
            }
            for f in filters {
                FilterRow {
                    key: "{f.id}",
                    active: matches!(&current, View::Filter(id, _, _) if *id == f.id),
                    filter: f,
                    view,
                    reload,
                }
            }
        }
    }
}

/// What a sidebar row is doing besides navigating.
#[derive(Clone, Copy, PartialEq)]
enum RowMode {
    Normal,
    Renaming,
    ConfirmDelete,
}

/// A project or label row with hover actions: rename inline, delete with an
/// inline confirm. `kind` is "project" or "label".
#[component]
fn ManagedRow(
    name: String,
    icon: Element,
    icon_class: String,
    active: bool,
    kind: String,
    on_open: EventHandler<()>,
    on_rename: EventHandler<String>,
    on_delete: EventHandler<()>,
) -> Element {
    let mut mode = use_signal(|| RowMode::Normal);
    let mut draft = use_signal(String::new);
    let start_name = name.clone();
    let warning = if kind == "project" {
        format!("Delete “{name}” and all its tasks?")
    } else {
        format!("Delete label “{name}”? It’s removed from its tasks.")
    };

    match mode() {
        RowMode::Renaming => rsx! {
            form {
                class: "project-form",
                onsubmit: move |e| {
                    e.prevent_default();
                    let n = draft().trim().to_string();
                    if !n.is_empty() {
                        on_rename.call(n);
                    }
                    mode.set(RowMode::Normal);
                },
                input {
                    class: "project-input",
                    autofocus: true,
                    value: "{draft}",
                    oninput: move |e| draft.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Escape {
                            mode.set(RowMode::Normal);
                        }
                    },
                }
            }
        },
        RowMode::ConfirmDelete => rsx! {
            div { class: "confirm-box",
                div { class: "confirm-text", "{warning}" }
                div { class: "confirm-actions",
                    button {
                        class: "btn btn-cancel",
                        onclick: move |_| mode.set(RowMode::Normal),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-add",
                        onclick: move |_| {
                            mode.set(RowMode::Normal);
                            on_delete.call(());
                        },
                        "Delete"
                    }
                }
            }
        },
        RowMode::Normal => rsx! {
            div {
                class: nav_class(active),
                onclick: move |_| on_open.call(()),
                span { class: "nav-icon {icon_class}", {icon} }
                span { class: "nav-name", "{name}" }
                span { class: "nav-actions",
                    button {
                        class: "icon-btn",
                        title: "Rename",
                        onclick: move |e| {
                            e.stop_propagation();
                            draft.set(start_name.clone());
                            mode.set(RowMode::Renaming);
                        },
                        Icon { svg: icons::pencil() }
                    }
                    button {
                        class: "icon-btn",
                        title: "Delete",
                        onclick: move |e| {
                            e.stop_propagation();
                            mode.set(RowMode::ConfirmDelete);
                        },
                        Icon { svg: icons::trash_small() }
                    }
                }
            }
        },
    }
}

#[component]
fn NavProject(project: Project, active: bool, view: Signal<View>, reload: Signal<u32>) -> Element {
    let mut view = view;
    let mut reload = reload;
    let open_target = View::Project(project.id.clone(), project.name.clone());
    let id_rename = project.id.clone();
    let id_delete = project.id.clone();
    rsx! {
        ManagedRow {
            name: project.name.clone(),
            icon: rsx! { "#" },
            icon_class: "hash".to_string(),
            active,
            kind: "project".to_string(),
            on_open: move |_| view.set(open_target.clone()),
            on_rename: move |new_name: String| {
                let id = id_rename.clone();
                spawn(async move {
                    let _ = api::rename_project(id.clone(), new_name.clone()).await;
                    // Keep the open view's title in sync with the new name.
                    if matches!(view(), View::Project(ref open, _) if *open == id) {
                        view.set(View::Project(id, new_name));
                    }
                    reload += 1;
                });
            },
            on_delete: move |_| {
                let id = id_delete.clone();
                spawn(async move {
                    let _ = api::delete_project(id.clone()).await;
                    if matches!(view(), View::Project(ref open, _) if *open == id) {
                        view.set(View::Inbox);
                    }
                    reload += 1;
                });
            },
        }
    }
}

#[component]
fn NavLabel(label: Label, active: bool, view: Signal<View>, reload: Signal<u32>) -> Element {
    let mut view = view;
    let mut reload = reload;
    let open_target = View::Label(label.name.clone());
    let old_name = label.name.clone();
    let old_name_del = label.name.clone();
    let id_rename = label.id.clone();
    let id_delete = label.id.clone();
    rsx! {
        ManagedRow {
            name: label.name.clone(),
            icon: rsx! { Icon { svg: icons::tag() } },
            icon_class: "label".to_string(),
            active,
            kind: "label".to_string(),
            on_open: move |_| view.set(open_target.clone()),
            on_rename: move |new_name: String| {
                let id = id_rename.clone();
                let old = old_name.clone();
                spawn(async move {
                    let _ = api::rename_label(id, new_name.clone()).await;
                    if matches!(view(), View::Label(ref open) if open.eq_ignore_ascii_case(&old)) {
                        view.set(View::Label(new_name));
                    }
                    reload += 1;
                });
            },
            on_delete: move |_| {
                let id = id_delete.clone();
                let old = old_name_del.clone();
                spawn(async move {
                    let _ = api::delete_label(id).await;
                    if matches!(view(), View::Label(ref open) if open.eq_ignore_ascii_case(&old)) {
                        view.set(View::Inbox);
                    }
                    reload += 1;
                });
            },
        }
    }
}
