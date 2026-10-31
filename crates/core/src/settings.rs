//! What Settings shows about the data, shared by the Dioxus and React UIs:
//! the reminder menus (choices, labels, which is checked), what a pick
//! changes, when an upcoming reminder fires, and an import's result line.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};

use crate::import::todoist::ImportSummary;
use crate::reminders::ReminderSettings;
use crate::views;

/// "At the due time", "1 hour before", "15 minutes before".
pub fn offset_label(m: i64) -> String {
    match m {
        0 => "At the due time".to_string(),
        60 => "1 hour before".to_string(),
        m => format!("{m} minutes before"),
    }
}

/// One choice in a settings menu.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SettingChoice {
    pub value: String,
    pub label: String,
}

/// A settings menu: the button's text, the checked value and the choices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SettingMenu {
    pub text: String,
    pub selected: String,
    pub items: Vec<SettingChoice>,
}

/// The three reminder menus in Settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ReminderMenus {
    pub offset: SettingMenu,
    pub all_day: SettingMenu,
    pub summary: SettingMenu,
}

/// Which reminder menu a pick came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum MenuKind {
    Offset,
    AllDay,
    Summary,
}

const OFFSETS: [i64; 6] = [0, 5, 10, 15, 30, 60];
const ALL_DAY: [(u32, u32); 6] = [(7, 0), (8, 0), (9, 0), (10, 0), (12, 0), (18, 0)];
const SUMMARY: [(u32, u32); 5] = [(6, 30), (7, 0), (7, 30), (8, 0), (9, 0)];

/// Menu value for an optional time ("off" or "HH:MM").
fn time_value(t: Option<NaiveTime>) -> String {
    t.map(|t| format!("{:02}:{:02}", t.hour(), t.minute())).unwrap_or_else(|| "off".to_string())
}

fn parse_time_value(v: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(v, "%H:%M").ok()
}

fn time_menu(current: Option<NaiveTime>, off_label: &str, times: &[(u32, u32)]) -> SettingMenu {
    let items = std::iter::once(SettingChoice { value: "off".to_string(), label: off_label.to_string() })
        .chain(times.iter().filter_map(|&(h, m)| NaiveTime::from_hms_opt(h, m, 0)).map(|t| SettingChoice {
            value: time_value(Some(t)),
            label: views::time_label(t),
        }))
        .collect();
    SettingMenu {
        text: current.map(views::time_label).unwrap_or_else(|| off_label.to_string()),
        selected: time_value(current),
        items,
    }
}

pub fn reminder_menus(s: &ReminderSettings) -> ReminderMenus {
    ReminderMenus {
        offset: SettingMenu {
            text: offset_label(s.offset_minutes),
            selected: s.offset_minutes.to_string(),
            items: OFFSETS.iter().map(|m| SettingChoice { value: m.to_string(), label: offset_label(*m) }).collect(),
        },
        all_day: time_menu(s.all_day_time, "No reminder", &ALL_DAY),
        summary: time_menu(s.daily_summary, "Off", &SUMMARY),
    }
}

/// The settings after picking `value` in one menu. An unreadable offset is
/// 0 and an unreadable time is off.
pub fn pick_setting(s: &ReminderSettings, kind: MenuKind, value: &str) -> ReminderSettings {
    let mut next = s.clone();
    match kind {
        MenuKind::Offset => next.offset_minutes = value.parse().unwrap_or(0),
        MenuKind::AllDay => next.all_day_time = parse_time_value(value),
        MenuKind::Summary => next.daily_summary = parse_time_value(value),
    }
    next
}

/// "Today 5 PM", "Tue 9 AM", "3 Nov 9 AM".
pub fn upcoming_when(at: NaiveDateTime, today: NaiveDate) -> String {
    let date = at.date();
    let day = match (date - today).num_days() {
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        2..=6 => date.format("%a").to_string(),
        _ => views::due_label(date, today),
    };
    format!("{day} {}", views::time_label(at.time()))
}

/// "Travel: 12 tasks, 2 sections, 3 new labels, 1 date kept as text in the description".
pub fn import_line(s: &ImportSummary) -> String {
    let plural = |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut parts = vec![plural(s.tasks, "task", "tasks")];
    if s.sections > 0 {
        parts.push(plural(s.sections, "section", "sections"));
    }
    if s.labels_created > 0 {
        parts.push(plural(s.labels_created, "new label", "new labels"));
    }
    if s.comments > 0 {
        parts.push(plural(s.comments, "comment", "comments"));
    }
    if s.kept_as_text > 0 {
        parts.push(format!("{} kept as text in the description", plural(s.kept_as_text, "date", "dates")));
    }
    format!("{}: {}", s.project_name, parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Id;

    fn hm(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    #[test]
    fn offset_labels() {
        assert_eq!(offset_label(0), "At the due time");
        assert_eq!(offset_label(60), "1 hour before");
        assert_eq!(offset_label(15), "15 minutes before");
    }

    #[test]
    fn menus_of_the_defaults() {
        let m = reminder_menus(&ReminderSettings::default());
        assert_eq!((m.offset.text.as_str(), m.offset.selected.as_str()), ("At the due time", "0"));
        assert_eq!(m.offset.items.len(), 6);
        assert_eq!(m.offset.items[5].label, "1 hour before");
        assert_eq!((m.all_day.text.as_str(), m.all_day.selected.as_str()), ("9 AM", "09:00"));
        assert_eq!(m.all_day.items.len(), 7);
        assert_eq!(m.all_day.items[0], SettingChoice { value: "off".into(), label: "No reminder".into() });
        assert_eq!(m.all_day.items[6].label, "6 PM");
        assert_eq!((m.summary.text.as_str(), m.summary.selected.as_str()), ("Off", "off"));
        assert_eq!(m.summary.items.len(), 6);
        assert_eq!(m.summary.items[1], SettingChoice { value: "06:30".into(), label: "6:30 AM".into() });
    }

    #[test]
    fn a_stored_time_with_seconds_still_selects_its_item() {
        let s = ReminderSettings { daily_summary: NaiveTime::from_hms_opt(7, 30, 15), ..Default::default() };
        let m = reminder_menus(&s);
        assert_eq!(m.summary.selected, "07:30");
        assert!(m.summary.items.iter().any(|i| i.value == m.summary.selected));
    }

    #[test]
    fn picks_change_one_setting() {
        let s = ReminderSettings::default();
        assert_eq!(pick_setting(&s, MenuKind::Offset, "30").offset_minutes, 30);
        assert_eq!(pick_setting(&s, MenuKind::Offset, "x").offset_minutes, 0);
        assert_eq!(pick_setting(&s, MenuKind::AllDay, "off").all_day_time, None);
        assert_eq!(pick_setting(&s, MenuKind::AllDay, "18:00").all_day_time, Some(hm(18, 0)));
        let picked = pick_setting(&s, MenuKind::Summary, "07:30");
        assert_eq!(picked.daily_summary, Some(hm(7, 30)));
        assert_eq!(picked.all_day_time, s.all_day_time);
        assert_eq!(picked.enabled, s.enabled);
    }

    #[test]
    fn upcoming_times() {
        let today = day(30); // a Friday
        assert_eq!(upcoming_when(today.and_time(hm(17, 0)), today), "Today 5 PM");
        assert_eq!(upcoming_when(day(31).and_time(hm(9, 0)), today), "Tomorrow 9 AM");
        let monday = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        assert_eq!(upcoming_when(monday.and_time(hm(9, 30)), today), "Mon 9:30 AM");
        let later = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        assert_eq!(upcoming_when(later.and_time(hm(9, 0)), today), format!("{} 9 AM", views::due_label(later, today)));
        assert_eq!(upcoming_when(day(29).and_time(hm(9, 0)), today), format!("{} 9 AM", views::due_label(day(29), today)));
    }

    fn summary(tasks: usize, sections: usize, labels: usize, comments: usize, kept: usize) -> ImportSummary {
        ImportSummary {
            project_id: Id::new(),
            project_name: "Travel".into(),
            tasks,
            sections,
            labels_created: labels,
            comments,
            kept_as_text: kept,
        }
    }

    #[test]
    fn import_lines() {
        assert_eq!(import_line(&summary(1, 0, 0, 0, 0)), "Travel: 1 task");
        assert_eq!(
            import_line(&summary(12, 2, 3, 4, 2)),
            "Travel: 12 tasks, 2 sections, 3 new labels, 4 comments, 2 dates kept as text in the description"
        );
        assert_eq!(import_line(&summary(2, 1, 1, 1, 1)), "Travel: 2 tasks, 1 section, 1 new label, 1 comment, 1 date kept as text in the description");
    }
}
