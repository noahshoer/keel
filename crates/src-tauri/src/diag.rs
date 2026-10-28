//! Diagnostics logging, one log for the page, Rust and (on iOS) the Swift
//! plugin. Off by default: the Settings switch (a device-local setting) or
//! `KEEL_DIAG=1` in the environment turns it on. While off, nothing is
//! written. On iOS the Swift plugin owns `Library/Caches/diag.log`; on
//! desktop the file is `diag.log` in the app data dir. Both roll over at
//! ~2 MB, keeping one previous file (`diag.1.log`).
//!
//! Use `diag::log("source", "message")` from anywhere in Rust; the page uses
//! `diag.rs` in the UI crate. Panics are logged automatically.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

#[cfg(not(target_os = "ios"))]
use keel_service::diag::DiagLog;
#[cfg(not(target_os = "ios"))]
use std::sync::Mutex;
use keel_service::AppService;

static ENABLED: AtomicBool = AtomicBool::new(false);
static SINK: OnceLock<Sink> = OnceLock::new();

struct Sink {
    // iOS logs through the Swift plugin; elsewhere Rust owns the file.
    #[cfg(target_os = "ios")]
    app: tauri::AppHandle,
    #[cfg(not(target_os = "ios"))]
    file: Mutex<DiagLog>,
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Reads the setting and `KEEL_DIAG`, opens the sink, and hooks panics.
/// Call once from `setup`, after the service is managed.
pub fn init(app: &tauri::AppHandle, svc: &AppService, dir: &std::path::Path) {
    let env = std::env::var("KEEL_DIAG").is_ok_and(|v| v == "1");
    let on = env || svc.diag_enabled().unwrap_or(false);
    #[cfg(target_os = "ios")]
    let _ = SINK.set(Sink { app: app.clone() });
    #[cfg(not(target_os = "ios"))]
    let _ = SINK.set(Sink { file: Mutex::new(DiagLog::new(dir)) });
    #[cfg(target_os = "ios")]
    let _ = dir;
    #[cfg(not(target_os = "ios"))]
    let _ = app;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if enabled() {
            let trace = std::backtrace::Backtrace::force_capture();
            log("panic", &format!("{info}\n{trace}"));
        }
        previous(info);
    }));
    set_enabled(on);
}

/// Switches logging on or off (the Settings toggle; `init` for startup).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    #[cfg(target_os = "ios")]
    if let Some(sink) = SINK.get() {
        tauri_plugin_glass::diag(&sink.app, Some(on), "rust", None);
    }
    if on {
        log("rust", "---- diagnostics on");
    }
}

/// Adds a line to the log; does nothing while logging is off.
pub fn log(source: &str, msg: &str) {
    if !enabled() {
        return;
    }
    let Some(sink) = SINK.get() else { return };
    #[cfg(target_os = "ios")]
    tauri_plugin_glass::diag(&sink.app, None, source, Some(msg));
    #[cfg(not(target_os = "ios"))]
    if let Ok(file) = sink.file.lock() {
        let _ = file.append(source, msg);
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn diag_enabled() -> bool {
    enabled()
}

/// Async: on iOS this calls into the Swift plugin (see CLAUDE.md).
#[tauri::command(rename_all = "snake_case")]
pub async fn set_diag_enabled(state: tauri::State<'_, AppService>, on: bool) -> Result<(), String> {
    state.set_diag_enabled(on).map_err(|e| e.to_string())?;
    set_enabled(on);
    Ok(())
}

/// A line from the page.
#[tauri::command(rename_all = "snake_case")]
pub async fn diag_log(source: String, msg: String) {
    log(&source, &msg);
}
