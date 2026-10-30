use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

/// A recurrence rule. Weekdays are 0 = Monday … 6 = Sunday.
///
/// Interval rules carry an `anchor` (any occurrence) that fixes their phase.
/// Variants are only ever added, so rules stored in older `Due` JSON still
/// deserialize.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Recurrence {
    Daily,
    Weekly(Vec<u8>),
    MonthlyDay(u8),
    /// Every `n` days, in phase with `anchor`.
    EveryNDays { n: u32, anchor: NaiveDate },
    /// On `days` in every `n`th week, counting from the week (Mon–Sun) that holds `anchor`.
    EveryNWeeks { n: u32, days: Vec<u8>, anchor: NaiveDate },
    /// The `day`-th (clamped) of every `n`th month, counting from `anchor`'s month.
    EveryNMonths { n: u32, day: u8, anchor: NaiveDate },
    /// Once a year; Feb 29 falls on Feb 28 in non-leap years.
    Yearly { month: u8, day: u8 },
    MonthlyLastDay,
    /// The first (`1`) or last (`-1`) Monday–Friday of each month.
    MonthlyWorkday { nth: i8 },
    /// The `nth` (1–4, or -1 = last) `weekday` of each month.
    MonthlyNthWeekday { nth: i8, weekday: u8 },
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = NaiveDate::from_ymd_opt(ny, nm, 1).expect("valid date");
    (first_next - Duration::days(1)).day()
}

/// The `day`-th of the given month, clamped to the month's last day.
fn monthly_date(year: i32, month: u32, day: u8) -> NaiveDate {
    let d = (day as u32).clamp(1, days_in_month(year, month));
    NaiveDate::from_ymd_opt(year, month, d).expect("clamped date is valid")
}

fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 { (year + 1, 1) } else { (year, month + 1) }
}

fn prev_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 { (year - 1, 12) } else { (year, month - 1) }
}

fn weekday_index(date: NaiveDate) -> u8 {
    date.weekday().num_days_from_monday() as u8
}

fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(weekday_index(date) as i64)
}

fn month_index(date: NaiveDate) -> i64 {
    date.year() as i64 * 12 + date.month0() as i64
}

fn is_workday(date: NaiveDate) -> bool {
    weekday_index(date) < 5
}

fn last_of_month(date: NaiveDate) -> NaiveDate {
    monthly_date(date.year(), date.month(), 31)
}

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// "1st", "2nd", "3rd", "11th", "22nd".
pub fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn nth_word(nth: i8) -> &'static str {
    match nth {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        _ => "last",
    }
}

fn every_n(n: u32, unit: &str) -> String {
    match n {
        1 => format!("every {unit}"),
        2 => format!("every other {unit}"),
        n => format!("every {n} {unit}s"),
    }
}

fn day_list(days: &[u8]) -> String {
    days.iter().filter_map(|d| WEEKDAYS.get(*d as usize).copied()).collect::<Vec<_>>().join(", ")
}

impl Recurrence {
    /// Whether `date` is a scheduled occurrence.
    pub fn contains(&self, date: NaiveDate) -> bool {
        match self {
            Recurrence::Daily => true,
            Recurrence::Weekly(days) => days.contains(&weekday_index(date)),
            Recurrence::MonthlyDay(day) => date == monthly_date(date.year(), date.month(), *day),
            Recurrence::EveryNDays { n, anchor } => (date - *anchor).num_days().rem_euclid((*n).max(1) as i64) == 0,
            Recurrence::EveryNWeeks { n, days, anchor } => {
                let weeks = (monday_of(date) - monday_of(*anchor)).num_days() / 7;
                days.contains(&weekday_index(date)) && weeks.rem_euclid((*n).max(1) as i64) == 0
            }
            Recurrence::EveryNMonths { n, day, anchor } => {
                (month_index(date) - month_index(*anchor)).rem_euclid((*n).max(1) as i64) == 0
                    && date == monthly_date(date.year(), date.month(), *day)
            }
            Recurrence::Yearly { month, day } => {
                date.month() == *month as u32 && date == monthly_date(date.year(), *month as u32, *day)
            }
            Recurrence::MonthlyLastDay => date == last_of_month(date),
            Recurrence::MonthlyWorkday { nth } => {
                if !is_workday(date) {
                    return false;
                }
                if *nth < 0 {
                    (1..=3).map(|k| date + Duration::days(k)).take_while(|d| d.month() == date.month()).all(|d| !is_workday(d))
                } else {
                    (1..=3).map(|k| date - Duration::days(k)).take_while(|d| d.month() == date.month()).all(|d| !is_workday(d))
                }
            }
            Recurrence::MonthlyNthWeekday { nth, weekday } => {
                if weekday_index(date) != *weekday {
                    return false;
                }
                if *nth < 0 {
                    (date + Duration::days(7)).month() != date.month()
                } else {
                    (date.day() as i64 - 1) / 7 + 1 == *nth as i64
                }
            }
        }
    }

    /// The longest gap between two occurrences (bounds the day-by-day scans).
    fn max_gap_days(&self) -> i64 {
        match self {
            Recurrence::Daily => 1,
            Recurrence::Weekly(_) => 7,
            Recurrence::EveryNDays { n, .. } => (*n).max(1) as i64,
            Recurrence::EveryNWeeks { n, .. } => 7 * (*n).max(1) as i64 + 7,
            Recurrence::EveryNMonths { n, .. } => 31 * (*n).max(1) as i64 + 31,
            Recurrence::Yearly { .. } => 366,
            _ => 62,
        }
    }

    /// The days in `[from, from + days)` the rule falls on, in order:
    /// what the date pickers ring as repeats.
    pub fn days_within(&self, from: NaiveDate, days: i64) -> Vec<NaiveDate> {
        (0..days).map(|i| from + chrono::Duration::days(i)).filter(|d| self.contains(*d)).collect()
    }

    /// The first occurrence strictly after `date`.
    pub fn next_after(&self, date: NaiveDate) -> NaiveDate {
        match self {
            Recurrence::Daily => date + Duration::days(1),
            Recurrence::MonthlyDay(day) => {
                let this = monthly_date(date.year(), date.month(), *day);
                if this > date {
                    this
                } else {
                    let (y, m) = next_month(date.year(), date.month());
                    monthly_date(y, m, *day)
                }
            }
            _ => (1..=self.max_gap_days())
                .map(|i| date + Duration::days(i))
                .find(|c| self.contains(*c))
                .unwrap_or(date + Duration::days(self.max_gap_days())),
        }
    }

    /// The last occurrence strictly before `date`.
    pub fn prev_before(&self, date: NaiveDate) -> NaiveDate {
        match self {
            Recurrence::Daily => date - Duration::days(1),
            Recurrence::MonthlyDay(day) => {
                let this = monthly_date(date.year(), date.month(), *day);
                if this < date {
                    this
                } else {
                    let (y, m) = prev_month(date.year(), date.month());
                    monthly_date(y, m, *day)
                }
            }
            _ => (1..=self.max_gap_days())
                .map(|i| date - Duration::days(i))
                .find(|c| self.contains(*c))
                .unwrap_or(date - Duration::days(self.max_gap_days())),
        }
    }

    /// Todoist-style text, e.g. "every other week on Wed" (stored in
    /// `Due.string`, and shown in the editor).
    pub fn describe(&self) -> String {
        match self {
            Recurrence::Daily => "every day".to_string(),
            Recurrence::Weekly(days) if days == &[0, 1, 2, 3, 4] => "every weekday".to_string(),
            Recurrence::Weekly(days) => format!("every {}", day_list(days)),
            Recurrence::MonthlyDay(day) => format!("every month on the {}", ordinal(*day as u32)),
            Recurrence::EveryNDays { n, .. } => every_n(*n, "day"),
            Recurrence::EveryNWeeks { n, days, .. } => format!("{} on {}", every_n(*n, "week"), day_list(days)),
            Recurrence::EveryNMonths { n, day, .. } => {
                format!("{} on the {}", every_n(*n, "month"), ordinal(*day as u32))
            }
            Recurrence::Yearly { month, day } => {
                format!("every year on {} {day}", MONTHS[(*month as usize).clamp(1, 12) - 1])
            }
            Recurrence::MonthlyLastDay => "every last day of the month".to_string(),
            Recurrence::MonthlyWorkday { nth } => format!("every {} workday", nth_word(*nth)),
            Recurrence::MonthlyNthWeekday { nth, weekday } => {
                format!("every {} {}", nth_word(*nth), WEEKDAYS[(*weekday as usize).min(6)])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Recurrence;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn daily_next_and_prev() {
        let r = Recurrence::Daily;
        assert_eq!(r.next_after(d(2026, 1, 5)), d(2026, 1, 6));
        assert_eq!(r.prev_before(d(2026, 1, 5)), d(2026, 1, 4));
        assert!(r.contains(d(2026, 3, 17)));
    }

    #[test]
    fn weekly_multi_weekday_occurrences() {
        // 0=Mon, 2=Wed, 4=Fri. 2026-01-05 is a Monday.
        let r = Recurrence::Weekly(vec![0, 2, 4]);
        assert_eq!(r.next_after(d(2026, 1, 5)), d(2026, 1, 7), "Mon -> Wed");
        assert_eq!(r.next_after(d(2026, 1, 9)), d(2026, 1, 12), "Fri -> next Mon");
        assert_eq!(r.prev_before(d(2026, 1, 7)), d(2026, 1, 5), "Wed -> Mon");
        assert!(!r.contains(d(2026, 1, 6)), "Tue is not scheduled");
        assert!(r.contains(d(2026, 1, 9)));
    }

    #[test]
    fn monthly_day_clamps_to_month_end() {
        assert_eq!(
            Recurrence::MonthlyDay(31).next_after(d(2026, 1, 31)),
            d(2026, 2, 28),
            "day 31 clamps to Feb 28 in a non-leap year"
        );
        assert_eq!(Recurrence::MonthlyDay(15).next_after(d(2026, 1, 20)), d(2026, 2, 15));
        assert_eq!(Recurrence::MonthlyDay(15).next_after(d(2026, 1, 10)), d(2026, 1, 15));
        assert_eq!(Recurrence::MonthlyDay(31).prev_before(d(2026, 3, 5)), d(2026, 2, 28));
        assert!(Recurrence::MonthlyDay(31).contains(d(2026, 2, 28)));
    }

    #[test]
    fn every_n_days_keeps_phase_from_anchor() {
        let r = Recurrence::EveryNDays { n: 3, anchor: d(2026, 10, 19) };
        assert!(r.contains(d(2026, 10, 22)));
        assert!(!r.contains(d(2026, 10, 21)));
        assert!(r.contains(d(2026, 10, 16)), "before the anchor too");
        assert_eq!(r.next_after(d(2026, 10, 19)), d(2026, 10, 22));
        assert_eq!(r.next_after(d(2026, 10, 20)), d(2026, 10, 22));
        assert_eq!(r.prev_before(d(2026, 10, 22)), d(2026, 10, 19));
    }

    #[test]
    fn every_n_weeks_alternates_weeks() {
        // Anchor Wed 2026-10-21; every other week on Wed.
        let r = Recurrence::EveryNWeeks { n: 2, days: vec![2], anchor: d(2026, 10, 21) };
        assert!(r.contains(d(2026, 10, 21)));
        assert!(!r.contains(d(2026, 10, 28)), "off week");
        assert!(r.contains(d(2026, 11, 4)));
        assert_eq!(r.next_after(d(2026, 10, 21)), d(2026, 11, 4));
        assert_eq!(r.prev_before(d(2026, 11, 4)), d(2026, 10, 21));
        // Mon + Thu every 2 weeks, anchored on a Thursday: that week's Monday counts too.
        let r = Recurrence::EveryNWeeks { n: 2, days: vec![0, 3], anchor: d(2026, 10, 22) };
        assert!(r.contains(d(2026, 10, 19)) && r.contains(d(2026, 10, 22)));
        assert!(!r.contains(d(2026, 10, 26)));
        assert_eq!(r.next_after(d(2026, 10, 22)), d(2026, 11, 2));
    }

    #[test]
    fn every_n_months_clamps_and_skips() {
        let r = Recurrence::EveryNMonths { n: 3, day: 31, anchor: d(2026, 1, 31) };
        assert_eq!(r.next_after(d(2026, 1, 31)), d(2026, 4, 30), "Apr has 30 days");
        assert_eq!(r.next_after(d(2026, 4, 30)), d(2026, 7, 31));
        assert!(!r.contains(d(2026, 2, 28)));
        assert_eq!(r.prev_before(d(2026, 4, 30)), d(2026, 1, 31));
    }

    #[test]
    fn yearly_including_leap_day() {
        let r = Recurrence::Yearly { month: 1, day: 5 };
        assert_eq!(r.next_after(d(2026, 1, 5)), d(2027, 1, 5));
        assert_eq!(r.next_after(d(2026, 10, 19)), d(2027, 1, 5));
        assert_eq!(r.prev_before(d(2026, 10, 19)), d(2026, 1, 5));
        let leap = Recurrence::Yearly { month: 2, day: 29 };
        assert_eq!(leap.next_after(d(2027, 1, 1)), d(2027, 2, 28));
        assert_eq!(leap.next_after(d(2027, 3, 1)), d(2028, 2, 29));
    }

    #[test]
    fn last_day_and_workdays_of_month() {
        assert_eq!(Recurrence::MonthlyLastDay.next_after(d(2026, 2, 10)), d(2026, 2, 28));
        assert_eq!(Recurrence::MonthlyLastDay.next_after(d(2026, 2, 28)), d(2026, 3, 31));
        // Oct 2026 ends on Saturday 31st → last workday is Fri 30th.
        let last = Recurrence::MonthlyWorkday { nth: -1 };
        assert_eq!(last.next_after(d(2026, 10, 19)), d(2026, 10, 30));
        assert!(!last.contains(d(2026, 10, 31)));
        // Nov 2026 starts on Sunday → first workday is Mon 2nd.
        let first = Recurrence::MonthlyWorkday { nth: 1 };
        assert_eq!(first.next_after(d(2026, 10, 19)), d(2026, 11, 2));
    }

    #[test]
    fn nth_weekday_of_month() {
        // First Monday of Nov 2026 = 2nd; last Friday of Oct 2026 = 30th.
        let first_mon = Recurrence::MonthlyNthWeekday { nth: 1, weekday: 0 };
        assert_eq!(first_mon.next_after(d(2026, 10, 19)), d(2026, 11, 2));
        let last_fri = Recurrence::MonthlyNthWeekday { nth: -1, weekday: 4 };
        assert_eq!(last_fri.next_after(d(2026, 10, 1)), d(2026, 10, 30));
        let third_tue = Recurrence::MonthlyNthWeekday { nth: 3, weekday: 1 };
        assert_eq!(third_tue.next_after(d(2026, 10, 1)), d(2026, 10, 20));
        assert_eq!(third_tue.prev_before(d(2026, 10, 20)), d(2026, 9, 15));
    }

    /// For every rule, next/prev agree with `contains` over a long stretch.
    #[test]
    fn next_and_prev_agree_with_contains() {
        let rules = vec![
            Recurrence::Daily,
            Recurrence::Weekly(vec![1, 5]),
            Recurrence::MonthlyDay(30),
            Recurrence::EveryNDays { n: 4, anchor: d(2026, 3, 3) },
            Recurrence::EveryNWeeks { n: 3, days: vec![0, 4], anchor: d(2026, 5, 6) },
            Recurrence::EveryNMonths { n: 2, day: 29, anchor: d(2026, 1, 29) },
            Recurrence::Yearly { month: 2, day: 29 },
            Recurrence::MonthlyLastDay,
            Recurrence::MonthlyWorkday { nth: 1 },
            Recurrence::MonthlyWorkday { nth: -1 },
            Recurrence::MonthlyNthWeekday { nth: 4, weekday: 6 },
            Recurrence::MonthlyNthWeekday { nth: -1, weekday: 2 },
        ];
        let start = d(2026, 1, 1);
        for r in &rules {
            let days: Vec<NaiveDate> = (0..900).map(|i| start + chrono::Duration::days(i)).collect();
            for &day in &days[..800] {
                let next = r.next_after(day);
                assert!(next > day && r.contains(next), "{r:?} next_after({day}) = {next}");
                assert!(
                    (1..(next - day).num_days()).all(|k| !r.contains(day + chrono::Duration::days(k))),
                    "{r:?} skipped an occurrence after {day}"
                );
                if day > d(2026, 6, 1) {
                    let prev = r.prev_before(day);
                    assert!(prev < day && r.contains(prev), "{r:?} prev_before({day}) = {prev}");
                    assert_eq!(r.next_after(prev) >= day, true, "{r:?} prev_before({day}) not the latest");
                }
            }
        }
    }

    #[test]
    fn old_rules_still_deserialize() {
        let r: Recurrence = serde_json::from_str(r#"{"Weekly":[0,2]}"#).unwrap();
        assert_eq!(r, Recurrence::Weekly(vec![0, 2]));
        let r: Recurrence = serde_json::from_str(r#""Daily""#).unwrap();
        assert_eq!(r, Recurrence::Daily);
    }

    #[test]
    fn descriptions() {
        assert_eq!(Recurrence::Daily.describe(), "every day");
        assert_eq!(Recurrence::Weekly(vec![0, 1, 2, 3, 4]).describe(), "every weekday");
        assert_eq!(Recurrence::Weekly(vec![0, 2]).describe(), "every Mon, Wed");
        assert_eq!(Recurrence::MonthlyDay(15).describe(), "every month on the 15th");
        assert_eq!(Recurrence::EveryNDays { n: 2, anchor: d(2026, 1, 1) }.describe(), "every other day");
        assert_eq!(Recurrence::EveryNDays { n: 3, anchor: d(2026, 1, 1) }.describe(), "every 3 days");
        assert_eq!(
            Recurrence::EveryNWeeks { n: 2, days: vec![2], anchor: d(2026, 1, 1) }.describe(),
            "every other week on Wed"
        );
        assert_eq!(
            Recurrence::EveryNMonths { n: 3, day: 1, anchor: d(2026, 1, 1) }.describe(),
            "every 3 months on the 1st"
        );
        assert_eq!(Recurrence::Yearly { month: 1, day: 5 }.describe(), "every year on Jan 5");
        assert_eq!(Recurrence::MonthlyLastDay.describe(), "every last day of the month");
        assert_eq!(Recurrence::MonthlyWorkday { nth: -1 }.describe(), "every last workday");
        assert_eq!(Recurrence::MonthlyWorkday { nth: 1 }.describe(), "every first workday");
        assert_eq!(Recurrence::MonthlyNthWeekday { nth: 2, weekday: 1 }.describe(), "every second Tue");
        assert_eq!(Recurrence::MonthlyNthWeekday { nth: -1, weekday: 4 }.describe(), "every last Fri");
    }

    #[test]
    fn days_within_lists_each_occurrence_in_the_window() {
        let mon_fri = Recurrence::Weekly(vec![0, 4]);
        let from = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(); // a Friday
        let days = mon_fri.days_within(from, 14);
        let ymd = |d: u32| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
        assert_eq!(days, vec![ymd(23), ymd(26), ymd(30), NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()]);
        assert!(Recurrence::Daily.days_within(from, 0).is_empty());
    }
}
