use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use keel_core::commands::{Action, Command};
use keel_core::model::Due;
use keel_core::reminders::ReminderSettings;
use keel_service::AppService;

fn at(d: u32, h: u32, m: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, m, 0).unwrap()
}

fn add(svc: &AppService, uuid: &str, name: &str, due: &str) {
    let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap().id;
    svc.dispatch_at(
        Command {
            uuid: uuid.into(),
            temp_id: Some("t".into()),
            action: Action::TaskAdd {
                content: name.into(),
                project_id: inbox,
                description: None,
                priority: None,
                due: Some(Due {
                    date: due.into(),
                    timezone: None,
                    string: due.into(),
                    lang: "en".into(),
                    is_recurring: false,
                    recurrence: None,
                }),
                labels: None,
                parent_id: None,
                section_id: None,
                recurrence: None,
                deadline: None,
            },
        },
        NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(),
    )
    .unwrap();
}

#[test]
fn settings_default_then_persist_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keel.db");
    let path = path.to_str().unwrap();
    let svc = AppService::open(path).unwrap();
    assert_eq!(svc.reminder_settings().unwrap(), ReminderSettings::default());

    let custom = ReminderSettings {
        enabled: true,
        offset_minutes: 15,
        all_day_time: None,
        daily_summary: NaiveTime::from_hms_opt(8, 0, 0),
    };
    svc.set_reminder_settings(&custom).unwrap();
    drop(svc);
    let svc = AppService::open(path).unwrap();
    assert_eq!(svc.reminder_settings().unwrap(), custom);
}

#[test]
fn plan_uses_stored_settings_and_open_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
    add(&svc, "u1", "Call mom", "2026-10-19T17:00:00");
    assert!(svc.reminder_plan_at(at(19, 8, 0)).unwrap().is_empty(), "off by default");

    svc.set_reminder_settings(&ReminderSettings { enabled: true, ..ReminderSettings::default() }).unwrap();
    let plan = svc.reminder_plan_at(at(19, 8, 0)).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].at, at(19, 17, 0));
    assert_eq!(plan[0].title, "Call mom");
}

#[test]
fn plan_includes_task_reminders_and_list_reminders_returns_them() {
    let dir = tempfile::tempdir().unwrap();
    let svc = AppService::open(dir.path().join("keel.db").to_str().unwrap()).unwrap();
    add(&svc, "u1", "Call mom", "2026-10-19T17:00:00");
    let task = svc.list_tasks(svc.list_projects().unwrap()[0].id.as_str()).unwrap().remove(0);
    svc.dispatch_at(
        Command {
            uuid: "u2".into(),
            temp_id: Some("r".into()),
            action: Action::ReminderAdd { task_id: task.id.clone(), minute_offset: Some(30), at: None },
        },
        NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(),
    )
    .unwrap();
    assert_eq!(svc.list_reminders().unwrap().len(), 1);
    svc.set_reminder_settings(&ReminderSettings { enabled: true, ..ReminderSettings::default() }).unwrap();
    let times: Vec<NaiveDateTime> = svc.reminder_plan_at(at(19, 8, 0)).unwrap().into_iter().map(|r| r.at).collect();
    assert_eq!(times, vec![at(19, 16, 30), at(19, 17, 0)]);
}
