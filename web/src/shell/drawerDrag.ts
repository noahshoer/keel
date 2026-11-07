// The phone's drawer follows the finger: dragged right from the page it opens,
// dragged left (over the page or the drawer) it closes, and released past
// halfway, or flicked, it carries on at the pace of a tap; otherwise it goes
// back. While it moves, `.app[data-drawer-drag]` and `--p` (0 shut, 1 open)
// place the drawer, the scrim and the page's parallax in CSS (app.css); the
// attribute reads "settle" while it runs the rest of the way, then goes and
// React's drawer state takes over. Rules: gestures/sheet-rules.ts.
import { installPan } from "../gestures/pan";
import { NARROW_MAX } from "../gestures/sheet-rules";

/** What the drawer drag needs from the app. */
export type DrawerControl = {
  isOpen: () => boolean;
  /** False where the page's own swipe takes over (the Search screen goes back). */
  canOpen: () => boolean;
  setOpen: (open: boolean) => void;
};

/** The CSS's settle time, in ms (`.26s` in app.css). */
const SETTLE_MS = 260;

export function installDrawerDrag(doc: Document, ctl: DrawerControl): () => void {
  let settling = false;
  const timers = new Set<ReturnType<typeof setTimeout>>();
  const off = installPan(doc, (ctx) => {
    if (settling || ctx.blocked || ctx.inSheet || ctx.inSettings || ctx.width > NARROW_MAX) return null;
    const app = doc.querySelector<HTMLElement>(".app");
    const sidebar = doc.querySelector<HTMLElement>(".sidebar");
    const open = ctl.isOpen();
    if (!app || !sidebar || (!open && !ctl.canOpen())) return null;
    const place = (progress: number) => app.style.setProperty("--p", String(open ? 1 - progress : progress));
    return {
      dir: open ? -1 : 1,
      range: sidebar.offsetWidth,
      onStart: () => {
        place(0);
        app.dataset["drawerDrag"] = "";
        // Opening: the scrim has to be there to dim the page.
        if (!open) ctl.setOpen(true);
      },
      onMove: place,
      onEnd: (commit) => {
        const final = commit ? !open : open;
        settling = true;
        app.dataset["drawerDrag"] = "settle";
        app.style.setProperty("--p", final ? "1" : "0");
        const t = setTimeout(() => {
          timers.delete(t);
          settling = false;
          delete app.dataset["drawerDrag"];
          // The scrim a drag made is already showing; keep it from fading in again.
          if (final) app.dataset["drawerRest"] = "";
          app.style.removeProperty("--p");
          ctl.setOpen(final);
        }, SETTLE_MS);
        timers.add(t);
      },
    };
  });
  return () => {
    off();
    for (const t of timers) clearTimeout(t);
  };
}
