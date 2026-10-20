//! Native Liquid Glass controls drawn over the web view on iOS. The Swift
//! side (`ios/`) owns the views; the web UI drives it with plugin commands
//! and listens for its events. On other platforms the plugin does nothing.

use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_glass);

/// Whether the native glass views exist on this platform. The web UI asks
/// once at startup and keeps its own composer and buttons when it's false.
fn available() -> bool {
    cfg!(target_os = "ios")
}

#[tauri::command]
fn is_available() -> bool {
    available()
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("glass")
        .invoke_handler(tauri::generate_handler![is_available])
        .setup(|_app, _api| {
            #[cfg(target_os = "ios")]
            _api.register_ios_plugin(init_plugin_glass)?;
            Ok(())
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "ios"))]
    #[test]
    fn available_is_false_on_desktop() {
        assert!(!available());
    }
}
