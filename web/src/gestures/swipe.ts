// Reports each finished single-finger touch as a `Swipe`, for the instant
// page swipes (Search in and back); `pan.ts` follows the finger for the
// drawer and Settings. Where a touch started is `touchContext`.
import type { Swipe } from "./sheet-rules";

const BLOCK =
  "input, textarea, select, [contenteditable], .editor, .editor-scrim, .pop-backdrop, .popover, .menu";
/** A touch on the scrim counts as in the sheet, so a swipe there doesn't open the drawer over it. */
const SHEET = ".tv-sheet, .tv-scrim";
/** The Settings screen; its fields, menus and popovers are still blocked. */
const SETTINGS = ".modal-scrim";

/** What a touch started on: shared by the swipe and the pan. */
export type TouchContext = Pick<Swipe, "width" | "blocked" | "inSheet" | "inSettings">;

export function touchContext(doc: Document, ev: TouchEvent): TouchContext {
  const at = (sel: string) => ev.target instanceof Element && ev.target.closest(sel) !== null;
  return {
    width: doc.defaultView?.innerWidth ?? 0,
    blocked: at(BLOCK),
    inSheet: at(SHEET),
    inSettings: at(SETTINGS),
  };
}

/** Installs the listeners on `doc`; returns a function that removes them. */
export function installSwipe(doc: Document, onSwipe: (s: Swipe) => void): () => void {
  let start: { x: number; y: number; ctx: TouchContext } | null = null;
  const onStart = (ev: TouchEvent) => {
    const t = ev.touches[0];
    if (ev.touches.length !== 1 || !t) {
      start = null;
      return;
    }
    start = { x: t.clientX, y: t.clientY, ctx: touchContext(doc, ev) };
  };
  const onEnd = (ev: TouchEvent) => {
    const t = ev.changedTouches[0];
    if (!start || !t) return;
    const { x, y, ctx } = start;
    start = null;
    onSwipe({ dx: t.clientX - x, dy: t.clientY - y, ...ctx, width: doc.defaultView?.innerWidth ?? 0 });
  };

  const passive = { passive: true } as const;
  doc.addEventListener("touchstart", onStart, passive);
  doc.addEventListener("touchend", onEnd, passive);
  return () => {
    doc.removeEventListener("touchstart", onStart);
    doc.removeEventListener("touchend", onEnd);
  };
}
