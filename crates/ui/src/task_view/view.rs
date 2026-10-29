//! The task view: header, title line, fields card and sub-task card for the
//! task on top of the view stack. Every change saves right away through
//! `editor::save_edit`, the same path as the editor.

use chrono::NaiveDate;
use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::model::{Label, Project, Reminder, Section};
use keel_core::recurrence::Recurrence;
use keel_core::views::CardRow;

use super::fields::{layout, pending_save_before_nav, subtask_header, takes_stored_text, Field, Missing};
use super::nav::{use_nav, TaskDetail, TaskViewNav};
use crate::api;
use crate::editor::{
    self, add_from_editor, apply_pick, flag_class, location_items, location_pick, location_text, priority_items,
    repeat_chip_text, repeat_options, repeat_text, save_edit, toggle_label, EditorValues, LabelPicker, TaskEditor,
};
use crate::glass::{ActionItem, ChipId, Pick, TaskAction};
use crate::icons::{self, Icon};
use crate::pickers::{
    date_chip, deadline_text, reminders_text, DatePicker, DeadlinePicker, Face, Menu, MenuItem, OpenPop, ReminderDraft,
    ReminderPicker,
};
use crate::task_row::{check_class, StreakPanel, SubtaskCount};

/// Where the view goes after saving a pending title or description.
#[derive(Clone, PartialEq)]
enum NavTo {
    Push(Id),
    Back,
    Close,
}

fn go(nav: TaskViewNav, to: NavTo) {
    match to {
        NavTo::Push(id) => nav.push(id),
        NavTo::Back => nav.back(),
        NavTo::Close => nav.close(),
    }
}

/// Saves a title / description edit when there is one; true if it saved.
async fn save_text(id: Id, pair: Option<(EditorValues, EditorValues)>, rule: Option<Recurrence>) -> bool {
    match pair {
        Some((before, after)) => {
            let _ = save_edit(id, before, after, rule).await;
            true
        }
        None => false,
    }
}

/// The ••• menu: Streak for a repeating task, then Delete.
fn view_actions(recurring: bool) -> Vec<ActionItem> {
    let item = |id, label: &str, destructive| ActionItem { id, label: label.to_string(), destructive };
    let mut items = Vec::new();
    if recurring {
        items.push(item(TaskAction::Streak, "Streak", false));
    }
    items.push(item(TaskAction::Delete, "Delete task", true));
    items
}

/// The native picker for a field row or pill. Repeat has no row: a rule
/// is edited in Date's sheet. Description is typed in place (None).
fn native_chip(field: Option<Field>, missing: Option<Missing>) -> Option<ChipId> {
    let field = match (field, missing) {
        (_, Some(Missing::Repeat)) => return Some(ChipId::Repeat),
        (Some(f), _) | (None, Some(Missing::Field(f))) => f,
        (None, None) => return None,
    };
    match field {
        Field::Description => None,
        Field::Date => Some(ChipId::Date),
        Field::Deadline => Some(ChipId::Deadline),
        Field::Priority => Some(ChipId::Priority),
        Field::Labels => Some(ChipId::Labels),
        Field::Reminders => Some(ChipId::Reminders),
        Field::Location => Some(ChipId::Location),
    }
}

fn row_face(class: &str, text: String) -> Face {
    Face { class: format!("tv-field-btn {class}"), text, icon: true }
}

fn pill_face(m: Missing) -> Face {
    Face { class: "tv-pill".to_string(), text: m.label().to_string(), icon: false }
}

#[component]
pub fn TaskView(
    detail: TaskDetail,
    /// How many views are stacked (Back shows above one).
    depth: usize,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    inbox_id: Id,
    reload: Signal<u32>,
    /// False for the view under a sub-task's, kept mounted and out of sight
    /// for the edge swipe: no header (a spacer of its height), and it leaves
    /// the shared flags (add sub-task, close, the native menu's choice) to
    /// the top one. A Back makes it active in place.
    #[props(default = true)] active: bool,
) -> Element {
    let mut reload = reload;
    let nav = use_nav();
    // Effects read it: they're made once, so a captured prop would stay stale.
    let mut is_active = use_signal(|| active);
    use_effect(use_reactive!(|active| is_active.set(active)));
    let glass = crate::glass::use_glass();
    let native = (glass.native)();
    let today = crate::today();
    let task = detail.task.clone();
    let id = task.id.clone();

    // The task as the editor sees it, with its reminders.
    let all_reminders = use_context::<Signal<Vec<Reminder>>>();
    let my_reminders: Vec<ReminderDraft> =
        all_reminders.read().iter().filter(|r| r.task_id == task.id).map(ReminderDraft::from_reminder).collect();
    let mut stored = EditorValues::from_task(&task);
    stored.reminders = my_reminders.clone();
    let rule = task.due.as_ref().and_then(|d| d.recurrence.clone());
    let recurring = task.due.as_ref().is_some_and(|d| d.is_recurring);
    let keep_label = (stored.repeat == "keep").then(|| rule.as_ref().map(repeat_text).unwrap_or_default());

    // `base` is what was last saved (ahead of the reload that confirms it);
    // `title` and `desc` are the drafts typed in place.
    let init = stored.clone();
    let mut base = use_signal(|| init.clone());
    let mut seen = use_signal(|| init.clone());
    let mut title = use_signal(|| init.content.clone());
    let mut desc = use_signal(|| init.description.clone());
    // The stored task changed (a save landed, or an edit elsewhere): take
    // it, and reset only the drafts whose stored text changed. Our own save
    // coming back leaves a draft typed since alone.
    use_effect(use_reactive!(|stored| {
        let old = seen.peek().clone();
        let saved = base.peek().clone();
        if takes_stored_text(&stored.content, &old.content, &saved.content) {
            title.set(stored.content.clone());
        }
        if takes_stored_text(&stored.description, &old.description, &saved.description) {
            desc.set(stored.description.clone());
        }
        base.set(stored.clone());
        seen.set(stored);
    }));
    let mut desc_open = use_signal(|| false);
    let mut adding = use_signal(|| false);
    let mut streak = use_signal(|| false);
    let mut more_open = use_signal(|| false);
    let open: OpenPop = use_signal(|| None);
    let mut open_set = open;
    // A list row's "Add sub-task" opens the view with the editor showing.
    use_effect(move || {
        let mut flag = nav.add_sub;
        if is_active() && flag() {
            flag.set(false);
            adding.set(true);
        }
    });

    // The title / description edit to save, if any (`base` moves ahead).
    // An emptied title goes back to the saved one.
    let mut commit = move || -> Option<(EditorValues, EditorValues)> {
        let before = base.peek().clone();
        let mut after = before.clone();
        let typed = title.peek().trim().to_string();
        if typed.is_empty() {
            title.set(before.content.clone());
        } else {
            if *title.peek() != typed {
                title.set(typed.clone());
            }
            after.content = typed;
        }
        after.description = desc.peek().clone();
        if !pending_save_before_nav(&before, &after) {
            return None;
        }
        base.set(after.clone());
        Some((before, after))
    };
    let save_drafts = {
        let (id, rule) = (id.clone(), rule.clone());
        use_callback(move |_: ()| {
            let pair = commit();
            if pair.is_some() {
                let (id, rule) = (id.clone(), rule.clone());
                // Not tied to the view: it may close before the save lands.
                spawn_forever(async move {
                    save_text(id, pair, rule).await;
                    reload += 1;
                });
            }
        })
    };
    // Moves at once (a second fast tap on Back acts on the view below,
    // not again on this one); the pending edit still saves.
    let navigate = use_callback(move |to: NavTo| {
        save_drafts.call(());
        go(nav, to);
    });
    // A tap on the scrim or a drag that pulled the sheet down.
    use_effect(move || {
        let mut flag = nav.close_asked;
        if is_active() && flag() {
            flag.set(false);
            navigate.call(NavTo::Close);
        }
    });
    // A picker's change, saved now.
    let save_values = {
        let (id, rule) = (id.clone(), rule.clone());
        use_callback(move |after: EditorValues| {
            let before = base.peek().clone();
            if before == after {
                return;
            }
            base.set(after.clone());
            let (id, rule) = (id.clone(), rule.clone());
            spawn_forever(async move {
                let _ = save_edit(id, before, after, rule).await;
                reload += 1;
            });
        })
    };
    let pick = use_callback(move |p: Pick| {
        let mut after = base.peek().clone();
        apply_pick(&mut after, &p, false, &[], &[], crate::today());
        save_values.call(after);
    });
    let toggle = use_callback(move |name: String| {
        let mut after = base.peek().clone();
        toggle_label(&mut after, &name, false, &[], &[], crate::today());
        save_values.call(after);
    });
    // iPhone: a field opens only its native picker; the pick saves at once.
    let pick_native = {
        let (task, mine) = (task.clone(), my_reminders.clone());
        let (p, s, l) = (projects.clone(), sections.clone(), labels.clone());
        use_callback(move |chip: ChipId| {
            glass.pick_field(&task, mine.clone(), p.clone(), s.clone(), l.clone(), chip, reload)
        })
    };
    let delete = {
        let id = id.clone();
        use_callback(move |_: ()| {
            let id = id.clone();
            // The reload prunes the stack: this view and any above it close.
            spawn_forever(async move {
                let _ = api::delete_task(id).await;
                reload += 1;
            });
        })
    };
    let on_action = use_callback(move |a: TaskAction| match a {
        TaskAction::Streak => streak.set(!streak()),
        TaskAction::Delete => delete.call(()),
        TaskAction::SubTask | TaskAction::Rename => {}
    });
    // iPhone: the ••• menu is the native action menu; its choice comes back here.
    let menu_target = format!("view:{}", task.id);
    {
        let target = menu_target.clone();
        use_effect(move || {
            let mut chosen = glass.chosen;
            let Some(c) = chosen().filter(|c| is_active() && c.target == target) else { return };
            chosen.set(None);
            on_action.call(c.action);
        });
    }

    let v = {
        let mut v = base();
        v.description = desc();
        v
    };
    let (mut fields, mut missing) = layout(&v, &inbox_id);
    // "+ Description" opens an empty description row to type into.
    if desc_open() && !fields.contains(&Field::Description) {
        fields.insert(0, Field::Description);
        missing.retain(|m| *m != Missing::Field(Field::Description));
    }
    let crumb = match &detail.parent {
        Some(p) => p.content.clone(),
        None => location_text(&task.project_id, None, &projects, &sections),
    };
    let repeat_opts = repeat_options(&v.date, keep_label.as_deref(), today);
    let repeat_items: Vec<MenuItem> = repeat_opts.iter().map(|(val, label)| MenuItem::new(*val, label.clone())).collect();
    let repeat_label = repeat_chip_text(&v.repeat, &repeat_opts);
    let repeat_rule = editor::recurrence_for_at(&v.repeat, &v.date, rule.clone(), today);
    let has_due = !v.date.is_empty() || !v.time.is_empty();
    let show_completed = (nav.show_completed)();
    let mut sub_defaults = EditorValues::blank(task.project_id.clone(), task.section_id.clone(), String::new(), Vec::new());
    sub_defaults.parent_id = Some(task.id.clone());

    // One field's control: on iPhone a plain button into the native card;
    // elsewhere the editor's own picker with this face.
    let field = |f: Option<Field>, m: Option<Missing>| -> Element {
        let row = f.is_some();
        let which = f.or(match m {
            Some(Missing::Field(x)) => Some(x),
            _ => None,
        });
        let face = |class: &str, text: String| match m {
            Some(m) => pill_face(m),
            None => row_face(class, text),
        };
        if m == Some(Missing::Field(Field::Description)) {
            return rsx! {
                button { r#type: "button", class: "tv-pill", onclick: move |_| desc_open.set(true), "{Missing::Field(Field::Description).label()}" }
            };
        }
        if native {
            let (class, text, icon) = match (which, m) {
                (_, Some(m)) => ("tv-pill".to_string(), m.label().to_string(), None),
                (Some(Field::Date), None) => {
                    let (text, cls) = date_chip(
                        NaiveDate::parse_from_str(&v.date, "%Y-%m-%d").ok(),
                        chrono::NaiveTime::parse_from_str(&v.time, "%H:%M").ok(),
                        today,
                    );
                    let text = if v.repeat != "none" { format!("{text} · {repeat_label}") } else { text };
                    (format!("tv-field-btn due-chip {cls}"), text, Some((icons::calendar_filled(), "")))
                }
                (Some(Field::Deadline), None) => (
                    "tv-field-btn deadline-chip set".to_string(),
                    format!("Deadline {}", deadline_text(NaiveDate::parse_from_str(&v.deadline, "%Y-%m-%d").ok(), today)),
                    Some((icons::target_small(), "")),
                ),
                (Some(Field::Priority), None) => {
                    ("tv-field-btn".to_string(), format!("Priority {}", v.priority), Some((icons::flag_filled(), flag_class(&v.priority))))
                }
                (Some(Field::Labels), None) => ("tv-field-btn".to_string(), v.labels.join(", "), Some((icons::tag_small(), ""))),
                (Some(Field::Reminders), None) => {
                    ("tv-field-btn".to_string(), reminders_text(&v.reminders, today), Some((icons::bell_small(), "")))
                }
                (_, None) => (
                    "tv-field-btn".to_string(),
                    location_text(&v.project_id, v.section_id.as_ref(), &projects, &sections),
                    Some((icons::hash_small(), "")),
                ),
            };
            let chip = native_chip(which, m);
            return rsx! {
                button { r#type: "button", class: "{class}", onclick: move |_| {
                    if let Some(chip) = chip {
                        pick_native.call(chip);
                    }
                },
                    if let Some((svg, color)) = icon {
                        span { class: "chip-icon {color}", Icon { svg } }
                    }
                    "{text}"
                }
            };
        }
        match (which, m) {
            (Some(Field::Date), _) => {
                let (text, cls) = date_chip(
                    NaiveDate::parse_from_str(&v.date, "%Y-%m-%d").ok(),
                    chrono::NaiveTime::parse_from_str(&v.time, "%H:%M").ok(),
                    today,
                );
                rsx! {
                    DatePicker {
                        date: v.date.clone(),
                        time: v.time.clone(),
                        open,
                        rule: repeat_rule.clone(),
                        face: face(&format!("due-chip {cls}"), text),
                        icon: icons::calendar_filled(),
                        on_change: move |(date, time): (String, String)| pick.call(Pick::Date { date, time }),
                    }
                    // A rule shows in the date's row; without one it's a pill.
                    if row && v.repeat != "none" {
                        Menu {
                            id: "repeat",
                            title: "Repeat",
                            icon: icons::repeat_small(),
                            text: repeat_label.clone(),
                            items: repeat_items.clone(),
                            selected: v.repeat.clone(),
                            open,
                            face: Face { class: "tv-field-btn tv-repeat".to_string(), text: repeat_label.clone(), icon: true },
                            on_select: move |value: String| pick.call(Pick::Repeat { value }),
                        }
                    }
                }
            }
            (None, Some(Missing::Repeat)) => rsx! {
                Menu {
                    id: "repeat",
                    title: "Repeat",
                    icon: icons::repeat_small(),
                    text: repeat_label.clone(),
                    items: repeat_items.clone(),
                    selected: v.repeat.clone(),
                    open,
                    face: pill_face(Missing::Repeat),
                    on_select: move |value: String| pick.call(Pick::Repeat { value }),
                }
            },
            (Some(Field::Deadline), _) => {
                let selected = NaiveDate::parse_from_str(&v.deadline, "%Y-%m-%d").ok();
                let cls = match selected {
                    Some(d) if d <= today => "deadline-chip urgent",
                    _ => "deadline-chip set",
                };
                rsx! {
                    DeadlinePicker {
                        deadline: v.deadline.clone(),
                        open,
                        face: face(cls, format!("Deadline {}", deadline_text(selected, today))),
                        on_change: move |date: String| pick.call(Pick::Deadline { date }),
                    }
                }
            }
            (Some(Field::Priority), _) => rsx! {
                Menu {
                    id: "priority",
                    title: "Priority",
                    icon: icons::flag_filled(),
                    icon_class: flag_class(&v.priority),
                    text: String::new(),
                    items: priority_items(),
                    selected: v.priority.clone(),
                    open,
                    face: face("", format!("Priority {}", v.priority)),
                    on_select: move |value: String| pick.call(Pick::Priority { value }),
                }
            },
            (Some(Field::Labels), _) => rsx! {
                LabelPicker {
                    labels: labels.clone(),
                    selected: v.labels.clone(),
                    text: String::new(),
                    open,
                    face: face("", v.labels.join(", ")),
                    on_toggle: move |name: String| toggle.call(name),
                    reload,
                }
            },
            (Some(Field::Reminders), _) => rsx! {
                ReminderPicker {
                    reminders: v.reminders.clone(),
                    has_due,
                    open,
                    face: face("", reminders_text(&v.reminders, today)),
                    on_change: move |drafts: Vec<ReminderDraft>| pick.call(Pick::Reminders { drafts }),
                }
            },
            (Some(Field::Location), _) => rsx! {
                Menu {
                    id: "location",
                    title: "Project / section",
                    icon: icons::hash_small(),
                    text: String::new(),
                    items: location_items(&projects, &sections),
                    selected: v.location(),
                    open,
                    face: face("", location_text(&v.project_id, v.section_id.as_ref(), &projects, &sections)),
                    on_select: move |value: String| pick.call(location_pick(&value)),
                }
            },
            _ => rsx! {},
        }
    };

    let header = detail.progress.map(|p| (p, subtask_header(p, show_completed)));
    let id_circle = id.clone();
    let rule_circle = rule.clone();
    let checked = task.checked;

    rsx! {
        div { class: if active { "task-view" } else { "task-view tv-under" }, "data-view-id": "{task.id}", aria_hidden: if active { "false" } else { "true" },
            if open().is_some() || more_open() {
                div {
                    class: "pop-backdrop",
                    onclick: move |_| {
                        open_set.set(None);
                        more_open.set(false);
                    },
                }
            }
            if !active {
                div { class: "tv-head-space" }
            }
            if active {
            div { class: "tv-head",
                button {
                    r#type: "button",
                    class: "tv-close tv-round",
                    title: "Close",
                    aria_label: "Close",
                    // On the phone layout the sheet slides down first.
                    onclick: move |_| {
                        spawn(async move {
                            if !crate::gestures::slide_close().await {
                                navigate.call(NavTo::Close);
                            }
                        });
                    },
                    Icon { svg: icons::x() }
                }
                if depth > 1 {
                    button {
                        r#type: "button",
                        class: "tv-back icon-btn",
                        title: "Back",
                        onclick: move |_| navigate.call(NavTo::Back),
                        Icon { svg: icons::chevron_left() }
                        span { "Back" }
                    }
                }
                span { class: "tv-crumb", "{crumb}" }
                div { class: "chip-wrap tv-head-actions",
                    button {
                        r#type: "button",
                        class: "tv-more tv-round",
                        title: "More",
                        aria_label: "More",
                        onclick: {
                            let (target, title) = (menu_target.clone(), task.content.clone());
                            move |_| {
                                if native {
                                    let (target, title) = (target.clone(), title.clone());
                                    let items = view_actions(recurring);
                                    spawn(async move { api::glass_actions(&target, &title, items).await });
                                } else {
                                    more_open.set(!more_open());
                                }
                            }
                        },
                        Icon { svg: icons::more() }
                    }
                    if more_open() {
                        div { class: "popover menu",
                            onmounted: move |_| crate::pickers::place_popover(),
                            for (action, label, destructive) in view_actions(recurring).into_iter().map(|i| (i.id, i.label, i.destructive)) {
                                button {
                                    key: "{label}",
                                    r#type: "button",
                                    class: if destructive { "menu-item danger" } else { "menu-item" },
                                    onclick: move |_| {
                                        more_open.set(false);
                                        on_action.call(action);
                                    },
                                    span { class: "menu-label", "{label}" }
                                }
                            }
                        }
                    }
                }
            }
            }
            div { class: "tv-body",
            div { class: "tv-scroll",
                div { class: if checked { "tv-titleline done" } else { "tv-titleline" },
                    button {
                        r#type: "button",
                        class: if checked { "check done" } else { check_class(task.priority) },
                        title: if checked { "Mark as not done" } else { "Complete task" },
                        onclick: move |_| {
                            if !nav.hold_circles() {
                                return;
                            }
                            let pair = commit();
                            let (id, rule) = (id_circle.clone(), rule_circle.clone());
                            spawn_forever(async move {
                                save_text(id.clone(), pair, rule).await;
                                if checked {
                                    let _ = api::uncomplete_task(id).await;
                                } else if api::complete_task(id).await.is_ok() {
                                    // Back to the parent, or closed at the top.
                                    nav.completed_here();
                                }
                                reload += 1;
                                nav.release_circles().await;
                            });
                        },
                        Icon { svg: icons::check() }
                    }
                    GrowingText {
                        class: "tv-title",
                        value: title(),
                        placeholder: "Task name",
                        on_input: move |s: String| title.set(s),
                        on_commit: move |_| save_drafts.call(()),
                    }
                }
                div { class: "tv-card tv-fields",
                    for f in fields.iter().copied() {
                        div { key: "{f:?}", class: "tv-row",
                            if f == Field::Description {
                                span { class: "tv-row-icon", Icon { svg: icons::text_lines() } }
                                GrowingText {
                                    class: "tv-desc",
                                    value: desc(),
                                    placeholder: "Description",
                                    autofocus: desc_open() && base.peek().description.trim().is_empty(),
                                    on_input: move |s: String| desc.set(s),
                                    on_commit: move |_| {
                                        save_drafts.call(());
                                        desc_open.set(false);
                                    },
                                }
                            } else {
                                {field(Some(f), None)}
                            }
                        }
                    }
                    if !missing.is_empty() {
                        div { class: "tv-pills",
                            for m in missing.iter().copied() {
                                span { key: "{m:?}", class: "tv-pill-slot", {field(None, Some(m))} }
                            }
                        }
                    }
                }
                if streak() && recurring {
                    div { class: "tv-card", StreakPanel { task: task.clone(), reload } }
                }
                div { class: "tv-card tv-subtasks",
                    if let Some((_, (count, toggle_text, all_done))) = header.clone() {
                        div { class: "tv-sub-head",
                            b { "Sub-tasks" }
                            span { class: "tv-sub-side",
                                "{count} · "
                                button {
                                    r#type: "button",
                                    class: "tv-toggle",
                                    onclick: move |_| {
                                        let mut s = nav.show_completed;
                                        s.set(!show_completed);
                                    },
                                    "{toggle_text}"
                                }
                                if all_done {
                                    span { class: "done-check", title: "All sub-tasks done", Icon { svg: icons::check_small() } }
                                }
                            }
                        }
                    }
                    div { class: "tv-sub-list",
                        if let Some((p, _)) = header {
                            div { class: "tv-progress",
                                i { style: "width: {p.done * 100 / p.total.max(1)}%" }
                            }
                        }
                        for row in detail.visible.iter().cloned() {
                            SubtaskRow { key: "{row.task.id}", row, on_open: move |id: Id| navigate.call(NavTo::Push(id)), reload }
                        }
                        if adding() && !native {
                            div { class: "tv-add-editor",
                                TaskEditor {
                                    initial: sub_defaults.clone(),
                                    projects: projects.clone(),
                                    sections: sections.clone(),
                                    labels: labels.clone(),
                                    keep_label: None,
                                    submit_text: "Add sub-task".to_string(),
                                    quick_add: true,
                                    keep_open: true,
                                    on_submit: {
                                        let (p, s, l) = (projects.clone(), sections.clone(), labels.clone());
                                        move |v: EditorValues| {
                                            let (p, s, l) = (p.clone(), s.clone(), l.clone());
                                            spawn_forever(async move {
                                                let _ = add_from_editor(v, p, s, l).await;
                                                reload += 1;
                                            });
                                        }
                                    },
                                    on_cancel: move |_| adding.set(false),
                                    reload,
                                }
                            }
                        } else {
                            button {
                                r#type: "button",
                                class: "tv-add-sub",
                                onclick: {
                                    let d = sub_defaults.clone();
                                    let (p, s, l) = (projects.clone(), sections.clone(), labels.clone());
                                    move |_| {
                                        if native {
                                            glass.add(d.clone(), p.clone(), s.clone(), l.clone(), reload);
                                        } else {
                                            adding.set(true);
                                        }
                                    }
                                },
                                Icon { svg: icons::plus(14) }
                                "Add sub-task"
                            }
                        }
                    }
                }
            }
            }
        }
    }
}

/// One direct sub-task: its circle (complete, or reopen when checked), its
/// name, its own "x/y", and a ›. The rest of the row opens its view.
#[component]
fn SubtaskRow(row: CardRow, on_open: EventHandler<Id>, reload: Signal<u32>) -> Element {
    let mut reload = reload;
    let t = row.task.clone();
    let checked = t.checked;
    let id_check = t.id.clone();
    let id_open = t.id.clone();
    let nav = use_nav();
    rsx! {
        div { class: if checked { "tv-sub done" } else { "tv-sub" },
            button {
                r#type: "button",
                class: if checked { "check done" } else { check_class(t.priority) },
                title: if checked { "Mark as not done" } else { "Complete sub-task" },
                onclick: move |_| {
                    if !nav.hold_circles() {
                        return;
                    }
                    let id = id_check.clone();
                    // Not tied to the row: it goes once the task is done.
                    spawn_forever(async move {
                        let _ = if checked { api::uncomplete_task(id).await } else { api::complete_task(id).await };
                        reload += 1;
                        nav.release_circles().await;
                    });
                },
                Icon { svg: icons::check() }
            }
            div { class: "tv-sub-body", title: "Open task", onclick: move |_| on_open.call(id_open.clone()),
                span { class: "task-content", "{t.content}" }
                if let Some(p) = row.progress {
                    SubtaskCount { progress: p }
                }
                span { class: "card-chev", Icon { svg: icons::chevron_right() } }
            }
        }
    }
}

/// A text field that grows with its text (a textarea over a mirror). Enter
/// and blur commit; line breaks are typed as spaces.
#[component]
fn GrowingText(
    class: &'static str,
    value: String,
    placeholder: &'static str,
    #[props(default)] autofocus: bool,
    on_input: EventHandler<String>,
    on_commit: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "tv-grow {class}",
            div { class: "tv-grow-mirror", aria_hidden: "true", "{value}\u{200b}" }
            textarea {
                rows: "1",
                placeholder,
                value: "{value}",
                // `autofocus` is only an attribute to Dioxus, which a browser
                // ignores after load; focus the new field itself.
                onmounted: move |e| async move {
                    if autofocus {
                        let _ = e.set_focus(true).await;
                    }
                },
                oninput: move |e| on_input.call(e.value().replace(['\n', '\r'], " ")),
                onkeydown: move |e| {
                    if e.key() == Key::Enter {
                        e.prevent_default();
                        on_commit.call(());
                    }
                },
                onblur: move |_| on_commit.call(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_field_opens_its_own_native_picker() {
        let rows = [
            (Field::Date, ChipId::Date),
            (Field::Deadline, ChipId::Deadline),
            (Field::Priority, ChipId::Priority),
            (Field::Labels, ChipId::Labels),
            (Field::Reminders, ChipId::Reminders),
            (Field::Location, ChipId::Location),
        ];
        for (f, chip) in rows {
            assert_eq!(native_chip(Some(f), None), Some(chip), "{f:?} row");
            assert_eq!(native_chip(None, Some(Missing::Field(f))), Some(chip), "{f:?} pill");
        }
        assert_eq!(native_chip(None, Some(Missing::Repeat)), Some(ChipId::Repeat));
        // Description is typed in place, not picked.
        assert_eq!(native_chip(Some(Field::Description), None), None);
        assert_eq!(native_chip(None, Some(Missing::Field(Field::Description))), None);
    }
}
