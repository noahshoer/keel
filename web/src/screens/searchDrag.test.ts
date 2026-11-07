import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { setWidth, touch } from "../test/touch";
import { installSearchDrag } from "./searchDrag";

let uninstall: () => void;
let calls: string[];
let isSearch: boolean;
let drawerOpen: boolean;

const ghostEl = () => document.querySelector<HTMLElement>(".view-ghost");
const content = () => document.querySelector(".content") as HTMLElement;

beforeEach(() => {
  vi.useFakeTimers();
  setWidth(390);
  document.body.innerHTML = `<div class="app"><main class="main"><div class="content"><p>Results</p></div></main></div>`;
  Object.defineProperty(document.querySelector(".main"), "offsetWidth", { value: 390 });
  calls = [];
  isSearch = true;
  drawerOpen = false;
  uninstall = installSearchDrag(document, {
    isSearch: () => isSearch,
    drawerOpen: () => drawerOpen,
    leave: () => calls.push("leave"),
    cancel: () => calls.push("cancel"),
  });
});

afterEach(() => {
  uninstall();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

const drag = (to: number, end: boolean, at = 0) => {
  touch("touchstart", content(), 20, 300, at);
  touch("touchmove", content(), 40, 302, at + 10);
  touch("touchmove", content(), to, 304, at + 500);
  if (end) touch("touchend", content(), to, 304, at + 510);
};

it("dragging right lays a still copy of Search over the page and moves it with the finger", () => {
  drag(240, false);
  expect(calls).toEqual(["leave"]);
  const g = ghostEl();
  expect(g?.classList.contains("pop")).toBe(true);
  expect(g?.classList.contains("dragging")).toBe(true);
  expect(g?.style.transform).toBe("translateX(200px)");
});

it("released past halfway it carries off, then goes", () => {
  drag(300, true);
  expect(ghostEl()?.style.transform).toBe("translateX(390px)");
  expect(calls).toEqual(["leave"]);
  vi.advanceTimersByTime(400);
  expect(ghostEl()).toBeNull();
});

it("released short and slow it springs back and Search comes back under it", () => {
  drag(100, true);
  expect(ghostEl()?.style.transform).toBe("translateX(0px)");
  expect(calls).toEqual(["leave"]);
  vi.advanceTimersByTime(400);
  expect(calls).toEqual(["leave", "cancel"]);
  expect(ghostEl()).not.toBeNull(); // it stays until Search has landed again
});

it("only on the Search page, with the drawer shut", () => {
  isSearch = false;
  drag(300, true);
  drawerOpen = true;
  isSearch = true;
  drag(300, true, 2000);
  expect(calls).toEqual([]);
  expect(ghostEl()).toBeNull();
});

it("leaves the swipe to the instant fallback when motion is reduced", () => {
  vi.stubGlobal("matchMedia", () => ({ matches: true, addEventListener() {}, removeEventListener() {} }));
  drag(300, true);
  vi.unstubAllGlobals();
  expect(calls).toEqual([]);
});
