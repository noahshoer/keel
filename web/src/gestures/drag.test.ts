import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { installDrag, TASK_DROP } from "./drag";
import { ROW_DRAG, ROW_HOLD } from "./long-press";

let uninstall: () => void;
let hit: { y: number };

beforeEach(() => {
  vi.useFakeTimers();
  document.body.innerHTML = `
    <div class="list-scroll" id="scroll">
      <div id="home" data-drop-zone data-drop-project="p" data-drop-section="" data-drop-name="Home">
        <div class="task" id="task" data-menu-id="t1" data-draggable data-project="p" data-section="">
          <div class="task-row" id="row"><div id="body">Do laundry</div><button id="btn" type="button">x</button></div>
        </div>
        <div class="task" id="sub" data-menu-id="t2" data-project="p" data-section="">
          <div class="task-row" id="subrow">A sub-task</div>
        </div>
      </div>
      <div id="chores" data-drop-zone data-drop-project="p" data-drop-section="s" data-drop-name="Chores"></div>
      <div id="new" data-drop-zone data-drop-project="p" data-drop-new="" data-drop-name=""></div>
    </div>`;
  hit = { y: 0 };
  // Rows above 300 are in Home, below it in Chores; above 0 or past 900 is nothing.
  uninstall = installDrag(document, window, {
    at: (_x, y) =>
      y < 0 || y > 900 ? null : document.getElementById(y < 300 ? "home" : y >= 700 ? "new" : "chores"),
  });
  const scroll = document.getElementById("scroll") as HTMLElement;
  scroll.getBoundingClientRect = () =>
    ({ top: 0, bottom: 800, left: 0, right: 400, width: 400, height: 800 }) as DOMRect;
  const task = document.getElementById("task") as HTMLElement;
  task.getBoundingClientRect = () =>
    ({ top: 100, bottom: 160, left: 10, right: 390, width: 380, height: 60 }) as DOMRect;
});

afterEach(() => {
  uninstall();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

const el = (id: string) => document.getElementById(id) as HTMLElement;
const ghost = () => document.querySelector(".drag-ghost");

function touch(type: "touchmove" | "touchend" | "touchcancel", x = 50, y = 120) {
  const ev = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(ev, "touches", { value: type === "touchmove" ? [{ clientX: x, clientY: y }] : [] });
  Object.defineProperty(ev, "changedTouches", { value: [{ clientX: x, clientY: y }] });
  document.body.dispatchEvent(ev);
  return ev;
}
const hold = (id = "task", x = 50, y = 120) =>
  el(id).dispatchEvent(new CustomEvent(ROW_HOLD, { bubbles: true, detail: { x, y } }));

function pointer(type: string, target: HTMLElement, x: number, y: number, pointerType = "mouse") {
  const ev = new Event(type, { bubbles: true, cancelable: true });
  Object.assign(ev, { pointerType, clientX: x, clientY: y, button: 0 });
  target.dispatchEvent(ev);
  return ev;
}

it("a held task that has moved past the slop is dragged: its ghost follows and the page doesn't scroll", () => {
  const drag = vi.fn();
  el("task").addEventListener(ROW_DRAG, drag);
  hold();
  touch("touchmove", 53, 123);
  expect(ghost()).toBeNull();
  expect(drag).not.toHaveBeenCalled();
  const ev = touch("touchmove", 80, 180);
  expect(drag).toHaveBeenCalledTimes(1);
  expect(ghost()).not.toBeNull();
  expect(ghost()?.getAttribute("data-menu-id")).toBeNull();
  expect(el("task").hasAttribute("data-drag-source")).toBe(true);
  expect(ev.defaultPrevented).toBe(true);
  // The ghost keeps the grip: the finger is 40 px in and 20 px down the row.
  const g = ghost() as HTMLElement;
  expect(g.style.left).toBe("40px");
  expect(g.style.top).toBe("160px");
});

it("only a top-level task, and only on a page with somewhere to drop, can be dragged", () => {
  hold("sub");
  touch("touchmove", 80, 180);
  expect(ghost()).toBeNull();
  for (const z of document.querySelectorAll("[data-drop-zone]")) z.removeAttribute("data-drop-zone");
  hold();
  touch("touchmove", 80, 180);
  expect(ghost()).toBeNull();
});

it("the zone under the finger takes the drop, unless it is where the task already is", () => {
  hold();
  touch("touchmove", 80, 180);
  expect(el("home").classList.contains("drop-target")).toBe(false);
  touch("touchmove", 80, 400);
  expect(el("chores").classList.contains("drop-target")).toBe(true);
  touch("touchmove", 80, 200);
  expect(el("chores").classList.contains("drop-target")).toBe(false);
  expect(el("home").classList.contains("drop-target")).toBe(false);
});

it("letting go over another zone drops the task there, and clears the drag", () => {
  const dropped = vi.fn();
  el("chores").addEventListener(TASK_DROP, dropped);
  hold();
  touch("touchmove", 80, 400);
  touch("touchend", 80, 400);
  expect(dropped).toHaveBeenCalledTimes(1);
  expect((dropped.mock.calls[0]?.[0] as CustomEvent | undefined)?.detail).toEqual({ taskId: "t1" });
  expect(ghost()).toBeNull();
  expect(el("task").hasAttribute("data-drag-source")).toBe(false);
  expect(el("chores").classList.contains("drop-target")).toBe(false);
});

it("the new-section zone takes a drop from any task, also in its own project", () => {
  const dropped = vi.fn();
  el("new").addEventListener(TASK_DROP, dropped);
  hold();
  touch("touchmove", 80, 750);
  expect(el("new").classList.contains("drop-target")).toBe(true);
  touch("touchend", 80, 750);
  expect(dropped).toHaveBeenCalledTimes(1);
  expect((dropped.mock.calls[0]?.[0] as CustomEvent | undefined)?.detail).toEqual({ taskId: "t1" });
  expect(el("new").classList.contains("drop-target")).toBe(false);
});

it("letting go over its own zone or over nothing drops nothing", () => {
  const dropped = vi.fn();
  document.addEventListener(TASK_DROP, dropped);
  hold();
  touch("touchmove", 80, 200);
  touch("touchend", 80, 200);
  hold();
  touch("touchmove", 80, 950);
  touch("touchend", 80, 950);
  document.removeEventListener(TASK_DROP, dropped);
  expect(dropped).not.toHaveBeenCalled();
  expect(ghost()).toBeNull();
});

it("a cancelled touch ends the drag without a drop", () => {
  const dropped = vi.fn();
  document.addEventListener(TASK_DROP, dropped);
  hold();
  touch("touchmove", 80, 400);
  touch("touchcancel", 80, 400);
  document.removeEventListener(TASK_DROP, dropped);
  expect(dropped).not.toHaveBeenCalled();
  expect(ghost()).toBeNull();
  expect(el("chores").classList.contains("drop-target")).toBe(false);
});

it("near the bottom of the list the page scrolls under the finger, near the top it scrolls back", () => {
  const scroll = el("scroll");
  scroll.scrollTop = 200;
  hold();
  touch("touchmove", 80, 790);
  vi.advanceTimersByTime(50);
  expect(scroll.scrollTop).toBeGreaterThan(200);
  const down = scroll.scrollTop;
  touch("touchmove", 80, 400);
  vi.advanceTimersByTime(50);
  expect(scroll.scrollTop).toBe(down);
  touch("touchmove", 80, 20);
  vi.advanceTimersByTime(50);
  expect(scroll.scrollTop).toBeLessThan(down);
  touch("touchend", 80, 20);
});

it("a mouse press that moves past a few pixels drags, drops, and the click after it is swallowed", () => {
  const dropped = vi.fn();
  el("chores").addEventListener(TASK_DROP, dropped);
  const opened = vi.fn();
  el("body").addEventListener("click", opened);
  pointer("pointerdown", el("body"), 50, 120);
  pointer("pointermove", el("body"), 52, 121);
  expect(ghost()).toBeNull();
  pointer("pointermove", el("body"), 60, 400);
  expect(ghost()).not.toBeNull();
  pointer("pointerup", el("body"), 60, 400);
  expect(dropped).toHaveBeenCalledTimes(1);
  el("body").click();
  expect(opened).not.toHaveBeenCalled();
  // Only the click right after: the next is a real one.
  vi.advanceTimersByTime(500);
  el("body").click();
  expect(opened).toHaveBeenCalledTimes(1);
});

it("a plain mouse click, or a press on a button, is not a drag", () => {
  pointer("pointerdown", el("body"), 50, 120);
  pointer("pointerup", el("body"), 50, 120);
  pointer("pointerdown", el("btn"), 50, 120);
  pointer("pointermove", el("btn"), 60, 400);
  expect(ghost()).toBeNull();
  // A touch's pointer events are not the mouse's.
  pointer("pointerdown", el("body"), 50, 120, "touch");
  pointer("pointermove", el("body"), 60, 400, "touch");
  expect(ghost()).toBeNull();
});
