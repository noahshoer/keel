// Reports each finished single-finger touch as a `Swipe`, for the drawer
// (crates/ui/src/gestures.rs `SWIPE_JS`); `drawerSwipe` decides what it does.
import type { Swipe } from "./sheet-rules";

const BLOCK = "input, textarea, select, [contenteditable], .editor, .popover, .menu, .modal-scrim";
/** A touch on the scrim counts as in the sheet, so a swipe there doesn't open the drawer over it. */
const SHEET = ".tv-sheet, .tv-scrim";

/** Installs the listeners on `doc`; returns a function that removes them. */
export function installSwipe(doc: Document, onSwipe: (s: Swipe) => void): () => void {
  let start: { x: number; y: number; blocked: boolean; inSheet: boolean } | null = null;
  const onStart = (ev: TouchEvent) => {
    const t = ev.touches[0];
    if (ev.touches.length !== 1 || !t) {
      start = null;
      return;
    }
    const at = (sel: string) => ev.target instanceof Element && ev.target.closest(sel) !== null;
    start = { x: t.clientX, y: t.clientY, blocked: at(BLOCK), inSheet: at(SHEET) };
  };
  const onEnd = (ev: TouchEvent) => {
    const t = ev.changedTouches[0];
    if (!start || !t) return;
    const width = doc.defaultView?.innerWidth ?? 0;
    const { x, y, blocked, inSheet } = start;
    start = null;
    onSwipe({ dx: t.clientX - x, dy: t.clientY - y, width, blocked, inSheet });
  };

  const passive = { passive: true } as const;
  doc.addEventListener("touchstart", onStart, passive);
  doc.addEventListener("touchend", onEnd, passive);
  return () => {
    doc.removeEventListener("touchstart", onStart);
    doc.removeEventListener("touchend", onEnd);
  };
}
