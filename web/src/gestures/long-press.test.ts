import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { installLongPress, ROW_PRESS } from "./long-press";

let uninstall: () => void;

beforeEach(() => {
  vi.useFakeTimers();
  document.body.innerHTML = `
    <div class="task" id="task">
      <div class="task-row" id="row">
        <div class="task-body" id="body">Do laundry</div>
        <div class="task-actions"><button id="delete">Delete</button></div>
      </div>
    </div>
    <div class="section-row" id="section">Chores</div>
    <div class="nav-item" id="nav">Home</div>
    <div class="nav-item" id="fixed">Inbox</div>
    <div id="elsewhere">elsewhere</div>`;
  uninstall = installLongPress(document);
});

afterEach(() => {
  uninstall();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

const el = (id: string) => document.getElementById(id) as HTMLElement;

// jsdom has no Touch constructor: a plain event carrying the touch points.
function touch(type: "touchstart" | "touchmove" | "touchend", target: HTMLElement, x = 10, y = 10) {
  const ev = new Event(type, { bubbles: true });
  const points = type === "touchend" ? [] : [{ clientX: x, clientY: y }];
  Object.defineProperty(ev, "touches", { value: points });
  target.dispatchEvent(ev);
}

it("a 450 ms hold asks the task for its menu, once", () => {
  const opened = vi.fn();
  el("task").addEventListener(ROW_PRESS, opened);
  touch("touchstart", el("body"));
  vi.advanceTimersByTime(449);
  expect(opened).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(opened).toHaveBeenCalledTimes(1);
  // The menu lifts the row itself; the press adds no class to it.
  expect(el("task").classList.contains("pressed")).toBe(false);
});

it("a sidebar row without a task marks itself", () => {
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(450);
  expect(el("nav").classList.contains("pressed")).toBe(true);
});

it("moving more than 10 px cancels the hold", () => {
  const opened = vi.fn();
  el("task").addEventListener(ROW_PRESS, opened);
  touch("touchstart", el("body"), 10, 10);
  touch("touchmove", el("body"), 22, 10);
  vi.advanceTimersByTime(500);
  expect(opened).not.toHaveBeenCalled();
});

it("lifting early cancels the hold", () => {
  const opened = vi.fn();
  el("task").addEventListener(ROW_PRESS, opened);
  touch("touchstart", el("body"));
  touch("touchend", el("body"));
  vi.advanceTimersByTime(500);
  expect(opened).not.toHaveBeenCalled();
});

it("the click right after a long press is swallowed, except on the row's actions", () => {
  const opened = vi.fn();
  el("body").addEventListener("click", opened);
  const deleted = vi.fn();
  el("delete").addEventListener("click", deleted);
  touch("touchstart", el("body"));
  vi.advanceTimersByTime(450);
  el("body").click();
  expect(opened).not.toHaveBeenCalled();
  // Only the click that follows right away: the next real tap goes through.
  el("body").click();
  expect(opened).toHaveBeenCalledTimes(1);

  touch("touchstart", el("body"));
  vi.advanceTimersByTime(450);
  el("delete").click();
  expect(deleted).toHaveBeenCalledTimes(1);
});

it("touching elsewhere clears a pressed sidebar row", () => {
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(450);
  touch("touchstart", el("elsewhere"));
  expect(el("nav").classList.contains("pressed")).toBe(false);
});

it("the context menu is suppressed on rows only", () => {
  const onRow = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
  el("body").dispatchEvent(onRow);
  expect(onRow.defaultPrevented).toBe(true);
  const off = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
  el("elsewhere").dispatchEvent(off);
  expect(off.defaultPrevented).toBe(false);
});

it("a sidebar row's highlight goes when the finger lifts", () => {
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(450);
  expect(el("nav").classList.contains("pressed")).toBe(true);
  touch("touchend", el("nav"));
  expect(el("nav").classList.contains("pressed")).toBe(false);

  touch("touchstart", el("fixed"));
  vi.advanceTimersByTime(450);
  touch("touchend", el("fixed"));
  expect(el("fixed").classList.contains("pressed")).toBe(false);
});

it("a right click on a sidebar row or a task asks for its menu", () => {
  const opened = vi.fn();
  el("nav").addEventListener(ROW_PRESS, opened);
  el("nav").dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  expect(opened).toHaveBeenCalledTimes(1);
  const onTask = vi.fn();
  el("task").addEventListener(ROW_PRESS, onTask);
  el("body").dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  expect(onTask).toHaveBeenCalledTimes(1);
});

it("a long press on a sidebar row asks for its menu, once", () => {
  const opened = vi.fn();
  el("nav").addEventListener(ROW_PRESS, opened);
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(449);
  expect(opened).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(opened).toHaveBeenCalledTimes(1);
});

it("the click after lifting a long held press is swallowed for every listener", () => {
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(3000);
  touch("touchend", el("nav"));
  const later = vi.fn();
  document.addEventListener("click", later, true);
  el("nav").click();
  document.removeEventListener("click", later, true);
  expect(later).not.toHaveBeenCalled();
});

it("a long press or a right click on a section's header asks for its menu", () => {
  const opened = vi.fn();
  el("section").addEventListener(ROW_PRESS, opened);
  touch("touchstart", el("section"));
  vi.advanceTimersByTime(450);
  expect(opened).toHaveBeenCalledTimes(1);
  el("section").dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  expect(opened).toHaveBeenCalledTimes(2);
});
