//! Todoist-style filter queries: parse once, evaluate per task.
//!
//! Precedence (loosest first): `,` separates result groups, then `|`, then
//! `&`, then `!`; parentheses group. Terms are case-insensitive.
//!
//! Between operators is a piece. A piece that starts with a colon keyword
//! (`due before:`, `search:` …) is that one term. Otherwise it is read word
//! by word like quick add: keywords, priorities, `{deadline}`, names, repeat,
//! date and time phrases each make a term, the words left over make one text
//! term, and the piece is all of them ANDed. `"…"` is always text.

use std::collections::HashMap;
use std::fmt;

use chrono::{Duration, NaiveDate, NaiveTime};

use crate::ids::Id;
use crate::model::{Priority, Task};
use crate::phrases::{
    is_name, match_date, match_deadline, match_name, match_repeat, match_time, repeat_matches, repeat_word, tokens,
    RepeatSpec, Token as Word,
};
use crate::quickadd::{Span, SpanKind};
use crate::views::{due_date_of, due_time};

/// What a query is evaluated against besides the task itself.
#[derive(Clone, Debug)]
pub struct FilterContext {
    pub today: NaiveDate,
    /// Project id → name.
    pub projects: HashMap<Id, String>,
    /// Section id → name.
    pub sections: HashMap<Id, String>,
}

/// One comma-separated part of a filter and the open tasks it matched.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct FilterGroup {
    /// The group's own query text (e.g. "overdue | @home").
    pub query: String,
    pub tasks: Vec<Task>,
}

/// The text of each comma-separated group, trimmed (labels for result lists).
pub fn group_texts(query: &str) -> Vec<String> {
    query.split(',').map(|s| s.trim().to_string()).collect()
}

/// The names that exist, so a multi-word name reads whole (`/Next Up`).
#[derive(Clone, Debug, Default)]
pub struct FilterNames {
    pub projects: Vec<String>,
    pub sections: Vec<String>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterError {
    pub message: String,
    /// The byte range of the query the error is about, when known.
    pub at: Option<(usize, usize)>,
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FilterError {}

fn err<T>(msg: impl Into<String>) -> Result<T, FilterError> {
    Err(FilterError { message: msg.into(), at: None })
}

/// A date relative to today, or a fixed date.
#[derive(Debug, Clone, PartialEq)]
pub enum DateRef {
    Offset(i64),
    On(NaiveDate),
}

impl DateRef {
    fn resolve(&self, today: NaiveDate) -> NaiveDate {
        match self {
            DateRef::Offset(n) => today + Duration::days(*n),
            DateRef::On(d) => *d,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    All,
    Today,
    Tomorrow,
    Overdue,
    NoDate,
    NoDeadline,
    Recurring,
    NoLabels,
    Subtask,
    Priority(Priority),
    /// Lower-cased names.
    Project(String),
    Section(String),
    Label(String),
    /// `search: …`.
    Search(String),
    DueBefore(DateRef),
    DueAfter(DateRef),
    DueOn(DateRef),
    DeadlineBefore(DateRef),
    DeadlineAfter(DateRef),
    DeadlineOn(DateRef),
    /// today ≤ due < today + n.
    NextDays(i64),
    /// Plain or quoted words: the task contains them all (like search).
    Text(String),
    /// A repeat phrase: the task's rule has that kind and interval.
    Repeat(RepeatSpec),
    /// A date with a time ("tomorrow at 5pm"; a time alone is today).
    DueAt(NaiveDate, NaiveTime),
    Streak,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Term(Term),
}

/// A stretch of a piece: plain words, or a quote's inside.
#[derive(Debug, Clone, PartialEq)]
struct Part {
    start: usize,
    end: usize,
    quoted: bool,
}

/// The text between operators: its parts and byte range (quotes included).
#[derive(Debug, Clone, PartialEq)]
struct Piece {
    start: usize,
    end: usize,
    parts: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    And,
    Or,
    Not,
    Open,
    Close,
    Comma,
    Piece(Piece),
}

fn closing_quote(c: char) -> bool {
    matches!(c, '"' | '\u{201D}')
}

fn tokenize(query: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut piece: Option<Piece> = None;
    let mut plain: Option<usize> = None;
    fn add(piece: &mut Option<Piece>, part: Part, end: usize) {
        let p = piece.get_or_insert(Piece { start: part.start, end, parts: Vec::new() });
        p.end = end;
        p.parts.push(part);
    }
    // Ends the plain run at `upto`, adding its trimmed text as a part.
    let flush_plain = |piece: &mut Option<Piece>, plain: &mut Option<usize>, upto: usize| {
        if let Some(s) = plain.take() {
            let text = &query[s..upto];
            let lead = text.len() - text.trim_start().len();
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let (a, b) = (s + lead, s + lead + trimmed.len());
                add(piece, Part { start: a, end: b, quoted: false }, b);
            }
        }
    };
    let mut i = 0;
    while let Some(c) = query[i..].chars().next() {
        let next = i + c.len_utf8();
        let op = match c {
            '&' => Some(Token::And),
            '|' => Some(Token::Or),
            '!' => Some(Token::Not),
            '(' => Some(Token::Open),
            ')' => Some(Token::Close),
            ',' => Some(Token::Comma),
            _ => None,
        };
        if let Some(op) = op {
            flush_plain(&mut piece, &mut plain, i);
            if let Some(p) = piece.take() {
                out.push(Token::Piece(p));
            }
            out.push(op);
            i = next;
        } else if c == '"' || c == '\u{201C}' {
            // A quote runs to its closing quote, or to the end of the query.
            flush_plain(&mut piece, &mut plain, i);
            let (inner_end, after) = match query[next..].char_indices().find(|(_, c)| closing_quote(*c)) {
                Some((k, q)) => (next + k, next + k + q.len_utf8()),
                None => (query.len(), query.len()),
            };
            let inner = &query[next..inner_end];
            let trimmed = inner.trim();
            if !trimmed.is_empty() {
                let a = next + (inner.len() - inner.trim_start().len());
                let quote = Part { start: a, end: a + trimmed.len(), quoted: true };
                let p = piece.get_or_insert(Piece { start: i, end: after, parts: Vec::new() });
                p.end = after;
                p.parts.push(quote);
            }
            i = after;
        } else {
            if plain.is_none() && !c.is_whitespace() {
                plain = Some(i);
            }
            i = next;
        }
    }
    flush_plain(&mut piece, &mut plain, query.len());
    if let Some(p) = piece.take() {
        out.push(Token::Piece(p));
    }
    out
}

/// `N days`, `N day`, `+N days`, `-N days` → N (sign kept).
fn parse_days(s: &str) -> Option<i64> {
    let s = s.trim();
    let num = s.strip_suffix("days").or_else(|| s.strip_suffix("day"))?.trim();
    let num = num.strip_prefix('+').unwrap_or(num);
    num.parse::<i64>().ok()
}

fn parse_date(raw: &str, today: NaiveDate) -> Result<DateRef, FilterError> {
    let s = raw.trim().to_lowercase();
    match s.as_str() {
        "" => err("A date is missing after the colon"),
        "today" => Ok(DateRef::Offset(0)),
        "tomorrow" => Ok(DateRef::Offset(1)),
        "yesterday" => Ok(DateRef::Offset(-1)),
        _ => {
            if let Some(n) = parse_days(&s) {
                return Ok(DateRef::Offset(n));
            }
            if let Ok(d) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                return Ok(DateRef::On(d));
            }
            // Quick add's date phrases, when they are the whole date.
            let words = tokens(&s);
            match match_date(&words, 0, today) {
                Some((n, d)) if n == words.len() => Ok(DateRef::On(d)),
                _ => err(format!("Unrecognized date “{}” (try today, +3 days, or 2026-10-31)", raw.trim())),
            }
        }
    }
}

/// The colon keywords, each read with the rest of its piece.
const COLON_TERMS: [&str; 7] =
    ["due before:", "due after:", "due:", "deadline before:", "deadline after:", "deadline:", "search:"];

fn colon_term(raw: &str, today: NaiveDate) -> Result<Term, FilterError> {
    let lower = raw.to_lowercase();
    let t = lower.as_str();
    if let Some(rest) = t.strip_prefix("search:") {
        let words = rest.trim();
        return if words.is_empty() { err(format!("“{}” needs a search name", raw.trim())) } else { Ok(Term::Search(words.to_string())) };
    }
    let date = |rest: &str| parse_date(rest, today);
    Ok(if let Some(rest) = t.strip_prefix("due before:") {
        Term::DueBefore(date(rest)?)
    } else if let Some(rest) = t.strip_prefix("due after:") {
        Term::DueAfter(date(rest)?)
    } else if let Some(rest) = t.strip_prefix("due:") {
        Term::DueOn(date(rest)?)
    } else if let Some(rest) = t.strip_prefix("deadline before:") {
        Term::DeadlineBefore(date(rest)?)
    } else if let Some(rest) = t.strip_prefix("deadline after:") {
        Term::DeadlineAfter(date(rest)?)
    } else if let Some(rest) = t.strip_prefix("deadline:") {
        Term::DeadlineOn(date(rest)?)
    } else {
        return err(format!("Unknown filter term “{}”", raw.trim()));
    })
}

/// `N days` / `Ndays` at word `j` (N ≥ 1): (N, words).
fn days_at(t: &[Word], j: usize) -> Option<(i64, usize)> {
    let w = |k: usize| t.get(j + k).map(|x| x.norm.as_str()).unwrap_or("");
    let count = |s: &str| s.strip_prefix('+').unwrap_or(s).parse::<i64>().ok();
    let found = if matches!(w(1), "day" | "days") {
        count(w(0)).map(|n| (n, 2))
    } else {
        w(0).strip_suffix("days").or_else(|| w(0).strip_suffix("day")).and_then(count).map(|n| (n, 1))
    };
    found.filter(|(n, _)| *n >= 1)
}

/// A filter keyword at word `i`: (words, term).
fn keyword(t: &[Word], i: usize) -> Option<(usize, Term)> {
    let w = |k: usize| t.get(i + k).map(|x| x.norm.as_str()).unwrap_or("");
    match (w(0), w(1)) {
        ("no", "deadline") => return Some((2, Term::NoDeadline)),
        ("no", "labels") => return Some((2, Term::NoLabels)),
        ("no", "date") => return Some((2, Term::NoDate)),
        _ => {}
    }
    if w(0) == "next" {
        if let Some((n, used)) = days_at(t, i + 1) {
            return Some((1 + used, Term::NextDays(n)));
        }
    }
    if let Some((n, used)) = days_at(t, i) {
        return Some((used, Term::NextDays(n)));
    }
    let term = match w(0) {
        "overdue" => Term::Overdue,
        "recurring" => Term::Recurring,
        "subtask" => Term::Subtask,
        "all" => Term::All,
        "streak" => Term::Streak,
        _ => return None,
    };
    Some((1, term))
}

/// A `#` / `##`, `/` or `@` name at word `i`: the longest known name, else
/// the one word (a name that matches nothing).
fn name_at(t: &[Word], i: usize, names: &FilterNames) -> Option<(usize, Term, SpanKind)> {
    let raw = t[i].raw.as_str();
    let (sigil, list, kind) = match raw.chars().next()? {
        '#' => ("#", &names.projects, SpanKind::Project),
        '/' => ("/", &names.sections, SpanKind::Section),
        '@' => ("@", &names.labels, SpanKind::Label),
        _ => return None,
    };
    let make = |name: String| match kind {
        SpanKind::Project => Term::Project(name),
        SpanKind::Section => Term::Section(name),
        _ => Term::Label(name),
    };
    let known: Vec<&str> = list.iter().map(String::as_str).collect();
    if let Some((n, pos)) = match_name(t, i, sigil, &known) {
        return Some((n, make(list[pos].to_lowercase()), kind));
    }
    let rest = raw.strip_prefix(sigil)?;
    let rest = if sigil == "#" { rest.strip_prefix('#').unwrap_or(rest) } else { rest };
    let name = rest.trim_end_matches([',', '.', ';', '!', '?']);
    is_name(name).then(|| (1, make(name.to_lowercase()), kind))
}

/// A bare date phrase: today and tomorrow keep their own terms.
fn date_term(d: NaiveDate, today: NaiveDate) -> Term {
    if d == today {
        Term::Today
    } else if d == today + Duration::days(1) {
        Term::Tomorrow
    } else {
        Term::DueOn(DateRef::On(d))
    }
}

/// What a piece reads as: its terms (by position) and their spans.
struct Read<'a> {
    terms: Vec<(usize, Term)>,
    spans: Vec<Span>,
    text: Vec<&'a str>,
    text_at: Option<usize>,
}

impl<'a> Read<'a> {
    fn text(&mut self, at: usize, words: &'a str) {
        self.text_at.get_or_insert(at);
        self.text.push(words);
    }

    fn term(&mut self, t: &[Word], i: usize, n: usize, term: Term, kind: SpanKind) {
        self.terms.push((t[i].start, term));
        self.spans.push(Span { start: t[i].start, end: t[i + n - 1].end, kind });
    }
}

/// Reads one run of plain words (`t` has byte ranges into `query`).
fn read_words<'a>(query: &'a str, t: &[Word], today: NaiveDate, names: &FilterNames, r: &mut Read<'a>) {
    let mut i = 0;
    while i < t.len() {
        let norm = t[i].norm.as_str();
        if let Some((n, term)) = keyword(t, i) {
            r.term(t, i, n, term, SpanKind::Keyword);
            i += n;
            continue;
        }
        let priority = match norm {
            "p1" => Some(Priority::P1),
            "p2" => Some(Priority::P2),
            "p3" => Some(Priority::P3),
            "p4" => Some(Priority::None),
            _ => None,
        };
        if let Some(p) = priority {
            r.term(t, i, 1, Term::Priority(p), SpanKind::Priority);
            i += 1;
            continue;
        }
        if t[i].raw.starts_with('{') {
            if let Some((n, d)) = match_deadline(query, t, i, today) {
                r.term(t, i, n, Term::DeadlineOn(DateRef::On(d)), SpanKind::Deadline);
                i += n;
                continue;
            }
        }
        if let Some((n, term, kind)) = name_at(t, i, names) {
            r.term(t, i, n, term, kind);
            i += n;
            continue;
        }
        if let Some((n, spec)) = match_repeat(t, i) {
            r.term(t, i, n, Term::Repeat(spec), SpanKind::Repeat);
            i += n;
            continue;
        }
        // An "every ..." phrase it can't read stays text as a whole.
        if norm == "every" {
            let from = i;
            i += 1;
            while i < t.len() && repeat_word(&t[i].norm) {
                i += 1;
            }
            r.text(t[from].start, &query[t[from].start..t[i - 1].end]);
            continue;
        }
        if let Some((n, d)) = match_date(t, i, today) {
            r.spans.push(Span { start: t[i].start, end: t[i + n - 1].end, kind: SpanKind::Date });
            match match_time(t, i + n) {
                Some((m, time)) => {
                    r.terms.push((t[i].start, Term::DueAt(d, time)));
                    r.spans.push(Span { start: t[i + n].start, end: t[i + n + m - 1].end, kind: SpanKind::Time });
                    i += n + m;
                }
                None => {
                    r.terms.push((t[i].start, date_term(d, today)));
                    i += n;
                }
            }
            continue;
        }
        if let Some((n, time)) = match_time(t, i) {
            r.term(t, i, n, Term::DueAt(today, time), SpanKind::Time);
            i += n;
            continue;
        }
        r.text(t[i].start, &query[t[i].start..t[i].end]);
        i += 1;
    }
}

/// A piece's terms, in order, and their spans.
fn read_piece(query: &str, p: &Piece, today: NaiveDate, names: &FilterNames) -> Result<(Vec<Term>, Vec<Span>), FilterError> {
    if let Some(first) = p.parts.first().filter(|f| !f.quoted) {
        let lower = query[first.start..first.end].to_lowercase();
        if COLON_TERMS.iter().any(|k| lower.starts_with(k)) {
            let whole: Vec<&str> = p.parts.iter().map(|x| &query[x.start..x.end]).collect();
            let term = colon_term(&whole.join(" "), today).map_err(|e| FilterError { at: Some((p.start, p.end)), ..e })?;
            return Ok((vec![term], vec![Span { start: p.start, end: p.end, kind: SpanKind::Keyword }]));
        }
    }
    let mut r = Read { terms: Vec::new(), spans: Vec::new(), text: Vec::new(), text_at: None };
    for part in &p.parts {
        if part.quoted {
            r.text(part.start, &query[part.start..part.end]);
            continue;
        }
        let mut words = tokens(&query[part.start..part.end]);
        for w in &mut words {
            w.start += part.start;
            w.end += part.start;
        }
        read_words(query, &words, today, names, &mut r);
    }
    if let Some(at) = r.text_at {
        r.terms.push((at, Term::Text(r.text.join(" "))));
    }
    r.terms.sort_by_key(|(at, _)| *at);
    Ok((r.terms.into_iter().map(|(_, t)| t).collect(), r.spans))
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    query: &'a str,
    today: NaiveDate,
    names: &'a FilterNames,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn or(&mut self) -> Result<Expr, FilterError> {
        let mut left = self.and()?;
        while self.peek() == Some(&Token::Or) {
            self.pos += 1;
            left = Expr::Or(Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, FilterError> {
        let mut left = self.unary()?;
        while self.peek() == Some(&Token::And) {
            self.pos += 1;
            left = Expr::And(Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, FilterError> {
        if self.peek() == Some(&Token::Not) {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, FilterError> {
        match self.next() {
            Some(Token::Open) => {
                let inner = self.or()?;
                match self.next() {
                    Some(Token::Close) => Ok(inner),
                    _ => err("Missing a closing parenthesis"),
                }
            }
            Some(Token::Piece(p)) => {
                let (terms, _) = read_piece(self.query, &p, self.today, self.names)?;
                let mut terms = terms.into_iter().map(Expr::Term);
                let first = terms.next().map_or_else(|| err("Expected a filter term"), Ok)?;
                Ok(terms.fold(first, |a, b| Expr::And(Box::new(a), Box::new(b))))
            }
            Some(Token::Close) => err("Unexpected “)”"),
            Some(Token::Comma) | None => err("Expected a filter term"),
            Some(op) => err(format!("Expected a filter term before {}", op_name(&op))),
        }
    }
}

fn op_name(t: &Token) -> &'static str {
    match t {
        Token::And => "“&”",
        Token::Or => "“|”",
        Token::Not => "“!”",
        _ => "an operator",
    }
}

/// Parses a query into its comma-separated groups. Relative phrases resolve
/// against `today`, so parse on every run.
pub fn parse(query: &str, today: NaiveDate, names: &FilterNames) -> Result<Vec<Expr>, FilterError> {
    let mut p = Parser { tokens: tokenize(query), pos: 0, query, today, names };
    if p.tokens.is_empty() {
        return err("The query is empty");
    }
    let mut groups = vec![p.or()?];
    loop {
        match p.next() {
            None => return Ok(groups),
            Some(Token::Comma) => groups.push(p.or()?),
            Some(Token::Close) => return err("Unexpected “)”"),
            Some(_) => return err("Expected “&”, “|” or “,” between terms"),
        }
    }
}

/// Every recognized term's byte range and kind, for highlighting. Best
/// effort: a piece that doesn't read (or an operator error) costs only its
/// own spans.
pub fn spans(query: &str, today: NaiveDate, names: &FilterNames) -> Vec<Span> {
    let mut out: Vec<Span> = tokenize(query)
        .iter()
        .filter_map(|t| match t {
            Token::Piece(p) => read_piece(query, p, today, names).ok().map(|(_, s)| s),
            _ => None,
        })
        .flatten()
        .collect();
    out.sort_by_key(|s| s.start);
    out
}

/// Whether `task` satisfies `expr`.
pub fn matches(expr: &Expr, task: &Task, ctx: &FilterContext) -> bool {
    match expr {
        Expr::And(a, b) => matches(a, task, ctx) && matches(b, task, ctx),
        Expr::Or(a, b) => matches(a, task, ctx) || matches(b, task, ctx),
        Expr::Not(a) => !matches(a, task, ctx),
        Expr::Term(term) => term_matches(term, task, ctx),
    }
}

fn term_matches(term: &Term, task: &Task, ctx: &FilterContext) -> bool {
    let due = due_date_of(task);
    let today = ctx.today;
    let deadline = task.deadline.as_deref().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let lower_eq = |a: &str, b: &str| a.to_lowercase() == b;
    match term {
        Term::All => true,
        Term::Today => due == Some(today),
        Term::Tomorrow => due == Some(today + Duration::days(1)),
        Term::Overdue => due.is_some_and(|d| d < today),
        Term::NoDate => due.is_none(),
        Term::NoDeadline => deadline.is_none(),
        Term::Recurring => task.due.as_ref().is_some_and(|d| d.is_recurring),
        Term::NoLabels => task.labels.is_empty(),
        Term::Subtask => task.parent_id.is_some(),
        Term::Priority(p) => task.priority == *p,
        Term::Project(name) => ctx.projects.get(&task.project_id).is_some_and(|n| lower_eq(n, name)),
        Term::Section(name) => task
            .section_id
            .as_ref()
            .and_then(|s| ctx.sections.get(s))
            .is_some_and(|n| lower_eq(n, name)),
        Term::Label(name) => task.labels.iter().any(|l| lower_eq(l, name)),
        Term::Search(text) | Term::Text(text) => crate::search::matches(task, &crate::search::words(text)),
        Term::DueBefore(r) => due.is_some_and(|d| d < r.resolve(today)),
        Term::DueAfter(r) => due.is_some_and(|d| d > r.resolve(today)),
        Term::DueOn(r) => due == Some(r.resolve(today)),
        Term::DeadlineBefore(r) => deadline.is_some_and(|d| d < r.resolve(today)),
        Term::DeadlineAfter(r) => deadline.is_some_and(|d| d > r.resolve(today)),
        Term::DeadlineOn(r) => deadline == Some(r.resolve(today)),
        Term::NextDays(n) => due.is_some_and(|d| d >= today && d < today + Duration::days(*n)),
        Term::Repeat(spec) => task
            .due
            .as_ref()
            .and_then(|d| d.recurrence.as_ref())
            .is_some_and(|rule| repeat_matches(spec, rule)),
        Term::DueAt(d, t) => due == Some(*d) && due_time(task) == Some(*t),
        Term::Streak => task.streak_enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::{matches, parse, spans, DateRef, Expr, FilterContext, FilterNames, Term};
    use crate::ids::Id;
    use crate::model::{Due, Priority, Task};
    use crate::phrases::RepeatSpec;
    use crate::quickadd::{Span, SpanKind};
    use crate::recurrence::Recurrence;
    use chrono::{NaiveDate, NaiveTime};
    use std::collections::HashMap;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn ctx() -> FilterContext {
        let mut projects = HashMap::new();
        projects.insert(Id::from("P-WORK"), "Work".to_string());
        projects.insert(Id::from("P-HOME"), "Home".to_string());
        let mut sections = HashMap::new();
        sections.insert(Id::from("S-NEXT"), "Next Up".to_string());
        FilterContext { today: d(2026, 10, 19), projects, sections }
    }

    fn names() -> FilterNames {
        FilterNames {
            projects: vec!["Work".into(), "Home".into()],
            sections: vec!["Next Up".into()],
            labels: vec!["home".into()],
        }
    }

    /// The single group `query` parses to.
    fn one(query: &str) -> Expr {
        let mut groups = parse(query, ctx().today, &names()).unwrap_or_else(|e| panic!("{query:?} should parse: {e}"));
        assert_eq!(groups.len(), 1, "{query:?}");
        groups.remove(0)
    }

    fn term(t: Term) -> Expr {
        Expr::Term(t)
    }

    fn and(a: Expr, b: Expr) -> Expr {
        Expr::And(Box::new(a), Box::new(b))
    }

    fn task(name: &str) -> Task {
        Task::new(name, Id::from("P-WORK"))
    }

    fn due(mut t: Task, date: &str) -> Task {
        t.due = Some(Due {
            date: date.to_string(),
            timezone: None,
            string: date.to_string(),
            lang: "en".into(),
            is_recurring: false,
            recurrence: None,
            until: None,
            moved_from: None,
        });
        t
    }

    /// Does `query` (single group) match `t`?
    fn m(query: &str, t: &Task) -> bool {
        matches(&one(query), t, &ctx())
    }

    #[test]
    fn invalid_queries_error() {
        let today = ctx().today;
        for q in ["", "   ", "&", "(today", "today)", "& today", "today &", "today | ", "!", "due before:", "due before: someday", "due: blursday"] {
            assert!(parse(q, today, &names()).is_err(), "{q:?} should be rejected");
        }
        for q in ["foo", "p5", "3 dayz", "#", "@"] {
            assert!(parse(q, today, &names()).is_ok(), "{q:?} should parse as text");
        }
        assert_eq!(one("foo"), term(Term::Text("foo".into())));
        assert_eq!(one("3 dayz"), term(Term::Text("3 dayz".into())));
    }

    #[test]
    fn colon_term_error_has_its_range() {
        let e = parse("today & due: blursday", ctx().today, &names()).unwrap_err();
        assert_eq!(e.at, Some((8, 21)));
        assert!(e.to_string().contains("blursday"), "{e}");
    }

    #[test]
    fn multi_word_names_read_whole() {
        assert_eq!(one("/Next Up & tomorrow"), and(term(Term::Section("next up".into())), term(Term::Tomorrow)));
        assert_eq!(one("#Gone today"), and(term(Term::Project("gone".into())), term(Term::Today)));
    }

    #[test]
    fn words_and_phrases_are_anded() {
        let q = "buy milk tomorrow";
        assert!(m(q, &due(task("Buy milk"), "2026-10-20")));
        assert!(!m(q, &due(task("Buy milk"), "2026-10-21")));
        assert!(!m(q, &due(task("Buy bread"), "2026-10-20")));
        assert_eq!(one(q), and(term(Term::Text("buy milk".into())), term(Term::Tomorrow)));
    }

    #[test]
    fn quoted_word_is_text() {
        assert!(m("\"tomorrow\"", &task("Call mom tomorrow")));
        assert!(!m("\"tomorrow\"", &due(task(""), "2026-10-20")));
    }

    #[test]
    fn quotes_keep_operators() {
        assert_eq!(one("\"a & b\""), term(Term::Text("a & b".into())));
    }

    #[test]
    fn unclosed_and_empty_quotes() {
        assert_eq!(one("\"tomorrow"), term(Term::Text("tomorrow".into())));
        assert_eq!(one("buy \"\""), term(Term::Text("buy".into())));
        let e = parse("\"\"", ctx().today, &names()).unwrap_err();
        assert_eq!(e.message, "The query is empty");
    }

    #[test]
    fn quick_add_dates_in_filters() {
        assert_eq!(one("in 3 days"), term(Term::DueOn(DateRef::On(d(2026, 10, 22)))));
        // Mon 10-19: Friday 10-23 is this week, so "next friday" is 10-30.
        assert_eq!(one("next friday"), term(Term::DueOn(DateRef::On(d(2026, 10, 30)))));
        assert_eq!(one("nov 30"), term(Term::DueOn(DateRef::On(d(2026, 11, 30)))));
    }

    #[test]
    fn repeat_phrase_term() {
        assert_eq!(one("every 2 weeks"), term(Term::Repeat(RepeatSpec::EveryNWeeks(2, None))));
    }

    #[test]
    fn unreadable_every_stays_text() {
        assert_eq!(one("every 5th monday"), term(Term::Text("every 5th monday".into())));
    }

    #[test]
    fn date_and_time() {
        let five = NaiveTime::from_hms_opt(17, 0, 0).unwrap();
        assert_eq!(one("tomorrow at 5pm"), term(Term::DueAt(d(2026, 10, 20), five)));
        assert_eq!(one("5pm"), term(Term::DueAt(d(2026, 10, 19), five)));
    }

    #[test]
    fn old_queries_still_parse() {
        for q in [
            "today",
            "overdue | @home",
            "#Work & p1",
            "(today | overdue) & !@x",
            "due before: +3 days",
            "deadline: 2026-10-31",
            "7 days",
            "next 7 days",
            "no date, overdue",
            "search: Milk",
            "/Next Up",
        ] {
            assert!(parse(q, ctx().today, &names()).is_ok(), "{q:?} should parse");
        }
    }

    #[test]
    fn next_seven_days_is_a_range() {
        assert_eq!(one("next 7 days"), term(Term::NextDays(7)));
        assert_eq!(one("7 days"), term(Term::NextDays(7)));
    }

    #[test]
    fn spans_for_highlighting() {
        let today = ctx().today;
        let span = |start, end, kind| Span { start, end, kind };
        assert_eq!(
            spans("buy milk tomorrow & #Work", today, &names()),
            vec![span(9, 17, SpanKind::Date), span(20, 25, SpanKind::Project)]
        );
        assert_eq!(spans("\"tomorrow\" p1", today, &names()), vec![span(11, 13, SpanKind::Priority)]);
        assert_eq!(
            spans("due before: +3 days | overdue & streak", today, &names()),
            vec![span(0, 19, SpanKind::Keyword), span(22, 29, SpanKind::Keyword), span(32, 38, SpanKind::Keyword)]
        );
        // Best effort: the bad piece has no span, the good ones still do.
        assert_eq!(spans("p1 & (", today, &names()), vec![span(0, 2, SpanKind::Priority)]);
    }

    #[test]
    fn new_terms_match() {
        let mut daily = due(task("water"), "2026-10-19T17:00:00");
        if let Some(d) = daily.due.as_mut() {
            d.is_recurring = true;
            d.recurrence = Some(Recurrence::EveryNWeeks { n: 2, days: vec![0], anchor: d_anchor() });
        }
        assert!(m("every 2 weeks", &daily) && !m("every 3 weeks", &daily));
        assert!(m("5pm", &daily) && !m("6pm", &daily) && !m("tomorrow 5pm", &daily));
        assert!(!m("streak", &daily));
        daily.streak_enabled = true;
        assert!(m("streak", &daily));
    }

    fn d_anchor() -> NaiveDate {
        d(2026, 10, 19)
    }

    #[test]
    fn date_terms() {
        let t_today = due(task("a"), "2026-10-19");
        let t_tomorrow = due(task("b"), "2026-10-20T09:00:00");
        let t_overdue = due(task("c"), "2026-10-01");
        let undated = task("d");
        assert!(m("today", &t_today) && !m("today", &t_tomorrow) && !m("today", &undated));
        assert!(m("tomorrow", &t_tomorrow));
        assert!(m("overdue", &t_overdue) && !m("overdue", &t_today));
        assert!(m("no date", &undated) && !m("no date", &t_today));
        assert!(m("all", &undated));
    }

    #[test]
    fn priorities() {
        let mut t = task("x");
        t.priority = Priority::P1;
        assert!(m("p1", &t) && !m("p2", &t));
        let plain = task("y");
        assert!(m("p4", &plain), "no priority is p4");
    }

    #[test]
    fn project_section_label_case_insensitive() {
        let mut t = task("x");
        t.section_id = Some(Id::from("S-NEXT"));
        t.labels = vec!["Home".into(), "errands".into()];
        assert!(m("#work", &t) && m("##WORK", &t) && !m("#home", &t));
        assert!(m("/next up", &t) && !m("/later", &t));
        assert!(m("@home", &t) && m("@ERRANDS", &t) && !m("@office", &t));
    }

    #[test]
    fn precedence_and_binds_tighter_than_or() {
        let mut overdue_low = due(task("x"), "2026-10-01");
        overdue_low.priority = Priority::None;
        let today_low = due(task("y"), "2026-10-19");
        // today | (overdue & p1): the overdue low-priority task must NOT match.
        assert!(!m("today | overdue & p1", &overdue_low));
        assert!(m("today | overdue & p1", &today_low));
        // Parentheses change it: (today | overdue) & p4 matches both.
        assert!(m("(today | overdue) & p4", &overdue_low));
    }

    #[test]
    fn not_and_parentheses() {
        let t = due(task("x"), "2026-10-19");
        assert!(m("!no date", &t));
        assert!(!m("!today", &t));
        assert!(m("!(overdue | no date)", &t));
        assert!(m("!!today", &t));
    }

    #[test]
    fn comma_makes_groups() {
        let groups = parse("today, overdue,  @home", ctx().today, &names()).unwrap();
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn deadline_terms() {
        let mut t = task("report");
        t.deadline = Some("2026-10-21".into());
        let none = task("free");
        assert!(m("deadline: +2 days", &t) && !m("deadline: today", &t));
        assert!(m("deadline before: +3 days", &t) && !m("deadline before: +2 days", &t), "strict");
        assert!(m("deadline after: today", &t) && !m("deadline after: 2026-10-21", &t));
        assert!(m("no deadline", &none) && !m("no deadline", &t));
        assert!(!m("deadline before: +30 days", &none), "no deadline never matches a comparison");
        assert!(m("deadline before: +7 days & p4", &t));
    }

    #[test]
    fn search_matches_content_and_description() {
        let mut t = task("Buy Milk");
        t.description = "from the corner shop".into();
        assert!(m("search: milk", &t) && m("search: corner", &t) && !m("search: bread", &t));
        assert!(m("search: milk shop", &t), "every word, anywhere (same matcher as the Search view)");
    }

    #[test]
    fn due_before_after_on() {
        let t = due(task("x"), "2026-10-22");
        assert!(m("due before: +7 days", &t));
        assert!(!m("due before: 2026-10-22", &t), "before is strict");
        assert!(m("due after: tomorrow", &t));
        assert!(!m("due after: 2026-10-22", &t), "after is strict");
        assert!(m("due: 2026-10-22", &t) && m("due: +3 days", &t));
        assert!(!m("due before: today", &task("undated")), "undated never matches a date comparison");
        assert!(m("due after: -1 days", &due(task("y"), "2026-10-19")));
        assert!(m("due before: yesterday", &due(task("z"), "2026-10-17")));
    }

    #[test]
    fn next_n_days() {
        let in_six = due(task("x"), "2026-10-25");
        let in_seven = due(task("y"), "2026-10-26");
        let past = due(task("z"), "2026-10-18");
        assert!(m("7 days", &in_six) && !m("7 days", &in_seven) && !m("7 days", &past));
        assert!(m("next 7 days", &in_six));
        assert!(m("1 days", &due(task("t"), "2026-10-19")), "today counts");
    }

    #[test]
    fn recurring_no_labels_subtask() {
        let mut r = due(task("x"), "2026-10-19");
        if let Some(d) = r.due.as_mut() {
            d.is_recurring = true;
            d.recurrence = Some(Recurrence::Daily);
        }
        assert!(m("recurring", &r) && !m("recurring", &task("plain")));
        let mut labelled = task("l");
        labelled.labels = vec!["home".into()];
        assert!(m("no labels", &task("n")) && !m("no labels", &labelled));
        let mut child = task("c");
        child.parent_id = Some(Id::from("PARENT"));
        assert!(m("subtask", &child) && !m("subtask", &task("top")));
    }
}
