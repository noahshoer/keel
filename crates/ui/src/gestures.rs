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
/// ...that travelled at least this far. From the large height, only a
/// flick this fast closes it outright (a normal flick steps down to medium).
const FLICK_MIN: f64 = 16.0;
const STRONG_FLICK: f64 = 1.5;
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
    /// The height the sheet was resting at.
    pub detent: Detent,
}

/// The sheet's two resting heights on the phone: medium (67% of the
/// screen, where it opens) and large (nearly full screen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detent {
    Medium,
    Large,
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

/// iOS sheet behaviour, so the sheet drags from anywhere on it. Once the
/// touch has moved [`DRAG_SLOP`], a sideways (or exactly diagonal) move is
/// left to the page; a mostly vertical one:
/// - up at medium expands the sheet (even over scrolled content); up at
///   large scrolls the content;
/// - down drags the sheet when its content is scrolled to the top, and
///   scrolls the content otherwise.
///
/// A text field drags too, unless it has focus (then the touch is for the
/// text). Popovers, menus, the editor and the desktop panel never drag.
pub fn sheet_drag_action(s: &DragStart, dx: f64, dy: f64) -> SheetDrag {
    if excluded(s) {
        return SheetDrag::Ignore;
    }
    if dx.abs() < DRAG_SLOP && dy.abs() < DRAG_SLOP {
        return SheetDrag::Pending;
    }
    if dx.abs() >= dy.abs() {
        return SheetDrag::Ignore;
    }
    vertical(s, dy)
}

fn excluded(s: &DragStart) -> bool {
    s.blocked || (s.in_field && s.field_focused) || s.width >= SHEET_MAX
}

/// A mostly vertical move's effect (see [`sheet_drag_action`]).
fn vertical(s: &DragStart, dy: f64) -> SheetDrag {
    match (dy < 0.0, s.detent) {
        (true, Detent::Medium) => SheetDrag::Drag,
        (true, Detent::Large) => SheetDrag::Scroll,
        (false, _) if s.scroll_top > 0.0 => SheetDrag::Scroll,
        (false, _) => SheetDrag::Drag,
    }
}

/// While still [`SheetDrag::Pending`], whether a move cancels the browser's
/// scroll: only a vertical, not-sideways-leaning move that would become a
/// drag (iOS would otherwise start a scroll it won't give back). A move
/// leaning sideways is left alone, so the pill row still scrolls.
pub fn holds_scroll(s: &DragStart, dx: f64, dy: f64) -> bool {
    !excluded(s) && dy != 0.0 && dy.abs() >= dx.abs() && vertical(s, dy) == SheetDrag::Drag
}

/// The sheet's offset below its large height while dragged: it follows the
/// finger from where it started (`start`: 0 at large, the medium offset at
/// medium), never above the large height.
pub fn drag_offset(start: f64, dy: f64) -> f64 {
    (start + dy).max(0.0)
}

/// Where a drag starts, as an offset below the large height: where the sheet
/// is right now if it was still settling (`settling`, its current shift),
/// else its detent's resting place.
pub fn drag_from(detent: Detent, settling: Option<f64>, medium: f64) -> f64 {
    match (settling, detent) {
        (Some(shift), _) => shift.max(0.0),
        (None, Detent::Large) => 0.0,
        (None, Detent::Medium) => medium.max(0.0),
    }
}

/// Where a released drag sends the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settle {
    Large,
    Medium,
    /// Slides down and the view closes (saving a pending edit first).
    Close,
}

/// The release decision. `y` is the sheet's offset below its large height,
/// `medium` the medium height's offset, `velocity` px per ms (positive down).
/// - From medium: down past [`CLOSE_DISTANCE`] (unless flung back up) or a
///   downward flick closes; up a third of the way to large (unless flung
///   back down) or an upward flick reaches large; otherwise back to medium.
/// - From large: past medium by [`CLOSE_DISTANCE`] or a strong flick closes;
///   down a third of the way to medium (unless flung back up) or a flick
///   settles at medium; otherwise back to large.
pub fn sheet_settle(from: Detent, y: f64, medium: f64, velocity: f64) -> Settle {
    let medium = medium.max(0.0);
    let flick_down = velocity >= FLICK_SPEED;
    let flick_up = velocity <= -FLICK_SPEED;
    match from {
        Detent::Medium => {
            let (down, up) = (y - medium, medium - y);
            if (down >= CLOSE_DISTANCE && !flick_up) || (flick_down && down >= FLICK_MIN) {
                Settle::Close
            } else if (up > 0.0 && up >= medium / 3.0 && !flick_down) || (flick_up && up >= FLICK_MIN) {
                Settle::Large
            } else {
                Settle::Medium
            }
        }
        Detent::Large => {
            if (y >= medium + CLOSE_DISTANCE && !flick_up) || (velocity >= STRONG_FLICK && y >= FLICK_MIN) {
                Settle::Close
            } else if (y >= medium / 3.0 && !flick_up) || (flick_down && y >= FLICK_MIN) {
                Settle::Medium
            } else {
                Settle::Large
            }
        }
    }
}

/// Where a settled sheet rests, as an offset below the large height (0 at
/// large). Close isn't a resting place: infinity (the caller slides it off).
pub fn settle_offset(settle: Settle, medium: f64) -> f64 {
    match settle {
        Settle::Large => 0.0,
        Settle::Medium => medium.max(0.0),
        Settle::Close => f64::INFINITY,
    }
}

/// Exposes the sheet's rules to [`SHEET_DRAG_JS`] as `window.keelSheet`:
/// - `decide(start, dx, dy)`: "pending" / "drag" / "scroll" / "ignore"
///   ([`sheet_drag_action`]); `hold(start, dx, dy)`: bool ([`holds_scroll`]).
///   `start` is `[scrollTop, blocked, inField, fieldFocused, width, detent]`.
/// - `drag(start, dy)`: [`drag_offset`]; `from(detent, settling, medium)`:
///   [`drag_from`] (`settling` is the current shift, or null at rest).
/// - `settle(detent, y, medium, velocity)`: "large" / "medium" / "close"
///   ([`sheet_settle`]); `rest(settle, medium)`: [`settle_offset`].
/// - `glass(rects, sheetTop, vvLeft, vvTop)`: the glass X and •••
///   ([`crate::glass::sheet_glass`]).
///
/// `detent` / `settle` are "medium" / "large" / "close". The touch handlers
/// need the answers synchronously (to cancel the browser's scroll), so this
/// can't wait for a message round trip.
pub fn install_sheet_rules() {
    use crate::glass::{sheet_glass, WebRect};
    use wasm_bindgen::prelude::*;
    fn detent(v: &JsValue) -> Detent {
        if v.as_string().as_deref() == Some("large") {
            Detent::Large
        } else {
            Detent::Medium
        }
    }
    fn start(a: &js_sys::Array) -> DragStart {
        let n = |i| a.get(i).as_f64().unwrap_or(0.0);
        let b = |i| a.get(i).as_bool().unwrap_or(false);
        DragStart {
            scroll_top: n(0),
            blocked: b(1),
            in_field: b(2),
            field_focused: b(3),
            width: n(4),
            detent: detent(&a.get(5)),
        }
    }
    fn name(s: Settle) -> String {
        match s {
            Settle::Large => "large",
            Settle::Medium => "medium",
            Settle::Close => "close",
        }
        .to_string()
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
    let drag = Closure::<dyn Fn(f64, f64) -> f64>::new(drag_offset);
    let from = Closure::<dyn Fn(JsValue, JsValue, f64) -> f64>::new(|d: JsValue, s: JsValue, medium| {
        drag_from(detent(&d), s.as_f64(), medium)
    });
    let settle = Closure::<dyn Fn(JsValue, f64, f64, f64) -> String>::new(|d: JsValue, y, medium, v| {
        name(sheet_settle(detent(&d), y, medium, v))
    });
    let rest = Closure::<dyn Fn(JsValue, f64) -> f64>::new(|s: JsValue, medium| {
        let s = match s.as_string().as_deref() {
            Some("large") => Settle::Large,
            Some("medium") => Settle::Medium,
            _ => Settle::Close,
        };
        settle_offset(s, medium)
    });
    let glass = Closure::<dyn Fn(JsValue, f64, f64, f64) -> JsValue>::new(|rects: JsValue, top, left, vv_top| {
        let rects: Vec<WebRect> = serde_wasm_bindgen::from_value(rects).unwrap_or_default();
        let s = serde_wasm_bindgen::Serializer::json_compatible();
        serde::Serialize::serialize(&sheet_glass(&rects, top, left, vv_top), &s).unwrap_or(JsValue::NULL)
    });
    let rules = js_sys::Object::new();
    let set = |k: &str, f: &JsValue| {
        let _ = js_sys::Reflect::set(&rules, &k.into(), f);
    };
    set("decide", decide.as_ref());
    set("hold", hold.as_ref());
    set("drag", drag.as_ref());
    set("from", from.as_ref());
    set("settle", settle.as_ref());
    set("rest", rest.as_ref());
    set("glass", glass.as_ref());
    let _ = js_sys::Reflect::set(&js_sys::global(), &"keelSheet".into(), &rules);
    decide.forget();
    hold.forget();
    drag.forget();
    from.forget();
    settle.forget();
    rest.forget();
    glass.forget();
}

/// What [`SHEET_DRAG_JS`] reports.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SheetMsg {
    /// A drag was released past the threshold and the sheet slid away.
    #[serde(default)]
    pub close: bool,
}

/// The task sheet follows the finger between its two heights (phone layout).
/// A touch anywhere on the sheet is classified by `keelSheet.decide` once it
/// passes the slop (until then `keelSheet.hold` says whether to cancel the
/// browser's scroll). A drag switches the sheet to its large layout
/// (`.tv-large`) and moves it with `translateY` below that, so it can grow
/// and shrink with the finger; on release `keelSheet.settle` picks large,
/// medium (the 67% layout, swapped back in once it has settled there) or
/// close (it slides off, then `{close}` goes to Rust, which closes the view
/// through its save-first path). Reduced motion snaps.
///
/// iPhone: the glass X and ••• (`plugin:glass|sheet`) are sent once, as the
/// sheet opens. Swift puts them inside the web buttons' own layers, so they
/// move with the sheet by themselves (slide, drag, settle, keyboard).
///
/// The keyboard: `--vv-top`, `--vv-h` and `--kb` (on the root) keep the
/// sheet inside the visible area, above the keyboard, header in view.
/// Slides the task sheet down off the screen (phone layout); the page then
/// asks the view to close, as a drag past the threshold does. False on the
/// desktop panel, which the caller closes at once.
pub async fn slide_close() -> bool {
    dioxus::prelude::document::eval("dioxus.send(window.keelSheetSlideClose?.() === true);")
        .recv::<bool>()
        .await
        .unwrap_or(false)
}

pub const SHEET_DRAG_JS: &str = r#"
const BLOCK = '.popover, .menu, .editor, .modal-scrim, .pop-backdrop';
const FIELD = 'input, textarea, select, [contenteditable]';
const EASE = 'cubic-bezier(.32,.72,0,1)';
const SETTLE_S = 0.3;
const reduce = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
const sheetEl = () => document.querySelector('.tv-sheet');
const scrim = () => document.querySelector('.tv-scrim');
const vvTop = () => window.visualViewport?.offsetTop ?? 0;
const native = () => document.body.classList.contains('native-glass');
// If showing the glass fails, the web X and ••• come back.
const send = (m) => {
  if (!native()) return;
  window.__TAURI__?.core.invoke('plugin:glass|sheet', m)
    .catch(() => { if (m.buttons) sheetEl()?.classList.remove('glass-head'); });
};
const shiftOf = (el) => new DOMMatrix(getComputedStyle(el).transform).m42 || 0;
const measureGlass = (sheet) => {
  const r = sheet.getBoundingClientRect();
  const rects = [['tv-close', '.tv-head .tv-close'], ['tv-back', '.tv-head .tv-back'], ['tv-more', '.tv-head .tv-more']].map(([id, sel]) => {
    const b = sheet.querySelector(sel)?.getBoundingClientRect();
    return b && b.width > 0 && { id, x: b.x, y: b.y, w: b.width, h: b.height };
  }).filter(Boolean);
  return keelSheet.glass(rects, r.top, window.visualViewport?.offsetLeft ?? 0, vvTop());
};
window.keelSheetOpened = (tries = 0) => {
  const sheet = sheetEl();
  if (!window.keelSheet) return;
  const glass = sheet && measureGlass(sheet);
  // Not measurable: the web X and ••• stay (they're hidden only under glass-head).
  if (!glass || glass.buttons.length < 2) {
    if (tries < 10) requestAnimationFrame(() => keelSheetOpened(tries + 1));
    return;
  }
  sheet.dataset.detent = 'medium';
  sheet.classList.add('glass-head');
  send({ buttons: glass.buttons });
};
// A sub-task's view pushed or popped: the ‹ comes or goes.
window.keelSheetGlassUpdate = () => requestAnimationFrame(() => {
  const sheet = sheetEl();
  if (!sheet || !sheet.classList.contains('glass-head')) return;
  const glass = measureGlass(sheet);
  if (glass && glass.buttons.length >= 2) send({ buttons: glass.buttons });
});
// Both layouts' tops (no transform): `M` is how far medium sits below large.
const heights = (sheet) => {
  const was = sheet.classList.contains('tv-large'), keep = sheet.style.transform;
  sheet.style.transform = 'none';
  sheet.classList.remove('tv-large');
  const m = sheet.getBoundingClientRect();
  sheet.classList.add('tv-large');
  const l = sheet.getBoundingClientRect();
  if (!was) sheet.classList.remove('tv-large');
  sheet.style.transform = keep;
  return { M: m.top - l.top, largeTop: l.top, mediumH: m.height };
};
const place = (d, y, s) => {
  const t = s > 0 ? `transform ${s}s ${EASE}` : 'none';
  d.sheet.style.transition = t;
  d.sheet.style.transform = `translateY(${y}px)`;
  const sc = scrim();
  if (sc) {
    sc.style.transition = s > 0 ? `opacity ${s}s ${EASE}` : 'none';
    sc.style.opacity = String(Math.max(0, Math.min(1, 1 - (y - Math.max(0, d.M)) / d.mediumH)));
  }
};
let g = null, seq = 0;
document.addEventListener('touchstart', (ev) => {
  g = null;
  if (ev.touches.length !== 1 || !window.keelSheet) return;
  const sheet = ev.target.closest?.('.tv-sheet');
  if (!sheet) return;
  const t = ev.touches[0];
  const field = ev.target.closest(FIELD);
  const scroller = ev.target.closest('.tv-scroll');
  g = { sheet, x: t.clientX, y: t.clientY, mode: 'pending', samples: [],
    start: [scroller ? scroller.scrollTop : 0, !!ev.target.closest(BLOCK), !!field,
      !!field && field.contains(document.activeElement), innerWidth, sheet.dataset.detent || 'medium'] };
}, { passive: true, capture: true });
document.addEventListener('touchmove', (ev) => {
  if (!g || g.mode === 'scroll' || g.mode === 'ignore') return;
  const t = ev.touches[0];
  const dx = t.clientX - g.x, dy = t.clientY - g.y;
  if (g.mode === 'pending') {
    g.mode = keelSheet.decide(g.start, dx, dy);
    if (g.mode === 'pending' && keelSheet.hold(g.start, dx, dy)) ev.preventDefault();
    if (g.mode !== 'drag') return;
    seq++;
    // Grabbed while settling: freeze it where it is (its pending settle is
    // dropped by `seq`) and drag from there.
    const sheet = g.sheet, settling = sheet.classList.contains('tv-large') ? shiftOf(sheet) : null;
    sheet.style.transition = 'none';
    if (settling !== null) sheet.style.transform = `translateY(${settling}px)`;
    sheet.getAnimations().forEach((a) => a.finish());
    Object.assign(g, heights(sheet), { dy0: dy });
    g.from = keelSheet.from(g.start[5], settling, g.M);
    sheet.classList.add('tv-large');
  }
  ev.preventDefault();
  g.yNow = keelSheet.drag(g.from, dy - g.dy0);
  g.samples.push([ev.timeStamp, dy]);
  if (g.samples.length > 6) g.samples.shift();
  place(g, g.yNow, 0);
}, { passive: false, capture: true });
const end = (cancelled) => {
  const d = g;
  g = null;
  if (!d || d.mode !== 'drag') return;
  const s = d.samples, a = s[0], b = s[s.length - 1];
  const v = a && b && b[0] > a[0] ? (b[1] - a[1]) / (b[0] - a[0]) : 0;
  const to = cancelled ? d.start[5] : keelSheet.settle(d.start[5], d.yNow, d.M, v);
  const secs = reduce() ? 0 : SETTLE_S;
  // Close slides it just past the bottom of the screen.
  const target = to === 'close' ? innerHeight - d.largeTop + 40 : keelSheet.rest(to, d.M);
  place(d, target, secs);
  const mine = seq;
  if (to !== 'close') d.sheet.dataset.detent = to;
  setTimeout(() => {
    if (to === 'close') { dioxus.send({ close: true }); return; }
    if (mine !== seq || !d.sheet.isConnected) return;
    if (to === 'medium') d.sheet.classList.remove('tv-large');
    d.sheet.style.transition = 'none';
    d.sheet.style.transform = '';
    const sc = scrim();
    if (sc) { sc.style.transition = 'none'; sc.style.opacity = ''; }
  }, secs * 1000);
};
// The X and the scrim: the sheet slides down off the screen, then closes the
// way a drag past the threshold does (`{close}`). False on the desktop panel
// (at least 900px wide), which closes at once.
let closing = false;
window.keelSheetSlideClose = () => {
  const sheet = sheetEl();
  if (!sheet || !window.keelSheet || innerWidth >= 900) return false;
  if (closing) return true;
  closing = true;
  seq++;
  g = null;
  const d = { sheet, ...heights(sheet) };
  sheet.getAnimations().forEach((a) => a.finish());
  const settling = sheet.classList.contains('tv-large') ? shiftOf(sheet) : null;
  const from = keelSheet.from(sheet.dataset.detent || 'medium', settling, d.M);
  sheet.classList.add('tv-large');
  place(d, from, 0);
  sheet.getBoundingClientRect();
  const secs = reduce() ? 0 : SETTLE_S;
  place(d, innerHeight - d.largeTop + 40, secs);
  setTimeout(() => { closing = false; dioxus.send({ close: true }); }, secs * 1000);
  return true;
};
document.addEventListener('touchend', () => end(false), { passive: true, capture: true });
document.addEventListener('touchcancel', () => end(true), { passive: true, capture: true });
// The keyboard: keep the sheet in the visible area above it.
// Only on iPhone at 1x: a desktop pinch-zoom mustn't lift the sheet.
const viewport = () => {
  const v = window.visualViewport, root = document.documentElement;
  for (const k of ['--vv-top', '--vv-h', '--kb']) root.style.removeProperty(k);
  if (v && native() && v.scale === 1) {
    root.style.setProperty('--vv-top', `${v.offsetTop}px`);
    root.style.setProperty('--vv-h', `${v.height}px`);
    root.style.setProperty('--kb', `${Math.max(0, root.clientHeight - v.offsetTop - v.height)}px`);
  }
};
window.keelSheetViewport = viewport;
window.visualViewport?.addEventListener('resize', viewport);
window.visualViewport?.addEventListener('scroll', viewport);
window.addEventListener('resize', viewport);
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
        DragStart { scroll_top, blocked: false, in_field: false, field_focused: false, width: 390.0, detent: Detent::Medium }
    }

    fn large(scroll_top: f64) -> DragStart {
        DragStart { detent: Detent::Large, ..start(scroll_top) }
    }

    #[test]
    fn a_downward_drag_at_the_top_moves_the_sheet() {
        assert_eq!(sheet_drag_action(&start(0.0), 2.0, 9.0), SheetDrag::Drag);
        assert_eq!(sheet_drag_action(&large(0.0), 2.0, 9.0), SheetDrag::Drag);
    }

    #[test]
    fn scrolled_content_scrolls_first_on_the_way_down() {
        assert_eq!(sheet_drag_action(&start(40.0), 0.0, 9.0), SheetDrag::Scroll);
        assert_eq!(sheet_drag_action(&large(40.0), 0.0, 9.0), SheetDrag::Scroll);
    }

    #[test]
    fn an_upward_drag_at_medium_expands_the_sheet() {
        // iOS: the sheet grows to its large height before the content scrolls.
        assert_eq!(sheet_drag_action(&start(0.0), 0.0, -9.0), SheetDrag::Drag);
        assert_eq!(sheet_drag_action(&start(40.0), 0.0, -9.0), SheetDrag::Drag, "even over scrolled content");
    }

    #[test]
    fn an_upward_drag_at_large_scrolls_the_content() {
        assert_eq!(sheet_drag_action(&large(0.0), 0.0, -9.0), SheetDrag::Scroll);
    }

    #[test]
    fn a_sideways_move_is_left_alone() {
        // The pill row scrolls sideways, rows and circles keep their taps.
        assert_eq!(sheet_drag_action(&start(0.0), 12.0, 5.0), SheetDrag::Ignore);
        assert_eq!(sheet_drag_action(&start(0.0), -12.0, -5.0), SheetDrag::Ignore, "also sideways-and-up at medium");
        assert_eq!(sheet_drag_action(&large(0.0), -12.0, 5.0), SheetDrag::Ignore);
    }

    #[test]
    fn nothing_is_decided_inside_the_slop() {
        // A jittery tap on a row, circle or pill stays a tap.
        assert_eq!(sheet_drag_action(&start(0.0), 2.0, 7.9), SheetDrag::Pending);
        assert_eq!(sheet_drag_action(&start(0.0), -7.9, 3.0), SheetDrag::Pending);
        assert_eq!(sheet_drag_action(&start(0.0), 0.0, -7.9), SheetDrag::Pending);
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
        assert_eq!(sheet_drag_action(&start(0.0), 10.0, -10.0), SheetDrag::Ignore);
    }

    #[test]
    fn an_unfocused_field_drags_and_a_focused_one_does_not() {
        // "Drag from anywhere": the title and description drag the sheet
        // until they're being typed in.
        let unfocused = DragStart { in_field: true, ..start(0.0) };
        assert_eq!(sheet_drag_action(&unfocused, 0.0, 12.0), SheetDrag::Drag);
        let focused = DragStart { in_field: true, field_focused: true, ..start(0.0) };
        assert_eq!(sheet_drag_action(&focused, 0.0, 12.0), SheetDrag::Ignore);
        assert_eq!(sheet_drag_action(&focused, 0.0, -12.0), SheetDrag::Ignore);
    }

    #[test]
    fn inputs_menus_and_the_desktop_panel_never_drag() {
        let blocked = DragStart { blocked: true, ..start(0.0) };
        assert_eq!(sheet_drag_action(&blocked, 0.0, 9.0), SheetDrag::Ignore, "inside the ••• menu or a popover");
        let wide = DragStart { width: 1280.0, ..start(0.0) };
        assert_eq!(sheet_drag_action(&wide, 0.0, 9.0), SheetDrag::Ignore, "desktop panel");
        assert_eq!(sheet_drag_action(&wide, 0.0, -9.0), SheetDrag::Ignore, "desktop panel");
    }

    #[test]
    fn a_pending_move_holds_the_scroll_only_when_it_could_drag() {
        // So iOS doesn't start a scroll before the drag is decided.
        assert!(holds_scroll(&start(0.0), 1.0, 3.0));
        assert!(holds_scroll(&start(40.0), 1.0, -3.0), "upward at medium expands");
        assert!(!holds_scroll(&start(0.0), 4.0, 3.0), "leaning sideways: the pills may scroll");
        assert!(!holds_scroll(&start(20.0), 0.0, 3.0), "scrolled content scrolls");
        assert!(!holds_scroll(&large(0.0), 0.0, -3.0), "upward at large scrolls");
        assert!(!holds_scroll(&DragStart { blocked: true, ..start(0.0) }, 0.0, 3.0));
        assert!(!holds_scroll(&start(0.0), 0.0, 0.0));
    }

    #[test]
    fn the_sheet_follows_the_finger_but_not_above_large() {
        assert_eq!(drag_offset(300.0, 50.0), 350.0);
        assert_eq!(drag_offset(300.0, -120.0), 180.0);
        assert_eq!(drag_offset(300.0, -400.0), 0.0, "stops at the large height");
        assert_eq!(drag_offset(0.0, -30.0), 0.0);
    }

    #[test]
    fn a_drag_starts_where_the_sheet_is() {
        assert_eq!(drag_from(Detent::Medium, None, 300.0), 300.0, "at rest at medium");
        assert_eq!(drag_from(Detent::Large, None, 300.0), 0.0, "at rest at large");
        // Grabbed again while still settling: from where it is right now,
        // not from the detent it was heading to.
        assert_eq!(drag_from(Detent::Medium, Some(140.0), 300.0), 140.0);
        assert_eq!(drag_from(Detent::Large, Some(-3.0), 300.0), 0.0);
        assert_eq!(drag_from(Detent::Medium, None, -20.0), 0.0, "keyboard up: medium meets large");
    }

    // The medium detent sits 300px below the large one in these tests.
    const M: f64 = 300.0;

    #[test]
    fn from_medium_a_drag_down_closes_past_80px_or_on_a_flick() {
        assert_eq!(sheet_settle(Detent::Medium, M + 80.0, M, 0.0), Settle::Close, "exactly 80px");
        assert_eq!(sheet_settle(Detent::Medium, M + 79.0, M, 0.0), Settle::Medium);
        assert_eq!(sheet_settle(Detent::Medium, M + 40.0, M, 0.8), Settle::Close, "a fast flick down");
        assert_eq!(sheet_settle(Detent::Medium, M + 6.0, M, 0.8), Settle::Medium, "a twitch, not a flick");
        assert_eq!(sheet_settle(Detent::Medium, M + 120.0, M, -0.6), Settle::Medium, "flung back up");
    }

    #[test]
    fn from_medium_a_drag_up_reaches_large_past_a_third_or_on_a_flick() {
        assert_eq!(sheet_settle(Detent::Medium, M - 100.0, M, 0.0), Settle::Large, "a third of the way");
        assert_eq!(sheet_settle(Detent::Medium, M - 99.0, M, 0.0), Settle::Medium);
        assert_eq!(sheet_settle(Detent::Medium, M - 40.0, M, -0.8), Settle::Large, "an upward flick");
        assert_eq!(sheet_settle(Detent::Medium, M - 200.0, M, 0.8), Settle::Medium, "flung back down");
    }

    #[test]
    fn from_large_a_short_drag_settles_at_medium_and_a_long_one_closes() {
        assert_eq!(sheet_settle(Detent::Large, 100.0, M, 0.0), Settle::Medium, "a third of the way to medium");
        assert_eq!(sheet_settle(Detent::Large, 99.0, M, 0.0), Settle::Large, "barely moved");
        assert_eq!(sheet_settle(Detent::Large, 40.0, M, 0.8), Settle::Medium, "a flick: one step down");
        assert_eq!(sheet_settle(Detent::Large, M + 79.0, M, 0.0), Settle::Medium, "short of medium + 80px");
        assert_eq!(sheet_settle(Detent::Large, M + 80.0, M, 0.0), Settle::Close, "past medium by 80px");
        assert_eq!(sheet_settle(Detent::Large, 60.0, M, 2.0), Settle::Close, "a strong flick");
        assert_eq!(sheet_settle(Detent::Large, 200.0, M, -0.8), Settle::Large, "flung back up");
    }

    #[test]
    fn settle_positions() {
        assert_eq!(settle_offset(Settle::Large, M), 0.0);
        assert_eq!(settle_offset(Settle::Medium, M), M);
        // With the keyboard up the two heights can meet: medium is never above large.
        assert_eq!(settle_offset(Settle::Medium, -20.0), 0.0);
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
