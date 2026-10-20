//! Todoist-style filter queries: parse once, evaluate per task.
//!
//! Precedence (loosest first): `,` separates result groups, then `|`, then
//! `&`, then `!`; parentheses group. Terms are case-insensitive.

use std::collections::HashMap;
use std::fmt;

use chrono::{Duration, NaiveDate};

use crate::ids::Id;
use crate::model::{Priority, Task};
use crate::views::due_date_of;

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
pub struct FilterGroup {
    /// The group's own query text (e.g. "overdue | @home").
    pub query: String,
    pub tasks: Vec<Task>,
}

/// The text of each comma-separated group, trimmed (labels for result lists).
pub fn group_texts(query: &str) -> Vec<String> {
    query.split(',').map(|s| s.trim().to_string()).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterError(pub String);

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FilterError {}

fn err<T>(msg: impl Into<String>) -> Result<T, FilterError> {
    Err(FilterError(msg.into()))
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
    Recurring,
    NoLabels,
    Subtask,
    Priority(Priority),
    /// Lower-cased names.
    Project(String),
    Section(String),
    Label(String),
    Search(String),
    DueBefore(DateRef),
    DueAfter(DateRef),
    DueOn(DateRef),
    /// today ≤ due < today + n.
    NextDays(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Term(Term),
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    And,
    Or,
    Not,
    Open,
    Close,
    Comma,
    Text(String),
}

fn tokenize(query: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut text = String::new();
    let flush = |text: &mut String, tokens: &mut Vec<Token>| {
        let t = text.trim();
        if !t.is_empty() {
            tokens.push(Token::Text(t.to_string()));
        }
        text.clear();
    };
    for c in query.chars() {
        let op = match c {
            '&' => Some(Token::And),
            '|' => Some(Token::Or),
            '!' => Some(Token::Not),
            '(' => Some(Token::Open),
            ')' => Some(Token::Close),
            ',' => Some(Token::Comma),
            _ => None,
        };
        match op {
            Some(tok) => {
                flush(&mut text, &mut tokens);
                tokens.push(tok);
            }
            None => text.push(c),
        }
    }
    flush(&mut text, &mut tokens);
    tokens
}

/// `N days`, `N day`, `+N days`, `-N days` → N (sign kept).
fn parse_days(s: &str) -> Option<i64> {
    let s = s.trim();
    let num = s.strip_suffix("days").or_else(|| s.strip_suffix("day"))?.trim();
    let num = num.strip_prefix('+').unwrap_or(num);
    num.parse::<i64>().ok()
}

fn parse_date(raw: &str) -> Result<DateRef, FilterError> {
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
            NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(DateRef::On)
                .or_else(|_| err(format!("Unrecognized date “{}” (try today, +3 days, or 2026-10-31)", raw.trim())))
        }
    }
}

fn named(rest: &str, what: &str, raw: &str) -> Result<String, FilterError> {
    let name = rest.trim().to_lowercase();
    if name.is_empty() {
        err(format!("“{raw}” needs a {what} name"))
    } else {
        Ok(name)
    }
}

fn parse_term(raw: &str) -> Result<Term, FilterError> {
    let lower = raw.to_lowercase();
    let t = lower.as_str();
    Ok(match t {
        "all" => Term::All,
        "today" => Term::Today,
        "tomorrow" => Term::Tomorrow,
        "overdue" => Term::Overdue,
        "no date" => Term::NoDate,
        "recurring" => Term::Recurring,
        "no labels" => Term::NoLabels,
        "subtask" => Term::Subtask,
        "p1" => Term::Priority(Priority::P1),
        "p2" => Term::Priority(Priority::P2),
        "p3" => Term::Priority(Priority::P3),
        "p4" => Term::Priority(Priority::None),
        _ => {
            if let Some(rest) = t.strip_prefix("##").or_else(|| t.strip_prefix('#')) {
                Term::Project(named(rest, "project", raw)?)
            } else if let Some(rest) = t.strip_prefix('/') {
                Term::Section(named(rest, "section", raw)?)
            } else if let Some(rest) = t.strip_prefix('@') {
                Term::Label(named(rest, "label", raw)?)
            } else if let Some(rest) = t.strip_prefix("search:") {
                Term::Search(named(rest, "search", raw)?)
            } else if let Some(rest) = t.strip_prefix("due before:") {
                Term::DueBefore(parse_date(rest)?)
            } else if let Some(rest) = t.strip_prefix("due after:") {
                Term::DueAfter(parse_date(rest)?)
            } else if let Some(rest) = t.strip_prefix("due:") {
                Term::DueOn(parse_date(rest)?)
            } else if let Some(n) = parse_days(t.strip_prefix("next ").unwrap_or(t)) {
                if n < 1 {
                    return err(format!("“{raw}” needs at least 1 day"));
                }
                Term::NextDays(n)
            } else {
                return err(format!("Unknown filter term “{raw}”"));
            }
        }
    })
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
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
            Some(Token::Text(text)) => Ok(Expr::Term(parse_term(&text)?)),
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

/// Parses a query into its comma-separated groups.
pub fn parse(query: &str) -> Result<Vec<Expr>, FilterError> {
    let mut p = Parser { tokens: tokenize(query), pos: 0 };
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
    let lower_eq = |a: &str, b: &str| a.to_lowercase() == b;
    match term {
        Term::All => true,
        Term::Today => due == Some(today),
        Term::Tomorrow => due == Some(today + Duration::days(1)),
        Term::Overdue => due.is_some_and(|d| d < today),
        Term::NoDate => due.is_none(),
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
        Term::Search(text) => crate::search::matches(task, &crate::search::words(text)),
        Term::DueBefore(r) => due.is_some_and(|d| d < r.resolve(today)),
        Term::DueAfter(r) => due.is_some_and(|d| d > r.resolve(today)),
        Term::DueOn(r) => due == Some(r.resolve(today)),
        Term::NextDays(n) => due.is_some_and(|d| d >= today && d < today + Duration::days(*n)),
    }
}

#[cfg(test)]
mod tests {
    use super::{matches, parse, FilterContext};
    use crate::ids::Id;
    use crate::model::{Due, Priority, Task};
    use crate::recurrence::Recurrence;
    use chrono::NaiveDate;
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
        });
        t
    }

    /// Does `query` (single group) match `t`?
    fn m(query: &str, t: &Task) -> bool {
        let groups = parse(query).unwrap_or_else(|e| panic!("{query:?} should parse: {e}"));
        assert_eq!(groups.len(), 1);
        matches(&groups[0], t, &ctx())
    }

    #[test]
    fn invalid_queries_error() {
        for q in ["", "   ", "p5", "(today", "today)", "& today", "today &", "today | ", "!", "foo", "#", "@", "due before:", "due before: someday", "3 dayz"] {
            assert!(parse(q).is_err(), "{q:?} should be rejected");
        }
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
        let groups = parse("today, overdue,  @home").unwrap();
        assert_eq!(groups.len(), 3);
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
