// The Search screen follows the finger, in and out, as the drawer does.
//
// Back: dragged right it moves off over the page it came from, and released
// past halfway (or flicked) it goes; otherwise it springs back. It is moved
// as a still copy (the `.view-ghost.pop` the Back button's slide uses), laid
// over the main pane when the drag starts, while the page behind is asked for
// (so it is landing as the finger moves).
//
// In: dragged left on a page with the search button, the pane switches to
// Search at once, and once Search has landed its `.main` is placed by `--push`
// (app.css) behind the finger, over the still copy (`.view-ghost.push`) of the
// page it leaves, which the drag moves aside a little as the slide does. Let
// go short and the page it came from is asked for again under that copy.
//
// `ghostGo` leaves a dragged ghost alone. Reduced motion gets the instant
// swipes instead (gestures/sheet-rules.ts).
import { canPan, installPan } from "../gestures/pan";
import { EASE } from "../gestures/sheet-rules";
import { makeTimers } from "../gestures/timers";
import { prefersReducedMotion } from "../shell/device";
import { ghost } from "./ghost";

export type SearchDragControl = {
  isSearch: () => boolean;
  drawerOpen: () => boolean;
  /** Back: the drag began: go to the page Search was opened from (it loads under the ghost). */
  leave: () => void;
  /** Back: let go short: Search is wanted again (the caller removes the ghost once it has landed). */
  cancel: () => void;
  /** In: the drag began: switch to Search (its screen is placed by the drag as it lands). */
  push: () => void;
  /** In: let go past halfway: Search stays. */
  pushed: () => void;
  /** In: let go short: the page it came from is wanted again (the caller removes the ghost once it has landed). */
  pushCancel: () => void;
};

/** The pace and curve of the Search slide (`.view-ghost.pop.go` in app.css). */
const SETTLE_MS = 360;
/** How far the page left behind slides, and how much it fades (`ghost-under`). */
const UNDER_SHIFT = 24;
const UNDER_FADE = 0.4;

/** Removes what a drag in left on the app: the placing of Search's screen and the copy of the page it left. */
export function clearPush(doc: Document = document): void {
  const app = doc.querySelector<HTMLElement>(".app");
  if (app) {
    delete app.dataset["pushDrag"];
    app.style.removeProperty("--push");
  }
  for (const g of doc.querySelectorAll(".view-ghost")) g.remove();
}

export function installSearchDrag(doc: Document, ctl: SearchDragControl): () => void {
  const { later, clear } = makeTimers();
  const reduced = () => prefersReducedMotion(doc.defaultView);

  const back = installPan(doc, (ctx) => {
    if (!canPan(ctx)) return null;
    if (!ctl.isSearch() || ctl.drawerOpen() || reduced()) return null;
    const main = doc.querySelector<HTMLElement>(".main");
    if (!main) return null;
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

  const forward = installPan(doc, (ctx) => {
    if (!canPan(ctx)) return null;
    if (ctl.isSearch() || ctl.drawerOpen() || reduced()) return null;
    const app = doc.querySelector<HTMLElement>(".app");
    const main = doc.querySelector<HTMLElement>(".main");
    if (!app || !main) return null;
    const range = main.offsetWidth;
    // The copy of the page being left: it is made when the view changes, so it is found as the finger moves.
    const under = () => {
      const g = doc.querySelector<HTMLElement>(".view-ghost.push");
      g?.classList.add("dragging");
      return g;
    };
    return {
      dir: -1,
      range,
      onStart: () => {
        app.dataset["pushDrag"] = "";
        app.style.setProperty("--push", `${range}px`);
        ctl.push();
      },
      onMove: (p) => {
        app.style.setProperty("--push", `${Math.round((1 - p) * range)}px`);
        const g = under();
        if (g) {
          g.style.transform = `translateX(${-UNDER_SHIFT * p}%)`;
          g.style.opacity = String(1 - UNDER_FADE * p);
        }
      },
      onEnd: (commit) => {
        app.dataset["pushDrag"] = "settle";
        app.style.setProperty("--push", commit ? "0px" : `${range}px`);
        const g = under();
        if (g) {
          g.style.transition = `transform ${SETTLE_MS}ms ${EASE}, opacity ${SETTLE_MS}ms ${EASE}`;
          g.style.transform = commit ? `translateX(${-UNDER_SHIFT}%)` : "none";
          g.style.opacity = commit ? String(1 - UNDER_FADE) : "1";
        }
        later(() => {
          if (commit) {
            clearPush(doc);
            ctl.pushed();
            return;
          }
          ctl.pushCancel();
          // Search never showed: nothing is under the copy to wait for.
          if (!doc.querySelector(".search-input")) clearPush(doc);
        }, SETTLE_MS);
      },
    };
  });

  return () => {
    back();
    forward();
    clear();
  };
}
