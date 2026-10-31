// Synthetic touches for gesture tests. jsdom has no Touch constructor: a
// plain cancelable event carrying the touch points, at a chosen time.

export type TouchType = "touchstart" | "touchmove" | "touchend" | "touchcancel";

export function touch(type: TouchType, target: Element, x: number, y: number, at = 0): Event {
  const ev = new Event(type, { bubbles: true, cancelable: true });
  const point = [{ clientX: x, clientY: y }];
  const lifted = type === "touchend" || type === "touchcancel";
  Object.defineProperty(ev, "touches", { value: lifted ? [] : point });
  Object.defineProperty(ev, "changedTouches", { value: point });
  Object.defineProperty(ev, "timeStamp", { value: at });
  target.dispatchEvent(ev);
  return ev;
}

export function setWidth(w: number): void {
  Object.defineProperty(window, "innerWidth", { configurable: true, value: w });
}
