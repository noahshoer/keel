// Long press on a task row or a sidebar row (touch only): the row's task
// (or the row) gets `.pressed`, which shows its row actions
// (crates/ui/src/gestures.rs `LONG_PRESS_JS`, without the glass menu, which
// is parked). A plain tap opens the row, as on desktop; hover reveals are
// mouse-only (app.css).

const HOLD_MS = 450;
const SLOP = 10;
const ROWS = ".task-row, .nav-item";
const ACTIONS = ".task-actions, .nav-actions";

/** Installs the listeners on `doc`; returns a function that removes them. */
export function installLongPress(doc: Document): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let origin: { x: number; y: number } | null = null;
  // After a long press, ignore only a click that follows right away: iOS
  // often sends none, and a flag left set would eat the next real tap.
  let pressedAt = 0;
  const clear = () => {
    for (const t of doc.querySelectorAll(".pressed")) t.classList.remove("pressed");
  };
  const stop = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
  const closest = (target: EventTarget | null, sel: string): Element | null =>
    target instanceof Element ? target.closest(sel) : null;

  const onStart = (ev: TouchEvent) => {
    const row = closest(ev.target, ROWS);
    if (!row || closest(ev.target, ACTIONS)) {
      if (!closest(ev.target, ACTIONS)) clear();
      return;
    }
    const t = ev.touches[0];
    if (!t) return;
    origin = { x: t.clientX, y: t.clientY };
    stop();
    timer = setTimeout(() => {
      timer = null;
      clear();
      (row.closest(".task") ?? row).classList.add("pressed");
      pressedAt = Date.now();
    }, HOLD_MS);
  };
  const onMove = (ev: TouchEvent) => {
    const t = ev.touches[0];
    if (timer === null || !origin || !t) return;
    if (Math.abs(t.clientX - origin.x) > SLOP || Math.abs(t.clientY - origin.y) > SLOP) stop();
  };
  const onClick = (ev: MouseEvent) => {
    if (pressedAt && Date.now() - pressedAt < 700 && !closest(ev.target, ACTIONS)) {
      ev.stopPropagation();
      ev.preventDefault();
    }
    pressedAt = 0;
  };
  const onMenu = (ev: MouseEvent) => {
    if (closest(ev.target, ROWS)) ev.preventDefault();
  };

  const passive = { passive: true } as const;
  doc.addEventListener("touchstart", onStart, passive);
  doc.addEventListener("touchmove", onMove, passive);
  doc.addEventListener("touchend", stop, passive);
  doc.addEventListener("touchcancel", stop, passive);
  doc.addEventListener("click", onClick, true);
  doc.addEventListener("contextmenu", onMenu);
  return () => {
    stop();
    doc.removeEventListener("touchstart", onStart);
    doc.removeEventListener("touchmove", onMove);
    doc.removeEventListener("touchend", stop);
    doc.removeEventListener("touchcancel", stop);
    doc.removeEventListener("click", onClick, true);
    doc.removeEventListener("contextmenu", onMenu);
  };
}
