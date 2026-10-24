//! Touch gestures: swipes open and close the sidebar drawer on narrow
//! layouts, the task sheet follows a drag down, and a long press on a task
//! shows its row actions.

use serde::Deserialize;

/// The layout is a drawer below this width (matches the CSS breakpoint).
const NARROW_MAX: f64 = 700.0;
/// The task view is a bottom sheet below this width, a panel at it and above.
const SHEET_MAX: f64 = 900.0;
/// A swipe must travel at least this far sideways...
const MIN_DISTANCE: f64 = 60.0;
/// ...and stay mostly horizontal (|dy| under this share of |dx|).
const MAX_SLOPE: f64 = 0.6;

/// Released past this far down, the task sheet closes...
const CLOSE_DISTANCE: f64 = 80.0;
/// ...or sooner on a downward flick this fast (px per ms)...
const FLICK_SPEED: f64 = 0.5;
/// ...that travelled at least this far.
const FLICK_MIN: f64 = 16.0;
/// A touch on the sheet is a tap until it moves this far either way.
const DRAG_SLOP: f64 = 8.0;

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

/// Where a touch on the task sheet started, as [`SHEET_DRAG_JS`] sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragStart {
    /// The sheet's scroll area's `scrollTop` (0 for a touch on the header).
    pub scroll_top: f64,
    /// In a popover, a menu, the sub-task editor or a dialog.
    pub blocked: bool,
    /// In a text field (the title, the description).
    pub in_field: bool,
    /// ...and that field has focus (being typed in).
    pub field_focused: bool,
    /// Viewport width.
    pub width: f64,
}

/// What a touch on the sheet does, decided once it moves past the slop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetDrag {
    /// Not moved past the slop yet (still a tap).
    Pending,
    /// The sheet follows the finger.
    Drag,
    /// The content scrolls (the browser's own scrolling).
    Scroll,
    /// Left to the page: taps, sideways pill scrolling, fields, menus.
    Ignore,
}

/// iOS sheet behaviour, so the sheet drags from anywhere on it: once the
/// touch has moved [`DRAG_SLOP`], a mostly downward move drags the sheet
/// when its content is scrolled to the top; scrolled content scrolls first;
/// a sideways (or exactly diagonal) move is left to the page. A text field
/// drags too, unless it has focus (then the touch is for the text). Popovers,
/// menus, the editor and the desktop panel never drag.
pub fn sheet_drag_action(s: &DragStart, dx: f64, dy: f64) -> SheetDrag {
    if s.blocked || (s.in_field && s.field_focused) || s.width >= SHEET_MAX {
        return SheetDrag::Ignore;
    }
    if dx.abs() < DRAG_SLOP && dy.abs() < DRAG_SLOP {
        return SheetDrag::Pending;
    }
    if dx.abs() >= dy.abs() {
        return SheetDrag::Ignore;
    }
    if dy < 0.0 || s.scroll_top > 0.0 {
        return SheetDrag::Scroll;
    }
    SheetDrag::Drag
}

/// While still [`SheetDrag::Pending`], whether a move cancels the browser's
/// scroll: only a downward, not-sideways-leaning move that could become a
/// drag (iOS would otherwise start an overscroll it won't give back).
pub fn holds_scroll(s: &DragStart, dx: f64, dy: f64) -> bool {
    !s.blocked
        && !(s.in_field && s.field_focused)
        && s.width < SHEET_MAX
        && s.scroll_top <= 0.0
        && dy > 0.0
        && dy >= dx.abs()
}

/// Where a released drag sends the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetRelease {
    /// Slides down and the view closes (saving a pending edit first).
    Close,
    /// Back to its resting place.
    SpringBack,
}

/// Closes past [`CLOSE_DISTANCE`] (unless flung back up) or on a fast
/// downward flick; `velocity` is px per ms, positive downward.
pub fn sheet_release(dy: f64, velocity: f64) -> SheetRelease {
    let far = dy >= CLOSE_DISTANCE && velocity > -FLICK_SPEED;
    let flick = velocity >= FLICK_SPEED && dy >= FLICK_MIN;
    if far || flick {
        SheetRelease::Close
    } else {
        SheetRelease::SpringBack
    }
}

/// Exposes [`sheet_drag_action`], [`holds_scroll`] and [`sheet_release`] to
/// [`SHEET_DRAG_JS`] as `window.keelSheet.decide(start, dx, dy)` ("pending" /
/// "drag" / "scroll" / "ignore"), `keelSheet.hold(start, dx, dy)` (bool), where
/// `start` is `[scrollTop, blocked, inField, fieldFocused, width]`, and
/// `window.keelSheet.release(dy, velocity)` ("close" / "spring"). The touch
/// handlers need the answer synchronously (to cancel the browser's scroll),
/// so this can't wait for a message round trip.
pub fn install_sheet_rules() {
    use wasm_bindgen::prelude::*;
    fn start(a: &js_sys::Array) -> DragStart {
        let n = |i| a.get(i).as_f64().unwrap_or(0.0);
        let b = |i| a.get(i).as_bool().unwrap_or(false);
        DragStart { scroll_top: n(0), blocked: b(1), in_field: b(2), field_focused: b(3), width: n(4) }
    }
    let decide = Closure::<dyn Fn(js_sys::Array, f64, f64) -> String>::new(|a: js_sys::Array, dx, dy| {
        match sheet_drag_action(&start(&a), dx, dy) {
            SheetDrag::Pending => "pending",
            SheetDrag::Drag => "drag",
            SheetDrag::Scroll => "scroll",
            SheetDrag::Ignore => "ignore",
        }
        .to_string()
    });
    let hold = Closure::<dyn Fn(js_sys::Array, f64, f64) -> bool>::new(|a: js_sys::Array, dx, dy| {
        holds_scroll(&start(&a), dx, dy)
    });
    let release = Closure::<dyn Fn(f64, f64) -> String>::new(|dy, velocity| {
        match sheet_release(dy, velocity) {
            SheetRelease::Close => "close",
            SheetRelease::SpringBack => "spring",
        }
        .to_string()
    });
    let rules = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&rules, &"decide".into(), decide.as_ref());
    let _ = js_sys::Reflect::set(&rules, &"hold".into(), hold.as_ref());
    let _ = js_sys::Reflect::set(&rules, &"release".into(), release.as_ref());
    let _ = js_sys::Reflect::set(&js_sys::global(), &"keelSheet".into(), &rules);
    decide.forget();
    hold.forget();
    release.forget();
}

/// What [`SHEET_DRAG_JS`] reports.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SheetMsg {
    /// A drag was released past the threshold and the sheet slid away.
    #[serde(default)]
    pub close: bool,
    /// The sheet started moving (a drag, or the keyboard shifting the page).
    #[serde(default)]
    pub moving: bool,
    /// The sheet is at rest: where its header's X and ••• are now.
    #[serde(default)]
    pub rects: Option<Vec<crate::glass::WebRect>>,
}

/// The task sheet follows the finger (phone layout). A touch anywhere on
/// the sheet is classified by `keelSheet.decide` once it passes the slop
/// (until then `keelSheet.hold` says whether to cancel the browser's scroll);
/// a drag cancels the browser's scroll and moves the sheet (and fades the scrim)
/// with the finger, never above its resting place. On release
/// `keelSheet.release` either slides it away and reports `close` (Rust
/// then closes the view through its save-first path) or springs it back.
/// Reduced motion snaps instead of animating. It also reports `moving` and,
/// once at rest again, the header buttons' `rects` (for the iPhone's glass
/// X and •••); `window.keelMeasureSheet()` measures on demand.
pub const SHEET_DRAG_JS: &str = r#"
const BLOCK = '.popover, .menu, .editor, .modal-scrim, .pop-backdrop';
const FIELD = 'input, textarea, select, [contenteditable]';
const EASE = 'cubic-bezier(.32,.72,0,1)';
const reduce = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
const scrim = () => document.querySelector('.tv-scrim');
const measure = () => [['tv-close', '.tv-sheet .tv-head .tv-close'], ['tv-more', '.tv-sheet .tv-head .tv-more']].map(([id, sel]) => {
  const r = document.querySelector(sel)?.getBoundingClientRect();
  // On screen: the keyboard can pan the visual viewport over the layout one.
  const v = window.visualViewport;
  return r && r.width > 0 && { id, x: r.x - (v?.offsetLeft ?? 0), y: r.y - (v?.offsetTop ?? 0), w: r.width, h: r.height };
}).filter(Boolean);
// Where the sheet is on screen: its layout rect plus the visual viewport's
// pan, zoom and height (the keyboard moves those, not the layout rect).
const rectKey = (el) => {
  const r = el.getBoundingClientRect(), v = window.visualViewport;
  const pan = v ? [v.offsetLeft, v.offsetTop, v.scale * 100, v.height] : [];
  return [r.x, r.y, r.width, r.height, ...pan].map(Math.round).join();
};
window.keelSheetKey = () => { const s = document.querySelector('.tv-sheet'); return s ? rectKey(s) : ''; };
let lastRect = '';
const settle = async () => {
  const sheet = document.querySelector('.tv-sheet');
  if (!sheet) return;
  await Promise.all(sheet.getAnimations().map((a) => a.finished.catch(() => {})));
  if (!sheet.isConnected || g?.mode === 'drag') return;
  lastRect = rectKey(sheet);
  dioxus.send({ rects: measure() });
};
window.keelMeasureSheet = settle;
let g = null;
const place = (sheet, y, animate) => {
  const t = animate && !reduce() ? `transform .3s ${EASE}` : 'none';
  sheet.style.transition = t;
  sheet.style.transform = y > 0 ? `translateY(${y}px)` : '';
  const s = scrim();
  if (s) {
    s.style.transition = t === 'none' ? 'none' : `opacity .3s ${EASE}`;
    s.style.opacity = y > 0 ? String(Math.max(0, 1 - y / sheet.offsetHeight)) : '';
  }
};
document.addEventListener('touchstart', (ev) => {
  g = null;
  if (ev.touches.length !== 1 || !window.keelSheet) return;
  const sheet = ev.target.closest?.('.tv-sheet');
  if (!sheet) return;
  const t = ev.touches[0];
  const field = ev.target.closest(FIELD);
  const scroller = ev.target.closest('.tv-scroll');
  g = { sheet, x: t.clientX, y: t.clientY, mode: 'pending', dy: 0, samples: [],
    start: [scroller ? scroller.scrollTop : 0, !!ev.target.closest(BLOCK), !!field, !!field && field.contains(document.activeElement), innerWidth] };
}, { passive: true, capture: true });
document.addEventListener('touchmove', (ev) => {
  if (!g || g.mode === 'scroll' || g.mode === 'ignore') return;
  const t = ev.touches[0];
  const dx = t.clientX - g.x, dy = t.clientY - g.y;
  if (g.mode === 'pending') {
    g.mode = keelSheet.decide(g.start, dx, dy);
    if (g.mode === 'pending' && keelSheet.hold(g.start, dx, dy)) ev.preventDefault();
    if (g.mode !== 'drag') return;
    g.sheet.getAnimations().forEach((a) => a.finish());
    dioxus.send({ moving: true });
  }
  ev.preventDefault();
  g.dy = Math.max(0, dy);
  g.samples.push([ev.timeStamp, dy]);
  if (g.samples.length > 6) g.samples.shift();
  place(g.sheet, g.dy, false);
}, { passive: false, capture: true });
const end = (ev, cancelled) => {
  const d = g;
  g = null;
  if (!d || d.mode !== 'drag') return;
  const s = d.samples, a = s[0], b = s[s.length - 1];
  const v = a && b && b[0] > a[0] ? (b[1] - a[1]) / (b[0] - a[0]) : 0;
  if (!cancelled && keelSheet.release(d.dy, v) === 'close') {
    place(d.sheet, d.sheet.offsetHeight, true);
    setTimeout(() => dioxus.send({ close: true }), reduce() ? 0 : 300);
  } else {
    place(d.sheet, 0, true);
    setTimeout(settle, reduce() ? 0 : 320);
  }
};
document.addEventListener('touchend', (ev) => end(ev, false), { passive: true, capture: true });
document.addEventListener('touchcancel', (ev) => end(ev, true), { passive: true, capture: true });
// The keyboard shifts the page under the glass buttons: hide them, then
// measure again once it has stopped moving.
let moved = null;
const shifted = () => {
  const sheet = document.querySelector('.tv-sheet');
  if (!sheet) return;
  const key = rectKey(sheet);
  if (key === lastRect) return;
  lastRect = key;
  dioxus.send({ moving: true });
  clearTimeout(moved);
  moved = setTimeout(settle, 250);
};
window.visualViewport?.addEventListener('resize', shifted);
window.visualViewport?.addEventListener('scroll', shifted);
window.addEventListener('resize', shifted);
await new Promise(() => {});
"#;

/// Reports each single-finger touch to Rust as a [`Swipe`].
pub const SWIPE_JS: &str = r#"
const BLOCK = 'input, textarea, select, [contenteditable], .editor, .popover, .menu, .modal-scrim';
let start = null;
document.addEventListener('touchstart', (ev) => {
  if (ev.touches.length !== 1) { start = null; return; }
  const t = ev.touches[0];
  const at = (sel) => !!ev.target.closest?.(sel);
  start = { x: t.clientX, y: t.clientY, blocked: at(BLOCK), in_sheet: at('.tv-sheet, .tv-scrim') };
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
        Swipe { dx, dy, width: 390.0, blocked: false, in_sheet: false }
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
    fn no_drawer_swipe_from_the_sheet() {
        assert_eq!(drawer_swipe(&Swipe { in_sheet: true, ..swipe(120.0, 0.0) }, false), None);
    }

    fn start(scroll_top: f64) -> DragStart {
        DragStart { scroll_top, blocked: false, in_field: false, field_focused: false, width: 390.0 }
    }

    #[test]
    fn a_downward_drag_at_the_top_moves_the_sheet() {
        assert_eq!(sheet_drag_action(&start(0.0), 2.0, 9.0), SheetDrag::Drag);
    }

    #[test]
    fn nothing_is_decided_inside_the_slop() {
        // A jittery tap on a row, circle or pill stays a tap.
        assert_eq!(sheet_drag_action(&start(0.0), 2.0, 7.9), SheetDrag::Pending);
        assert_eq!(sheet_drag_action(&start(0.0), -7.9, 3.0), SheetDrag::Pending);
        assert_eq!(sheet_drag_action(&start(0.0), 0.0, 8.0), SheetDrag::Drag, "past the slop");
    }

    #[test]
    fn a_sideways_swipe_that_starts_downward_still_goes_sideways() {
        // The first samples look vertical but under the slop, so nothing is decided.
        assert_eq!(sheet_drag_action(&start(0.0), 1.0, 3.0), SheetDrag::Pending);
        assert_eq!(sheet_drag_action(&start(0.0), 14.0, 5.0), SheetDrag::Ignore);
    }

    #[test]
    fn a_diagonal_tie_is_not_a_drag() {
        assert_eq!(sheet_drag_action(&start(0.0), 10.0, 10.0), SheetDrag::Ignore);
    }

    #[test]
    fn an_unfocused_field_drags_and_a_focused_one_does_not() {
        // "Drag from anywhere": the title and description drag the sheet
        // until they're being typed in.
        let unfocused = DragStart { in_field: true, ..start(0.0) };
        assert_eq!(sheet_drag_action(&unfocused, 0.0, 12.0), SheetDrag::Drag);
        let focused = DragStart { in_field: true, field_focused: true, ..start(0.0) };
        assert_eq!(sheet_drag_action(&focused, 0.0, 12.0), SheetDrag::Ignore);
    }

    #[test]
    fn a_pending_downward_move_at_the_top_holds_the_scroll() {
        // So iOS doesn't start a scroll before the drag is decided.
        assert!(holds_scroll(&start(0.0), 1.0, 3.0));
        assert!(!holds_scroll(&start(0.0), 4.0, 3.0), "leaning sideways: the pills may scroll");
        assert!(!holds_scroll(&start(20.0), 0.0, 3.0), "scrolled content scrolls");
        assert!(!holds_scroll(&start(0.0), 0.0, -3.0), "upward scrolls");
        assert!(!holds_scroll(&DragStart { blocked: true, ..start(0.0) }, 0.0, 3.0));
    }

    #[test]
    fn scrolled_content_scrolls_first() {
        assert_eq!(sheet_drag_action(&start(40.0), 0.0, 9.0), SheetDrag::Scroll);
    }

    #[test]
    fn an_upward_move_scrolls_the_content() {
        assert_eq!(sheet_drag_action(&start(0.0), 0.0, -9.0), SheetDrag::Scroll);
    }

    #[test]
    fn a_sideways_move_is_left_alone() {
        // The pill row scrolls sideways, rows and circles keep their taps.
        assert_eq!(sheet_drag_action(&start(0.0), 12.0, 5.0), SheetDrag::Ignore);
        assert_eq!(sheet_drag_action(&start(0.0), -12.0, 5.0), SheetDrag::Ignore);
    }

    #[test]
    fn no_movement_yet_waits() {
        assert_eq!(sheet_drag_action(&start(0.0), 0.0, 0.0), SheetDrag::Pending);
    }

    #[test]
    fn inputs_menus_and_the_desktop_panel_never_drag() {
        let blocked = DragStart { blocked: true, ..start(0.0) };
        assert_eq!(sheet_drag_action(&blocked, 0.0, 9.0), SheetDrag::Ignore, "inside the ••• menu or a popover");
        let typing = DragStart { in_field: true, field_focused: true, ..start(0.0) };
        assert_eq!(sheet_drag_action(&typing, 0.0, 9.0), SheetDrag::Ignore, "in a field being edited");
        let wide = DragStart { width: 1280.0, ..start(0.0) };
        assert_eq!(sheet_drag_action(&wide, 0.0, 9.0), SheetDrag::Ignore, "desktop panel");
    }

    #[test]
    fn release_closes_past_80px_or_on_a_flick() {
        assert_eq!(sheet_release(90.0, 0.0), SheetRelease::Close);
        assert_eq!(sheet_release(80.0, 0.0), SheetRelease::Close, "exactly 80px");
        assert_eq!(sheet_release(79.0, 0.0), SheetRelease::SpringBack);
        assert_eq!(sheet_release(60.0, 0.1), SheetRelease::SpringBack, "short and slow");
        assert_eq!(sheet_release(40.0, 0.8), SheetRelease::Close, "a fast flick down");
        assert_eq!(sheet_release(6.0, 0.8), SheetRelease::SpringBack, "a twitch, not a flick");
        assert_eq!(sheet_release(120.0, -0.6), SheetRelease::SpringBack, "flung back up");
    }

    #[test]
    fn the_keyboard_panning_the_viewport_counts_as_moving_the_sheet() {
        // The layout rect doesn't change when the keyboard pans the visual
        // viewport; the glass buttons must still be measured again.
        assert!(SHEET_DRAG_JS.contains("[v.offsetLeft, v.offsetTop, v.scale * 100, v.height]"));
        assert!(SHEET_DRAG_JS.contains("y: r.y - (v?.offsetTop ?? 0)"));
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
