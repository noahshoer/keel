//! Touch gestures: swipes open and close the sidebar drawer on narrow
//! layouts, and a long press on a task shows its row actions.

use serde::Deserialize;

/// The layout is a drawer below this width (matches the CSS breakpoint).
const NARROW_MAX: f64 = 700.0;
/// The task view is a bottom sheet below this width, a panel at it and above.
const SHEET_MAX: f64 = 900.0;
/// A swipe must travel at least this far sideways...
const MIN_DISTANCE: f64 = 60.0;
/// ...and stay mostly horizontal (|dy| under this share of |dx|).
const MAX_SLOPE: f64 = 0.6;

/// A swipe down on the task sheet closes it once it travels this far.
const CLOSE_DISTANCE: f64 = 80.0;

/// One finished touch, as reported by [`SWIPE_JS`].
#[derive(Debug, Clone, Deserialize)]
pub struct Swipe {
    pub dx: f64,
    pub dy: f64,
    /// Viewport width when the touch ended.
    pub width: f64,
    /// It started in a field, the editor, a popover or a dialog.
    pub blocked: bool,
    /// It started in the task view's sheet or its scrim (no drawer swipes there).
    #[serde(default)]
    pub in_sheet: bool,
    /// It started on the sheet's grabber or header (a swipe down closes it).
    #[serde(default)]
    pub on_grip: bool,
}

/// The drawer state a swipe asks for, if any: right opens, left closes.
pub fn drawer_swipe(s: &Swipe, open: bool) -> Option<bool> {
    if s.blocked || s.in_sheet || s.width > NARROW_MAX {
        return None;
    }
    if s.dx.abs() < MIN_DISTANCE || s.dy.abs() > s.dx.abs() * MAX_SLOPE {
        return None;
    }
    let want = s.dx > 0.0;
    (want != open).then_some(want)
}

/// A swipe that closes the task sheet: at least [`CLOSE_DISTANCE`] down
/// and mostly vertical (|dx| under [`MAX_SLOPE`] of dy).
pub fn is_close_swipe(dx: f64, dy: f64) -> bool {
    dy >= CLOSE_DISTANCE && dx.abs() <= dy * MAX_SLOPE
}

/// Whether a finished touch closes the task sheet (phone layout only).
pub fn sheet_swipe(s: &Swipe) -> bool {
    !s.blocked && s.on_grip && s.width < SHEET_MAX && is_close_swipe(s.dx, s.dy)
}

/// Reports each single-finger touch to Rust as a [`Swipe`].
pub const SWIPE_JS: &str = r#"
const BLOCK = 'input, textarea, select, [contenteditable], .editor, .popover, .menu, .modal-scrim';
let start = null;
document.addEventListener('touchstart', (ev) => {
  if (ev.touches.length !== 1) { start = null; return; }
  const t = ev.touches[0];
  const at = (sel) => !!ev.target.closest?.(sel);
  start = { x: t.clientX, y: t.clientY, blocked: at(BLOCK), in_sheet: at('.tv-sheet, .tv-scrim'), on_grip: at('.tv-grabber, .tv-head') };
}, { passive: true });
document.addEventListener('touchend', (ev) => {
  if (!start) return;
  const t = ev.changedTouches[0];
  dioxus.send({ ...start, dx: t.clientX - start.x, dy: t.clientY - start.y, width: window.innerWidth });
  start = null;
}, { passive: true });
await new Promise(() => {});
"#;

/// Long press on a task row or a sidebar row (touch only). On iPhone it
/// fires `keel-long-press` with the row's `data-menu-id`, and the native
/// action menu opens (`glass::use_row_menu`); the row is highlighted while
/// held. Elsewhere it marks a task `.pressed`, which shows its row actions.
/// A plain tap opens the row, as on desktop; hover reveals are mouse-only
/// (see `style.rs`).
pub const LONG_PRESS_JS: &str = r#"
const HOLD_MS = 450, SLOP = 10;
// After a long press, ignore only a click that follows right away: iOS
// often sends none, and a flag left set would eat the next real tap.
let timer = null, origin = null, pressedAt = 0;
const ROWS = '.task-row, .nav-item', ACTIONS = '.task-actions, .nav-actions';
const clear = () => document.querySelectorAll('.pressed').forEach((t) => t.classList.remove('pressed'));
document.addEventListener('touchstart', (ev) => {
  const row = ev.target.closest?.(ROWS);
  if (!row || ev.target.closest(ACTIONS)) { if (!ev.target.closest?.(ACTIONS)) clear(); return; }
  const t = ev.touches[0];
  origin = { x: t.clientX, y: t.clientY };
  timer = setTimeout(() => {
    clear();
    const holder = row.closest('[data-menu-id]');
    (row.closest('.task') || row).classList.add('pressed');
    pressedAt = Date.now();
    if (document.body.classList.contains('native-glass') && holder) {
      document.dispatchEvent(new CustomEvent('keel-long-press', { detail: holder.dataset.menuId }));
      setTimeout(clear, 400);
    }
  }, HOLD_MS);
}, { passive: true });
document.addEventListener('touchmove', (ev) => {
  if (!timer || !origin) return;
  const t = ev.touches[0];
  if (Math.abs(t.clientX - origin.x) > SLOP || Math.abs(t.clientY - origin.y) > SLOP) { clearTimeout(timer); timer = null; }
}, { passive: true });
const stop = () => { clearTimeout(timer); timer = null; };
document.addEventListener('touchend', stop, { passive: true });
document.addEventListener('touchcancel', stop, { passive: true });
document.addEventListener('click', (ev) => {
  if (pressedAt && Date.now() - pressedAt < 700 && !ev.target.closest?.(ACTIONS)) {
    ev.stopPropagation(); ev.preventDefault();
  }
  pressedAt = 0;
}, true);
document.addEventListener('contextmenu', (ev) => { if (ev.target.closest?.(ROWS)) ev.preventDefault(); });
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn swipe(dx: f64, dy: f64) -> Swipe {
        Swipe { dx, dy, width: 390.0, blocked: false, in_sheet: false, on_grip: false }
    }

    #[test]
    fn swipe_right_opens_a_closed_drawer() {
        assert_eq!(drawer_swipe(&swipe(120.0, 10.0), false), Some(true));
    }

    #[test]
    fn swipe_left_closes_an_open_drawer() {
        assert_eq!(drawer_swipe(&swipe(-120.0, 10.0), true), Some(false));
    }

    #[test]
    fn same_direction_as_the_state_does_nothing() {
        assert_eq!(drawer_swipe(&swipe(120.0, 0.0), true), None);
        assert_eq!(drawer_swipe(&swipe(-120.0, 0.0), false), None);
    }

    #[test]
    fn short_or_mostly_vertical_moves_are_ignored() {
        assert_eq!(drawer_swipe(&swipe(40.0, 0.0), false), None, "too short");
        assert_eq!(drawer_swipe(&swipe(100.0, 90.0), false), None, "a scroll, not a swipe");
    }

    #[test]
    fn close_swipe_needs_mostly_downward_80px() {
        assert!(is_close_swipe(0.0, 90.0));
        assert!(!is_close_swipe(0.0, 60.0), "too short");
        assert!(!is_close_swipe(90.0, 90.0), "diagonal");
        assert!(!is_close_swipe(0.0, -90.0), "upward");
    }

    #[test]
    fn sheet_closes_from_its_grip_on_the_phone_only() {
        let grip = Swipe { on_grip: true, in_sheet: true, ..swipe(0.0, 120.0) };
        assert!(sheet_swipe(&grip));
        assert!(!sheet_swipe(&Swipe { on_grip: false, ..grip.clone() }), "content scrolls instead");
        assert!(!sheet_swipe(&Swipe { width: 1280.0, ..grip.clone() }), "desktop panel");
        assert_eq!(drawer_swipe(&Swipe { in_sheet: true, ..swipe(120.0, 0.0) }, false), None, "no drawer from the sheet");
    }

    #[test]
    fn a_drag_in_a_blocked_target_never_closes_the_sheet() {
        let grip = Swipe { on_grip: true, in_sheet: true, blocked: true, ..swipe(0.0, 120.0) };
        assert!(!sheet_swipe(&grip), "a drag inside the ••• menu");
    }

    #[test]
    fn a_touch_on_the_scrim_counts_as_in_the_sheet() {
        // So a sideways swipe on the scrim doesn't open the drawer over it.
        assert!(SWIPE_JS.contains("in_sheet: at('.tv-sheet, .tv-scrim')"));
    }

    #[test]
    fn wide_layouts_and_blocked_targets_are_ignored() {
        assert_eq!(drawer_swipe(&Swipe { width: 1200.0, ..swipe(120.0, 0.0) }, false), None);
        assert_eq!(drawer_swipe(&Swipe { blocked: true, ..swipe(120.0, 0.0) }, false), None);
    }
}
