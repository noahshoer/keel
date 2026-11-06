// Long press on a task row, a section's header or a sidebar row (touch only). A
// section's header or a sidebar row is sent `ROW_PRESS` when held, which opens its
// menu. A task is sent `ROW_HOLD` when held and lifts; `ROW_PRESS` (its menu,
// `TaskMenu`) comes half a second later if the finger hasn't started to move, or
// when it lifts, unless the task was dragged meanwhile
// (`ROW_DRAG`, sent by `drag.ts`) or the touch was cancelled. A right click sends
// `ROW_PRESS` at once (crates/ui/src/gestures.rs `LONG_PRESS_JS`, without the glass menu,
// which is parked). A sidebar row is highlighted only while the finger is down.
// A plain tap opens the row, as on desktop; hover reveals are mouse-only (app.css).

const HOLD_MS = 450;
/** A held task that hasn't started to move this long after it lifted gets its menu, finger still down. */
export const MENU_DELAY_MS = 500;
const SLOP = 10;
const ROWS = ".task-row, .nav-item, .section-row";
const ACTIONS = ".task-actions, .row-menu";

/** Sent to a task, a section's header or a sidebar row to open its menu: by a long press (a task's, once the finger lifts) or a right click. */
export const ROW_PRESS = "keel-row-press";
/** Sent to a task once it has been held (detail: where the finger is): it lifts, and may be dragged. */
export const ROW_HOLD = "keel-row-hold";
/** Sent to a held task that is being dragged: no menu then. */
export const ROW_DRAG = "keel-row-drag";
/** Sent to a held task whose touch was cancelled: it goes back, with no menu. */
export const ROW_RELEASE = "keel-row-release";

/** Installs the listeners on `doc`; returns a function that removes them. */
export function installLongPress(doc: Document): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let origin: { x: number; y: number } | null = null;
  // After a long press, ignore only a click that follows right away: iOS
  // often sends none, and a flag left set would eat the next real tap.
  let pressedAt = 0;
  // The task that was held and whose finger is still down.
  let held: Element | null = null;
  let menuTimer: ReturnType<typeof setTimeout> | null = null;
  const stopMenu = () => {
    if (menuTimer !== null) clearTimeout(menuTimer);
    menuTimer = null;
  };
  const clear = () => {
    for (const t of doc.querySelectorAll(".pressed")) t.classList.remove("pressed");
  };
  const stop = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
  const lift = () => {
    stop();
    stopMenu();
    for (const t of doc.querySelectorAll(".nav-item.pressed")) t.classList.remove("pressed");
    if (held) {
      const task = held;
      held = null;
      task.dispatchEvent(new Event(ROW_PRESS, { bubbles: true }));
    }
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
        held = task;
        task.dispatchEvent(new CustomEvent(ROW_HOLD, { bubbles: true, detail: { ...origin } }));
        // Not moving: the menu now, not when the finger lifts.
        stopMenu();
        menuTimer = setTimeout(() => {
          menuTimer = null;
          if (held !== task) return;
          held = null;
          task.dispatchEvent(new Event(ROW_PRESS, { bubbles: true }));
        }, MENU_DELAY_MS);
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

  const onDrag = () => {
    held = null;
    stopMenu();
  };
  const onCancel = () => {
    if (!held) return lift();
    const task = held;
    held = null;
    lift();
    task.dispatchEvent(new Event(ROW_RELEASE, { bubbles: true }));
  };

  const passive = { passive: true } as const;
  doc.addEventListener("touchstart", onStart, passive);
  doc.addEventListener("touchmove", onMove, passive);
  doc.addEventListener("touchend", lift, passive);
  doc.addEventListener("touchcancel", onCancel, passive);
  doc.addEventListener(ROW_DRAG, onDrag);
  doc.addEventListener("click", onClick, true);
  doc.addEventListener("contextmenu", onMenu);
  return () => {
    stop();
    stopMenu();
    doc.removeEventListener("touchstart", onStart);
    doc.removeEventListener("touchmove", onMove);
    doc.removeEventListener("touchend", lift);
    doc.removeEventListener("touchcancel", onCancel);
    doc.removeEventListener(ROW_DRAG, onDrag);
    doc.removeEventListener("click", onClick, true);
    doc.removeEventListener("contextmenu", onMenu);
  };
}
