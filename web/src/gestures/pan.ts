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
  const finish = (commit: boolean) => {
    const a = cur;
    cur = null;
    if (a?.mode !== "pan") return;
    a.pan.onEnd(commit ? panCommits(a.progress, velocity(a.samples)) : false, a.progress);
  };

  const onStart = (ev: TouchEvent) => {
    if (cur) finish(false);
    const t = ev.touches[0];
    if (ev.touches.length !== 1 || !t) return;
    const pan = begin(touchContext(doc, ev));
    if (!pan) return;
    cur = { pan, x: t.clientX, y: t.clientY, mode: "pending", anchor: 0, progress: 0, samples: [] };
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
  const onEnd = () => finish(true);
  const onCancel = () => finish(false);

  const passive = { passive: true } as const;
  doc.addEventListener("touchstart", onStart, passive);
  doc.addEventListener("touchmove", onMove, { passive: false });
  doc.addEventListener("touchend", onEnd, passive);
  doc.addEventListener("touchcancel", onCancel, passive);
  return () => {
    doc.removeEventListener("touchstart", onStart);
    doc.removeEventListener("touchmove", onMove);
    doc.removeEventListener("touchend", onEnd);
    doc.removeEventListener("touchcancel", onCancel);
  };
}
