// A one-finger sideways pan that follows the finger (the drawer, Settings
// leaving). `begin` is asked at touchstart whether this touch can pan and
// how; the rules (`panIntent`, `panProgress`, `panCommits`) are in
// sheet-rules.ts. A pan starts once the touch has moved past the slop
// sideways, with the right direction; from then the browser's scroll is
// cancelled and `onMove` gets the progress, 0 to 1, as the finger goes. The
// anchor is where the pan started, so the view doesn't jump by the slop.

import { type PanIntent, panCommits, panIntent, panProgress, velocity } from "./sheet-rules";
import { type TouchContext, touchContext } from "./swipe";

export type Pan = {
  /** Which way the finger goes to advance it: 1 right, -1 left. */
  dir: 1 | -1;
  /** Finger travel for progress 1, in px. */
  range: number;
  /** The pan began (the finger passed the slop, sideways, the right way). */
  onStart: () => void;
  /** Progress along `dir`, 0 to 1. */
  onMove: (progress: number) => void;
  /** Released (or cancelled): `commit` says carry on, `progress` where it was let go. */
  onEnd: (commit: boolean, progress: number) => void;
};

type Active = {
  pan: Pan;
  /** Removes the listeners on the touch's own element. */
  unbind: () => void;
  x: number;
  y: number;
  mode: PanIntent | "pan" | "ignore";
  anchor: number;
  progress: number;
  samples: Array<[number, number]>;
};

/** Installs the listeners on `doc`; returns a function that removes them. */
export function installPan(doc: Document, begin: (ctx: TouchContext) => Pan | null): () => void {
  let cur: Active | null = null;
  const finish = (commit: boolean, at?: number) => {
    const a = cur;
    cur = null;
    a?.unbind();
    if (a?.mode !== "pan") return;
    // A finger that rested before lifting is not a flick: the speed runs to the lift.
    const last = a.samples.at(-1);
    if (at !== undefined && last) a.samples.push([at, last[1]]);
    a.pan.onEnd(commit ? panCommits(a.progress, velocity(a.samples)) : false, a.progress);
  };

  const onStart = (ev: TouchEvent) => {
    if (cur) finish(false);
    const t = ev.touches[0];
    if (ev.touches.length !== 1 || !t) return;
    const pan = begin(touchContext(doc, ev));
    if (!pan) return;
    // The rest of the touch is heard on its own element: if the page replaces it mid-pan
    // (the Search screen does), WebKit sends the touch's moves and end there and no
    // longer up to the document.
    const el = ev.target;
    const unbind = () => {
      el?.removeEventListener("touchmove", onMove as EventListener);
      el?.removeEventListener("touchend", onEnd as EventListener);
      el?.removeEventListener("touchcancel", onCancel);
    };
    cur = { pan, unbind, x: t.clientX, y: t.clientY, mode: "pending", anchor: 0, progress: 0, samples: [] };
    el?.addEventListener("touchmove", onMove as EventListener, { passive: false });
    el?.addEventListener("touchend", onEnd as EventListener, { passive: true });
    el?.addEventListener("touchcancel", onCancel, { passive: true });
  };
  const onMove = (ev: TouchEvent) => {
    const a = cur;
    const t = ev.touches[0];
    if (!a || !t || a.mode === "ignore") return;
    if (ev.touches.length !== 1) return finish(false);
    if (a.mode === "pending") {
      const dx = t.clientX - a.x;
      const intent = panIntent(dx, t.clientY - a.y);
      if (intent === "pending") return;
      if (intent === "vertical" || dx * a.pan.dir <= 0) {
        a.mode = "ignore";
        return;
      }
      a.mode = "pan";
      a.anchor = t.clientX;
      a.pan.onStart();
    }
    const along = (t.clientX - a.anchor) * a.pan.dir;
    a.progress = panProgress(t.clientX - a.anchor, a.pan.dir, a.pan.range);
    a.samples.push([ev.timeStamp, along]);
    if (a.samples.length > 6) a.samples.shift();
    a.pan.onMove(a.progress);
    if (ev.cancelable) ev.preventDefault();
  };
  const onEnd = (ev: TouchEvent) => finish(true, ev.timeStamp);
  const onCancel = () => finish(false);

  doc.addEventListener("touchstart", onStart, { passive: true });
  return () => {
    doc.removeEventListener("touchstart", onStart);
    cur?.unbind();
    cur = null;
  };
}
