//! Completing a task: the circle fills with a Keel icon, then the row
//! switches off like an old TV (squashes to a line, shrinks to a dot) before
//! the task is actually completed. The icon is a per-device preference.

use wasm_bindgen::prelude::*;

/// How long the filled circle shows before the row switches off.
pub const FILL_MS: u32 = 450;
/// The switch-off animation (`.task.switch-off` in style.rs).
pub const OFF_MS: u32 = 500;

/// The mark drawn in a completed circle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompletionIcon {
    /// The logo's calm-water stroke.
    #[default]
    Wave,
    /// The pine from the logo.
    Pine,
    /// The pine, waterline and reflection, like the app icon.
    Reflection,
}

const PINE: &str = "M32 15 L36 20.6 Q35.2 21 34.3 20.9 L39 26.2 Q37.8 26.7 36.4 26.5 L41.4 31.2 Q32 29.6 22.6 31.2 L27.6 26.5 Q26.2 26.7 25 26.2 L29.7 20.9 Q28.8 21 28 20.6 Z";

impl CompletionIcon {
    pub const ALL: [CompletionIcon; 3] = [CompletionIcon::Wave, CompletionIcon::Pine, CompletionIcon::Reflection];

    pub fn key(self) -> &'static str {
        match self {
            CompletionIcon::Wave => "wave",
            CompletionIcon::Pine => "pine",
            CompletionIcon::Reflection => "reflection",
        }
    }

    pub fn from_key(key: &str) -> Self {
        Self::ALL.into_iter().find(|i| i.key() == key).unwrap_or_default()
    }

    pub fn label(self) -> &'static str {
        match self {
            CompletionIcon::Wave => "Waterline",
            CompletionIcon::Pine => "Pine",
            CompletionIcon::Reflection => "Pine + reflection",
        }
    }

    /// White on the filled circle (`currentColor`), sized by CSS.
    pub fn svg(self) -> String {
        match self {
            CompletionIcon::Wave => r#"<svg class="done-mark" viewBox="0 0 24 24"><path d="M3.5 13 Q7 10 10.5 13 T17.5 13 T22 12" stroke="currentColor" stroke-width="2.6" fill="none" stroke-linecap="round"/></svg>"#.to_string(),
            CompletionIcon::Pine => format!(
                r#"<svg class="done-mark" viewBox="20 12 24 22"><path d="{PINE}" fill="currentColor" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/><rect x="31" y="30" width="2" height="3" rx=".5" fill="currentColor"/></svg>"#
            ),
            CompletionIcon::Reflection => format!(
                r#"<svg class="done-mark" viewBox="16 10 32 44"><g transform="translate(32 15.3) scale(.82) translate(-32 -15)"><path d="{PINE}" fill="currentColor" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/></g><g transform="translate(0 64) scale(1 -1)" opacity=".45"><g transform="translate(32 15.3) scale(.82) translate(-32 -15)"><path d="{PINE}" fill="currentColor"/></g></g><path d="M17 32 Q22 30.6 27 32 T37 32 T47 32" stroke="currentColor" stroke-width="3" fill="none" stroke-linecap="round"/></svg>"#
            ),
        }
    }
}

#[wasm_bindgen(inline_js = r#"
export function __get_icon() {
  try { return localStorage.getItem('completion-icon') || ''; } catch (e) { return ''; }
}
export function __set_icon(k) {
  try { localStorage.setItem('completion-icon', k); } catch (e) {}
}
export function __sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }
"#)]
extern "C" {
    fn __get_icon() -> String;
    fn __set_icon(k: &str);
    fn __sleep(ms: u32) -> js_sys::Promise;
}

/// This device's choice (remembered like the theme).
pub fn saved() -> CompletionIcon {
    CompletionIcon::from_key(&__get_icon())
}

pub fn save(icon: CompletionIcon) {
    __set_icon(icon.key());
}

/// Waits `ms` milliseconds.
pub async fn sleep(ms: u32) {
    let _ = wasm_bindgen_futures::JsFuture::from(__sleep(ms)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_waterline_is_the_default_icon() {
        assert_eq!(CompletionIcon::from_key(""), CompletionIcon::Wave);
        assert_eq!(CompletionIcon::from_key("nonsense"), CompletionIcon::Wave);
        assert_eq!(CompletionIcon::default(), CompletionIcon::Wave);
    }

    #[test]
    fn icons_round_trip_through_their_keys() {
        for icon in CompletionIcon::ALL {
            assert_eq!(CompletionIcon::from_key(icon.key()), icon);
            assert!(icon.svg().starts_with("<svg"));
            assert!(!icon.label().is_empty());
        }
    }

    #[test]
    fn the_switch_off_follows_a_visible_fill() {
        assert!(FILL_MS >= 300 && OFF_MS >= 300);
    }
}
