// The phone's drawer follows the finger: dragged right from the page it opens,
// dragged left (over the page or the drawer) it closes, and released past
// halfway, or flicked, it carries on at the pace of a tap; otherwise it goes
// back. Rules: gestures/sheet-rules.ts.
//
// While it moves, this sets the drawer's, the page's and a dimming layer's
// styles directly (no CSS variable, so nothing re-styles the whole page per
// frame), and `.app[data-drawer-drag]` hides React's own scrim. The dimming
// layer is made here because React's scrim only appears a frame or two after
// the drag starts, and would pop in. When the drag ends:
// - carried to open, or back to open: the styles go, `data-drawer-rest` stops
//   React's scrim (already showing) from fading in again, and the dim goes
//   with it in the same frame;
// - carried to shut, or back to shut: React is told, and `clearDrag` (called
//   once its drawer state is false) removes the rest, so the scrim React is
//   about to remove never shows between.
import { canPan, installPan } from "../gestures/pan";
import { EASE_SOFT } from "../gestures/sheet-rules";
import { makeTimers } from "../gestures/timers";

/** What the drawer drag needs from the app. */
type DrawerControl = {
  isOpen: () => boolean;
  /** False where the page's own swipe takes over (the Search screen goes back). */
  canOpen: () => boolean;
  setOpen: (open: boolean) => void;
};

/** The drawer's own pace and curve (`.26s var(--ease)` in app.css). */
const SETTLE_S = 0.26;
/** How far the page slides right while the drawer is open (the CSS parallax). */
const PARALLAX = 28;
/** The dimming at full open (`.drawer-scrim`'s background alpha is its own). */
const DIM = "drag-dim";

/** Removes everything a drag left on `app`: inline styles, the dimming layer, the markers. */
export function clearDrag(app: HTMLElement): void {
  delete app.dataset["drawerDrag"];
  app.querySelector(`.${DIM}`)?.remove();
  for (const el of app.querySelectorAll<HTMLElement>(".sidebar, .content")) {
    el.style.removeProperty("transform");
    el.style.removeProperty("translate");
    el.style.removeProperty("transition");
    el.style.removeProperty("visibility");
  }
}

export function installDrawerDrag(doc: Document, ctl: DrawerControl): () => void {
  let settling = false;
  const { later, clear } = makeTimers();
  const off = installPan(doc, (ctx) => {
    if (settling || !canPan(ctx)) return null;
    const app = doc.querySelector<HTMLElement>(".app");
    const sidebar = doc.querySelector<HTMLElement>(".sidebar");
    const content = doc.querySelector<HTMLElement>(".content");
    const open = ctl.isOpen();
    if (!app || !sidebar || !content || (!open && !ctl.canOpen())) return null;
    let dim: HTMLElement | null = null;
    const ease = `${SETTLE_S}s ${EASE_SOFT}`;
    // `p`: 0 shut, 1 open.
    const place = (p: number, settle = false) => {
      sidebar.style.transition = settle ? `transform ${ease}` : "none";
      content.style.transition = settle ? `translate ${ease}` : "none";
      if (dim) dim.style.transition = settle ? `opacity ${ease}` : "none";
      sidebar.style.transform = `translateX(${(p - 1) * 100}%)`;
      sidebar.style.visibility = "visible";
      content.style.translate = `${PARALLAX * p}px 0`;
      if (dim) dim.style.opacity = String(p);
    };
    return {
      dir: open ? -1 : 1,
      range: sidebar.offsetWidth,
      onStart: () => {
        app.dataset["drawerDrag"] = "";
        delete app.dataset["drawerRest"];
        dim = doc.createElement("div");
        dim.className = `drawer-scrim ${DIM}`;
        dim.style.animation = "none";
        app.prepend(dim);
        place(open ? 1 : 0);
        // Opening: the drawer's state (its class, the page hold) follows the finger.
        if (!open) ctl.setOpen(true);
      },
      onMove: (progress) => place(open ? 1 - progress : progress),
      onEnd: (commit) => {
        const final = commit ? !open : open;
        settling = true;
        place(final ? 1 : 0, true);
        later(() => {
          settling = false;
          if (final) {
            // Open: React's scrim is showing under the dim; the rest of the styles are the class's.
            app.dataset["drawerRest"] = "";
            clearDrag(app);
          }
          // Shut: it was open and is closing (or was a cancelled open): Frame clears once React has it.
          ctl.setOpen(final);
        }, SETTLE_S * 1000);
      },
    };
  });
  return () => {
    off();
    clear();
  };
}
