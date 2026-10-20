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
        None => "No date".to_string(),
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
