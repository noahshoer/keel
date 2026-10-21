//! Touch gestures: swipes open and close the sidebar drawer on narrow
//! layouts, and a long press on a task shows its row actions.

use serde::Deserialize;

/// The layout is a drawer below this width (matches the CSS breakpoint).
const NARROW_MAX: f64 = 700.0;
/// A swipe must travel at least this far sideways...
const MIN_DISTANCE: f64 = 60.0;
/// ...and stay mostly horizontal (|dy| under this share of |dx|).
const MAX_SLOPE: f64 = 0.6;

/// One finished touch, as reported by [`SWIPE_JS`].
#[derive(Debug, Clone, Deserialize)]
pub struct Swipe {
    pub dx: f64,
    pub dy: f64,
    /// Viewport width when the touch ended.
    pub width: f64,
    /// It started in a field, the editor, a popover or a dialog.
    pub blocked: bool,
}

/// The drawer state a swipe asks for, if any: right opens, left closes.
pub fn drawer_swipe(s: &Swipe, open: bool) -> Option<bool> {
    if s.blocked || s.width > NARROW_MAX {
        return None;
    }
    if s.dx.abs() < MIN_DISTANCE || s.dy.abs() > s.dx.abs() * MAX_SLOPE {
        return None;
    }
    let want = s.dx > 0.0;
    (want != open).then_some(want)
}

/// Reports each single-finger touch to Rust as a [`Swipe`].
pub const SWIPE_JS: &str = r#"
const BLOCK = 'input, textarea, select, [contenteditable], .editor, .popover, .menu, .modal-scrim';
let start = null;
document.addEventListener('touchstart', (ev) => {
  if (ev.touches.length !== 1) { start = null; return; }
  const t = ev.touches[0];
  start = { x: t.clientX, y: t.clientY, blocked: !!ev.target.closest?.(BLOCK) };
}, { passive: true });
document.addEventListener('touchend', (ev) => {
  if (!start) return;
  const t = ev.changedTouches[0];
  dioxus.send({ dx: t.clientX - start.x, dy: t.clientY - start.y, width: window.innerWidth, blocked: start.blocked });
  start = null;
}, { passive: true });
await new Promise(() => {});
"#;

/// Long press on a task row (touch only). On iPhone it fires
/// `keel-long-press` with the task id, and the native action menu opens
/// (`glass.rs`); the row is highlighted while held. Elsewhere it marks the
/// task `.pressed`, which shows its row actions. A plain tap opens the task,
/// as on desktop; the hover-only reveal is mouse-only (see `style.rs`).
pub const LONG_PRESS_JS: &str = r#"
const HOLD_MS = 450, SLOP = 10;
// After a long press, ignore only a click that follows right away: iOS
// often sends none, and a flag left set would eat the next real tap.
let timer = null, origin = null, pressedAt = 0;
const clear = () => document.querySelectorAll('.task.pressed').forEach((t) => t.classList.remove('pressed'));
document.addEventListener('touchstart', (ev) => {
  const row = ev.target.closest?.('.task-row');
  if (!row || ev.target.closest('.task-actions')) { if (!ev.target.closest?.('.task-actions')) clear(); return; }
  const t = ev.touches[0];
  origin = { x: t.clientX, y: t.clientY };
  timer = setTimeout(() => {
    clear();
    const task = row.closest('.task');
    task?.classList.add('pressed');
    pressedAt = Date.now();
    if (document.body.classList.contains('native-glass') && task?.dataset.taskId) {
      document.dispatchEvent(new CustomEvent('keel-long-press', { detail: task.dataset.taskId }));
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
  if (pressedAt && Date.now() - pressedAt < 700 && !ev.target.closest?.('.task-actions')) {
    ev.stopPropagation(); ev.preventDefault();
  }
  pressedAt = 0;
}, true);
document.addEventListener('contextmenu', (ev) => { if (ev.target.closest?.('.task-row')) ev.preventDefault(); });
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn swipe(dx: f64, dy: f64) -> Swipe {
        Swipe { dx, dy, width: 390.0, blocked: false }
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
    fn wide_layouts_and_blocked_targets_are_ignored() {
        assert_eq!(drawer_swipe(&Swipe { width: 1200.0, ..swipe(120.0, 0.0) }, false), None);
        assert_eq!(drawer_swipe(&Swipe { blocked: true, ..swipe(120.0, 0.0) }, false), None);
    }
}
