//! Todoist-style popovers for the task editor: a scheduler (quick picks,
//! month grid, time) and a generic menu used for priority, repeat and project.
//! One popover is open at a time per editor (`open` holds its id); the editor
//! draws a transparent backdrop that closes it.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use dioxus::prelude::*;
use keel_core::{calendar, quickadd, views};

use crate::icons::{self, Icon};

/// Which popover an editor has open.
pub type OpenPop = Signal<Option<&'static str>>;

/// How a picker's button looks outside the editor's chip row: the task
/// view's field rows and dotted "+ …" pills. The popover is the same.
#[derive(Clone, PartialEq, Debug)]
pub struct Face {
    /// The button's classes, in place of the chip's.
    pub class: String,
    /// The button's text, in place of the chip's.
    pub text: String,
    /// Shows the picker's icon before the text.
    pub icon: bool,
}

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

/// A native `<input type="time">` value: `Some(None)` when cleared,
/// `None` when it isn't `HH:MM[:SS]`.
fn native_time(v: &str) -> Option<Option<NaiveTime>> {
    if v.is_empty() {
        return Some(None);
    }
    NaiveTime::parse_from_str(v, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(v, "%H:%M:%S"))
        .ok()
        .map(Some)
}

/// True on touch screens (phones), where pickers use native controls.
/// The device's first day of the week (from its locale, read at startup):
/// the calendar's first column on every platform.
static WEEK_START: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

pub fn week_start() -> chrono::Weekday {
    // 1 = Monday … 7 = Sunday (Intl.Locale weekInfo's firstDay).
    match WEEK_START.load(std::sync::atomic::Ordering::Relaxed) {
        7 => chrono::Weekday::Sun,
        6 => chrono::Weekday::Sat,
        _ => chrono::Weekday::Mon,
    }
}

/// Reads the locale's first weekday once (call from App).
pub fn provide_week_start() {
    use_hook(|| {
        spawn(async {
            let mut e = document::eval(
                "const l = new Intl.Locale(navigator.language || 'en-US'); \
                 const w = (l.getWeekInfo ? l.getWeekInfo() : l.weekInfo); dioxus.send(w ? w.firstDay : 7);",
            );
            if let Ok(day) = e.recv::<u8>().await {
                WEEK_START.store(day, std::sync::atomic::Ordering::Relaxed);
            }
        });
    });
}

#[derive(Clone, Copy)]
pub struct Touch(pub Signal<bool>);

const TOUCH_JS: &str = "dioxus.send(matchMedia('(pointer: coarse)').matches); await new Promise(() => {});";

/// Provides [`Touch`] to the app (call once, in `App`).
pub fn provide_touch() {
    let mut touch = use_context_provider(|| Touch(Signal::new(false))).0;
    use_hook(move || {
        spawn(async move {
            if let Ok(t) = document::eval(TOUCH_JS).recv::<bool>().await {
                touch.set(t);
            }
        });
    });
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
/// The Date chip's text and due class ("" when no date is set).
pub fn date_chip(date: Option<NaiveDate>, time: Option<NaiveTime>, today: NaiveDate) -> (String, &'static str) {
    match (date, time) {
        (Some(d), Some(t)) => (format!("{} {}", views::due_label(d, today), views::time_label(t)), due_class(d, today)),
        (Some(d), None) => (views::due_label(d, today), due_class(d, today)),
        (None, Some(t)) => (format!("Today {}", views::time_label(t)), "today"),
        (None, None) => ("Date".to_string(), ""),
    }
}

/// The Deadline chip's text.
pub fn deadline_text(deadline: Option<NaiveDate>, today: NaiveDate) -> String {
    deadline.map(|d| views::due_label(d, today)).unwrap_or_else(|| "Deadline".to_string())
}

/// The Reminders chip's text.
pub fn reminders_text(reminders: &[ReminderDraft], today: NaiveDate) -> String {
    match reminders.len() {
        0 => "Reminders".to_string(),
        1 => reminders[0].label(today),
        n => format!("{n} reminders"),
    }
}

#[component]
pub fn DatePicker(
    date: String,
    time: String,
    open: OpenPop,
    on_change: EventHandler<(String, String)>,
    /// The task's repeat rule: a ⟳ on the chip and dashed rings in the
    /// calendar (core works out the days).
    #[props(default)]
    rule: Option<keel_core::recurrence::Recurrence>,
    #[props(default)]
    face: Option<Face>,
    /// The chip's icon in place of the calendar outline.
    #[props(default)]
    icon: Option<String>,
) -> Element {
    let mut open = open;
    let today = crate::today();
    let selected = NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok();
    let current_time = NaiveTime::parse_from_str(&time, "%H:%M").ok();
    let is_open = open() == Some("date");

    let (chip_text, chip_class) = date_chip(selected, current_time, today);

    let btn_class = match &face { Some(f) => f.class.clone(), None => format!("chip chip-btn due-chip {chip_class}") };
    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: "{btn_class}",
                class: if is_open { "chip-open" },
                title: "Date",
                onclick: move |_| open.set(if is_open { None } else { Some("date") }),
                if face.as_ref().is_none_or(|f| f.icon) {
                    Icon { svg: icon.clone().unwrap_or_else(icons::calendar_small) }
                }
                match &face {
                    Some(f) => rsx! { "{f.text}" },
                    None => rsx! {
                        "{chip_text}"
                        if rule.is_some() {
                            Icon { svg: icons::repeat_small() }
                        }
                    },
                }
            }
            if is_open {
                Scheduler { selected, time: current_time, on_change, open, rule: rule.clone() }
            }
        }
    }
}

/// The date picker's shortcuts: (label, hint, date).
pub fn quick_dates(today: NaiveDate) -> Vec<(&'static str, String, NaiveDate)> {
    let saturday = today + Duration::days(((5 - today.weekday().num_days_from_monday() as i64) + 7) % 7);
    let next_monday = today + Duration::days(7 - today.weekday().num_days_from_monday() as i64);
    let tomorrow = today + Duration::days(1);
    vec![
        ("Today", DOW[today.weekday().num_days_from_monday() as usize].to_string(), today),
        ("Tomorrow", DOW[tomorrow.weekday().num_days_from_monday() as usize].to_string(), tomorrow),
        ("This weekend", DOW[5].to_string(), saturday),
        ("Next week", format!("Mon {} {}", next_monday.day(), &MONTHS[next_monday.month0() as usize][..3]), next_monday),
    ]
}

/// Drives the month / year wheels and reports the row that settles in the
/// band as `[column, row]`. Rows are 36px, matching `.wheel-item`.
///
/// The month wheel loops with only its twelve rows: `pos` is the (fractional)
/// month at the band, and each row is moved to its distance from it, wrapped
/// to -6…6, so December sits above January. Dragging, the mouse wheel and a
/// tap move `pos`; a flick coasts, then it snaps to a month. The year wheel is
/// a plain snap-scrolling list.
const WHEEL_JS: &str = r#"
(window.__keelWheel ??= {})['$ID'] = (startM, startY) => requestAnimationFrame(() => {
  const H = 36;
  const [months, years] = document.querySelectorAll('.wheel[data-wheel="$ID"] .wheel-col');
  if (months) {
    const rows = [...months.querySelectorAll('.wheel-item')], n = rows.length;
    let pos = startM, last = startM, anim = 0, snapT;
    const wrap = (i) => ((i % n) + n) % n;
    const draw = () => rows.forEach((el, i) => {
      let d = wrap(i - pos);
      if (d > n / 2) d -= n;
      el.style.transform = `translateY(${d * H}px)`;
      el.style.visibility = Math.abs(d) > 3 ? 'hidden' : '';
    });
    const settle = () => {
      pos = wrap(Math.round(pos));
      draw();
      if (pos !== last) { last = pos; dioxus.send([0, pos]); }
    };
    const glide = (to, ms) => {
      cancelAnimationFrame(anim);
      const from = pos, t0 = performance.now();
      const step = (t) => {
        const k = Math.min(1, (t - t0) / ms);
        pos = from + (to - from) * (1 - Math.pow(1 - k, 3));
        draw();
        if (k < 1) anim = requestAnimationFrame(step); else settle();
      };
      anim = requestAnimationFrame(step);
    };
    let y0 = null, yPrev = 0, tPrev = 0, v = 0, moved = 0;
    months.addEventListener('pointerdown', (e) => {
      cancelAnimationFrame(anim);
      try { months.setPointerCapture(e.pointerId); } catch (_) {}
      y0 = yPrev = e.clientY; tPrev = performance.now(); v = 0; moved = 0;
    });
    months.addEventListener('pointermove', (e) => {
      if (y0 === null) return;
      const t = performance.now(), dy = e.clientY - yPrev;
      pos -= dy / H;
      moved += Math.abs(dy);
      v = dy / Math.max(1, t - tPrev);
      yPrev = e.clientY; tPrev = t;
      draw();
    });
    const release = (e) => {
      if (y0 === null) return;
      y0 = null;
      if (moved < 4) {
        // A tap: spin the tapped month to the band.
        const it = document.elementFromPoint(e.clientX, e.clientY)?.closest('.wheel-col.loop .wheel-item');
        if (it) {
          let d = wrap(Number(it.dataset.i) - pos);
          if (d > n / 2) d -= n;
          glide(pos + d, 260);
          return;
        }
      }
      // A flick coasts about a third of a second's worth.
      glide(Math.round(pos - (v * 320) / H), 420);
    };
    months.addEventListener('pointerup', release);
    months.addEventListener('pointercancel', release);
    months.addEventListener('wheel', (e) => {
      e.preventDefault();
      cancelAnimationFrame(anim);
      pos += e.deltaY / H;
      draw();
      clearTimeout(snapT);
      snapT = setTimeout(() => glide(Math.round(pos), 160), 110);
    }, { passive: false });
    draw();
  }
  if (years) {
    years.scrollTop = startY * H;
    let last = startY, t;
    years.addEventListener('scroll', () => {
      clearTimeout(t);
      t = setTimeout(() => {
        const i = Math.round(years.scrollTop / H);
        if (i !== last) { last = i; dioxus.send([1, i]); }
      }, 110);
    });
    years.addEventListener('click', (e) => {
      const it = e.target.closest('.wheel-item');
      if (it) years.scrollTo({ top: Number(it.dataset.i) * H, behavior: 'smooth' });
    });
  }
});
await new Promise(() => {});
"#;

/// Relative reminder choices, in minutes before the due time.
pub const REMINDER_PRESETS: [i64; 5] = [0, 10, 30, 60, 1440];

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
    /// The repeat rule whose days are ringed.
    #[props(default)]
    rule: Option<keel_core::recurrence::Recurrence>,
) -> Element {
    let mut open = open;
    let today = crate::today();
    // The month shown (year, month); core builds the grid for it.
    let mut month = use_signal(move || {
        let d = selected.unwrap_or(today);
        (d.year(), d.month())
    });
    // The month / year wheels, opened from the title. The years are fixed
    // while they're open, so a wheel's rows don't shift under it.
    let mut choosing = use_signal(|| false);
    let mut wheel_years = use_signal(Vec::<i32>::new);
    // One channel per calendar for its wheels' turns, set up once
    // (WHEEL_JS); opening the wheels just starts them at the shown month.
    let wheel_id = use_hook(|| {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    });
    use_hook(move || {
        spawn(async move {
            let mut e = document::eval(&WHEEL_JS.replace("$ID", &wheel_id.to_string()));
            while let Ok((col, i)) = e.recv::<(u8, usize)>().await {
                let (y, mo) = *month.peek();
                match col {
                    0 => month.set((y, i as u32 % 12 + 1)),
                    _ => {
                        let year = wheel_years.peek().get(i).copied();
                        if let Some(year) = year {
                            month.set((year, mo));
                        }
                    }
                }
            }
        });
    });
    use_drop(move || {
        let _ = document::eval(&format!("delete window.__keelWheel?.['{wheel_id}'];"));
    });
    use_effect(move || {
        if !choosing() {
            return;
        }
        let (y, mo) = *month.peek();
        let years = calendar::year_choices(today, y);
        let at = years.iter().position(|v| *v == y).unwrap_or(0);
        wheel_years.set(years);
        let _ = document::eval(&format!("window.__keelWheel?.['{wheel_id}']?.({}, {at});", mo - 1));
    });
    let mut time_text = use_signal(move || time.map(views::time_label).unwrap_or_default());
    let mut time_bad = use_signal(|| false);
    let time_str = time.map(|t| t.format("%H:%M").to_string()).unwrap_or_default();
    let touch = use_context::<Touch>().0;

    let quick: Vec<(&str, String, NaiveDate, String, &str)> = quick_dates(today)
        .into_iter()
        .zip([
            (icons::calendar_small(), "q-today"),
            (icons::sun(), "q-tomorrow"),
            (icons::sofa(), "q-weekend"),
            (icons::arrow_right(), "q-next"),
        ])
        .map(|((label, hint, d), (icon, cls))| (label, hint, d, icon, cls))
        .collect();

    let (y, mo) = month();
    let view = calendar::month_view(y, mo, week_start(), today, selected, rule.as_ref());
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
                    button {
                        r#type: "button",
                        class: if choosing() { "cal-title chooser open" } else { "cal-title chooser" },
                        title: "Choose month and year",
                        onclick: move |_| choosing.set(!choosing()),
                        "{view.title}"
                        Icon { svg: icons::chevron_down_small() }
                    }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Previous month",
                        onclick: move |_| month.set(calendar::shift_month(y, mo, -1)),
                        Icon { svg: icons::chevron_left() }
                    }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Today",
                        onclick: move |_| month.set((today.year(), today.month())),
                        span { class: "cal-dot" }
                    }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Next month",
                        onclick: move |_| month.set(calendar::shift_month(y, mo, 1)),
                        Icon { svg: icons::chevron_right() }
                    }
                }
                if choosing() {
                    // Month and year wheels, like iOS Calendar's; they
                    // stand in for the day grid while open.
                    div { class: "wheel", "data-wheel": "{wheel_id}",
                        div { class: "wheel-band" }
                        // The twelve months, looping: WHEEL_JS places each
                        // row around the band (no copies).
                        div { class: "wheel-col loop", "data-col": "0",
                            for (i, name) in calendar::MONTHS.iter().enumerate() {
                                div {
                                    key: "{i}",
                                    class: if i as u32 + 1 == mo { "wheel-item on" } else { "wheel-item" },
                                    "data-i": "{i}",
                                    "{name}"
                                }
                            }
                        }
                        div { class: "wheel-col", "data-col": "1",
                            for (i, year) in wheel_years().iter().enumerate() {
                                div { key: "{year}", class: if *year == y { "wheel-item on" } else { "wheel-item" }, "data-i": "{i}", "{year}" }
                            }
                        }
                    }
                } else {
                div { class: "cal-grid",
                    for (i, d) in view.weekdays.iter().enumerate() {
                        span { key: "h{i}", class: "cal-dow", "{d}" }
                    }
                    for (i, cell) in view.weeks.iter().flatten().cloned().enumerate() {
                        {
                            let ts = time_str.clone();
                            let day = cell.date;
                            let mut cls = String::from("cal-day");
                            if !cell.in_month { cls.push_str(" other"); }
                            if cell.past { cls.push_str(" past"); }
                            if cell.today { cls.push_str(" is-today"); }
                            if cell.selected { cls.push_str(&format!(" selected sel-{}", due_class(day, today))); }
                            if cell.repeat { cls.push_str(" repeat-day"); }
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
            }
            if with_time {
                div { class: "sched-time",
                    Icon { svg: icons::clock_small() }
                    if touch() {
                        // Phones: the OS time wheel instead of typing. iOS
                        // shows no placeholder, so an empty field gets a hint.
                        span { class: "time-native",
                        if time_str.is_empty() {
                            span { class: "time-hint", "Add time" }
                        }
                        input {
                            r#type: "time",
                            class: "time-input native",
                            value: "{time_str}",
                            onchange: move |e| match native_time(&e.value()) {
                                Some(Some(t)) => {
                                    time_text.set(views::time_label(t));
                                    // A time without a date means today.
                                    pick(Some(selected.unwrap_or(today)), t.format("%H:%M").to_string());
                                }
                                Some(None) => {
                                    time_text.set(String::new());
                                    pick(selected, String::new());
                                }
                                None => {}
                            },
                        }
                        }
                    } else {
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
pub fn DeadlinePicker(
    deadline: String,
    open: OpenPop,
    on_change: EventHandler<String>,
    #[props(default)] face: Option<Face>,
) -> Element {
    let mut open = open;
    let today = crate::today();
    let selected = NaiveDate::parse_from_str(&deadline, "%Y-%m-%d").ok();
    let is_open = open() == Some("deadline");
    let text = deadline_text(selected, today);
    let class = match selected {
        Some(d) if d <= today => "deadline-chip urgent",
        Some(_) => "deadline-chip set",
        None => "deadline-chip",
    };
    let btn_class = match &face { Some(f) => f.class.clone(), None => format!("chip chip-btn {class}") };
    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: "{btn_class}",
                class: if is_open { "chip-open" },
                title: "Deadline: the date it must be done by",
                onclick: move |_| open.set(if is_open { None } else { Some("deadline") }),
                if face.as_ref().is_none_or(|f| f.icon) {
                    Icon { svg: icons::target_small() }
                }
                match &face {
                    Some(f) => rsx! { "{f.text}" },
                    None => rsx! { "{text}" },
                }
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
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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
    #[props(default)]
    face: Option<Face>,
) -> Element {
    let mut open = open;
    let today = crate::today();
    let is_open = open() == Some("reminders");
    let mut custom = use_signal(String::new);
    let mut custom_bad = use_signal(|| false);
    let text = reminders_text(&reminders, today);
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
    let presets = REMINDER_PRESETS;

    let btn_class = match &face {
        Some(f) => f.class.clone(),
        None if reminders.is_empty() => "chip chip-btn".to_string(),
        None => "chip chip-btn reminder-chip".to_string(),
    };
    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: "{btn_class}",
                class: if is_open { "chip-open" },
                title: "Reminders",
                onclick: move |_| open.set(if is_open { None } else { Some("reminders") }),
                if face.as_ref().is_none_or(|f| f.icon) {
                    Icon { svg: icons::bell_small() }
                }
                match &face {
                    Some(f) => rsx! { "{f.text}" },
                    None => rsx! { "{text}" },
                }
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
    #[props(default)]
    face: Option<Face>,
) -> Element {
    let mut open = open;
    let is_open = open() == Some(id);
    let btn_class = match &face { Some(f) => f.class.clone(), None => "chip chip-btn".to_string() };
    rsx! {
        div { class: "chip-wrap",
            button {
                r#type: "button",
                class: "{btn_class}",
                class: if is_open { "chip-open" },
                title,
                onclick: move |_| open.set(if is_open { None } else { Some(id) }),
                if face.as_ref().is_none_or(|f| f.icon) {
                    span { class: "chip-icon {icon_class}", Icon { svg: icon } }
                }
                match &face {
                    Some(f) => rsx! { "{f.text}" },
                    None => rsx! {
                        "{text}"
                        span { class: "chip-caret", Icon { svg: icons::chevron_down_small() } }
                    },
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_time_reads_the_time_inputs_value() {
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).unwrap();
        assert_eq!(native_time("17:30"), Some(Some(t(17, 30))));
        assert_eq!(native_time("07:05:00"), Some(Some(t(7, 5))), "with seconds");
        assert_eq!(native_time(""), Some(None), "cleared");
        assert_eq!(native_time("5pm"), None, "not the input's format");
    }
}
