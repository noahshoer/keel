//! Our own simple line icons (inline SVG strings, rendered via inner HTML).

use dioxus::prelude::*;

const OPEN: &str = r#"<svg width="SZ" height="SZ" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="SW" stroke-linecap="round" stroke-linejoin="round">"#;

fn svg(size: u32, stroke: &str, body: &str) -> String {
    format!(
        "{}{}</svg>",
        OPEN.replace("SZ", &size.to_string()).replace("SW", stroke),
        body
    )
}

pub fn inbox() -> String {
    svg(20, "1.6", r#"<path d="M4 13h4l1.5 2.5h5L16 13h4"/><path d="M5.5 5h13L20 13v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-5z"/>"#)
}

/// Calendar showing today's day-of-month.
pub fn today(day: u32) -> String {
    svg(
        20,
        "1.6",
        &format!(
            r#"<rect x="4" y="5" width="16" height="15" rx="2"/><path d="M4 9h16"/><text x="12" y="17.6" text-anchor="middle" font-size="7.5" font-weight="600" fill="currentColor" stroke="none" font-family="sans-serif">{day}</text>"#
        ),
    )
}

pub fn upcoming() -> String {
    svg(20, "1.6", r#"<rect x="4" y="5" width="16" height="15" rx="2"/><path d="M4 9h16M8 3v4M16 3v4M8 13h2M12 13h2M8 16.5h2M12 16.5h2"/>"#)
}

pub fn calendar_small() -> String {
    svg(12, "2", r#"<rect x="4" y="5" width="16" height="15" rx="2"/><path d="M4 10h16M8 3v4M16 3v4"/>"#)
}

pub fn clock_small() -> String {
    svg(12, "2", r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>"#)
}

pub fn repeat_small() -> String {
    svg(12, "2", r#"<path d="M17 2l3 3-3 3"/><path d="M4 11V9a4 4 0 0 1 4-4h12"/><path d="M7 22l-3-3 3-3"/><path d="M20 13v2a4 4 0 0 1-4 4H4"/>"#)
}

pub fn trash() -> String {
    svg(18, "1.6", r#"<path d="M5 7h14M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3"/>"#)
}

pub fn plus(size: u32) -> String {
    svg(size, "2", r#"<path d="M12 5v14M5 12h14"/>"#)
}

pub fn check() -> String {
    svg(11, "3", r#"<path d="M5 12l5 5L20 7"/>"#)
}

pub fn flag_small() -> String {
    svg(12, "2", r#"<path d="M5 21V4M5 4h11l-2 4 2 4H5"/>"#)
}

/// Renders an SVG string from this module.
#[component]
pub fn Icon(svg: String) -> Element {
    rsx! { span { class: "icon", dangerous_inner_html: "{svg}" } }
}

pub fn sun() -> String {
    svg(18, "1.6", r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>"#)
}

pub fn moon() -> String {
    svg(18, "1.6", r#"<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/>"#)
}

/// "Follow the system" — a monitor.
pub fn monitor() -> String {
    svg(18, "1.6", r#"<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>"#)
}

pub fn tag_small() -> String {
    svg(12, "2", r#"<path d="M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9z"/><circle cx="7.5" cy="7.5" r="1.3"/>"#)
}

pub fn tag() -> String {
    svg(18, "1.6", r#"<path d="M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9z"/><circle cx="7.5" cy="7.5" r="1.3"/>"#)
}

pub fn pencil() -> String {
    svg(16, "1.6", r#"<path d="M4 20h4L19 9l-4-4L4 16v4z"/><path d="M13.5 6.5l4 4"/>"#)
}

pub fn trash_small() -> String {
    svg(16, "1.6", r#"<path d="M5 7h14M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3"/>"#)
}

pub fn check_circle() -> String {
    svg(16, "1.6", r#"<circle cx="12" cy="12" r="9"/><path d="M8 12.5l2.5 2.5L16 9.5"/>"#)
}

/// Small down-chevron; rotated by CSS when collapsed.
pub fn chevron() -> String {
    svg(14, "2", r#"<path d="M6 9l6 6 6-6"/>"#)
}

/// "Add sub-task": a branch arrow.
pub fn subtask() -> String {
    svg(18, "1.6", r#"<path d="M6 4v8a4 4 0 0 0 4 4h8"/><path d="M15 13l3 3-3 3"/>"#)
}

/// Filter: a funnel.
pub fn funnel() -> String {
    svg(18, "1.6", r#"<path d="M4 5h16l-6 7.5V19l-4-2v-4.5z"/>"#)
}
