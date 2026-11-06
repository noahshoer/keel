// Dragging a task to another section. A touch drag starts from a hold
// (`ROW_HOLD`, long-press.ts): the finger moves on and the task's ghost follows
// it, even when the hold has opened the task's menu (the drag closes it). A mouse drag starts from a press on a row that moves a few pixels. Either
// way, `[data-drop-zone]` elements are where it can land: the one under the
// finger (unless it's where the task already is) is the `drop-target`, and letting
// go sends it `TASK_DROP`; its component does the move. The page scrolls under a
// finger near the list's top or bottom. Only a task marked `data-draggable` goes
// (a sub-task doesn't), and only on a page that has a drop zone. A zone marked
// `data-drop-new` starts a new section: it takes a drop from any task.
import {
  edgeScroll,
  isMove,
  MOUSE_SLOP,
  movedBeyond,
  NEW_SECTION,
  type Place,
  TOUCH_SLOP,
} from "./drag-rules";
import { ROW_DRAG, ROW_HOLD } from "./long-press";

/** Sent to the drop zone a task is let go over (detail: `{ taskId }`). */
export const TASK_DROP = "keel-task-drop";

const ZONE = "[data-drop-zone]";
/** A press on these is theirs, not the start of a drag. */
const OWN = "button, a, input, textarea, select, .task-actions";
/** The click that follows a mouse drag's release is not a click on the row. */
const SWALLOW_MS = 400;

type Options = {
  /** What's at a point on the page (the ghost doesn't count). */
  at?: (x: number, y: number) => Element | null;
};

type Press = { task: HTMLElement; x: number; y: number; mouse: boolean };
type Drag = {
  task: HTMLElement;
  ghost: HTMLElement;
  offX: number;
  offY: number;
  scroller: HTMLElement | null;
  target: HTMLElement | null;
  x: number;
  y: number;
  frame: number;
};

export function installDrag(doc: Document, win: Window, options: Options = {}): () => void {
  const at = options.at ?? ((x, y) => doc.elementFromPoint(x, y));
  let press: Press | null = null;
  let drag: Drag | null = null;
  let swallowUntil = 0;

  const placeOf = (task: HTMLElement): Place => ({
    project: task.dataset["project"] ?? "",
    section: task.dataset["section"] ?? "",
  });
  // A zone that starts a new section is never where a task already is.
  const zonePlace = (zone: HTMLElement): Place => ({
    project: zone.dataset["dropProject"] ?? "",
    section: zone.dataset["dropNew"] !== undefined ? NEW_SECTION : (zone.dataset["dropSection"] ?? ""),
  });
  const canDrag = (task: HTMLElement) =>
    task.hasAttribute("data-draggable") && doc.querySelector(ZONE) !== null;

  const setTarget = (d: Drag, next: HTMLElement | null) => {
    if (d.target === next) return;
    d.target?.classList.remove("drop-target");
    next?.classList.add("drop-target");
    d.target = next;
  };

  const move = (d: Drag, x: number, y: number) => {
    d.x = x;
    d.y = y;
    d.ghost.style.left = `${x - d.offX}px`;
    d.ghost.style.top = `${y - d.offY}px`;
    const zone = at(x, y)?.closest<HTMLElement>(ZONE) ?? null;
    setTarget(d, zone && isMove(placeOf(d.task), zonePlace(zone)) ? zone : null);
  };

  // Scrolls the list while the finger is near its edge, and finds the zone under it again.
  const scrollLoop = (d: Drag) => {
    const tick = () => {
      if (drag !== d) return;
      if (d.scroller) {
        const rect = d.scroller.getBoundingClientRect();
        const dy = edgeScroll(d.y, rect.top, rect.bottom);
        if (dy !== 0) {
          d.scroller.scrollTop += dy;
          move(d, d.x, d.y);
        }
      }
      d.frame = win.requestAnimationFrame(tick);
    };
    d.frame = win.requestAnimationFrame(tick);
  };

  const begin = (p: Press, x: number, y: number) => {
    const rect = p.task.getBoundingClientRect();
    const ghost = p.task.cloneNode(true) as HTMLElement;
    for (const attr of ["id", "data-menu-id", "data-draggable"]) ghost.removeAttribute(attr);
    ghost.classList.remove("lifted", "pressed");
    ghost.classList.add("drag-ghost");
    Object.assign(ghost.style, {
      position: "fixed",
      margin: "0",
      width: `${rect.width}px`,
      pointerEvents: "none",
      zIndex: "1000",
    });
    doc.body.appendChild(ghost);
    // An attribute, not a class: the row's component owns its `className` and resets it when the hold ends.
    p.task.setAttribute("data-drag-source", "");
    doc.body.classList.add("dragging");
    const d: Drag = {
      task: p.task,
      ghost,
      offX: p.x - rect.left,
      offY: p.y - rect.top,
      scroller: p.task.closest<HTMLElement>(".list-scroll"),
      target: null,
      x,
      y,
      frame: 0,
    };
    drag = d;
    p.task.dispatchEvent(new Event(ROW_DRAG, { bubbles: true }));
    move(d, x, y);
    scrollLoop(d);
  };

  /** Lets go: drops on the target (unless `commit` is false), and clears everything the drag put on the page. */
  const end = (commit: boolean) => {
    const d = drag;
    drag = null;
    const p = press;
    press = null;
    detach();
    if (!d) return;
    win.cancelAnimationFrame(d.frame);
    d.ghost.remove();
    d.task.removeAttribute("data-drag-source");
    d.target?.classList.remove("drop-target");
    doc.body.classList.remove("dragging");
    if (commit && d.target) {
      d.target.dispatchEvent(
        new CustomEvent(TASK_DROP, { bubbles: true, detail: { taskId: d.task.dataset["menuId"] ?? "" } }),
      );
    }
    if (p?.mouse) swallowUntil = Date.now() + SWALLOW_MS;
  };

  const onTouchMove = (e: TouchEvent) => {
    const t = e.touches[0];
    if (!t || !press || press.mouse) return;
    // The page doesn't scroll under a held finger: a scroll would close the menu and take the touch.
    e.preventDefault();
    if (!drag && movedBeyond(t.clientX - press.x, t.clientY - press.y, TOUCH_SLOP))
      begin(press, t.clientX, t.clientY);
    if (drag) move(drag, t.clientX, t.clientY);
  };
  const onTouchEnd = () => end(true);
  const onTouchCancel = () => end(false);
  const onPointerMove = (e: PointerEvent) => {
    if (!press?.mouse) return;
    if (!drag && movedBeyond(e.clientX - press.x, e.clientY - press.y, MOUSE_SLOP))
      begin(press, e.clientX, e.clientY);
    if (drag) move(drag, e.clientX, e.clientY);
  };
  const onPointerUp = () => end(true);

  const nonPassive = { passive: false } as const;
  const attach = () => {
    doc.addEventListener("touchmove", onTouchMove, nonPassive);
    doc.addEventListener("touchend", onTouchEnd);
    doc.addEventListener("touchcancel", onTouchCancel);
    doc.addEventListener("pointermove", onPointerMove);
    doc.addEventListener("pointerup", onPointerUp);
    doc.addEventListener("pointercancel", onPointerUp);
  };
  function detach() {
    doc.removeEventListener("touchmove", onTouchMove);
    doc.removeEventListener("touchend", onTouchEnd);
    doc.removeEventListener("touchcancel", onTouchCancel);
    doc.removeEventListener("pointermove", onPointerMove);
    doc.removeEventListener("pointerup", onPointerUp);
    doc.removeEventListener("pointercancel", onPointerUp);
  }

  const onHold = (e: Event) => {
    const task = e.target instanceof HTMLElement ? e.target.closest<HTMLElement>(".task") : null;
    if (!task || !canDrag(task)) return;
    const { x, y } = (e as CustomEvent<{ x: number; y: number }>).detail;
    press = { task, x, y, mouse: false };
    attach();
  };
  const onPointerDown = (e: PointerEvent) => {
    if (e.pointerType !== "mouse" || e.button !== 0 || !(e.target instanceof Element)) return;
    if (e.target.closest(OWN)) return;
    const task = e.target.closest<HTMLElement>(".task");
    if (!task || !e.target.closest(".task-row") || !canDrag(task)) return;
    press = { task, x: e.clientX, y: e.clientY, mouse: true };
    attach();
  };
  const onClick = (e: MouseEvent) => {
    if (Date.now() >= swallowUntil) return;
    swallowUntil = 0;
    e.stopImmediatePropagation();
    e.preventDefault();
  };

  doc.addEventListener(ROW_HOLD, onHold);
  doc.addEventListener("pointerdown", onPointerDown);
  doc.addEventListener("click", onClick, true);
  return () => {
    end(false);
    doc.removeEventListener(ROW_HOLD, onHold);
    doc.removeEventListener("pointerdown", onPointerDown);
    doc.removeEventListener("click", onClick, true);
  };
}
