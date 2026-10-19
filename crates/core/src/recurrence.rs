use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

/// A recurrence rule. Weekdays are 0 = Monday … 6 = Sunday.
/// (EveryNDays and natural-language rules are deferred.)
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Recurrence {
    Daily,
    Weekly(Vec<u8>),
    MonthlyDay(u8),
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

impl Recurrence {
    /// Whether `date` is a scheduled occurrence.
    pub fn contains(&self, date: NaiveDate) -> bool {
        match self {
            Recurrence::Daily => true,
            Recurrence::Weekly(days) => {
                days.contains(&(date.weekday().num_days_from_monday() as u8))
            }
            Recurrence::MonthlyDay(day) => date == monthly_date(date.year(), date.month(), *day),
        }
    }

    /// The first occurrence strictly after `date`.
    pub fn next_after(&self, date: NaiveDate) -> NaiveDate {
        match self {
            Recurrence::Daily => date + Duration::days(1),
            Recurrence::Weekly(_) => (1..=7)
                .map(|i| date + Duration::days(i))
                .find(|c| self.contains(*c))
                .unwrap_or(date + Duration::days(7)),
            Recurrence::MonthlyDay(day) => {
                let this = monthly_date(date.year(), date.month(), *day);
                if this > date {
                    this
                } else {
                    let (y, m) = next_month(date.year(), date.month());
                    monthly_date(y, m, *day)
                }
            }
        }
    }

    /// The last occurrence strictly before `date`.
    pub fn prev_before(&self, date: NaiveDate) -> NaiveDate {
        match self {
            Recurrence::Daily => date - Duration::days(1),
            Recurrence::Weekly(_) => (1..=7)
                .map(|i| date - Duration::days(i))
                .find(|c| self.contains(*c))
                .unwrap_or(date - Duration::days(7)),
            Recurrence::MonthlyDay(day) => {
                let this = monthly_date(date.year(), date.month(), *day);
                if this < date {
                    this
                } else {
                    let (y, m) = prev_month(date.year(), date.month());
                    monthly_date(y, m, *day)
                }
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
}
