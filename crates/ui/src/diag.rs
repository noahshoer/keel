//! Diagnostics logging from the page. Off by default: the app turns it on
//! from the Settings switch or `KEEL_DIAG=1` (see `src-tauri/src/diag.rs`),
//! and asks us at startup (`init`). While off, nothing is sent and no hooks
//! are installed. While on, `diag::log("what happened")` goes to the same
//! `diag.log` as Rust's and Swift's lines, and JS errors, unhandled
//! rejections, `console.error` and wasm panics are captured automatically.

use std::sync::Once;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
let installed = false;
const send = (src, msg) => {
  if (!window.__keelDiag) return;
  try { window.__TAURI__?.core.invoke('diag_log', { args: { source: src, msg: String(msg) } }).catch(() => {}); } catch (_) {}
};
export function __diag_log(src, msg) { send(src, msg); }
export function __diag_set(on) {
  window.__keelDiag = on;
  if (on) send('js', `diagnostics on; ${navigator.userAgent}; inner ${innerWidth}x${innerHeight}`);
  if (!on || installed) return;
  installed = true;
  window.addEventListener('error', (e) => {
    send('js-error', `${e.message} at ${e.filename}:${e.lineno}:${e.colno}${e.error?.stack ? '\n' + e.error.stack : ''}`);
  });
  window.addEventListener('unhandledrejection', (e) => {
    const r = e.reason;
    send('js-rejection', r?.stack || r?.message || String(r));
  });
  const err = console.error;
  console.error = (...a) => { send('console.error', a.map((x) => x?.stack || (typeof x === 'object' ? JSON.stringify(x) : String(x))).join(' ')); err.apply(console, a); };
  window.visualViewport?.addEventListener('resize', () => {
    const v = window.visualViewport;
    send('viewport', `resize ${v.width.toFixed(0)}x${v.height.toFixed(1)} top ${v.offsetTop.toFixed(1)} scale ${v.scale}`);
  });
}
"#)]
extern "C" {
    fn __diag_log(src: &str, msg: &str);
    fn __diag_set(on: bool);
}

/// Adds a line to the diagnostics log (a no-op while logging is off). Meant
/// to be sprinkled in while chasing a bug, so it may have no callers.
#[allow(dead_code)]
pub fn log(msg: &str) {
    __diag_log("js", msg);
}

/// Turns the page's side of logging on or off; the first time it's on, the
/// error hooks are installed.
pub fn set_active(on: bool) {
    static PANIC_HOOK: Once = Once::new();
    __diag_set(on);
    if on {
        PANIC_HOOK.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                __diag_log("wasm-panic", &info.to_string());
                previous(info);
            }));
        });
    }
}

/// Asks the app whether logging is on at startup.
pub async fn init() {
    if let Ok(on) = crate::api::diag_enabled().await {
        set_active(on);
    }
}
