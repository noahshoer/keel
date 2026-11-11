import { render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ArrangePanel } from "./ArrangePanel";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

const narrow = () =>
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));

const panel = () => (
  <ArrangePanel
    title="Filter"
    action={{ label: "Reset", run: () => {} }}
    anchor={{ current: null }}
    onClose={() => {}}
  >
    <p>body</p>
  </ArrangePanel>
);

it("the phone's sheet is modal, has no grabber, and follows the keyboard through visualViewport", () => {
  narrow();
  const vv = new EventTarget() as EventTarget & { height: number; offsetTop: number };
  vv.height = 476;
  vv.offsetTop = 0;
  const add = vi.spyOn(vv, "addEventListener");
  const remove = vi.spyOn(vv, "removeEventListener");
  Object.defineProperty(window, "visualViewport", { value: vv, configurable: true });
  Object.defineProperty(window, "innerHeight", { value: 812, configurable: true });
  const { unmount } = render(panel());
  const sheet = document.querySelector<HTMLElement>(".arrange-sheet") as HTMLElement;
  expect(sheet.getAttribute("aria-modal")).toBe("true");
  expect(document.querySelector(".tv-grabber")).toBeNull();
  expect(sheet.style.getPropertyValue("--kb")).toBe("336px");
  expect(sheet.style.getPropertyValue("--vvh")).toBe("476px");
  expect(add.mock.calls.map((c) => c[0]).sort()).toEqual(["resize", "scroll"]);
  vv.height = 812;
  vv.dispatchEvent(new Event("resize"));
  expect(sheet.style.getPropertyValue("--kb")).toBe("0px");
  unmount();
  expect(remove.mock.calls.map((c) => c[0]).sort()).toEqual(["resize", "scroll"]);
  Object.defineProperty(window, "visualViewport", { value: undefined, configurable: true });
});

it("the popover is not modal", () => {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: true,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
  render(panel());
  expect(document.querySelector(".arrange-popover")?.getAttribute("aria-modal")).toBeNull();
});
