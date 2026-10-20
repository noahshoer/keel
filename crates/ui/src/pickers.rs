//! Todoist-style popovers for the task editor: a scheduler (quick picks,
//! month grid, time) and a generic menu used for priority, repeat and project.
//! One popover is open at a time per editor (`open` holds its id); the editor
//! draws a transparent backdrop that closes it.

use chrono::{Datelike, Duration, Months, NaiveDate, NaiveTime};
use dioxus::prelude::*;
use keel_core::{calendar, quickadd, views};

use crate::icons::{self, Icon};

/// Which popover an editor has open.
pub type OpenPop = Signal<Option<&'static str>>;

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
    "November", "December",
];
const DOW: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Places the open popover next to its chip, inside the viewport: below the
/// chip, flipped above when there's no room, shifted left at the right edge.
/// (Fixed positioning, so scrolling containers never clip it.)
const PLACE_JS: &str = r#"
const p = document.querySelector('.popover');
if (p && p.previousElementSibling) {
  const c = p.previousElementSibling.getBoundingClientRect();
  const r = p.getBoundingClientRect();
  const m = 8, vw = innerWidth, vh = innerHeight;
  let top = c.bottom + 6;
  if (top + r.height > vh - m && c.top - 6 - r.height >= m) top = c.top - 6 - r.height;
  top = Math.max(m, Math.min(top, vh - m - r.height));
  const left = Math.max(m, Math.min(c.left, vw - m - r.width));
  Object.assign(p.style, { position: 'fixed', top: top + 'px', left: left + 'px', right: 'auto', margin: '0' });
}
"#;

pub fn place_popover() {
    let _ = document::eval(PLACE_JS);
}

/// The due chip's color class for a date (matches task rows).
pub fn due_class(d: NaiveDate, today: NaiveDate) -> &'static str {
    match views::due_bucket(d, today) {
        views::DueBucket::Overdue => "overdue",
        views::DueBucket::Today => "today",
        views::DueBucket::Tomorrow => "tomorrow",
        views::DueBucket::ThisWeek => "week",
        views::DueBucket::Later => "later",
    }
}

/// The scheduler chip + popover. `date` is `YYYY-MM-DD` or empty; `time` is
/// `HH:MM` or empty.
#[component]
pub fn DatePicker(date: String, time: String, open: OpenPop, on_change: EventHandler<(String, String)>) -> Element {
    let mut open = open;
    let today = crate::today();
    let selected = NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok();
    let current_time = NaiveTime::parse_from_str(&time, "%H:%M").ok();
    let is_open = open() == Some("date");

    let (chip_text, chip_class) = match (selected, current_time) {
        (Some(d), Some(t)) => (format!("{} {}", views::due_label(d, today), views::time_label(t)), due_class(d, today)),
        (Some(d), None) => (views::due_label(d, today), due_class(d, today)),
        (None, Some(t)) => (format!("Today {}", views::time_label(t)), "today"),
        (None, None) => ("Date".to_string(), ""),
    };

    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: "chip chip-btn due-chip {chip_class}",
                class: if is_open { "chip-open" },
                title: "Date",
                onclick: move |_| open.set(if is_open { None } else { Some("date") }),
                Icon { svg: icons::calendar_small() }
                "{chip_text}"
            }
            if is_open {
                Scheduler { selected, time: current_time, on_change, open }
            }
        }
    }
}

#[component]
fn Scheduler(
    selected: Option<NaiveDate>,
    time: Option<NaiveTime>,
    on_change: EventHandler<(String, String)>,
    open: OpenPop,
    /// Show the time field (off for deadlines).
    #[props(default = true)]
    with_time: bool,
    /// Header when nothing is picked.
    #[props(default = "No date")]
    empty_text: &'static str,
) -> Element {
    let mut open = open;
    let today = crate::today();
    let mut month = use_signal(move || selected.unwrap_or(today).with_day(1).expect("day 1"));
    let mut time_text = use_signal(move || time.map(views::time_label).unwrap_or_default());
    let mut time_bad = use_signal(|| false);
    let time_str = time.map(|t| t.format("%H:%M").to_string()).unwrap_or_default();

    let saturday = today + Duration::days(((5 - today.weekday().num_days_from_monday() as i64) + 7) % 7);
    let next_monday = today + Duration::days(7 - today.weekday().num_days_from_monday() as i64);
    let tomorrow = today + Duration::days(1);
    let quick: Vec<(&str, String, NaiveDate, String, &str)> = vec![
        ("Today", DOW[today.weekday().num_days_from_monday() as usize].to_string(), today, icons::calendar_small(), "q-today"),
        ("Tomorrow", DOW[tomorrow.weekday().num_days_from_monday() as usize].to_string(), tomorrow, icons::sun(), "q-tomorrow"),
        ("This weekend", DOW[5].to_string(), saturday, icons::sofa(), "q-weekend"),
        ("Next week", format!("Mon {} {}", next_monday.day(), &MONTHS[next_monday.month0() as usize][..3]), next_monday, icons::arrow_right(), "q-next"),
    ];

    let m = month();
    let weeks = calendar::month_weeks(m);
    let header = match selected {
        Some(d) => views::due_label(d, today),
        None => empty_text.to_string(),
    };

    let pick = move |d: Option<NaiveDate>, t: String| {
        on_change.call((d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(), t));
    };

    let commit_time = {
        let date_for_time = selected;
        move || {
            let text = time_text();
            if text.trim().is_empty() {
                time_bad.set(false);
                pick(date_for_time, String::new());
                return true;
            }
            match quickadd::parse_time(&text) {
                Some(t) => {
                    time_bad.set(false);
                    time_text.set(views::time_label(t));
                    // A time without a date means today.
                    pick(Some(date_for_time.unwrap_or(today)), t.format("%H:%M").to_string());
                    true
                }
                None => {
                    time_bad.set(true);
                    false
                }
            }
        }
    };
    let mut commit_blur = commit_time.clone();
    let mut commit_key = commit_time;

    rsx! {
        div { class: "popover scheduler",
            onmounted: move |_| place_popover(),
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    e.stop_propagation();
                    open.set(None);
                }
            },
            div { class: "sched-head", "{header}" }
            div { class: "sched-quick",
                for (label, hint, d, icon, cls) in quick {
                    {
                        let ts = time_str.clone();
                        rsx! {
                            button {
                                key: "{label}",
                                r#type: "button",
                                class: "menu-item",
                                onclick: move |_| {
                                    pick(Some(d), ts.clone());
                                    open.set(None);
                                },
                                span { class: "menu-icon {cls}", Icon { svg: icon } }
                                span { class: "menu-label", "{label}" }
                                span { class: "menu-hint", "{hint}" }
                            }
                        }
                    }
                }
                button {
                    r#type: "button",
                    class: "menu-item",
                    onclick: move |_| {
                        time_text.set(String::new());
                        pick(None, String::new());
                        open.set(None);
                    },
                    span { class: "menu-icon q-none", Icon { svg: icons::no_date() } }
                    span { class: "menu-label", "No date" }
                }
            }
            div { class: "sched-cal",
                div { class: "cal-nav",
                    span { class: "cal-title", "{MONTHS[m.month0() as usize]} {m.year()}" }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Previous month",
                        disabled: m <= today.with_day(1).expect("day 1"),
                        onclick: move |_| month.set(m - Months::new(1)),
                        Icon { svg: icons::chevron_left() }
                    }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Today",
                        onclick: move |_| month.set(today.with_day(1).expect("day 1")),
                        span { class: "cal-dot" }
                    }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Next month",
                        onclick: move |_| month.set(m + Months::new(1)),
                        Icon { svg: icons::chevron_right() }
                    }
                }
                div { class: "cal-grid",
                    for d in ["M", "T", "W", "T", "F", "S", "S"].iter().enumerate() {
                        span { key: "h{d.0}", class: "cal-dow", "{d.1}" }
                    }
                    for (i, day) in weeks.iter().flatten().copied().enumerate() {
                        {
                            let ts = time_str.clone();
                            let mut cls = String::from("cal-day");
                            if day.month() != m.month() { cls.push_str(" other"); }
                            if day < today { cls.push_str(" past"); }
                            if day == today { cls.push_str(" is-today"); }
                            if Some(day) == selected { cls.push_str(" selected"); }
                            rsx! {
                                button {
                                    key: "{i}",
                                    r#type: "button",
                                    class: "{cls}",
                                    onclick: move |_| {
                                        pick(Some(day), ts.clone());
                                        open.set(None);
                                    },
                                    "{day.day()}"
                                }
                            }
                        }
                    }
                }
            }
            if with_time {
                div { class: "sched-time",
                    Icon { svg: icons::clock_small() }
                    input {
                        class: if time_bad() { "time-input bad" } else { "time-input" },
                        placeholder: "Add time, e.g. 5pm or 17:30",
                        value: "{time_text}",
                        oninput: move |e| {
                            time_bad.set(false);
                            time_text.set(e.value());
                        },
                        onblur: move |_| {
                            commit_blur();
                        },
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                e.prevent_default();
                                e.stop_propagation();
                                if commit_key() {
                                    open.set(None);
                                }
                            }
                        },
                    }
                    if time.is_some() {
                        button {
                            r#type: "button",
                            class: "icon-btn",
                            title: "Remove time",
                            onclick: move |_| {
                                time_text.set(String::new());
                                pick(selected, String::new());
                            },
                            "×"
                        }
                    }
                }
            }
        }
    }
}

/// The deadline chip + date-only scheduler. `deadline` is `YYYY-MM-DD` or empty.
#[component]
pub fn DeadlinePicker(deadline: String, open: OpenPop, on_change: EventHandler<String>) -> Element {
    let mut open = open;
    let today = crate::today();
    let selected = NaiveDate::parse_from_str(&deadline, "%Y-%m-%d").ok();
    let is_open = open() == Some("deadline");
    let (text, class) = match selected {
        Some(d) => (views::due_label(d, today), if d <= today { "deadline-chip urgent" } else { "deadline-chip set" }),
        None => ("Deadline".to_string(), "deadline-chip"),
    };
    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: "chip chip-btn {class}",
                class: if is_open { "chip-open" },
                title: "Deadline: the date it must be done by",
                onclick: move |_| open.set(if is_open { None } else { Some("deadline") }),
                Icon { svg: icons::target_small() }
                "{text}"
            }
            if is_open {
                Scheduler {
                    selected,
                    time: None,
                    with_time: false,
                    empty_text: "No deadline",
                    on_change: move |(date, _): (String, String)| on_change.call(date),
                    open,
                }
            }
        }
    }
}

/// A reminder being edited: saved ones have an `id`; new ones don't yet.
#[derive(Clone, Debug, PartialEq)]
pub struct ReminderDraft {
    pub id: Option<keel_core::ids::Id>,
    /// Minutes before the due time (relative).
    pub minute_offset: Option<i64>,
    /// `YYYY-MM-DDTHH:MM` (absolute).
    pub at: Option<String>,
}

impl ReminderDraft {
    pub fn from_reminder(r: &keel_core::model::Reminder) -> Self {
        let relative = r.kind == keel_core::model::ReminderType::Relative;
        ReminderDraft {
            id: Some(r.id.clone()),
            minute_offset: if relative { r.minute_offset } else { None },
            at: if relative { None } else { r.at().map(|a| a.format("%Y-%m-%dT%H:%M").to_string()) },
        }
    }

    /// "At time of task", "30 min before", "Tuesday 9 AM".
    pub fn label(&self, today: NaiveDate) -> String {
        if let Some(m) = self.minute_offset {
            return offset_text(m);
        }
        match self.at.as_deref().and_then(|a| chrono::NaiveDateTime::parse_from_str(a, "%Y-%m-%dT%H:%M").ok()) {
            Some(at) => format!("{} {}", views::due_label(at.date(), today), views::time_label(at.time())),
            None => "Reminder".to_string(),
        }
    }
}

/// "At time of task", "10 min before", "1 hour before", "1 day before".
pub fn offset_text(m: i64) -> String {
    match m {
        0 => "At time of task".to_string(),
        m if m % 1440 == 0 => {
            let d = m / 1440;
            format!("{d} day{} before", if d == 1 { "" } else { "s" })
        }
        m if m % 60 == 0 => {
            let h = m / 60;
            format!("{h} hour{} before", if h == 1 { "" } else { "s" })
        }
        m => format!("{m} min before"),
    }
}

/// The reminders chip + popover: current reminders (removable), presets
/// relative to the due time, and a custom time typed in plain words.
#[component]
pub fn ReminderPicker(
    reminders: Vec<ReminderDraft>,
    /// Whether the task has a due date (relative presets need one).
    has_due: bool,
    open: OpenPop,
    on_change: EventHandler<Vec<ReminderDraft>>,
) -> Element {
    let mut open = open;
    let today = crate::today();
    let is_open = open() == Some("reminders");
    let mut custom = use_signal(String::new);
    let mut custom_bad = use_signal(|| false);
    let text = match reminders.len() {
        0 => "Reminders".to_string(),
        1 => reminders[0].label(today),
        n => format!("{n} reminders"),
    };
    let current = reminders.clone();
    let add = move |draft: ReminderDraft| {
        let mut next = current.clone();
        if !next.iter().any(|r| r.minute_offset == draft.minute_offset && r.at == draft.at) {
            next.push(draft);
        }
        on_change.call(next);
    };
    let add_for_custom = add.clone();
    let mut add_custom = move || {
        // Parsed as if after a title ("x …"): quick add applies nothing to a
        // phrase that would leave the title empty.
        let q = keel_core::quickadd::parse(&format!("x {}", custom()), crate::today(), &[], &[]);
        match (q.date, q.time) {
            (Some(d), Some(t)) => {
                let at = d.and_time(t).format("%Y-%m-%dT%H:%M").to_string();
                add_for_custom(ReminderDraft { id: None, minute_offset: None, at: Some(at) });
                custom.set(String::new());
                custom_bad.set(false);
            }
            _ => custom_bad.set(true),
        }
    };
    let presets = [0i64, 10, 30, 60, 1440];

    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: if reminders.is_empty() { "chip chip-btn" } else { "chip chip-btn reminder-chip" },
                class: if is_open { "chip-open" },
                title: "Reminders",
                onclick: move |_| open.set(if is_open { None } else { Some("reminders") }),
                Icon { svg: icons::bell_small() }
                "{text}"
            }
            if is_open {
                div { class: "popover menu reminder-pop",
                    onmounted: move |_| place_popover(),
                    if reminders.is_empty() {
                        div { class: "label-empty", "No reminders yet" }
                    }
                    for (i, r) in reminders.iter().enumerate() {
                        {
                            let rest: Vec<ReminderDraft> =
                                reminders.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, r)| r.clone()).collect();
                            rsx! {
                                div { key: "{i}", class: "reminder-item",
                                    span { class: "menu-icon", Icon { svg: icons::bell_small() } }
                                    span { class: "menu-label", "{r.label(today)}" }
                                    button {
                                        r#type: "button",
                                        class: "icon-btn",
                                        title: "Remove reminder",
                                        onclick: move |_| on_change.call(rest.clone()),
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "menu-divider" }
                    if !has_due {
                        div { class: "label-empty", "Give the task a date to use these:" }
                    }
                    for m in presets {
                        {
                            let add = add.clone();
                            rsx! {
                                button {
                                    key: "p{m}",
                                    r#type: "button",
                                    class: "menu-item",
                                    disabled: !has_due,
                                    onclick: move |_| add(ReminderDraft { id: None, minute_offset: Some(m), at: None }),
                                    span { class: "menu-icon", Icon { svg: icons::plus(14) } }
                                    span { class: "menu-label", "{offset_text(m)}" }
                                }
                            }
                        }
                    }
                    input {
                        class: if custom_bad() { "label-new bad" } else { "label-new" },
                        placeholder: "Custom, e.g. tomorrow 9am, then Enter",
                        value: "{custom}",
                        oninput: move |e| {
                            custom_bad.set(false);
                            custom.set(e.value());
                        },
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                e.prevent_default();
                                e.stop_propagation();
                                add_custom();
                            }
                        },
                    }
                    if custom_bad() {
                        div { class: "filter-problem", "Needs a time, like “friday 3pm” or “in 2 days at 9am”." }
                    }
                }
            }
        }
    }
}

/// One menu entry.
#[derive(Clone, PartialEq)]
pub struct MenuItem {
    pub value: String,
    pub label: String,
    /// Inline SVG for the item's icon, if any.
    pub icon: Option<String>,
    /// Extra class on the icon (e.g. a priority color).
    pub icon_class: &'static str,
    /// Nested one level (sections under their project).
    pub indent: bool,
}

impl MenuItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        MenuItem { value: value.into(), label: label.into(), icon: None, icon_class: "", indent: false }
    }
    pub fn icon(mut self, svg: String, class: &'static str) -> Self {
        self.icon = Some(svg);
        self.icon_class = class;
        self
    }
    pub fn indented(mut self) -> Self {
        self.indent = true;
        self
    }
}

/// A chip that opens a list of choices; the current one is checked.
#[component]
pub fn Menu(
    id: &'static str,
    title: &'static str,
    /// The chip's icon (inline SVG) and its color class.
    icon: String,
    #[props(default)] icon_class: &'static str,
    /// The chip's text.
    text: String,
    items: Vec<MenuItem>,
    selected: String,
    open: OpenPop,
    on_select: EventHandler<String>,
) -> Element {
    let mut open = open;
    let is_open = open() == Some(id);
    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: "chip chip-btn",
                class: if is_open { "chip-open" },
                title,
                onclick: move |_| open.set(if is_open { None } else { Some(id) }),
                span { class: "chip-icon {icon_class}", Icon { svg: icon } }
                "{text}"
                span { class: "chip-caret", Icon { svg: icons::chevron_down_small() } }
            }
            if is_open {
                div { class: "popover menu",
                    onmounted: move |_| place_popover(),
                    for item in items.iter().cloned() {
                        {
                            let checked = item.value == selected;
                            let value = item.value.clone();
                            rsx! {
                                button {
                                    key: "{item.value}",
                                    r#type: "button",
                                    class: "menu-item",
                                    class: if item.indent { "indent" },
                                    class: if checked { "checked" },
                                    onclick: move |_| {
                                        on_select.call(value.clone());
                                        open.set(None);
                                    },
                                    if let Some(svg) = item.icon.clone() {
                                        span { class: "menu-icon {item.icon_class}", Icon { svg } }
                                    }
                                    span { class: "menu-label", "{item.label}" }
                                    if checked {
                                        span { class: "menu-check", Icon { svg: icons::check_small() } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
