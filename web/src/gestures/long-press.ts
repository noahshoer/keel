// Long press on a task row, a section's header or a sidebar row (touch only). Held
// for half a second, a section's header or a sidebar row is sent `ROW_PRESS`, which
// opens its menu. A task is sent `ROW_HOLD` (it lifts, and may be dragged from there,
// drag.ts) and then `ROW_PRESS` (its menu, `TaskMenu`), with the finger still down; a
// drag that starts then closes the menu (`ROW_DRAG`). A right click sends `ROW_PRESS`
// at once (crates/ui/src/gestures.rs `LONG_PRESS_JS`, without the glass menu, which is
// parked). A sidebar row is highlighted only while the finger is down. A plain tap
// opens the row, as on desktop; hover reveals are mouse-only (app.css).

/** How long a finger holds still on a row before it is a long press (ms). */
export const HOLD_MS = 500;
const SLOP = 10;
const ROWS = ".task-row, .nav-item, .section-row";
const ACTIONS = ".task-actions, .row-menu";

/** Sent to a task, a section's header or a sidebar row to open its menu: by a long press or a right click. */
export const ROW_PRESS = "keel-row-press";
/** Sent to a task once it has been held (detail: where the finger is): it lifts, and may be dragged. */
export const ROW_HOLD = "keel-row-hold";
/** Sent to a held task that is being dragged (by drag.ts): its menu closes. */
export const ROW_DRAG = "keel-row-drag";

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
  const lift = () => {
    stop();
    for (const t of doc.querySelectorAll(".nav-item.pressed")) t.classList.remove("pressed");
    // A hold can far outlast the click's window, which starts when the finger lifts.
    if (pressedAt) pressedAt = Date.now();
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
      const task = row.closest(".task");
      if (task) {
        task.dispatchEvent(new CustomEvent(ROW_HOLD, { bubbles: true, detail: { ...origin } }));
        task.dispatchEvent(new Event(ROW_PRESS, { bubbles: true }));
      } else {
        if (row.matches(".nav-item")) row.classList.add("pressed");
        row.dispatchEvent(new Event(ROW_PRESS));
      }
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
      // Every listener: a menu that just opened closes on a click outside it.
      ev.stopImmediatePropagation();
      ev.preventDefault();
    }
    pressedAt = 0;
  };
  const onMenu = (ev: MouseEvent) => {
    if (closest(ev.target, ROWS)) ev.preventDefault();
    const row = closest(ev.target, ROWS);
    (row?.closest(".task") ?? row)?.dispatchEvent(new Event(ROW_PRESS));
  };

  const passive = { passive: true } as const;
  doc.addEventListener("touchstart", onStart, passive);
  doc.addEventListener("touchmove", onMove, passive);
  doc.addEventListener("touchend", lift, passive);
  doc.addEventListener("touchcancel", lift, passive);
  doc.addEventListener("click", onClick, true);
  doc.addEventListener("contextmenu", onMenu);
  return () => {
    stop();
    doc.removeEventListener("touchstart", onStart);
    doc.removeEventListener("touchmove", onMove);
    doc.removeEventListener("touchend", lift);
    doc.removeEventListener("touchcancel", lift);
    doc.removeEventListener("click", onClick, true);
    doc.removeEventListener("contextmenu", onMenu);
  };
}
