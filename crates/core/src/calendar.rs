//! The date picker's calendar, shared by every platform (pure; host-tested):
//! the month grid with each day's flags, moving between months, and the
//! month / year choices. The web and the iPhone only draw what this returns.

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::Serialize;

use crate::recurrence::Recurrence;

pub const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

/// One day in the grid.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DayCell {
    pub date: NaiveDate,
    /// Day of the month.
    pub day: u32,
    /// Part of the shown month (not a leading/trailing day).
    pub in_month: bool,
    pub today: bool,
    pub past: bool,
    pub selected: bool,
    /// The task repeats on this day (today or later).
    pub repeat: bool,
}

/// A month as the picker draws it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct MonthView {
    pub year: i32,
    pub month: u32,
    /// "October 2026".
    pub title: String,
    /// One-letter weekday headings, in the grid's order.
    pub weekdays: Vec<&'static str>,
    /// Six weeks of seven days.
    pub weeks: Vec<Vec<DayCell>>,
}

/// The month `year`/`month` (1-based), weeks starting on `week_start`.
pub fn month_view(
    year: i32,
    month: u32,
    week_start: Weekday,
    today: NaiveDate,
    selected: Option<NaiveDate>,
    rule: Option<&Recurrence>,
) -> MonthView {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(today);
    let lead = (7 + first.weekday().num_days_from_monday() as i64 - week_start.num_days_from_monday() as i64) % 7;
    let start = first - Duration::days(lead);
    let weeks = (0..6)
        .map(|w| {
            (0..7)
                .map(|i| {
                    let date = start + Duration::days(w * 7 + i);
                    DayCell {
                        date,
                        day: date.day(),
                        in_month: date.month() == first.month(),
                        today: date == today,
                        past: date < today,
                        selected: Some(date) == selected,
                        repeat: date >= today && rule.is_some_and(|r| r.contains(date)),
                    }
                })
                .collect()
        })
        .collect();
    let letters = ["M", "T", "W", "T", "F", "S", "S"];
    let from = week_start.num_days_from_monday() as usize;
    MonthView {
        year: first.year(),
        month: first.month(),
        title: format!("{} {}", MONTHS[first.month0() as usize], first.year()),
        weekdays: (0..7).map(|i| letters[(from + i) % 7]).collect(),
        weeks,
    }
}

/// The month `delta` months from `year`/`month`.
pub fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let index = year * 12 + month as i32 - 1 + delta;
    (index.div_euclid(12), index.rem_euclid(12) as u32 + 1)
}

/// The years the month / year wheel offers: last year to ten ahead,
/// stretched to hold the `shown` year when the arrows went past either end.
pub fn year_choices(today: NaiveDate, shown: i32) -> Vec<i32> {
    (shown.min(today.year() - 1)..=shown.max(today.year() + 10)).collect()
}

/// The weeks shown for the month containing `day`: always 6 rows of
/// Monday–Sunday, starting with the week that holds the 1st.
pub fn month_weeks(day: NaiveDate) -> Vec<[NaiveDate; 7]> {
    let first = day.with_day(1).expect("day 1 exists");
    let start = first - Duration::days(first.weekday().num_days_from_monday() as i64);
    (0..6)
        .map(|w| std::array::from_fn(|d| start + Duration::days(w * 7 + d as i64)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recurrence::Recurrence;
    use chrono::{NaiveDate, Weekday};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn six_monday_first_weeks() {
        // October 2026 starts on a Thursday.
        let weeks = month_weeks(d(2026, 10, 19));
        assert_eq!(weeks.len(), 6);
        assert_eq!(weeks[0][0], d(2026, 9, 28), "leading days from September");
        assert_eq!(weeks[0][3], d(2026, 10, 1));
        assert_eq!(weeks[5][6], d(2026, 11, 8), "trailing days from November");
    }

    #[test]
    fn month_starting_monday_has_no_leading_days() {
        // June 2026 starts on a Monday.
        assert_eq!(month_weeks(d(2026, 6, 30))[0][0], d(2026, 6, 1));
    }

    #[test]
    fn crosses_year_boundary() {
        // January 2027 starts on a Friday.
        let weeks = month_weeks(d(2027, 1, 1));
        assert_eq!(weeks[0][0], d(2026, 12, 28));
        assert_eq!(weeks[0][4], d(2027, 1, 1));
    }

    #[test]
    fn a_month_view_flags_each_day() {
        let today = d(2026, 10, 23);
        let mon_fri = Recurrence::Weekly(vec![0, 4]);
        let v = month_view(2026, 10, Weekday::Sun, today, Some(d(2026, 10, 24)), Some(&mon_fri));
        assert_eq!(v.title, "October 2026");
        assert_eq!(v.weekdays, ["S", "M", "T", "W", "T", "F", "S"]);
        assert_eq!(v.weeks.len(), 6);
        assert_eq!(v.weeks[0][0].date, d(2026, 9, 27), "Sunday-first: starts the Sunday before the 1st");
        assert!(!v.weeks[0][0].in_month);
        let cell = |day| v.weeks.iter().flatten().find(|c| c.date == d(2026, 10, day)).unwrap().clone();
        assert!(cell(23).today && !cell(23).past);
        assert!(cell(22).past);
        assert!(cell(24).selected);
        assert!(cell(26).repeat && cell(30).repeat && !cell(27).repeat);
        assert!(!cell(19).repeat, "past days aren't marked as repeats");
        assert_eq!(cell(5).day, 5);
    }

    #[test]
    fn monday_first_weeks_match_month_weeks() {
        let v = month_view(2026, 10, Weekday::Mon, d(2026, 10, 23), None, None);
        assert_eq!(v.weekdays[0], "M");
        assert_eq!(v.weeks[0][0].date, d(2026, 9, 28));
    }

    #[test]
    fn months_shift_across_years() {
        assert_eq!(shift_month(2026, 10, 1), (2026, 11));
        assert_eq!(shift_month(2026, 12, 1), (2027, 1));
        assert_eq!(shift_month(2026, 1, -1), (2025, 12));
        assert_eq!(shift_month(2026, 10, -22), (2024, 12));
    }

    #[test]
    fn the_year_wheel_offers_last_year_to_ten_ahead_and_the_shown_year() {
        let today = d(2026, 10, 23);
        assert_eq!(year_choices(today, 2026), (2025..=2036).collect::<Vec<_>>());
        // Arrowed past either end: the wheel still holds the shown year.
        assert_eq!(year_choices(today, 2040).last(), Some(&2040));
        assert_eq!(year_choices(today, 2020).first(), Some(&2020));
        assert_eq!(MONTHS[9], "October");
    }
}
