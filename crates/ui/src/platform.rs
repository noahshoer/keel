//! Which kind of device the UI runs on, for text that names keys.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Platform {
    /// A phone or tablet: no keyboard shortcuts to mention.
    Touch,
    Mac,
    /// Windows, Linux.
    Other,
}

/// Classifies from `navigator.platform`, `navigator.maxTouchPoints` and
/// the user agent.
pub fn detect(platform: &str, touch_points: u32, user_agent: &str) -> Platform {
    let mobile = ["iPhone", "iPad", "iPod", "Android"];
    if mobile.iter().any(|m| platform.contains(m) || user_agent.contains(m)) {
        return Platform::Touch;
    }
    if platform.starts_with("Mac") {
        // iPadOS reports a Mac but has touch points; a Mac has none.
        return if touch_points > 1 { Platform::Touch } else { Platform::Mac };
    }
    Platform::Other
}

/// This device's platform (read from the navigator once).
pub fn current() -> Platform {
    thread_local! {
        static CURRENT: Platform = {
            let nav = js_sys::Reflect::get(&js_sys::global(), &"navigator".into()).ok();
            let field = |name: &str| {
                nav.as_ref().and_then(|n| js_sys::Reflect::get(n, &name.into()).ok())
            };
            detect(
                &field("platform").and_then(|v| v.as_string()).unwrap_or_default(),
                field("maxTouchPoints").and_then(|v| v.as_f64()).unwrap_or(0.0) as u32,
                &field("userAgent").and_then(|v| v.as_string()).unwrap_or_default(),
            )
        };
    }
    CURRENT.with(|p| *p)
}

/// The search page's hint: the shortcut only where there are keys.
pub fn search_hint(p: Platform) -> &'static str {
    match p {
        Platform::Touch => "Every word must match, in any order.",
        Platform::Mac => "Every word must match, in any order. Press / or ⌘K anytime to search.",
        Platform::Other => "Every word must match, in any order. Press / or Ctrl+K anytime to search.",
    }
}

/// The search button's tooltip.
pub fn search_tooltip(p: Platform) -> &'static str {
    match p {
        Platform::Touch => "Search",
        Platform::Mac => "Search (/ or ⌘K)",
        Platform::Other => "Search (/ or Ctrl+K)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_platform_from_the_navigator() {
        assert_eq!(detect("iPhone", 5, ""), Platform::Touch);
        // iPadOS reports a Mac; its touch points give it away.
        assert_eq!(detect("MacIntel", 5, ""), Platform::Touch);
        assert_eq!(detect("Linux armv8l", 5, "Mozilla/5.0 (Linux; Android 15)"), Platform::Touch);
        assert_eq!(detect("MacIntel", 0, ""), Platform::Mac);
        assert_eq!(detect("Win32", 0, ""), Platform::Other);
        assert_eq!(detect("Linux x86_64", 0, ""), Platform::Other);
    }

    #[test]
    fn the_search_hint_names_this_platforms_keys() {
        assert_eq!(search_hint(Platform::Touch), "Every word must match, in any order.");
        assert_eq!(search_hint(Platform::Mac), "Every word must match, in any order. Press / or ⌘K anytime to search.");
        assert_eq!(search_hint(Platform::Other), "Every word must match, in any order. Press / or Ctrl+K anytime to search.");
        assert_eq!(search_tooltip(Platform::Touch), "Search");
        assert_eq!(search_tooltip(Platform::Mac), "Search (/ or ⌘K)");
        assert_eq!(search_tooltip(Platform::Other), "Search (/ or Ctrl+K)");
    }
}
