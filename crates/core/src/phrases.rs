//! The words quick add and filters share: dates, times, deadlines, repeats
//! and names, matched over whitespace-split tokens.

use chrono::{Datelike, Duration, Months, NaiveDate, NaiveTime};

use crate::model::Priority;
use crate::recurrence::Recurrence;

pub(crate) struct Token {
    pub(crate) raw: String,
    /// Lower-cased with trailing punctuation removed.
    pub(crate) norm: String,
    /// Byte range of `raw` in the input.
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn tokens(input: &str) -> Vec<Token> {
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
pub(crate) fn weekday(word: &str) -> Option<u32> {
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

pub(crate) fn month(word: &str) -> Option<u32> {
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
pub(crate) fn day_number(word: &str) -> Option<u32> {
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
pub(crate) fn match_date(t: &[Token], i: usize, today: NaiveDate) -> Option<(usize, NaiveDate)> {
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
    let (m, day, used) = match (month(w(0)), day_number(w(1)), day_number(w(0)), month(w(1))) {
        (Some(m), Some(day), _, _) => (m, day, 2),
        (_, _, Some(day), Some(m)) => (m, day, 2),
        // A bare month is its 1st ("dec", "dec 2028"); "may" is too often a verb.
        (Some(m), None, _, _) if w(0) != "may" => (m, 1, 1),
        _ => return None,
    };
    let year = w(used).parse::<i32>().ok().filter(|y| (1000..=9999).contains(y));
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
    Some((if year.is_some() { used + 1 } else { used }, date))
}

/// `5pm`, `5:30pm`, `12am`, `9a`, `10p` → time.
fn clock_ampm(word: &str) -> Option<NaiveTime> {
    let (num, pm) = if let Some(n) = word.strip_suffix("pm").or(word.strip_suffix('p')) {
        (n, true)
    } else if let Some(n) = word.strip_suffix("am").or(word.strip_suffix('a')) {
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
pub(crate) fn match_time(t: &[Token], i: usize) -> Option<(usize, NaiveTime)> {
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

/// A repeat that may need the task's date to anchor on.
#[derive(Clone, Debug, PartialEq)]
pub enum RepeatSpec {
    Rule(Recurrence),
    /// "every week" — weekday of the date.
    Weekly,
    /// "every month" — day of the date.
    Monthly,
    /// "every 3 days" / "every other day".
    EveryNDays(u32),
    /// "every 2 weeks [on mon and thu]" / "every other friday".
    EveryNWeeks(u32, Option<Vec<u8>>),
    /// "every 3 months [on the 1st]" ("every 2 years" = 24 months) — the
    /// given day, or the date's.
    EveryNMonths(u32, Option<u8>),
    /// "every year" (the date's month/day) or "every jan 5".
    Yearly(Option<(u32, u32)>),
}

/// A `{date}` group starting at token `i` (closed within a few words), whose
/// inside is exactly one date phrase: (tokens consumed, date).
pub(crate) fn match_deadline(input: &str, t: &[Token], i: usize, today: NaiveDate) -> Option<(usize, NaiveDate)> {
    let close = (i..t.len().min(i + 5)).find(|&j| t[j].raw.ends_with('}'))?;
    let inner = input.get(t[i].start + 1..t[close].end - 1)?;
    let words = tokens(inner);
    match match_date(&words, 0, today) {
        Some((n, date)) if n == words.len() => Some((close - i + 1, date)),
        _ => None,
    }
}

/// Vocabulary of repeat phrases (numbers, ordinals, weekdays, months, units).
pub(crate) fn repeat_word(word: &str) -> bool {
    word.parse::<u32>().is_ok()
        || day_number(word).is_some()
        || weekday(word).is_some()
        || month(word).is_some()
        || nth(word).is_some()
        || matches!(
            word,
            "other" | "and" | "&" | "of" | "the" | "on" | "every" | "each" | "day" | "days" | "week" | "weeks"
                | "month" | "months" | "year" | "years" | "workday" | "workdays" | "weekday" | "weekdays"
                | "weekend" | "weekends" | "wk" | "wks" | "fortnight"
        )
        || number_word(word).is_some()
}

/// "one".."twelve" → 1..12.
pub(crate) fn number_word(word: &str) -> Option<u32> {
    let words = ["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    words.iter().position(|w| *w == word).map(|i| i as u32 + 1)
}

/// An ordinal that counts: "second", "3rd", "15th" (not "last", not a bare "3").
fn ordinal_count(word: &str) -> Option<u32> {
    match nth(word) {
        Some(k) if k > 0 => Some(k as u32),
        Some(_) => None,
        None if word.chars().any(|c| c.is_ascii_alphabetic()) => day_number(word),
        None => None,
    }
}

fn is_unit(word: &str) -> bool {
    matches!(word, "day" | "days" | "week" | "weeks" | "wk" | "wks" | "month" | "months" | "year" | "years")
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
    // "mondays" too, in a repeat.
    let day = |word: &str| weekday(word).or_else(|| word.strip_suffix('s').and_then(weekday));
    loop {
        if let Some(wd) = day(w(k)) {
            days.push(wd as u8);
            k += 1;
        } else if matches!(w(k), "and" | "&") && !days.is_empty() && day(w(k + 1)).is_some() {
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

pub(crate) fn match_repeat(t: &[Token], i: usize) -> Option<(usize, RepeatSpec)> {
    let w = |k: usize| t.get(i + k).map(|x| x.norm.as_str()).unwrap_or("");
    let weekdays = || RepeatSpec::Rule(Recurrence::Weekly(vec![0, 1, 2, 3, 4]));
    match w(0) {
        "daily" | "everyday" => return Some((1, RepeatSpec::Rule(Recurrence::Daily))),
        "weekly" => return Some((1, RepeatSpec::Weekly)),
        "monthly" => return Some((1, RepeatSpec::Monthly)),
        "yearly" | "annually" => return Some((1, RepeatSpec::Yearly(None))),
        "biweekly" | "fortnightly" => return Some((1, RepeatSpec::EveryNWeeks(2, None))),
        "every" => {}
        _ => return None,
    }
    match w(1) {
        "day" => return Some((2, RepeatSpec::Rule(Recurrence::Daily))),
        "weekday" | "weekdays" | "workday" | "workdays" => return Some((2, weekdays())),
        "week" => return Some((2, RepeatSpec::Weekly)),
        "year" => return Some((2, RepeatSpec::Yearly(None))),
        "weekend" | "weekends" => return Some((2, RepeatSpec::Rule(Recurrence::Weekly(vec![5, 6])))),
        "fortnight" => return Some((2, RepeatSpec::EveryNWeeks(2, None))),
        "month" => {
            // "every month on [the] 15th" / "every month on the last day"
            let (skip, rest) = match (w(2), w(3)) {
                ("on", "the") => (4, 4),
                ("on", _) => (3, 3),
                _ => (2, 2),
            };
            if skip > 2 {
                // "every month on the 2nd monday"
                if let (Some(k), Some(wd)) = (nth(w(rest)), weekday(w(rest + 1))) {
                    return Some((rest + 2, RepeatSpec::Rule(Recurrence::MonthlyNthWeekday { nth: k, weekday: wd as u8 })));
                }
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

    // Yearly: "every jan 5", "every 5 january", "every january 5th",
    // "every march on the 3rd"; a month alone is its 1st ("every january").
    let yearly = |used: usize, m: u32, day: u32| {
        NaiveDate::from_ymd_opt(2024, m, day).map(|_| (used, RepeatSpec::Yearly(Some((m, day)))))
    };
    match (month(w(1)), day_number(w(2)), day_number(w(1)), month(w(2))) {
        (Some(m), Some(day), _, _) | (_, _, Some(day), Some(m)) => return yearly(3, m, day),
        (Some(m), None, _, _) => {
            let at = match (w(2), w(3)) {
                ("on", "the") => 4,
                ("on", _) => 3,
                _ => 0,
            };
            return match (at > 0).then(|| day_number(w(at))).flatten() {
                Some(day) => yearly(at + 1, m, day),
                None => yearly(2, m, 1),
            };
        }
        _ => {}
    }

    // "every 2nd day of the month" is a date; "every 2nd week of the month
    // [on fri]" is that week's Monday (or the given weekday).
    let of = of_month(t, i + 3);
    if of > 0 && (w(2) == "week" || (w(2) == "day" && w(1) != "last")) {
        let k = if w(1) == "last" { -1 } else { ordinal_count(w(1))? as i8 };
        if w(2) == "day" {
            return (k > 0).then_some((3 + of, RepeatSpec::Rule(Recurrence::MonthlyDay(k as u8))));
        }
        if k > 4 {
            return None;
        }
        let (weekday, used) = match (w(3 + of), weekday(w(4 + of))) {
            ("on", Some(wd)) if !matches!(w(5 + of), "and" | "&") => (wd as u8, 2),
            _ => (0, 0),
        };
        return Some((3 + of + used, RepeatSpec::Rule(Recurrence::MonthlyNthWeekday { nth: k, weekday })));
    }

    // Intervals: "every 3 days", "every two weeks", "every second week",
    // "every other week [on mon and thu]", "every other friday", "every 3 tuesdays".
    let count = match w(1) {
        "other" => Some(2),
        n => n
            .parse::<u32>()
            .ok()
            .or_else(|| number_word(n))
            .or_else(|| ordinal_count(n).filter(|_| is_unit(w(2)))),
    };
    if let Some(n) = count {
        if !(1..=99).contains(&n) {
            return None;
        }
        let spec = match w(2) {
            "day" | "days" => (3, if n == 1 { RepeatSpec::Rule(Recurrence::Daily) } else { RepeatSpec::EveryNDays(n) }),
            "week" | "weeks" | "wk" | "wks" => {
                let (days, used) = if w(3) == "on" { weekday_list(t, i + 4) } else { (Vec::new(), 0) };
                let consumed = if used > 0 { 4 + used } else { 3 };
                let days = (!days.is_empty()).then_some(days);
                (consumed, RepeatSpec::EveryNWeeks(n, days))
            }
            "month" | "months" => {
                // "every 3 months on [the] 1st"
                let at = if w(3) == "on" { if w(4) == "the" { 5 } else { 4 } } else { 0 };
                let day = (at > 0).then(|| day_number(w(at))).flatten();
                match (n, day) {
                    (1, Some(day)) => (at + 1, RepeatSpec::Rule(Recurrence::MonthlyDay(day as u8))),
                    (1, None) => (3, RepeatSpec::Monthly),
                    (n, Some(day)) => (at + 1, RepeatSpec::EveryNMonths(n, Some(day as u8))),
                    (n, None) => (3, RepeatSpec::EveryNMonths(n, None)),
                }
            }
            "year" | "years" => (3, if n == 1 { RepeatSpec::Yearly(None) } else { RepeatSpec::EveryNMonths(12 * n, None) }),
            // "every other friday", "every 3 tuesdays": every n weeks on those days.
            _ => {
                let (days, used) = weekday_list(t, i + 2);
                if days.is_empty() {
                    return None;
                }
                let spec = if n == 1 { RepeatSpec::Rule(Recurrence::Weekly(days)) } else { RepeatSpec::EveryNWeeks(n, Some(days)) };
                (2 + used, spec)
            }
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
pub(crate) fn match_name(t: &[Token], i: usize, sigil: &str, names: &[&str]) -> Option<(usize, usize)> {
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

/// Whether a word after `#` or `@` can be a name: it needs a letter ("#123"
/// and "@1" stay text).
pub(crate) fn is_name(word: &str) -> bool {
    word.chars().any(|c| c.is_alphabetic())
}

/// The one-word name after `sigil` in `raw` (trailing `,.;!?` trimmed),
/// when it can be a name.
pub(crate) fn sigil_name(raw: &str, sigil: &str) -> Option<String> {
    let name = raw.strip_prefix(sigil)?.trim_end_matches([',', '.', ';', '!', '?']);
    is_name(name).then(|| name.to_string())
}

/// `p1`–`p4` at word `i` (`p4` is no priority).
pub(crate) fn match_priority(t: &[Token], i: usize) -> Option<Priority> {
    match t.get(i)?.norm.as_str() {
        "p1" => Some(Priority::P1),
        "p2" => Some(Priority::P2),
        "p3" => Some(Priority::P3),
        "p4" => Some(Priority::None),
        _ => None,
    }
}

/// The words an `every …` phrase at `i` covers when it couldn't be read as a
/// repeat (it stays text as a whole); 0 when `t[i]` isn't "every".
pub(crate) fn unread_every(t: &[Token], i: usize) -> usize {
    if t.get(i).is_none_or(|w| w.norm != "every") {
        return 0;
    }
    1 + t[i + 1..].iter().take_while(|w| repeat_word(&w.norm)).count()
}

/// The rule a repeat phrase means when the task starts on `start`.
pub(crate) fn resolve_repeat(spec: RepeatSpec, start: NaiveDate) -> Recurrence {
    match spec {
        RepeatSpec::Rule(r) => r,
        RepeatSpec::Weekly => Recurrence::Weekly(vec![idx(start) as u8]),
        RepeatSpec::Monthly => Recurrence::MonthlyDay(start.day() as u8),
        RepeatSpec::EveryNDays(n) => Recurrence::EveryNDays { n, anchor: start },
        RepeatSpec::EveryNWeeks(1, days) => Recurrence::Weekly(days.unwrap_or_else(|| vec![idx(start) as u8])),
        RepeatSpec::EveryNWeeks(n, days) => {
            // The first chosen day on/after the start opens the cycle, so
            // a day already past this week doesn't push it n weeks out.
            let days = days.unwrap_or_else(|| vec![idx(start) as u8]);
            let weekly = Recurrence::Weekly(days.clone());
            let anchor = if weekly.contains(start) { start } else { weekly.next_after(start) };
            Recurrence::EveryNWeeks { n, days, anchor }
        }
        RepeatSpec::EveryNMonths(n, None) => Recurrence::EveryNMonths { n, day: start.day() as u8, anchor: start },
        RepeatSpec::EveryNMonths(n, Some(day)) => {
            let monthly = Recurrence::MonthlyDay(day);
            let anchor = if monthly.contains(start) { start } else { monthly.next_after(start) };
            Recurrence::EveryNMonths { n, day, anchor }
        }
        RepeatSpec::Yearly(Some((m, d))) => Recurrence::Yearly { month: m as u8, day: d as u8 },
        RepeatSpec::Yearly(None) => Recurrence::Yearly { month: start.month() as u8, day: start.day() as u8 },
    }
}

/// Whether a task's `rule` has the kind and interval the phrase names.
/// Anchors, start and end dates are ignored; a phrase's weekdays need only
/// be among the rule's.
pub(crate) fn repeat_matches(spec: &RepeatSpec, rule: &Recurrence) -> bool {
    let subset = |want: &[u8], have: &[u8]| want.iter().all(|d| have.contains(d));
    // A weekly rule's days, whether it is `Weekly` or every 1st week.
    let weekly = match rule {
        Recurrence::Weekly(days) | Recurrence::EveryNWeeks { n: 1, days, .. } => Some(days),
        _ => None,
    };
    match spec {
        RepeatSpec::Rule(Recurrence::Weekly(want)) => weekly.is_some_and(|have| subset(want, have)),
        RepeatSpec::Rule(want) => want == rule,
        RepeatSpec::Weekly | RepeatSpec::EveryNWeeks(1, None) => weekly.is_some(),
        RepeatSpec::EveryNWeeks(1, Some(want)) => weekly.is_some_and(|have| subset(want, have)),
        RepeatSpec::Monthly => matches!(rule, Recurrence::MonthlyDay(_)),
        RepeatSpec::EveryNDays(n) => matches!(rule, Recurrence::EveryNDays { n: m, .. } if m == n),
        RepeatSpec::EveryNWeeks(n, want) => match rule {
            Recurrence::EveryNWeeks { n: m, days, .. } if m == n => want.as_ref().is_none_or(|w| subset(w, days)),
            _ => false,
        },
        RepeatSpec::EveryNMonths(n, want) => match rule {
            Recurrence::EveryNMonths { n: m, day, .. } if m == n => want.is_none_or(|w| w == *day),
            _ => false,
        },
        RepeatSpec::Yearly(None) => matches!(rule, Recurrence::Yearly { .. }),
        RepeatSpec::Yearly(Some((m, d))) => {
            matches!(rule, Recurrence::Yearly { month, day } if u32::from(*month) == *m && u32::from(*day) == *d)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn check(phrase: &str, yes: &[Recurrence], no: &[Recurrence]) {
        let (_, spec) = match_repeat(&tokens(phrase), 0).unwrap_or_else(|| panic!("{phrase} not read"));
        for r in yes {
            assert!(repeat_matches(&spec, r), "{phrase} should match {r:?}");
        }
        for r in no {
            assert!(!repeat_matches(&spec, r), "{phrase} should not match {r:?}");
        }
    }

    fn weeks(n: u32, days: &[u8], anchor: NaiveDate) -> Recurrence {
        Recurrence::EveryNWeeks { n, days: days.to_vec(), anchor }
    }

    #[test]
    fn shared_word_helpers() {
        assert_eq!(sigil_name("#Work,", "#"), Some("Work".into()));
        assert_eq!(sigil_name("#123", "#"), None);
        assert_eq!(sigil_name("@x", "#"), None);
        let t = tokens("p2 every 5th monday later");
        assert_eq!(match_priority(&t, 0), Some(Priority::P2));
        assert_eq!(match_priority(&t, 1), None);
        assert_eq!(unread_every(&t, 1), 3);
        assert_eq!(unread_every(&t, 0), 0);
    }

    #[test]
    fn repeat_phrase_matches_by_kind_and_interval() {
        let a = d(2026, 10, 1);
        let b = d(2026, 11, 5);
        check("every day", &[Recurrence::Daily], &[Recurrence::Weekly(vec![0])]);
        check(
            "every week",
            &[Recurrence::Weekly(vec![0]), Recurrence::Weekly(vec![2, 4])],
            &[weeks(2, &[0], a)],
        );
        check(
            "every monday",
            &[Recurrence::Weekly(vec![0]), Recurrence::Weekly(vec![0, 2])],
            &[Recurrence::Weekly(vec![1])],
        );
        check(
            "every 2 weeks",
            &[weeks(2, &[3], a), weeks(2, &[3], b)],
            &[weeks(3, &[3], a), Recurrence::Weekly(vec![3])],
        );
        check(
            "every 2 weeks on mon and wed",
            &[weeks(2, &[0, 2, 4], a)],
            &[weeks(2, &[0], a)],
        );
        check(
            "every 3 days",
            &[Recurrence::EveryNDays { n: 3, anchor: a }, Recurrence::EveryNDays { n: 3, anchor: b }],
            &[Recurrence::EveryNDays { n: 4, anchor: a }],
        );
        check(
            "every month",
            &[Recurrence::MonthlyDay(5), Recurrence::MonthlyDay(20)],
            &[Recurrence::EveryNMonths { n: 2, day: 5, anchor: a }],
        );
        check(
            "every 3 months",
            &[Recurrence::EveryNMonths { n: 3, day: 9, anchor: b }],
            &[Recurrence::EveryNMonths { n: 2, day: 9, anchor: b }],
        );
        check(
            "every year",
            &[Recurrence::Yearly { month: 3, day: 1 }],
            &[Recurrence::MonthlyDay(1)],
        );
        check(
            "every last day",
            &[Recurrence::MonthlyLastDay],
            &[Recurrence::MonthlyDay(31)],
        );
    }
}
