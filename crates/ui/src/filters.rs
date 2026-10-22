//! Sidebar pieces for saved filters: the add/edit form (with live query
//! validation) and a filter row (open / edit / delete).

use dioxus::prelude::*;
use keel_core::model::Filter;

use crate::glass::{answer_text, use_prompt_answer, Prompt};
use crate::icons::{self, Icon};
use crate::{api, View};

/// Name + query form. Save is enabled only for a non-empty name and a query
/// the backend accepts; problems are shown as you type.
#[component]
pub fn FilterForm(
    initial_name: String,
    initial_query: String,
    submit_text: String,
    on_save: EventHandler<(String, String)>,
    on_cancel: EventHandler<()>,
) -> Element {
    let start_name = initial_name.clone();
    let start_query = initial_query.clone();
    let mut name = use_signal(move || start_name);
    let mut query = use_signal(move || start_query);
    // None = valid (or not yet checked and empty); Some(msg) = invalid.
    let mut problem = use_signal(|| None::<String>);
    // Increments per keystroke so a slow, stale check can't overwrite a newer one.
    let mut generation = use_signal(|| 0u64);

    let check = move |q: String| {
        generation += 1;
        let mine = generation();
        spawn(async move {
            let result = if q.trim().is_empty() { Ok(()) } else { api::check_filter(&q).await };
            if generation() == mine {
                problem.set(result.err());
            }
        });
    };
    // Validate the initial query once (edit mode).
    let mut check_initial = check;
    use_hook(move || check_initial(initial_query.clone()));

    let can_save = !name().trim().is_empty() && !query().trim().is_empty() && problem().is_none();

    rsx! {
        form {
            class: "filter-form",
            onsubmit: move |e| {
                e.prevent_default();
                if can_save {
                    on_save.call((name().trim().to_string(), query().trim().to_string()));
                }
            },
            input {
                class: "project-input",
                placeholder: "Filter name",
                autofocus: true,
                value: "{name}",
                oninput: move |e| name.set(e.value()),
            }
            input {
                class: if problem().is_some() { "project-input mono invalid" } else { "project-input mono" },
                placeholder: "Query, e.g. today & p1",
                value: "{query}",
                oninput: move |e| {
                    let q = e.value();
                    query.set(q.clone());
                    let mut check = check;
                    check(q);
                },
                onkeydown: move |e| {
                    if e.key() == Key::Escape {
                        on_cancel.call(());
                    }
                },
            }
            if let Some(msg) = problem() {
                div { class: "filter-problem", "{msg}" }
            } else {
                div { class: "filter-hint",
                    "today · overdue · 7 days · p1 · #Project · /Section · @label · no date · search: word · due before: +3 days · deadline before: +7 days · no deadline — combine with & | ! ( ) and , for separate lists"
                }
            }
            div { class: "confirm-actions",
                button { r#type: "button", class: "btn btn-cancel", onclick: move |_| on_cancel.call(()), "Cancel" }
                button { r#type: "submit", class: "btn btn-add", disabled: !can_save, "{submit_text}" }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum RowMode {
    Normal,
    Editing,
    ConfirmDelete,
}

/// A filter in the sidebar: click to open; hover to edit or delete.
/// The native filter prompt (iPhone): name and query, with `problem` shown
/// when a query was rejected.
pub fn filter_prompt(id: &str, title: &str, confirm: &str, name: &str, query: &str, problem: &str) -> Prompt {
    Prompt::new(id, title, confirm).field("Name", name).field("Query, e.g. today & p1", query).message(problem)
}

/// Checks a prompt's filter answer. Ok((name, query)) when both are set
/// and the query parses; Err(message) to ask again.
pub async fn checked_filter(values: &[String]) -> Result<(String, String), String> {
    let (Some(name), Some(query)) = (answer_text(values, 0), answer_text(values, 1)) else {
        return Err("A filter needs a name and a query.".to_string());
    };
    api::check_filter(&query).await.map(|_| (name, query))
}

#[component]
pub fn FilterRow(filter: Filter, active: bool, view: Signal<View>, reload: Signal<u32>) -> Element {
    let mut view = view;
    let mut reload = reload;
    let mut mode = use_signal(|| RowMode::Normal);
    let open = View::Filter(filter.id.clone(), filter.name.clone(), filter.query.clone());
    let id_save = filter.id.clone();
    let id_delete = filter.id.clone();
    let class = if active { "nav-item active" } else { "nav-item" };
    let menu_id = format!("filter:{}", filter.id);
    let glass = crate::glass::use_glass();
    // iPhone: a long press offers Edit and Delete (the desktop's hover
    // buttons); Edit is a native prompt (a web form gets no keyboard).
    let edit_id = format!("edit:{menu_id}");
    {
        let (pid, name, query) = (edit_id.clone(), filter.name.clone(), filter.query.clone());
        crate::glass::use_row_menu(menu_id.clone(), filter.name.clone(), crate::glass::nav_actions("Edit filter"), move |action| {
            match action {
                crate::glass::TaskAction::Rename => glass.ask(filter_prompt(&pid, "Edit filter", "Save", &name, &query, "")),
                crate::glass::TaskAction::Delete => mode.set(RowMode::ConfirmDelete),
                _ => {}
            }
        });
    }
    {
        let (pid, id) = (edit_id.clone(), filter.id.clone());
        use_prompt_answer(edit_id, move |values| {
            let (pid, id) = (pid.clone(), id.clone());
            spawn(async move {
                match checked_filter(&values).await {
                    Ok((name, query)) => {
                        let _ = api::update_filter(id.clone(), name.clone(), query.clone()).await;
                        if matches!(view(), View::Filter(ref open, _, _) if *open == id) {
                            view.set(View::Filter(id, name, query));
                        }
                        reload += 1;
                    }
                    Err(problem) => {
                        let (n, q) = (values.first().cloned().unwrap_or_default(), values.get(1).cloned().unwrap_or_default());
                        glass.ask(filter_prompt(&pid, "Edit filter", "Save", &n, &q, &problem));
                    }
                }
            });
        });
    }

    match mode() {
        RowMode::Editing => rsx! {
            FilterForm {
                initial_name: filter.name.clone(),
                initial_query: filter.query.clone(),
                submit_text: "Save".to_string(),
                on_save: move |(name, query): (String, String)| {
                    mode.set(RowMode::Normal);
                    let id = id_save.clone();
                    spawn(async move {
                        let _ = api::update_filter(id.clone(), name.clone(), query.clone()).await;
                        // Keep the open view in sync with the edited filter.
                        if matches!(view(), View::Filter(ref open, _, _) if *open == id) {
                            view.set(View::Filter(id, name, query));
                        }
                        reload += 1;
                    });
                },
                on_cancel: move |_| mode.set(RowMode::Normal),
            }
        },
        RowMode::ConfirmDelete => rsx! {
            div { class: "confirm-box",
                div { class: "confirm-text", "Delete filter “{filter.name}”? Your tasks aren’t affected." }
                div { class: "confirm-actions",
                    button { class: "btn btn-cancel", onclick: move |_| mode.set(RowMode::Normal), "Cancel" }
                    button {
                        class: "btn btn-add",
                        onclick: move |_| {
                            mode.set(RowMode::Normal);
                            let id = id_delete.clone();
                            spawn(async move {
                                let _ = api::delete_filter(id.clone()).await;
                                if matches!(view(), View::Filter(ref open, _, _) if *open == id) {
                                    view.set(View::Inbox);
                                }
                                reload += 1;
                            });
                        },
                        "Delete"
                    }
                }
            }
        },
        RowMode::Normal => rsx! {
            div {
                class,
                "data-menu-id": "{menu_id}",
                title: "{filter.query}",
                onclick: move |_| view.set(open.clone()),
                span { class: "nav-icon filter", Icon { svg: icons::funnel() } }
                span { class: "nav-name", "{filter.name}" }
                span { class: "nav-actions",
                    button {
                        class: "icon-btn",
                        title: "Edit filter",
                        onclick: move |e| {
                            e.stop_propagation();
                            mode.set(RowMode::Editing);
                        },
                        Icon { svg: icons::pencil() }
                    }
                    button {
                        class: "icon-btn",
                        title: "Delete filter",
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
