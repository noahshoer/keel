// The Search screen follows the finger back, as the drawer does: dragged
// right it moves off over the page it came from, and released past halfway
// (or flicked) it goes; otherwise it springs back. It is moved as a still
// copy (the `.view-ghost.pop` the Back button's slide uses), laid over the
// main pane when the drag starts, while the page behind is asked for (so it
// is landing as the finger moves). `ghostGo` leaves a dragged ghost alone.
// Reduced motion gets the instant swipe back instead (gestures/sheet-rules.ts).
import { installPan } from "../gestures/pan";
import { NARROW_MAX } from "../gestures/sheet-rules";
import { ghost } from "./ghost";

export type SearchDragControl = {
  isSearch: () => boolean;
  drawerOpen: () => boolean;
  /** The drag began: go to the page Search was opened from (it loads under the ghost). */
  leave: () => void;
  /** Let go short: Search is wanted again (the caller removes the ghost once it has landed). */
  cancel: () => void;
};

/** The pace and curve of the Search slide (`.view-ghost.pop.go` in app.css). */
const SETTLE_MS = 360;
const EASE = "cubic-bezier(.32,.72,0,1)";

export function installSearchDrag(doc: Document, ctl: SearchDragControl): () => void {
  const timers = new Set<ReturnType<typeof setTimeout>>();
  const later = (f: () => void, ms: number) => {
    const t = setTimeout(() => {
      timers.delete(t);
      f();
    }, ms);
    timers.add(t);
  };
  const off = installPan(doc, (ctx) => {
    if (ctx.blocked || ctx.inSheet || ctx.inSettings || ctx.width > NARROW_MAX) return null;
    if (!ctl.isSearch() || ctl.drawerOpen()) return null;
    const reduced = doc.defaultView?.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    const main = doc.querySelector<HTMLElement>(".main");
    if (reduced || !main) return null;
    const range = main.offsetWidth;
    let g: HTMLElement | null = null;
    return {
      dir: 1,
      range,
      onStart: () => {
        ghost("pop", doc);
        g = doc.querySelector<HTMLElement>(".view-ghost.pop");
        g?.classList.add("dragging");
        ctl.leave();
      },
      onMove: (p) => {
        if (g) g.style.transform = `translateX(${Math.round(p * range)}px)`;
      },
      onEnd: (commit) => {
        const el = g;
        if (el) {
          el.style.transition = `transform ${SETTLE_MS}ms ${EASE}`;
          el.style.transform = commit ? `translateX(${range}px)` : "translateX(0px)";
        }
        later(() => {
          if (commit) el?.remove();
          else ctl.cancel();
        }, SETTLE_MS);
      },
    };
  });
  return () => {
    off();
    for (const t of timers) clearTimeout(t);
  };
}
