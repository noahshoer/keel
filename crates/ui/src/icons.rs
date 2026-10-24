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

pub fn search() -> String {
    svg(20, "1.7", r#"<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/>"#)
}

pub fn target_small() -> String {
    svg(12, "2", r#"<circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="12" r="4.5"/><circle cx="12" cy="12" r="1" fill="currentColor"/>"#)
}

pub fn bell_small() -> String {
    svg(12, "2", r#"<path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>"#)
}

/// Keel's mark: a pine above a waterline at the circle's center, mirrored
/// below it (balance between tree and water), with space around the tree.
/// The tile takes the accent color.
pub fn logo(size: u32) -> String {
    const PINE: &str = "M32 15 L36 20.6 Q35.2 21 34.3 20.9 L39 26.2 Q37.8 26.7 36.4 26.5 L41.4 31.2         Q32 29.6 22.6 31.2 L27.6 26.5 Q26.2 26.7 25 26.2 L29.7 20.9 Q28.8 21 28 20.6 Z";
    let tree = format!(
        r##"<path d="{PINE}" transform="translate(32 15.3) scale(.82) translate(-32 -15)" fill="#fff" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/><rect x="31" y="28" width="2" height="4" rx=".5" fill="#fff"/>"##
    );
    format!(
        r##"<svg width="{size}" height="{size}" viewBox="0 0 64 64" fill="none"><rect width="64" height="64" rx="16" fill="currentColor"/><circle cx="32" cy="32" r="22" stroke="#fff" stroke-width="3"/><g>{tree}</g><g transform="translate(0 64) scale(1 -1)" opacity=".42">{tree}</g><path d="M10.6 32 Q16 30.6 21.5 32 T32.5 32 T43.5 32 T53.4 32" stroke="#fff" stroke-width="3" stroke-linecap="round"/></svg>"##
    )
}

pub fn hash_small() -> String {
    svg(14, "2", r#"<path d="M5 9h14M5 15h14M10 4l-2 16M16 4l-2 16"/>"#)
}

/// A section: a short bar with lines under it.
pub fn section_small() -> String {
    svg(14, "2", r#"<path d="M4 6h16M8 12h12M8 18h12"/>"#)
}

pub fn chevron_left() -> String {
    svg(16, "2", r#"<path d="M15 6l-6 6 6 6"/>"#)
}

pub fn chevron_right() -> String {
    svg(16, "2", r#"<path d="M9 6l6 6-6 6"/>"#)
}

pub fn chevron_down_small() -> String {
    svg(12, "2", r#"<path d="M6 9l6 6 6-6"/>"#)
}

pub fn check_small() -> String {
    svg(14, "2.4", r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#)
}

pub fn sofa() -> String {
    svg(16, "1.8", r#"<path d="M5 11V8a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v3"/><path d="M3 13a2 2 0 0 1 4 0v1h10v-1a2 2 0 0 1 4 0v4a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z"/><path d="M6 18v2M18 18v2"/>"#)
}

pub fn arrow_right() -> String {
    svg(16, "1.8", r#"<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><path d="M8 12h8M13 9l3 3-3 3"/>"#)
}

/// The ✕ inside the search field's clear button.
pub fn x_small() -> String {
    svg(12, "2.4", r#"<path d="M7 7l10 10M17 7L7 17"/>"#)
}

pub fn no_date() -> String {
    svg(16, "1.8", r#"<circle cx="12" cy="12" r="8.5"/><path d="M6 6l12 12"/>"#)
}

pub fn gear() -> String {
    svg(18, "1.6", r#"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>"#)
}

/// Hamburger: opens the sidebar drawer at narrow widths.
pub fn menu() -> String {
    svg(20, "1.8", r#"<path d="M4 7h16M4 12h16M4 17h16"/>"#)
}

/// Filter: a funnel.
pub fn funnel() -> String {
    svg(18, "1.6", r#"<path d="M4 5h16l-6 7.5V19l-4-2v-4.5z"/>"#)
}
