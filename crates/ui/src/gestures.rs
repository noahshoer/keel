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
/// A swipe back starts this close to the sheet's left edge...
const EDGE_WIDTH: f64 = 24.0;
/// ...and goes back when released past this share of the width, or on a
/// rightward flick this fast (px per ms) that travelled at least [`FLICK_MIN`] x 2.
const BACK_SHARE: f64 = 0.4;
const BACK_FLICK: f64 = 0.5;

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

/// Whether a keyboard coming up (covering `kb` px) takes the sheet from
/// medium to large, which it then keeps once the keyboard is gone. Phone
/// layout only.
pub fn keyboard_expands(detent: Detent, width: f64, kb: f64) -> bool {
    detent == Detent::Medium && width < SHEET_MAX && kb > 0.0
}

/// How far up the keyboard keeps the sheet's bottom (`--kb`), from the
/// layout height and the visual viewport. While a keyboard is still coming
/// up (`held`, its height) the viewport hasn't resized yet, so the held
/// height stands in; once it has, the viewport is right, also when WebKit
/// has panned it to reveal a field (the held height knows nothing of that).
pub fn keyboard_inset(client_h: f64, vv_top: f64, vv_h: f64, held: Option<f64>) -> f64 {
    let resized = vv_h < client_h - 0.5;
    match held {
        Some(kb) if !resized => kb.max(0.0),
        _ => (client_h - vv_top - vv_h).max(0.0),
    }
}

/// Whether a touch at `x` (from the viewport's left) began at the edge.
pub fn at_edge(x: f64) -> bool {
    x <= EDGE_WIDTH
}

/// What a touch that began at the left edge of the sheet turns into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeBack {
    /// Not far enough to say.
    Pending,
    /// The content follows the finger.
    Drag,
    /// Not a swipe back (vertical, leftward, or no Back to go to).
    Ignore,
}

/// `can`: the touch began at the edge of a sheet that has a Back (inside a
/// sub-task). Rightward and mostly sideways past the slop is a drag.
pub fn edge_back_action(can: bool, _x: f64, dx: f64, dy: f64) -> EdgeBack {
    if !can {
        return EdgeBack::Ignore;
    }
    if dx.abs().max(dy.abs()) < DRAG_SLOP {
        return EdgeBack::Pending;
    }
    if dx > 0.0 && dx > dy.abs() {
        EdgeBack::Drag
    } else {
        EdgeBack::Ignore
    }
}

/// How far the content has moved: with the finger, rightward, within the width.
pub fn edge_offset(dx: f64, width: f64) -> f64 {
    dx.clamp(0.0, width.max(0.0))
}

/// The release decision: past [`BACK_SHARE`] of the width, or a rightward
/// flick (`velocity` px per ms, positive right) that went at least 2 x
/// [`FLICK_MIN`]. A leftward fling cancels.
pub fn edge_back_commits(dx: f64, velocity: f64, width: f64) -> bool {
    if velocity <= -BACK_FLICK {
        return false;
    }
    dx >= width * BACK_SHARE || (velocity >= BACK_FLICK && dx >= FLICK_MIN * 2.0)
}

/// Exposes the sheet's rules to [`SHEET_DRAG_JS`] as `window.keelSheet`:
/// - `decide(start, dx, dy)`: "pending" / "drag" / "scroll" / "ignore"
///   ([`sheet_drag_action`]); `hold(start, dx, dy)`: bool ([`holds_scroll`]).
///   `start` is `[scrollTop, blocked, inField, fieldFocused, width, detent]`.
/// - `drag(start, dy)`: [`drag_offset`]; `from(detent, settling, medium)`:
///   [`drag_from`] (`settling` is the current shift, or null at rest).
/// - `settle(detent, y, medium, velocity)`: "large" / "medium" / "close"
///   ([`sheet_settle`]); `rest(settle, medium)`: [`settle_offset`].
/// - `edge(can, x, dx, dy)`: "pending" / "drag" / "ignore" ([`edge_back_action`]);
///   `atEdge(x)`: [`at_edge`]; `edgeOffset(dx, width)`: [`edge_offset`];
///   `edgeCommits(dx, velocity, width)`: [`edge_back_commits`].
/// - `kbExpand(detent, width, kb)`: [`keyboard_expands`];
///   `kbInset(clientH, vvTop, vvH, held)`: [`keyboard_inset`] (`held` or null).
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
    let edge = Closure::<dyn Fn(bool, f64, f64, f64) -> String>::new(|can, x, dx, dy| {
        match edge_back_action(can, x, dx, dy) {
            EdgeBack::Pending => "pending",
            EdgeBack::Drag => "drag",
            EdgeBack::Ignore => "ignore",
        }
        .to_string()
    });
    let at = Closure::<dyn Fn(f64) -> bool>::new(at_edge);
    let offset = Closure::<dyn Fn(f64, f64) -> f64>::new(edge_offset);
    let commits = Closure::<dyn Fn(f64, f64, f64) -> bool>::new(edge_back_commits);
    let kb_expand = Closure::<dyn Fn(JsValue, f64, f64) -> bool>::new(|d: JsValue, width, kb| {
        keyboard_expands(detent(&d), width, kb)
    });
    let kb_inset = Closure::<dyn Fn(f64, f64, f64, JsValue) -> f64>::new(|h, top, vh, held: JsValue| {
        keyboard_inset(h, top, vh, held.as_f64())
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
    set("edge", edge.as_ref());
    set("atEdge", at.as_ref());
    set("edgeOffset", offset.as_ref());
    set("edgeCommits", commits.as_ref());
    set("kbExpand", kb_expand.as_ref());
    set("kbInset", kb_inset.as_ref());
    set("glass", glass.as_ref());
    let _ = js_sys::Reflect::set(&js_sys::global(), &"keelSheet".into(), &rules);
    decide.forget();
    hold.forget();
    drag.forget();
    from.forget();
    settle.forget();
    rest.forget();
    edge.forget();
    at.forget();
    offset.forget();
    commits.forget();
    kb_expand.forget();
    kb_inset.forget();
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
/// sheet inside the visible area, above the keyboard, header in view. The
/// visual viewport only resizes once the keyboard has arrived, so Swift
/// (`KeyboardPace`) calls `keelKeyboard` as it starts: the sheet's bottom
/// rises with it, on its curve and at its pace, and a medium sheet goes to
/// large alongside ([`keyboard_expands`]) and stays there.
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
// Swipe in from the left edge inside a sub-task: the content follows the
// finger; released past the threshold (or flicked) it slides off and the
// Back button is clicked (so a pending edit saves, as for a tap on ‹),
// otherwise it springs back. Only where a Back exists.
let e = null;
// The view under (`.tv-under`, mounted but hidden) comes in from a third of
// a width to the left, as in iOS, while the top view's content leaves.
const topBody = (sheet) => sheet.querySelector('.tv-stack > .task-view:not(.tv-under) > .tv-body');
const under = (sheet) => sheet.querySelector('.tv-stack > .tv-under');
const edgeMove = (d, x, secs) => {
  const t = secs > 0 ? `transform ${secs}s ${EASE}` : 'none';
  const share = Math.min(1, x / innerWidth);
  const body = topBody(d.sheet), u = under(d.sheet), ub = u?.querySelector(':scope > .tv-body');
  if (body) {
    body.style.transition = t;
    body.style.transform = x ? `translateX(${x}px)` : '';
    body.style.boxShadow = x ? '-6px 0 18px rgba(0,0,0,.12)' : '';
  }
  if (u) u.style.visibility = x ? 'visible' : '';
  if (ub) {
    ub.style.transition = t;
    ub.style.transform = x ? `translateX(${-innerWidth / 3 * (1 - share)}px)` : '';
  }
};
document.addEventListener('touchstart', (ev) => {
  e = null;
  if (ev.touches.length !== 1 || !window.keelSheet) return;
  const sheet = ev.target.closest?.('.tv-sheet');
  const t = ev.touches[0];
  if (!sheet || innerWidth >= 900) return;
  const can = keelSheet.atEdge(t.clientX) && !!sheet.querySelector('.tv-back') && !ev.target.closest(BLOCK);
  if (can) e = { sheet, x: t.clientX, y: t.clientY, mode: 'pending', samples: [] };
}, { passive: true, capture: true });
document.addEventListener('touchmove', (ev) => {
  if (!e || e.mode === 'ignore') return;
  const t = ev.touches[0], dx = t.clientX - e.x, dy = t.clientY - e.y;
  if (e.mode === 'pending') {
    e.mode = keelSheet.edge(true, e.x, dx, dy);
    if (e.mode !== 'drag') { if (e.mode === 'ignore') e = null; return; }
    // The vertical sheet drag must not also take this touch.
    if (typeof g !== 'undefined') g = null;
  }
  ev.preventDefault();
  ev.stopPropagation();
  e.dx = keelSheet.edgeOffset(dx, innerWidth);
  e.samples.push([ev.timeStamp, dx]);
  if (e.samples.length > 6) e.samples.shift();
  edgeMove(e, e.dx, 0);
}, { passive: false, capture: true });
const edgeEnd = (cancelled, at) => {
  const d = e;
  e = null;
  if (!d || d.mode !== 'drag') return;
  // The lift counts as a sample, so a pause before it isn't read as a flick.
  d.samples.push([at, d.dx]);
  const s = d.samples, a = s[0], b = s[s.length - 1];
  const v = a && b && b[0] > a[0] ? (b[1] - a[1]) / (b[0] - a[0]) : 0;
  const secs = reduce() ? 0 : SETTLE_S;
  if (cancelled || !keelSheet.edgeCommits(d.dx, v, innerWidth)) { edgeMove(d, 0, secs); return; }
  edgeMove(d, innerWidth, secs);
  // Back keeps the view under mounted and makes it the top one. It stays
  // shown until it has lost `.tv-under`, then the swipe's styles go (it's at
  // rest by now, so nothing moves). The leaving view stays off to the right
  // until it's removed.
  const u = under(d.sheet), ub = u?.querySelector(':scope > .tv-body');
  const settle = (frames) => {
    if (u && u.isConnected && u.classList.contains('tv-under') && frames > 0) {
      requestAnimationFrame(() => settle(frames - 1));
      return;
    }
    if (u) u.style.visibility = '';
    if (ub) { ub.style.transition = 'none'; ub.style.transform = ''; }
  };
  setTimeout(() => {
    d.sheet.querySelector('.tv-back')?.click();
    settle(120);
  }, secs * 1000);
};
document.addEventListener('touchend', (ev) => edgeEnd(false, ev.timeStamp), { passive: true, capture: true });
document.addEventListener('touchcancel', (ev) => edgeEnd(true, ev.timeStamp), { passive: true, capture: true });
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
// `kbHold`: the height of a keyboard still coming up, standing in until the
// visual viewport (resized only once it's up) has caught up (`kbInset`).
let kbHold = null;
const viewport = () => {
  const v = window.visualViewport, root = document.documentElement;
  for (const k of ['--vv-top', '--vv-h', '--kb']) root.style.removeProperty(k);
  if (v && native() && v.scale === 1) {
    const held = kbHold && performance.now() < kbHold.until ? kbHold.kb : null;
    root.style.setProperty('--vv-top', `${v.offsetTop}px`);
    root.style.setProperty('--vv-h', `${v.height}px`);
    root.style.setProperty('--kb', `${keelSheet.kbInset(root.clientHeight, v.offsetTop, v.height, held)}px`);
  }
};
// Swift, as the keyboard starts to move (`kb` px of the web view it will
// cover, 0 as it goes; `secs` and `ease` its pace and curve). The sheet's
// bottom follows it both ways; coming up, a medium sheet also slides up to
// large from wherever it is, and stays large once the keyboard has gone.
window.keelKeyboard = ({ kb, secs, ease }) => {
  if (!native()) return;
  const sheet = sheetEl(), s = reduce() ? 0 : secs;
  kbHold = kb > 0 ? { kb, until: performance.now() + s * 1000 + 500 } : null;
  if (!sheet || !window.keelSheet || closing) { viewport(); return; }
  const bottom = s > 0 ? `bottom ${s}s ${ease}` : 'none';
  if (!keelSheet.kbExpand(sheet.dataset.detent || 'medium', innerWidth, kb)) {
    // Large already, or the keyboard going: only the bottom moves (a
    // settle's own transform, if any, carries on).
    const t = sheet.style.transition;
    sheet.style.transition = t && t !== 'none' && s > 0 ? `${t}, ${bottom}` : bottom;
    viewport();
    return;
  }
  seq++;
  g = null;
  const d = { sheet, ...heights(sheet) };
  sheet.getAnimations().forEach((a) => a.finish());
  const settling = sheet.classList.contains('tv-large') ? shiftOf(sheet) : null;
  sheet.dataset.detent = 'large';
  sheet.classList.add('tv-large');
  place(d, keelSheet.from('medium', settling, d.M), 0);
  sheet.getBoundingClientRect();
  sheet.style.transition = s > 0 ? `transform ${s}s ${ease}, ${bottom}` : 'none';
  sheet.style.transform = 'translateY(0px)';
  viewport();
  const mine = seq;
  setTimeout(() => {
    if (mine !== seq || !sheet.isConnected) return;
    sheet.style.transition = 'none';
    sheet.style.transform = '';
    const sc = scrim();
    if (sc) { sc.style.transition = 'none'; sc.style.opacity = ''; }
  }, s * 1000);
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
    fn the_keyboard_takes_a_medium_sheet_to_large() {
        assert!(keyboard_expands(Detent::Medium, 390.0, 328.0));
        assert!(!keyboard_expands(Detent::Large, 390.0, 328.0), "already large");
        assert!(!keyboard_expands(Detent::Medium, 1024.0, 328.0), "the desktop panel has no detents");
        assert!(!keyboard_expands(Detent::Medium, 390.0, 0.0), "a hardware keyboard covers nothing");
    }

    #[test]
    fn a_keyboard_coming_up_holds_until_the_viewport_catches_up() {
        // Not resized yet: the keyboard's height.
        assert_eq!(keyboard_inset(874.0, 0.0, 874.0, Some(328.0)), 328.0);
        // Resized: the viewport's answer, even with no hold.
        assert_eq!(keyboard_inset(874.0, 0.0, 546.0, None), 328.0);
        assert_eq!(keyboard_inset(874.0, 0.0, 874.0, None), 0.0);
    }

    #[test]
    fn a_panned_viewport_wins_over_the_keyboards_height() {
        // The description field: WebKit pans 192 px and the keyboard (with
        // its form bar) covers 396; the sheet ends at the keyboard, 204 up.
        assert_eq!(keyboard_inset(874.0, 192.0, 478.0, Some(396.0)), 204.0);
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
    fn an_edge_swipe_right_starts_only_at_the_left_edge_with_a_back() {
        assert_eq!(edge_back_action(true, 4.0, 14.0, 2.0), EdgeBack::Drag);
        assert_eq!(edge_back_action(true, 4.0, 5.0, 2.0), EdgeBack::Pending, "inside the slop");
        assert_eq!(edge_back_action(false, 4.0, 14.0, 2.0), EdgeBack::Ignore, "not from the edge, or no Back");
    }

    #[test]
    fn an_edge_touch_that_goes_vertical_or_left_is_not_a_back() {
        assert_eq!(edge_back_action(true, 4.0, 3.0, 14.0), EdgeBack::Ignore, "vertical: the sheet drag's");
        assert_eq!(edge_back_action(true, 4.0, -14.0, 2.0), EdgeBack::Ignore, "leftward");
        assert_eq!(edge_back_action(true, 4.0, 10.0, 10.0), EdgeBack::Ignore, "a diagonal tie");
    }

    #[test]
    fn only_touches_that_begin_at_the_edge_count() {
        assert!(at_edge(0.0) && at_edge(EDGE_WIDTH));
        assert!(!at_edge(EDGE_WIDTH + 1.0));
    }

    #[test]
    fn release_past_the_threshold_or_with_a_flick_goes_back() {
        assert!(edge_back_commits(390.0 * 0.4, 0.0, 390.0));
        assert!(!edge_back_commits(390.0 * 0.4 - 1.0, 0.0, 390.0));
        assert!(edge_back_commits(40.0, 0.6, 390.0), "a flick");
        assert!(!edge_back_commits(10.0, 0.9, 390.0), "too short even when fast");
        assert!(!edge_back_commits(250.0, -0.6, 390.0), "flung back to the left");
    }

    #[test]
    fn the_content_follows_the_finger_to_the_right_only() {
        assert_eq!(edge_offset(90.0, 390.0), 90.0);
        assert_eq!(edge_offset(-20.0, 390.0), 0.0);
        assert_eq!(edge_offset(500.0, 390.0), 390.0);
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
