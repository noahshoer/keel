// The task sheet's touch handling on the phone (crates/ui/src/gestures.rs
// `SHEET_DRAG_JS`, without the glass X and •••, which are parked):
// - The vertical drag: a touch anywhere on the sheet is classified by
//   `sheetDragAction` once it passes the slop (until then `holdsScroll` says
//   whether to cancel the browser's scroll). A drag switches the sheet to its
//   large layout (`.tv-large`) and moves it with `translateY` below that, so it
//   can grow and shrink with the finger; on release `sheetSettle` picks large,
//   medium (the 67% layout, swapped back in once it has settled there) or
//   close (it slides off, then `onClose`, which closes the view through its
//   save-first path).
// - The swipe back: in from the left edge inside a sub-task, the content
//   follows the finger; released past the threshold (or flicked) it slides
//   off and Back is clicked (so a pending edit saves, as for a tap on ‹),
//   otherwise it springs back.
// - The keyboard: `--vv-top`, `--vv-h` and `--kb` (on the root) keep the sheet
//   inside the visible area, above the keyboard, header in view. The visual
//   viewport only resizes once the keyboard has arrived, so Swift
//   (crates/plugin-ios Keyboard.swift) calls `window.keelKeyboard` as it
//   starts: the sheet's bottom rises with it, on its curve and at its pace,
//   and a medium sheet goes to large alongside and stays there.
// Reduced motion snaps.
import { currentPlatform } from "../shell/platform";
import {
  atEdge,
  type Detent,
  type DragStart,
  dragFrom,
  dragOffset,
  EASE,
  edgeBackAction,
  edgeBackCommits,
  edgeOffset,
  holdsScroll,
  keyboardExpands,
  keyboardInset,
  SETTLE_S,
  SHEET_MAX,
  type SheetDrag,
  settleOffset,
  sheetDragAction,
  sheetSettle,
  velocity,
} from "./sheet-rules";

/** A keyboard move as Swift reports it: `kb` px of the web view it will cover (0 as it goes), its pace and curve. */
export type KeyboardMove = { kb: number; secs: number; ease: string };

declare global {
  interface Window {
    /** Called by crates/plugin-ios Keyboard.swift as the keyboard starts to move; set while the sheet is installed. */
    keelKeyboard?: (m: KeyboardMove) => void;
  }
}

const BLOCK = ".popover, .menu, .editor, .modal-scrim, .pop-backdrop";
const FIELD = "input, textarea, select, [contenteditable]";

type Heights = { M: number; largeTop: number; mediumH: number };
type Placed = Heights & { sheet: HTMLElement };
type Drag = Heights & {
  sheet: HTMLElement;
  x: number;
  y: number;
  mode: SheetDrag;
  samples: Array<[number, number]>;
  start: DragStart;
  dy0: number;
  from: number;
  yNow: number;
};
type Edge = {
  sheet: HTMLElement;
  x: number;
  y: number;
  mode: "pending" | "drag" | "ignore";
  dx: number;
  samples: Array<[number, number]>;
};

/** The installed sheet's slide-close (shared state with its listeners), or null when none is installed. */
let active: ((doc: Document, win: Window) => boolean) | null = null;

/**
 * The X and the scrim: slides the sheet down off the screen, then calls the
 * `onClose` given to `installSheet`, as a drag past the threshold does. False
 * on the desktop panel (at least SHEET_MAX wide), which the caller closes at
 * once, and when no sheet listeners are installed. A second call while
 * sliding is true and doesn't close twice.
 */
export function slideClose(doc: Document, win: Window): boolean {
  return active ? active(doc, win) : false;
}

/** Installs the sheet's listeners on `doc` and `win`; returns a function that removes them. */
export function installSheet(doc: Document, win: Window, onClose: () => void): () => void {
  const timers = new Set<ReturnType<typeof setTimeout>>();
  const later = (f: () => void, ms: number) => {
    const t = setTimeout(() => {
      timers.delete(t);
      f();
    }, ms);
    timers.add(t);
  };
  const reduce = () =>
    typeof win.matchMedia === "function" && win.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const sheetEl = () => doc.querySelector<HTMLElement>(".tv-sheet");
  const scrim = () => doc.querySelector<HTMLElement>(".tv-scrim");
  const closest = (target: EventTarget | null, sel: string): Element | null =>
    target instanceof Element ? target.closest(sel) : null;
  const detentOf = (sheet: HTMLElement): Detent => (sheet.dataset["detent"] === "large" ? "large" : "medium");
  const shiftOf = (el: HTMLElement): number => {
    if (typeof DOMMatrix === "undefined") return 0;
    return new DOMMatrix(win.getComputedStyle(el).transform).m42 || 0;
  };
  const finishAnimations = (el: HTMLElement) => {
    if (typeof el.getAnimations !== "function") return;
    for (const a of el.getAnimations()) a.finish();
  };
  // Both layouts' tops (no transform): `M` is how far medium sits below large.
  const heights = (sheet: HTMLElement): Heights => {
    const was = sheet.classList.contains("tv-large");
    const keep = sheet.style.transform;
    sheet.style.transform = "none";
    sheet.classList.remove("tv-large");
    const m = sheet.getBoundingClientRect();
    sheet.classList.add("tv-large");
    const l = sheet.getBoundingClientRect();
    if (!was) sheet.classList.remove("tv-large");
    sheet.style.transform = keep;
    return { M: m.top - l.top, largeTop: l.top, mediumH: m.height };
  };
  const place = (d: Placed, y: number, s: number) => {
    d.sheet.style.transition = s > 0 ? `transform ${s}s ${EASE}` : "none";
    d.sheet.style.transform = `translateY(${y}px)`;
    const sc = scrim();
    if (sc) {
      sc.style.transition = s > 0 ? `opacity ${s}s ${EASE}` : "none";
      sc.style.opacity = String(Math.max(0, Math.min(1, 1 - (y - Math.max(0, d.M)) / d.mediumH)));
    }
  };

  let g: Drag | null = null;
  let seq = 0;
  let closing = false;

  const onDragStart = (ev: TouchEvent) => {
    g = null;
    const t = ev.touches[0];
    if (ev.touches.length !== 1 || !t) return;
    const sheet = closest(ev.target, ".tv-sheet");
    if (!(sheet instanceof HTMLElement)) return;
    const field = closest(ev.target, FIELD);
    const scroller = closest(ev.target, ".tv-scroll");
    g = {
      sheet,
      x: t.clientX,
      y: t.clientY,
      mode: "pending",
      samples: [],
      // Measured when it turns into a drag.
      M: 0,
      largeTop: 0,
      mediumH: 0,
      dy0: 0,
      from: 0,
      yNow: 0,
      start: {
        scrollTop: scroller ? scroller.scrollTop : 0,
        blocked: closest(ev.target, BLOCK) !== null,
        inField: field !== null,
        fieldFocused: field?.contains(doc.activeElement) ?? false,
        width: win.innerWidth,
        detent: detentOf(sheet),
      },
    };
  };
  const onDragMove = (ev: TouchEvent) => {
    const t = ev.touches[0];
    if (!g || g.mode === "scroll" || g.mode === "ignore" || !t) return;
    const dx = t.clientX - g.x;
    const dy = t.clientY - g.y;
    if (g.mode === "pending") {
      g.mode = sheetDragAction(g.start, dx, dy);
      if (g.mode === "pending" && holdsScroll(g.start, dx, dy)) ev.preventDefault();
      if (g.mode !== "drag") return;
      seq++;
      // Grabbed while settling: freeze it where it is (its pending settle is
      // dropped by `seq`) and drag from there.
      const sheet = g.sheet;
      const settling = sheet.classList.contains("tv-large") ? shiftOf(sheet) : null;
      sheet.style.transition = "none";
      if (settling !== null) sheet.style.transform = `translateY(${settling}px)`;
      finishAnimations(sheet);
      const h = heights(sheet);
      Object.assign(g, h, { dy0: dy, from: dragFrom(g.start.detent, settling, h.M) });
      sheet.classList.add("tv-large");
    }
    ev.preventDefault();
    g.yNow = dragOffset(g.from, dy - g.dy0);
    g.samples.push([ev.timeStamp, dy]);
    if (g.samples.length > 6) g.samples.shift();
    place(g, g.yNow, 0);
  };
  const end = (cancelled: boolean) => {
    const d = g;
    g = null;
    if (d?.mode !== "drag") return;
    const from = d.start.detent;
    const to = cancelled ? from : sheetSettle(from, d.yNow, d.M, velocity(d.samples));
    const secs = reduce() ? 0 : SETTLE_S;
    // Close slides it just past the bottom of the screen.
    const target = to === "close" ? win.innerHeight - d.largeTop + 40 : settleOffset(to, d.M);
    place(d, target, secs);
    const mine = seq;
    if (to !== "close") d.sheet.dataset["detent"] = to;
    later(() => {
      if (to === "close") {
        onClose();
        return;
      }
      if (mine !== seq || !d.sheet.isConnected) return;
      if (to === "medium") d.sheet.classList.remove("tv-large");
      d.sheet.style.transition = "none";
      d.sheet.style.transform = "";
      const sc = scrim();
      if (sc) {
        sc.style.transition = "none";
        sc.style.opacity = "";
      }
    }, secs * 1000);
  };

  // The swipe back. The view under (`.tv-under`, mounted but hidden) comes in
  // from a third of a width to the left, as in iOS, while the top view's
  // content leaves.
  let e: Edge | null = null;
  const topBody = (sheet: HTMLElement) =>
    sheet.querySelector<HTMLElement>(".tv-stack > .task-view:not(.tv-under) > .tv-body");
  const under = (sheet: HTMLElement) => sheet.querySelector<HTMLElement>(".tv-stack > .tv-under");
  const edgeMove = (d: Edge, x: number, secs: number) => {
    const t = secs > 0 ? `transform ${secs}s ${EASE}` : "none";
    const width = win.innerWidth;
    const share = Math.min(1, x / width);
    const body = topBody(d.sheet);
    const u = under(d.sheet);
    const ub = u?.querySelector<HTMLElement>(":scope > .tv-body");
    if (body) {
      body.style.transition = t;
      body.style.transform = x ? `translateX(${x}px)` : "";
      body.style.boxShadow = x ? "-6px 0 18px rgba(0,0,0,.12)" : "";
    }
    if (u) u.style.visibility = x ? "visible" : "";
    if (ub) {
      ub.style.transition = t;
      ub.style.transform = x ? `translateX(${(-width / 3) * (1 - share)}px)` : "";
    }
  };
  const onEdgeStart = (ev: TouchEvent) => {
    e = null;
    const t = ev.touches[0];
    if (ev.touches.length !== 1 || !t) return;
    const sheet = closest(ev.target, ".tv-sheet");
    if (!(sheet instanceof HTMLElement) || win.innerWidth >= SHEET_MAX) return;
    const can = atEdge(t.clientX) && sheet.querySelector(".tv-back") !== null && !closest(ev.target, BLOCK);
    if (can) e = { sheet, x: t.clientX, y: t.clientY, mode: "pending", dx: 0, samples: [] };
  };
  const onEdgeMove = (ev: TouchEvent) => {
    const t = ev.touches[0];
    if (!e || e.mode === "ignore" || !t) return;
    const dx = t.clientX - e.x;
    const dy = t.clientY - e.y;
    if (e.mode === "pending") {
      e.mode = edgeBackAction(true, dx, dy);
      if (e.mode !== "drag") {
        if (e.mode === "ignore") e = null;
        return;
      }
      // The vertical sheet drag must not also take this touch.
      g = null;
    }
    ev.preventDefault();
    ev.stopPropagation();
    e.dx = edgeOffset(dx, win.innerWidth);
    e.samples.push([ev.timeStamp, dx]);
    if (e.samples.length > 6) e.samples.shift();
    edgeMove(e, e.dx, 0);
  };
  const edgeEnd = (cancelled: boolean, at: number) => {
    const d = e;
    e = null;
    if (d?.mode !== "drag") return;
    // The lift counts as a sample, so a pause before it isn't read as a flick.
    d.samples.push([at, d.dx]);
    const secs = reduce() ? 0 : SETTLE_S;
    if (cancelled || !edgeBackCommits(d.dx, velocity(d.samples), win.innerWidth)) {
      edgeMove(d, 0, secs);
      return;
    }
    edgeMove(d, win.innerWidth, secs);
    // Back keeps the view under mounted and makes it the top one. It stays
    // shown until it has lost `.tv-under`, then the swipe's styles go (it's at
    // rest by now, so nothing moves). The leaving view stays off to the right
    // until it's removed.
    const u = under(d.sheet);
    const ub = u?.querySelector<HTMLElement>(":scope > .tv-body");
    const settle = (frames: number) => {
      if (u?.isConnected && u.classList.contains("tv-under") && frames > 0) {
        win.requestAnimationFrame(() => settle(frames - 1));
        return;
      }
      if (u) u.style.visibility = "";
      if (ub) {
        ub.style.transition = "none";
        ub.style.transform = "";
      }
    };
    later(() => {
      d.sheet.querySelector<HTMLElement>(".tv-back")?.click();
      settle(120);
    }, secs * 1000);
  };
  const onEdgeEnd = (ev: TouchEvent) => edgeEnd(false, ev.timeStamp);
  const onEdgeCancel = (ev: TouchEvent) => edgeEnd(true, ev.timeStamp);
  const onEnd = () => end(false);
  const onCancel = () => end(true);

  const slide = (d: Document, w: Window): boolean => {
    const sheet = d.querySelector<HTMLElement>(".tv-sheet");
    if (!sheet || w.innerWidth >= SHEET_MAX) return false;
    if (closing) return true;
    closing = true;
    seq++;
    g = null;
    const p = { sheet, ...heights(sheet) };
    finishAnimations(sheet);
    const settling = sheet.classList.contains("tv-large") ? shiftOf(sheet) : null;
    const from = dragFrom(detentOf(sheet), settling, p.M);
    sheet.classList.add("tv-large");
    place(p, from, 0);
    sheet.getBoundingClientRect();
    const secs = reduce() ? 0 : SETTLE_S;
    place(p, w.innerHeight - p.largeTop + 40, secs);
    later(() => {
      closing = false;
      onClose();
    }, secs * 1000);
    return true;
  };

  // The keyboard: keep the sheet in the visible area above it. Only on a
  // touch device at 1x: a desktop pinch-zoom mustn't lift the sheet. A
  // touch-screen Windows laptop counts too, harmlessly: --kb is 0 at 1x there.
  // `kbHold`: the height of a keyboard still coming up, standing in until the
  // visual viewport (resized only once it's up) has caught up (`keyboardInset`).
  const touchDevice = () => currentPlatform() === "touch";
  let kbHold: { kb: number; until: number } | null = null;
  const VARS = ["--vv-top", "--vv-h", "--kb"];
  const viewport = () => {
    const v = win.visualViewport;
    const root = doc.documentElement;
    for (const k of VARS) root.style.removeProperty(k);
    if (v && touchDevice() && v.scale === 1) {
      const held = kbHold && win.performance.now() < kbHold.until ? kbHold.kb : null;
      root.style.setProperty("--vv-top", `${v.offsetTop}px`);
      root.style.setProperty("--vv-h", `${v.height}px`);
      root.style.setProperty("--kb", `${keyboardInset(root.clientHeight, v.offsetTop, v.height, held)}px`);
    }
  };
  // The sheet's bottom follows the keyboard both ways; coming up, a medium
  // sheet also slides up to large from wherever it is, and stays large once
  // the keyboard has gone.
  const keyboard = ({ kb, secs, ease }: KeyboardMove) => {
    if (!touchDevice()) return;
    const sheet = sheetEl();
    const s = reduce() ? 0 : secs;
    kbHold = kb > 0 ? { kb, until: win.performance.now() + s * 1000 + 500 } : null;
    if (!sheet || closing) {
      viewport();
      return;
    }
    const bottom = s > 0 ? `bottom ${s}s ${ease}` : "none";
    if (!keyboardExpands(detentOf(sheet), win.innerWidth, kb)) {
      // Large already, or the keyboard going: only the bottom moves (a
      // settle's own transform, if any, carries on).
      const t = sheet.style.transition;
      sheet.style.transition = t && t !== "none" && s > 0 ? `${t}, ${bottom}` : bottom;
      viewport();
      return;
    }
    seq++;
    g = null;
    const p = { sheet, ...heights(sheet) };
    finishAnimations(sheet);
    const settling = sheet.classList.contains("tv-large") ? shiftOf(sheet) : null;
    sheet.dataset["detent"] = "large";
    sheet.classList.add("tv-large");
    place(p, dragFrom("medium", settling, p.M), 0);
    sheet.getBoundingClientRect();
    // The first keyboard after launch comes with no pace (iOS reports a 0 s
    // animation): the sheet still slides, at its own settle pace.
    const slide = s > 0 ? `transform ${s}s ${ease}` : reduce() ? "" : `transform ${SETTLE_S}s ${EASE}`;
    const slideS = s > 0 ? s : reduce() ? 0 : SETTLE_S;
    sheet.style.transition = [slide, s > 0 ? bottom : ""].filter(Boolean).join(", ") || "none";
    sheet.style.transform = "translateY(0px)";
    viewport();
    const mine = seq;
    later(() => {
      if (mine !== seq || !sheet.isConnected) return;
      sheet.style.transition = "none";
      sheet.style.transform = "";
      const sc = scrim();
      if (sc) {
        sc.style.transition = "none";
        sc.style.opacity = "";
      }
    }, slideS * 1000);
  };

  const passive = { passive: true, capture: true } as const;
  const nonPassive = { passive: false, capture: true } as const;
  doc.addEventListener("touchstart", onDragStart, passive);
  doc.addEventListener("touchmove", onDragMove, nonPassive);
  doc.addEventListener("touchstart", onEdgeStart, passive);
  doc.addEventListener("touchmove", onEdgeMove, nonPassive);
  doc.addEventListener("touchend", onEdgeEnd, passive);
  doc.addEventListener("touchcancel", onEdgeCancel, passive);
  doc.addEventListener("touchend", onEnd, passive);
  doc.addEventListener("touchcancel", onCancel, passive);
  win.keelKeyboard = keyboard;
  win.visualViewport?.addEventListener("resize", viewport);
  win.visualViewport?.addEventListener("scroll", viewport);
  win.addEventListener("resize", viewport);
  active = slide;

  return () => {
    for (const t of timers) clearTimeout(t);
    timers.clear();
    doc.removeEventListener("touchstart", onDragStart, true);
    doc.removeEventListener("touchmove", onDragMove, true);
    doc.removeEventListener("touchstart", onEdgeStart, true);
    doc.removeEventListener("touchmove", onEdgeMove, true);
    doc.removeEventListener("touchend", onEdgeEnd, true);
    doc.removeEventListener("touchcancel", onEdgeCancel, true);
    doc.removeEventListener("touchend", onEnd, true);
    doc.removeEventListener("touchcancel", onCancel, true);
    if (win.keelKeyboard === keyboard) delete win.keelKeyboard;
    win.visualViewport?.removeEventListener("resize", viewport);
    win.visualViewport?.removeEventListener("scroll", viewport);
    win.removeEventListener("resize", viewport);
    for (const k of VARS) doc.documentElement.style.removeProperty(k);
    if (active === slide) active = null;
  };
}
