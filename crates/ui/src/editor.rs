//! The Todoist-style task editor card, shared by "+ Add task", "Add sub-task"
//! and in-place editing, plus the collapsed "+ Add task" row. When adding, the
//! title is parsed as natural language (dates, times, repeats, p1, #project,
//! /section, @labels) with a live preview.

use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use keel_core::commands::DueEdit;
use keel_core::ids::Id;
use keel_core::model::{Label, Priority, Project, Section, Task};
use keel_core::quickadd::{self, QuickAdd, Span, SpanKind};
use keel_core::recurrence::Recurrence;
use keel_core::views;

use crate::api::{self, NewTask, TaskPatch};
use crate::glass::Pick;
use crate::icons::{self, Icon};
use crate::pickers::{DatePicker, DeadlinePicker, Face, Menu, MenuItem, OpenPop, ReminderDraft, ReminderPicker};

/// Everything the editor edits, as form-friendly values.
#[derive(Clone, PartialEq, Debug)]
pub struct EditorValues {
    pub content: String,
    pub description: String,
    /// `YYYY-MM-DD` or empty.
    pub date: String,
    /// `HH:MM` or empty.
    pub time: String,
    /// "1".."4".
    pub priority: String,
    /// none | daily | weekdays | weekly | monthly | keep (an existing custom rule).
    pub repeat: String,
    pub project_id: Id,
    pub section_id: Option<Id>,
    pub labels: Vec<String>,
    /// Only used when adding a sub-task.
    pub parent_id: Option<Id>,
    /// `YYYY-MM-DD` or empty.
    pub deadline: String,
    pub reminders: Vec<ReminderDraft>,
    /// Byte ranges of `content` turned back into plain text (backspace
    /// right after a highlight); quick add leaves them alone.
    pub plain: Vec<(usize, usize)>,
}

impl EditorValues {
    pub fn blank(project_id: Id, section_id: Option<Id>, date: String, labels: Vec<String>) -> Self {
        EditorValues {
            content: String::new(),
            description: String::new(),
            date,
            time: String::new(),
            priority: "4".to_string(),
            repeat: "none".to_string(),
            project_id,
            section_id,
            labels,
            parent_id: None,
            deadline: String::new(),
            reminders: Vec::new(),
            plain: Vec::new(),
        }
    }

    pub fn from_task(t: &Task) -> Self {
        let rule = t.due.as_ref().and_then(|d| d.recurrence.clone());
        EditorValues {
            content: t.content.clone(),
            description: t.description.clone(),
            date: t.due.as_ref().and_then(|d| d.date.get(..10)).unwrap_or("").to_string(),
            time: t.due.as_ref().and_then(|d| d.date.get(11..16)).unwrap_or("").to_string(),
            priority: (5 - t.priority.to_api()).to_string(),
            repeat: repeat_choice(&rule).to_string(),
            project_id: t.project_id.clone(),
            section_id: t.section_id.clone(),
            labels: t.labels.clone(),
            parent_id: t.parent_id.clone(),
            deadline: t.deadline.clone().unwrap_or_default(),
            reminders: Vec::new(),
            plain: Vec::new(),
        }
    }

    fn priority(&self) -> Priority {
        match self.priority.as_str() {
            "1" => Priority::P1,
            "2" => Priority::P2,
            "3" => Priority::P3,
            _ => Priority::None,
        }
    }

    /// `project` or `project|section`, for the location select.
    pub(crate) fn location(&self) -> String {
        match &self.section_id {
            Some(s) => format!("{}|{}", self.project_id, s),
            None => self.project_id.to_string(),
        }
    }

    /// The `Due.date` string for the chosen date/time ("" = no due). A time
    /// without a date means today.
    fn due_string(&self) -> String {
        match (self.date.is_empty(), self.time.is_empty()) {
            (true, true) => String::new(),
            (false, true) => self.date.clone(),
            (true, false) => format!("{}T{}:00", crate::today().format("%Y-%m-%d"), self.time),
            (false, false) => format!("{}T{}:00", self.date, self.time),
        }
    }
}

/// Which repeat option represents an existing rule.
fn repeat_choice(rule: &Option<Recurrence>) -> &'static str {
    match rule {
        None => "none",
        Some(Recurrence::Daily) => "daily",
        Some(Recurrence::Weekly(days)) if days == &vec![0, 1, 2, 3, 4] => "weekdays",
        Some(Recurrence::Weekly(days)) if days.len() == 1 => "weekly",
        Some(Recurrence::MonthlyDay(_)) => "monthly",
        Some(_) => "keep",
    }
}

/// The rule for a repeat choice; "Every week/month" anchor on `date` (or today).
fn recurrence_for(choice: &str, date: &str, existing: Option<Recurrence>) -> Option<Recurrence> {
    recurrence_for_at(choice, date, existing, crate::today())
}

/// [`recurrence_for`] relative to `today` (testable off the browser).
pub(crate) fn recurrence_for_at(choice: &str, date: &str, existing: Option<Recurrence>, today: NaiveDate) -> Option<Recurrence> {
    let anchor = NaiveDate::parse_from_str(date.get(..10).unwrap_or(date), "%Y-%m-%d").unwrap_or(today);
    match choice {
        "daily" => Some(Recurrence::Daily),
        "weekdays" => Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])),
        "weekly" => Some(Recurrence::Weekly(vec![anchor.weekday().num_days_from_monday() as u8])),
        "monthly" => Some(Recurrence::MonthlyDay(anchor.day() as u8)),
        "keep" => existing,
        _ => None,
    }
}

fn ordinal(n: u8) -> String {
    keel_core::recurrence::ordinal(n as u32)
}

/// Human text for a repeat rule ("Every other week on Wed").
pub fn repeat_text(rule: &Recurrence) -> String {
    let text = rule.describe();
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

/// The Labels chip's text.
pub(crate) fn labels_text(labels: &[String]) -> String {
    if labels.is_empty() { "Labels".to_string() } else { labels.join(", ") }
}

/// The Priority chip's text ("4" is no priority).
pub(crate) fn priority_text(priority: &str) -> String {
    if priority == "4" { "Priority".to_string() } else { format!("P{priority}") }
}

/// The repeat choices as (value, label); week/month options name the day
/// they repeat on, and "keep" appears when the task has a custom rule.
pub(crate) fn repeat_options(date: &str, keep_label: Option<&str>, today: NaiveDate) -> Vec<(&'static str, String)> {
    let anchor = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap_or(today);
    let mut opts = vec![
        ("none", "No repeat".to_string()),
        ("daily", "Every day".to_string()),
        ("weekdays", "Every weekday (Mon – Fri)".to_string()),
        ("weekly", format!("Every week on {}", anchor.format("%A"))),
        ("monthly", format!("Every month on the {}", ordinal(anchor.day() as u8))),
    ];
    if let Some(text) = keep_label {
        opts.push(("keep", text.to_string()));
    }
    opts
}

/// The Repeat chip's text for the chosen option.
pub(crate) fn repeat_chip_text(repeat: &str, opts: &[(&'static str, String)]) -> String {
    opts.iter()
        .find(|(value, _)| *value == repeat && *value != "none")
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| "Repeat".to_string())
}

/// "Project" or "Project / Section" for the location chip.
pub(crate) fn location_text(project_id: &Id, section_id: Option<&Id>, projects: &[Project], sections: &[Section]) -> String {
    let Some(p) = projects.iter().find(|p| &p.id == project_id) else { return "Inbox".to_string() };
    match section_id.and_then(|s| sections.iter().find(|x| &x.id == s && x.project_id == p.id)) {
        Some(s) => format!("{} / {}", p.name, s.name),
        None => p.name.clone(),
    }
}

/// The project / section menu: each project, then its sections indented.
/// Values are `project` or `project|section` ([`location_pick`] reads them).
pub(crate) fn location_items(projects: &[Project], sections: &[Section]) -> Vec<MenuItem> {
    let mut items = Vec::new();
    for p in projects.iter() {
        let icon = if p.is_inbox { icons::inbox() } else { icons::hash_small() };
        let class = if p.is_inbox { "loc-inbox" } else { "loc-project" };
        items.push(MenuItem::new(p.id.to_string(), p.name.clone()).icon(icon, class));
        for s in sections.iter().filter(|s| s.project_id == p.id) {
            let value = format!("{}|{}", p.id, s.id);
            items.push(MenuItem::new(value, s.name.clone()).icon(icons::section_small(), "loc-section").indented());
        }
    }
    items
}

/// The location menu's choice as a pick.
pub(crate) fn location_pick(value: &str) -> Pick {
    match value.split_once('|') {
        Some((p, s)) => Pick::Location { project_id: Id::from(p), section_id: Some(Id::from(s)) },
        None => Pick::Location { project_id: Id::from(value), section_id: None },
    }
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
        .map(|p| MenuItem::new(*p, format!("Priority {p}")).icon(icons::flag_small(), flag_class(p)))
        .collect()
}

/// Field changes between `before` and `after`, plus a move when the
/// project/section changed (moves are a separate command that also carries
/// the sub-tree).
pub fn changes(
    before: &EditorValues,
    after: &EditorValues,
    existing: Option<Recurrence>,
) -> (TaskPatch, Option<(Id, Option<Id>)>) {
    let due_changed = before.date != after.date || before.time != after.time || before.repeat != after.repeat;
    let due = after.due_string();
    let patch = TaskPatch {
        content: (after.content.trim() != before.content).then(|| after.content.trim().to_string()),
        description: (after.description != before.description).then(|| after.description.clone()),
        priority: (after.priority != before.priority).then(|| after.priority()),
        project_id: None,
        labels: (after.labels != before.labels).then(|| after.labels.clone()),
        deadline: (after.deadline != before.deadline).then(|| after.deadline.clone()),
        due: due_changed.then(|| DueEdit {
            date: (!due.is_empty()).then(|| due.clone()),
            recurrence: recurrence_for(&after.repeat, &due, existing),
        }),
    };
    let moved = (after.project_id != before.project_id || after.section_id != before.section_id)
        .then(|| (after.project_id.clone(), after.section_id.clone()));
    (patch, moved)
}

/// Parses the editor's title as quick add.
fn parse_title(v: &EditorValues, projects: &[Project], sections: &[Section]) -> QuickAdd {
    parse_title_at(v, projects, sections, crate::today())
}

/// Whether the title has a name to save: with quick add, something besides
/// the recognized phrases ("This weekend" alone highlights but can't be
/// added yet).
pub(crate) fn has_name(v: &EditorValues, quick_add: bool, projects: &[Project], sections: &[Section], today: NaiveDate) -> bool {
    if quick_add {
        !parse_title_at(v, projects, sections, today).content.trim().is_empty()
    } else {
        !v.content.trim().is_empty()
    }
}

/// The typed phrases a pick replaces.
fn kinds_for(pick: &Pick) -> &'static [SpanKind] {
    match pick {
        Pick::Date { .. } => &[SpanKind::Date, SpanKind::Time],
        Pick::Priority { .. } => &[SpanKind::Priority],
        Pick::Labels { .. } => &[SpanKind::Label],
        Pick::Reminders { .. } => &[],
        Pick::Deadline { .. } => &[SpanKind::Deadline],
        Pick::Repeat { .. } => &[SpanKind::Repeat],
        Pick::Location { .. } => &[SpanKind::Project, SpanKind::Section],
    }
}

/// Applies a chip pick. With quick add, the typed phrase the pick replaces
/// comes out of the title (Todoist-style), so the web editor and the iPhone
/// card save the same input the same way. A different day also replaces a
/// typed repeat (it set the date); a time for the same day keeps it.
pub(crate) fn apply_pick(
    v: &mut EditorValues,
    pick: &Pick,
    quick_add: bool,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) {
    if quick_add {
        let qa = parse_title_at(v, projects, sections, today);
        let mut kinds = kinds_for(pick).to_vec();
        if let Pick::Date { date, .. } = pick {
            let shown = qa.date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| v.date.clone());
            if *date != shown {
                kinds.push(SpanKind::Repeat);
            }
        }
        // Through `with_title`, so un-highlighted ranges move with the text.
        *v = with_title(v, quickadd::strip_spans(&v.content, &qa.spans, &kinds));
    }
    match pick {
        Pick::Date { date, time } => (v.date, v.time) = (date.clone(), time.clone()),
        Pick::Priority { value } => v.priority = value.clone(),
        Pick::Labels { names } => v.labels = names.clone(),
        Pick::Reminders { drafts } => v.reminders = drafts.clone(),
        Pick::Deadline { date } => v.deadline = date.clone(),
        Pick::Repeat { value } => v.repeat = value.clone(),
        Pick::Location { project_id, section_id } => (v.project_id, v.section_id) = (project_id.clone(), section_id.clone()),
    }
}

/// Toggles one label from the labels menu: the typed @labels count as picked
/// (so taking them out of the title doesn't drop them), then it's applied
/// like the card's labels pick.
pub(crate) fn toggle_label(
    v: &mut EditorValues,
    name: &str,
    quick_add: bool,
    projects: &[Project],
    sections: &[Section],
    today: NaiveDate,
) {
    let mut names = v.labels.clone();
    if quick_add {
        for l in parse_title_at(v, projects, sections, today).labels {
            if !names.iter().any(|n| n.eq_ignore_ascii_case(&l)) {
                names.push(l);
            }
        }
    }
    match names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
        Some(i) => {
            names.remove(i);
        }
        None => names.push(name.to_string()),
    }
    apply_pick(v, &Pick::Labels { names }, quick_add, projects, sections, today);
}

/// The values after the title is retyped: kept-plain phrases follow the
/// text, or are dropped when the edit touches them.
pub(crate) fn with_title(v: &EditorValues, text: String) -> EditorValues {
    EditorValues { plain: quickadd::shift_plain(&v.content, &text, &v.plain), content: text, ..v.clone() }
}

/// Turns the highlighted phrase ending at byte `end` back into plain text
/// (Todoist's backspace). False when no highlight ends there.
pub(crate) fn unhighlight(v: &mut EditorValues, projects: &[Project], sections: &[Section], today: NaiveDate, end: usize) -> bool {
    let qa = parse_title_at(v, projects, sections, today);
    match qa.spans.iter().find(|s| s.end == end) {
        Some(s) => {
            v.plain.push((s.start, s.end));
            true
        }
        None => false,
    }
}

/// [`parse_title`] relative to `today` (testable off the browser).
pub(crate) fn parse_title_at(v: &EditorValues, projects: &[Project], sections: &[Section], today: NaiveDate) -> QuickAdd {
    let mut qa = quickadd::parse_keeping(&v.content, today, projects, sections, &v.plain);
    // A bare /Section resolves within the Inbox; carry its project along.
    if qa.project_id.is_none() {
        if let Some(s) = qa.section_id.as_ref().and_then(|id| sections.iter().find(|s| &s.id == id)) {
            qa.project_id = Some(s.project_id.clone());
        }
    }
    qa
}

/// The new task described by the editor values with the quick-add parse
/// applied on top (text wins over chips for the fields it mentions).
fn new_task_from(v: &EditorValues, qa: &QuickAdd, today: NaiveDate) -> NewTask {
    // A typed date wins; a picked time goes with it unless a time was typed.
    let date = match (qa.date, qa.time) {
        (Some(d), None) => match chrono::NaiveTime::parse_from_str(&v.time, "%H:%M") {
            Ok(t) => format!("{}T{}", d.format("%Y-%m-%d"), t.format("%H:%M:00")),
            Err(_) => d.format("%Y-%m-%d").to_string(),
        },
        _ => qa.due_string().unwrap_or_else(|| v.due_string()),
    };
    let recurrence = qa.recurrence.clone().or_else(|| recurrence_for_at(&v.repeat, &date, None, today));
    let mut labels = v.labels.clone();
    for l in &qa.labels {
        if !labels.iter().any(|x| x.eq_ignore_ascii_case(l)) {
            labels.push(l.clone());
        }
    }
    let (project_id, section_id) = match (&qa.project_id, &qa.section_id) {
        (Some(p), s) => (p.clone(), s.clone()),
        (None, _) => (v.project_id.clone(), v.section_id.clone()),
    };
    NewTask {
        content: qa.content.trim().to_string(),
        description: v.description.clone(),
        project_id,
        recurrence,
        date,
        priority: qa.priority.unwrap_or_else(|| v.priority()),
        labels,
        section_id,
        parent_id: v.parent_id.clone(),
        deadline: qa.deadline.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| v.deadline.clone()),
    }
}

/// Adds the task the editor describes (quick add applied), creating any
/// labels it mentions that don't exist yet. Fails when the task wasn't added.
pub async fn add_from_editor(
    v: EditorValues,
    projects: Vec<Project>,
    sections: Vec<Section>,
    labels: Vec<Label>,
) -> Result<(), String> {
    let qa = parse_title(&v, &projects, &sections);
    let task = new_task_from(&v, &qa, crate::today());
    for name in &qa.labels {
        if !labels.iter().any(|l| l.name.eq_ignore_ascii_case(name)) {
            let _ = api::add_label(name.clone()).await;
        }
    }
    let result = api::add_task(task).await?;
    if let Some(id) = result.id_mapping.values().next() {
        sync_reminders(id.clone(), &[], &v.reminders).await;
    }
    Ok(())
}

/// Saves an edited task: the move (with its sub-tree), reminder changes,
/// then the field changes. Fails when the move or the field changes weren't
/// saved.
pub async fn save_edit(id: Id, before: EditorValues, after: EditorValues, existing: Option<Recurrence>) -> Result<(), String> {
    let (patch, moved) = changes(&before, &after, existing);
    if let Some((project, section)) = moved {
        api::move_task(id.clone(), project, section).await?;
    }
    sync_reminders(id.clone(), &before.reminders, &after.reminders).await;
    if !patch.is_empty() {
        api::update_task(id, patch).await?;
    }
    Ok(())
}

/// Saves reminder edits: removes the ones taken out, adds the new ones.
pub async fn sync_reminders(task_id: Id, before: &[ReminderDraft], after: &[ReminderDraft]) {
    for old in before {
        if let Some(id) = &old.id {
            if !after.iter().any(|a| a.id.as_ref() == Some(id)) {
                let _ = api::delete_reminder(id.clone()).await;
            }
        }
    }
    for new in after.iter().filter(|a| a.id.is_none()) {
        let _ = api::add_reminder(task_id.clone(), new.minute_offset, new.at.clone()).await;
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
    let preview: Vec<(&'static str, String)> = match &qa {
        Some(qa) => {
            let mut chips = Vec::new();
            if let Some(d) = qa.date {
                let mut text = views::due_label(d, today);
                if let Some(t) = qa.time {
                    text = format!("{text} {}", views::time_label(t));
                }
                chips.push(("chip-date", text));
            }
            if let Some(r) = &qa.recurrence {
                chips.push(("chip-repeat", repeat_text(r)));
            }
            if let Some(p) = qa.priority {
                chips.push(("chip-priority", format!("P{}", 5 - p.to_api())));
            }
            if let Some(pid) = &qa.project_id {
                let project = projects.iter().find(|p| &p.id == pid).map(|p| p.name.clone()).unwrap_or_default();
                let text = match qa.section_id.as_ref().and_then(|s| sections.iter().find(|x| &x.id == s)) {
                    Some(section) => format!("#{project} / {}", section.name),
                    None => format!("#{project}"),
                };
                chips.push(("chip-project", text));
            }
            for l in &qa.labels {
                chips.push(("chip-label", format!("@{l}")));
            }
            if let Some(d) = qa.deadline {
                chips.push(("chip-deadline", format!("Deadline {}", views::due_label(d, today))));
            }
            chips
        }
        None => Vec::new(),
    };

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
    let mut segments: Vec<(String, &'static str)> = Vec::new();
    let mut at = 0;
    for s in spans.iter().filter(|s| s.end <= value.len() && value.is_char_boundary(s.start) && value.is_char_boundary(s.end)) {
        if s.start < at {
            continue;
        }
        if s.start > at {
            segments.push((value[at..s.start].to_string(), ""));
        }
        let class = match s.kind {
            SpanKind::Date | SpanKind::Time => "hl hl-date",
            SpanKind::Repeat => "hl hl-repeat",
            // The flag's color: the level is the phrase's digit ("p2", "!!2").
            SpanKind::Priority => match value[s.start..s.end].chars().rev().find(|c| c.is_ascii_digit()) {
                Some('1') => "hl hl-p1",
                Some('2') => "hl hl-p2",
                Some('3') => "hl hl-p3",
                _ => "hl hl-p4",
            },
            SpanKind::Project | SpanKind::Section => "hl hl-project",
            SpanKind::Label => "hl hl-label",
            SpanKind::Deadline => "hl hl-deadline",
        };
        segments.push((value[s.start..s.end].to_string(), class));
        at = s.end;
    }
    if at < value.len() {
        segments.push((value[at..].to_string(), ""));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_changes_leave_quick_add_text_alone() {
        let mut task = Task::new("Buy p1 batteries #2", Id::from("P-IN"));
        task.id = Id::from("T-1");
        let before = EditorValues::from_task(&task);
        let mut after = before.clone();
        after.priority = "1".to_string();
        let (patch, moved) = changes(&before, &after, None);
        assert_eq!(patch.content, None);
        assert_eq!(patch.priority, Some(Priority::P1));
        assert_eq!(moved, None);
    }

    fn day19() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()
    }

    fn typed(content: &str) -> EditorValues {
        let mut v = EditorValues::blank(Id::from("P-IN"), None, String::new(), Vec::new());
        v.content = content.into();
        v
    }

    #[test]
    fn a_web_pick_replaces_the_typed_phrase_like_the_card() {
        let mut v = typed("call mom tomorrow");
        let pick = crate::glass::Pick::Date { date: "2026-10-23".into(), time: String::new() };
        apply_pick(&mut v, &pick, true, &[], &[], day19());
        assert_eq!(v.content, "call mom");
        let task = new_task_from(&v, &parse_title_at(&v, &[], &[], day19()), day19());
        assert_eq!(task.date, "2026-10-23", "the pick is what's saved");
        // Editing doesn't parse the title, so nothing is stripped.
        let mut e = typed("call mom tomorrow");
        apply_pick(&mut e, &pick, false, &[], &[], day19());
        assert_eq!(e.content, "call mom tomorrow");
    }

    #[test]
    fn toggling_a_label_keeps_the_typed_ones() {
        let mut v = typed("x @home");
        toggle_label(&mut v, "work", true, &[], &[], day19());
        assert_eq!(v.content, "x");
        assert_eq!(v.labels, vec!["home".to_string(), "work".to_string()]);
        toggle_label(&mut v, "HOME", true, &[], &[], day19());
        assert_eq!(v.labels, vec!["work".to_string()]);
    }

    #[test]
    fn a_picked_time_is_saved_with_a_typed_date() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut v = EditorValues::blank(Id::from("P-IN"), None, String::new(), Vec::new());
        v.content = "water plants every monday".into();
        v.date = "2026-10-19".into();
        v.time = "17:00".into();
        let task = new_task_from(&v, &parse_title_at(&v, &[], &[], today), today);
        assert_eq!(task.date, "2026-10-19T17:00:00");
        assert!(task.recurrence.is_some(), "the repeat is kept");
    }
}
