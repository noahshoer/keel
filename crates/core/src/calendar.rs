//! Month grid for the date picker (pure; host-tested).

use chrono::{Datelike, Duration, NaiveDate};

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
    use super::month_weeks;
    use chrono::NaiveDate;

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
}
