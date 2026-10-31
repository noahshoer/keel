// Pure rules for the phone's gestures: the drawer swipe, the task sheet's
// drag and detents, the keyboard's inset, and the swipe back inside a
// sub-task. Ported from crates/ui/src/gestures.rs with the same numbers.

/** The layout is a drawer below this width (matches the CSS breakpoint). */
export const NARROW_MAX = 700;
/** The task view is a bottom sheet below this width, a panel at it and above. */
export const SHEET_MAX = 900;
/** A swipe must travel at least this far sideways... */
export const MIN_DISTANCE = 60;
/** ...and stay mostly horizontal (|dy| under this share of |dx|). */
export const MAX_SLOPE = 0.6;

/** Released past this far down, the task sheet closes... */
export const CLOSE_DISTANCE = 80;
/** ...or sooner on a downward flick this fast (px per ms)... */
export const FLICK_SPEED = 0.5;
/**
 * ...that travelled at least this far. From the large height, only a flick
 * this fast closes it outright (a normal flick steps down to medium).
 */
export const FLICK_MIN = 16;
export const STRONG_FLICK = 1.5;
/** A touch on the sheet is a tap until it moves this far either way. */
export const DRAG_SLOP = 8;
/** A swipe back starts this close to the sheet's left edge... */
export const EDGE_WIDTH = 24;
/**
 * ...and goes back when released past this share of the width, or on a
 * rightward flick this fast (px per ms) that travelled at least FLICK_MIN x 2.
 */
export const BACK_SHARE = 0.4;
export const BACK_FLICK = 0.5;

/** Seconds the sheet and the swipe back take to settle, and their easing. */
export const SETTLE_S = 0.3;
export const EASE = "cubic-bezier(.32,.72,0,1)";

/** One finished touch. */
export type Swipe = {
  dx: number;
  dy: number;
  /** Viewport width when the touch ended. */
  width: number;
  /** It started in a field, the editor, a popover or a dialog. */
  blocked: boolean;
  /** It started in the task view's sheet or its scrim (no drawer swipes there). */
  inSheet: boolean;
};

/** The drawer state a swipe asks for, if any: right opens, left closes. */
export function drawerSwipe(s: Swipe, open: boolean): boolean | null {
  if (s.blocked || s.inSheet || s.width > NARROW_MAX) return null;
  if (Math.abs(s.dx) < MIN_DISTANCE || Math.abs(s.dy) > Math.abs(s.dx) * MAX_SLOPE) return null;
  const want = s.dx > 0;
  return want !== open ? want : null;
}

/** The sheet's two resting heights on the phone: medium (where it opens) and large. */
export type Detent = "medium" | "large";

/** Where a touch on the task sheet started. */
export type DragStart = {
  /** The sheet's scroll area's `scrollTop` (0 for a touch on the header). */
  scrollTop: number;
  /** In a popover, a menu, the sub-task editor or a dialog. */
  blocked: boolean;
  /** In a text field (the title, the description). */
  inField: boolean;
  /** ...and that field has focus (being typed in). */
  fieldFocused: boolean;
  /** Viewport width. */
  width: number;
  /** The height the sheet was resting at. */
  detent: Detent;
};

/**
 * What a touch on the sheet does, decided once it moves past the slop:
 * pending (still a tap), drag (the sheet follows the finger), scroll (the
 * content scrolls) or ignore (left to the page: taps, sideways pill
 * scrolling, fields, menus).
 */
export type SheetDrag = "pending" | "drag" | "scroll" | "ignore";

function excluded(s: DragStart): boolean {
  return s.blocked || (s.inField && s.fieldFocused) || s.width >= SHEET_MAX;
}

/** A mostly vertical move's effect (see `sheetDragAction`). */
function vertical(s: DragStart, dy: number): SheetDrag {
  if (dy < 0) return s.detent === "medium" ? "drag" : "scroll";
  return s.scrollTop > 0 ? "scroll" : "drag";
}

/**
 * iOS sheet behaviour, so the sheet drags from anywhere on it. Once the touch
 * has moved DRAG_SLOP, a sideways (or exactly diagonal) move is left to the
 * page; a mostly vertical one:
 * - up at medium expands the sheet (even over scrolled content); up at large
 *   scrolls the content;
 * - down drags the sheet when its content is scrolled to the top, and scrolls
 *   the content otherwise.
 *
 * A text field drags too, unless it has focus (then the touch is for the
 * text). Popovers, menus, the editor and the desktop panel never drag.
 */
export function sheetDragAction(s: DragStart, dx: number, dy: number): SheetDrag {
  if (excluded(s)) return "ignore";
  if (Math.abs(dx) < DRAG_SLOP && Math.abs(dy) < DRAG_SLOP) return "pending";
  if (Math.abs(dx) >= Math.abs(dy)) return "ignore";
  return vertical(s, dy);
}

/**
 * While still pending, whether a move cancels the browser's scroll: only a
 * vertical, not-sideways-leaning move that would become a drag (iOS would
 * otherwise start a scroll it won't give back). A move leaning sideways is
 * left alone, so the pill row still scrolls.
 */
export function holdsScroll(s: DragStart, dx: number, dy: number): boolean {
  return !excluded(s) && dy !== 0 && Math.abs(dy) >= Math.abs(dx) && vertical(s, dy) === "drag";
}

/**
 * The sheet's offset below its large height while dragged: it follows the
 * finger from where it started (`start`: 0 at large, the medium offset at
 * medium), never above the large height.
 */
export function dragOffset(start: number, dy: number): number {
  return Math.max(start + dy, 0);
}

/**
 * Where a drag starts, as an offset below the large height: where the sheet
 * is right now if it was still settling (`settling`, its current shift), else
 * its detent's resting place.
 */
export function dragFrom(detent: Detent, settling: number | null, medium: number): number {
  if (settling !== null) return Math.max(settling, 0);
  return detent === "large" ? 0 : Math.max(medium, 0);
}

/** Where a released drag sends the sheet; "close" slides down and the view closes. */
export type Settle = Detent | "close";

/**
 * The release decision. `y` is the sheet's offset below its large height,
 * `medium` the medium height's offset, `velocity` px per ms (positive down).
 * - From medium: down past CLOSE_DISTANCE (unless flung back up) or a
 *   downward flick closes; up a third of the way to large (unless flung back
 *   down) or an upward flick reaches large; otherwise back to medium.
 * - From large: past medium by CLOSE_DISTANCE or a strong flick closes; down
 *   a third of the way to medium (unless flung back up) or a flick settles at
 *   medium; otherwise back to large.
 */
export function sheetSettle(from: Detent, y: number, medium: number, velocity: number): Settle {
  const m = Math.max(medium, 0);
  const flickDown = velocity >= FLICK_SPEED;
  const flickUp = velocity <= -FLICK_SPEED;
  if (from === "medium") {
    const down = y - m;
    const up = m - y;
    if ((down >= CLOSE_DISTANCE && !flickUp) || (flickDown && down >= FLICK_MIN)) return "close";
    if ((up > 0 && up >= m / 3 && !flickDown) || (flickUp && up >= FLICK_MIN)) return "large";
    return "medium";
  }
  if ((y >= m + CLOSE_DISTANCE && !flickUp) || (velocity >= STRONG_FLICK && y >= FLICK_MIN)) return "close";
  if ((y >= m / 3 && !flickUp) || (flickDown && y >= FLICK_MIN)) return "medium";
  return "large";
}

/**
 * Where a settled sheet rests, as an offset below the large height (0 at
 * large). Close isn't a resting place: infinity (the caller slides it off).
 */
export function settleOffset(settle: Settle, medium: number): number {
  if (settle === "large") return 0;
  if (settle === "medium") return Math.max(medium, 0);
  return Number.POSITIVE_INFINITY;
}

/**
 * Whether a keyboard coming up (covering `kb` px) takes the sheet from medium
 * to large, which it then keeps once the keyboard is gone. Phone layout only.
 */
export function keyboardExpands(detent: Detent, width: number, kb: number): boolean {
  return detent === "medium" && width < SHEET_MAX && kb > 0;
}

/**
 * How far up the keyboard keeps the sheet's bottom (`--kb`), from the layout
 * height and the visual viewport. While a keyboard is still coming up (`held`,
 * its height) the viewport hasn't resized yet, so the held height stands in;
 * once it has, the viewport is right, also when WebKit has panned it to
 * reveal a field (the held height knows nothing of that).
 */
export function keyboardInset(clientH: number, vvTop: number, vvH: number, held: number | null): number {
  const resized = vvH < clientH - 0.5;
  if (held !== null && !resized) return Math.max(held, 0);
  return Math.max(clientH - vvTop - vvH, 0);
}

/** Whether a touch at `x` (from the viewport's left) began at the edge. */
export function atEdge(x: number): boolean {
  return x <= EDGE_WIDTH;
}

/** What a touch that began at the left edge of the sheet turns into. */
export type EdgeBack = "pending" | "drag" | "ignore";

/**
 * `can`: the touch began at the edge of a sheet that has a Back (inside a
 * sub-task). Rightward and mostly sideways past the slop is a drag.
 */
export function edgeBackAction(can: boolean, dx: number, dy: number): EdgeBack {
  if (!can) return "ignore";
  if (Math.max(Math.abs(dx), Math.abs(dy)) < DRAG_SLOP) return "pending";
  return dx > 0 && dx > Math.abs(dy) ? "drag" : "ignore";
}

/** How far the content has moved: with the finger, rightward, within the width. */
export function edgeOffset(dx: number, width: number): number {
  return Math.min(Math.max(dx, 0), Math.max(width, 0));
}

/**
 * The release decision: past BACK_SHARE of the width, or a rightward flick
 * (`velocity` px per ms, positive right) that went at least 2 x FLICK_MIN. A
 * leftward fling cancels.
 */
export function edgeBackCommits(dx: number, velocity: number, width: number): boolean {
  if (velocity <= -BACK_FLICK) return false;
  return dx >= width * BACK_SHARE || (velocity >= BACK_FLICK && dx >= FLICK_MIN * 2);
}

/**
 * Speed in px per ms from `[timeStamp, position]` samples, first to last; 0
 * with under two samples or no time elapsed.
 */
export function velocity(samples: Array<[number, number]>): number {
  const a = samples[0];
  const b = samples[samples.length - 1];
  if (a === undefined || b === undefined || samples.length < 2) return 0;
  const dt = b[0] - a[0];
  return dt > 0 ? (b[1] - a[1]) / dt : 0;
}
