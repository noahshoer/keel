//! What Keel needs from Swift on iOS, and nothing else: the diagnostics log
//! (`Diag.swift`), keyboard pace (`Keyboard.swift`, which tells the page
//! through `window.keelKeyboard`), holding the page still under the task view
//! (`PageScroll.swift`), and the simulator-check script runner
//! (`Script.swift`, `KEEL_SCRIPT`). It draws no views. The Liquid Glass
//! controls are a separate, parked plugin (`tauri-plugin-glass`) that only a
//! build with the `glass` feature loads. On other platforms this plugin does nothing.

use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_keel_ios);

#[cfg(target_os = "ios")]
struct IosHandle<R: Runtime>(tauri::plugin::PluginHandle<R>);

#[cfg(target_os = "ios")]
#[derive(serde::Serialize)]
struct DiagArgs<'a> {
    enabled: Option<bool>,
    source: Option<&'a str>,
    line: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fresh: Option<bool>,
}

/// Talks to the Swift diagnostics log (iOS; a no-op elsewhere): `enabled`
/// switches it on or off, `line` (with its `source`) adds an entry. Errors
/// are ignored, since logging must never get in the way.
pub fn diag<R: Runtime>(
    _app: &tauri::AppHandle<R>,
    _enabled: Option<bool>,
    _source: &str,
    _line: Option<&str>,
) {
    #[cfg(target_os = "ios")]
    {
        use tauri::Manager;
        if let Some(h) = _app.try_state::<IosHandle<R>>() {
            let args = DiagArgs { enabled: _enabled, source: Some(_source), line: _line, fresh: None };
            let _ = h.0.run_mobile_plugin::<()>("diag", args);
        }
    }
}

/// Starts a new diagnostics log on iOS (the last one becomes diag.1.log);
/// a no-op elsewhere.
pub fn diag_fresh<R: Runtime>(_app: &tauri::AppHandle<R>) {
    #[cfg(target_os = "ios")]
    {
        use tauri::Manager;
        if let Some(h) = _app.try_state::<IosHandle<R>>() {
            let args = DiagArgs { enabled: None, source: None, line: None, fresh: Some(true) };
            let _ = h.0.run_mobile_plugin::<()>("diag", args);
        }
    }
}

#[cfg(target_os = "ios")]
#[derive(serde::Serialize)]
struct PageScrollArgs {
    on: bool,
}

/// Holds the page's scroll while the task view is open, so WebKit revealing
/// a field in the sheet doesn't move the list under it (iOS; a no-op
/// elsewhere). Call it from an async command: it waits for Swift.
pub fn hold_page_scroll<R: Runtime>(_app: &tauri::AppHandle<R>, _on: bool) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        use tauri::Manager;
        if let Some(h) = _app.try_state::<IosHandle<R>>() {
            h.0.run_mobile_plugin::<()>("pageScroll", PageScrollArgs { on: _on }).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("keel-ios")
        .setup(|_app, _api| {
            #[cfg(target_os = "ios")]
            {
                use tauri::Manager;
                let handle = _api.register_ios_plugin(init_plugin_keel_ios)?;
                _app.manage(IosHandle(handle));
            }
            Ok(())
        })
        .build()
}
