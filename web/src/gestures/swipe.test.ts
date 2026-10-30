import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { setWidth, touch } from "../test/touch";
import type { Swipe } from "./sheet-rules";
import { installSwipe } from "./swipe";

let uninstall: () => void;
let swipes: Swipe[];

beforeEach(() => {
  setWidth(390);
  document.body.innerHTML = `
    <main id="list">List</main>
    <input id="field" />
    <div class="tv-scrim" id="scrim"></div>
    <div class="tv-sheet" id="sheet">Sheet</div>`;
  swipes = [];
  uninstall = installSwipe(document, (s) => swipes.push(s));
});

afterEach(() => {
  uninstall();
  document.body.innerHTML = "";
});

const el = (id: string) => document.getElementById(id) as HTMLElement;

function swipe(id: string, dx: number, dy = 0) {
  touch("touchstart", el(id), 20, 100);
  touch("touchend", el(id), 20 + dx, 100 + dy);
}

it("a rightward touch reports its travel and the width", () => {
  swipe("list", 120);
  expect(swipes).toEqual([{ dx: 120, dy: 0, width: 390, blocked: false, inSheet: false }]);
});

it("a touch on the scrim counts as in the sheet", () => {
  // So a sideways swipe on the scrim doesn't open the drawer over it.
  swipe("scrim", 120);
  expect(swipes[0]?.inSheet).toBe(true);
  swipe("sheet", 120);
  expect(swipes[1]?.inSheet).toBe(true);
});

it("a touch starting in a field is blocked", () => {
  swipe("field", 120);
  expect(swipes[0]?.blocked).toBe(true);
});

it("reports nothing after uninstall", () => {
  const spy = vi.fn();
  uninstall();
  uninstall = installSwipe(document, spy);
  uninstall();
  swipe("list", 120);
  expect(spy).not.toHaveBeenCalled();
  expect(swipes).toEqual([]);
  uninstall = () => {};
});
