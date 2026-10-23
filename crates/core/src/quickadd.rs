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
    /// From `{oct 30}`: the "must be done by" date.
    pub deadline: Option<NaiveDate>,
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
    Deadline,
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
            && self.deadline.is_none()
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

/// `input` without the recognized phrases of the given kinds, with the
/// leftover whitespace collapsed. Used when a value is picked by hand: the
/// pick replaces whatever was typed for it.
pub fn strip_spans(input: &str, spans: &[Span], kinds: &[SpanKind]) -> String {
    let mut out = String::with_capacity(input.len());
    let mut at = 0;
    for s in spans.iter().filter(|s| kinds.contains(&s.kind)) {
        out.push_str(&input[at..s.start]);
        at = s.end;
    }
    out.push_str(&input[at..]);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

const ALL_KINDS: [SpanKind; 8] = [
    SpanKind::Date,
    SpanKind::Time,
    SpanKind::Repeat,
    SpanKind::Priority,
    SpanKind::Project,
    SpanKind::Section,
    SpanKind::Label,
    SpanKind::Deadline,
];

/// [`parse`], leaving the `plain` byte ranges alone: phrases the user
/// turned back into text (backspace right after a highlight, as in
/// Todoist). They're masked with filler of the same byte length, so every
/// other offset is unchanged, and the title is rebuilt from `input`.
pub fn parse_keeping(
    input: &str,
    today: NaiveDate,
    projects: &[Project],
    sections: &[Section],
    plain: &[(usize, usize)],
) -> QuickAdd {
    if plain.is_empty() {
        return parse(input, today, projects, sections);
    }
    let masked: String = input
        .char_indices()
        .map(|(i, c)| {
            let kept = plain.iter().any(|&(a, b)| i >= a && i < b);
            if kept && !c.is_whitespace() { "\u{1}".repeat(c.len_utf8()) } else { c.to_string() }
        })
        .collect();
    let mut q = parse(&masked, today, projects, sections);
    q.content = if q.spans.is_empty() { input.trim().to_string() } else { strip_spans(input, &q.spans, &ALL_KINDS) };
    q
}

/// Moves `plain` ranges through an edit from `old` to `new`: ranges before
/// or after the changed stretch stay with their text; a range the edit
/// touches is dropped, so the phrase is read again.
pub fn shift_plain(old: &str, new: &str, plain: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut prefix = old.bytes().zip(new.bytes()).take_while(|(a, b)| a == b).count();
    while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let max_suffix = old.len().min(new.len()) - prefix;
    let mut suffix = old.bytes().rev().zip(new.bytes().rev()).take(max_suffix).take_while(|(a, b)| a == b).count();
    while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix) {
        suffix -= 1;
    }
    let (old_end, delta) = (old.len() - suffix, new.len() as isize - old.len() as isize);
    plain
        .iter()
        .filter_map(|&(a, b)| {
            if b <= prefix {
                Some((a, b))
            } else if a >= old_end {
                Some(((a as isize + delta) as usize, (b as isize + delta) as usize))
            } else {
                None
            }
        })
        .collect()
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
    /// "every 3 days" / "every other day".
    EveryNDays(u32),
    /// "every 2 weeks [on mon and thu]" / "every other friday".
    EveryNWeeks(u32, Option<Vec<u8>>),
    /// "every 3 months" ("every 2 years" = 24 months) — day of the date.
    EveryNMonths(u32),
    /// "every year" (the date's month/day) or "every jan 5".
    Yearly(Option<(u32, u32)>),
}

/// A `{date}` group starting at token `i` (closed within a few words), whose
/// inside is exactly one date phrase: (tokens consumed, date).
fn match_deadline(input: &str, t: &[Token], i: usize, today: NaiveDate) -> Option<(usize, NaiveDate)> {
    let close = (i..t.len().min(i + 5)).find(|&j| t[j].raw.ends_with('}'))?;
    let inner = input.get(t[i].start + 1..t[close].end - 1)?;
    let words = tokens(inner);
    match match_date(&words, 0, today) {
        Some((n, date)) if n == words.len() => Some((close - i + 1, date)),
        _ => None,
    }
}

/// Vocabulary of repeat phrases (numbers, ordinals, weekdays, months, units).
fn repeat_word(word: &str) -> bool {
    word.parse::<u32>().is_ok()
        || day_number(word).is_some()
        || weekday(word).is_some()
        || month(word).is_some()
        || nth(word).is_some()
        || matches!(
            word,
            "other" | "and" | "of" | "the" | "on" | "every" | "each" | "day" | "days" | "week" | "weeks"
                | "month" | "months" | "year" | "years" | "workday" | "workdays" | "weekday" | "weekdays"
        )
}

/// "first".."fourth", "1st".."4th", "last" → 1..4 / -1.
fn nth(word: &str) -> Option<i8> {
    Some(match word {
        "first" | "1st" => 1,
        "second" | "2nd" => 2,
        "third" | "3rd" => 3,
        "fourth" | "4th" => 4,
        "last" => -1,
        _ => return None,
    })
}

/// A weekday list starting at token `j`: "mon", "mon, wed and fri".
/// Returns (sorted days, tokens consumed).
fn weekday_list(t: &[Token], j: usize) -> (Vec<u8>, usize) {
    let w = |k: usize| t.get(k).map(|x| x.norm.as_str()).unwrap_or("");
    let mut days = Vec::new();
    let mut k = j;
    loop {
        if let Some(wd) = weekday(w(k)) {
            days.push(wd as u8);
            k += 1;
        } else if w(k) == "and" && !days.is_empty() && weekday(w(k + 1)).is_some() {
            k += 1;
        } else {
            break;
        }
    }
    days.sort_unstable();
    days.dedup();
    (days, k - j)
}

/// Optional "of the month" / "of every month" / "of each month" at `j`.
fn of_month(t: &[Token], j: usize) -> usize {
    let w = |k: usize| t.get(k).map(|x| x.norm.as_str()).unwrap_or("");
    if w(j) == "of" && matches!(w(j + 1), "the" | "every" | "each") && w(j + 2) == "month" {
        3
    } else {
        0
    }
}

fn match_repeat(t: &[Token], i: usize) -> Option<(usize, RepeatSpec)> {
    let w = |k: usize| t.get(i + k).map(|x| x.norm.as_str()).unwrap_or("");
    let weekdays = || RepeatSpec::Rule(Recurrence::Weekly(vec![0, 1, 2, 3, 4]));
    match w(0) {
        "daily" | "everyday" => return Some((1, RepeatSpec::Rule(Recurrence::Daily))),
        "weekly" => return Some((1, RepeatSpec::Weekly)),
        "monthly" => return Some((1, RepeatSpec::Monthly)),
        "yearly" | "annually" => return Some((1, RepeatSpec::Yearly(None))),
        "every" => {}
        _ => return None,
    }
    match w(1) {
        "day" => return Some((2, RepeatSpec::Rule(Recurrence::Daily))),
        "weekday" | "weekdays" | "workday" | "workdays" => return Some((2, weekdays())),
        "week" => return Some((2, RepeatSpec::Weekly)),
        "year" => return Some((2, RepeatSpec::Yearly(None))),
        "month" => {
            // "every month on [the] 15th" / "every month on the last day"
            let (skip, rest) = match (w(2), w(3)) {
                ("on", "the") => (4, 4),
                ("on", _) => (3, 3),
                _ => (2, 2),
            };
            if skip > 2 {
                if w(rest) == "last" && w(rest + 1) == "day" {
                    return Some((rest + 2, RepeatSpec::Rule(Recurrence::MonthlyLastDay)));
                }
                if let Some(day) = day_number(w(rest)) {
                    return Some((rest + 1, RepeatSpec::Rule(Recurrence::MonthlyDay(day as u8))));
                }
            }
            return Some((2, RepeatSpec::Monthly));
        }
        _ => {}
    }

    // Yearly: "every jan 5", "every 5 january", "every january 5th".
    match (month(w(1)), day_number(w(2)), day_number(w(1)), month(w(2))) {
        (Some(m), Some(day), _, _) | (_, _, Some(day), Some(m)) => {
            return NaiveDate::from_ymd_opt(2024, m, day).map(|_| (3, RepeatSpec::Yearly(Some((m, day)))));
        }
        _ => {}
    }

    // Intervals: "every 3 days", "every other week [on mon and thu]", "every other friday".
    let count = match w(1) {
        "other" => Some(2),
        n => n.parse::<u32>().ok(),
    };
    if let Some(n) = count {
        if !(1..=99).contains(&n) {
            return None;
        }
        let spec = match w(2) {
            "day" | "days" => (3, if n == 1 { RepeatSpec::Rule(Recurrence::Daily) } else { RepeatSpec::EveryNDays(n) }),
            "week" | "weeks" => {
                let (days, used) = if w(3) == "on" { weekday_list(t, i + 4) } else { (Vec::new(), 0) };
                let consumed = if used > 0 { 4 + used } else { 3 };
                let days = (!days.is_empty()).then_some(days);
                (consumed, RepeatSpec::EveryNWeeks(n, days))
            }
            "month" | "months" => (3, if n == 1 { RepeatSpec::Monthly } else { RepeatSpec::EveryNMonths(n) }),
            "year" | "years" => (3, if n == 1 { RepeatSpec::Yearly(None) } else { RepeatSpec::EveryNMonths(12 * n) }),
            _ if w(1) == "other" => {
                let (days, used) = weekday_list(t, i + 2);
                if days.is_empty() {
                    return None;
                }
                (2 + used, RepeatSpec::EveryNWeeks(2, Some(days)))
            }
            _ => return None,
        };
        return Some(spec);
    }

    // "every last day [of the month]", "every first/last workday".
    if w(1) == "last" && w(2) == "day" {
        return Some((3 + of_month(t, i + 3), RepeatSpec::Rule(Recurrence::MonthlyLastDay)));
    }
    if let (Some(k), "workday") = (nth(w(1)), w(2)) {
        if k == 1 || k == -1 {
            return Some((3 + of_month(t, i + 3), RepeatSpec::Rule(Recurrence::MonthlyWorkday { nth: k })));
        }
        return None;
    }
    // "every first monday", "every 3rd tue", "every last friday".
    if let (Some(k), Some(wd)) = (nth(w(1)), weekday(w(2))) {
        let rule = Recurrence::MonthlyNthWeekday { nth: k, weekday: wd as u8 };
        return Some((3 + of_month(t, i + 3), RepeatSpec::Rule(rule)));
    }
    // "every 15th [of the month]" (an ordinal; "every 5th monday" is not a thing).
    if w(1).chars().any(|c| c.is_ascii_alphabetic()) {
        if let Some(day) = day_number(w(1)) {
            if weekday(w(2)).is_some() {
                return None;
            }
            return Some((2 + of_month(t, i + 2), RepeatSpec::Rule(Recurrence::MonthlyDay(day as u8))));
        }
    }
    // "every mon, wed and fri"
    let (days, used) = weekday_list(t, i + 1);
    if days.is_empty() {
        return None;
    }
    Some((1 + used, RepeatSpec::Rule(Recurrence::Weekly(days))))
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

/// Parses a quick-add title. Unknown projects/sections stay in the title. A
/// phrase on its own is still recognized (`content` is then empty); the
/// editors won't save until there's a name besides it, as in Todoist.
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
        // "{oct 30}" — a deadline (Todoist's brace syntax).
        if q.deadline.is_none() && t[i].raw.starts_with('{') {
            if let Some((n, date)) = match_deadline(input, &t, i, today) {
                q.deadline = Some(date);
                take(&mut used, &mut spans, i, n, SpanKind::Deadline);
                i += n;
                continue;
            }
        }
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
        // An "every ..." phrase we didn't understand (or a second repeat)
        // stays text as a whole, so e.g. "every 5th monday" doesn't turn
        // "monday" into a due date.
        if norm == "every" {
            i += 1;
            while i < t.len() && repeat_word(&t[i].norm) {
                i += 1;
            }
            continue;
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
            RepeatSpec::EveryNDays(n) => Recurrence::EveryNDays { n, anchor: start },
            RepeatSpec::EveryNWeeks(1, days) => Recurrence::Weekly(days.unwrap_or_else(|| vec![idx(start) as u8])),
            RepeatSpec::EveryNWeeks(n, days) => Recurrence::EveryNWeeks {
                n,
                days: days.unwrap_or_else(|| vec![idx(start) as u8]),
                anchor: start,
            },
            RepeatSpec::EveryNMonths(n) => Recurrence::EveryNMonths { n, day: start.day() as u8, anchor: start },
            RepeatSpec::Yearly(Some((m, d))) => Recurrence::Yearly { month: m as u8, day: d as u8 },
            RepeatSpec::Yearly(None) => Recurrence::Yearly { month: start.month() as u8, day: start.day() as u8 },
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
    spans.sort_by_key(|s| s.start);
    q.spans = spans;
    q
}

#[cfg(test)]
mod tests {
    use super::{parse, parse_keeping, shift_plain, strip_spans, QuickAdd, SpanKind};
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
    fn a_phrase_alone_is_still_recognized() {
        // Todoist-style: the phrase highlights as soon as it's typed; the
        // editors refuse to save until there's a name besides it.
        let q = p("tomorrow");
        assert_eq!(q.content, "");
        assert_eq!(q.date, Some(d(2026, 10, 20)));
        assert_eq!(q.spans.len(), 1);
        for input in ["This weekend", "This Weekend", "weekend"] {
            assert_eq!(p(input).date, Some(d(2026, 10, 24)), "{input}");
        }
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
    fn interval_repeats() {
        // today = Mon 2026-10-19
        let q = p("water plants every 3 days");
        assert_eq!(q.content, "water plants");
        assert_eq!(q.recurrence, Some(Recurrence::EveryNDays { n: 3, anchor: today() }));
        assert_eq!(q.date, Some(today()));
        assert_eq!(p("x every other day").recurrence, Some(Recurrence::EveryNDays { n: 2, anchor: today() }));
        assert_eq!(p("x every 1 day").recurrence, Some(Recurrence::Daily), "n = 1 is the plain rule");
        // Weeks anchor on the parsed date.
        let q = p("review every 2 weeks wed");
        assert_eq!(q.content, "review");
        assert_eq!(q.recurrence, Some(Recurrence::EveryNWeeks { n: 2, days: vec![2], anchor: d(2026, 10, 21) }));
        assert_eq!(q.date, Some(d(2026, 10, 21)));
        let q = p("gym every other week on mon and thu");
        assert_eq!(q.recurrence, Some(Recurrence::EveryNWeeks { n: 2, days: vec![0, 3], anchor: today() }));
        assert_eq!(q.date, Some(today()));
        let q = p("1:1 every other friday");
        assert_eq!(q.recurrence, Some(Recurrence::EveryNWeeks { n: 2, days: vec![4], anchor: today() }));
        assert_eq!(q.date, Some(d(2026, 10, 23)));
        assert_eq!(
            p("x every 3 months").recurrence,
            Some(Recurrence::EveryNMonths { n: 3, day: 19, anchor: today() })
        );
        assert_eq!(
            p("x every 2 years").recurrence,
            Some(Recurrence::EveryNMonths { n: 24, day: 19, anchor: today() })
        );
    }

    #[test]
    fn yearly_repeats() {
        assert_eq!(p("x every year").recurrence, Some(Recurrence::Yearly { month: 10, day: 19 }));
        assert_eq!(p("x yearly").recurrence, Some(Recurrence::Yearly { month: 10, day: 19 }));
        assert_eq!(p("x annually").recurrence, Some(Recurrence::Yearly { month: 10, day: 19 }));
        let q = p("mom's birthday every jan 5");
        assert_eq!(q.content, "mom's birthday");
        assert_eq!(q.recurrence, Some(Recurrence::Yearly { month: 1, day: 5 }));
        assert_eq!(q.date, Some(d(2027, 1, 5)));
        assert_eq!(p("x every 5 january").recurrence, Some(Recurrence::Yearly { month: 1, day: 5 }));
        assert_eq!(p("x every january 5th").recurrence, Some(Recurrence::Yearly { month: 1, day: 5 }));
    }

    #[test]
    fn month_relative_repeats() {
        let q = p("rent every last day of the month");
        assert_eq!(q.content, "rent");
        assert_eq!(q.recurrence, Some(Recurrence::MonthlyLastDay));
        assert_eq!(q.date, Some(d(2026, 10, 31)));
        assert_eq!(p("x every last day").recurrence, Some(Recurrence::MonthlyLastDay));
        assert_eq!(p("x every month on the last day").recurrence, Some(Recurrence::MonthlyLastDay));
        let q = p("invoice every last workday");
        assert_eq!(q.recurrence, Some(Recurrence::MonthlyWorkday { nth: -1 }));
        assert_eq!(q.date, Some(d(2026, 10, 30)));
        assert_eq!(p("x every first workday").recurrence, Some(Recurrence::MonthlyWorkday { nth: 1 }));
        assert_eq!(p("x every workday").recurrence, Some(Recurrence::Weekly(vec![0, 1, 2, 3, 4])));
        let q = p("book club every first monday");
        assert_eq!(q.content, "book club");
        assert_eq!(q.recurrence, Some(Recurrence::MonthlyNthWeekday { nth: 1, weekday: 0 }));
        assert_eq!(q.date, Some(d(2026, 11, 2)));
        assert_eq!(p("x every 3rd tuesday").recurrence, Some(Recurrence::MonthlyNthWeekday { nth: 3, weekday: 1 }));
        assert_eq!(p("x every last fri").recurrence, Some(Recurrence::MonthlyNthWeekday { nth: -1, weekday: 4 }));
        let q = p("pay card every 3rd of the month");
        assert_eq!(q.content, "pay card");
        assert_eq!(q.recurrence, Some(Recurrence::MonthlyDay(3)));
        assert_eq!(p("x every 15th of every month").content, "x");
    }

    #[test]
    fn unknown_repeats_stay_text() {
        for input in ["x every 2", "x every 0 days", "x every 5th monday", "x every 100 days", "x every other"] {
            let q = p(input);
            assert_eq!(q.recurrence, None, "{input}");
            assert_eq!(q.content, input, "{input}");
        }
    }

    #[test]
    fn spans_cover_new_repeats() {
        let input = "Gym every other week on mon and thu at 7am";
        let q = p(input);
        let got: Vec<&str> = q.spans.iter().map(|s| &input[s.start..s.end]).collect();
        assert_eq!(got, vec!["every other week on mon and thu", "at 7am"]);
    }

    #[test]
    fn deadline_in_braces() {
        let input = "Ship report tomorrow {oct 30} p1";
        let q = p(input);
        assert_eq!(q.content, "Ship report");
        assert_eq!(q.deadline, Some(d(2026, 10, 30)));
        assert_eq!(q.date, Some(d(2026, 10, 20)), "due and deadline are separate");
        let deadline_spans: Vec<&str> = q
            .spans
            .iter()
            .filter(|s| s.kind == super::SpanKind::Deadline)
            .map(|s| &input[s.start..s.end])
            .collect();
        assert_eq!(deadline_spans, vec!["{oct 30}"]);
        assert_eq!(p("x {friday}").deadline, Some(d(2026, 10, 23)));
        assert_eq!(p("x {next week}").deadline, Some(d(2026, 10, 26)));
        // Not a date, or unclosed: stays text.
        let q = p("x {someday}");
        assert_eq!((q.deadline, q.content.as_str()), (None, "x {someday}"));
        let q = p("x {oct 30");
        assert_eq!(q.deadline, None);
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
        assert_eq!(p("tomorrow").spans.len(), 1, "a phrase alone still highlights");
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

    #[test]
    fn strip_spans_removes_every_span_of_the_kinds() {
        let input = "@home call mom @phone";
        assert_eq!(strip_spans(input, &p(input).spans, &[SpanKind::Label]), "call mom");
        let input = "call mom tomorrow 5pm";
        assert_eq!(strip_spans(input, &p(input).spans, &[SpanKind::Date, SpanKind::Time]), "call mom");
    }

    #[test]
    fn strip_spans_leaves_unrecognized_repeats_as_text() {
        // Only the first date is applied; the second "tomorrow" is title text.
        let input = "tomorrow call mom tomorrow";
        assert_eq!(strip_spans(input, &p(input).spans, &[SpanKind::Date]), "call mom tomorrow");
    }

    #[test]
    fn strip_spans_keeps_other_kinds() {
        let input = "call mom tomorrow p1";
        assert_eq!(strip_spans(input, &p(input).spans, &[SpanKind::Priority]), "call mom tomorrow");
    }

    #[test]
    fn strip_spans_collapses_spaces() {
        let input = "a  p1  b";
        assert_eq!(strip_spans(input, &p(input).spans, &[SpanKind::Priority]), "a b");
    }

    fn pk(input: &str, plain: &[(usize, usize)]) -> QuickAdd {
        parse_keeping(input, today(), &projects(), &sections(), plain)
    }

    #[test]
    fn a_kept_phrase_stays_plain_text() {
        let q = pk("call mom today", &[(9, 14)]);
        assert_eq!(q.date, None);
        assert_eq!(q.content, "call mom today");
        assert!(q.spans.is_empty());
    }

    #[test]
    fn other_phrases_still_parse_around_a_kept_one() {
        let q = pk("buy today paper p1 tomorrow", &[(4, 9)]);
        assert_eq!(q.priority, Some(Priority::P1));
        assert_eq!(q.date, Some(d(2026, 10, 20)));
        assert_eq!(q.content, "buy today paper");
        assert!(q.spans.iter().all(|s| s.start >= 9));
    }

    #[test]
    fn no_kept_phrases_is_a_plain_parse() {
        assert_eq!(pk("call mom today", &[]), p("call mom today"));
    }

    #[test]
    fn kept_ranges_follow_edits_around_them() {
        // Typing after the kept word: it stays put.
        assert_eq!(shift_plain("x today", "x today 5pm", &[(2, 7)]), vec![(2, 7)]);
        // Typing before it: it moves along.
        assert_eq!(shift_plain("x today", "abc x today", &[(2, 7)]), vec![(6, 11)]);
        // Editing inside it: it's forgotten (read again).
        assert_eq!(shift_plain("x today", "x toda", &[(2, 7)]), vec![]);
        assert_eq!(shift_plain("x today", "x tOday", &[(2, 7)]), vec![]);
    }

    #[test]
    fn this_weekend_in_any_case() {
        for input in ["x this weekend", "x This weekend", "x This Weekend", "x THIS WEEKEND", "x weekend", "x Weekend"] {
            let q = p(input);
            assert_eq!(q.date, Some(d(2026, 10, 24)), "{input}");
            assert_eq!(q.content, "x", "{input}");
        }
    }

}
