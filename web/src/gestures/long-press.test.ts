import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { installLongPress } from "./long-press";

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
    <div class="nav-item" id="nav">Home<span class="nav-actions"><button class="nav-more" id="more">More</button></span></div>
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

it("a 450 ms hold marks the task pressed", () => {
  touch("touchstart", el("body"));
  vi.advanceTimersByTime(449);
  expect(el("task").classList.contains("pressed")).toBe(false);
  vi.advanceTimersByTime(1);
  expect(el("task").classList.contains("pressed")).toBe(true);
});

it("a sidebar row without a task marks itself", () => {
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(450);
  expect(el("nav").classList.contains("pressed")).toBe(true);
});

it("moving more than 10 px cancels the hold", () => {
  touch("touchstart", el("body"), 10, 10);
  touch("touchmove", el("body"), 22, 10);
  vi.advanceTimersByTime(500);
  expect(el("task").classList.contains("pressed")).toBe(false);
});

it("lifting early cancels the hold", () => {
  touch("touchstart", el("body"));
  touch("touchend", el("body"));
  vi.advanceTimersByTime(500);
  expect(el("task").classList.contains("pressed")).toBe(false);
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

it("touching elsewhere clears the pressed row", () => {
  touch("touchstart", el("body"));
  vi.advanceTimersByTime(450);
  touch("touchstart", el("elsewhere"));
  expect(el("task").classList.contains("pressed")).toBe(false);
});

it("the context menu is suppressed on rows only", () => {
  const onRow = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
  el("body").dispatchEvent(onRow);
  expect(onRow.defaultPrevented).toBe(true);
  const off = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
  el("elsewhere").dispatchEvent(off);
  expect(off.defaultPrevented).toBe(false);
});

it("a sidebar row's highlight goes when the finger lifts; a task's stays", () => {
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(450);
  expect(el("nav").classList.contains("pressed")).toBe(true);
  touch("touchend", el("nav"));
  expect(el("nav").classList.contains("pressed")).toBe(false);

  touch("touchstart", el("fixed"));
  vi.advanceTimersByTime(450);
  touch("touchend", el("fixed"));
  expect(el("fixed").classList.contains("pressed")).toBe(false);

  touch("touchstart", el("body"));
  vi.advanceTimersByTime(450);
  touch("touchend", el("body"));
  expect(el("task").classList.contains("pressed")).toBe(true);
});

it("a long press on a sidebar row opens its menu, once", () => {
  const opened = vi.fn();
  el("more").addEventListener("click", opened);
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
