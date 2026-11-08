//! Natural-language quick add: pull dates, times, repeats, priority, project,
//! section and labels out of a typed task title (English, Todoist-style).
//!
//! Matching is on whole whitespace-separated words (trailing `,.;!?` ignored),
//! so "today's" or "Mondays" stay text. The first date, time and repeat win.
//! "streak" counts only beside the repeat phrase, and is otherwise just a word.

use chrono::{Duration, Months, NaiveDate, NaiveTime};

use crate::ids::Id;
use crate::model::{Priority, Project, Section};
use crate::phrases::{
    is_name, match_date, match_deadline, match_name, match_repeat, match_time, number_word, repeat_word, resolve_repeat, tokens,
    RepeatSpec, Token,
};
use crate::recurrence::Recurrence;

/// What was understood from a quick-add title.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct QuickAdd {
    /// The title with recognized phrases removed.
    pub content: String,
    pub date: Option<NaiveDate>,
    pub time: Option<NaiveTime>,
    pub recurrence: Option<Recurrence>,
    pub priority: Option<Priority>,
    pub project_id: Option<Id>,
    pub section_id: Option<Id>,
    /// From `#name` when no project has that name: the typed name (case kept),
    /// to be created on save. `project_id` stays `None`.
    pub new_project: Option<String>,
    pub labels: Vec<String>,
    /// From `{oct 30}`: the "must be done by" date.
    pub deadline: Option<NaiveDate>,
    /// From "for 10 days" / "until nov 30" after the repeat: its last day.
    pub until: Option<NaiveDate>,
    /// From "streak" beside the repeat phrase: track a streak.
    pub streak: bool,
    /// From "streak 30": its goal.
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub streak_goal: Option<i64>,
    /// The recognized phrases, as byte ranges into the input (sorted), for
    /// highlighting. Empty when nothing was applied.
    pub spans: Vec<Span>,
}

/// A recognized phrase in the input (`input[start..end]`).
// `start`/`end` are UTF-8 byte offsets into the Rust string, not JS UTF-16
// indices. Any wasm export that returns spans (Plan 3's quick add) must
// convert them first. A plain comment, not `///`: ts-rs copies doc comments
// into the binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: SpanKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum SpanKind {
    Date,
    Time,
    Repeat,
    Priority,
    Project,
    Section,
    Label,
    Deadline,
    Streak,
    // A filter keyword: `overdue`, `no date`, `next 7 days`, `due before: …`.
    Keyword,
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
            && self.new_project.is_none()
            && self.labels.is_empty()
            && self.deadline.is_none()
    }
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
    let (mut q, used) = parse_tokens(&masked, today, today, projects, sections);
    // The title is the words the parser didn't use, as in `parse`, taken from
    // the real text. Masking keeps every word boundary, so the words line up.
    q.content = unused_words(&tokens(input), &used);
    q
}

/// The title left after parsing: the unused words, single-spaced.
fn unused_words(tokens: &[Token], used: &[bool]) -> String {
    tokens.iter().zip(used).filter(|(_, used)| !**used).map(|(tok, _)| tok.raw.as_str()).collect::<Vec<_>>().join(" ")
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

/// A time typed on its own ("5pm", "5:30 pm", "17:00", "noon"), for time
/// fields; `None` unless the whole text is one time phrase.
pub fn parse_time(text: &str) -> Option<NaiveTime> {
    let t = tokens(text);
    match match_time(&t, 0) {
        Some((n, time)) if n == t.len() => Some(time),
        _ => None,
    }
}

/// Parses a quick-add title. Unknown projects/sections stay in the title. A
/// phrase on its own is still recognized (`content` is then empty); the
/// editors won't save until there's a name besides it, as in Todoist.
pub fn parse(input: &str, today: NaiveDate, projects: &[Project], sections: &[Section]) -> QuickAdd {
    parse_tokens(input, today, today, projects, sections).0
}

/// A repeat typed on its own (the Repeat menu's field): the phrase, with an
/// optional start, end and time, and nothing else. "every" may be left off.
/// `start` anchors the repeat when no start date is typed.
pub fn parse_repeat(text: &str, today: NaiveDate, start: NaiveDate) -> Option<QuickAdd> {
    let read = |input: &str| {
        let q = parse_tokens(input, today, start, &[], &[]).0;
        let only = q.content.trim().is_empty()
            && q.recurrence.is_some()
            && q.priority.is_none()
            && q.labels.is_empty()
            && q.deadline.is_none()
            && q.project_id.is_none()
            && !q.streak;
        only.then_some(q)
    };
    let text = text.trim();
    read(text).or_else(|| read(&format!("every {text}")))
}

/// `parse`, also returning which of `input`'s words it used.
/// `start` anchors a repeat when no date is typed (quick add: today).
fn parse_tokens(
    input: &str,
    today: NaiveDate,
    start: NaiveDate,
    projects: &[Project],
    sections: &[Section],
) -> (QuickAdd, Vec<bool>) {
    let t = tokens(input);
    let mut used = vec![false; t.len()];
    let mut q = QuickAdd::default();
    let mut repeat: Option<RepeatSpec> = None;
    // Where the repeat phrase is: (first word, words).
    let mut repeat_at: Option<(usize, usize)> = None;
    // "for 3 weeks": how long the repeat runs, counted from its first day.
    let mut repeat_for: Option<(Months, i64)> = None;
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
        if q.project_id.is_none() && q.new_project.is_none() && t[i].raw.starts_with('#') {
            if let Some((n, pos)) = match_name(&t, i, "#", &project_names) {
                q.project_id = Some(projects[pos].id.clone());
                take(&mut used, &mut spans, i, n, SpanKind::Project);
                i += n;
                continue;
            }
        }
        if q.project_id.is_none() && q.new_project.is_none() && t[i].raw.starts_with('#') {
            let name = t[i].raw[1..].trim_end_matches([',', '.', ';', '!', '?']);
            if is_name(name) {
                q.new_project = Some(name.to_string());
                take(&mut used, &mut spans, i, 1, SpanKind::Project);
                i += 1;
                continue;
            }
        }
        if t[i].raw.len() > 1 && t[i].raw.starts_with('@') {
            let name = t[i].raw[1..].trim_end_matches([',', '.', ';', '!', '?']).to_string();
            if is_name(&name) {
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
                repeat_at = Some((i, n));
                take(&mut used, &mut spans, i, n, SpanKind::Repeat);
                i += n;
                continue;
            }
        }
        // "until nov 30" / "for 10 days" after a repeat: when it ends.
        if repeat.is_some() && q.until.is_none() && repeat_for.is_none() {
            if norm == "until" {
                if let Some((n, date)) = match_date(&t, i + 1, today) {
                    q.until = Some(date);
                    take(&mut used, &mut spans, i, n + 1, SpanKind::Repeat);
                    i += n + 1;
                    continue;
                }
            }
            let w = |k: usize| t.get(k).map(|x| x.norm.as_str()).unwrap_or("");
            let count = w(i + 1).parse::<u32>().ok().or_else(|| number_word(w(i + 1))).filter(|n| (1..=99).contains(n));
            if let (true, Some(n)) = (norm == "for", count) {
                let span = match w(i + 2) {
                    "day" | "days" => Some((Months::new(0), n as i64)),
                    "week" | "weeks" => Some((Months::new(0), 7 * n as i64)),
                    "month" | "months" => Some((Months::new(n), 0)),
                    "year" | "years" => Some((Months::new(12 * n), 0)),
                    _ => None,
                };
                if span.is_some() {
                    repeat_for = span;
                    take(&mut used, &mut spans, i, 3, SpanKind::Repeat);
                    i += 3;
                    continue;
                }
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
            // "every 10 days starting nov 1": the start date of a repeat.
            if repeat.is_some() && matches!(norm, "starting" | "from" | "beginning") {
                if let Some((n, date)) = match_date(&t, i + 1, today) {
                    q.date = Some(date);
                    take(&mut used, &mut spans, i, n + 1, SpanKind::Date);
                    i += n + 1;
                    continue;
                }
            }
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
    let scope = if q.new_project.is_some() { None } else { q.project_id.clone().or(inbox.clone()) };
    if let Some(scope) = scope {
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

    // "streak" (or "streak 30") as the next words either side of the repeat;
    // other phrases may sit between ("every day 5pm streak"). A streak means
    // nothing without a repeat, so with none it stays text.
    if let Some((at, n)) = repeat_at {
        let goal_at = |j: usize| {
            let digits = t.get(j).filter(|w| !used[j] && w.norm.bytes().all(|b| b.is_ascii_digit()))?;
            digits.norm.parse::<i64>().ok().filter(|g| *g > 0)
        };
        let is_streak = |j: &usize| t[*j].norm == "streak";
        let after = (at + n..t.len()).find(|&j| !used[j]).filter(is_streak);
        let before = (0..at).rev().find(|&j| !used[j]).and_then(|j| {
            // The word before the repeat may be the goal, with "streak" right before it.
            if goal_at(j).is_some() { j.checked_sub(1).filter(|k| !used[*k]) } else { Some(j) }
        });
        if let Some(j) = after.or(before.filter(is_streak)) {
            q.streak = true;
            q.streak_goal = goal_at(j + 1);
            let words = if q.streak_goal.is_some() { 2 } else { 1 };
            take(&mut used, &mut spans, j, words, SpanKind::Streak);
        }
    }

    // Repeats anchor on the parsed date (or today); the due becomes the first occurrence.
    if let Some(spec) = repeat {
        let start = q.date.unwrap_or(start);
        let rule = resolve_repeat(spec, start);
        let first = if rule.contains(start) { start } else { rule.next_after(start) };
        q.date = Some(first);
        if let Some((months, days)) = repeat_for {
            let end = first.checked_add_months(months).unwrap_or(first) + Duration::days(days);
            q.until = Some(end - Duration::days(1));
        }
        q.recurrence = Some(rule);
    } else if q.time.is_some() && q.date.is_none() {
        q.date = Some(today);
    }

    q.content = unused_words(&t, &used);
    spans.sort_by_key(|s| s.start);
    q.spans = spans;
    (q, used)
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
    fn bare_month_is_the_first() {
        assert_eq!(date_of("x dec"), Some(d(2026, 12, 1)));
        assert_eq!(date_of("x Sep"), Some(d(2027, 9, 1)), "past date rolls to next year");
        assert_eq!(date_of("x oct"), Some(d(2027, 10, 1)));
        assert_eq!(date_of("x dec 2028"), Some(d(2028, 12, 1)));
        assert_eq!(date_of("x dec 25"), Some(d(2026, 12, 25)), "a day still wins");
        assert_eq!(p("x dec").content, "x");
        assert_eq!(date_of("x may"), None, "a verb, not a month");
        assert_eq!(date_of("x may 5"), Some(d(2027, 5, 5)));
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
    fn short_am_pm() {
        let q = p("x tomorrow at 9a");
        assert_eq!(q.due_string().as_deref(), Some("2026-10-20T09:00:00"));
        assert_eq!(q.content, "x");
        assert_eq!(p("x 10p").time, Some(t(22, 0)));
        assert_eq!(p("x 5:30p").time, Some(t(17, 30)));
        assert_eq!(p("x 12a").time, Some(t(0, 0)));
        assert_eq!(p("x 12p").time, Some(t(12, 0)));
        assert_eq!(p("x 9A").time, Some(t(9, 0)));
    }

    #[test]
    fn short_am_pm_stays_text_unless_attached_to_a_valid_hour() {
        for input in ["x 5 a", "x 5 p", "x 13p", "x 0a", "x a", "x map"] {
            let q = p(input);
            assert_eq!(q.time, None, "{input}");
            assert_eq!(q.content, input, "{input}");
        }
    }

    #[test]
    fn streak_counts_only_beside_a_repeat() {
        let q = p("Run every day streak");
        assert!(q.streak);
        assert_eq!(q.content, "Run");
        assert_eq!(q.recurrence, Some(Recurrence::Daily));
        assert_eq!(q.spans.last().map(|s| s.kind), Some(SpanKind::Streak));

        let q = p("Run streak every day");
        assert!(q.streak);
        assert_eq!(q.content, "Run");
        assert_eq!(q.spans.iter().map(|s| s.kind).collect::<Vec<_>>(), [SpanKind::Streak, SpanKind::Repeat]);

        // Other phrases between it and the repeat don't matter; words do.
        assert!(p("Run every day 5pm streak").streak);
        let q = p("Keep my reading streak alive every day");
        assert!(!q.streak);
        assert_eq!(q.content, "Keep my reading streak alive");

        // A number right after it is the goal, whichever side of the repeat.
        let q = p("Run every day streak 30");
        assert_eq!((q.streak, q.streak_goal), (true, Some(30)));
        assert_eq!(q.content, "Run");
        let q = p("Run streak 30 every day");
        assert_eq!((q.streak, q.streak_goal), (true, Some(30)));
        assert_eq!(q.content, "Run");
        assert_eq!(q.spans.iter().map(|s| s.kind).collect::<Vec<_>>(), [SpanKind::Streak, SpanKind::Repeat]);
        // Only a whole number above zero; anything else stays in the title.
        let q = p("Run every day streak 0");
        assert_eq!((q.streak, q.streak_goal, q.content.as_str()), (true, None, "Run 0"));
        let q = p("Run every day streak 3x");
        assert_eq!((q.streak, q.streak_goal, q.content.as_str()), (true, None, "Run 3x"));
        // A number by the repeat without "streak" is just a number.
        let q = p("Run 30 every day");
        assert_eq!((q.streak, q.content.as_str()), (false, "Run 30"));

        // No repeat, no streak: it's only a word.
        let q = p("Keep my streak");
        assert!(!q.streak);
        assert_eq!(q.content, "Keep my streak");
        assert!(q.spans.is_empty());
        let q = p("Keep my streak 30");
        assert_eq!((q.streak, q.streak_goal, q.content.as_str()), (false, None, "Keep my streak 30"));
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
        assert_eq!((q.project_id, q.content.as_str(), q.new_project.as_deref()), (None, "x", Some("unknown")));
    }

    #[test]
    fn unknown_project_is_taken_as_new() {
        let q = p("Buy ink #wrok");
        assert_eq!((q.content.as_str(), q.project_id.clone(), q.new_project.as_deref()), ("Buy ink", None, Some("wrok")));
        assert_eq!(q.spans, vec![crate::quickadd::Span { start: 8, end: 13, kind: SpanKind::Project }]);
    }

    #[test]
    fn first_hash_phrase_wins() {
        let q = p("Buy #foo #work");
        assert_eq!((q.new_project.as_deref(), q.project_id.clone(), q.content.as_str()), (Some("foo"), None, "Buy #work"));
        assert_eq!(q.spans.len(), 1);
        assert_eq!((q.spans[0].start, q.spans[0].end, q.spans[0].kind), (4, 8, SpanKind::Project));
        let q = p("Buy #work #foo");
        assert_eq!((q.project_id, q.new_project, q.content.as_str()), (Some(Id::from("P-WORK")), None, "Buy #foo"));
    }

    #[test]
    fn known_project_still_wins() {
        let q = p("Buy ink #work");
        assert_eq!((q.project_id, q.new_project), (Some(Id::from("P-WORK")), None));
    }

    #[test]
    fn numbers_are_not_names() {
        let q = p("fix bug #123");
        assert_eq!((q.content.as_str(), q.new_project, q.spans.len()), ("fix bug #123", None, 0));
        let q = p("call @1 now");
        assert_eq!((q.content.as_str(), q.labels.len()), ("call @1 now", 0));
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
        for input in ["Plan today's agenda", "Mondays are hard", "Buy todays paper", "email@example.com"] {
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
        assert_eq!(q.recurrence, Some(Recurrence::EveryNWeeks { n: 2, days: vec![4], anchor: d(2026, 10, 23) }));
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
    fn word_numbers_and_ordinal_intervals() {
        assert_eq!(p("x every three days").recurrence, Some(Recurrence::EveryNDays { n: 3, anchor: today() }));
        assert_eq!(p("x every two weeks").recurrence, Some(Recurrence::EveryNWeeks { n: 2, days: vec![0], anchor: today() }));
        assert_eq!(p("x every twelve months").recurrence, Some(Recurrence::EveryNMonths { n: 12, day: 19, anchor: today() }));
        let q = p("x every second week");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::EveryNWeeks { n: 2, days: vec![0], anchor: today() })));
        let q = p("x every 3rd day");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::EveryNDays { n: 3, anchor: today() })));
        assert_eq!(p("x every third month").recurrence, Some(Recurrence::EveryNMonths { n: 3, day: 19, anchor: today() }));
    }

    #[test]
    fn nth_day_and_week_of_the_month() {
        let q = p("x every 2nd day of the month");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::MonthlyDay(2))));
        assert_eq!(p("x every 15th day of every month").recurrence, Some(Recurrence::MonthlyDay(15)));
        let q = p("x every 2nd week of the month");
        assert_eq!(q.content, "x");
        assert_eq!(q.recurrence, Some(Recurrence::MonthlyNthWeekday { nth: 2, weekday: 0 }), "its Monday");
        assert_eq!(q.date, Some(d(2026, 11, 9)));
        let q = p("x every second week of the month on fri");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::MonthlyNthWeekday { nth: 2, weekday: 4 })));
        assert_eq!(p("x every last week of the month").recurrence, Some(Recurrence::MonthlyNthWeekday { nth: -1, weekday: 0 }));
    }

    #[test]
    fn month_repeats_with_a_day() {
        let q = p("x every month on the 2nd monday");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::MonthlyNthWeekday { nth: 2, weekday: 0 })));
        let q = p("x every 3 months on the 1st");
        assert_eq!(q.content, "x");
        assert_eq!(q.recurrence, Some(Recurrence::EveryNMonths { n: 3, day: 1, anchor: d(2026, 11, 1) }));
        assert_eq!(q.date, Some(d(2026, 11, 1)));
        let q = p("x every other month on the 15th");
        assert_eq!(q.recurrence, Some(Recurrence::EveryNMonths { n: 2, day: 15, anchor: d(2026, 11, 15) }));
        assert_eq!(p("x every 1 month on the 5th").recurrence, Some(Recurrence::MonthlyDay(5)));
    }

    #[test]
    fn weekend_and_ampersand() {
        let q = p("x every weekend");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::Weekly(vec![5, 6]))));
        assert_eq!(q.date, Some(d(2026, 10, 24)));
        let q = p("x every tue & thu");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::Weekly(vec![1, 3]))));
    }

    #[test]
    fn a_start_date_after_the_repeat_belongs_to_it() {
        let q = p("x every 10 days starting nov 1");
        assert_eq!(q.content, "x");
        assert_eq!(q.recurrence, Some(Recurrence::EveryNDays { n: 10, anchor: d(2026, 11, 1) }));
        let q = p("x every day from tomorrow");
        assert_eq!((q.content.as_str(), q.date), ("x", Some(d(2026, 10, 20))));
        let q = p("x every week beginning friday");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::Weekly(vec![4]))));
        let input = "x every day starting tomorrow";
        let q = p(input);
        let got: Vec<&str> = q.spans.iter().map(|s| &input[s.start..s.end]).collect();
        assert_eq!(got, vec!["every day", "starting tomorrow"]);
        // Without a repeat, "from" is a word.
        assert_eq!(p("call from friday").content, "call from");
    }

    #[test]
    fn week_intervals_start_on_the_next_chosen_day() {
        // Today is Mon Oct 19: Tuesday this week is still ahead; Monday is today.
        let q = p("x every 3 weeks on tue");
        assert_eq!(q.date, Some(d(2026, 10, 20)));
        assert_eq!(q.recurrence, Some(Recurrence::EveryNWeeks { n: 3, days: vec![1], anchor: d(2026, 10, 20) }));
        // From a Saturday, the coming Tuesday, not the one three weeks out.
        let sat = d(2026, 10, 24);
        let q = parse("x every 3 weeks on tue", sat, &[], &[]);
        assert_eq!(q.date, Some(d(2026, 10, 27)));
        let q = parse("x every other week on mon and thu", sat, &[], &[]);
        assert_eq!(q.date, Some(d(2026, 10, 26)));
        let q = parse("x every other friday", sat, &[], &[]);
        assert_eq!(q.date, Some(d(2026, 10, 30)));
        let next = q.recurrence.unwrap().next_after(d(2026, 10, 30));
        assert_eq!(next, d(2026, 11, 13), "then every other week");
    }

    #[test]
    fn week_interval_spellings() {
        let every3 = |days: Vec<u8>, anchor| Some(Recurrence::EveryNWeeks { n: 3, days, anchor });
        let q = p("x every 3 weeks on tuesdays");
        assert_eq!((q.content.as_str(), q.recurrence), ("x", every3(vec![1], d(2026, 10, 20))));
        assert_eq!(p("x every 3 wks").recurrence, every3(vec![0], today()));
        assert_eq!(p("x every 3 wk").recurrence, every3(vec![0], today()));
        let biweekly = Some(Recurrence::EveryNWeeks { n: 2, days: vec![0], anchor: today() });
        assert_eq!(p("x every fortnight").recurrence, biweekly);
        assert_eq!(p("x biweekly").recurrence, biweekly);
        assert_eq!(p("x fortnightly").recurrence, biweekly);
        assert_eq!(p("x every mondays and fridays").recurrence, Some(Recurrence::Weekly(vec![0, 4])));
    }

    #[test]
    fn every_n_weekdays() {
        // Today is Mon Oct 19.
        let q = p("x every 3 tuesdays");
        assert_eq!(q.content, "x");
        assert_eq!(q.recurrence, Some(Recurrence::EveryNWeeks { n: 3, days: vec![1], anchor: d(2026, 10, 20) }));
        assert_eq!(q.date, Some(d(2026, 10, 20)));
        assert_eq!(p("x every two fridays").recurrence, Some(Recurrence::EveryNWeeks { n: 2, days: vec![4], anchor: d(2026, 10, 23) }));
        assert_eq!(p("x every 2 mon").recurrence, Some(Recurrence::EveryNWeeks { n: 2, days: vec![0], anchor: today() }));
        assert_eq!(
            p("x every 4 mondays and thursdays").recurrence,
            Some(Recurrence::EveryNWeeks { n: 4, days: vec![0, 3], anchor: today() })
        );
        assert_eq!(p("x every 1 friday").recurrence, Some(Recurrence::Weekly(vec![4])));
        // An ordinal is still the nth weekday of the month.
        assert_eq!(p("x every 2nd tuesday").recurrence, Some(Recurrence::MonthlyNthWeekday { nth: 2, weekday: 1 }));
    }

    #[test]
    fn every_month_name_is_its_first_unless_a_day_is_given() {
        let q = p("taxes every january");
        assert_eq!(q.content, "taxes");
        assert_eq!(q.recurrence, Some(Recurrence::Yearly { month: 1, day: 1 }));
        assert_eq!(q.date, Some(d(2027, 1, 1)));
        assert_eq!(p("x every dec").recurrence, Some(Recurrence::Yearly { month: 12, day: 1 }));
        // A given day wins, however it's written.
        assert_eq!(p("x every jan 5").recurrence, Some(Recurrence::Yearly { month: 1, day: 5 }));
        assert_eq!(p("x every march on the 3rd").recurrence, Some(Recurrence::Yearly { month: 3, day: 3 }));
        assert_eq!(p("x every march on 3rd").content, "x");
        assert_eq!(p("x every march 3").recurrence, Some(Recurrence::Yearly { month: 3, day: 3 }));
        // A time after it is still a time.
        let q = p("x every may at 9am");
        assert_eq!((q.recurrence, q.time.is_some()), (Some(Recurrence::Yearly { month: 5, day: 1 }), true));
    }

    #[test]
    fn a_repeat_read_on_its_own() {
        use super::parse_repeat;
        let sat = d(2026, 10, 24);
        // Anchored on the given start, not today.
        let q = parse_repeat("every week", today(), sat).unwrap();
        assert_eq!((q.recurrence, q.date), (Some(Recurrence::Weekly(vec![5])), Some(sat)));
        let q = parse_repeat("every 3 tuesdays", today(), sat).unwrap();
        assert_eq!(q.date, Some(d(2026, 10, 27)));
        // "every" may be left off.
        let q = parse_repeat("3 tuesdays", today(), sat).unwrap();
        assert_eq!(q.recurrence, Some(Recurrence::EveryNWeeks { n: 3, days: vec![1], anchor: d(2026, 10, 27) }));
        assert_eq!(parse_repeat("other week", today(), sat).unwrap().recurrence.map(|r| r.describe()), Some("every other week on Sat".into()));
        assert!(parse_repeat("daily", today(), sat).is_some());
        // A start, an end and a time come along; relative dates count from today.
        let q = parse_repeat("every day starting tomorrow until nov 30 at 9am", today(), sat).unwrap();
        assert_eq!((q.date, q.until), (Some(d(2026, 10, 20)), Some(d(2026, 11, 30))));
        assert!(q.time.is_some());
        // Anything else, or no repeat, isn't one.
        for text in ["", "tomorrow", "every day p1", "every day call mom", "every 5th monday", "buy milk"] {
            assert!(parse_repeat(text, today(), sat).is_none(), "{text}");
        }
    }

    #[test]
    fn a_repeat_can_end() {
        let input = "x every day for 10 days";
        let q = p(input);
        assert_eq!((q.content.as_str(), q.recurrence), ("x", Some(Recurrence::Daily)));
        assert_eq!(q.until, Some(d(2026, 10, 28)), "ten days, today included");
        let got: Vec<&str> = q.spans.iter().map(|s| &input[s.start..s.end]).collect();
        assert_eq!(got, vec!["every day", "for 10 days"]);
        assert_eq!(p("x every mon for two weeks").until, Some(d(2026, 11, 1)));
        assert_eq!(p("x every 15th for 3 months").until, Some(d(2027, 2, 14)), "from the first occurrence");
        let q = p("x every day until nov 30");
        assert_eq!((q.content.as_str(), q.until), ("x", Some(d(2026, 11, 30))));
        assert_eq!(q.date, Some(today()), "the end isn't the due date");
        let q = p("x every 10 days starting nov 1 until dec 31");
        assert_eq!((q.date, q.until), (Some(d(2026, 11, 1)), Some(d(2026, 12, 31))));
        // Without a repeat they're words.
        let q = p("walk for 10 days");
        assert_eq!((q.content.as_str(), q.until), ("walk for 10 days", None));
        let q = p("wait until friday");
        assert_eq!((q.content.as_str(), q.until, q.date), ("wait until", None, Some(d(2026, 10, 23))));
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
        assert_eq!(parse_time("9a"), Some(t(9, 0)));
        assert_eq!(parse_time("10p"), Some(t(22, 0)));
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


    #[test]
    fn kept_text_is_spaced_like_parse() {
        // Kept as text with no other phrase: whitespace collapses, as in parse.
        assert_eq!(pk("call  mom   today", &[(12, 17)]).content, "call mom today");
        // With another phrase recognized too.
        let q = pk("call  mom today   tomorrow", &[(10, 15)]);
        assert_eq!(q.content, "call mom today");
        assert_eq!(q.date, Some(d(2026, 10, 20)));
    }
}
