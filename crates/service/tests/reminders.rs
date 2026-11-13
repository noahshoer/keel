use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use keel_core::commands::Action;
use keel_core::ids::Id;
use keel_core::model::Due;
use keel_core::reminders::ReminderSettings;

mod common;
use common::{d, Fx, NewTask};

fn at(d: u32, h: u32, m: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, d)
        .unwrap()
        .and_hms_opt(h, m, 0)
        .unwrap()
}

impl Fx {
    fn add(&mut self, name: &str, due: &str) -> Id {
        let due = Due {
            date: due.into(),
            timezone: None,
            string: due.into(),
            lang: "en".into(),
            is_recurring: false,
            recurrence: None,
            until: None,
            moved_from: None,
        };
        self.add_task(NewTask::new(name).due(Some(due)))
    }
}

#[test]
fn settings_default_then_persist_across_reopen() {
    let mut f = Fx::on(d(2026, 10, 19));
    assert_eq!(
        f.svc.reminder_settings().unwrap(),
        ReminderSettings::default()
    );

    let custom = ReminderSettings {
        enabled: true,
        offset_minutes: 15,
        all_day_time: None,
        daily_summary: NaiveTime::from_hms_opt(8, 0, 0),
    };
    f.svc.set_reminder_settings(&custom).unwrap();
    f.reopen();
    assert_eq!(f.svc.reminder_settings().unwrap(), custom);
}

#[test]
fn plan_uses_stored_settings_and_open_tasks() {
    let mut f = Fx::on(d(2026, 10, 19));
    f.add("Call mom", "2026-10-19T17:00:00");
    assert!(
        f.svc.reminder_plan_at(at(19, 8, 0)).unwrap().is_empty(),
        "off by default"
    );

    f.svc
        .set_reminder_settings(&ReminderSettings {
            enabled: true,
            ..ReminderSettings::default()
        })
        .unwrap();
    let plan = f.svc.reminder_plan_at(at(19, 8, 0)).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].at, at(19, 17, 0));
    assert_eq!(plan[0].title, "Call mom");
}

#[test]
fn plan_includes_task_reminders_and_list_reminders_returns_them() {
    let mut f = Fx::on(d(2026, 10, 19));
    let task = f.add("Call mom", "2026-10-19T17:00:00");
    f.run(Action::ReminderAdd {
        task_id: task,
        minute_offset: Some(30),
        at: None,
    });
    assert_eq!(f.svc.list_reminders().unwrap().len(), 1);
    f.svc
        .set_reminder_settings(&ReminderSettings {
            enabled: true,
            ..ReminderSettings::default()
        })
        .unwrap();
    let times: Vec<NaiveDateTime> = f
        .svc
        .reminder_plan_at(at(19, 8, 0))
        .unwrap()
        .into_iter()
        .map(|r| r.at)
        .collect();
    assert_eq!(times, vec![at(19, 16, 30), at(19, 17, 0)]);
}

#[test]
fn scheduled_ids_default_empty_then_persist_across_reopen() {
    let mut f = Fx::on(d(2026, 10, 19));
    assert!(f.svc.scheduled_reminder_ids().unwrap().is_empty());

    f.svc.set_scheduled_reminder_ids(&[7, -3, 42]).unwrap();
    f.reopen();
    assert_eq!(f.svc.scheduled_reminder_ids().unwrap(), vec![7, -3, 42]);
}
