import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { haptic } from "../shell/haptics";
import { touch as sharedTouch } from "../test/touch";
import { HOLD_MS, installLongPress, ROW_HOLD, ROW_PRESS } from "./long-press";

vi.mock("../shell/haptics", () => ({ haptic: vi.fn() }));

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
  vi.mocked(haptic).mockClear();
  uninstall();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

const el = (id: string) => document.getElementById(id) as HTMLElement;

function touch(type: "touchstart" | "touchmove" | "touchend", target: HTMLElement, x = 10, y = 10) {
  sharedTouch(type, target, x, y);
}

it("a half-second hold lifts the task and opens its menu, finger still down", () => {
  const held = vi.fn();
  const pressed = vi.fn();
  el("task").addEventListener(ROW_HOLD, held);
  el("task").addEventListener(ROW_PRESS, pressed);
  touch("touchstart", el("body"));
  vi.advanceTimersByTime(HOLD_MS - 1);
  expect(held).not.toHaveBeenCalled();
  expect(pressed).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(held).toHaveBeenCalledTimes(1);
  expect(pressed).toHaveBeenCalledTimes(1);
  // The hold says where the finger is, so the task can be dragged from there.
  const event = held.mock.calls[0]?.[0] as CustomEvent | undefined;
  expect(event?.detail).toEqual({ x: 10, y: 10 });
  // Lifting the finger then doesn't ask again.
  touch("touchend", el("body"));
  expect(pressed).toHaveBeenCalledTimes(1);
  expect(el("task").classList.contains("pressed")).toBe(false);
});

it("lifting or cancelling before the half second opens nothing", () => {
  const pressed = vi.fn();
  el("task").addEventListener(ROW_PRESS, pressed);
  touch("touchstart", el("body"));
  vi.advanceTimersByTime(HOLD_MS - 1);
  touch("touchend", el("body"));
  vi.advanceTimersByTime(2000);
  touch("touchstart", el("body"));
  vi.advanceTimersByTime(HOLD_MS - 1);
  sharedTouch("touchcancel", el("body"), 10, 10);
  vi.advanceTimersByTime(2000);
  expect(pressed).not.toHaveBeenCalled();
});

it("a sidebar row without a task marks itself", () => {
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(HOLD_MS);
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
  vi.advanceTimersByTime(HOLD_MS);
  el("body").click();
  expect(opened).not.toHaveBeenCalled();
  // Only the click that follows right away: the next real tap goes through.
  el("body").click();
  expect(opened).toHaveBeenCalledTimes(1);

  touch("touchstart", el("body"));
  vi.advanceTimersByTime(HOLD_MS);
  el("delete").click();
  expect(deleted).toHaveBeenCalledTimes(1);
});

it("touching elsewhere clears a pressed sidebar row", () => {
  touch("touchstart", el("nav"));
  vi.advanceTimersByTime(HOLD_MS);
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
  vi.advanceTimersByTime(HOLD_MS);
  expect(el("nav").classList.contains("pressed")).toBe(true);
  touch("touchend", el("nav"));
  expect(el("nav").classList.contains("pressed")).toBe(false);

  touch("touchstart", el("fixed"));
  vi.advanceTimersByTime(HOLD_MS);
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
  vi.advanceTimersByTime(HOLD_MS - 1);
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

it("a long press or a right click on a section's header asks for its menu at once", () => {
  const opened = vi.fn();
  el("section").addEventListener(ROW_PRESS, opened);
  touch("touchstart", el("section"));
  vi.advanceTimersByTime(HOLD_MS);
  expect(opened).toHaveBeenCalledTimes(1);
  el("section").dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  expect(opened).toHaveBeenCalledTimes(2);
});

it("a long press thumps once when it registers; a hold cut short doesn't", () => {
  touch("touchstart", el("body"));
  vi.advanceTimersByTime(HOLD_MS - 1);
  touch("touchend", el("body"));
  vi.advanceTimersByTime(HOLD_MS);
  expect(haptic).not.toHaveBeenCalled();
  touch("touchstart", el("section"));
  vi.advanceTimersByTime(HOLD_MS);
  expect(haptic).toHaveBeenCalledExactlyOnceWith("press");
});
