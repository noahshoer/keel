import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { setWidth, touch } from "../test/touch";
import { canPan, installPan, type Pan } from "./pan";

let uninstall: () => void;
let pan: Pan;
let log: string[];

beforeEach(() => {
  setWidth(390);
  document.body.innerHTML = `<main id="list">List</main><input id="field" /><div class="modal-scrim"><p id="settings"></p></div>`;
  log = [];
  pan = {
    dir: 1,
    range: 300,
    onStart: () => log.push("start"),
    onMove: (p) => log.push(`move ${p.toFixed(2)}`),
    onEnd: (commit, p) => log.push(`end ${commit} ${p.toFixed(2)}`),
  };
  uninstall = installPan(document, () => pan);
});

afterEach(() => {
  uninstall();
  document.body.innerHTML = "";
});

const list = () => document.getElementById("list") as HTMLElement;

it("follows the finger sideways, then commits when released past halfway", () => {
  touch("touchstart", list(), 20, 100, 0);
  touch("touchmove", list(), 40, 102, 50); // past the slop: the pan starts here
  touch("touchmove", list(), 190, 104, 400);
  touch("touchend", list(), 190, 104, 410);
  expect(log).toEqual(["start", "move 0.00", "move 0.50", "end true 0.50"]);
});

it("springs back when released short and slow", () => {
  touch("touchstart", list(), 20, 100, 0);
  touch("touchmove", list(), 40, 100, 100);
  touch("touchmove", list(), 100, 100, 1000);
  touch("touchend", list(), 100, 100, 1010);
  expect(log.at(-1)).toBe("end false 0.20");
});

it("a quick flick commits however short", () => {
  touch("touchstart", list(), 20, 100, 0);
  touch("touchmove", list(), 40, 100, 10);
  touch("touchmove", list(), 100, 100, 50);
  touch("touchend", list(), 100, 100, 60);
  expect(log.at(-1)).toMatch(/^end true/);
});

it("cancels the browser's scroll once it is a pan, not before", () => {
  touch("touchstart", list(), 20, 100, 0);
  const early = touch("touchmove", list(), 24, 100, 10);
  const moving = touch("touchmove", list(), 60, 100, 20);
  expect(early.defaultPrevented).toBe(false);
  expect(moving.defaultPrevented).toBe(true);
});

it("leaves a vertical scroll alone", () => {
  touch("touchstart", list(), 20, 100, 0);
  const ev = touch("touchmove", list(), 24, 160, 20);
  touch("touchmove", list(), 60, 300, 40);
  touch("touchend", list(), 60, 300, 50);
  expect(ev.defaultPrevented).toBe(false);
  expect(log).toEqual([]);
});

it("ignores a move against the pan's direction", () => {
  touch("touchstart", list(), 200, 100, 0);
  touch("touchmove", list(), 100, 100, 20);
  touch("touchend", list(), 100, 100, 30);
  expect(log).toEqual([]);
});

it("a left pan counts leftward travel", () => {
  pan.dir = -1;
  touch("touchstart", list(), 300, 100, 0);
  touch("touchmove", list(), 280, 100, 50);
  touch("touchmove", list(), 130, 100, 400);
  touch("touchend", list(), 130, 100, 410);
  expect(log.at(-1)).toBe("end true 0.50");
});

it("does nothing when begin declines the touch", () => {
  uninstall();
  uninstall = installPan(document, () => null);
  touch("touchstart", list(), 20, 100, 0);
  const ev = touch("touchmove", list(), 100, 100, 20);
  expect(ev.defaultPrevented).toBe(false);
  expect(log).toEqual([]);
});

it("a second finger or a cancel springs back", () => {
  touch("touchstart", list(), 20, 100, 0);
  touch("touchmove", list(), 40, 100, 50);
  touch("touchmove", list(), 230, 100, 400);
  touch("touchcancel", list(), 230, 100, 410);
  expect(log.at(-1)).toBe("end false 0.63");
});

it("tells begin where the touch started and how wide the screen is", () => {
  const seen: unknown[] = [];
  uninstall();
  uninstall = installPan(document, (info) => {
    seen.push(info);
    return null;
  });
  touch("touchstart", document.getElementById("field") as HTMLElement, 20, 100);
  touch("touchstart", document.getElementById("settings") as HTMLElement, 20, 100);
  expect(seen).toEqual([
    { width: 390, blocked: true, inSheet: false, inSettings: false },
    { width: 390, blocked: false, inSheet: false, inSettings: true },
  ]);
});

it("reports nothing after uninstall", () => {
  const spy = vi.fn(() => pan);
  uninstall();
  uninstall = installPan(document, spy);
  uninstall();
  touch("touchstart", list(), 20, 100);
  expect(spy).not.toHaveBeenCalled();
  uninstall = () => {};
});

it("a finger that rests before lifting is not a flick", () => {
  touch("touchstart", list(), 20, 100, 0);
  touch("touchmove", list(), 40, 100, 10);
  touch("touchmove", list(), 100, 100, 20); // fast, then it stops
  touch("touchend", list(), 100, 100, 900);
  expect(log.at(-1)).toBe("end false 0.20");
});

it("keeps following a touch whose element left the page mid-pan", () => {
  // The Search screen is replaced when the pan starts; WebKit then sends the
  // touch's moves and its end to the removed element only, not up to the document.
  const target = list();
  touch("touchstart", target, 20, 100, 0);
  touch("touchmove", target, 40, 102, 50);
  target.remove();
  touch("touchmove", target, 190, 104, 400);
  touch("touchend", target, 190, 104, 410);
  expect(log).toEqual(["start", "move 0.00", "move 0.50", "end true 0.50"]);
});

it("canPan wants the narrow layout with nothing blocking", () => {
  const ok = { width: 390, blocked: false, inSheet: false, inSettings: false };
  expect(canPan(ok)).toBe(true);
  expect(canPan({ ...ok, width: 700 })).toBe(true);
  expect(canPan({ ...ok, width: 701 })).toBe(false);
  expect(canPan({ ...ok, blocked: true })).toBe(false);
  expect(canPan({ ...ok, inSheet: true })).toBe(false);
  expect(canPan({ ...ok, inSettings: true })).toBe(false);
});
