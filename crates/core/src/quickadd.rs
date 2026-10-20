//! Natural-language quick add: pull dates, times, repeats, priority, project,
//! section and labels out of a typed task title (English, Todoist-style).
//!
//! Matching is on whole whitespace-separated words (trailing `,.;!?` ignored),
//! so "today's" or "Mondays" stay text. The first date, time and repeat win.

use chrono::{Datelike, Duration, Months, NaiveDate, NaiveTime};

use crate::ids::Id;
use crate::model::{Priority, Project, Section};
use crate::recurrence::Recurrence;

/// What was understood from a quick-add title.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct QuickAdd {
    /// The title with recognized phrases removed.
    pub content: String,
    pub date: Option<NaiveDate>,
    pub time: Option<NaiveTime>,
    pub recurrence: Option<Recurrence>,
    pub priority: Option<Priority>,
    pub project_id: Option<Id>,
    pub section_id: Option<Id>,
    pub labels: Vec<String>,
    /// The recognized phrases, as byte ranges into the input (sorted), for
    /// highlighting. Empty when nothing was applied.
    pub spans: Vec<Span>,
}

/// A recognized phrase in the input (`input[start..end]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: SpanKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanKind {
    Date,
    Time,
    Repeat,
    Priority,
    Project,
    Section,
    Label,
}

impl QuickAdd {
    /// `YYYY-MM-DD`, or `YYYY-MM-DDTHH:MM:00` with a time (the `Due.date` format).
    pub fn due_string(&self) -> Option<String> {
        self.date.map(|d| match self.time {
            Some(t) => format!("{}T{}", d.format("%Y-%m-%d"), t.format("%H:%M:00")),
            None => d.format("%Y-%m-%d").to_string(),
        })
    }

    /// True when nothing besides the title was recognized.
    pub fn is_plain(&self) -> bool {
        self.date.is_none()
            && self.time.is_none()
            && self.recurrence.is_none()
            && self.priority.is_none()
            && self.project_id.is_none()
            && self.section_id.is_none()
            && self.labels.is_empty()
    }
}

struct Token {
    raw: String,
    /// Lower-cased with trailing punctuation removed.
    norm: String,
    /// Byte range of `raw` in the input.
    start: usize,
    end: usize,
}

fn tokens(input: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut push = |s: usize, e: usize| {
        let raw = &input[s..e];
        out.push(Token {
            raw: raw.to_string(),
            norm: raw.trim_end_matches([',', '.', ';', '!', '?']).to_lowercase(),
            start: s,
            end: e,
        });
    };
    for (i, c) in input.char_indices() {
        match (c.is_whitespace(), start) {
            (true, Some(s)) => {
                push(s, i);
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        push(s, input.len());
    }
    out
}

/// Mon = 0 … Sun = 6.
fn weekday(word: &str) -> Option<u32> {
    Some(match word {
        "mon" | "monday" => 0,
        "tue" | "tues" | "tuesday" => 1,
        "wed" | "weds" | "wednesday" => 2,
        "thu" | "thur" | "thurs" | "thursday" => 3,
        "fri" | "friday" => 4,
        "sat" | "saturday" => 5,
        "sun" | "sunday" => 6,
        _ => return None,
    })
}

fn month(word: &str) -> Option<u32> {
    Some(match word {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    })
}

/// `15`, `15th`, `1st`, `2nd`, `3rd` → 1..=31.
fn day_number(word: &str) -> Option<u32> {
    let digits = word
        .strip_suffix("st")
        .or_else(|| word.strip_suffix("nd"))
        .or_else(|| word.strip_suffix("rd"))
        .or_else(|| word.strip_suffix("th"))
        .unwrap_or(word);
    let n: u32 = digits.parse().ok()?;
    (1..=31).contains(&n).then_some(n)
}

fn idx(d: NaiveDate) -> u32 {
    d.weekday().num_days_from_monday()
}

/// That weekday this week if today or later, else next week's.
fn today_or_next(today: NaiveDate, wd: u32) -> NaiveDate {
    today + Duration::days(((wd + 7 - idx(today)) % 7) as i64)
}

fn saturday_this_weekend(today: NaiveDate) -> NaiveDate {
    if idx(today) >= 5 { today } else { today_or_next(today, 5) }
}

/// A date phrase starting at `i`: (words consumed, date).
fn match_date(t: &[Token], i: usize, today: NaiveDate) -> Option<(usize, NaiveDate)> {
    let w = |k: usize| t.get(i + k).map(|x| x.norm.as_str()).unwrap_or("");
    if w(0) == "on" {
        return match_date(t, i + 1, today).map(|(n, d)| (n + 1, d));
    }
    match w(0) {
        "today" | "tod" => return Some((1, today)),
        "tomorrow" | "tmr" | "tom" | "tmrw" => return Some((1, today + Duration::days(1))),
        "weekend" => return Some((1, saturday_this_weekend(today))),
        _ => {}
    }
    if w(0) == "this" && w(1) == "weekend" {
        return Some((2, saturday_this_weekend(today)));
    }
    if let Some(wd) = weekday(w(0)) {
        return Some((1, today_or_next(today, wd)));
    }
    if w(0) == "next" {
        if let Some(wd) = weekday(w(1)) {
            let d = today_or_next(today, wd);
            let same_week = d.iso_week() == today.iso_week();
            return Some((2, if same_week { d + Duration::days(7) } else { d }));
        }
        match w(1) {
            "week" => return Some((2, today + Duration::days((7 - idx(today)) as i64))),
            "month" => {
                let first = today.with_day(1)?.checked_add_months(Months::new(1))?;
                return Some((2, first));
            }
            "weekend" => return Some((2, saturday_this_weekend(today) + Duration::days(7))),
            _ => {}
        }
    }
    if w(0) == "in" {
        if let Ok(n) = w(1).parse::<u32>() {
            let d = match w(2) {
                "day" | "days" => today + Duration::days(n as i64),
                "week" | "weeks" => today + Duration::days(7 * n as i64),
                "month" | "months" => today.checked_add_months(Months::new(n))?,
                _ => return None,
            };
            return Some((3, d));
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(w(0), "%Y-%m-%d") {
        return Some((1, d));
    }
    // "oct 20 [2027]" or "20 oct [2027]".
    let (m, day) = match (month(w(0)), day_number(w(1)), day_number(w(0)), month(w(1))) {
        (Some(m), Some(day), _, _) => (m, day),
        (_, _, Some(day), Some(m)) => (m, day),
        _ => return None,
    };
    let year = w(2).parse::<i32>().ok().filter(|y| (1000..=9999).contains(y));
    let date = match year {
        Some(y) => NaiveDate::from_ymd_opt(y, m, day)?,
        None => {
            let this_year = NaiveDate::from_ymd_opt(today.year(), m, day)?;
            if this_year < today {
                NaiveDate::from_ymd_opt(today.year() + 1, m, day)?
            } else {
                this_year
            }
        }
    };
    Some((if year.is_some() { 3 } else { 2 }, date))
}

/// `5pm`, `5:30pm`, `12am` → time.
fn clock_ampm(word: &str) -> Option<NaiveTime> {
    let (num, pm) = if let Some(n) = word.strip_suffix("pm") {
        (n, true)
    } else if let Some(n) = word.strip_suffix("am") {
        (n, false)
    } else {
        return None;
    };
    hm_ampm(num, pm)
}

fn hm_ampm(num: &str, pm: bool) -> Option<NaiveTime> {
    let (h, m) = match num.split_once(':') {
        Some((h, m)) => (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?),
        None => (num.parse::<u32>().ok()?, 0),
    };
    if !(1..=12).contains(&h) || m > 59 {
        return None;
    }
    let h24 = match (h, pm) {
        (12, false) => 0,
        (12, true) => 12,
        (h, true) => h + 12,
        (h, false) => h,
    };
    NaiveTime::from_hms_opt(h24, m, 0)
}

/// A time phrase starting at `i`: (words consumed, time).
fn match_time(t: &[Token], i: usize) -> Option<(usize, NaiveTime)> {
    let w = |k: usize| t.get(i + k).map(|x| x.norm.as_str()).unwrap_or("");
    if w(0) == "at" {
        return match_time(t, i + 1).map(|(n, time)| (n + 1, time));
    }
    match w(0) {
        "noon" => return Some((1, NaiveTime::from_hms_opt(12, 0, 0)?)),
        "midnight" => return Some((1, NaiveTime::from_hms_opt(0, 0, 0)?)),
        _ => {}
    }
    if let Some(time) = clock_ampm(w(0)) {
        return Some((1, time));
    }
    if matches!(w(1), "am" | "pm") {
        if let Some(time) = hm_ampm(w(0), w(1) == "pm") {
            return Some((2, time));
        }
    }
    // 24-hour "17:00".
    if let Some((h, m)) = w(0).split_once(':') {
        if let (Ok(h), Ok(m)) = (h.parse::<u32>(), m.parse::<u32>()) {
            if h < 24 && m < 60 && w(0).len() >= 4 {
                return Some((1, NaiveTime::from_hms_opt(h, m, 0)?));
            }
        }
    }
    None
}

/// A time typed on its own ("5pm", "5:30 pm", "17:00", "noon"), for time
/// fields; `None` unless the whole text is one time phrase.
pub fn parse_time(text: &str) -> Option<NaiveTime> {
    let t = tokens(text);
    match match_time(&t, 0) {
        Some((n, time)) if n == t.len() => Some(time),
        _ => None,
    }
}

/// A repeat that may need the task's date to anchor on.
enum RepeatSpec {
    Rule(Recurrence),
    /// "every week" — weekday of the date.
    Weekly,
    /// "every month" — day of the date.
    Monthly,
}

fn match_repeat(t: &[Token], i: usize) -> Option<(usize, RepeatSpec)> {
    let w = |k: usize| t.get(i + k).map(|x| x.norm.as_str()).unwrap_or("");
    match w(0) {
        "daily" | "everyday" => return Some((1, RepeatSpec::Rule(Recurrence::Daily))),
        "weekly" => return Some((1, RepeatSpec::Weekly)),
        "monthly" => return Some((1, RepeatSpec::Monthly)),
        "every" => {}
        _ => return None,
    }
    match w(1) {
        "day" => return Some((2, RepeatSpec::Rule(Recurrence::Daily))),
        "weekday" | "weekdays" => {
            return Some((2, RepeatSpec::Rule(Recurrence::Weekly(vec![0, 1, 2, 3, 4]))))
        }
        "week" => return Some((2, RepeatSpec::Weekly)),
        "month" => {
            // "every month on [the] 15th"
            let (skip, n) = match (w(2), w(3), w(4)) {
                ("on", "the", day) => (5, day_number(day)),
                ("on", day, _) => (4, day_number(day)),
                _ => (2, None),
            };
            return Some(match n {
                Some(day) => (skip, RepeatSpec::Rule(Recurrence::MonthlyDay(day as u8))),
                None => (2, RepeatSpec::Monthly),
            });
        }
        _ => {}
    }
    // "every 15th" (an ordinal, not a bare count like "every 2 weeks").
    if w(1).chars().any(|c| c.is_ascii_alphabetic()) {
        if let Some(day) = day_number(w(1)) {
            return Some((2, RepeatSpec::Rule(Recurrence::MonthlyDay(day as u8))));
        }
    }
    // "every mon, wed and fri"
    let mut days = Vec::new();
    let mut j = 1;
    loop {
        if let Some(wd) = weekday(w(j)) {
            days.push(wd as u8);
            j += 1;
        } else if w(j) == "and" && weekday(w(j + 1)).is_some() {
            j += 1;
        } else {
            break;
        }
    }
    if days.is_empty() {
        return None;
    }
    days.sort_unstable();
    days.dedup();
    Some((j, RepeatSpec::Rule(Recurrence::Weekly(days))))
}

/// Longest run of words starting at `i` (first word prefixed by `sigil`)
/// whose text equals one of `names` (case-insensitive). Returns (len, index).
fn match_name(t: &[Token], i: usize, sigil: &str, names: &[&str]) -> Option<(usize, usize)> {
    let first = t[i].raw.strip_prefix(sigil)?;
    let first = first.strip_prefix(sigil).unwrap_or(first); // "##Work"
    for len in (1..=(t.len() - i).min(6)).rev() {
        let mut text = first.to_string();
        for tok in &t[i + 1..i + len] {
            text.push(' ');
            text.push_str(&tok.raw);
        }
        let text = text.trim_end_matches([',', '.', ';', '!', '?']).to_lowercase();
        if text.is_empty() {
            continue;
        }
        if let Some(pos) = names.iter().position(|n| n.to_lowercase() == text) {
            return Some((len, pos));
        }
    }
    None
}

/// Parses a quick-add title. Unknown projects/sections stay in the title; if
/// stripping would leave no title, nothing is applied.
pub fn parse(input: &str, today: NaiveDate, projects: &[Project], sections: &[Section]) -> QuickAdd {
    let t = tokens(input);
    let mut used = vec![false; t.len()];
    let mut q = QuickAdd::default();
    let mut repeat: Option<RepeatSpec> = None;
    let project_names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();

    let mut spans: Vec<Span> = Vec::new();
    let take = |used: &mut [bool], spans: &mut Vec<Span>, i: usize, n: usize, kind: SpanKind| {
        used[i..i + n].iter_mut().for_each(|u| *u = true);
        spans.push(Span { start: t[i].start, end: t[i + n - 1].end, kind });
    };

    let mut i = 0;
    while i < t.len() {
        if used[i] {
            i += 1;
            continue;
        }
        let norm = t[i].norm.as_str();
        if q.priority.is_none() {
            let p = match norm {
                "p1" => Some(Priority::P1),
                "p2" => Some(Priority::P2),
                "p3" => Some(Priority::P3),
                "p4" => Some(Priority::None),
                _ => None,
            };
            if p.is_some() {
                q.priority = p;
                take(&mut used, &mut spans, i, 1, SpanKind::Priority);
                i += 1;
                continue;
            }
        }
        if q.project_id.is_none() && t[i].raw.starts_with('#') {
            if let Some((n, pos)) = match_name(&t, i, "#", &project_names) {
                q.project_id = Some(projects[pos].id.clone());
                take(&mut used, &mut spans, i, n, SpanKind::Project);
                i += n;
                continue;
            }
        }
        if t[i].raw.len() > 1 && t[i].raw.starts_with('@') {
            let name = t[i].raw[1..].trim_end_matches([',', '.', ';', '!', '?']).to_string();
            if !name.is_empty() {
                if !q.labels.iter().any(|l| l.eq_ignore_ascii_case(&name)) {
                    q.labels.push(name);
                }
                take(&mut used, &mut spans, i, 1, SpanKind::Label);
                i += 1;
                continue;
            }
        }
        if repeat.is_none() {
            if let Some((n, spec)) = match_repeat(&t, i) {
                repeat = Some(spec);
                take(&mut used, &mut spans, i, n, SpanKind::Repeat);
                i += n;
                continue;
            }
        }
        if q.date.is_none() {
            if let Some((n, date)) = match_date(&t, i, today) {
                q.date = Some(date);
                take(&mut used, &mut spans, i, n, SpanKind::Date);
                i += n;
                continue;
            }
        }
        if q.time.is_none() {
            if let Some((n, time)) = match_time(&t, i) {
                q.time = Some(time);
                take(&mut used, &mut spans, i, n, SpanKind::Time);
                i += n;
                continue;
            }
        }
        i += 1;
    }

    // Sections resolve within the chosen project (or the Inbox).
    let inbox = projects.iter().find(|p| p.is_inbox).map(|p| p.id.clone());
    if let Some(scope) = q.project_id.clone().or(inbox.clone()) {
        let scoped: Vec<&Section> = sections.iter().filter(|s| s.project_id == scope).collect();
        let names: Vec<&str> = scoped.iter().map(|s| s.name.as_str()).collect();
        for i in 0..t.len() {
            if used[i] || q.section_id.is_some() || !t[i].raw.starts_with('/') {
                continue;
            }
            if let Some((n, pos)) = match_name(&t, i, "/", &names) {
                q.section_id = Some(scoped[pos].id.clone());
                q.project_id.get_or_insert(scope.clone());
                take(&mut used, &mut spans, i, n, SpanKind::Section);
            }
        }
    }

    // Repeats anchor on the parsed date (or today); the due becomes the first occurrence.
    if let Some(spec) = repeat {
        let start = q.date.unwrap_or(today);
        let rule = match spec {
            RepeatSpec::Rule(r) => r,
            RepeatSpec::Weekly => Recurrence::Weekly(vec![idx(start) as u8]),
            RepeatSpec::Monthly => Recurrence::MonthlyDay(start.day() as u8),
        };
        q.date = Some(if rule.contains(start) { start } else { rule.next_after(start) });
        q.recurrence = Some(rule);
    } else if q.time.is_some() && q.date.is_none() {
        q.date = Some(today);
    }

    q.content = t
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|(tok, _)| tok.raw.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if q.content.trim().is_empty() {
        return QuickAdd { content: input.trim().to_string(), ..QuickAdd::default() };
    }
    spans.sort_by_key(|s| s.start);
    q.spans = spans;
    q
}

#[cfg(test)]
mod tests {
    use super::{parse, QuickAdd};
    use crate::ids::Id;
    use crate::model::{Priority, Project, Section};
    use crate::recurrence::Recurrence;
    use chrono::{NaiveDate, NaiveTime};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn today() -> NaiveDate {
        d(2026, 10, 19) // Monday
    }

    fn projects() -> Vec<Project> {
        let mut inbox = Project::inbox();
        inbox.id = Id::from("P-IN");
        let mut work = Project::new("Work");
        work.id = Id::from("P-WORK");
        let mut side = Project::new("Side Project");
        side.id = Id::from("P-SIDE");
        vec![inbox, work, side]
    }

    fn sections() -> Vec<Section> {
        let mut next = Section::new(Id::from("P-WORK"), "Next", 0);
        next.id = Id::from("S-NEXT");
        let mut later = Section::new(Id::from("P-IN"), "Later", 0);
        later.id = Id::from("S-LATER");
        vec![next, later]
    }

    fn p(input: &str) -> QuickAdd {
        parse(input, today(), &projects(), &sections())
    }

    fn date_of(input: &str) -> Option<NaiveDate> {
        p(input).date
    }

    #[test]
    fn relative_dates() {
        let q = p("Pay rent today");
        assert_eq!(q.content, "Pay rent");
        assert_eq!(q.date, Some(today()));
        for word in ["tomorrow", "tmr", "tom", "tmrw", "Tomorrow"] {
            assert_eq!(date_of(&format!("x {word}")), Some(d(2026, 10, 20)), "{word}");
        }
    }

    #[test]
    fn weekdays_today_or_next() {
        assert_eq!(date_of("x monday"), Some(today()), "same weekday = today");
        assert_eq!(date_of("x fri"), Some(d(2026, 10, 23)));
        let q = p("x on wednesday");
        assert_eq!(q.date, Some(d(2026, 10, 21)));
        assert_eq!(q.content, "x", "leading 'on' consumed with the date");
    }

    #[test]
    fn next_forms() {
        assert_eq!(date_of("x next mon"), Some(d(2026, 10, 26)));
        assert_eq!(date_of("x next friday"), Some(d(2026, 10, 30)), "Friday of next week");
        assert_eq!(date_of("x next week"), Some(d(2026, 10, 26)));
        assert_eq!(date_of("x next month"), Some(d(2026, 11, 1)));
        assert_eq!(date_of("x weekend"), Some(d(2026, 10, 24)));
        assert_eq!(date_of("x this weekend"), Some(d(2026, 10, 24)));
        assert_eq!(date_of("x next weekend"), Some(d(2026, 10, 31)));
    }

    #[test]
    fn in_n_units() {
        assert_eq!(date_of("x in 3 days"), Some(d(2026, 10, 22)));
        assert_eq!(date_of("x in 1 day"), Some(d(2026, 10, 20)));
        assert_eq!(date_of("x in 2 weeks"), Some(d(2026, 11, 2)));
        assert_eq!(date_of("x in 1 month"), Some(d(2026, 11, 19)));
    }

    #[test]
    fn month_day_forms() {
        assert_eq!(date_of("x oct 25"), Some(d(2026, 10, 25)));
        assert_eq!(date_of("x October 25th"), Some(d(2026, 10, 25)));
        assert_eq!(date_of("x 25 oct"), Some(d(2026, 10, 25)));
        assert_eq!(date_of("x oct 1"), Some(d(2027, 10, 1)), "past date rolls to next year");
        assert_eq!(date_of("x oct 1 2028"), Some(d(2028, 10, 1)));
        assert_eq!(date_of("x 2026-12-31"), Some(d(2026, 12, 31)));
        assert_eq!(p("x oct 25").content, "x");
    }

    #[test]
    fn times() {
        let q = p("x at 5pm");
        assert_eq!((q.date, q.time), (Some(today()), Some(t(17, 0))), "time alone means today");
        assert_eq!(q.content, "x");
        assert_eq!(p("x 5:30pm").time, Some(t(17, 30)));
        assert_eq!(p("x 17:00").time, Some(t(17, 0)));
        assert_eq!(p("x 5 pm").time, Some(t(17, 0)));
        assert_eq!(p("x noon").time, Some(t(12, 0)));
        assert_eq!(p("x midnight").time, Some(t(0, 0)));
        assert_eq!(p("x 12am").time, Some(t(0, 0)));
        assert_eq!(p("x 12pm").time, Some(t(12, 0)));
        let q = p("x tomorrow at 9am");
        assert_eq!(q.due_string().as_deref(), Some("2026-10-20T09:00:00"));
    }

    #[test]
    fn repeats() {
        let q = p("x every day");
        assert_eq!((q.recurrence, q.date), (Some(Recurrence::Daily), Some(today())));
        assert_eq!(q.content, "x");
        assert_eq!(p("x daily").recurrence, Some(Recurrence::Daily));
        assert_eq!(p("x everyday").recurrence, Some(Recurrence::Daily));
        assert_eq!(p("x every weekday").recurrence, Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])));
        assert_eq!(p("x every week").recurrence, Some(Recurrence::Weekly(vec![0])), "anchored on today (Mon)");
        assert_eq!(p("x weekly").recurrence, Some(Recurrence::Weekly(vec![0])));
        assert_eq!(p("x every month").recurrence, Some(Recurrence::MonthlyDay(19)));
        assert_eq!(p("x monthly").recurrence, Some(Recurrence::MonthlyDay(19)));
        let q = p("x every wed");
        assert_eq!((q.recurrence, q.date), (Some(Recurrence::Weekly(vec![2])), Some(d(2026, 10, 21))));
        let q = p("x every mon, wed and fri");
        assert_eq!(q.recurrence, Some(Recurrence::Weekly(vec![0, 2, 4])));
        assert_eq!(q.date, Some(today()));
        assert_eq!(q.content, "x");
        let q = p("x every 15th");
        assert_eq!((q.recurrence, q.date), (Some(Recurrence::MonthlyDay(15)), Some(d(2026, 11, 15))));
        let q = p("x every month on the 1st");
        assert_eq!((q.recurrence, q.date), (Some(Recurrence::MonthlyDay(1)), Some(d(2026, 11, 1))));
    }

    #[test]
    fn repeat_anchors_on_parsed_date_and_takes_time() {
        let q = p("x friday every week");
        assert_eq!((q.recurrence, q.date), (Some(Recurrence::Weekly(vec![4])), Some(d(2026, 10, 23))));
        let q = p("standup every weekday at 9am");
        assert_eq!(q.content, "standup");
        assert_eq!(q.recurrence, Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])));
        assert_eq!(q.due_string().as_deref(), Some("2026-10-19T09:00:00"));
    }

    #[test]
    fn unsupported_repeats_stay_in_title() {
        let q = p("x every 2 weeks");
        assert_eq!(q.recurrence, None);
        assert_eq!(q.content, "x every 2 weeks");
        assert_eq!(p("x every year").content, "x every year");
    }

    #[test]
    fn priority() {
        let q = p("x p1");
        assert_eq!((q.priority, q.content.as_str()), (Some(Priority::P1), "x"));
        assert_eq!(p("x P2").priority, Some(Priority::P2));
        assert_eq!(p("x p4").priority, Some(Priority::None));
    }

    #[test]
    fn projects_greedy_and_unknown() {
        assert_eq!(p("x #work").project_id, Some(Id::from("P-WORK")));
        let q = p("x #Side Project");
        assert_eq!((q.project_id, q.content.as_str()), (Some(Id::from("P-SIDE")), "x"));
        let q = p("x #unknown");
        assert_eq!((q.project_id, q.content.as_str()), (None, "x #unknown"));
    }

    #[test]
    fn sections_within_project_or_inbox() {
        let q = p("x #work /next");
        assert_eq!((q.project_id, q.section_id), (Some(Id::from("P-WORK")), Some(Id::from("S-NEXT"))));
        let q = p("x /later");
        assert_eq!((q.project_id, q.section_id), (Some(Id::from("P-IN")), Some(Id::from("S-LATER"))));
        let q = p("x /nowhere");
        assert_eq!((q.section_id, q.content.as_str()), (None, "x /nowhere"));
        // /next exists only in Work, not the Inbox.
        assert_eq!(p("x /next").content, "x /next");
    }

    #[test]
    fn labels() {
        let q = p("x @home @Errands");
        assert_eq!(q.labels, vec!["home".to_string(), "Errands".to_string()]);
        assert_eq!(q.content, "x");
    }

    #[test]
    fn first_date_wins() {
        let q = p("x today tomorrow");
        assert_eq!(q.date, Some(today()));
        assert_eq!(q.content, "x tomorrow");
    }

    #[test]
    fn empty_title_falls_back_to_plain_text() {
        let q = p("tomorrow");
        assert_eq!(q.content, "tomorrow");
        assert_eq!(q.date, None, "nothing is applied when it would leave no title");
    }

    #[test]
    fn whole_words_only() {
        for input in ["Plan today's agenda", "Mondays are hard", "Buy todays paper", "#hashtag culture", "email@example.com"] {
            let q = p(input);
            assert_eq!(q.date, None, "{input}");
            assert_eq!(q.content, input, "{input}");
        }
        let q = p("Email Bob tomorrow.");
        assert_eq!((q.date, q.content.as_str()), (Some(d(2026, 10, 20)), "Email Bob"));
    }

    #[test]
    fn spans_mark_recognized_text() {
        use super::SpanKind::*;
        let input = "Call  mom tomorrow at 5pm p1 #Side Project @phone";
        let q = p(input);
        let got: Vec<(&str, super::SpanKind)> =
            q.spans.iter().map(|s| (&input[s.start..s.end], s.kind)).collect();
        assert_eq!(
            got,
            vec![
                ("tomorrow", Date),
                ("at 5pm", Time),
                ("p1", Priority),
                ("#Side Project", Project),
                ("@phone", Label),
            ]
        );
    }

    #[test]
    fn spans_are_byte_offsets_with_multibyte_text() {
        let input = "Café ☕ every day";
        let q = p(input);
        assert_eq!(q.spans.len(), 1);
        assert_eq!(&input[q.spans[0].start..q.spans[0].end], "every day");
        assert_eq!(q.spans[0].kind, super::SpanKind::Repeat);
    }

    #[test]
    fn section_span_and_no_spans_when_nothing_applied() {
        let input = "x /Later";
        let q = p(input);
        assert_eq!(&input[q.spans[0].start..q.spans[0].end], "/Later");
        assert_eq!(q.spans[0].kind, super::SpanKind::Section);
        assert!(p("tomorrow").spans.is_empty(), "empty-title fallback applies nothing");
        assert!(p("plain words").spans.is_empty());
    }

    #[test]
    fn parse_time_standalone() {
        use super::parse_time;
        assert_eq!(parse_time("5pm"), Some(t(17, 0)));
        assert_eq!(parse_time(" 5:30 PM "), Some(t(17, 30)));
        assert_eq!(parse_time("17:45"), Some(t(17, 45)));
        assert_eq!(parse_time("noon"), Some(t(12, 0)));
        assert_eq!(parse_time("12am"), Some(t(0, 0)));
        assert_eq!(parse_time("5"), None, "bare hour is ambiguous");
        assert_eq!(parse_time("5pm tomorrow"), None, "only a time");
        assert_eq!(parse_time(""), None);
    }

    #[test]
    fn everything_together() {
        let q = p("Call mom tomorrow at 5pm p1 #work /next @phone");
        assert_eq!(q.content, "Call mom");
        assert_eq!(q.due_string().as_deref(), Some("2026-10-20T17:00:00"));
        assert_eq!(q.priority, Some(Priority::P1));
        assert_eq!(q.project_id, Some(Id::from("P-WORK")));
        assert_eq!(q.section_id, Some(Id::from("S-NEXT")));
        assert_eq!(q.labels, vec!["phone".to_string()]);
    }
}
