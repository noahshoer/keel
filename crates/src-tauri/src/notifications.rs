//! Delivers the reminder plan (`AppService::reminder_plan`).
//!
//! - Mobile: the OS holds scheduled notifications, so each re-plan cancels
//!   what's pending and schedules the new plan (`Schedule::At`); they arrive
//!   while the app is closed.
//! - Desktop: the plugin can only show immediately, so a background thread
//!   fires plan entries as they come due while the app is open (each id once).

use std::collections::HashSet;
use std::sync::Mutex;

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::{NotificationExt, PermissionState};
use todo_core::reminders::PlannedReminder;
use todo_service::AppService;

/// The current plan, plus (desktop) which ids have already been shown.
#[derive(Default)]
pub struct Reminders {
    plan: Mutex<Vec<PlannedReminder>>,
    fired: Mutex<HashSet<i32>>,
}

/// A reminder that comes due while the computer sleeps is still shown on wake
/// if it's at most this late; older ones are dropped.
#[cfg(desktop)]
const LATE_GRACE_MINUTES: i64 = 60;

pub fn permission_text(state: PermissionState) -> &'static str {
    match state {
        PermissionState::Granted => "granted",
        PermissionState::Denied => "denied",
        _ => "prompt",
    }
}

/// Re-plans and hands the plan to the OS (mobile) or the desktop ticker.
pub fn resync<R: Runtime>(app: &AppHandle<R>) {
    let plan = match app.state::<AppService>().reminder_plan() {
        Ok(plan) => plan,
        Err(e) => {
            eprintln!("reminders: planning failed: {e}");
            return;
        }
    };
    #[cfg(mobile)]
    schedule_on_device(app, &plan);
    *app.state::<Reminders>().plan.lock().expect("reminder plan lock") = plan;
}

#[cfg(mobile)]
fn schedule_on_device<R: Runtime>(app: &AppHandle<R>, plan: &[PlannedReminder]) {
    use chrono::{Local, TimeZone};
    use tauri_plugin_notification::Schedule;

    let n = app.notification();
    // Everything pending was scheduled by us: replace it with the new plan.
    if let Ok(pending) = n.pending() {
        let ids: Vec<i32> = pending.iter().map(|p| p.id()).collect();
        if !ids.is_empty() {
            let _ = n.cancel(ids);
        }
    }
    if !matches!(n.permission_state(), Ok(PermissionState::Granted)) {
        return;
    }
    for r in plan {
        let Some(ts) = Local.from_local_datetime(&r.at).earliest().map(|d| d.timestamp()) else { continue };
        let Ok(date) = time::OffsetDateTime::from_unix_timestamp(ts) else { continue };
        let result = n
            .builder()
            .id(r.id)
            .title(&r.title)
            .body(&r.body)
            .schedule(Schedule::At { date, repeating: false, allow_while_idle: true })
            .show();
        if let Err(e) = result {
            eprintln!("reminders: scheduling {} failed: {e}", r.id);
        }
    }
}

/// Desktop: checks the plan every 20 seconds and shows what's due.
#[cfg(desktop)]
pub fn start_ticker<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(20));
        fire_due(&app);
    });
}

#[cfg(desktop)]
fn fire_due<R: Runtime>(app: &AppHandle<R>) {
    let now = chrono::Local::now().naive_local();
    let state = app.state::<Reminders>();
    let due: Vec<PlannedReminder> = {
        let plan = state.plan.lock().expect("reminder plan lock");
        let mut fired = state.fired.lock().expect("fired lock");
        plan.iter()
            .filter(|r| r.at <= now && fired.insert(r.id))
            .filter(|r| now - r.at <= chrono::Duration::minutes(LATE_GRACE_MINUTES))
            .cloned()
            .collect()
    };
    for r in due {
        match app.notification().builder().title(&r.title).body(&r.body).show() {
            Ok(()) => eprintln!("reminders: showed {} \"{}\" (due {})", r.id, r.title, r.at),
            Err(e) => eprintln!("reminders: showing {} failed: {e}", r.id),
        }
    }
}
