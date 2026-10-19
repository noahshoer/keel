//! Theme preference: "system" (follow the OS), "light", or "dark".
//! Applied as `data-theme` on <html> and remembered in localStorage.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
export function __get_theme() {
  try { return localStorage.getItem('theme') || 'system'; } catch (e) { return 'system'; }
}
export function __apply_theme(t) {
  try { localStorage.setItem('theme', t); } catch (e) {}
  if (t === 'light' || t === 'dark') { document.documentElement.dataset.theme = t; }
  else { delete document.documentElement.dataset.theme; }
}
"#)]
extern "C" {
    fn __get_theme() -> String;
    fn __apply_theme(t: &str);
}

/// The saved preference (defaults to "system").
pub fn saved() -> String {
    match __get_theme().as_str() {
        t @ ("light" | "dark") => t.to_string(),
        _ => "system".to_string(),
    }
}

/// Apply and remember a preference.
pub fn apply(theme: &str) {
    __apply_theme(theme);
}

/// System → Light → Dark → System.
pub fn next(theme: &str) -> &'static str {
    match theme {
        "system" => "light",
        "light" => "dark",
        _ => "system",
    }
}

pub fn label(theme: &str) -> &'static str {
    match theme {
        "light" => "Theme: Light (click for Dark)",
        "dark" => "Theme: Dark (click for System)",
        _ => "Theme: System (click for Light)",
    }
}
