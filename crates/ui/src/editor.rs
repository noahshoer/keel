//! The Todoist-style task editor card, shared by "+ Add task", "Add sub-task"
//! and in-place editing, plus the collapsed "+ Add task" row. When adding, the
//! title is parsed as natural language (dates, times, repeats, p1, #project,
//! /section, @labels) with a live preview. The data rules are
//! `keel_core::editor`'s (shared with the React editor).

use dioxus::prelude::*;
use keel_core::editor::{self as core_editor, LocationKind, Pick};
use keel_core::model::{Label, Project, Section};
use keel_core::quickadd::{Span, SpanKind};
use keel_core::recurrence::Recurrence;

pub use keel_core::editor::{
    apply_pick, has_name, labels_text, labels_to_create, location_pick, location_text, parse_title as parse_title_at,
    priority_text, recurrence_for as recurrence_for_at, repeat_chip_text, repeat_options, repeat_text, toggle_label,
    unhighlight, with_title, EditorValues,
};

use crate::api;
use crate::icons::{self, Icon};
use keel_core::ids::Id;
use crate::pickers::{DatePicker, DeadlinePicker, Face, Menu, MenuItem, OpenPop, ReminderDraft, ReminderPicker};

/// The project / section menu: each project, then its sections indented.
/// Values are `project` or `project|section` ([`location_pick`] reads them).
pub(crate) fn location_items(projects: &[Project], sections: &[Section]) -> Vec<MenuItem> {
    core_editor::location_choices(projects, sections)
        .into_iter()
        .map(|c| match c.kind {
            LocationKind::Inbox => MenuItem::new(c.value, c.label).icon(icons::inbox(), "loc-inbox"),
            LocationKind::Project => MenuItem::new(c.value, c.label).icon(icons::hash_small(), "loc-project"),
            LocationKind::Section => {
                MenuItem::new(c.value, c.label).icon(icons::section_small(), "loc-section").indented()
            }
        })
        .collect()
}

/// A priority's flag color class ("4" is no priority).
pub(crate) fn flag_class(priority: &str) -> &'static str {
    match priority {
        "1" => "pri-1",
        "2" => "pri-2",
        "3" => "pri-3",
        _ => "pri-4",
    }
}

/// The priority menu with colored flags.
pub(crate) fn priority_items() -> Vec<MenuItem> {
    ["1", "2", "3", "4"]
        .iter()
        .map(|p| MenuItem::new(*p, format!("Priority {p}")).icon(icons::flag_filled(), flag_class(p)))
        .collect()
}

/// Parses the editor's title as quick add.
fn parse_title(v: &EditorValues, projects: &[Project], sections: &[Section]) -> keel_core::quickadd::QuickAdd {
    parse_title_at(v, projects, sections, crate::today())
}

/// Adds the task the editor describes (quick add applied), creating any
/// labels it mentions that don't exist yet. Fails when the task wasn't added.
pub async fn add_from_editor(
    v: EditorValues,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
) -> Result<(), String> {
    let plan = core_editor::add_plan(&v, &projects, &sections, &labels, crate::today());
    for name in plan.labels {
        let _ = api::add_label(name).await;
    }
    let result = api::dispatch(&api::command(plan.task, true)).await?;
    if let Some(id) = result.id_mapping.values().next() {
        sync_reminders(id.clone(), &[], &plan.reminders).await;
    }
    Ok(())
}

/// Saves an edited task: the move (with its sub-tree), reminder changes,
/// then the field changes. Fails when the move or the field changes weren't
/// saved.
pub async fn save_edit(id: Id, before: EditorValues, after: EditorValues, existing: Option<Recurrence>) -> Result<(), String> {
    let plan = core_editor::edit_plan(&id, &before, &after, existing, crate::today());
    if let Some(moved) = plan.moved {
        api::dispatch(&api::command(moved, false)).await?;
    }
    for action in plan.reminders {
        let _ = api::dispatch(&api::command(action, false)).await;
    }
    if let Some(update) = plan.update {
        api::dispatch(&api::command(update, false)).await?;
    }
    Ok(())
}

/// Saves reminder edits: removes the ones taken out, adds the new ones.
pub async fn sync_reminders(task_id: Id, before: &[ReminderDraft], after: &[ReminderDraft]) {
    for action in core_editor::reminder_actions(&task_id, before, after) {
        let _ = api::dispatch(&api::command(action, false)).await;
    }
}

#[component]
pub fn TaskEditor(
    initial: EditorValues,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    /// Text for the "keep current rule" repeat option, when the rule is custom.
    keep_label: Option<String>,
    /// The edited task's recurrence (what "keep" repeats by).
    #[props(default)]
    existing: Option<Recurrence>,
    submit_text: String,
    /// Parse the title as natural language (add mode) and preview it.
    #[props(default)]
    quick_add: bool,
    /// Stay open after submitting, reset to `initial` (Todoist's add flow).
    #[props(default)]
    keep_open: bool,
    on_submit: EventHandler<EditorValues>,
    on_cancel: EventHandler<()>,
    reload: Signal<u32>,
) -> Element {
    let start = initial.clone();
    let mut vals = use_signal(move || start);
    // The lists quick add resolves #project and /section against, kept
    // current as projects and sections change (a signal, so the handlers
    // below stay Copy).
    let mut lists = use_signal(|| (projects.clone(), sections.clone()));
    {
        let (projects, sections) = (projects.clone(), sections.clone());
        use_effect(use_reactive!(|(projects, sections)| lists.set((projects, sections))));
    }
    // This editor's id: Backspace in its title reaches it, not another one.
    let editor_id = use_hook(|| {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    });
    // Backspace right after a highlight turns it back into text
    // (UNHIGHLIGHT_KEY_JS reports the caret, in UTF-16 units).
    use_hook(move || {
        if quick_add {
            spawn(async move {
                let mut asks = document::eval(&UNHIGHLIGHT_JS.replace("$ID", &editor_id.to_string()));
                while let Ok(at) = asks.recv::<usize>().await {
                    let mut v = vals();
                    let (projects, sections) = lists();
                    let end = crate::glass::byte_at_utf16(&v.content, at);
                    if unhighlight(&mut v, &projects, &sections, crate::today(), end) {
                        vals.set(v);
                    }
                }
            });
        }
    });
    use_drop(move || {
        let _ = document::eval(&format!("delete window.__keelUnhighlight?.['{editor_id}'];"));
    });
    // A chip pick, applied the same way as on the iPhone card.
    let mut pick = move |p: Pick| {
        let (projects, sections) = lists();
        vals.with_mut(|v| apply_pick(v, &p, quick_add, &projects, &sections, crate::today()));
    };
    let mut toggle = move |name: String| {
        let (projects, sections) = lists();
        vals.with_mut(|v| toggle_label(v, &name, quick_add, &projects, &sections, crate::today()));
    };
    let blank = initial.clone();
    let blank = use_signal(move || blank);
    let mut open: OpenPop = use_signal(|| None);
    let v = vals();
    let can_submit = has_name(&v, quick_add, &projects, &sections, crate::today());
    let label_text = labels_text(&v.labels);
    let today = crate::today();

    let location = v.location();
    let location_items = location_items(&projects, &sections);
    let location_text = location_text(&v.project_id, v.section_id.as_ref(), &projects, &sections);
    let priority_items = priority_items();
    let priority_text = priority_text(&v.priority);

    let repeat_opts = repeat_options(&v.date, keep_label.as_deref(), today);
    let repeat_text_chip = repeat_chip_text(&v.repeat, &repeat_opts);
    let repeat_items: Vec<MenuItem> = repeat_opts.into_iter().map(|(value, label)| MenuItem::new(value, label)).collect();

    // Live quick-add parse: highlighted spans in the title + preview chips.
    let qa = quick_add.then(|| parse_title(&v, &projects, &sections));
    let spans = qa.as_ref().map(|q| q.spans.clone()).unwrap_or_default();
    // The labels the task will get: picked plus typed (the menu ticks both).
    let mut shown_labels = v.labels.clone();
    for l in qa.iter().flat_map(|q| q.labels.iter()) {
        if !shown_labels.iter().any(|n| n.eq_ignore_ascii_case(l)) {
            shown_labels.push(l.clone());
        }
    }
    let preview: Vec<(&'static str, String)> = qa
        .as_ref()
        .map(|qa| core_editor::preview_chips(qa, &projects, &sections, today))
        .unwrap_or_default()
        .into_iter()
        .map(|c| {
            let class = match c.kind {
                SpanKind::Date | SpanKind::Time => "chip-date",
                SpanKind::Repeat => "chip-repeat",
                SpanKind::Priority => "chip-priority",
                SpanKind::Project | SpanKind::Section => "chip-project",
                SpanKind::Label => "chip-label",
                SpanKind::Deadline => "chip-deadline",
            };
            (class, c.text)
        })
        .collect();

    // The days it will repeat on (typed rule, or the Repeat menu's).
    let repeat_rule = qa
        .as_ref()
        .and_then(|q| q.recurrence.clone())
        .or_else(|| recurrence_for_at(&v.repeat, &v.date, existing.clone(), today));
    // Relative reminders count back from a due date (typed or picked).
    let has_due = !v.date.is_empty() || !v.time.is_empty() || qa.as_ref().is_some_and(|q| q.date.is_some());
    let title_placeholder: &'static str =
        if quick_add { "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”" } else { "Task name" };
    // Submits once: a fast second Enter (before the editor closes) is ignored.
    // In keep-open mode the form resets to blank instead (an empty title
    // can't be submitted, so a second Enter is a no-op there too).
    let mut submitted = use_signal(|| false);
    let mut submit = move || {
        let v = vals();
        let (p, s) = lists();
        if submitted() || !has_name(&v, quick_add, &p, &s, crate::today()) {
            return;
        }
        if keep_open {
            vals.set(blank());
            open.set(None);
        } else {
            submitted.set(true);
        }
        on_submit.call(v);
    };

    rsx! {
        form {
            class: "editor",
            onsubmit: move |e| {
                e.prevent_default();
                submit();
            },
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    e.prevent_default();
                    if open().is_some() {
                        open.set(None);
                    } else {
                        on_cancel.call(());
                    }
                }
            },
            if open().is_some() {
                div { class: "pop-backdrop", onclick: move |_| open.set(None), onwheel: move |_| open.set(None) }
            }
            HighlightedTitle {
                editor_id,
                value: v.content.clone(),
                spans,
                placeholder: title_placeholder,
                on_input: move |val: String| vals.with_mut(|v| *v = with_title(v, val)),
                on_enter: move |_| submit(),
            }
            if !preview.is_empty() {
                div { class: "quick-preview",
                    for (i, (class, text)) in preview.iter().enumerate() {
                        span { key: "{i}", class: "quick-chip {class}", "{text}" }
                    }
                }
            }
            input {
                class: "desc",
                placeholder: "Description",
                value: "{v.description}",
                oninput: move |e| {
                    let val = e.value();
                    vals.with_mut(|v| v.description = val);
                },
            }
            div { class: "chips",
                DatePicker {
                    date: v.date.clone(),
                    time: v.time.clone(),
                    open,
                    rule: repeat_rule.clone(),
                    on_change: move |(date, time): (String, String)| pick(Pick::Date { date, time }),
                }
                DeadlinePicker {
                    deadline: v.deadline.clone(),
                    open,
                    on_change: move |date: String| pick(Pick::Deadline { date }),
                }
                ReminderPicker {
                    reminders: v.reminders.clone(),
                    has_due: has_due,
                    open,
                    on_change: move |drafts: Vec<ReminderDraft>| pick(Pick::Reminders { drafts }),
                }
                Menu {
                    id: "priority",
                    title: "Priority",
                    icon: icons::flag_small(),
                    icon_class: flag_class(&v.priority),
                    text: priority_text,
                    items: priority_items,
                    selected: v.priority.clone(),
                    open,
                    on_select: move |value: String| pick(Pick::Priority { value }),
                }
                Menu {
                    id: "repeat",
                    title: "Repeat",
                    icon: icons::repeat_small(),
                    text: repeat_text_chip,
                    items: repeat_items,
                    selected: v.repeat.clone(),
                    open,
                    on_select: move |value: String| pick(Pick::Repeat { value }),
                }
                Menu {
                    id: "location",
                    title: "Project / section",
                    icon: icons::hash_small(),
                    text: location_text,
                    items: location_items,
                    selected: location.clone(),
                    open,
                    on_select: move |val: String| pick(location_pick(&val)),
                }
                LabelPicker {
                    labels: labels.clone(),
                    selected: shown_labels.clone(),
                    text: label_text,
                    open,
                    on_toggle: move |name: String| toggle(name),
                    reload,
                }
            }
            div { class: "editor-footer",
                button {
                    r#type: "button",
                    class: "btn btn-cancel",
                    onclick: move |_| on_cancel.call(()),
                    "Cancel"
                }
                button { r#type: "submit", class: "btn btn-add", disabled: !can_submit, "{submit_text}" }
            }
        }
    }
}

/// Todoist's backspace: in a title field, Backspace right where a
/// highlighted phrase ends un-highlights it instead of deleting. Installed
/// once (App); hands the caret to the open editor (`UNHIGHLIGHT_JS`).
pub const UNHIGHLIGHT_KEY_JS: &str = r#"
document.addEventListener('keydown', (e) => {
  const ta = e.target;
  if (e.key !== 'Backspace' || !ta.classList?.contains('title-input') || ta.selectionStart !== ta.selectionEnd) return;
  const pos = ta.selectionStart;
  let at = 0;
  for (const n of ta.parentElement.querySelector('.title-mirror')?.childNodes ?? []) {
    const len = (n.textContent || '').length;
    if (n.nodeName === 'MARK' && at + len === pos) {
      const send = window.__keelUnhighlight?.[ta.closest('[data-editor]')?.dataset.editor];
      if (send) { e.preventDefault(); send(pos); }
      return;
    }
    at += len;
  }
}, true);
"#;

/// One editor's end of it, keyed by its id (`$ID`): the Backspace goes to
/// the editor whose title it was typed in.
const UNHIGHLIGHT_JS: &str = r#"
(window.__keelUnhighlight ??= {})['$ID'] = (pos) => dioxus.send(pos);
await new Promise(() => {});
"#;

/// The task title: an auto-growing textarea over a mirror that paints the
/// quick-add phrases it recognized. Enter submits (titles are one line).
#[component]
fn HighlightedTitle(
    /// Routes un-highlight Backspaces to this editor.
    editor_id: u32,
    value: String,
    spans: Vec<Span>,
    placeholder: &'static str,
    on_input: EventHandler<String>,
    on_enter: EventHandler<()>,
) -> Element {
    // (text, highlight class) segments covering the whole value.
    let segments: Vec<(String, &'static str)> = core_editor::title_segments(&value, &spans)
        .into_iter()
        .map(|s| {
            let class = match s.kind {
                None => "",
                Some(SpanKind::Date | SpanKind::Time) => "hl hl-date",
                Some(SpanKind::Repeat) => "hl hl-repeat",
                // The flag's color: the level is the phrase's digit ("p2", "!!2").
                Some(SpanKind::Priority) => match s.level {
                    1 => "hl hl-p1",
                    2 => "hl hl-p2",
                    3 => "hl hl-p3",
                    _ => "hl hl-p4",
                },
                Some(SpanKind::Project | SpanKind::Section) => "hl hl-project",
                Some(SpanKind::Label) => "hl hl-label",
                Some(SpanKind::Deadline) => "hl hl-deadline",
            };
            (s.text, class)
        })
        .collect();

    rsx! {
        div { class: "title-wrap", "data-editor": "{editor_id}",
            div { class: "title-mirror", aria_hidden: "true",
                for (i, (text, class)) in segments.iter().enumerate() {
                    if class.is_empty() {
                        span { key: "{i}", "{text}" }
                    } else {
                        mark { key: "{i}", class: "{class}", "{text}" }
                    }
                }
                // Keeps the last line's height when the value ends in a space.
                "\u{200b}"
            }
            textarea {
                class: "name title-input",
                rows: "1",
                placeholder,
                autofocus: true,
                spellcheck: "false",
                value: "{value}",
                oninput: move |e| on_input.call(e.value().replace(['\n', '\r'], " ")),
                onkeydown: move |e| {
                    if e.key() == Key::Enter {
                        e.prevent_default();
                        on_enter.call(());
                    }
                },
            }
        }
    }
}

/// The Labels chip and its menu: tick labels, or type a new one and press
/// Enter (it's created and ticked).
#[component]
pub fn LabelPicker(
    labels: Vec<Label>,
    /// The ticked names.
    selected: Vec<String>,
    /// The chip's text.
    text: String,
    open: OpenPop,
    on_toggle: EventHandler<String>,
    reload: Signal<u32>,
    #[props(default)]
    face: Option<Face>,
) -> Element {
    let mut open = open;
    let mut reload = reload;
    let mut new_label = use_signal(String::new);
    let is_open = open() == Some("labels");
    let btn_class = match &face { Some(f) => f.class.clone(), None => "chip chip-btn".to_string() };
    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: "{btn_class}",
                class: if is_open { "chip-open" },
                title: "Labels",
                onclick: move |_| open.set(if is_open { None } else { Some("labels") }),
                if face.as_ref().is_none_or(|f| f.icon) {
                    span { class: "chip-icon", Icon { svg: icons::tag_small() } }
                }
                match &face {
                    Some(f) => rsx! { "{f.text}" },
                    None => rsx! { "{text}" },
                }
            }
            if is_open {
                div { class: "popover menu label-pop",
                    onmounted: move |_| crate::pickers::place_popover(),
                    if labels.is_empty() {
                        div { class: "label-empty", "No labels yet" }
                    }
                    for l in labels.iter() {
                        LabelOption {
                            key: "{l.id}",
                            name: l.name.clone(),
                            checked: selected.iter().any(|n| n.eq_ignore_ascii_case(&l.name)),
                            on_toggle: move |name: String| on_toggle.call(name),
                        }
                    }
                    input {
                        class: "label-new",
                        placeholder: "New label, then Enter",
                        value: "{new_label}",
                        oninput: move |e| new_label.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                e.prevent_default();
                                let name = new_label().trim().to_string();
                                if name.is_empty() {
                                    return;
                                }
                                if !selected.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                                    on_toggle.call(name.clone());
                                }
                                new_label.set(String::new());
                                spawn(async move {
                                    let _ = api::add_label(name).await;
                                    reload += 1;
                                });
                            }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn LabelOption(name: String, checked: bool, on_toggle: EventHandler<String>) -> Element {
    rsx! {
        label { class: "label-opt",
            input {
                r#type: "checkbox",
                checked,
                onchange: move |_| on_toggle.call(name.clone()),
            }
            Icon { svg: icons::tag_small() }
            "{name}"
        }
    }
}

/// The collapsed "+ Add task" row; expands into a [`TaskEditor`].
#[component]
pub fn AddTask(
    defaults: EditorValues,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
    reload: Signal<u32>,
    /// The view's own row (the floating + opens this one).
    #[props(default)]
    main: bool,
) -> Element {
    let mut reload = reload;
    let mut open = use_signal(|| false);
    let glass = crate::glass::use_glass();

    if !open() {
        let (d, p, s, l) = (defaults.clone(), projects.clone(), sections.clone(), labels.clone());
        return rsx! {
            div {
                class: "add-task-row",
                class: if main { "view-add" },
                onclick: move |_| {
                    if (glass.native)() {
                        glass.add(d.clone(), p.clone(), s.clone(), l.clone(), reload);
                    } else {
                        open.set(true);
                    }
                },
                span { class: "plus", Icon { svg: icons::plus(14) } }
                "Add task"
            }
        };
    }

    let (p2, s2, l2) = (projects.clone(), sections.clone(), labels.clone());
    rsx! {
        TaskEditor {
            initial: defaults.clone(),
            projects,
            sections,
            labels,
            keep_label: None,
            submit_text: "Add task".to_string(),
            quick_add: true,
            keep_open: true,
            on_submit: move |v: EditorValues| {
                let (p, s, l) = (p2.clone(), s2.clone(), l2.clone());
                spawn(async move {
                    let _ = add_from_editor(v, p, s, l).await;
                    reload += 1;
                });
            },
            on_cancel: move |_| open.set(false),
            reload,
        }
    }
}
